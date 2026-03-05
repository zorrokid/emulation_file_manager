# Libretro Integration

This document explains how the libretro emulation system works in this codebase and how to add support for new cores.

## Table of Contents

- [What is libretro?](#what-is-libretro)
- [Architecture overview](#architecture-overview)
- [The files](#the-files)
- [How it works — detailed walkthrough](#how-it-works--detailed-walkthrough)
- [Adding a new core](#adding-a-new-core)
- [Troubleshooting](#troubleshooting)

---

## What is libretro?

A libretro **core** is a `.so` shared library that emulates a system (NES, SNES, Game Boy, etc.). It implements a standardised C interface. Your application (the "frontend") loads the library at runtime and they communicate through **callbacks** — function pointers each side registers with the other.

The core does not open windows, play audio, or read input on its own. It only emulates the hardware and hands the results back to the frontend via callbacks. The frontend is responsible for displaying video, playing audio, and feeding input.

---

## Architecture overview

```
This App (Rust)                    Core (.so, C)
──────────────────────────────────────────────────
loads library via dlopen()
calls retro_set_environment()  →   core stores your fn pointer
calls retro_init()             →   core calls environment_cb() back
calls retro_load_game()        →   core loads ROM, calls callbacks

loop every ~16ms:
calls retro_run()              →   core renders one frame
                               ←   calls video_refresh_cb()
                               ←   calls audio_sample_batch_cb()
                               ←   calls input_state_cb()
```

The crate responsible for all of this is `libretro_runner`. The GTK window that displays the game lives in `relm4-ui/src/libretro/`.

---

## The files

| File | Purpose |
|---|---|
| `libretro_runner/src/ffi.rs` | C-compatible type definitions (structs, enums, fn pointer typedefs) |
| `libretro_runner/src/core.rs` | Loads the `.so`, runs the init sequence, exposes `run_frame()` / `shutdown()` |
| `libretro_runner/src/callbacks.rs` | The six `extern "C"` callbacks the core calls back into, plus global state |
| `libretro_runner/src/frame_buffer.rs` | Converts core pixel formats to Cairo ARgb32 |
| `libretro_runner/src/audio.rs` | Opens the cpal audio device and owns the sample ring buffer |
| `libretro_runner/src/input.rs` | Joypad button constants and `InputState` bitfield |
| `relm4-ui/src/libretro/window.rs` | GTK4 game window, glib game loop, Cairo rendering |
| `relm4-ui/src/libretro/input.rs` | Maps GTK key events to libretro joypad button IDs |
| `service/src/libretro_runner/service.rs` | Prepares ROM files (download, extract) before launch |

---

## How it works — detailed walkthrough

### 1. Loading the library (`core.rs`)

`libloading` calls `dlopen()` under the hood. It returns `Symbol<T>` values that borrow from the `Library`. Because you cannot store a `Symbol` (it would borrow `lib` forever), you immediately dereference each one to copy the raw function pointer out:

```rust
let sym: Symbol<unsafe extern "C" fn()> = unsafe { lib.get(b"retro_init\0") }?;
let retro_init = *sym;  // copies the fn pointer, Symbol is then dropped
```

The `\0` at the end of the symbol name is the C null terminator — `dlsym()` expects a C string.

### 2. The init sequence (order is critical)

`LibretroCore::load()` follows this exact order:

1. **Register `environment_cb`** — the core needs it before `retro_init` is called, because it calls back immediately during init to query capabilities.
2. **Install global state** — `CoreCallbackState` must be in the global static before any callback fires (see below).
3. **Call `retro_init`** — wakes the core up; triggers `environment_cb` calls.
4. **Register the remaining callbacks** — video, audio, input.
5. **Call `retro_load_game`** — the core loads the ROM.
6. **Call `retro_get_system_av_info`** — returns the authoritative resolution and sample rate now that a game is loaded.

### 3. The global state problem (`callbacks.rs`)

C callbacks are free functions — they have no `self`, no closure environment, no way to carry context. The standard solution is a **process-wide global**:

```rust
static CORE_STATE: OnceLock<Mutex<Option<CoreCallbackState>>> = OnceLock::new();
```

- `OnceLock` — initialised exactly once (on first access), never re-initialised.
- `Mutex` — protects against concurrent access (GTK thread vs audio thread).
- `Option` — `Some` while a core is loaded, `None` after shutdown. This lets the same static be reused across multiple load/unload cycles.

Every callback calls `with_state(|s| { ... })` to borrow the state for the duration of that call.

`CoreCallbackState` holds:

| Field | Type | Purpose |
|---|---|---|
| `frame_buffer` | `Arc<Mutex<FrameBuffer>>` | Shared with the GTK draw callback |
| `input_state` | `Arc<Mutex<InputState>>` | Shared with the GTK key event handler |
| `pixel_format` | `RetroPixelFormat` | Set by the core via `SET_PIXEL_FORMAT` |
| `system_directory` | `CString` | Path handed to the core for BIOS files |
| `audio_buffer` | `Arc<Mutex<VecDeque<f32>>>` | Shared with the cpal audio thread |

### 4. The environment callback

`environment_cb(cmd, data)` is the main negotiation channel. The core calls it with a command ID and a type-erased `*mut c_void` pointer whose meaning depends on the command. The frontend returns `true` to accept or `false` to decline.

| Command | What the core wants | What we do |
|---|---|---|
| `GET_SYSTEM_DIRECTORY` (9) | Path to BIOS/system files | Write our `system_directory` CString pointer into `data` |
| `SET_PIXEL_FORMAT` (10) | Tell us which pixel format it will use | Accept all three formats; store the choice in `pixel_format` |
| `GET_LOG_INTERFACE` (27) | A logging function pointer | Return `false` (variadic C fn requires Rust nightly) |
| `GET_VARIABLE` (15) | Value of a named core option | Return `false` (core uses its defaults) |
| `SET_GEOMETRY` (37) | Output resolution changed | Resize the frame buffer to match |

Any unrecognised command returns `false`. Cores are required by the libretro spec to handle this gracefully.

### 5. Pixel format conversion (`frame_buffer.rs`)

The core renders pixels in one of three formats. Cairo wants `ARgb32`: four bytes per pixel, in `[B, G, R, A]` order in memory on little-endian x86.

The `pitch` parameter in `video_refresh_cb` is the number of **bytes** per row in the source buffer. It may be wider than `width × bytes_per_pixel` because cores sometimes pad rows for memory alignment. You must use `pitch` as the row stride when reading from the source:

```rust
let source_offset = row * pitch + col * bytes_per_pixel;
```

If you used `width × bytes_per_pixel` as the stride instead, padded rows would cause the image to appear horizontally smeared or doubled.

### 6. Audio (`audio.rs` + `callbacks.rs`)

#### What is cpal?

**cpal** (Cross-Platform Audio Library) is a Rust crate that provides a single API for audio output regardless of which audio system the OS uses. On Linux it talks to ALSA or PulseAudio (or PipeWire via its PulseAudio compatibility layer); on Windows it would use WASAPI; on macOS CoreAudio. You do not need to handle these differences — cpal abstracts them away.

We use it to open the default output device and start a stream. When the OS audio driver needs more samples it fires our callback to fill a buffer. That callback drains the shared ring buffer that the libretro audio callbacks are pushing into.

Audio samples are represented as `f32` values in the range `−1.0..=1.0` (standard for modern audio APIs). The libretro core provides `i16` samples (integer range −32768..32767), so we convert on push: `s as f32 / 32768.0`.

#### The threading problem

Audio has a threading mismatch:
- The libretro callbacks run on the **GTK main thread** (inside `retro_run()`).
- The cpal output callback runs on a **private audio thread** managed by the OS.

The bridge is an `Arc<Mutex<VecDeque<f32>>>`:

```
GTK thread:   retro_run() → audio_sample_batch_cb() → pushes f32 samples → VecDeque
Audio thread:                                 cpal callback ← pops f32 samples ← VecDeque
                                                                     ↓
                                                                  speakers
```

`cpal::Stream` cannot live in the global static because it contains raw pointers that are not `Sync`. So only the buffer (plain data, fully thread-safe) lives in `CoreCallbackState`. The `AudioOutput` struct (which holds the stream) lives in `LibretroCore` — it only needs to stay alive, not be shared globally.

The `AudioOutput` is created after `retro_get_system_av_info` so the stream is opened at the core's actual sample rate (typically 44100 Hz for NES cores). If no audio device is available, the failure is non-fatal — the game runs silently.

### 7. The game loop (`window.rs`)

`glib::timeout_add_local` fires a closure on the GTK main thread every N milliseconds. The interval is derived from the core's reported FPS:

```rust
let interval = Duration::from_secs_f64(1.0 / core.fps);
glib::timeout_add_local(interval, move || {
    core.run_frame();          // calls retro_run() → all callbacks fire
    drawing_area.queue_draw(); // schedules a repaint
    glib::ControlFlow::Continue
});
```

The draw callback locks the frame buffer, wraps the pixel data in a `cairo::ImageSurface`, scales it to fill the window while preserving aspect ratio, and blits it.

### 8. Input (`input.rs` + `window.rs`)

GTK fires key events with `gdk::Key` values. `map_key_event()` translates them to libretro joypad button IDs and sets flags in `InputState`. When the core calls `input_state_cb()` asking "is button X pressed on port 0?", the callback reads those flags.

Default key bindings:

| Key | Joypad button |
|---|---|
| Arrow keys | D-pad |
| Z | B |
| X | A |
| A | Y |
| S | X |
| Q | L shoulder |
| W | R shoulder |
| Enter | Start |
| Backspace | Select |

### 9. Full data flow

```
Key press
  → GTK key event
  → map_key_event()
  → InputState (Arc<Mutex>)
        ↓ read by
  input_state_cb() during retro_run()

glib timer fires
  → retro_run()
      → video_refresh_cb() → FrameBuffer (Arc<Mutex>)
      → audio_sample_batch_cb() → VecDeque<f32> (Arc<Mutex>)
                                       ↓ drained by
                               cpal audio thread → speakers

GTK repaint
  → draw callback
  → reads FrameBuffer
  → Cairo ImageSurface
  → screen
```

---

## Adding a new core

### Step 1: Install the core

Cores are standard `.so` files. Install to a known path:

```bash
# From your distro (if packaged):
sudo apt install libretro-mgba        # Game Boy Advance
sudo apt install libretro-nestopia    # NES

# Via RetroArch's online updater:
# Online Updater → Core Downloader → pick a core
# Installs to ~/.config/retroarch/cores/

# Build from source (example for fceumm):
git clone https://github.com/libretro/libretro-fceumm
cd libretro-fceumm && make
sudo cp fceumm_libretro.so /usr/lib/libretro/
```

### Step 2: Check the core's pixel format support

Most modern cores support `XRGB8888`. Some older ones only output `RGB565`. The `frame_buffer.rs` handles all three formats automatically — you do not need to change anything.

To confirm what format a core uses, run the app with `RUST_LOG=debug` and look for `SET_PIXEL_FORMAT` in the logs (once logging is wired up), or just try it — the image will be correct if the format is handled.

### Step 3: Check `need_fullpath`

Some cores want the ROM path (they open the file themselves); others want the file loaded into memory. `LibretroCore::load()` handles both cases automatically by checking `retro_get_system_info().need_fullpath`.

You do not need to change anything for this.

### Step 4: Wire the core path into the UI

Currently the core path is hardcoded in `relm4-ui/src/release.rs`:

```rust
core_path: PathBuf::from("/usr/lib/libretro/fceumm_libretro.so"),
```

To add a new core, either:

**Option A — Change the hardcoded path** (quick, single-core):
```rust
core_path: PathBuf::from("/usr/lib/libretro/mgba_libretro.so"),
```

**Option B — Per-system core mapping** (proper solution, future work):
Add a `libretro_core_path` column to the `systems` table so each system has its own configured core. Then look up the core path from the release's system when launching.

### Step 5: Handle BIOS files (if required)

Some cores need BIOS files (e.g. PlayStation needs `scph1001.bin`). The core requests the system directory via `GET_SYSTEM_DIRECTORY` and looks for BIOS files there.

The system directory is currently set to the OS temp directory in `LibretroRunnerService::prepare_rom()`. Change it to a real directory containing BIOS files:

```rust
// service/src/libretro_runner/service.rs
let system_dir = PathBuf::from("/home/user/.config/retroarch/system");
```

### Step 6: Test

The integration tests in `libretro_runner/tests/core_tests.rs` are marked `#[ignore]` (they require a real ROM file). Run them with:

```bash
TEST_NES_ROM=/path/to/rom.nes cargo test -p libretro_runner -- --ignored --test-threads=1
```

`--test-threads=1` is required because the libretro global state only supports one core at a time.

---

## Troubleshooting

**`dlopen failed`**
The `.so` file was not found at the given path, or one of its own dependencies is missing. Check:
```bash
ls -la /path/to/core_libretro.so   # does it exist?
ldd /path/to/core_libretro.so      # are all dependencies found?
```

**Image is doubled horizontally / wrong colors**
The pixel format in `CoreCallbackState` does not match what the core is actually sending. This was a historical bug — the fix is to accept all pixel formats in `environment_cb` (do not reject `RGB565` or `RGB1555`). See `callbacks.rs` `SET_PIXEL_FORMAT` handling.

**No audio**
Check that a default audio output device exists:
```bash
pactl info | grep "Default Sink"
aplay -l
```
Audio failure is non-fatal — the game runs silently and a warning is logged.

**Game runs too fast or too slow**
The game loop interval is calculated from `av_info.timing.fps`. If the timer fires at the wrong rate, verify that `retro_get_system_av_info` is returning a sane value and that the `Duration` calculation in `window.rs` `start_game_loop()` is correct.

**Core crashes the app**
The core runs in-process. A segfault in the core will kill the whole app. Check:
- The ROM file is not corrupted
- The core and ROM are for the same system
- `need_fullpath` handling is correct for this core (some cores crash if given a data pointer when they expected a path)
