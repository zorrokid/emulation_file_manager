# 006: Libretro Core-to-System Mapping — Task Breakdown

## Layer 1: Database (Migration & Repository)

### Task 1.1: Create database migration

**File:** `database/migrations/20260307120000_add_system_libretro_core.sql`

**SQL Migration:**
```sql
CREATE TABLE system_libretro_core (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    system_id INTEGER NOT NULL REFERENCES system(id) ON DELETE CASCADE,
    core_name TEXT NOT NULL,
    UNIQUE(system_id, core_name)
);
```

**Acceptance:**
- [x] Migration file created
- [x] `cargo sqlx prepare --workspace -- --all-targets` runs without error
- [x] `tbls doc` updates schema documentation

### Task 1.2: Add database model

**File:** `database/src/models.rs`

**Code to add:**
```rust
#[derive(Clone, Debug, PartialEq)]
pub struct SystemLibretroCore {
    pub id: i64,
    pub system_id: i64,
    pub core_name: String,
}
```

**Acceptance:**
- [ ] `cargo build` succeeds
- [ ] Model compiles without warnings

### Task 1.3: Implement repository

**File:** `database/src/repository/system_libretro_core_repository.rs` (new file)

**Public API:**
```rust
pub struct SystemLibretroCoreRepository { pool: Arc<Pool<Sqlite>> }

impl SystemLibretroCoreRepository {
    pub fn new(pool: Arc<Pool<Sqlite>>) -> Self { ... }

    /// Return (id, core_name) tuples for the system, sorted by core_name
    pub async fn get_cores_for_system(&self, system_id: i64)
        -> Result<Vec<(i64, String)>, DatabaseError>;

    /// Add a core mapping. Validates that core_name is non-empty.
    /// Returns the inserted ID on success.
    /// Returns DatabaseError::ValidationError if core_name is empty or whitespace-only.
    /// Returns DatabaseError::UniqueViolation if mapping already exists.
    pub async fn add_core(&self, system_id: i64, core_name: &str)
        -> Result<i64, DatabaseError>;

    /// Delete a core mapping by ID
    pub async fn remove_core(&self, id: i64)
        -> Result<(), DatabaseError>;

    /// Delete all core mappings for a system
    pub async fn remove_all_cores_for_system(&self, system_id: i64)
        -> Result<(), DatabaseError>;
}
```

**Tests (inline `#[cfg(test)]` with `#[async_std::test]`):**

1. `test_add_core_returns_id`
   - Arrange: create test system
   - Act: add core "fceumm_libretro.so"
   - Assert: returned ID > 0

2. `test_get_cores_for_system`
   - Arrange: create system, add cores "fceumm_libretro.so", "snes9x_libretro.so"
   - Act: call `get_cores_for_system(system_id)`
   - Assert: returns vec with both cores, sorted by name

3. `test_remove_core`
   - Arrange: create system, add core "fceumm_libretro.so", get mapping ID
   - Act: call `remove_core(id)`
   - Assert: returns Ok(()), next `get_cores_for_system` returns empty vec

4. `test_add_duplicate_core_fails`
   - Arrange: create system, add core "fceumm_libretro.so"
   - Act: add same core again to same system
   - Assert: returns Err (DatabaseError)

5. `test_add_empty_core_name_fails`
   - Arrange: create system
   - Act: call `add_core(system_id, "")`
   - Assert: returns DatabaseError::ValidationError

6. `test_cascade_delete_with_system`
   - Arrange: create system, add core mapping
   - Act: delete the system
   - Assert: core mapping is automatically deleted (no orphans)

7. `test_remove_all_cores_for_system`
   - Arrange: create system, add 3 cores
   - Act: call `remove_all_cores_for_system(system_id)`
   - Assert: all mappings deleted, `get_cores_for_system` returns empty vec

8. `test_add_whitespace_only_core_name_fails`
   - Arrange: create system
   - Act: call `add_core(system_id, "   ")`
   - Assert: returns DatabaseError::ValidationError

**Acceptance:**
- [ ] All 8 tests pass
- [ ] `cargo clippy` shows no warnings in this file
- [ ] `cargo build` succeeds

### Task 1.4: Register repository in RepositoryManager

**Files:**
- `database/src/repository/mod.rs` — add `pub mod system_libretro_core_repository;`
- `database/src/repository_manager.rs`:
  - Add field: `system_libretro_core_repository: SystemLibretroCoreRepository`
  - Construct in `new(pool)`: `system_libretro_core_repository: SystemLibretroCoreRepository::new(pool.clone())`
  - Add accessor: `pub fn get_system_libretro_core_repository(&self) -> &SystemLibretroCoreRepository { ... }`

**Acceptance:**
- [ ] `cargo build` succeeds
- [ ] `cargo test database` passes

---

## Layer 2: Service

### Task 2.1: Add view model

**File:** `service/src/view_models.rs`

**Code to add:**
```rust
#[derive(Debug, Clone, PartialEq)]
pub struct LibretroCoreMappingModel {
    pub id: i64,
    pub system_id: i64,
    pub core_name: String,
}
```

**Acceptance:**
- [ ] `cargo build` succeeds
- [ ] Type is importable from service crate

### Task 2.2: Define Supported Cores

**File:** `libretro_runner/src/supported_cores.rs` (new file)

**Code:**
```rust
/// List of libretro cores supported by this application.
/// Core names are stored WITHOUT extension (extension added at runtime based on platform).
/// Only cores in this list can be mapped to systems and launched.
pub const SUPPORTED_CORES: &[&str] = &[
    "fceumm_libretro",       // NES
    "snes9x_libretro",       // SNES
    "mgba_libretro",         // Game Boy Advance
    // Add more cores as needed
];

/// Check if a core name is supported.
/// core_name should be provided WITHOUT extension.
pub fn is_core_supported(core_name: &str) -> bool {
    SUPPORTED_CORES.contains(&core_name)
}

/// Get the filename for a core with platform-specific extension.
/// On Linux: "fceumm_libretro" → "fceumm_libretro.so"
#[cfg(target_os = "linux")]
pub fn get_core_filename(core_name: &str) -> String {
    format!("{}.so", core_name)
}

#[cfg(target_os = "macos")]
pub fn get_core_filename(core_name: &str) -> String {
    format!("{}.dylib", core_name)
}

#[cfg(target_os = "windows")]
pub fn get_core_filename(core_name: &str) -> String {
    format!("{}.dll", core_name)
}
```

**Acceptance:**
- [ ] `cargo build` succeeds
- [ ] `SUPPORTED_CORES` contains core names without extensions
- [ ] `is_core_supported()` checks core names without extensions
- [ ] `get_core_filename()` appends correct extension for the platform
- [ ] All functions are public and importable
- [ ] On Linux build, `get_core_filename("fceumm_libretro")` returns `"fceumm_libretro.so"`

**Note:** Will be exported from `libretro_runner/src/lib.rs` so it's available to the service layer.

---

### Task 2.3: Create LibretroCoreService

**Files:**
- `service/src/libretro_core/mod.rs` (new dir)
- `service/src/libretro_core/service.rs` (new file)

**Modify** `libretro_runner/src/lib.rs` to export:
```rust
pub mod supported_cores;
pub use supported_cores::{SUPPORTED_CORES, is_core_supported, get_core_filename};
```

**Public API:**
```rust
pub struct LibretroCoreService {
    repository_manager: Arc<RepositoryManager>,
    settings: Arc<Settings>,
}

impl LibretroCoreService {
    pub fn new(repository_manager: Arc<RepositoryManager>, settings: Arc<Settings>) -> Self { ... }

    /// Scan libretro_core_dir for files matching supported cores (with platform extension).
    /// Returns sorted **core names without extension** (e.g., "fceumm_libretro", not "fceumm_libretro.so").
    /// Returns Ok(vec![]) if libretro_core_dir is None or no supported cores found on disk.
    /// Returns Error if directory scan fails.
    pub fn scan_available_cores(&self) -> Result<Vec<String>, Error>;

    /// Returns (id, core_name) pairs for a system.
    /// Returns empty vec if no cores mapped.
    pub async fn get_cores_for_system(&self, system_id: i64)
        -> Result<Vec<(i64, String)>, Error>;

    /// Add a core mapping. Validates:
    /// - core_name is non-empty and non-whitespace
    /// - core_name is in SUPPORTED_CORES list
    /// Returns the mapping ID on success.
    /// Returns ValidationError if core is not supported.
    pub async fn add_core_mapping(&self, system_id: i64, core_name: &str)
        -> Result<i64, Error>;

    /// Remove a core mapping by ID.
    pub async fn remove_core_mapping(&self, mapping_id: i64)
        -> Result<(), Error>;
}
```

**Implementation details:**
- Import from `libretro_runner`: `use libretro_runner::{SUPPORTED_CORES, is_core_supported, get_core_filename};`
- `scan_available_cores()` should:
  1. Return early with `Ok(vec![])` if `libretro_core_dir` is `None`
  2. For each supported core in `SUPPORTED_CORES`, check if the platform-specific file exists (using `get_core_filename()`)
  3. Collect core names (WITHOUT extension) for cores that exist
  4. Sort and return
  - Example: If `SUPPORTED_CORES = ["fceumm_libretro", "snes9x_libretro"]` and only `fceumm_libretro.so` exists, return `["fceumm_libretro"]`
- `add_core_mapping()` should validate using `is_core_supported()` before calling repository

**Error handling:**
- Propagate database errors as `Error::RepositoryError`
- Propagate filesystem errors (scan) as `Error::FileSystemError`
- Validation errors as `Error::ValidationError`

**Acceptance:**
- [ ] `cargo build` succeeds
- [ ] `cargo clippy` shows no warnings

### Task 2.4: Add path resolver to LibretroRunnerService

**File:** `service/src/libretro_runner/service.rs`

**Code to add:**
```rust
impl LibretroRunnerService {
    /// Resolve the full path to a libretro core file.
    /// core_name should be provided WITHOUT extension (e.g., "fceumm_libretro").
    /// The platform-specific extension is appended automatically.
    /// Returns Err if libretro_core_dir is not configured.
    /// Example: core_name = "fceumm_libretro"
    ///          → PathBuf("/path/to/cores/fceumm_libretro.so") on Linux
    pub fn resolve_core_path(&self, core_name: &str) -> Result<PathBuf, Error> {
        match &self.settings.libretro_core_dir {
            Some(dir) => {
                let filename = libretro_runner::get_core_filename(core_name);
                Ok(dir.join(filename))
            },
            None => Err(Error::SettingsError(
                "Libretro core directory not configured".into()
            )),
        }
    }
}
```

**Acceptance:**
- [ ] `cargo build` succeeds
- [ ] `cargo test service` passes

### Task 2.5: Register LibretroCoreService in AppServices

**File:** `service/src/app_services.rs`

**Changes:**
- Add field: `libretro_core: OnceLock<Arc<LibretroCoreService>>`
- Add accessor method:
  ```rust
  pub fn libretro_core(&self) -> Arc<LibretroCoreService> {
      self.libretro_core.get_or_init(|| {
          Arc::new(LibretroCoreService::new(
              self.repository_manager.clone(),
              self.settings.clone(),
          ))
      }).clone()
  }
  ```

**Acceptance:**
- [ ] `cargo build` succeeds

---

## Layer 3: UI

### Task 3.1: Create Core Mapping Dialog

**File:** `relm4-ui/src/settings_components/libretro_cores_dialog.rs` (implemented)

**Actual design** (core-centric, not system-centric):
- Left column: `StringListView<String>` for available cores (cores are the primary selection)
- Right column: `TypedListView<ListItem, gtk::SingleSelection>` for systems mapped to the selected core
- "Add System" opens `SystemSelector` to pick a system to map to the selected core
- "Remove System" removes the selected system mapping
- "Close" hides the dialog

**Acceptance:**
- [x] Component compiles
- [x] Dialog launches without panic
- [x] Available cores list populates with supported cores found in cores dir (without extension)
- [x] Selecting a core loads systems mapped to that core
- [x] Mapped systems list populates for selected core

### Task 3.2: Create Core Picker Dialog

**File:** `relm4-ui/src/libretro/core_picker.rs` (new file)

**Relm4 Component:**

```rust
pub struct CorePickerDialog {
    file_set_id: i64,
    cores_wrapper: TypedListView<StringListItem<String>, gtk::SingleSelection>,
    selected_core: Option<String>,
}

pub struct CorePickerInit;  // no app_services needed — cores passed via Show

#[derive(Debug)]
pub enum CorePickerMsg {
    Show { cores: Vec<String>, file_set_id: i64 },
    SelectionChanged,
    LaunchClicked,
    Cancelled,
}

#[derive(Debug)]
pub enum CorePickerOutput {
    CoreChosen { core_name: String, file_set_id: i64 },
    Cancelled,
}

impl Component for CorePickerDialog {
    type Init = CorePickerInit;
    type Input = CorePickerMsg;
    type Output = CorePickerOutput;
    type CommandOutput = ();
    // ... impl methods
}
```

Uses `TypedListView<StringListItem<String>, gtk::SingleSelection>` from `ui-components`.

**View Layout:**
```
gtk::Window [modal, transient_for=main_window]
  gtk::Box [vertical, spacing=10, margin=10]
    Label "Select a libretro core" [bold]
    gtk::ScrolledWindow
      gtk::ListView (cores) [core_name strings]
    gtk::Box [horizontal]
      Button "Launch" [sensitive only if core selected]
      Button "Cancel"
```

**Message Handlers:**

- `Show { cores, file_set_id }`:
  - Store file_set_id, clear list, populate with cores
  - Clear selected_core
  - Present window

- `SelectionChanged`:
  - Update selected_core from list selection

- `LaunchClicked`:
  - Emit `CorePickerOutput::CoreChosen { core_name, file_set_id }`
  - Hide window

- `Cancelled`:
  - Emit `CorePickerOutput::Cancelled`
  - Hide window

**Acceptance:**
- [ ] Component compiles
- [ ] Dialog launches without panic
- [ ] Dialog shows core names from input
- [ ] Selecting core and clicking "Launch" emits CoreChosen with correct core_name
- [ ] Clicking "Cancel" emits Cancelled

### Task 3.3: Update libretro module exports

**File:** `relm4-ui/src/libretro/mod.rs`

**Changes:**
```rust
pub mod core_picker;

pub use core_picker::{CorePickerDialog, CorePickerInit, CorePickerMsg, CorePickerOutput};
```

Note: Core mapping dialog lives in `settings_components/libretro_cores_dialog.rs`, not in `libretro/`.

**Acceptance:**
- [ ] `cargo build` succeeds

### Task 3.4: Add button to Settings Form

**File:** `relm4-ui/src/settings_form.rs`

**Changes:**
- Add field to `SettingsFormModel`:
  ```rust
  core_mapping_dialog: Controller<CoreMappingModel>,
  ```

- In `init()`, add:
  ```rust
  let core_mapping_dialog = CoreMappingModel::builder()
      .launch(CoreMappingInit { app_services })
      .transient_for(root)
      .build();
  ```

- Add variant to `SettingsFormMsg`:
  ```rust
  OpenCoreMappingDialog,
  ```

- In view, add button below `libretro_core_dir` row:
  ```rust
  gtk::Button::new() [
      set_label: "Manage Core Mappings",
      set_sensitive: self.libretro_core_dir.is_some(),
      connect_clicked[sender = sender.clone()] => move |_| {
          sender.input(SettingsFormMsg::OpenCoreMappingDialog);
      },
  ]
  ```

- In `update_with_view`, handle message:
  ```rust
  SettingsFormMsg::OpenCoreMappingDialog => {
      self.core_mapping_dialog.emit(CoreMappingMsg::Show);
  }
  ```

**Acceptance:**
- [x] Button appears in settings form
- [x] Button is disabled when libretro_core_dir not set
- [x] Button is enabled when libretro_core_dir is set
- [x] Clicking button opens Core Mapping dialog (dialog lives at `settings_components/libretro_cores_dialog.rs`)

### Task 3.5: Update Release Model — Add Core Picker

**File:** `relm4-ui/src/release.rs`

**Changes:**

1. Add field to `ReleaseModel`:
   ```rust
   core_picker: Controller<CorePickerModel>,
   ```

2. In `init()`, add:
   ```rust
   let core_picker = CorePickerModel::builder()
       .launch(CorePickerInit { app_services })
       .forward(sender.input_sender(), |msg| match msg {
           CorePickerOutputMsg::CoreChosen { file_set_id, core_name } => {
               ReleaseMsg::CorePickerConfirmed { file_set_id, core_name }
           },
           CorePickerOutputMsg::Cancelled => ReleaseMsg::CorePickerCancelled,
       });
   ```

3. Add variants to `ReleaseMsg`:
   ```rust
   CorePickerChosen { core_name: String, file_set_id: i64 },
   CorePickerCancelled,
   ```
   (`StartLibretroRunner` already exists)

4. Add variant to `ReleaseCommandMsg`:
   ```rust
   CoresFetchedForLaunch { file_set_id: i64, cores: Result<Vec<CoreMappingModel>, Error> },
   // CoreMappingModel is from service::libretro_core::service; has .core_name: String field
   ```

5. Replace `StartLibretroRunner` handler:
   ```rust
   ReleaseMsg::StartLibretroRunner => {
       if let (Some(file_set), Some(release)) = (&self.selected_file_set, &self.selected_release) {
           if let Some(system) = release.systems.first() {
               let file_set_id = file_set.id;
               let system_id = system.id;
               let app_services = Arc::clone(&self.app_services);
               sender.oneshot_command(async move {
                   let cores = app_services
                       .libretro_core()
                       .get_cores_for_system(system_id)
                       .await;
                   ReleaseCommandMsg::CoresFetchedForLaunch { file_set_id, cores }
               });
           }
       }
   }
   ```

6. Add handler in `update_cmd` for `CoresFetchedForLaunch`:
   ```rust
   ReleaseCommandMsg::CoresFetchedForLaunch { file_set_id, cores } => {
       match cores {
           Err(e) => { /* send ShowError output */ },
           Ok(cores) if cores.is_empty() => {
               // 0 cores: show error output
               sender.output(ReleaseOutputMsg::ShowError(
                   "No libretro cores mapped for this system. Configure in Settings → Manage Core Mappings.".into()
               )).unwrap_or_default();
           },
           Ok(cores) if cores.len() == 1 => {
               // 1 core: proceed directly
               sender.input(ReleaseMsg::CorePickerChosen {
                   core_name: cores[0].core_name.clone(),
                   file_set_id,
               });
           },
           Ok(cores) => {
               // 2+ cores: show picker
               self.core_picker.emit(CorePickerMsg::Show {
                   cores: cores.into_iter().map(|c| c.core_name).collect(),
                   file_set_id,
               });
           },
       }
   }
   ```

7. Add handler for `CorePickerChosen` (in `update`):
   ```rust
   ReleaseMsg::CorePickerChosen { core_name, file_set_id } => {
       let app_services = Arc::clone(&self.app_services);
       match app_services.libretro_runner().resolve_core_path(&core_name) {
           Ok(core_path) => {
               sender.oneshot_command(async move {
                   ReleaseCommandMsg::LibretroRomPrepared(
                       app_services.libretro_runner().prepare_rom(LibretroLaunchModel {
                           file_set_id,
                           initial_file: None,
                           core_path,
                       }).await
                   )
               });
           },
           Err(e) => { /* send ShowError output */ },
       }
   }
   ```

8. Add simple handler for `CorePickerCancelled`:
   ```rust
   ReleaseMsg::CorePickerCancelled => {
       // Dialog closed, return to normal state
   }
   ```

**Acceptance:**
- [ ] `cargo build` succeeds
- [ ] No warnings in release.rs

---

## Manual Verification Checklist (GUI Testing)

After all tasks complete, verify manually in running app:

### Settings Dialog
- [ ] Open Settings → Libretro section
- [ ] "Manage Core Mappings" button is **disabled** if cores dir not set
- [ ] Set libretro_core_dir to a directory with some supported core files (e.g., `fceumm_libretro.so` on Linux)
- [ ] "Manage Core Mappings" button becomes **enabled**
- [ ] Click button → dialog opens
- [ ] Dialog shows all systems from database in dropdown
- [ ] Dialog shows **only supported** cores that exist in cores dir in "Available Cores" list, displayed **without extension** (e.g., "fceumm_libretro", not "fceumm_libretro.so")
- [ ] Select a system → see its currently mapped cores in "Mapped Cores" list
- [ ] Select an available core and click "Add Mapping" → core appears in mapped list
- [ ] Close dialog and reopen → mapping persists
- [ ] Select mapped core and click "Remove Mapping" → core disappears from mapped list
- [ ] Attempting to map an unsupported core shows error (if possible to test)
- [ ] Close dialog without error

### Launch with Multiple Cores
- [ ] Map 2 cores to a system (in Core Mapping dialog)
- [ ] Launch a game from that system
- [ ] Core Picker dialog appears
- [ ] Dialog shows both cores
- [ ] Select one and click "Launch" → game launches successfully
- [ ] Game runs without errors and can be closed cleanly

### Launch with One Core
- [ ] Map only 1 core to a system
- [ ] Launch a game from that system
- [ ] Core Picker dialog **does not** appear
- [ ] Game launches immediately with that core
- [ ] Game runs successfully

### Launch with No Cores
- [ ] Ensure a system has **no** mapped cores
- [ ] Launch a game from that system
- [ ] Error toast appears: "No libretro cores mapped for this system. Configure in Settings → Manage Core Mappings."
- [ ] No picker shown, game does not launch

---

## Final Build & Test Commands

```bash
cargo sqlx prepare --workspace -- --all-targets
tbls doc
cargo build
cargo clippy --all-targets
cargo test --verbose
```

All tests must pass, clippy must show zero warnings.
