# AppServices Migration — Task Breakdown

**Branch**: `003-app-services-migration`
**Spec**: `specs/003-app-services-migration.md`
**Created**: 2026-02-20

---

## Phase 1: Create Domain Services and AppServices

### Task 1: Create SystemService
**Estimate**: 45 min
**Files**: `service/src/system_service.rs`, `service/src/lib.rs`

Create `service/src/system_service.rs`:

```rust
pub struct SystemService {
    repository_manager: Arc<RepositoryManager>,
}

impl SystemService {
    pub fn new(repository_manager: Arc<RepositoryManager>) -> Self
    pub async fn add_system(&self, name: &str) -> Result<i64, Error>
    pub async fn update_system(&self, id: i64, name: &str) -> Result<i64, Error>
    pub async fn delete_system(&self, id: i64) -> Result<(), Error>
}
```

Add `pub mod system_service;` to `service/src/lib.rs`.

**Automated test cases** (inline `#[cfg(test)]`, use `setup_test_repository_manager`):
- `add_system_returns_positive_id`
- `add_system_persists_name`
- `update_system_changes_name`
- `update_nonexistent_system_returns_error`
- `delete_system_removes_record`
- `delete_nonexistent_system_returns_error`

**Manual verification**: N/A (no GUI change in Phase 1)

**Dependencies**: None

---

### Task 2: Create ReleaseService
**Estimate**: 45 min
**Files**: `service/src/release_service.rs`, `service/src/lib.rs`

```rust
pub struct ReleaseService {
    repository_manager: Arc<RepositoryManager>,
}

impl ReleaseService {
    pub fn new(repository_manager: Arc<RepositoryManager>) -> Self
    pub async fn add_release(
        &self, name: &str, software_title_ids: &[i64],
        file_set_ids: &[i64], system_ids: &[i64],
    ) -> Result<i64, Error>
    pub async fn update_release(
        &self, id: i64, name: &str, software_title_ids: &[i64],
        file_set_ids: &[i64], system_ids: &[i64],
    ) -> Result<i64, Error>
    pub async fn delete_release(&self, id: i64) -> Result<i64, Error>
}
```

Add `pub mod release_service;` to `service/src/lib.rs`.

**Automated test cases**:
- `add_release_returns_positive_id`
- `add_release_links_all_associations`
- `update_release_changes_name`
- `update_release_replaces_system_associations`
- `update_release_replaces_software_title_associations`
- `update_release_replaces_file_set_associations`
- `delete_release_removes_record`

**Manual verification**: N/A

**Dependencies**: None

---

### Task 3: Create ReleaseItemService
**Estimate**: 45 min
**Files**: `service/src/release_item_service.rs`, `service/src/lib.rs`

```rust
pub struct ReleaseItemService {
    repository_manager: Arc<RepositoryManager>,
}

impl ReleaseItemService {
    pub fn new(repository_manager: Arc<RepositoryManager>) -> Self
    pub async fn create_item(
        &self, release_id: i64, item_type: ItemType, notes: Option<String>,
    ) -> Result<i64, Error>
    pub async fn get_item(&self, item_id: i64) -> Result<ReleaseItem, Error>
    pub async fn update_item(
        &self, item_id: i64, item_type: ItemType, notes: Option<String>,
    ) -> Result<i64, Error>
    pub async fn delete_item(&self, item_id: i64) -> Result<(), Error>
}
```

Add `pub mod release_item_service;` to `service/src/lib.rs`.

**Automated test cases**:
- `create_item_returns_positive_id`
- `get_item_returns_correct_item_type_and_notes`
- `update_item_changes_item_type`
- `update_item_clears_notes_when_none`
- `delete_item_removes_record`
- `get_nonexistent_item_returns_error`
- `delete_nonexistent_item_returns_error`

**Manual verification**: N/A

**Dependencies**: `ReleaseItemRepository` (from spec 001 — already implemented)

---

### Task 4: Add mutation methods to SoftwareTitleService
**Estimate**: 30 min
**Files**: `service/src/software_title_service.rs`

Add to the existing `SoftwareTitleService` struct:

```rust
pub async fn add_software_title(&self, name: &str) -> Result<i64, Error>
pub async fn update_software_title(&self, id: i64, name: &str) -> Result<i64, Error>
pub async fn delete_software_title(&self, id: i64) -> Result<i64, Error>
```

**Automated test cases** (add to existing test module):
- `add_software_title_returns_positive_id`
- `add_software_title_persists`
- `update_software_title_changes_name`
- `update_nonexistent_software_title_returns_error`
- `delete_software_title_removes_record`
- `delete_nonexistent_software_title_returns_error`

**Manual verification**: N/A

**Dependencies**: None

---

### Task 5: Create EmulatorService
**Estimate**: 45 min
**Files**: `service/src/emulator_service.rs`, `service/src/lib.rs`

```rust
pub struct EmulatorService {
    repository_manager: Arc<RepositoryManager>,
}

impl EmulatorService {
    pub fn new(repository_manager: Arc<RepositoryManager>) -> Self
    pub async fn add_emulator(
        &self, name: &str, executable: &str, extract_files: bool,
        arguments: &[ArgumentType], system_id: i64,
    ) -> Result<i64, Error>
    pub async fn update_emulator(
        &self, id: i64, name: &str, executable: &str, extract_files: bool,
        arguments: &[ArgumentType], system_id: i64,
    ) -> Result<i64, Error>
    pub async fn delete_emulator(&self, id: i64) -> Result<i64, Error>
}
```

The service serializes `arguments` via `serde_json::to_string` before passing to the repository.

Add `pub mod emulator_service;` to `service/src/lib.rs`.

**Automated test cases**:
- `add_emulator_returns_positive_id`
- `add_emulator_with_empty_arguments_persists`
- `add_emulator_with_arguments_round_trips`
- `update_emulator_changes_name_and_executable`
- `update_emulator_changes_arguments`
- `delete_emulator_removes_record`
- `delete_nonexistent_emulator_returns_error`

**Manual verification**: N/A

**Dependencies**: None

---

### Task 6: Create DocumentViewerService
**Estimate**: 45 min
**Files**: `service/src/document_viewer_service.rs`, `service/src/lib.rs`

```rust
pub struct DocumentViewerService {
    repository_manager: Arc<RepositoryManager>,
}

impl DocumentViewerService {
    pub fn new(repository_manager: Arc<RepositoryManager>) -> Self
    pub async fn add_document_viewer(
        &self, name: &str, executable: &str, arguments: &[ArgumentType],
        document_type: &DocumentType, cleanup_temp_files: bool,
    ) -> Result<i64, Error>
    pub async fn update_document_viewer(
        &self, id: i64, name: &str, executable: &str, arguments: &[ArgumentType],
        document_type: &DocumentType, cleanup_temp_files: bool,
    ) -> Result<i64, Error>
    pub async fn delete_document_viewer(&self, id: i64) -> Result<i64, Error>
}
```

The service serializes `arguments` via `serde_json::to_string`.

Add `pub mod document_viewer_service;` to `service/src/lib.rs`.

**Automated test cases**:
- `add_document_viewer_returns_positive_id`
- `add_document_viewer_persists_all_fields`
- `add_document_viewer_with_arguments_round_trips`
- `update_document_viewer_changes_fields`
- `delete_document_viewer_removes_record`
- `delete_nonexistent_document_viewer_returns_error`

**Manual verification**: N/A

**Dependencies**: None

---

### Task 7: Create AppServices struct
**Estimate**: 20 min
**Files**: `service/src/app_services.rs`, `service/src/lib.rs`

```rust
pub struct AppServices {
    pub view_model: Arc<ViewModelService>,
    pub system: Arc<SystemService>,
    pub release: Arc<ReleaseService>,
    pub release_item: Arc<ReleaseItemService>,
    pub software_title: Arc<SoftwareTitleService>,
    pub emulator: Arc<EmulatorService>,
    pub document_viewer: Arc<DocumentViewerService>,
}

impl AppServices {
    pub fn new(repository_manager: Arc<RepositoryManager>) -> Self {
        Self {
            view_model: Arc::new(ViewModelService::new(Arc::clone(&repository_manager))),
            system: Arc::new(SystemService::new(Arc::clone(&repository_manager))),
            release: Arc::new(ReleaseService::new(Arc::clone(&repository_manager))),
            release_item: Arc::new(ReleaseItemService::new(Arc::clone(&repository_manager))),
            software_title: Arc::new(SoftwareTitleService::new(Arc::clone(&repository_manager))),
            emulator: Arc::new(EmulatorService::new(Arc::clone(&repository_manager))),
            document_viewer: Arc::new(DocumentViewerService::new(Arc::clone(&repository_manager))),
        }
    }
}
```

Add `pub mod app_services;` to `service/src/lib.rs`.

**Automated test cases**: None — pure constructor aggregator.

**Manual verification**: N/A

**Dependencies**: Tasks 1–6

---

## Phase 2: Migrate GUI Components

All Phase 2 tasks require Phase 1 complete and `cargo build` passing. Tasks must be done in
leaf-first order — a child component's `Init` must accept `Arc<AppServices>` before the parent
can be migrated.

---

### Task 8: Migrate `system_form.rs`
**Estimate**: 30 min
**Files**: `relm4-ui/src/system_form.rs`

- `SystemFormInit`: replace `repository_manager: Arc<RepositoryManager>` with `app_services: Arc<AppServices>`
- `SystemFormModel`: replace `repository_manager` field with `app_services`
- Submit (new): `app_services.system.add_system(&name).await`
- Submit (edit): `app_services.system.update_system(edit_id, &name).await`
- Update `SystemFormCommandMsg` error type to `service::error::Error`
- Remove `use database::{database_error::Error, repository_manager::RepositoryManager};`

**Manual verification**:
- [ ] System form opens blank when creating
- [ ] Enter a name, Submit — new system appears in list
- [ ] Edit: form opens with current name pre-filled
- [ ] Change name, Submit — list item updates
- [ ] Submit disabled when name is empty

**Dependencies**: Task 7

---

### Task 9: Migrate `system_selector.rs` and `release_form_components/system_list.rs`
**Estimate**: 30 min
**Files**: `relm4-ui/src/system_selector.rs`, `relm4-ui/src/release_form_components/system_list.rs`

`system_selector.rs`:
- `SystemSelectInit`: replace `view_model_service` and `repository_manager` with `app_services: Arc<AppServices>`
- `SystemSelectModel`: replace both fields with `app_services`
- `FetchSystems`: use `Arc::clone(&self.app_services.view_model)`
- `DeleteConfirmed`: `app_services.system.delete_system(id).await`
- `SystemFormInit` construction: pass `app_services: Arc::clone(&self.app_services)`

`system_list.rs`:
- `SystemListInit`: replace `view_model_service` and `repository_manager` with `app_services: Arc<AppServices>`
- `SystemSelectInit` construction: pass `app_services`

**Manual verification**:
- [ ] System selector opens with populated list
- [ ] Delete disabled for systems in use; enabled for unused ones
- [ ] Confirming deletion removes system from list
- [ ] Add and Edit open system form correctly

**Dependencies**: Task 8

---

### Task 10: Migrate `software_title_form.rs`
**Estimate**: 30 min
**Files**: `relm4-ui/src/software_title_form.rs`

- `SoftwareTitleFormInit`: replace `repository_manager` with `app_services: Arc<AppServices>`
- `SoftwareTitleFormModel`: replace `repository_manager` field with `app_services`
- Submit (new): `app_services.software_title.add_software_title(&name).await`
- Submit (edit): `app_services.software_title.update_software_title(edit_id, &name).await`
- Update `SoftwareTitleFormCommandMsg` error type to `service::error::Error`
- Remove `use database::{database_error::Error, repository_manager::RepositoryManager};`

**Manual verification**:
- [ ] Form opens blank for new title
- [ ] Submit creates title, which appears in selector list
- [ ] Edit opens form with current name
- [ ] Submit updates name in list
- [ ] Submit disabled when name empty

**Dependencies**: Task 7

---

### Task 11: Migrate `software_title_selector.rs` and `release_form_components/software_title_list.rs`
**Estimate**: 30 min
**Files**: `relm4-ui/src/software_title_selector.rs`, `relm4-ui/src/release_form_components/software_title_list.rs`

`software_title_selector.rs`:
- Replace `view_model_service` and `repository_manager` with `app_services: Arc<AppServices>`
- `FetchSoftwareTitles`: use `Arc::clone(&self.app_services.view_model)`
- `DeleteConfirmed`: `app_services.software_title.delete_software_title(id).await`
- `SoftwareTitleFormInit` construction: pass `app_services`

`software_title_list.rs`:
- Replace `view_model_service` and `repository_manager` with `app_services: Arc<AppServices>`
- `SoftwareTitleSelectInit` construction: pass `app_services`

**Manual verification**:
- [ ] Software title selector opens with all titles
- [ ] Delete respects `can_delete` flag
- [ ] Confirming deletion removes title
- [ ] Add and Edit open software title form

**Dependencies**: Task 10

---

### Task 12: Migrate `release_form_components/item_form.rs`
**Estimate**: 30 min
**Files**: `relm4-ui/src/release_form_components/item_form.rs`

- `ItemFormInit`: replace `repository_manager` with `app_services: Arc<AppServices>`
- `ItemForm` struct: replace `repository_manager` field with `app_services`
- CreateOrUpdateItem (new): `app_services.release_item.create_item(release_id, item_type, notes).await`
- CreateOrUpdateItem (edit): `app_services.release_item.update_item(edit_item_id, item_type, notes).await`
- Show (edit): `app_services.release_item.get_item(edit_item_id).await`
- Update `ItemFormCommandMsg` error types to `service::error::Error`
- Remove `use database::{database_error::Error, models::ReleaseItem, repository_manager::RepositoryManager};`

**Manual verification**:
- [ ] "Add Item" opens blank form with item type dropdown
- [ ] Select type, add notes, Submit — item appears in list
- [ ] "Edit Item" opens form pre-filled with type and notes
- [ ] Submit updates item in list
- [ ] Error dialog shown on failure

**Dependencies**: Task 3 (ReleaseItemService)

---

### Task 13: Migrate `release_form_components/item_list.rs`
**Estimate**: 20 min
**Files**: `relm4-ui/src/release_form_components/item_list.rs`

- `ItemListInit`: replace `repository_manager` with `app_services: Arc<AppServices>`
- `ItemList` struct: replace `repository_manager` field with `app_services`
- `RemoveItem`: `app_services.release_item.delete_item(item_id).await`
- `ItemFormInit` construction: pass `app_services: Arc::clone(&self.app_services)`
- Update `ItemListCommandMsg` error type to `service::error::Error`
- Remove `use database::{database_error::Error, repository_manager::RepositoryManager};`

**Manual verification**:
- [ ] When `release_id` is `None`, "Please create a release first" shown
- [ ] Items list correctly when release exists
- [ ] Add/Edit Item buttons open item form
- [ ] Delete removes item from list immediately

**Dependencies**: Task 12

---

### Task 14: Migrate `document_viewer_form.rs`
**Estimate**: 30 min
**Files**: `relm4-ui/src/document_viewer_form.rs`

- `DocumentViewerFormInit`: replace `repository_manager` with `app_services: Arc<AppServices>`
- `DocumentViewerFormModel`: replace `repository_manager` field with `app_services`
- Submit (new): `app_services.document_viewer.add_document_viewer(...).await`
- Submit (edit): `app_services.document_viewer.update_document_viewer(...).await`
- Update `DocumentViewerFormCommandMsg` error types to `service::error::Error`
- Remove `use database::{database_error::DatabaseError, repository_manager::RepositoryManager};`

**Manual verification**:
- [ ] Form opens with empty fields for new viewer
- [ ] All fields (name, executable, document type, arguments, cleanup flag) saved on Submit
- [ ] Edit mode pre-fills all fields correctly
- [ ] Cleanup checkbox state preserved on submit
- [ ] Error dialog on failure

**Dependencies**: Task 6

---

### Task 15: Migrate `document_file_set_viewer.rs`
**Estimate**: 45 min
**Files**: `relm4-ui/src/document_file_set_viewer.rs`

The current code constructs `ExternalExecutableRunnerService` inside `init` using
`Arc<RepositoryManager>` and `Arc<Settings>`. After migration, pass it in from the parent.

- `DocumentViewerInit`: replace `view_model_service` and `repository_manager` with `app_services: Arc<AppServices>`; add `executable_runner: Arc<ExternalExecutableRunnerService>`
- `DocumentViewer` model: replace both fields with `app_services`; store `executable_runner`
- Remove in-`init` construction of `ExternalExecutableRunnerService`; use `init.executable_runner`
- `DocumentViewerFormInit` construction: pass `app_services: Arc::clone(&self.app_services)`
- `FetchViewers`: use `Arc::clone(&self.app_services.view_model)`
- `DeleteConfirmed`: `app_services.document_viewer.delete_document_viewer(viewer_id).await`
- Update all call sites to pass `executable_runner` in `DocumentViewerInit`

**Manual verification**:
- [ ] Document viewer window opens with file list for current file set
- [ ] Viewer list populates with available document viewers
- [ ] Add/Edit/Delete viewer buttons work correctly
- [ ] Selecting file + viewer and clicking Start launches the viewer
- [ ] Cleanup temp files behavior preserved

**Dependencies**: Task 14

---

### Task 16: Migrate `emulator_form.rs`
**Estimate**: 30 min
**Files**: `relm4-ui/src/emulator_form.rs`

- `EmulatorFormInit`: replace `view_model_service` and `repository_manager` with `app_services: Arc<AppServices>`
- `EmulatorFormModel`: replace both fields with `app_services`
- `SystemSelectInit` construction: pass `app_services: Arc::clone(&self.app_services)`
- Submit (new): `app_services.emulator.add_emulator(...).await`
- Submit (edit): `app_services.emulator.update_emulator(...).await`
- Update `EmulatorFormCommandMsg` error types to `service::error::Error`
- Remove `use database::{database_error::DatabaseError, repository_manager::RepositoryManager};`

**Manual verification**:
- [ ] Emulator form opens with empty fields
- [ ] System selector opens; selected system name shown in form
- [ ] Submit creates emulator with all fields
- [ ] Edit mode pre-fills all fields; submitting updates them
- [ ] Submit disabled until executable and system are filled

**Dependencies**: Tasks 5 and 9 (EmulatorService + SystemSelectInit accepting AppServices)

---

### Task 17: Migrate `emulator_runner.rs`
**Estimate**: 45 min
**Files**: `relm4-ui/src/emulator_runner.rs`

Same approach as Task 15: `ExternalExecutableRunnerService` is passed in from parent.

- `EmulatorRunnerInit`: replace `view_model_service` and `repository_manager` with `app_services: Arc<AppServices>`; add `executable_runner: Arc<ExternalExecutableRunnerService>`
- `EmulatorRunnerModel`: replace both fields with `app_services`; store `executable_runner`
- Remove in-`init` construction of `ExternalExecutableRunnerService`
- `EmulatorFormInit` construction: pass `app_services: Arc::clone(&self.app_services)`
- `FetchEmulators`: use `Arc::clone(&self.app_services.view_model)`
- `DeleteConfirmed`: `app_services.emulator.delete_emulator(emulator_id).await`
- Update all call sites to pass `executable_runner`

**Manual verification**:
- [ ] Emulator runner opens with file list from current file set
- [ ] Emulator list populates for selected system
- [ ] Add/Edit/Delete emulator buttons work correctly
- [ ] Start Emulator launches selected emulator with selected file
- [ ] Delete confirmation dialog appears and works

**Dependencies**: Task 16

---

### Task 18: Migrate `release_form.rs`
**Estimate**: 45 min
**Files**: `relm4-ui/src/release_form.rs`

- `ReleaseFormInit`: replace `view_model_service` and `repository_manager` with `app_services: Arc<AppServices>`; keep `settings: Arc<Settings>`
- `ReleaseFormModel`: replace both fields with `app_services`
- Update all child `Init` constructions to pass `app_services: Arc::clone(&app_services)`:
  - `SystemListInit`, `SoftwareTitleListInit`, `ItemListInit`
  - `FileSetListInit`: keep `repository_manager` temporarily if its child selector is still out of scope
- `Show { release_id: Some(id) }`: use `Arc::clone(&self.app_services.view_model)` in `oneshot_command`
- StartSaveRelease (new): `app_services.release.add_release(...).await`
- StartSaveRelease (edit): `app_services.release.update_release(...).await`
- Update `ReleaseFormCommandMsg` error types to `service::error::Error`
- Remove `use database::{database_error::Error, repository_manager::RepositoryManager};`

**Manual verification**:
- [ ] Release form opens with empty fields
- [ ] All four tabs (Software Titles, Systems, File Sets, Items) load and function
- [ ] Submit without systems shows validation message
- [ ] Submit without file sets shows validation message
- [ ] Submit with required associations creates the release
- [ ] Edit mode pre-populates all tabs
- [ ] Edit and submit updates the release correctly

**Dependencies**: Tasks 2, 11, 13 (ReleaseService + SoftwareTitleListInit + ItemListInit accepting AppServices)

---

### Task 19: Migrate `releases.rs`
**Estimate**: 30 min
**Files**: `relm4-ui/src/releases.rs`

- `ReleasesInit`: replace `view_model_service` and `repository_manager` with `app_services: Arc<AppServices>`; keep `settings: Arc<Settings>`
- `ReleasesModel`: replace both fields with `app_services`
- `ReleaseFormInit` construction: pass `app_services: Arc::clone(&self.app_services), settings: Arc::clone(&self.settings)`
- `FetchReleases`: use `Arc::clone(&self.app_services.view_model)`
- `RemoveRelease`: `app_services.release.delete_release(release_id).await`
- Update `ReleasesCommandMsg` error type to `service::error::Error`
- Remove `use database::{database_error::DatabaseError, repository_manager::RepositoryManager};`

**Manual verification**:
- [ ] Releases list empty when no software title selected
- [ ] Selecting a software title populates releases list
- [ ] Add Release opens blank release form
- [ ] Edit Release opens form pre-filled with selected release
- [ ] Remove Release updates list after deletion
- [ ] Error shown when delete fails

**Dependencies**: Task 18

---

### Task 20: Update `app.rs` to construct and pass AppServices
**Estimate**: 45 min
**Files**: `relm4-ui/src/app.rs`

In `post_process_initialize`:
- Construct `let app_services = Arc::new(AppServices::new(Arc::clone(&repository_manager)));`
- Update `ReleasesInit` to pass `app_services: Arc::clone(&app_services)`
- Add `app_services: OnceCell<Arc<AppServices>>` field to `AppModel`; store the instance
- `RepositoryManager` remains in `AppModel` for out-of-scope infrastructure services (export, sync, import, settings)
- Add `use service::app_services::AppServices;`

**Manual verification**:
- [ ] Application starts without panics
- [ ] Software titles list loads on startup
- [ ] Selecting a software title loads releases
- [ ] Add/edit/delete for releases, systems, software titles, emulators, and document viewers all work end-to-end

**Dependencies**: Task 19 and all prior Phase 2 tasks

---

## Phase 3: Intermediate Cleanup

### Task 21: Intermediate lint and verification
**Estimate**: 20 min

- Run `cargo clippy --all-targets` — fix all warnings introduced by Phases 1–2
- Run `cargo test --workspace` — verify all existing tests pass
- Confirm zero `use database::repository_manager::RepositoryManager` imports in the 13 Phase 2 migrated files
- Note: `database` dependency stays in `relm4-ui/Cargo.toml` — Phase 4 files still need it

**Manual verification**:
- [ ] Launch application; software titles load
- [ ] Add/edit/delete a release, system, software title, emulator, document viewer — all work
- [ ] Cloud sync, export, and import continue to work (smoke test only — Phase 4 not started)

**Dependencies**: Task 20

---

## Phase 4: Pipeline Services and Remaining GUI Files

### Task 22: Extend AppServices with pipeline services
**Estimate**: 30 min
**Files**: `service/src/app_services.rs`

Update `AppServices` to include all pipeline services. The constructor signature gains
`Arc<Settings>` because `FileImportService`, `MassImportService`, and `CloudStorageSyncService`
require it at construction time.

```rust
pub struct AppServices {
    // existing Phase 1 fields unchanged ...
    pub settings: Arc<SettingsService>,
    pub file_import: Arc<FileImportService>,
    pub mass_import: Arc<MassImportService>,
    pub cloud_sync: Arc<CloudStorageSyncService>,
    pub export: Arc<ExportService>,
}

impl AppServices {
    pub fn new(repository_manager: Arc<RepositoryManager>, settings: Arc<Settings>) -> Self {
        Self {
            // existing fields ...
            settings: Arc::new(SettingsService::new(Arc::clone(&repository_manager))),
            file_import: Arc::new(FileImportService::new(Arc::clone(&repository_manager), Arc::clone(&settings))),
            mass_import: Arc::new(MassImportService::new(Arc::clone(&repository_manager), Arc::clone(&settings))),
            cloud_sync: Arc::new(CloudStorageSyncService::new(Arc::clone(&repository_manager), Arc::clone(&settings))),
            export: Arc::new(ExportService::new(Arc::clone(&repository_manager))),
        }
    }
}
```

Update the call site in `app.rs` (Task 20):
```rust
// Before:
let app_services = Arc::new(AppServices::new(Arc::clone(&repository_manager)));
// After:
let app_services = Arc::new(AppServices::new(Arc::clone(&repository_manager), Arc::clone(&settings)));
```

**Automated test cases**: None — pure constructor aggregator.

**Manual verification**: N/A

**Dependencies**: Task 20 (app.rs call site must be updated together)

---

### Task 23: Migrate `settings_form.rs`
**Estimate**: 20 min
**Files**: `relm4-ui/src/settings_form.rs`

Currently constructs `SettingsService::new(Arc::clone(&init.repository_manager))` in `init`.

- `SettingsFormInit`: replace `repository_manager: Arc<RepositoryManager>` with `app_services: Arc<AppServices>`
- `SettingsFormModel`: replace `repository_manager` and internal `settings_service` construction with `app_services`
- Use `Arc::clone(&init.app_services.settings)` in place of constructing a new `SettingsService`
- Remove `use database::repository_manager::RepositoryManager;`

**Manual verification**:
- [ ] Settings form opens with current settings pre-filled
- [ ] Changing settings and saving persists values
- [ ] Cloud storage settings section shows/hides correctly
- [ ] Reopening settings form shows saved values

**Dependencies**: Task 22

---

### Task 24: Migrate `software_titles_list.rs`
**Estimate**: 20 min
**Files**: `relm4-ui/src/software_titles_list.rs`

Currently passes `&init_model.repository_manager` to a child component (merge dialog or selector).

- `SoftwareTitleListInit` (the top-level app component, not the release form subcomponent): replace `repository_manager: Arc<RepositoryManager>` with `app_services: Arc<AppServices>`
- Update child Init construction to pass `app_services`
- Remove `use database::repository_manager::RepositoryManager;`

**Manual verification**:
- [ ] Software titles list loads on application startup
- [ ] Merge software titles dialog opens and works correctly
- [ ] Merged titles consolidate in the list

**Dependencies**: Task 22

---

### Task 25: Migrate `release.rs`
**Estimate**: 30 min
**Files**: `relm4-ui/src/release.rs`

Currently passes `repository_manager` to three child Init structs (lines 203, 218, 236) —
likely `FileSetFormInit`, `EmulatorRunnerInit`, and `DocumentViewerInit`.

- `ReleaseInit`: replace `repository_manager: Arc<RepositoryManager>` with `app_services: Arc<AppServices>`
- `ReleaseModel`: replace `repository_manager` field with `app_services`
- Update all three child Init constructions to pass `app_services: Arc::clone(&self.app_services)`
- Remove `use database::repository_manager::RepositoryManager;`

**Manual verification**:
- [ ] Selecting a release loads its detail view
- [ ] File set viewer opens and shows files
- [ ] Emulator runner opens for the selected release
- [ ] Document viewer opens for document file sets
- [ ] All child components function correctly

**Dependencies**: Tasks 15, 17, 22 (DocumentViewerInit, EmulatorRunnerInit, and FileSetFormInit must all accept AppServices before their parent can be migrated — FileSetFormInit is Task 26)

---

### Task 26: Migrate `file_set_form.rs`
**Estimate**: 45 min
**Files**: `relm4-ui/src/file_set_form.rs`

Currently constructs `FileImportService::new(Arc::clone(&init_model.repository_manager), ...)` in
`init` (lines 429–430) and also passes `repository_manager` to another child Init (line 435).

- `FileSetFormInit`: replace `repository_manager: Arc<RepositoryManager>` with `app_services: Arc<AppServices>`
- `FileSetFormModel`: replace `repository_manager` field with `app_services`; replace inline
  construction of `FileImportService` with `Arc::clone(&init_model.app_services.file_import)`
- Update the child Init at line 435 to pass `app_services`
- Remove `use database::repository_manager::RepositoryManager;`

**Manual verification**:
- [ ] File set form opens for an existing file set
- [ ] Adding a new file set works end-to-end (file selection → import → appears in release)
- [ ] Editing an existing file set updates correctly
- [ ] File type dropdown functions correctly
- [ ] Item selection (if shown) works correctly

**Dependencies**: Task 22

---

### Task 27: Migrate `import_form.rs`
**Estimate**: 30 min
**Files**: `relm4-ui/src/import_form.rs`

Currently constructs `MassImportService::new(Arc::clone(&init_model.repository_manager), ...)`
in `init` (lines 252–253) and also stores `repository_manager` directly in the model (line 259)
for re-use during the import operation.

- `ImportFormInit`: replace `repository_manager: Arc<RepositoryManager>` with `app_services: Arc<AppServices>`
- `ImportFormModel`: replace `repository_manager` and inline `MassImportService` construction
  with `app_services`; replace model's `repository_manager` field with `app_services`
- All uses of `self.repository_manager` during import execution: replace with
  `Arc::clone(&self.app_services.mass_import)` or the appropriate `app_services` field
- Remove `use database::repository_manager::RepositoryManager;`

**Manual verification**:
- [ ] Import form opens with folder/path selection
- [ ] Selecting a folder and starting import runs without errors
- [ ] Progress events display correctly during import
- [ ] Import completes and new releases/file sets appear in the list
- [ ] Error messages display correctly on import failure

**Dependencies**: Task 22

---

### Task 28: Update `app.rs` AppServices construction for Phase 4
**Estimate**: 15 min
**Files**: `relm4-ui/src/app.rs`

- Update the `AppServices::new` call to pass `Arc::clone(&settings)` as the second argument
- Update `SoftwareTitleListInit` and `ReleaseInit` constructions to pass `app_services` (if not already done in Task 20)
- `RepositoryManager` can be removed from `AppModel` if no remaining code in `app.rs` uses it directly; otherwise leave it with a `// TODO: remove after all pipeline callers migrated` comment

**Manual verification**:
- [ ] Application starts without panics
- [ ] All features from Phase 2 smoke test continue to work
- [ ] Import form works end-to-end
- [ ] Cloud sync triggers without error

**Dependencies**: Tasks 22–27

---

## Phase 5: Final Cleanup

### Task 29: Remove `database` from `relm4-ui/Cargo.toml`
**Estimate**: 20 min
**Files**: `relm4-ui/Cargo.toml`

- Run `cargo build -p relm4-ui` and confirm zero `use database::` imports remain in any GUI file
- Remove `database` from `relm4-ui/Cargo.toml`
- Confirm `cargo build` still succeeds — the compiler now enforces the layer boundary

**Dependencies**: Task 28

---

### Task 30: Final lint and smoke test
**Estimate**: 20 min

- Run `cargo clippy --all-targets` — no warnings
- Run `cargo test --workspace` — all tests pass
- Confirm zero `use database::repository_manager::RepositoryManager` in all GUI files

**Manual verification (full smoke test)**:
- [ ] Launch application; software titles load
- [ ] Add, edit, delete a release — works
- [ ] Run mass import — works end-to-end
- [ ] Open emulator runner and launch a file — works
- [ ] Open document viewer and open a document — works
- [ ] Trigger cloud sync — works
- [ ] Export a file set — works
- [ ] Open settings, change a value, reopen — persists

**Dependencies**: Task 29

---

## Summary

**Phase 1** (Tasks 1–7): ~4.5 hours
Create all domain services with full test coverage. No GUI files touched.

**Phase 2** (Tasks 8–20): ~7 hours
Migrate 13 GUI components in leaf-first order.

**Phase 3** (Task 21): ~20 min
Intermediate lint and verification checkpoint.

**Phase 4** (Tasks 22–28): ~3 hours
Extend `AppServices` with pipeline services; migrate the 5 remaining GUI files.

**Phase 5** (Tasks 29–30): ~40 min
Remove `database` from `relm4-ui/Cargo.toml`; final smoke test. At this point the layer
boundary is enforced at the compiler level — `relm4-ui` cannot import `database` types.

**Total estimate**: ~15.5 hours

## Migration Order Dependency Graph

```
Tasks 1-6 (domain services, parallel)
    └── Task 7 (AppServices v1)
            ├── Task 8 (system_form)
            │       └── Task 9 (system_selector + system_list)
            ├── Task 10 (software_title_form)
            │       └── Task 11 (software_title_selector + software_title_list)
            ├── Task 3 → Task 12 (item_form)
            │               └── Task 13 (item_list)
            ├── Task 6 → Task 14 (document_viewer_form)
            │               └── Task 15 (document_file_set_viewer)
            └── Task 5 + Task 9 → Task 16 (emulator_form)
                                        └── Task 17 (emulator_runner)

Tasks 2 + 11 + 13 → Task 18 (release_form)
                          └── Task 19 (releases)
                                    └── Task 20 (app.rs v1)
                                              └── Task 21 (checkpoint)

Task 21 → Task 22 (AppServices v2 — pipeline services added)
    ├── Task 23 (settings_form)
    ├── Task 24 (software_titles_list)
    ├── Task 26 (file_set_form)
    ├── Task 27 (import_form)
    └── Tasks 15 + 17 + 26 → Task 25 (release.rs)
                                    └── Task 28 (app.rs v2)
                                              └── Task 29 → Task 30
```
