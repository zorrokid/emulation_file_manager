# AppServices Migration Specification

**Feature**: Introduce `AppServices` to eliminate layer violations in the GUI
**Branch**: `003-app-services-migration`
**Created**: 2026-02-20
**Related**: `CLAUDE.md` (4-Layer Architecture section)

## Overview

The GUI (`relm4-ui`) currently imports and directly calls `RepositoryManager` from the `database`
crate in 13 components. This violates the project's 4-layer architecture, which requires all
database access to flow through the `service` layer. The `relm4-ui` crate must never depend on
`database` types for business logic — only the `service` and `database` crates should have that
relationship.

The fix introduces an `AppServices` struct that aggregates all domain services into a single shared
handle. Each GUI component's `Init` struct is updated to accept `Arc<AppServices>` instead of
carrying `Arc<RepositoryManager>`. The domain services wrap the repository calls that currently
live directly in GUI code.

## Goals

1. Remove all `Arc<RepositoryManager>` fields from GUI `Init` and model structs in all affected
   components.
2. Create named domain services in the `service` crate for each repository operation currently
   called directly from GUI code.
3. Introduce `AppServices` as the single service aggregate passed into all GUI components that
   need mutation or deletion capabilities beyond what `ViewModelService` already provides.
4. Preserve all current GUI behavior exactly — this is a pure refactor with no functional changes.
5. After migration, the migrated GUI files must contain zero
   `use database::repository_manager::RepositoryManager` imports.

## Current State

The following GUI files import and use `RepositoryManager` directly from the `database` crate.
Repository method calls are listed next to each component.

### `system_form.rs` — `SystemFormModel`
- `repository_manager.get_system_repository().add_system(&name)` (Submit → new)
- `repository_manager.get_system_repository().update_system(edit_id, &name)` (Submit → edit)

### `system_selector.rs` — `SystemSelectModel`
- `repository_manager.get_system_repository().delete_system(id)` (DeleteConfirmed)
- Passes `Arc<RepositoryManager>` down to `SystemFormInit`

### `software_title_form.rs` — `SoftwareTitleFormModel`
- `repository_manager.get_software_title_repository().add_software_title(&name, None)` (Submit → new)
- `repository_manager.get_software_title_repository().update_software_title(&update)` (Submit → edit)

### `software_title_selector.rs` — `SoftwareTitleSelectModel`
- `repository_manager.get_software_title_repository().delete_software_title(id)` (DeleteConfirmed)
- Passes `Arc<RepositoryManager>` down to `SoftwareTitleFormInit`

### `release_form_components/item_form.rs` — `ItemForm`
- `repository_manager.get_release_item_repository().create_item(release_id, item_type, notes)` (CreateOrUpdateItem → new)
- `repository_manager.get_release_item_repository().update_item(edit_item_id, item_type, notes)` (CreateOrUpdateItem → edit)
- `repository_manager.get_release_item_repository().get_item(edit_item_id)` (Show with edit_item_id)

### `release_form_components/item_list.rs` — `ItemList`
- `repository_manager.get_release_item_repository().delete_item(item_id)` (RemoveItem)
- Passes `Arc<RepositoryManager>` down to `ItemFormInit`

### `emulator_form.rs` — `EmulatorFormModel`
- `repository_manager.get_emulator_repository().add_emulator(&name, &executable, extract_files, &arguments, system_id)` (Submit → new)
- `repository_manager.get_emulator_repository().update_emulator(id, &name, &executable, extract_files, &arguments, system_id)` (Submit → edit)
- Passes `Arc<RepositoryManager>` down to `SystemSelectInit`

### `emulator_runner.rs` — `EmulatorRunnerModel`
- `repository_manager.get_emulator_repository().delete_emulator(emulator_id)` (DeleteConfirmed)
- Passes `Arc<RepositoryManager>` down to `EmulatorFormInit`

### `document_viewer_form.rs` — `DocumentViewerFormModel`
- `repository_manager.get_document_viewer_repository().add_document_viewer(...)` (Submit → new)
- `repository_manager.get_document_viewer_repository().update_document_viewer(...)` (Submit → edit)

### `document_file_set_viewer.rs` — `DocumentViewer`
- `repository_manager.get_document_viewer_repository().delete(viewer_id)` (DeleteConfirmed)
- Passes `Arc<RepositoryManager>` down to `DocumentViewerFormInit`

### `release_form.rs` — `ReleaseFormModel`
- `repository_manager.get_release_repository().add_release_full(...)` (StartSaveRelease → new)
- `repository_manager.get_release_repository().update_release_full(...)` (StartSaveRelease → edit)
- Passes `Arc<RepositoryManager>` down to `FileSetListInit`, `SystemListInit`, `SoftwareTitleListInit`, `ItemListInit`

### `releases.rs` — `ReleasesModel`
- `repository_manager.get_release_repository().delete_release(release_id)` (RemoveRelease)
- Passes `Arc<RepositoryManager>` down to `ReleaseFormInit`

### `release_form_components/system_list.rs`, `software_title_list.rs`, `file_set_list.rs`
These three components hold `Arc<RepositoryManager>` only to pass it down to their respective
selector child components. They call no repository methods themselves. They are fixed automatically
when their selector children are migrated.

## Requirements

### AppServices Struct

A new `AppServices` struct is created in `service/src/app_services.rs` and re-exported from
`service/src/lib.rs`. It grows across two phases:

**After Phase 1** (domain services only):
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
    pub fn new(repository_manager: Arc<RepositoryManager>) -> Self { ... }
}
```

**After Phase 4** (pipeline services added):
```rust
pub struct AppServices {
    // ... all Phase 1 fields ...
    pub settings: Arc<SettingsService>,
    pub file_import: Arc<FileImportService>,
    pub mass_import: Arc<MassImportService>,
    pub cloud_sync: Arc<CloudStorageSyncService>,
    pub export: Arc<ExportService>,
}

impl AppServices {
    // Arc<Settings> added because FileImportService, MassImportService,
    // and CloudStorageSyncService require it at construction time.
    pub fn new(repository_manager: Arc<RepositoryManager>, settings: Arc<Settings>) -> Self { ... }
}
```

`AppServices` is constructed once in `app.rs` inside `post_process_initialize` and wrapped in
`Arc<AppServices>` for sharing across all components.

### Domain Services to Create or Extend

#### `SystemService` (new — `service/src/system_service.rs`)

```rust
pub async fn add_system(&self, name: &str) -> Result<i64, Error>
pub async fn update_system(&self, id: i64, name: &str) -> Result<i64, Error>
pub async fn delete_system(&self, id: i64) -> Result<(), Error>
```

#### `ReleaseService` (new — `service/src/release_service.rs`)

```rust
pub async fn add_release(
    &self, name: &str, software_title_ids: &[i64],
    file_set_ids: &[i64], system_ids: &[i64],
) -> Result<i64, Error>
pub async fn update_release(
    &self, id: i64, name: &str, software_title_ids: &[i64],
    file_set_ids: &[i64], system_ids: &[i64],
) -> Result<i64, Error>
pub async fn delete_release(&self, id: i64) -> Result<i64, Error>
```

#### `ReleaseItemService` (new — `service/src/release_item_service.rs`)

```rust
pub async fn create_item(
    &self, release_id: i64, item_type: ItemType, notes: Option<String>,
) -> Result<i64, Error>
pub async fn get_item(&self, item_id: i64) -> Result<ReleaseItem, Error>
pub async fn update_item(
    &self, item_id: i64, item_type: ItemType, notes: Option<String>,
) -> Result<i64, Error>
pub async fn delete_item(&self, item_id: i64) -> Result<(), Error>
```

#### `SoftwareTitleService` (already exists — extend `service/src/software_title_service.rs`)

The existing `SoftwareTitleService` only implements `merge`. Add:

```rust
pub async fn add_software_title(&self, name: &str) -> Result<i64, Error>
pub async fn update_software_title(&self, id: i64, name: &str) -> Result<i64, Error>
pub async fn delete_software_title(&self, id: i64) -> Result<i64, Error>
```

#### `EmulatorService` (new — `service/src/emulator_service.rs`)

```rust
pub async fn add_emulator(
    &self, name: &str, executable: &str, extract_files: bool,
    arguments: &[ArgumentType], system_id: i64,
) -> Result<i64, Error>
pub async fn update_emulator(
    &self, id: i64, name: &str, executable: &str, extract_files: bool,
    arguments: &[ArgumentType], system_id: i64,
) -> Result<i64, Error>
pub async fn delete_emulator(&self, id: i64) -> Result<i64, Error>
```

`EmulatorService` is responsible for `serde_json::to_string` serialization of `arguments` before
passing to the repository.

#### `DocumentViewerService` (new — `service/src/document_viewer_service.rs`)

```rust
pub async fn add_document_viewer(
    &self, name: &str, executable: &str, arguments: &[ArgumentType],
    document_type: &DocumentType, cleanup_temp_files: bool,
) -> Result<i64, Error>
pub async fn update_document_viewer(
    &self, id: i64, name: &str, executable: &str, arguments: &[ArgumentType],
    document_type: &DocumentType, cleanup_temp_files: bool,
) -> Result<i64, Error>
pub async fn delete_document_viewer(&self, id: i64) -> Result<i64, Error>
```

`DocumentViewerService` handles `serde_json::to_string` serialization of `arguments`.

### GUI Component Changes

Each `*Init` struct that currently contains `Arc<RepositoryManager>` is updated to contain
`Arc<AppServices>` instead. Model structs replace their `repository_manager` field with
`app_services`. All `oneshot_command` closures that currently call repository methods are updated
to call the corresponding service method through `app_services`.

For the `ExternalExecutableRunnerService` that is currently constructed in
`document_file_set_viewer.rs` and `emulator_runner.rs` using `Arc<RepositoryManager>` and
`Arc<Settings>`, the preferred approach is to pass `Arc<ExternalExecutableRunnerService>` in from
the parent's `Init` struct so that the parent component can construct it.

### Error Type Consistency

All new service methods return `Result<T, service::error::Error>`. GUI `CommandMsg` variants that
currently unwrap `database::database_error::Error` are updated to unwrap `service::error::Error`.

## Phased Approach

### Phase 1 — Create Domain Services and AppServices

1. Create `SystemService` in `service/src/system_service.rs`
2. Create `ReleaseService` in `service/src/release_service.rs`
3. Create `ReleaseItemService` in `service/src/release_item_service.rs`
4. Add mutation methods to the existing `SoftwareTitleService`
5. Create `EmulatorService` in `service/src/emulator_service.rs`
6. Create `DocumentViewerService` in `service/src/document_viewer_service.rs`
7. Create `AppServices` in `service/src/app_services.rs`
8. Re-export all new types from `service/src/lib.rs`

No GUI files are touched in Phase 1. Each new service must have unit tests using
`setup_test_repository_manager` before the task is considered done.

### Phase 2 — Migrate GUI Components

Components are migrated in leaf-first order. A component that passes its `Init` field to a child
must be migrated only after the child is done.

Migration order:
1. `system_form.rs`
2. `system_selector.rs` (and consequently `system_list.rs`)
3. `software_title_form.rs`
4. `software_title_selector.rs` (and consequently `software_title_list.rs`)
5. `release_form_components/item_form.rs`
6. `release_form_components/item_list.rs`
7. `document_viewer_form.rs`
8. `document_file_set_viewer.rs`
9. `emulator_form.rs`
10. `emulator_runner.rs`
11. `release_form.rs`
12. `releases.rs`
13. `app.rs` — construct `Arc<AppServices>` in `post_process_initialize`

### Phase 3 — Intermediate Cleanup

1. Verify no `use database::repository_manager::RepositoryManager` imports remain in the 13 Phase 2 migrated files
2. Run `cargo clippy --all-targets` and resolve all warnings introduced so far
3. Note: `database` dependency stays in `relm4-ui/Cargo.toml` — Phase 4 files still need it
4. Run `cargo test --workspace` to confirm no regressions

### Phase 4 — Pipeline Services and Remaining GUI Files

The five files deferred from Phase 2 all share the same pattern: they construct a pipeline
service in `init` using `Arc<RepositoryManager>`. The fix is to extend `AppServices` with the
pipeline services and pass `Arc<AppServices>` instead.

**Services to add to `AppServices`:**
- `settings: Arc<SettingsService>` — already exists, constructed with `Arc<RepositoryManager>`
- `file_import: Arc<FileImportService>` — already exists, needs `Arc<RepositoryManager>` + `Arc<Settings>`
- `mass_import: Arc<MassImportService>` — already exists, needs `Arc<RepositoryManager>` + `Arc<Settings>`
- `cloud_sync: Arc<CloudStorageSyncService>` — already exists, needs `Arc<RepositoryManager>` + `Arc<Settings>`
- `export: Arc<ExportService>` — already exists, needs `Arc<RepositoryManager>`

**`AppServices::new` signature update:**

```rust
pub fn new(repository_manager: Arc<RepositoryManager>, settings: Arc<Settings>) -> Self
```

The `Arc<Settings>` parameter is needed because three pipeline services require it at
construction time. `Settings` is always available by the time `AppServices` is constructed in
`post_process_initialize`.

**GUI files to migrate:**
- `settings_form.rs` — constructs `SettingsService` in `init`; use `app_services.settings`
- `software_titles_list.rs` — passes `repository_manager` to merge dialog child; pass `app_services`
- `release.rs` — passes `repository_manager` to child Init structs; pass `app_services`
- `file_set_form.rs` — constructs `FileImportService` in `init`; use `app_services.file_import`
- `import_form.rs` — constructs `MassImportService` in `init`, also stores `repository_manager` in model for re-use; replace both with `app_services`

### Phase 5 — Final Cleanup

1. Verify zero `use database::repository_manager::RepositoryManager` imports across all GUI files
2. Attempt to remove `database` from `relm4-ui/Cargo.toml` — if no remaining imports exist, the crate-level dependency is gone and the layer boundary is enforced at the compiler level
3. Run `cargo clippy --all-targets` and `cargo test --workspace`
4. Full smoke test

## Success Criteria

**After Phase 3:**
- Zero `use database::repository_manager::RepositoryManager` in the 13 Phase 2 component files
- All new domain service methods have passing automated tests
- `cargo test --workspace` passes

**After Phase 5 (complete):**
- Zero `use database::repository_manager::RepositoryManager` across all GUI files
- `database` removed from `relm4-ui/Cargo.toml` (or a documented TODO if any non-RepositoryManager database import legitimately remains)
- `cargo clippy --all-targets` produces no warnings
- Full smoke test passes

## Out of Scope

- Functional changes of any kind: this migration is purely structural.
- `ExternalExecutableRunnerService` dependency cleanup: out of scope beyond making it injectable
  from the parent `Init` struct.
- Any GUI files not listed in the Current State section.
