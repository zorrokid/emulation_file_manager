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

**File:** `relm4-ui/src/libretro/core_mapping.rs` (new file)

**Relm4 Component:**

```rust
pub struct CoreMappingModel {
    app_services: Arc<AppServices>,
    systems: Vec<SystemListModel>,
    selected_system_index: u32,
    mapped_cores: Vec<(i64, String)>,
    available_cores: Vec<String>,
    selected_mapped_index: Option<u32>,
    selected_available_index: Option<u32>,
}

pub struct CoreMappingInit {
    pub app_services: Arc<AppServices>,
}

#[derive(Debug)]
pub enum CoreMappingMsg {
    Show,
    Hide,
    SystemSelected { index: u32 },
    MappedCoreSelected { index: u32 },
    AvailableCoreSelected { index: u32 },
    AddMapping,
    RemoveMapping,
}

#[derive(Debug)]
pub enum CoreMappingCommandMsg {
    SystemsLoaded(Result<Vec<SystemListModel>, Error>),
    CoresScanned(Result<Vec<String>, Error>),
    MappingsLoaded(Result<Vec<(i64, String)>, Error>),
    MappingAdded(Result<i64, Error>),
    MappingRemoved(Result<(), Error>),
}

impl Component for CoreMappingModel {
    type Init = CoreMappingInit;
    type Input = CoreMappingMsg;
    type Output = ();
    type CommandOutput = CoreMappingCommandMsg;
    // ... impl methods
}
```

**View Layout:**
```
gtk::Window "Manage Libretro Core Mappings"
  gtk::Box [vertical, spacing=10, margin=10]
    gtk::Box [horizontal]
      Label "System:"
      gtk::DropDown (systems)

    gtk::Box [horizontal, spacing=10]
      gtk::Box [vertical]
        Label "Mapped Cores" [bold]
        gtk::ScrolledWindow
          gtk::ListView (mapped_cores)
        Button "Remove Mapping" [sensitive only if mapped selected]

      gtk::Box [vertical]
        Label "Available Cores" [bold]
        gtk::ScrolledWindow
          gtk::ListView (available_cores)
        Button "Add Mapping" [sensitive only if available selected]

    gtk::Button "Close"
```

**Message Handlers:**

- `Show`:
  - Spawn two commands: `load_systems()` and `scan_cores()`
  - Show window when both complete
  - If no system loaded yet, select first system (if any)

- `SystemSelected { index }`:
  - Update selected_system_index
  - Spawn command: `load_mappings_for_system(systems[index].id)`
  - Clear available core selection

- `MappedCoreSelected { index }`:
  - Update selected_mapped_index
  - Disable "Add Mapping" button if no available selected

- `AvailableCoreSelected { index }`:
  - Update selected_available_index
  - Disable "Remove Mapping" button if no mapped selected

- `AddMapping`:
  - Get selected available core and current system ID
  - Spawn command: `add_core_mapping(system_id, core_name)`

- `RemoveMapping`:
  - Get selected mapped core ID
  - Spawn command: `remove_core_mapping(mapping_id)`

**Command Handlers (in `update_cmd`):**

- `SystemsLoaded(Ok(systems))`: Store systems, select first
- `CoresScanned(Ok(cores))`: Store available_cores
- `MappingsLoaded(Ok(cores))`: Store mapped_cores, clear selections
- `MappingAdded(Ok(_))`: Reload mappings for current system, clear selections
- `MappingAdded(Err(e))`: Show error toast
- `MappingRemoved(Ok(_))`: Reload mappings for current system
- `MappingRemoved(Err(e))`: Show error toast

**Acceptance:**
- [ ] Component compiles
- [ ] Dialog launches without panic
- [ ] Systems dropdown populates
- [ ] Available cores list populates with .so files from cores dir
- [ ] Mapped cores list populates for selected system

### Task 3.2: Create Core Picker Dialog

**File:** `relm4-ui/src/libretro/core_picker.rs` (new file)

**Relm4 Component:**

```rust
pub struct CorePickerModel {
    app_services: Arc<AppServices>,
    file_set_id: i64,
    cores: Vec<(i64, String)>,  // (mapping_id, core_name)
    selected_index: Option<u32>,
}

pub struct CorePickerInit {
    pub app_services: Arc<AppServices>,
}

#[derive(Debug)]
pub enum CorePickerMsg {
    Show { file_set_id: i64, cores: Vec<(i64, String)> },
    CoreSelected { index: u32 },
    Confirm,
    Cancel,
}

#[derive(Debug)]
pub enum CorePickerOutputMsg {
    CoreChosen { file_set_id: i64, core_name: String },
    Cancelled,
}

impl Component for CorePickerModel {
    type Init = CorePickerInit;
    type Input = CorePickerMsg;
    type Output = CorePickerOutputMsg;
    type CommandOutput = ();
    // ... impl methods
}
```

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

- `Show { file_set_id, cores }`:
  - Store file_set_id and cores
  - Clear selected_index
  - Show window

- `CoreSelected { index }`:
  - Update selected_index

- `Confirm`:
  - Get selected core name
  - Emit `CorePickerOutputMsg::CoreChosen { file_set_id, core_name }`
  - Hide window

- `Cancel`:
  - Emit `CorePickerOutputMsg::Cancelled`
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
pub mod core_mapping;
pub mod core_picker;

pub use core_mapping::{CoreMappingModel, CoreMappingInit, CoreMappingMsg};
pub use core_picker::{CorePickerModel, CorePickerInit, CorePickerMsg, CorePickerOutputMsg};
```

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
- [ ] Button appears in settings form
- [ ] Button is disabled when libretro_core_dir not set
- [ ] Button is enabled when libretro_core_dir is set
- [ ] Clicking button opens Core Mapping dialog

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
   StartLibretroRunner,
   CorePickerConfirmed { file_set_id: i64, core_name: String },
   CorePickerCancelled,
   ```

4. Add variant to `ReleaseCommandMsg`:
   ```rust
   CoresFetched { file_set_id: i64, result: Result<Vec<(i64, String)>, Error> },
   ```

5. Replace `StartLibretroRunner` handler:
   ```rust
   ReleaseMsg::StartLibretroRunner => {
       let system_id = self.selected_release.as_ref()
           .and_then(|r| r.systems.first())
           .map(|s| s.id);
       let file_set_id = self.selected_file_set.as_ref().map(|f| f.id);

       if let (Some(sid), Some(fid)) = (system_id, file_set_id) {
           sender.oneshot_command(async move {
               let cores = app_services
                   .libretro_core()
                   .get_cores_for_system(sid)
                   .await;
               ReleaseCommandMsg::CoresFetched {
                   file_set_id: fid,
                   result: cores,
               }
           });
       }
   }
   ```

6. Add handler in `update_cmd` for `CoresFetched`:
   ```rust
   ReleaseCommandMsg::CoresFetched { file_set_id, result } => {
       match result {
           Ok(cores) if cores.is_empty() => {
               // 0 cores: show error
               sender.input(ReleaseMsg::ShowErrorToast(
                   "No libretro cores mapped for this system. Configure in Settings → Manage Core Mappings.".into()
               ));
           },
           Ok(cores) if cores.len() == 1 => {
               // 1 core: proceed directly
               let (_, core_name) = cores[0].clone();
               sender.input(ReleaseMsg::CorePickerConfirmed { file_set_id, core_name });
           },
           Ok(cores) => {
               // 2+ cores: show picker
               self.core_picker.emit(CorePickerMsg::Show {
                   file_set_id,
                   cores,
               });
           },
           Err(e) => {
               sender.input(ReleaseMsg::ShowErrorToast(
                   format!("Failed to load cores: {}", e)
               ));
           },
       }
   }
   ```

7. Add handler for `CorePickerConfirmed`:
   ```rust
   ReleaseMsg::CorePickerConfirmed { file_set_id, core_name } => {
       sender.oneshot_command(async move {
           match app_services.libretro_runner().resolve_core_path(&core_name) {
               Ok(core_path) => {
                   match app_services.libretro_runner().prepare_rom(
                       LibretroLaunchModel {
                           file_set_id,
                           initial_file: None,
                           core_path,
                       }
                   ).await {
                       Ok(_) => ReleaseCommandMsg::LibretroRomPrepared(file_set_id),
                       Err(e) => ReleaseCommandMsg::LibretroPrepareFailed(e),
                   }
               },
               Err(e) => ReleaseCommandMsg::LibretroPrepareFailed(e),
           }
       });
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
