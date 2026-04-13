## Phase 3 — Implementation

### Core Metadata
- [ ] T1 [libretro_runner] — Replace the raw supported-core string list with structured metadata and add `freeintv_libretro`
  **File:** `libretro_runner/src/supported_cores.rs`
  Define core metadata that can express supported extensions and required firmware files, while keeping existing consumers simple.

### Settings Plumbing
- [ ] T2 [core_types] — Add a setting key for the libretro system directory
  **File:** `core_types/src/lib.rs`
  Extend `SettingName` so the system/firmware directory can be persisted like the existing libretro core directory.

- [ ] T3 [service] — Load and save the libretro system directory in settings models
  **File:** `service/src/view_models.rs`, `service/src/settings_service.rs`
  Thread the new setting through `Settings` and `SettingsSaveModel`.

### Input Model
- [ ] T4 [libretro_runner] — Extend input state and callbacks for analog-capable libretro input
  **File:** `libretro_runner/src/input.rs`, `libretro_runner/src/callbacks.rs`, `libretro_runner/src/ffi.rs`
  Keep the runner generic, but add the state needed to answer FreeIntv disc/keypad-related input requests.

### Launch Preflight
- [ ] T5 [service] — Add libretro core preflight validation for firmware and file extensions
  **File:** `service/src/libretro_runner/service.rs`
  Validate selected core metadata, ROM extension, and required firmware presence before launch.

- [ ] T6 [service] — Pass the configured libretro system directory to the runner
  **File:** `service/src/libretro_runner/service.rs`
  Stop using `temp_output_dir` as the libretro system directory.

### GUI
- [ ] T7 [relm4-ui] — Add libretro system directory controls to settings
  **File:** `relm4-ui/src/settings_form.rs`
  Add browse/select UI alongside the existing libretro core directory controls.

- [ ] T8 [relm4-ui] — Add keyboard-first FreeIntv keypad/disc/controller-swap controls in the libretro frontend
  **File:** `relm4-ui/src/libretro/input.rs`, `relm4-ui/src/libretro/window.rs`
  Feed the richer runner input state from the chosen FreeIntv control UX.

- [ ] T9 [relm4-ui] — Surface preflight failures with actionable messages
  **File:** `relm4-ui/src/libretro/runner.rs`
  Keep failures in the existing error-dialog flow rather than silent launch failure.

### Documentation
- [ ] T10 [docs] — Document FreeIntv setup and firmware requirements
  **File:** `docs/LIBRETRO_INTEGRATION.md`, `README.md`
  Update the onboarding docs to describe FreeIntv support, required firmware, the final chosen control scheme, and remove stale hardcoded-core-path guidance.

## Phase 5 — Tests

- [ ] T11 [libretro_runner] — Add tests for FreeIntv core metadata and input state
  **File:** `libretro_runner/src/supported_cores.rs`, `libretro_runner/src/input.rs`
  Cover supported extensions, required firmware declarations, and the extended input model.

- [ ] T12 [service] — Add tests for settings persistence and FreeIntv preflight validation
  **File:** `service/src/settings_service.rs`, `service/src/libretro_runner/service.rs`
  Cover missing system directory, missing firmware, unsupported extension, and happy-path launch preparation.

## Manual Verification Checklist

- [ ] Configure libretro core directory and libretro system directory in Settings.
- [ ] Place `freeintv_libretro.so` in the core directory and `exec.bin` / `grom.bin` in the system directory.
- [ ] Map `freeintv_libretro` to the Intellivision system.
- [ ] Launch a `.int`, `.rom`, or `.bin` Intellivision file successfully.
- [ ] Verify keypad, disc, and controller-swap controls work through the keyboard-first frontend UX.
- [ ] Confirm missing firmware produces a clear error dialog before the launch window opens.
- [ ] Confirm an existing NES/FCEUmm launch still works after the change.
