# 015: FreeIntv Libretro Core Support

## Status
<!-- Planning | In Progress | Complete | Abandoned -->
Complete

## Affected Crates
- `libretro_runner` — supported-core metadata, FreeIntv definition, and richer input state
- `core_types` — persisted setting name for libretro system directory
- `service` — settings plumbing and launch preflight validation
- `relm4-ui` — settings UI, generic controller input, and launch error handling
- `docs` — FreeIntv setup notes and firmware requirements

## Problem

The application already had per-system libretro core mapping and in-process launching, but FreeIntv support exposed several missing pieces across metadata, settings, launch validation, controller input, documentation, and tests.

The app needed structured supported-core metadata instead of a raw allowlist, a dedicated libretro system directory setting for firmware, typed preflight validation for launch-file selection / firmware / supported extensions, and a generic frontend input path that could feed both digital joypad state and libretro analog axes.

The work also needed to stay generic rather than FreeIntv-specific. Physical controller input belonged in `relm4-ui` behind a dedicated gamepad backend instead of GTK event controllers, and the docs/tests needed to match the implemented system-directory flow, typed preflight errors, and generic Retropad-style controller behavior.

## Proposed Solution

Promote supported libretro cores from a string allowlist to structured app-policy metadata in `libretro_runner`. Add a `freeintv_libretro` definition that captures app-owned policy such as:

- core name
- input profile

Add a dedicated libretro system directory setting so the user can point the app at the directory containing libretro firmware files. Update the launch path to use that directory instead of `temp_output_dir` when passing `GET_SYSTEM_DIRECTORY` to the core.

Extend the input layer so `libretro_runner` can answer more than digital joypad buttons. The input state should support:

- joypad buttons
- analog axes exposed through libretro's standard analog input surface

`libretro_runner` should remain generic here: it owns a frontend-facing input state plus libretro device/axis query support, and `input_state_cb` should answer both `RETRO_DEVICE_JOYPAD` and `RETRO_DEVICE_ANALOG` from that shared state.

On the frontend side, extend `relm4-ui/src/libretro/input.rs` beyond keyboard-only handling so libretro sessions can feed physical joypad/analog-stick input into the shared state. The first implementation keeps the existing keyboard path where useful and adds physical joypad/analog-stick capture in `relm4-ui/src/libretro/input.rs`, routing that through the generic input state owned by `libretro_runner`.

Physical controller input should not be read from GTK event controllers. GTK remains responsible for the libretro window lifecycle and keyboard fallback, while the actual joypad is read through a dedicated gamepad input layer in `relm4-ui` (planned as `gilrs` unless implementation constraints force a different backend). That layer translates controller events into the shared `InputState`.

The libretro-facing mapping should stay explicit:

- physical controller buttons and D-pad inputs update the digital button fields read through `RETRO_DEVICE_JOYPAD`
- analog-stick motion updates analog axis fields read through `RETRO_DEVICE_ANALOG`

Before launching a mapped core, run a small preflight in the service layer that validates:

1. the selected core is supported
2. the selected ROM filename matches the core's supported extensions
3. all required firmware files exist in the configured libretro system directory

If preflight fails, return a typed service error and show it in the existing GUI error dialog instead of letting the core fail deep inside `retro_load_game()`.

## Key Decisions

| Decision | Rationale |
|---|---|
| Reuse the existing system-to-core mapping flow | The database, service, and settings dialog already support per-system libretro core mappings. |
| Add a libretro system directory setting instead of reusing `temp_output_dir` | Firmware files are persistent configuration, not temp launch artifacts. |
| Model core requirements as metadata in `libretro_runner` | FreeIntv needs firmware and extension validation, and this structure scales to future cores better than string constants. |
| Extend the frontend input model instead of hardcoding FreeIntv logic inside callbacks | `libretro_runner` should stay generic and reusable for future cores that also need analog input. |
| Keep the control UX in `relm4-ui` | Input presentation is a frontend concern; `libretro_runner` should only store/query state. |
| Add physical joypad/analog-stick capture in the first phase | Joypad support is the target feature, and generic analog input support is useful for libretro cores beyond keyboard-only control. |
| Keep the existing keyboard path as secondary input | This preserves current usability while adding a generic physical-controller path for libretro sessions. |
| Keep controller handling generic | FreeIntv-specific controller UX is out of scope for this spec revision; the current goal is a reusable joypad/analog input path. |
| Read physical controllers through a dedicated input library rather than GTK | GTK4/relm4 does not provide the right abstraction for joypad polling; the controller path belongs in a frontend input backend such as `gilrs`. |
| Map controller state to libretro at the `InputState` boundary | This keeps controller backend details inside `relm4-ui` and keeps `libretro_runner` focused on libretro device queries. |

## Acceptance Criteria

- `freeintv_libretro` is included in the supported libretro core definitions used by the app.
- The settings UI lets the user configure both libretro core directory and libretro system directory.
- The libretro launch path passes the configured system directory to `LibretroCore::load()`.
- Launching FreeIntv content fails fast with a clear error if `exec.bin` or `grom.bin` is missing.
- Launching FreeIntv content fails fast with a clear error if the selected file does not use `.int`, `.rom`, or `.bin`.
- `libretro_runner` can answer both digital joypad and analog libretro input queries from a shared frontend-owned input state.
- The current implementation stage includes generic physical controller forwarding for Retropad buttons and analog axes.
- Physical joypad input is read in `relm4-ui` via a dedicated controller backend instead of GTK event controllers.
- `relm4-ui/src/libretro/input.rs` supports physical joypad/analog-stick capture for libretro sessions.
- Physical controller buttons and D-pad inputs are exposed to cores through `RETRO_DEVICE_JOYPAD`.
- Analog-stick motion is exposed to cores through `RETRO_DEVICE_ANALOG`.
- The current generic libretro controller behavior is documented in the app/docs, along with what remains out of scope.
- Existing mapped-core flows for already supported cores continue to work.
- Documentation explains how to configure FreeIntv firmware and what is intentionally out of scope.
- `docs/LIBRETRO_INTEGRATION.md` no longer describes hardcoded core-path wiring as the current approach.

## As Implemented

- `core_types`, `service`, and `relm4-ui` now persist and expose a dedicated `libretro_system_dir` setting alongside the existing core-directory setting.
- `LibretroRunnerService::prepare_rom()` now passes the configured libretro system directory to the runner instead of reusing `temp_output_dir`.
- `SUPPORTED_CORES` now includes `freeintv_libretro`, so the core can be mapped in the existing per-system core-mapping flow.
- `libretro_runner::libretro_info_parser` now parses `.info` files for supported extensions and firmware requirements, including the checked-in `freeintv_libretro.info` example data.
- `LibretroCoreService::get_core_system_info()` now reports whether a mapped core is available, which required firmware files are present in the configured system directory, which file extensions the core declares as supported, and which input profile the frontend should use.
- `relm4-ui/src/libretro/runner.rs` now enables launch when a core, file, and file set are selected, and surfaces launch-preparation, system-info, and core-path failures through `show_error_dialog(e.to_string(), root)` so typed `LibretroPreflightError` messages reach the user instead of generic launch-failure text.
- `libretro_runner::InputState` now stores both digital button state and analog axis state, and `input_state_cb` now answers both `RETRO_DEVICE_JOYPAD` and `RETRO_DEVICE_ANALOG`.
- `relm4-ui` now uses `gilrs` to poll physical gamepad input and forwards generic controller state into the shared `InputState`.
- The currently implemented gamepad mapping is generic Retropad-style input:
  - D-pad maps to libretro joypad directions
  - South/East/North/West map to `A`/`B`/`Y`/`X`
  - shoulder buttons map to `L`/`R`
  - Start/Select map to libretro `START`/`SELECT`
- left and right analog sticks map to libretro analog X/Y axes with Y inverted and a small deadzone
- FreeIntv-specific controller UX is now out of scope for this spec revision; the implemented input path is intentionally generic rather than core-specific.
- `supported_cores.rs` now uses structured app-policy metadata instead of a raw allowlist, while `.info`-derived extensions and firmware requirements remain the source of truth for per-core launch requirements.
- `LibretroRunnerService::prepare_rom()` now runs a typed preflight pipeline that downloads the file set, selects the launch file, validates required firmware, validates supported extensions, and builds launch paths using the configured libretro system directory.
- `LibretroPreflightError` now covers unsupported extensions, download failures, missing files, missing firmware, missing system directory configuration, and invalid initial file selection.
- `service/src/libretro/runner/prepare/context.rs` now depends on `DownloadServiceOps`, allowing the preflight download step to use the existing mock boundary in tests.
- `service/src/libretro/runner/prepare/steps.rs` now has unit tests covering the deterministic preflight steps, including the download step through `MockDownloadServiceOps`.
- `service/src/libretro/runner/service.rs` now has higher-level `prepare_rom()` tests covering download failure, missing files in the file set, invalid initial file selection, missing firmware, unsupported extension, missing system directory configuration, and the happy-path launch result.
- `relm4-ui/src/libretro/input.rs` now has tests for the extracted pure controller-mapping helpers, covering digital button mapping, analog-axis mapping, deadzone handling, and keyboard-to-libretro button mapping.
- `docs/LIBRETRO_INTEGRATION.md` and `README.md` now describe the current configured system-directory flow, metadata-driven preflight, and generic controller behavior, while explicitly keeping FreeIntv-specific controller UX out of scope for this revision.
