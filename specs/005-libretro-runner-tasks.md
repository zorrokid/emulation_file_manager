# Tasks 005 — Libretro Core Runner

## Phase 1 — `libretro_runner` crate

### T1.1 — Create crate scaffold
- Add `libretro_runner/` directory with `Cargo.toml`
- Dependencies: `libloading = "0.9"`, `thiserror = "2"`, `tracing = "0.1"`, `cpal = "0.15"`
- Add to workspace `Cargo.toml` members

### T1.2 — `src/error.rs`
```rust
pub enum LibretroError {
    LibraryLoad(libloading::Error),
    GameLoad(String),
    NotInitialized,
}
```

### T1.3 — `src/ffi.rs`

`RETRO_ENVIRONMENT_*` constants — sent by the core to the frontend via `environment_cb(cmd, data)`:

| Constant | ID | Direction | What to do |
|---|---|---|---|
| `GET_SYSTEM_DIRECTORY` | 9 | core asks frontend | Write a path pointer into `data` (temp dir) |
| `SET_PIXEL_FORMAT` | 10 | core tells frontend | Read pixel format from `data`, return `true` to accept XRGB8888 |
| `GET_VARIABLE` | 15 | core asks frontend | Return `false` — no core options supported yet |
| `GET_LOG_INTERFACE` | 27 | core asks frontend | Write a logging fn pointer into `data` |
| `SET_GEOMETRY` | 37 | core tells frontend | Read new dimensions from `data`, update frame buffer |
| Everything else | — | — | Return `false` — cores handle unsupported commands gracefully |

Reference: `libretro.h` in the RetroArch repo has all constants and struct definitions with comments.

- `RetroPixelFormat` repr(u32) enum
- `RetroGameInfo`, `RetroSystemInfo`, `RetroSystemAvInfo`, `RetroGameGeometry`, `RetroSystemTiming` repr(C) structs
- Callback function pointer typedefs

### T1.4 — `src/frame_buffer.rs`
- `FrameBuffer { width: u32, height: u32, rgba_data: Vec<u8>, dirty: bool }`
- `update(data, width, height, pitch, format)` — converts XRGB8888 → RGBA8888 (set alpha to 0xFF)

### T1.5 — `src/input.rs`
- Joypad ID constants (`JOYPAD_A` = 8, `JOYPAD_B` = 0, `JOYPAD_UP` = 4, etc.)
- `InputState { buttons: [bool; 16] }`
- `set_button(id, pressed)`, `get_button(id) -> bool`

### T1.6 — `src/callbacks.rs`
- `CoreCallbackState { frame_buffer, input_state, pixel_format, system_directory: CString, audio_buffer: Arc<Mutex<VecDeque<f32>>> }`
- `static CORE_STATE: OnceLock<Mutex<Option<CoreCallbackState>>>`
- `install_state()`, `remove_state()`, `with_state()`
- `extern "C"` implementations: `environment_cb`, `video_refresh_cb`, `audio_sample_cb`, `audio_sample_batch_cb`, `input_poll_cb`, `input_state_cb`
- `audio_buffer` is the shared ring buffer between the libretro audio callbacks and the cpal audio thread; only the buffer (not the cpal `Stream`) lives here because `cpal::Stream` is not `Sync` and cannot be stored in a `static`

### T1.7 — `src/core.rs`
- `LibretroCore` struct with `_library: Library`, `retro_run`, `retro_unload_game`, `retro_deinit`, `frame_buffer`, `input_state`, `_audio_output: Option<AudioOutput>`, `fps`
- `_audio_output` keeps the cpal stream alive for the duration of the session; `None` if no audio device was available at load time
- `load(core_path, rom_path, system_dir) -> Result<Self, LibretroError>` — init sequence (order is critical):
  1. `retro_set_environment(environment_cb)` — register callback before anything else
  2. Install `CORE_STATE` into the global — **must happen before `retro_init`**, which triggers callbacks immediately
  3. `retro_init()` — core wakes up and calls `environment_cb` to query system dir, pixel format, log interface
  4. `retro_set_video_refresh / audio_sample / audio_sample_batch / input_poll / input_state` — register remaining callbacks
  5. `retro_get_system_info()` — check `need_fullpath`; fceumm sets this `true` (pass file path, not ROM data)
  6. `retro_load_game(&RetroGameInfo { path: rom_path, data: null, size: 0 })` — more `environment_cb` calls may occur here (e.g. `SET_GEOMETRY`)
  7. `retro_get_system_av_info()` — read final output geometry and `fps` (NTSC NES ≈ 60.0988); size the frame buffer here
- `run_frame(&self)`
- `shutdown(self)`

### T1.8 — `src/audio.rs` (added during implementation)
- `AudioOutput` struct: opens the default cpal output device at the core's sample rate, owns the cpal `Stream` and shares the `Arc<Mutex<VecDeque<f32>>>` buffer with `CoreCallbackState`
- Created after `retro_get_system_av_info` so the stream uses the correct sample rate

### T1.9 — `src/lib.rs`
- Wire up all modules including `pub mod audio`

### Test cases (automated, `libretro_runner/tests/`)

- **T1.T1** `test_load_and_run_frames` — given a real `fceumm_libretro.so` and a NES ROM, `LibretroCore::load` succeeds and calling `run_frame()` 60 times sets `frame_buffer.dirty = true`. Mark `#[ignore]` if the `.so` isn't available in CI.
- **T1.T2** `test_frame_buffer_dimensions` — after `run_frame()`, frame buffer `width` and `height` are non-zero.
- **T1.T3** `test_shutdown_clears_state` — after `shutdown()`, `CORE_STATE` holds `None`.

---

## Phase 2 — GTK4 Game Window

### T2.1 — `relm4-ui/src/libretro/input.rs`
- `map_key_to_joypad(keyval: gtk::gdk::Key, input_state: &Arc<Mutex<InputState>>, pressed: bool)`
- Mapping:

| GTK Key | Libretro button |
|---|---|
| Up / Down / Left / Right | JOYPAD_UP/DOWN/LEFT/RIGHT |
| Z | JOYPAD_B |
| X | JOYPAD_A |
| A | JOYPAD_Y |
| S | JOYPAD_X |
| Q | JOYPAD_L (left shoulder) |
| W | JOYPAD_R (right shoulder) |
| Return | JOYPAD_START |
| BackSpace | JOYPAD_SELECT |

### T2.2 — `relm4-ui/src/libretro/window.rs`
- `LibretroWindowModel` with `Arc<Mutex<Option<LibretroCore>>>`, `drawing_area`, `timer_source_id`
- Messages: `Launch { core_path, rom_path, system_dir }`, `Close`
- Output: `LibretroWindowOutput::Error(String)`
- `view!` macro: `gtk::Window` > `gtk::DrawingArea`
- `start_game_loop(fps)` — `glib::timeout_add_local` calling `core.run_frame()` then `drawing_area.queue_draw()`
- `stop_game_loop()` — removes the glib source
- `set_draw_func` — cairo paint from frame buffer, scaled to fit, `Operator::Source`
- `EventControllerKey` on window → `input::map_key_to_joypad(...)`

### T2.3 — `relm4-ui/src/libretro/mod.rs`
- `pub mod input;`
- `pub mod window;`
- Re-export `LibretroWindowModel`, `LibretroWindowMsg`, `LibretroWindowOutput`

### T2.4 — Wire into `relm4-ui/src/main.rs`
- `mod libretro;`

### T2.5 — Update `relm4-ui/Cargo.toml`
- Add `libretro_runner = { path = "../libretro_runner" }`

### Manual verification checklist — Phase 2
- [x] Game window opens when `Launch` is emitted programmatically
- [x] Game renders at visible frame rate (not frozen)
- [x] Pressing arrow keys moves the character / navigates menus
- [x] Z and X buttons respond (A/B)
- [x] Closing the window hides it without crash
- [x] Opening a second time after closing works

---

## Phase 3 — `LibretroRunnerService`

### T3.1 — `service/src/libretro_runner/service.rs`
- `LibretroLaunchModel { file_set_id: i64, initial_file: Option<String>, core_path: PathBuf }`
- `LibretroLaunchPaths { rom_path, core_path, system_dir, temp_files: Vec<String> }`
- `LibretroRunnerService::new(settings, download_service)`
- `async fn prepare_rom(model) -> Result<LibretroLaunchPaths, Error>` — download + extract, resolve ROM path
- `fn cleanup(paths)` — remove temp files

### T3.2 — `service/src/libretro_runner/mod.rs`
- `pub mod service;`

### T3.3 — Register in `service/src/lib.rs`
- `pub mod libretro_runner;`

### T3.4 — Add to `service/src/app_services.rs`
- Field: `libretro_runner: OnceLock<Arc<LibretroRunnerService>>`
- Accessor: `pub fn libretro_runner(&self) -> &Arc<LibretroRunnerService>`

### Test cases (automated)

- **T3.T1** `test_prepare_rom_returns_path` — given a file set with a downloaded file, `prepare_rom` returns a `LibretroLaunchPaths` where `rom_path` exists on disk.
- **T3.T2** `test_cleanup_removes_temp_files` — after `cleanup()`, the temp file no longer exists.

---

## Phase 4 — UI Wiring

### T4.1 — Add controller to `ReleaseModel`
- Field: `libretro_window: Controller<LibretroWindowModel>`
- Init in `init()`: `LibretroWindowModel::builder().launch(()).forward(...)`

### T4.2 — Add messages to `ReleaseMsg` / command output
- `ReleaseMsg::StartLibretroRunner`
- `ReleaseCommandMsg::LibretroRomPrepared(Result<LibretroLaunchPaths, Error>)`

### T4.3 — Add button to view macro
```rust
gtk::Button {
    set_label: "Run with Libretro",
    #[watch]
    set_sensitive: model.selected_file_set.is_some(),
    connect_clicked => ReleaseMsg::StartLibretroRunner,
}
```

### T4.4 — Handle `StartLibretroRunner` in `update()`
- Spawn `oneshot_command` calling `app_services.libretro_runner().prepare_rom(...)`
- Hardcode core path: `/usr/lib/libretro/fceumm_libretro.so`

### T4.5 — Handle `LibretroRomPrepared` in `update_cmd()`
- On `Ok(paths)`: emit `LibretroWindowMsg::Launch { ... }`
- On `Err(e)`: forward to existing error display

### Manual verification checklist — Phase 4
- [x] "Run with Libretro" button is visible on a release with a file set
- [x] Button is greyed out when no file set is selected
- [x] Clicking the button opens the game window
- [x] Game plays correctly (see Phase 2 checklist)
- [x] Closing game window then clicking the button again works
- [x] "Run with Emulator" still works normally
- [x] Error toast appears if core `.so` is not found

---

## Phase 5 — Post-review fixes

Findings from architectural review after initial implementation. Ordered by severity.

### T5.1 — Fix temp file leak (bug)

`LibretroRunnerService::cleanup()` is never called. The `temp_files` field of `LibretroLaunchPaths` is silently dropped in `release.rs` when emitting `LibretroWindowMsg::Launch`.

Fix:
- Add `temp_files: Vec<String>` to `LibretroWindowMsg::Launch`
- Pass `paths.temp_files` through when emitting the message in `release.rs`
- In `window.rs`, store `temp_files` in `LibretroWindowModel`
- On `LibretroWindowMsg::Close`, emit a new `LibretroWindowOutput::SessionEnded(temp_files)` to the parent
- In `release.rs`, handle `SessionEnded` by calling `app_services.libretro_runner().cleanup(...)`

**Test case:** After closing the game window, verify the extracted ROM file no longer exists in the temp directory.

### T5.2 — Move `LibretroCore::load()` off the GTK main thread (architecture + UX)

`LibretroCore::load()` is called directly inside `window.rs` `update()`, which runs on the GTK main thread. `dlopen`, file I/O, and `retro_init` all block the UI while running.

Fix:
- Add `LibretroWindowCmdMsg` enum with a `CoreLoaded(Result<LibretroCore, LibretroError>)` variant
- Change `type CommandOutput = ()` to `type CommandOutput = LibretroWindowCmdMsg`
- In `update()`, handle `Launch` by spawning a `oneshot_command` that calls `LibretroCore::load()`
- Move the draw setup, input setup, and game loop start into `update_cmd()` on `CoreLoaded(Ok(core))`
- On `CoreLoaded(Err(e))`, emit `LibretroWindowOutput::Error`

### T5.3 — Fix input controller accumulation across sessions (bug)

`setup_input()` calls `root.add_controller()` on every `Launch`. After closing and re-opening, the window accumulates multiple controllers all writing to the same `InputState`. This means duplicate key events and potential ghost inputs.

Fix: Store the `EventControllerKey` in `LibretroWindowModel` and create it once in `init()` rather than on each `Launch`. Wire the key handler to update the `input_state` stored in the model, updating which `Arc<Mutex<InputState>>` it reads from on each launch.

**Manual verification:** Open a game, close it, open a second game — confirm no duplicate/ghost inputs.

### T5.4 — Implement missing key mappings (missing feature)

The spec and documentation list A→Y, S→X, Q→L, W→R bindings, but `relm4-ui/src/libretro/input.rs` does not implement them. `JOYPAD_Y`, `JOYPAD_X`, `JOYPAD_L`, `JOYPAD_R` constants exist in `libretro_runner` but are not imported or mapped.

Fix: Add the four missing arms to `keyval_to_button()` in `relm4-ui/src/libretro/input.rs`:
```rust
gtk::gdk::Key::a | gtk::gdk::Key::A => Some(JOYPAD_Y),
gtk::gdk::Key::s | gtk::gdk::Key::S => Some(JOYPAD_X),
gtk::gdk::Key::q | gtk::gdk::Key::Q => Some(JOYPAD_L),
gtk::gdk::Key::w | gtk::gdk::Key::W => Some(JOYPAD_R),
```

### T5.5 — Use a typed error in `AudioOutput::new()` (minor)

`AudioOutput::new()` returns `Result<Self, String>`. All other crates use typed `thiserror` enums. A bare `String` loses structure and cannot be matched on by callers.

Fix: Add an `AudioError` variant to `LibretroError` (or a dedicated `AudioError` enum) and return `Result<Self, LibretroError>` from `AudioOutput::new()`.

### T5.6 — Remove stale comment in `ffi.rs` (minor)

`RetroSystemTiming.sample_rate` has the comment `// audio sample rate — unused for now`. Audio is now implemented and `sample_rate` is passed to `AudioOutput::new()`.

Fix: Update the comment to `// audio sample rate — used to configure the cpal output stream`.

### T5.7 — Remove unused `LibretroError::NotInitialized` variant (minor)

`NotInitialized` is defined in `error.rs` but never constructed anywhere. The current ownership model (consuming `self` in `shutdown()`) prevents calling `run_frame()` after shutdown, so a runtime guard is not needed.

Fix: Remove the `NotInitialized` variant from `LibretroError`.

### T5.8 — Restrict visibility of `AudioOutput::sample_buffer` (minor)

`AudioOutput::sample_buffer` is `pub` but is never accessed from outside `audio.rs`. The sharing pattern uses the `Arc` clone passed into `AudioOutput::new()`, not the field on the struct after construction.

Fix: Change `pub sample_buffer` to `pub(crate) sample_buffer` or remove the field visibility entirely and only retain it as a keep-alive.

### T5.9 — Add `#[derive(Debug)]` to `LibretroLaunchModel` (minor)

`LibretroLaunchPaths` derives `Debug` but the sibling `LibretroLaunchModel` does not. Minor inconsistency in the service module.

Fix: Add `#[derive(Debug)]` to `LibretroLaunchModel` in `service/src/libretro_runner/service.rs`.

### Manual verification checklist — Phase 5
- [ ] Temp directory is empty after closing the game window
- [ ] UI does not freeze while the core is loading
- [ ] No ghost/duplicate inputs after closing and re-opening the game window
- [ ] A, S, Q, W keys produce the correct Y, X, L, R inputs in-game

---

## Implementation order

1. T1.1 → T1.8 (crate builds with `cargo build -p libretro_runner`)
2. T2.1 → T2.5 (Phase 1 tests pass; Phase 2 manual checks done)
3. T3.1 → T3.4 (Phase 3 tests pass)
4. T4.1 → T4.5 (full manual verification)
5. `cargo build` clean, `cargo test` green
