# Spec 005 — Libretro Core Runner

## Background

The app currently launches emulators as external subprocesses. This feature adds a parallel path: running libretro cores (`.so` shared libraries) in-process, giving full control over video, input, and eventually save states. The existing external launcher is untouched.

## Goal

Run a NES libretro core from within the app, displaying output in a separate GTK4 window, with keyboard input and audio output.

## Requirements

### Functional

1. A "Run with Libretro" button appears on a release that has at least one file set.
2. Clicking the button downloads/extracts the ROM to a temp directory (reusing the existing file preparation infrastructure).
3. The app dynamically loads the libretro core `.so` file and starts the game loop.
4. The game renders in a new GTK4 window at approximately 60fps.
5. Keyboard input controls the emulated NES:
   - Arrow keys → D-pad
   - Z → B button
   - X → A button
   - Enter → Start
   - Backspace → Select
6. Closing the game window stops the core cleanly and removes temp files.

### Non-functional

- The existing external emulator launcher is unaffected.
- Audio output is provided via the system's default audio device (cpal). Failure to open the device is non-fatal — the game runs silently.
- No database schema changes are required.
- Only one libretro core may be active at a time.
- The core path is hardcoded for the MVP (default: `/usr/lib/libretro/fceumm_libretro.so`).

## Out of Scope

- Hardware rendering (OpenGL/Vulkan cores)
- Save states / rewind
- Core options UI
- Configurable key bindings
- Gamepad / controller input
- Core path configuration in the database or settings UI
- Multiple simultaneous cores
- In-app core downloading

## Architecture

Four new components, following the existing 4-layer architecture:

```
libretro_runner crate   (new, no GTK)
      ↓
LibretroRunnerService   (service crate)
      ↓
LibretroWindowModel     (relm4-ui)
      ↓
ReleaseModel            (relm4-ui, adds button)
```

### `libretro_runner` crate

Standalone crate (no GTK, no async) responsible for:
- Dynamically loading a `.so` via `libloading`
- Implementing the minimum libretro frontend callbacks
- Exposing `LibretroCore::load()`, `run_frame()`, `shutdown()`
- Holding the frame buffer and input state behind `Arc<Mutex<_>>`

The libretro callbacks are `extern "C"` free functions that access a process-wide `OnceLock<Mutex<Option<CoreCallbackState>>>`.

Minimum `environment_cb` commands implemented:

| Command | ID | Behaviour |
|---|---|---|
| `GET_SYSTEM_DIRECTORY` | 9 | Write temp dir path pointer into `data` |
| `SET_PIXEL_FORMAT` | 10 | Accept all three formats (XRGB8888, RGB565, RGB1555), store choice |
| `GET_VARIABLE` | 15 | Return `false` (no core options supported) |
| `GET_LOG_INTERFACE` | 27 | Return `false` (variadic C fn requires Rust nightly) |
| `SET_GEOMETRY` | 37 | Update frame buffer dimensions |
| Everything else | — | Return `false` |

Pixel format note: all three libretro pixel formats are accepted and converted to Cairo ARgb32 (`[B,G,R,A]`) in `frame_buffer.rs`. Accepting only XRGB8888 caused a duplication bug with cores (e.g. nestopia) that exclusively output RGB565 — they ignore the rejection and keep using their format, while our `pixel_format` field stays wrong.

### `LibretroRunnerService` (service crate)

- `prepare_rom(LibretroLaunchModel) -> Result<LibretroLaunchPaths>` — downloads/extracts ROM to temp dir using existing download service
- `cleanup(LibretroLaunchPaths)` — removes temp files after session

### `LibretroWindowModel` (relm4-ui)

A relm4 `Component` owning its own `gtk::Window` with a `gtk::DrawingArea`.

- Game loop: `glib::timeout_add_local` at `1000/fps` ms (main thread — same thread as `retro_init`)
- Core shared with the timeout closure via `Arc<Mutex<Option<LibretroCore>>>`
- Frame buffer painted via `cairo::ImageSurface::create_for_data`, scaled to fit window
- Keyboard input via `gtk::EventControllerKey`

### `ReleaseModel` (relm4-ui)

- Adds "Run with Libretro" button (sensitive when a file set is selected)
- On click: async ROM preparation via `LibretroRunnerService`, then emits `Launch` to `LibretroWindowModel`

## Acceptance Criteria

1. `cargo build` succeeds.
2. "Run with Libretro" button is visible on a release with a file set.
3. A NES game window opens and the game runs visibly.
4. Keyboard input controls the game.
5. Closing the window does not crash; temp files are removed.
6. The existing "Run with Emulator" flow is unaffected.
