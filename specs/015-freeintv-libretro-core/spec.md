# 015: FreeIntv Libretro Core Support

## Status
<!-- Planning | In Progress | Complete | Abandoned -->
Planning

## Affected Crates
- `libretro_runner` — supported-core metadata, FreeIntv definition, and richer input state
- `core_types` — persisted setting name for libretro system directory
- `service` — settings plumbing and launch preflight validation
- `relm4-ui` — settings UI for firmware/system directory, FreeIntv input UX, and launch error handling
- `docs` — FreeIntv setup notes and firmware requirements

## Problem

The application already supports libretro core mapping and in-process launching, but it cannot reliably support the FreeIntv Intellivision core yet. The current implementation only knows about `fceumm_libretro`, does not model per-core requirements, always points libretro cores at the temp directory for system files, and only answers digital joypad input for player 1. That breaks FreeIntv, which requires `exec.bin` and `grom.bin` in the frontend system directory and relies on a wider Retropad surface, including analog-style disc and keypad interactions.

`docs/LIBRETRO_INTEGRATION.md` already documents the libretro callback architecture and BIOS/system-directory concept correctly, but its “hardcoded core path” onboarding section is stale relative to the current codebase, which already has per-system core mapping in the database, service, and settings UI.

The current input path is also too narrow for this core. `relm4-ui/src/libretro/input.rs` currently only maps GTK keyboard events to digital joypad buttons, `libretro_runner::InputState` only stores digital button state, and `input_state_cb` only answers `RETRO_DEVICE_JOYPAD`. That means the frontend cannot yet express a physical joypad/analog-stick control surface, and the runner cannot expose the analog state a core would need for something like the FreeIntv 16-way disc.

## Proposed Solution

Promote supported libretro cores from a string allowlist to structured metadata in `libretro_runner`. Add a `freeintv_libretro` definition that captures:

- core name
- supported ROM extensions (`.int`, `.rom`, `.bin`)
- required firmware (`exec.bin`, `grom.bin`)

Add a dedicated libretro system directory setting so the user can point the app at the directory containing libretro firmware files. Update the launch path to use that directory instead of `temp_output_dir` when passing `GET_SYSTEM_DIRECTORY` to the core.

Extend the input layer so `libretro_runner` can answer more than digital joypad buttons. The input state should support:

- joypad buttons
- analog axes required by the FreeIntv disc/keypad mapping
- FreeIntv-specific controller swap state routed through the existing libretro control surface

`libretro_runner` should remain generic here: it owns a frontend-facing input state plus libretro device/axis query support, and `input_state_cb` should answer both `RETRO_DEVICE_JOYPAD` and `RETRO_DEVICE_ANALOG` from that shared state.

On the frontend side, extend `relm4-ui/src/libretro/input.rs` beyond keyboard-only handling so libretro sessions can feed physical joypad/analog-stick input into the shared state. The frontend must be able to express:

- D-pad movement
- 16-way disc input
- keypad input including `0`, `Enter`, and `Clear`
- controller swap

The first implementation will not add an on-screen keypad overlay. Instead, it will keep the existing keyboard path where useful, but add physical joypad/analog-stick capture in `relm4-ui/src/libretro/input.rs` and route that through the generic input state owned by `libretro_runner`.

Physical controller input should not be read from GTK event controllers. GTK remains responsible for the libretro window lifecycle and keyboard fallback, while the actual joypad is read through a dedicated gamepad input layer in `relm4-ui` (planned as `gilrs` unless implementation constraints force a different backend). That layer translates controller events into the shared `InputState`.

For the 16-way disc, the frontend should map analog-stick direction changes onto generic libretro analog X/Y values written into the shared input state. This keeps the runner core-agnostic while letting the frontend define the control profile needed by FreeIntv.

The libretro-facing mapping should stay explicit:

- physical controller buttons and D-pad inputs update the digital button fields read through `RETRO_DEVICE_JOYPAD`
- analog-stick motion updates analog axis fields read through `RETRO_DEVICE_ANALOG`
- the FreeIntv 16-way disc profile is implemented as frontend-side analog X/Y translation rather than a FreeIntv-specific callback path in `libretro_runner`

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
| Add physical joypad/analog-stick capture in the first phase | Joypad support is the target feature, and analog-stick input is the clearest fit for the FreeIntv disc. |
| Keep the existing keyboard path as secondary input, not as the disc source | This preserves current usability without conflating disc emulation with keyboard bindings. |
| Represent the 16-way disc through generic analog axes | Libretro already has a standard analog input surface; using it avoids a FreeIntv-only callback path in the runner. |
| Read physical controllers through a dedicated input library rather than GTK | GTK4/relm4 does not provide the right abstraction for joypad polling; the controller path belongs in a frontend input backend such as `gilrs`. |
| Map controller state to libretro at the `InputState` boundary | This keeps controller backend details inside `relm4-ui` and keeps `libretro_runner` focused on libretro device queries. |

## Acceptance Criteria

- `freeintv_libretro` is included in the supported libretro core definitions used by the app.
- The settings UI lets the user configure both libretro core directory and libretro system directory.
- The libretro launch path passes the configured system directory to `LibretroCore::load()`.
- Launching FreeIntv content fails fast with a clear error if `exec.bin` or `grom.bin` is missing.
- Launching FreeIntv content fails fast with a clear error if the selected file does not use `.int`, `.rom`, or `.bin`.
- `libretro_runner` can answer both digital joypad and analog libretro input queries from a shared frontend-owned input state.
- The frontend can express the FreeIntv control surface needed for keypad input, disc input, and controller swap.
- Physical joypad input is read in `relm4-ui` via a dedicated controller backend instead of GTK event controllers.
- `relm4-ui/src/libretro/input.rs` supports physical joypad/analog-stick capture for libretro sessions.
- Physical controller buttons and D-pad inputs are exposed to cores through `RETRO_DEVICE_JOYPAD`.
- The 16-way disc mapping is documented as analog-stick input routed through libretro analog X/Y state.
- Analog-stick motion is exposed to cores through `RETRO_DEVICE_ANALOG`.
- The final control mapping is documented in the app/docs so users know how to operate the core.
- Existing mapped-core flows for already supported cores continue to work.
- Documentation explains how to configure FreeIntv firmware and what is intentionally out of scope.
- `docs/LIBRETRO_INTEGRATION.md` no longer describes hardcoded core-path wiring as the current approach.

## As Implemented
_(Pending)_
