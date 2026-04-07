# Review 1 — Cloud Sync Status in `file_info`

## Summary
8 findings (3 major, 3 minor, 2 nit). Tasks T19–T26 added to tasks.md Phase 6.

## Findings

### Major

#### R1 — Off-by-one in `DeleteMarkedFilesStep` progress events → T19
**File:** `service/src/cloud_sync/steps.rs`
`file_count` is incremented *after* `FileDeletionStarted` is sent, so the first file reports `file_number: 0`. This is inconsistent with the pattern in `UploadPendingFilesStep`.
**Fix:** Move `file_count += 1` to before `FileDeletionStarted`, matching the pattern in `UploadPendingFilesStep`.
**Status:** [ ] Open

#### R2 — `DeleteFileInfosStep` filter uses `!= DeletionPending` instead of `== NotSynced` → T20
**File:** `service/src/file_import/common_steps/file_deletion_steps.rs`
The current guard `!= DeletionPending` allows deleting records with `cloud_sync_status = Synced`, which is wrong (those should become tombstones via `MarkForCloudDeletionStep`). The spec says "Only deletes `file_info` records where `cloud_sync_status = NotSynced`".
**Fix:** Change to `== CloudSyncStatus::NotSynced`.
**Status:** [ ] Open

#### R3 — Debug `println!` leftover in `DeleteFileInfosStep` → T21
**File:** `service/src/file_import/common_steps/file_deletion_steps.rs` (line ~339)
A debug `println!` was left in production code. The `tracing::info!` immediately below covers the same information.
**Fix:** Remove the `println!` block.
**Status:** [ ] Open

### Minor

#### R4 — No `rows_affected` check in `update_cloud_sync_status` → T22
**File:** `database/src/repository/file_info_repository.rs`
If called with a non-existent `id`, the UPDATE silently succeeds with zero rows. This could cause `MarkForCloudDeletionStep` to proceed believing the status was persisted when it wasn't.
**Fix:** Check `result.rows_affected() == 0` and return `Err(Error::NotFound(...))`.
**Status:** [ ] Open

#### R5 — Missing test for `DeleteMarkedFilesStep` no-archive-file-name guard → T23
**File:** `service/src/cloud_sync/steps.rs`
Mirrors `test_upload_pending_files_step_handles_missing_archive_file_name`. A `DeletionPending` tombstone with `archive_file_name = None` must not cause an infinite loop.
**Fix:** Add test that verifies the step breaks out after one batch with no progress.
**Status:** [ ] Open

#### R6 — `SyncCancelled {}` struct syntax for unit variant → T24
**File:** `service/src/cloud_sync/steps.rs` (upload cancel branch)
`SyncCancelled` is a unit variant; struct-like syntax `SyncCancelled {}` is non-canonical. The deletion loop already uses `SyncCancelled` correctly.
**Fix:** Change to `SyncCancelled` (no braces).
**Status:** [ ] Open

### Nit

#### R7 — Error message says `"file set ids"` instead of `"file info ids"` → T25
**File:** `service/src/file_type_migration/steps.rs`
The `StepAction::Abort` message in `CollectCloudFileSetsStep` says `"file set ids"` but the method is `get_synced_file_info_ids`.
**Fix:** Fix to match the `tracing::error!` above it.
**Status:** [ ] Open

#### R8 — Redundant `.into_iter().collect()` in `CollectCloudFileSetsStep` → T26
**File:** `service/src/file_type_migration/steps.rs`
`get_synced_file_info_ids()` returns `HashSet<i64>` and the context field is `HashSet<i64>`. The `.into_iter().collect()` is a no-op conversion.
**Fix:** Change to direct assignment.
**Status:** [ ] Open
