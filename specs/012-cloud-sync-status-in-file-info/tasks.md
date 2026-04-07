# Spec 012 Tasks: Cloud Sync Status in `file_info`

## Completed Tasks

- [x] T1 — Add `CloudSyncStatus` enum to `core_types` (`NotSynced`/`Synced`/`DeletionPending`)
- [x] T2 — DB migration: add `cloud_sync_status INTEGER NOT NULL DEFAULT 0` to `file_info` with
  data backfill from `file_sync_log`
- [x] T3 — Update `FileInfo` / `FileSetFileInfo` models and `FromRow` impls
- [x] T4 — Split repository methods into purpose-specific names:
  `get_files_pending_upload`, `count_files_pending_upload`, `get_files_pending_deletion`,
  `count_files_pending_deletion`, `update_cloud_sync_status`, `get_synced_file_info_ids`
- [x] T5 — Remove deprecated `mark_files_for_cloud_sync` and `get_all_synced_file_set_ids` from
  `file_sync_log_repository`
- [x] T6 — Rewrite cloud sync pipeline: remove `PrepareFilesForUploadStep`; rewrite
  `GetSyncFileCountsStep`, `UploadPendingFilesStep`, `DeleteMarkedFilesStep`
- [x] T7 — Rewrite `MarkForCloudDeletionStep` to use `cloud_sync_status`; update
  `DeleteFileInfosStep` to skip `DeletionPending` tombstones; update in-memory status after mark
- [x] T8 — Remove `MarkNewFilesForCloudSyncStep` from `update_file_set` pipeline
- [x] T9 — Update `CollectCloudFileSetsStep` in `file_type_migration` to use
  `get_synced_file_info_ids`
- [x] T10 — Update all tests; fix service-level test assertions
- [x] T11 — `cargo sqlx prepare --workspace` + `tbls doc --force`
- [x] T12 — Add `should_execute` guard to `DeleteMarkedFilesStep`
- [x] T13 — Add `test_upload_pending_files_step_upload_failure`
- [x] T14 — Add `test_delete_marked_files_step_deletion_failure`
- [x] T15 — Add `test_get_synced_file_info_ids` and `get_files_pending_deletion` tests
- [x] T16 — Remove `CleanupOrphanedSyncLogsStep` (was immediately erasing `DeletionCompleted` audit logs)
- [x] T17 — Split generic `get_file_infos_by_cloud_sync_status` into 4 purpose-specific methods
- [x] T18 — Add comment to migration explaining `DeletionCompleted → DeletionPending` mapping

## Phase 6 — Review Fixes (Round 1)
<!-- Findings documented in review-1.md -->

### Major

- [ ] T19 — Fix off-by-one in `DeleteMarkedFilesStep` progress events
  **File:** `service/src/cloud_sync/steps.rs`
  `file_count` is incremented *after* `FileDeletionStarted` is sent, so the first file reports
  `file_number: 0`. Move `file_count += 1` to before `FileDeletionStarted`, matching the
  pattern in `UploadPendingFilesStep`.

- [ ] T20 — Fix `DeleteFileInfosStep` filter to use `== NotSynced` instead of `!= DeletionPending`
  **File:** `service/src/file_import/common_steps/file_deletion_steps.rs`
  The current guard `!= DeletionPending` allows deleting records with `cloud_sync_status = Synced`,
  which is wrong (those should become tombstones via `MarkForCloudDeletionStep`). The spec says
  "Only deletes `file_info` records where `cloud_sync_status = NotSynced`". Change to
  `== CloudSyncStatus::NotSynced`.

- [ ] T21 — Remove `println!` leftover in `DeleteFileInfosStep`
  **File:** `service/src/file_import/common_steps/file_deletion_steps.rs` (line ~339)
  A debug `println!` was left in production code. The `tracing::info!` immediately below covers
  the same information. Remove the `println!` block.

- [ ] T22 — Add `rows_affected` check to `update_cloud_sync_status`
  **File:** `database/src/repository/file_info_repository.rs`
  If called with a non-existent `id`, the UPDATE silently succeeds with zero rows. This could
  cause `MarkForCloudDeletionStep` to proceed believing the status was persisted when it wasn't.
  Check `result.rows_affected() == 0` and return `Err(Error::NotFound(...))`.

### Minor

- [ ] T23 — Add test for `DeleteMarkedFilesStep` no-archive-file-name infinite loop guard
  **File:** `service/src/cloud_sync/steps.rs`
  Mirrors `test_upload_pending_files_step_handles_missing_archive_file_name`. A `DeletionPending`
  tombstone with `archive_file_name = None` must not cause an infinite loop; the step should
  break out after one batch with no progress.

- [ ] T24 — Fix `SyncCancelled {}` to `SyncCancelled` in upload loop
  **File:** `service/src/cloud_sync/steps.rs` (upload cancel branch)
  `SyncCancelled` is a unit variant; struct-like syntax `SyncCancelled {}` is non-canonical.
  Change to match the deletion loop which already uses `SyncCancelled`.

- [ ] T25 — Fix error message: `"file set ids"` → `"file info ids"`
  **File:** `service/src/file_type_migration/steps.rs`
  The `StepAction::Abort` message in `CollectCloudFileSetsStep` says `"file set ids"` but
  the method is `get_synced_file_info_ids`. Fix to match the `tracing::error!` above it.

### Nit

- [ ] T26 — Remove redundant `.into_iter().collect()` in `CollectCloudFileSetsStep`
  **File:** `service/src/file_type_migration/steps.rs`
  `get_synced_file_info_ids()` returns `HashSet<i64>` and the context field is `HashSet<i64>`.
  Change `file_info_ids.into_iter().collect()` to direct assignment `file_info_ids`.

---

## Manual Verification Checklist

- [ ] Import a file set — `cloud_sync_status` defaults to `NotSynced`
- [ ] Trigger cloud sync — files are uploaded and `cloud_sync_status` changes to `Synced`
- [ ] Delete a file set (synced files) — `file_info` records remain with `DeletionPending`
  (verify in DB: `SELECT id, cloud_sync_status FROM file_info WHERE cloud_sync_status = 2`)
- [ ] Trigger cloud sync again — `DeletionPending` files are deleted from cloud and `file_info`
  records are removed
- [ ] Delete a file set (never-synced files) — `file_info` records are removed immediately
  (no tombstone)
- [ ] Disable cloud sync and import a new file set — `cloud_sync_status = NotSynced`; re-enable
  and trigger sync to confirm files are picked up
