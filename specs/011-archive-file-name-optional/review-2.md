# Review 2 — Make `archive_file_name` Optional

## Summary
5 findings (0 major, 2 minor, 3 nit). Tasks T26–T30 added to tasks.md Phase 6. All resolved.

## Findings

### Minor

#### R1 — Redundant double-guard in `DownloadFilesStep` → T26
**File:** `service/src/file_set_download/steps.rs`
After `generate_cloud_key()` returns `Some`, `archive_file_name` is guaranteed `Some` (they represent the same value). The second guard `let Some(archive_file_name) = &file_info.archive_file_name` can never trigger after the first.
**Fix:** Replace both with a single pattern that extracts both values simultaneously, or unwrap `archive_file_name` with `expect` after the cloud-key guard.
**Status:** [x] Fixed in T26

#### R2 — Silent skips without warning in `ExportFilesStep` and `export_service` → T27
**Files:** `service/src/file_set_download/steps.rs` (`ExportFilesStep`), `service/src/export_service.rs`
Both use `filter_map` to silently skip files with `archive_file_name=None`. If a user exports a file set with missing files they get a partial result with no indication.
**Fix:** Add `tracing::warn!` per skipped file in the `filter_map` body (convert to explicit `if let`).
**Status:** [x] Fixed in T27

### Nit

#### R3 — Missing doc comment on `FileSyncStatus::UploadSkipped` → T28
**File:** `core_types/src/lib.rs`
The variant solves a non-obvious infinite-retry problem with no explanation.
**Fix:** Add `///` doc comment explaining why it is distinct from `UploadFailed`: `UploadFailed` is in the `[UploadPending, UploadFailed]` retry query with `offset = 0`, so writing it would cause the same file to be re-fetched and re-processed indefinitely within a single sync run.
**Status:** [x] Fixed in T28

#### R4 — Missing migration comment on nullable `file_type` column → T29
**File:** `database/migrations/20260405065807_make_archive_file_name_nullable.sql`
The recreated table has `file_type INTEGER` (nullable). A future reader auditing migrations would wonder if `NOT NULL` was accidentally dropped.
**Fix:** Add a comment clarifying that `file_type` was added via `ALTER TABLE ADD COLUMN` (no `NOT NULL`) in an earlier migration, so it was already nullable before this change.
**Status:** [x] Fixed in T29

#### R5 — Missing doc comment on `force_invariant_violation` test helper → T30
**File:** `service/src/file_type_migration/steps.rs`
The mechanism (calling `update_is_available` with `None` to produce `is_available=1, archive_file_name=NULL`) is non-obvious.
**Fix:** Add a brief `///` doc comment explaining what it does and why.
**Status:** [x] Fixed in T30
