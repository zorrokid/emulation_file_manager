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

When a user opens the "Manage Core Mappings" dialog or clicks on a system:
- Scan the configured `Settings.libretro_core_dir` for files matching supported cores with platform extension (e.g., `fceumm_libretro.so` on Linux)
- Filter to only supported cores (from `SUPPORTED_CORES` list)
- Return sorted list of supported **core names without extension** (e.g., `fceumm_libretro`, `snes9x_libretro`)
- If `libretro_core_dir` is not configured, return empty list; button to open dialog is disabled in settings

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
- System selector dropdown (loads on dialog show)
- Two columns:
  - Left: "Mapped Cores" — listview of cores currently mapped to the selected system
  - Right: "Available Cores" — listview of all `.so` files in the cores directory
- "Add Mapping" button (moves selected available core to mapped list)
- "Remove Mapping" button (removes selected mapped core from list)
- "Close" button

**Behavior:**
- When dialog opens:
  - Load all systems from database
  - Scan cores directory
  - If no system selected, select first system (if any exist)
- When system selected:
  - Load mapped cores for that system
  - Update both lists
- When "Add Mapping" button clicked:
  - Take selected available core
  - Add mapping in database
  - Reload both lists for current system
  - Show error toast if mapping fails (e.g., validation error)
- When "Remove Mapping" button clicked:
  - Delete mapping from database
  - Reload both lists
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
- [ ] Migration creates `system_libretro_core` table with correct schema
- [ ] `SystemLibretroCoreRepository` implements all required methods
- [ ] All 8 repository tests pass (add, get, remove, duplicate validation, empty name validation, unsupported core validation, cascade delete, remove all)
- [ ] Unique constraint prevents duplicate core mappings for a system
- [ ] CASCADE delete removes cores when system is deleted

### Service Layer
- [ ] `SUPPORTED_CORES` constant defined with list of supported cores
- [ ] `LibretroCoreService::scan_available_cores()` scans configured dir, filters to **only supported cores**, returns sorted filenames
- [ ] `LibretroCoreService::scan_available_cores()` returns `Ok(vec![])` if `libretro_core_dir` is `None` or no supported cores exist
- [ ] `LibretroCoreService::get_cores_for_system(system_id)` returns `(id, core_name)` pairs
- [ ] `LibretroCoreService::add_core_mapping()` validates: non-empty `core_name` **and** `core_name` is in `SUPPORTED_CORES`; returns mapping ID on success
- [ ] `LibretroCoreService::add_core_mapping()` returns `ValidationError` if core is not supported
- [ ] `LibretroCoreService::remove_core_mapping()` deletes by ID, returns `Ok(())` on success
- [ ] `LibretroRunnerService::resolve_core_path(core_name)` returns full path `<libretro_core_dir>/<core_name>`
- [ ] `LibretroRunnerService::resolve_core_path()` returns error if `libretro_core_dir` not configured

### UI: Settings Dialog
- [ ] "Manage Core Mappings" button appears below libretro_core_dir field
- [ ] Button is **disabled** when `libretro_core_dir` is not set
- [ ] Opening dialog loads systems and scans cores directory (async)
- [ ] System dropdown shows all systems
- [ ] Mapped cores list shows only cores for selected system
- [ ] Available cores list shows all `.so` files from cores directory
- [ ] "Add Mapping" button adds selected available core to mapped list (persists)
- [ ] "Remove Mapping" button removes selected mapped core (persists)
- [ ] Lists update immediately after add/remove (no dialog close/reopen needed)
- [ ] Error toast shown if add/remove fails
- [ ] Closing and reopening dialog shows persisted mappings

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
