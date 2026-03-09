# 006: Libretro Core-to-System Mapping

## Overview

Users must be able to map libretro cores (`.so` files stored in the configured cores directory) to emulation systems (NES, SNES, Game Boy, etc.). The mapping is many-to-one: one system can have multiple cores. At launch time, the system determines which core to use based on the mapping.

## Motivation

Currently, the libretro runner uses a hardcoded core path, making it unusable for most systems. Users need a way to:
- Select which libretro cores they have installed
- Map those cores to their game systems
- Choose between cores at launch time if multiple are mapped

## Behavior

### Supported Cores

The application maintains a hardcoded list of supported libretro cores (without extension). Only these cores can be mapped and launched.

**Supported cores** (defined in `libretro_runner/src/supported_cores.rs`):
```rust
pub const SUPPORTED_CORES: &[&str] = &[
    "fceumm_libretro",      // NES
    "snes9x_libretro",      // SNES
    "mgba_libretro",        // Game Boy Advance
    // ... more cores as needed
];
```

At runtime, the platform-specific extension (`.so` on Linux, `.dylib` on macOS, `.dll` on Windows) is appended when scanning or resolving core paths.

Any file in the cores directory that is **not** a supported core (with the platform extension) is ignored.

### Core Scanning

Implemented in `LibretroCoreService::list_cores()` (`service/src/libretro_core/service.rs`).

When a user opens the "Manage Core Mappings" dialog:
- Scan the configured `Settings.libretro_core_dir` for files matching supported cores with platform extension (e.g., `fceumm_libretro.so` on Linux)
- Filter to only supported cores (from `SUPPORTED_CORES` list)
- Return list of supported **core names without extension** (e.g., `fceumm_libretro`, `snes9x_libretro`)
- If `libretro_core_dir` is not configured, return `Err(SettingsError)`; button to open dialog is disabled in settings so this path is not reached from the UI

### Mapping Storage

- Store system ↔ core mappings in `system_libretro_core` table
- `core_name` is stored **without extension** (e.g., `fceumm_libretro`, not `fceumm_libretro.so`)
- Directory and extension come from settings at runtime
- Unique constraint on `(system_id, core_name)` — a system cannot map the same core twice
- When a system is deleted, all its core mappings are automatically deleted (CASCADE)
- Only supported cores (from `SUPPORTED_CORES`) can be mapped; validation rejects unsupported core names

### Settings Dialog: "Manage Core Mappings"

Opened from the Settings form (below the libretro_core_dir field).

**Layout:**
- Two columns:
  - Left: "Available Cores" — listview of all supported cores found in the cores directory
  - Right: "Mapped Systems" — listview of systems currently mapped to the selected core
- "Add System" button (opens the `SystemSelector` dialog to pick a system to map to the selected core)
- "Remove System" button (removes selected system from the mapped list)
- "Close" button

**Behavior:**
- When dialog opens:
  - Scan cores directory; populate "Available Cores" list (async)
  - Clear "Mapped Systems" list until a core is selected
- When a core is selected in "Available Cores":
  - Load systems mapped to that core from the database
  - Populate "Mapped Systems" list
- When "Add System" button clicked:
  - Open the existing `SystemSelector` dialog (reuse `relm4-ui/src/system_selector.rs`)
  - Pass already-mapped system IDs so the selector can exclude or mark them
  - When a system is chosen from `SystemSelector`, add the mapping `(system_id, selected_core_name)` in the database
  - Reload "Mapped Systems" list for the current core
  - Show error toast if mapping fails (e.g., duplicate mapping)
- When "Remove System" button clicked:
  - Delete the selected mapping from the database
  - Reload "Mapped Systems" list
  - Show error toast if deletion fails
- Lists update immediately after add/remove operations
- Dialog remains open until user clicks "Close"

### Launch Flow (Core Selection)

When user launches a game (clicks "Start Libretro Runner"):

1. **Fetch mapped cores** for the game's system
2. **Handle result:**
   - **0 cores mapped:** Show error toast: "No libretro cores mapped for this system. Configure in Settings → Manage Core Mappings."
   - **1 core mapped:** Use that core immediately (no picker shown), proceed to ROM preparation
   - **2+ cores mapped:** Show the "Core Picker" dialog

### Core Picker Dialog

Modal dialog shown at launch when multiple cores are mapped to a system.

**Layout:**
- Listview of available cores (core filenames)
- "Launch" and "Cancel" buttons

**Behavior:**
- User selects a core from the list
- Clicks "Launch" → emit `CoreChosen` signal with selected core name and the file_set_id
- Clicks "Cancel" → emit `Cancelled` signal
- When `CoreChosen` is emitted:
  - Resolve the core path using `LibretroRunnerService::resolve_core_path(core_name)`
  - Proceed with ROM preparation and launch
- When `Cancelled` is emitted:
  - Close picker, return to previous state (no launch)

## Acceptance Criteria

### Database & Repository Layer
- [x] Migration creates `system_libretro_core` table with correct schema
- [x] `SystemLibretroCoreRepository` implements all required methods
- [x] Repository tests pass: add+get, get_for_core, remove, duplicate validation, empty name validation, cascade delete, remove all (7 tests)
- [x] Unique constraint prevents duplicate core mappings for a system
- [x] CASCADE delete removes cores when system is deleted

### Service Layer
- [x] `SUPPORTED_CORES` constant defined with list of supported cores (`libretro_runner/src/supported_cores.rs`)
- [x] `LibretroCoreService::list_cores()` scans configured dir, filters to **only supported cores**, returns filenames without extension
- [x] `LibretroCoreService::list_cores()` returns `Err(SettingsError)` if `libretro_core_dir` is `None`
- [x] `LibretroCoreService::get_systems_for_core(core_name)` returns `Vec<SystemCoreMappingModel>`
- [x] `LibretroCoreService::add_core_mapping(system_id, core_name)` validates `core_name` is in `SUPPORTED_CORES`; returns mapping ID on success
- [x] `LibretroCoreService::add_core_mapping()` returns `InvalidInput` error if core is not supported
- [x] `LibretroCoreService::get_cores_for_system(system_id)` returns `Vec<CoreMappingModel>` (used at launch time)
- [x] `LibretroCoreService::remove_core_mapping(mapping_id)` deletes by ID, returns `Ok(())`
- [x] `LibretroRunnerService::resolve_core_path(core_name)` returns full path `<libretro_core_dir>/<core_name>`
- [x] `LibretroRunnerService::resolve_core_path()` returns error if `libretro_core_dir` not configured

### UI: Settings Dialog
- [x] "Manage Core Mappings" button appears below libretro_core_dir field
- [x] Button is **disabled** when `libretro_core_dir` is not set
- [x] Opening dialog scans cores directory and populates "Available Cores" list (async)
- [x] Selecting a core in "Available Cores" loads and shows systems mapped to that core
- [x] "Add System" button opens `SystemSelector` dialog; already-mapped systems are excluded from selection
- [x] Choosing a system in `SystemSelector` creates the mapping and reloads "Mapped Systems" list
- [x] "Remove System" button removes selected system mapping and reloads list
- [x] Lists update immediately after add/remove (no dialog close/reopen needed)
- [x] Error toast shown if add/remove fails
- [ ] Closing and reopening dialog shows persisted mappings (manual verification needed)

### UI: Launch Flow
- [ ] 0 cores → error toast shown, no picker shown, no launch occurs
- [ ] 1 core → picker not shown, launch proceeds immediately with that core
- [ ] 2+ cores → picker dialog shown

### UI: Core Picker Dialog
- [ ] Dialog shows list of cores for the system
- [ ] User can select a core and click "Launch"
- [ ] Selecting a core and clicking "Launch" triggers ROM preparation with resolved core path
- [ ] Clicking "Cancel" closes picker without launching
- [ ] Session launches and runs without errors

## Notes

- Core names are stored **without extension** (e.g., `fceumm_libretro`, not `fceumm_libretro.so`)
- Extension is appended at scan/resolution time based on the platform
- Directory path comes from `Settings.libretro_core_dir` at runtime
- All core mapping operations are async (database access)
- Available cores list is scanned fresh each time dialog opens (reflects filesystem changes)
