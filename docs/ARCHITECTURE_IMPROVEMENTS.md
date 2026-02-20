# Architecture Improvements

This document records architectural findings from a full review of the Emulation File Manager
codebase. Each item includes severity, affected files with line references, and a recommended fix.

Severity scale:
- **Critical** — violates a hard architectural rule; fix before next feature work
- **High** — introduces real risk (panics, data loss, silent failures)
- **Medium** — code quality / maintainability concern
- **Low** — style, minor idiom issue

---

## 1. Layer Violations (Critical)

### 1.1 GUI code holds `Arc<RepositoryManager>` and calls repositories directly

**Rule violated:** GUI must never call database repositories directly. All data access must go
through the service layer. (See CLAUDE.md: "SQLx queries never appear in GUI code", "GUI never
calls database repositories directly".)

**Affected call sites in `relm4-ui/src/`:**

| File | Line(s) | Repository method |
|---|---|---|
| `system_selector.rs` | 242 | `get_system_repository().delete_system(id)` |
| `release_form.rs` | 304, 317 | `get_release_repository().*` |
| `release_form_components/item_form.rs` | 191, 201, 228 | `get_release_item_repository().*` |
| `release_form_components/item_list.rs` | 175 | `get_release_item_repository().*` |
| `document_file_set_viewer.rs` | 331 | `get_document_viewer_repository().*` |
| `software_title_form.rs` | 127, 134 | `get_software_title_repository().*` |
| `document_viewer_form.rs` | 198, 213 | `get_document_viewer_repository().*` |
| `software_title_selector.rs` | 247 | `get_software_title_repository().*` |
| `system_form.rs` | 120, 127 | `get_system_repository().*` |
| `emulator_form.rs` | 223, 238 | `get_emulator_repository().*` |
| `emulator_runner.rs` | 340 | `get_emulator_repository().*` |
| `releases.rs` | 260 | `get_release_repository().*` |

**Recommended fix:**

Introduce domain-scoped services in the `service` crate and bundle them into a single
`Arc<AppServices>` that is passed through the component tree in place of `Arc<RepositoryManager>`.

```rust
// service/src/app_services.rs
pub struct AppServices {
    pub view_model:     Arc<ViewModelService>,
    pub system:         Arc<SystemService>,
    pub release:        Arc<ReleaseService>,
    pub emulator:       Arc<EmulatorService>,
    pub software_title: Arc<SoftwareTitleService>,
    // extend as new domains are added
}
```

Each domain service (`SystemService`, `ReleaseService`, etc.) owns the CRUD operations for its
aggregate. `ViewModelService` remains query-only, assembling cross-entity read models.

GUI components continue to receive one handle through their `Init` struct — same ergonomics as
today — but call `services.system.delete(id)` instead of
`repo_manager.get_system_repository().delete_system(id)`. The layer boundary is enforced because
`AppServices` lives in the `service` crate and the GUI never sees `RepositoryManager`.

**Trade-off to be aware of:** `Arc<AppServices>` is a
[Service Locator](https://en.wikipedia.org/wiki/Service_locator_pattern). Components can
technically call any service even if they have no business doing so, which is a weaker contract
than passing each component only the exact service it needs. For a desktop app with relm4's
component tree depth, this is an acceptable pragmatic compromise — the layer violation being
fixed is far more harmful than the theoretical over-exposure.

**Migration steps:**
1. Create `AppServices` in the `service` crate
2. Add one domain service per aggregate, implementing the methods that are currently bypassed
3. Replace `Arc<RepositoryManager>` with `Arc<AppServices>` in all GUI `Init` structs
4. Update each call site to use the appropriate service method

### 1.2 GUI imports raw database model type

**File:** `release_form_components/item_form.rs:4`

```rust
use database::models::ReleaseItem;
```

`ReleaseItem` is a raw SQLx model from the `database` crate. GUI code must only consume view
model types defined in the `service` crate.

**Recommended fix:** Define a `ReleaseItemViewModel` (or reuse an existing one if available) in
`service`, map from `ReleaseItem` inside the service layer, and update `item_form.rs` to use it.

---

## 2. Dead Code

### 2.1 Domain layer — unused normalizer functions

**Files:**
- `domain/src/title_normalizer/rules/extension.rs` — `strip_extension()` is never called
- `domain/src/title_normalizer/rules/punctuation.rs` — `normalize_punctuation()` is never called
- `domain/src/title_normalizer/normalizer.rs` — `SoftwareTitle` struct and `get_software_title()` are never used

**Recommended fix:** Delete all three items. If the title normalizer is an intended feature,
track its completion as a separate issue rather than leaving dead stubs in production code.

### 2.2 Database layer — orphaned `FranchiseRepository`

**File:** `database/src/repository/franchise_repository.rs`

The repository implements all four CRUD operations and has tests, but:
- It is not registered in `RepositoryManager`
- No service methods reference it
- No UI surfaces it

This is a feature stub that was never wired up.

**Recommended fix:** Either complete the franchise feature (register in `RepositoryManager`, add
service methods, add UI) or remove the file entirely. The tests should be deleted alongside the
implementation if removed.

### 2.3 Database layer — unused `get_database_url()`

**File:** `database/src/database_path.rs`

`get_database_url()` is defined but never called from any crate.

**Recommended fix:** Delete the function. If the intent is to support configurable database
paths, track that as a separate feature.

### 2.4 Service layer — `MassImportDependencies` struct never constructed

**File:** `service/src/mass_import/context.rs:115`

`MassImportDependencies` with `repository_manager` and `settings` fields was apparently written
during a refactoring of `MassImportDeps` but was never completed.

**Recommended fix:** Either finish the refactoring (replace the two separate fields in
`MassImportDeps` with `MassImportDependencies`) or delete the struct.

### 2.5 Service layer — unused imports

**Files:**
- `service/src/mass_import/steps.rs:3` — unused import `Sha1Checksum`
- `service/src/file_import/common_steps/collect_file_info.rs` — unused imports `FileImportOps`,
  `mock::MockFileImportOps`

**Recommended fix:** Delete the unused imports. `cargo clippy` will flag these.

### 2.6 Cloud storage — unused `upload_file()` and unused variable

**File:** `cloud_storage/src/lib.rs`

- Line 63: `upload_file()` is never called from any other crate
- Line 38: variable `path` is assigned but not used (should be `_path` or removed)

**Recommended fix:** If `upload_file` is part of a planned sync feature, mark it
`#[allow(dead_code)]` with a comment; otherwise delete it. Fix the unused variable.

### 2.7 GUI — unused imports in `model.rs`

**File:** `relm4-ui/src/model.rs`

Multiple unused imports: `sync::Arc`, `RepositoryManager`, `FileImportOps`, `Settings`,
`FileSystemOps`.

**Recommended fix:** Delete the unused imports. They are likely leftovers from previous
refactoring passes.

---

## 3. Error Handling

### 3.1 `service::Error` does not use `thiserror`

**File:** `service/src/error.rs`

The `Error` enum manually implements `Display` and all `From` impls call `.to_string()` on the
source error, discarding structured error context. This makes downstream error introspection
(logging, matching) harder than it needs to be.

**Recommended fix:** Add `thiserror` to `service/Cargo.toml` and derive `#[derive(thiserror::Error)]`
on the enum. Replace manual `Display` impls with `#[error("...")]` attributes. Replace
`From` impls that call `.to_string()` with `#[from]` where the source error type is specific
enough to be preserved.

### 3.2 `SoftwareTitleServiceError` is inconsistent with the rest of the service layer

**File:** `service/src/software_title_service.rs:6-8`

`SoftwareTitleServiceError` is a one-off error enum that:
- Does not derive `thiserror::Error`
- Wraps all errors as bare strings via `format!("{:?}", e)`
- Is not unified with the shared `service::Error` type used by every other module

This inconsistency makes the API harder to consume and silently loses error context.

**Recommended fix:** Replace `SoftwareTitleServiceError` with `service::Error` (after fixing
3.1). Update all call sites.

### 3.3 `.unwrap()` in non-test production code

**Affected sites:**

- `service/src/file_import/common_steps/import.rs:195` — `.unwrap()` on a checksum map lookup.
  This is inside a pipeline `execute()` step that is not guarded by a corresponding
  `should_execute` check, so the pipeline exception documented in CLAUDE.md does not apply.
  A panic here would terminate the import without a useful error message.

- `ui-components/src/confirm_dialog.rs:80,84` — `sender.output(...).unwrap()` panics if the
  parent component has been dropped. relm4 senders are expected to be handled gracefully.

**Recommended fix:**
- `import.rs:195`: use `.ok_or(Error::...)` and propagate with `?`
- `confirm_dialog.rs`: replace `.unwrap()` with `.unwrap_or_else(|e| tracing::error!("confirm dialog output error: {e:?}"))`

---

## 4. GUI Anti-Patterns

### 4.1 `root.close()` instead of `root.hide()` for reusable dialogs

Calling `root.close()` on a GTK4 window destroys its widget tree. Reusable dialogs that are
`present()`-ed multiple times must use `root.hide()` to preserve the tree between invocations.
Using `close()` requires re-initialising the component on next use, which is error-prone and
can cause hard-to-reproduce state bugs.

**Affected files:**

| File | Line(s) |
|---|---|
| `system_selector.rs` | 211 |
| `release_form.rs` | 466 |
| `release_form_components/item_form.rs` | 280 |
| `file_set_form.rs` | 768 |
| `document_file_set_viewer.rs` | 378 |
| `software_title_form.rs` | 190 |
| `document_viewer_form.rs` | 296, 319 |
| `file_set_selector.rs` | 273 |
| `system_form.rs` | 181 |
| `emulator_form.rs` | 315, 327 |

**Recommended fix:** Replace every `root.close()` call in reusable dialog components with
`root.hide()`.

### 4.2 Debug `println!` / `dbg!` / `eprintln!` in production code

Production code must use `tracing::*` macros so output is controlled by the log level filter.
Raw `println!` / `eprintln!` / `dbg!` bypass the logging infrastructure and cannot be silenced
in release builds.

**Affected files (GUI layer):**

| File | Line(s) | Macro |
|---|---|---|
| `release_form_components/item_form.rs` | 169, 180 | `println!` |
| `release_form_components/item_form.rs` | 292 | `dbg!` — prints internal state |
| `document_file_set_viewer.rs` | 171 | `println!` |
| `file_set_details_view.rs` | 196 | `println!` |
| `software_titles_list.rs` | 170, 178, 211 | `println!` |
| `emulator_form.rs` | 306, 313, 318, 325 | `println!` / `eprintln!` |
| `app.rs` | 391, 488, 832, 836 | `println!` / `eprintln!` |
| `releases.rs` | 323 | `println!` |
| `argument_list.rs` | 141 | `eprintln!` |

**Affected files (service layer):**

| File | Line(s) | Note |
|---|---|---|
| `service/src/software_title_service.rs` | 81, 97 | `println!` inside merge business logic |

**Not a priority** (acceptable): `relm4-ui/src/logging.rs:60,61` — startup console output
before the tracing subscriber is initialised; `println!` is appropriate here.

**Recommended fix:** Replace each instance with the appropriate `tracing` macro:
- Diagnostic/flow info → `tracing::debug!` or `tracing::info!`
- Errors → `tracing::error!`
- Remove `dbg!` entirely (or demote to `tracing::debug!` with a named field)

---

## 5. Rust Idioms

### 5.1 Primitive obsession — raw `i64` entity IDs

All entity identifiers (`system_id`, `release_id`, `software_title_id`, `file_set_id`, etc.) are
bare `i64`. There is no compile-time distinction between, for example, a `system_id` and a
`release_id`. Swapping arguments of the same type is a silent bug.

**Recommended fix (medium-term):** Define newtype wrappers in `core_types`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SystemId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReleaseId(pub i64);
// etc.
```

Implement `sqlx::Type`, `From<i64>`, and `Display` as needed. This is a widespread change that
should be done incrementally, starting with the most confusion-prone IDs.

### 5.2 Inconsistent error types in the service layer

Covered in 3.2. The existence of `SoftwareTitleServiceError` alongside the shared `service::Error`
means consumers must handle two unrelated error types from the same crate, and internal code
cannot use `?` uniformly across modules.

---

## 6. Incomplete / Orphaned Features

### 6.1 Franchise feature — repository stub with no wiring

**File:** `database/src/repository/franchise_repository.rs`

A complete CRUD repository with passing tests exists, but `FranchiseRepository` is not
registered in `RepositoryManager`, has no service layer methods, and has no UI entry point.
It is effectively unreachable production code.

**Decision required:**
- **Complete the feature:** Register in `RepositoryManager`, add `service` methods, add UI
  components.
- **Remove the stub:** Delete `franchise_repository.rs` and its tests until the feature is
  actually prioritised.

Leaving the stub in place gives a false impression that franchise support is implemented.

### 6.2 `MassImportDependencies` — orphaned refactoring artifact

**File:** `service/src/mass_import/context.rs:115`

Covered in 2.4. The struct has the shape of an in-progress extraction of `MassImportDeps`
fields into a grouped dependency object, but was never finished.

**Decision required:**
- **Complete the refactoring:** Replace the two raw fields in `MassImportDeps` with
  `MassImportDependencies`.
- **Remove the struct:** If the refactoring is not being pursued, delete it to reduce noise.

---

## Suggested Priority Order

| Priority | Item | Effort |
|---|---|---|
| 1 | §1.1 — Layer violations (GUI → repository direct calls) | High |
| 2 | §4.2 — Replace `println!`/`dbg!` with `tracing::*` | Low |
| 3 | §4.1 — Replace `root.close()` with `root.hide()` | Low |
| 4 | §3.3 — Eliminate `.unwrap()` in production paths | Low |
| 5 | §3.1 + §3.2 — Adopt `thiserror`, unify error types | Medium |
| 6 | §2.x — Remove dead code | Low (each) |
| 7 | §6.x — Resolve orphaned feature stubs | Medium |
| 8 | §5.1 — Newtype ID wrappers | High (widespread) |
