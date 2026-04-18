## Phase 3 — Implementation

### Core Metadata
- [ ] T1 [libretro_runner] — Replace the raw supported-core string list with structured metadata and add `freeintv_libretro`
  **File:** `libretro_runner/src/supported_cores.rs`
  Define core metadata that can express supported extensions and required firmware files, while keeping existing consumers simple. Partial progress: `freeintv_libretro` is now in `SUPPORTED_CORES`, and `.info` parsing groundwork was added separately, but the metadata refactor is not wired into supported-core definitions yet.

### Settings Plumbing
- [x] T2 [core_types] — Add a setting key for the libretro system directory
  **File:** `core_types/src/lib.rs`
  Extend `SettingName` so the system/firmware directory can be persisted like the existing libretro core directory.

- [x] T3 [service] — Load and save the libretro system directory in settings models
  **File:** `service/src/view_models.rs`, `service/src/settings_service.rs`
  Thread the new setting through `Settings` and `SettingsSaveModel`.

### Input Model
- [ ] T4 [libretro_runner] — Extend input state and callbacks for joypad + analog libretro input
  **File:** `libretro_runner/src/input.rs`, `libretro_runner/src/callbacks.rs`, `libretro_runner/src/ffi.rs`
  Keep the runner generic, but add shared state and callback handling for both digital joypad reads and libretro analog-axis reads needed by controller-driven disc input. Document the runner-side contract clearly: digital buttons answer `RETRO_DEVICE_JOYPAD`, analog axes answer `RETRO_DEVICE_ANALOG`. Partial progress: shared digital + analog state and callback support are implemented; any remaining work here is documentation/tests only.

### Launch Preflight
- [ ] T5 [service] — Add libretro core preflight validation for firmware and file extensions
  **File:** `service/src/libretro_runner/service.rs`
  Validate selected core metadata, ROM extension, and required firmware presence before launch.

- [x] T6 [service] — Pass the configured libretro system directory to the runner
  **File:** `service/src/libretro_runner/service.rs`
  Stop using `temp_output_dir` as the libretro system directory.

### GUI
- [x] T7 [relm4-ui] — Add libretro system directory controls to settings
  **File:** `relm4-ui/src/settings_form.rs`
  Add browse/select UI alongside the existing libretro core directory controls.

- [ ] T8 [relm4-ui] — Add physical joypad/analog-stick capture and FreeIntv input profile in the libretro frontend
  **File:** `relm4-ui/Cargo.toml`, `relm4-ui/src/libretro/input.rs`, `relm4-ui/src/libretro/window.rs`
  Add a dedicated controller input backend in `relm4-ui` (planned as `gilrs`) so physical controller buttons and analog-stick motion feed the richer runner input state, including the 16-way disc profile, without relying on GTK event controllers for joypad polling. Partial progress: `gilrs` polling and generic button/axis forwarding are implemented; the final FreeIntv keypad/controller-swap/disc profile is still pending.

- [ ] T9 [relm4-ui] — Surface preflight failures with actionable messages
  **File:** `relm4-ui/src/libretro/runner.rs`
  Keep failures in the existing error-dialog flow rather than silent launch failure.

### Documentation
- [ ] T10 [docs] — Document FreeIntv setup and firmware requirements
  **File:** `docs/LIBRETRO_INTEGRATION.md`, `README.md`
  Update the onboarding docs to describe FreeIntv support, required firmware, the final chosen controller scheme, how the frontend reads physical controller input, the analog-stick disc mapping, and remove stale hardcoded-core-path guidance. Partial progress: the integration doc now links to the upstream libretro API reference, but the FreeIntv-specific setup/control docs are still missing and one system-directory note is stale.

## Phase 5 — Tests

- [ ] T11 [libretro_runner] — Add tests for FreeIntv core metadata and digital/analog input state
  **File:** `libretro_runner/src/supported_cores.rs`, `libretro_runner/src/input.rs`, `libretro_runner/src/callbacks.rs`
  Cover supported extensions, required firmware declarations, digital button reads, analog axis reads, and device filtering in the callback path.

- [ ] T12 [service] — Add tests for settings persistence and FreeIntv preflight validation
  **File:** `service/src/settings_service.rs`, `service/src/libretro_runner/service.rs`
  Cover missing system directory, missing firmware, unsupported extension, and happy-path launch preparation.

- [ ] T13 [relm4-ui] — Add tests for controller-to-libretro input mapping
  **File:** `relm4-ui/src/libretro/input.rs`
  Extract or reuse pure mapping helpers so tests can cover controller-backend event translation, physical joypad button handling, and the 16-way disc-to-analog mapping without needing a live libretro core.

## Manual Verification Checklist

- [x] Configure libretro core directory and libretro system directory in Settings.
- [x] Place `freeintv_libretro.so` in the core directory and `exec.bin` / `grom.bin` in the system directory.
- [x] Map `freeintv_libretro` to the Intellivision system.
- [x] Launch a `.int`, `.rom`, or `.bin` Intellivision file successfully.
- [ ] Verify generic physical joypad button and D-pad actions reach the core through the expected libretro joypad inputs.
- [ ] Verify left/right analog stick motion reaches the expected libretro analog X/Y state.
- [ ] Verify the final FreeIntv keypad, disc, and controller-swap UX once that profile is implemented.
- [ ] Confirm missing firmware produces a clear error dialog before the launch window opens.
- [ ] Confirm an existing NES/FCEUmm launch still works after the change.
