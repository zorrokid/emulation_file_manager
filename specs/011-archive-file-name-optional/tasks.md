# Spec 011 Tasks: Make `archive_file_name` Optional

## Completed Tasks

- [x] T1 — DB migration: make `archive_file_name` nullable
- [x] T2 — Struct field changes (`Option<String>`) + `generate_cloud_key` return type
- [x] T3 — Repository method signatures (`add_file_info`, `update_is_available`)
- [x] T4 — Service callsites: guard `generate_cloud_key()` returns
- [x] T5 — Service callsites: guard `get_file_path` with `let Some(name) = archive_file_name`
- [x] T6 — Replace empty-string sentinels with `None`
- [x] T7 — Update test construction sites (~61 string literals → `Some(...)`)
- [x] T8 — `cargo sqlx prepare --workspace` + `tbls doc`
- [x] T9 — All tests pass, `cargo clippy --all-targets` clean
- [x] T10 — Add `tracing::warn!` to all `continue` cases missing a warning
- [x] T11 — Filter `is_available = false` files from `MoveLocalFilesStep` loop

## Bug Fix Tasks (from code review)

- [x] T12 — Fix Finding 1 (initial): add `warn!` to `UploadPendingFilesStep` before the
  silent `continue` after `UploadInProgress` is written; also write `UploadFailed` entry
  *(superseded by T25 — writing `UploadFailed` causes an infinite loop)*

- [x] T25 — Fix Finding 1 (correct): add `FileSyncStatus::UploadSkipped` variant (value `8`)
  to `core_types`; update `UploadPendingFilesStep` to write `UploadSkipped` (not `UploadFailed`)
  when `archive_file_name` is `None`; revert the `UploadFailed` write from T12

- [x] T13 — Fix Finding 2: guard `update_is_available` in `UpdateFileInfoToDatabaseStep`
  with `if !imported_file.is_available { continue; }` to prevent writing
  `is_available=1, archive_file_name=NULL`

- [x] T14 — Fix Finding 3 (migration): add migration to change `is_available` column
  DEFAULT from 1 to 0 via recreate-table pattern

- [x] T15 — Fix Finding 3 (repository): update `add_file_info` to explicitly set
  `is_available = archive_file_name IS NOT NULL` in the INSERT instead of relying on
  the column default; regenerate `.sqlx/` metadata afterwards

## Test Coverage Tasks (from coverage analysis)

The coverage analysis identified 9 warn/skip paths that were added by this branch but
have no test coverage. All of them guard against the invariant violation
`is_available = true` with `archive_file_name = None`.

### Critical

- [x] T16 — `cloud_sync/steps.rs` — `UploadPendingFilesStep`: add test
  `test_upload_pending_files_step_handles_missing_archive_file_name` — verifies that when
  a pending file has `archive_file_name = None`, the step writes `UploadSkipped` to the sync
  log (not `UploadInProgress` or `UploadFailed`), does not call the cloud upload method, and
  does not loop infinitely (depends on T25)

- [x] T17 — `cloud_sync/steps.rs` — `PrepareFilesForUploadStep`: add test
  `test_prepare_files_for_upload_step_skips_when_archive_file_name_is_none` — verifies that
  a file with `is_available = true, archive_file_name = None` is skipped (no sync log entry
  created, step does not abort)

### High Priority

- [x] T18 — `file_type_migration/steps.rs` — `MoveLocalFilesStep`: add test
  `test_move_local_files_step_skips_unavailable_files` — verifies that files with
  `is_available = false` are excluded from the move; only available files appear in
  `moved_local_file_ids`

- [x] T19 — `file_type_migration/steps.rs` — `MoveLocalFilesStep`: add test
  `test_move_local_files_step_warns_when_archive_file_name_is_none` — verifies that a file
  that passes the `is_available` filter but has `archive_file_name = None` is recorded in
  `non_existing_local_file_ids` and not in `moved_local_file_ids`

- [x] T20 — `file_type_migration/steps.rs` — `MoveCloudFilesStep`: add test
  `test_move_cloud_files_step_skips_when_archive_file_name_is_none` — verifies that a file
  with `archive_file_name = None` is silently skipped (not added to `moved_cloud_file_ids`,
  cloud storage move not called)

### Medium Priority

- [x] T21 — `file_set_download/steps.rs` — `PrepareFileForDownloadStep`: add test
  `test_prepare_file_for_download_step_skips_missing_archive_file_name` — verifies that a
  file with `archive_file_name = None` is excluded from `files_to_download` and the step
  does not abort

- [x] T22 — `file_set_download/steps.rs` — `DownloadFilesStep`: add test
  `test_download_files_step_skips_missing_archive_file_name` — verifies that a file that
  reaches the download step with `archive_file_name = None` is skipped and the cloud storage
  download is not called for that file

- [x] T23 — `file_import/common_steps/file_deletion_steps.rs` — `DeleteLocalFilesStep`: add
  test `test_delete_local_files_step_skips_when_archive_file_name_is_none` — verifies that a
  file marked for deletion with `archive_file_name = None` is skipped and the file system
  delete is not called for it

- [x] T24 — `file_import/add_file_set/steps.rs` — `CreateFileSetToDatabaseStep` error
  cleanup path: add test
  `test_create_file_set_database_step_cleanup_skips_when_archive_file_name_missing` —
  verifies that when a DB insertion fails and the cleanup loop runs, a file with
  `is_available = true, archive_file_name = None` does not cause a file system delete call
  and a warning is emitted

---

## Phase 6 — Review Fixes (Round 2)
<!-- Findings documented in review-2.md -->

- [x] T26 — Fix redundant double-guard in `DownloadFilesStep`
  **File:** `service/src/file_set_download/steps.rs`
  After `generate_cloud_key()` returns `Some`, `archive_file_name` is guaranteed `Some` (they
  represent the same value). The second guard `let Some(archive_file_name) = &file_info.archive_file_name`
  can never trigger after the first. Replace both with a single pattern that extracts both values
  simultaneously, or unwrap `archive_file_name` with `expect` after the cloud-key guard.

- [x] T27 — Add `tracing::warn!` to silent skips in `ExportFilesStep` and `export_service`
  **Files:** `service/src/file_set_download/steps.rs` (`ExportFilesStep`),
  `service/src/export_service.rs`
  Both use `filter_map` to silently skip files with `archive_file_name=None`. If a user exports
  a file set with missing files they get a partial result with no indication. Add a `tracing::warn!`
  per skipped file in the `filter_map` body (convert to explicit `if let`).

- [x] T28 — Add doc comment to `FileSyncStatus::UploadSkipped`
  **File:** `core_types/src/lib.rs`
  The variant solves a non-obvious infinite-retry problem. Document why it is distinct from
  `UploadFailed`: `UploadFailed` is in the `[UploadPending, UploadFailed]` retry query with
  `offset = 0`, so writing it would cause the same file to be re-fetched and re-processed
  indefinitely within a single sync run.

- [x] T29 — Add explanatory comment to `file_type` column in first migration
  **File:** `database/migrations/20260405065807_make_archive_file_name_nullable.sql`
  The recreated table has `file_type INTEGER` (nullable). A future reader auditing migrations
  would wonder if `NOT NULL` was accidentally dropped. Add a comment clarifying that
  `file_type` was added via `ALTER TABLE ADD COLUMN` (no `NOT NULL`) in an earlier migration,
  so it was already nullable before this change.

- [x] T30 — Add doc comment to `force_invariant_violation` test helper
  **File:** `service/src/file_type_migration/steps.rs`
  The mechanism (calling `update_is_available` with `None` to produce `is_available=1,
  archive_file_name=NULL`) is non-obvious. Add a brief `///` doc comment explaining what it
  does and why.

---

## Manual Verification Checklist

- [ ] Import a file set normally — files appear as available, archive names populated
- [ ] Import with a DAT file where some files are missing — missing files recorded as
  `is_available=false, archive_file_name=NULL`; present files as `is_available=true, archive_file_name=Some`
- [ ] Re-import the same DAT file set when files are still missing — no change to missing
  file records (T13 regression check)
- [ ] Re-import the same DAT file set after the missing files become available — missing
  records are promoted to available with correct archive name
- [ ] Cloud sync runs without warnings for available files; skips missing files with warn
- [ ] File type migration skips unavailable files with warn, moves available files correctly
