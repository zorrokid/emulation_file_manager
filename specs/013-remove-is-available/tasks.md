# Spec 013 Tasks: Remove `is_available` from `file_info`

## Completed Tasks

- [x] T1 — DB migration: drop `is_available` column
  **File:** `database/migrations/20260407104000_remove_is_available_from_file_info.sql`

- [x] T2 — Remove `FileSyncStatus::UploadSkipped` from `core_types`
  **File:** `core_types/src/lib.rs`

- [x] T3 — Remove `is_available` field from `ImportedFile`; add `is_available()` method
  **File:** `core_types/src/lib.rs`

- [x] T4 — Remove `is_available` field from `FileInfo` and `FileSetFileInfo`; add methods
  **File:** `database/src/models.rs`

- [x] T5 — Update `FileInfoRepository`: queries, INSERT, `FromRow`, rename method, add `_with_tx` variant
  **File:** `database/src/repository/file_info_repository.rs`

- [x] T6 — Update `FileSetRepository`: queries, mapping, and inline `file_info` SQL
  **File:** `database/src/repository/file_set_repository.rs`

- [x] T7 — Update service layer: remove `is_available` struct fields, rename method calls
  **Files:** `service/src/cloud_sync/steps.rs`, `service/src/file_import/**`, `service/src/file_set/**`, `service/src/file_type_migration/steps.rs`, `service/src/file_set_download/steps.rs`, and others

- [x] T8 — Update `file_import` crate
  **File:** `file_import/src/lib.rs`

- [x] T9 — Regenerate `.sqlx/` metadata

- [x] T10 — Update ER diagrams

- [x] T11 — Full verification

---

## Phase 6 — Review Fixes (Round 1)

### Major

- [x] T12 — Remove `set_archive_file_name_with_tx` dead API or use it in `FileSetRepository`
  **File:** `database/src/repository/file_info_repository.rs:249–263`  
  `set_archive_file_name_with_tx` was added for `FileSetRepository` to call inside a transaction,
  but `FileSetRepository.add_file_set_with_tx` uses identical inline SQL instead. The method is
  never called — it is a dead `pub` API that also creates a DRY violation (the same
  `UPDATE file_info SET archive_file_name = ? WHERE id = ?` SQL exists in two places).  
  Option A (simpler): remove `set_archive_file_name_with_tx` entirely and add a comment in
  `file_set_repository.rs` explaining why `file_info` SQL is inlined there.  
  Option B: use `FileInfoRepository::new(self.pool.clone()).set_archive_file_name_with_tx(...)` in
  `add_file_set_with_tx`, removing the inline SQL.

### Minor

- [x] T13 — Update stale `is_available` doc comments in `file_import/model.rs`
  **File:** `service/src/file_import/model.rs:32, 80`  
  Two `///` doc comments say "stored as `file_info` records with `is_available = false`". The
  field no longer exists; replace both with `archive_file_name = NULL (unavailable)`.

- [x] T14 — Update stale `is_available` references in `update_file_set/steps.rs` test comments
  **File:** `service/src/file_import/update_file_set/steps.rs:985–986, 997, 1056`  
  Three inline comments reference the removed `is_available` field
  (`is_available=1 with archive_file_name=NULL`, `add_file_set with is_available=false`,
  `remain is_available=false with archive_file_name=None`). Rewrite to use
  `archive_file_name = NULL` terminology.

- [x] T15 — Update stale `is_available` reference in `add_file_set/steps.rs` test comment
- [x] T16 — Rename `setup_invariant_violating_file_set` and update its doc comment
- [x] T17 — Update stale `is_available=false` comments in `mass_import/with_dat/steps.rs`

---

## Manual Verification Checklist

- [ ] Import a file set — verify in DB: `SELECT archive_file_name FROM file_info` shows NULL for missing files, non-null for available files (no `is_available` column present)
- [ ] Trigger cloud sync — only files with non-null `archive_file_name` are uploaded
- [ ] `cargo test -p database` passes
- [ ] `cargo test -p service` passes
- [ ] `cargo test -p core_types` passes
- [ ] `cargo clippy --all-targets` clean
