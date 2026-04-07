# Spec 013 Tasks: Remove `is_available` from `file_info`

## Status

Planning.

## Tasks

- [ ] T1 — DB migration: drop `is_available` column
  Recreate `file_info` table without `is_available`. Copy all other columns.
  Run `sqlx migrate run` to verify it applies cleanly.
  **File:** `database/migrations/<new timestamp>_remove_is_available_from_file_info.sql`

- [ ] T2 — Remove `FileSyncStatus::UploadSkipped` from `core_types`
  Remove the variant, its `to_db_int` arm (value 8), and its `from_db_int` arm.
  Update the doc comment on `FileSyncStatus` to remove the `UploadSkipped` reference.
  **File:** `core_types/src/lib.rs`

- [ ] T3 — Remove `is_available` field from `ImportedFile`; add `is_available()` method
  Replace `pub is_available: bool` with `pub fn is_available(&self) -> bool { self.archive_file_name.is_some() }`.
  **File:** `core_types/src/lib.rs`

- [ ] T4 — Remove `is_available` field from `FileInfo` and `FileSetFileInfo`; add methods
  Remove `pub is_available: bool` from both structs.
  Add `pub fn is_available(&self) -> bool { self.archive_file_name.is_some() }` to both.
  Update `From<&FileSetFileInfo> for FileInfo` — remove the `is_available` field copy.
  **File:** `database/src/models.rs`

- [ ] T5 — Update `FileInfoRepository`: queries, INSERT, `FromRow`, rename method, add `_with_tx` variant
  - `FromRow` impl: remove `is_available: row.try_get("is_available")?` — `is_available()` is now computed.
  - `add_file_info` INSERT: remove `is_available` from column list and values.
  - `get_files_pending_upload` / `count_files_pending_upload`: replace `AND is_available = 1` with `AND archive_file_name IS NOT NULL`.
  - All SELECT column lists: remove `is_available`.
  - Rename `update_is_available(id, archive_file_name)` → `set_archive_file_name(id, archive_file_name)`; update SQL to only set `archive_file_name` (no longer sets `is_available`).
  - Add `set_archive_file_name_with_tx(&self, tx: &mut Transaction<'_, Sqlite>, id: i64, archive_file_name: Option<&str>)` — same SQL as `set_archive_file_name` but accepts a transaction executor. This is required by `FileSetRepository` which calls this operation inside a transaction.
  - Update test helper `insert_file_info` to remove `is_available` parameter and the explicit `is_available` column from raw INSERT.
  - Update test assertions that checked `file_info.is_available` to call `file_info.is_available()`.
  **File:** `database/src/repository/file_info_repository.rs`

- [ ] T6 — Update `FileSetRepository`: queries, mapping, and inline `file_info` SQL
  - Remove `is_available` from all SELECT column lists and JOIN queries.
  - Remove `is_available: row.try_get("is_available")?` from `FileSetFileInfo` mapping.
  - **`Some(id)` match arm** (existing file_info found by SHA1): replace
    `sqlx::query!("UPDATE file_info SET is_available = 1 WHERE id = ?", id)` with
    `file_info_repository.set_archive_file_name_with_tx(&mut *tx, id, file.archive_file_name.as_deref()).await?`.
    This ensures a previously-missing record (archive_file_name IS NULL) is correctly updated
    when the file is now available, keeping it visible to the `archive_file_name IS NOT NULL` query filter.
  - **`None` arm inline INSERT**: remove `is_available` from the column list and values in the
    raw `INSERT INTO file_info (...)` statement.
  - Update any `.filter(|f| !f.is_available)` → `.filter(|f| f.archive_file_name.is_none())`.
  - Update any `if file.is_available { ... }` → `if file.is_available() { ... }`.
  - Remove `is_available` from all test fixture struct initialisers.
  **File:** `database/src/repository/file_set_repository.rs`

- [ ] T7 — Update service layer: remove `is_available` struct fields, rename method calls
  - Before starting: grep for all `FileSyncStatus::UploadSkipped` and bare `UploadSkipped`
    references across all crates to confirm every write and match site is identified.
  - Remove all `is_available: true` / `is_available: false` from struct initialisers across the service crate (file_import, file_set, cloud_sync, file_type_migration, file_set_download, etc.).
  - Replace all `file.is_available` field accesses with `file.is_available()` method calls.
  - Replace all `update_is_available(...)` call sites with `set_archive_file_name(...)`.
  - Remove the `UploadSkipped` guard in `UploadPendingFilesStep` (cloud sync) — it was added
    solely to handle the invariant violation that is now structurally impossible.
  - Remove all remaining `FileSyncStatus::UploadSkipped` references in service code.
  - Remove/update tests that forced invariant violations (`update_is_available(id, None)` /
    `set_archive_file_name(id, None)` used in test helpers to create `is_available=true,
    archive_file_name=NULL`) — these violations are now impossible; remove the tests entirely
    or replace with simpler equivalents that test remaining behaviour.
  **Files:** `service/src/cloud_sync/steps.rs`, `service/src/file_import/**`, `service/src/file_set/**`, `service/src/file_type_migration/steps.rs`, `service/src/file_set_download/steps.rs`, and others

- [ ] T8 — Update `file_import` crate
  Remove `is_available: true` from all `ImportedFile` struct initialisers.
  **File:** `file_import/src/lib.rs`

- [ ] T9 — Regenerate `.sqlx/` metadata
  Run: `cargo sqlx prepare --workspace -- --all-targets`
  Commit the updated `.sqlx/` directory.

- [ ] T10 — Update ER diagrams
  Run: `tbls doc --force`
  Commit the updated `docs/schema/` files.

- [ ] T11 — Full verification
  Run `cargo test --workspace` — all tests must pass.
  Run `cargo clippy --all-targets` — no new warnings.

---

## Manual Verification Checklist

- [ ] Import a file set — verify in DB: `SELECT archive_file_name FROM file_info` shows NULL for missing files, non-null for available files (no `is_available` column present)
- [ ] Trigger cloud sync — only files with non-null `archive_file_name` are uploaded
- [ ] `cargo test -p database` passes
- [ ] `cargo test -p service` passes
- [ ] `cargo test -p core_types` passes
- [ ] `cargo clippy --all-targets` clean
