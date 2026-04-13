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

On the GTK side, add a keyboard-first FreeIntv input profile in the libretro window so the user can access:

- D-pad movement
- 16-way disc input
- keypad input including `0`, `Enter`, and `Clear`
- controller swap

The first implementation will not add an on-screen keypad overlay. Instead, `relm4-ui` will provide a documented keyboard mapping that feeds the generic input state owned by `libretro_runner`.

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
| Use keyboard-first FreeIntv controls in the first phase | This keeps the feature achievable without designing a new overlay component before the input model is proven. |

## Acceptance Criteria

- `freeintv_libretro` is included in the supported libretro core definitions used by the app.
- The settings UI lets the user configure both libretro core directory and libretro system directory.
- The libretro launch path passes the configured system directory to `LibretroCore::load()`.
- Launching FreeIntv content fails fast with a clear error if `exec.bin` or `grom.bin` is missing.
- Launching FreeIntv content fails fast with a clear error if the selected file does not use `.int`, `.rom`, or `.bin`.
- The frontend can express the FreeIntv control surface needed for keypad input, disc input, and controller swap.
- The keyboard-first FreeIntv control mapping is documented in the app/docs so users know how to operate the core.
- Existing mapped-core flows for already supported cores continue to work.
- Documentation explains how to configure FreeIntv firmware and what is intentionally out of scope.
- `docs/LIBRETRO_INTEGRATION.md` no longer describes hardcoded core-path wiring as the current approach.

## As Implemented
_(Pending)_
