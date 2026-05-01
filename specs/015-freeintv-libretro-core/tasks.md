## Phase 3 — Implementation

### Core Metadata
- [x] T1 [libretro_runner] — Replace the raw supported-core string list with structured app-policy metadata and add `freeintv_libretro`
  **File:** `libretro_runner/src/supported_cores.rs`
  Replace the raw supported-core string list with structured app-policy metadata while keeping existing consumers simple. `SUPPORTED_CORES` now defines supported core names and app-owned policy such as `InputProfile`, while supported extensions and firmware requirements continue to come from parsed `.info` data separately.

### Settings Plumbing
- [x] T2 [core_types] — Add a setting key for the libretro system directory
  **File:** `core_types/src/lib.rs`
  Extend `SettingName` so the system/firmware directory can be persisted like the existing libretro core directory.

- [x] T3 [service] — Load and save the libretro system directory in settings models
  **File:** `service/src/view_models.rs`, `service/src/settings_service.rs`
  Thread the new setting through `Settings` and `SettingsSaveModel`.

### Input Model
- [x] T4 [libretro_runner] — Extend input state and callbacks for joypad + analog libretro input
  **File:** `libretro_runner/src/input.rs`, `libretro_runner/src/callbacks.rs`, `libretro_runner/src/ffi.rs`
  Keep the runner generic, but add shared state and callback handling for both digital joypad reads and libretro analog-axis reads. Document the runner-side contract clearly: digital buttons answer `RETRO_DEVICE_JOYPAD`, analog axes answer `RETRO_DEVICE_ANALOG`.

### Launch Preflight
- [x] T5 [service] — Add libretro core preflight validation for firmware and file extensions
  **File:** `service/src/libretro_runner/service.rs`
  `prepare_rom()` now delegates to a typed preflight pipeline in `service/src/libretro_runner/prepare/` that downloads the file set, validates the selected file, validates required firmware, validates supported extensions, and builds launch paths using the configured system directory. Failures now return `LibretroPreflightError`.

- [x] T6 [service] — Pass the configured libretro system directory to the runner
  **File:** `service/src/libretro_runner/service.rs`
  Stop using `temp_output_dir` as the libretro system directory.

### GUI
- [x] T7 [relm4-ui] — Add libretro system directory controls to settings
  **File:** `relm4-ui/src/settings_form.rs`
  Add browse/select UI alongside the existing libretro core directory controls.

- [x] T8 [relm4-ui] — Add physical joypad/analog-stick capture in the libretro frontend
  **File:** `relm4-ui/Cargo.toml`, `relm4-ui/src/libretro/input.rs`, `relm4-ui/src/libretro/window.rs`
  Add a dedicated controller input backend in `relm4-ui` (implemented with `gilrs`) so physical controller buttons and analog-stick motion feed the richer runner input state without relying on GTK event controllers for joypad polling.

- [x] T9 [relm4-ui] — Surface preflight failures with actionable messages
  **File:** `relm4-ui/src/libretro/runner.rs`
  Keep failures in the existing error-dialog flow rather than silent launch failure. `relm4-ui/src/libretro/runner.rs` now allows launch attempts when a core/file is selected and surfaces launch preparation, system-info, and core-path failures through `show_error_dialog(e.to_string(), root)`, so typed `LibretroPreflightError` messages reach the user instead of generic launch-failure text.

### Documentation
- [x] T10 [docs] — Document current libretro setup flow
  **File:** `docs/LIBRETRO_INTEGRATION.md`, `README.md`
  The docs now describe `freeintv_libretro` support through the existing per-system core mapping flow, the configured libretro core/system directory setup, metadata-driven validation via `.info` files, the current generic libretro / Retropad-style controller behavior, and the current typed preflight/error-dialog flow. Stale hardcoded-core-path and preflight-status wording has been removed.

## Phase 5 — Tests

- [x] T11 [libretro_runner] — Add tests for supported-core metadata and digital/analog input state
  **File:** `libretro_runner/src/supported_cores.rs`, `libretro_runner/src/libretro_info_parser.rs`, `libretro_runner/src/input.rs`, `libretro_runner/src/callbacks.rs`
  `supported_cores.rs` now covers supported-core lookup, expected input profiles, the supported core list, and unknown-core handling. `libretro_info_parser.rs` covers `freeintv_libretro` parsed metadata including supported extensions and firmware declarations. `input.rs` and `callbacks.rs` cover digital button reads, analog axis reads, and callback device filtering.

- [ ] T12 [service] — Add higher-level tests for libretro launch preparation
  **File:** `service/src/libretro/runner/service.rs`
  Add service-level tests for `LibretroRunnerService::prepare_rom()` covering missing system directory, missing firmware, unsupported extension, invalid initial file, and happy-path launch preparation. `settings_service.rs` persistence coverage already exists, and `service/src/libretro/runner/prepare/steps.rs` already covers the individual preflight pipeline steps.

- [x] T13 [relm4-ui] — Add tests for controller-to-libretro input mapping
  **File:** `relm4-ui/src/libretro/input.rs`
  `relm4-ui/src/libretro/input.rs` now uses pure mapping helpers that are covered by tests for physical joypad button handling, analog-axis mapping, deadzone handling, and keyboard-to-libretro button mapping without needing a live libretro core.

## Manual Verification Checklist

- [x] Configure libretro core directory and libretro system directory in Settings.
- [x] Place `freeintv_libretro.so` in the core directory and `exec.bin` / `grom.bin` in the system directory.
- [x] Map `freeintv_libretro` to the Intellivision system.
- [x] Launch a `.int`, `.rom`, or `.bin` Intellivision file successfully.
- [ ] Verify generic physical joypad button and D-pad actions reach the core through the expected libretro joypad inputs.
- [ ] Verify left/right analog stick motion reaches the expected libretro analog X/Y state.
- [ ] Confirm missing firmware produces a clear error dialog before the launch window opens.
- [ ] Confirm an existing NES/FCEUmm launch still works after the change.
