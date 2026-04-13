# 014 — Cloud Sync Hardening: Task Breakdown

## Phase 3 — Implementation

### Cloud Storage

- [x] T1 [cloud_storage] — Add `cloud_key` free function
  **File:** `cloud_storage/src/lib.rs`
  Add:
  ```rust
  /// Compute the S3 cloud key for a file: `"{file_type_dir}/{archive_file_name}"`.
  pub fn cloud_key(file_type: FileType, archive_file_name: &str) -> String {
      format!("{}/{}", file_type.dir_name(), archive_file_name)
  }
  ```
  Add `use core_types::FileType;` if not already present.

### Core Types

- [x] T2 [core_types] — Add `SyncFailed` event variant to `SyncEvent`
  **File:** `core_types/src/events.rs`
  Add `SyncFailed { error: String }` variant to the `SyncEvent` enum.

- [x] T3 [core_types] — Update `SyncStarted` to carry both upload and deletion counts
  **File:** `core_types/src/events.rs`
  Change `SyncStarted { total_files_count: i64 }` to
  `SyncStarted { total_upload_count: i64, total_deletion_count: i64 }`.
  Update all match/construction sites in the codebase.

### Database

- [x] T4 [database] — Add `CloudSyncableFileInfo` model; remove `generate_cloud_key` from `FileInfo`
  **File:** `database/src/models.rs`
  Add struct:
  ```rust
  pub struct CloudSyncableFileInfo {
      pub id: i64,
      pub sha1_checksum: Sha1Checksum,
      pub file_size: u64,
      pub archive_file_name: String,  // guaranteed non-null by query
      pub file_type: FileType,
      pub cloud_sync_status: CloudSyncStatus,
  }
  ```
  Implement `sqlx::FromRow` for it (or use `query_as` with matching field names).
  Remove `FileInfo::generate_cloud_key()`. Do not add `generate_cloud_key` to
  `CloudSyncableFileInfo`; callers use `cloud_storage::cloud_key(file.file_type, &file.archive_file_name)`.

- [x] T5 [database] — Change `get_files_pending_upload` return type to `Vec<CloudSyncableFileInfo>`
  **File:** `database/src/repository/file_info_repository.rs`
  Update the query to map to `CloudSyncableFileInfo`. The SQL is unchanged
  (`WHERE cloud_sync_status = ? AND archive_file_name IS NOT NULL`). Update return type.

- [x] T6 [database] — Add split deletion queries
  **File:** `database/src/repository/file_info_repository.rs`
  Add four new methods:
  - `get_cloud_files_pending_deletion(limit, offset) -> Result<Vec<CloudSyncableFileInfo>, Error>`
    SQL: `WHERE cloud_sync_status = DeletionPending AND archive_file_name IS NOT NULL`
  - `count_cloud_files_pending_deletion() -> Result<i64, Error>`
  - `get_tombstones_pending_deletion(limit, offset) -> Result<Vec<FileInfo>, Error>`
    SQL: `WHERE cloud_sync_status = DeletionPending AND archive_file_name IS NULL`
  - `count_tombstones_pending_deletion() -> Result<i64, Error>`

### Service — Context

- [x] T7 [service] — Update `SyncContext` for split deletion counts
  **File:** `service/src/cloud_sync/context.rs`
  - Rename `files_prepared_for_deletion` → `cloud_files_prepared_for_deletion`.
  - Add `tombstones_prepared_for_cleanup: i64`.
  - Add `tombstones_cleaned_up: usize` result counter.
  - Update `should_connect` to use `cloud_files_prepared_for_deletion`.
  - Update `failed_deletions()` / `successful_deletions()` to use the new field name.

### Service — Steps

- [x] T8 [service] — Update `UploadPendingFilesStep`: new type, remove dead guards + lifecycle events, fix loop
  **File:** `service/src/cloud_sync/steps.rs`
  - Remove both `generate_cloud_key()` skip guards (type guarantee from `CloudSyncableFileInfo`).
  - Update file variable type to `CloudSyncableFileInfo`; replace key generation with
    `cloud_storage::cloud_key(file.file_type, &file.archive_file_name)`.
  - Remove `send_progress_event(SyncEvent::SyncStarted { ... }, ...)`.
  - Remove `send_progress_event(SyncEvent::SyncCompleted {}, ...)`.
  - Remove `send_progress_event(SyncEvent::SyncCancelled, ...)` from cancel check
    (keep the `return StepAction::Abort(Error::OperationCancelled)`).
  - Replace `batch_uploaded == 0` break with session-offset termination:
    track `session_skip: i64 = 0` before the loop; pass it as `offset` to
    `get_files_pending_upload`; increment `session_skip` for each failed file; break when
    the fetched batch is empty.
  - Delete or rewrite `test_upload_progress_messages`: it asserts `SyncStarted` and
    `SyncCompleted` are emitted from this step; after this task those events move to
    `service.rs` and the test no longer applies.

- [x] T9 [service] — Replace `DeleteMarkedFilesStep` with `DeleteCloudFilesStep` + `CleanupTombstonesStep`
  **File:** `service/src/cloud_sync/steps.rs`
  - Rename `DeleteMarkedFilesStep` → `DeleteCloudFilesStep`. Update to use
    `get_cloud_files_pending_deletion` (returns `Vec<CloudSyncableFileInfo>`). Remove
    `generate_cloud_key()` skip guard; use `cloud_storage::cloud_key(...)`.
    Remove `SyncCancelled` send from cancel check (keep `StepAction::Abort`).
    Apply session-offset termination: `session_skip` increments on each failed deletion;
    break when batch is empty.
    Update `should_execute` to check `context.cloud_files_prepared_for_deletion > 0`.
  - Add new `CleanupTombstonesStep`. Loop: `get_tombstones_pending_deletion(10, 0)`.
    For each tombstone, call `delete_file_info(file.id)`.
    Track `batch_cleaned`: increment on success. Break when `batch_cleaned == 0` (protects
    against infinite loop if all DB deletes fail in a batch).
    Track `context.tombstones_cleaned_up` on each success.
    `should_execute`: `context.tombstones_prepared_for_cleanup > 0`.
  - Delete `test_delete_marked_files_step_handles_missing_archive_file_name`: this test
    verified the old skip-via-`continue` path for tombstones; that behaviour is superseded
    by `CleanupTombstonesStep` and covered by T21/T22.

- [x] T10 [service] — Remove `GetSyncFileCountsStep` from pipeline and delete the step
  **File:** `service/src/cloud_sync/pipeline.rs`, `service/src/cloud_sync/steps.rs`
  Remove `GetSyncFileCountsStep` from the pipeline definition and delete the struct and
  its test (`test_get_sync_file_counts_step`).
  Pipeline becomes: `[ConnectToCloud, UploadPending, DeleteCloud, CleanupTombstones]`.

### Service — Service Layer

- [x] T11 [service] — Run counts + send `SyncStarted` in `service.rs` before pipeline
  **File:** `service/src/cloud_sync/service.rs`
  Before calling `pipeline.execute()`, directly run the three count queries and populate
  context fields:
  - `count_files_pending_upload()` → `context.files_prepared_for_upload`
  - `count_cloud_files_pending_deletion()` → `context.cloud_files_prepared_for_deletion`
  - `count_tombstones_pending_deletion()` → `context.tombstones_prepared_for_cleanup`
  Then send:
  ```rust
  SyncEvent::SyncStarted {
      total_upload_count: context.files_prepared_for_upload,
      total_deletion_count: context.cloud_files_prepared_for_deletion,
  }
  ```

- [x] T12 [service] — Send terminal events from `service.rs`
  **File:** `service/src/cloud_sync/service.rs`
  After `pipeline.execute(&mut context).await`:
  - `Ok(_)` → send `SyncEvent::SyncCompleted`
  - `Err(Error::OperationCancelled)` → send `SyncEvent::SyncCancelled`
  - `Err(e)` → send `SyncEvent::SyncFailed { error: e.to_string() }`

- [x] T13 [service] — Add `partial_successful_uploads` and `tombstones_cleaned_up` to `SyncResult`
  **File:** `service/src/cloud_sync/service.rs`
  Add both fields to `SyncResult`. Populate from `context.partial_successful_uploads()` and
  `context.tombstones_cleaned_up`. Add `tracing::warn!` when `partial_successful_uploads > 0`:
  ```
  partial_successful_uploads, "Ghost uploads detected: files uploaded to cloud but DB not updated"
  ```

### Service — Update Other Callers of `generate_cloud_key`

- [x] T14 [service] — Update all remaining `generate_cloud_key` callers to `cloud_storage::cloud_key`
  **Files:** `service/src/file_set_download/steps.rs`, `service/src/file_type_migration/steps.rs`,
  `service/src/file_import/service.rs`
  Replace each `file_info.generate_cloud_key()` call with
  `cloud_storage::cloud_key(file_info.file_type, archive_file_name)`, handling the `Option`
  from `archive_file_name` at the call site where needed.

## Phase 5 — Tests

- [x] T20 [service] — Test that upload loop tries all files when first batch all fail
  **File:** `service/src/cloud_sync/steps.rs` (test module)
  Set up 15 files. Configure `MockCloudStorage` to fail uploads for files 1–10. Verify that
  files 11–15 are attempted (appear in `upload_results`) and succeed.

- [x] T21 [service] — Test that tombstones are cleaned up by `CleanupTombstonesStep`
  **File:** `service/src/cloud_sync/steps.rs` (test module)
  Insert 3 `DeletionPending` records with `archive_file_name = NULL`. Run
  `CleanupTombstonesStep`. Verify all 3 are removed from `file_info` and
  `context.tombstones_cleaned_up == 3`.

- [x] T22 [service] — Test that tombstone cleanup handles more than one batch
  **File:** `service/src/cloud_sync/steps.rs` (test module)
  Insert 11 tombstones. Run `CleanupTombstonesStep`. Verify all 11 are removed.

- [x] T23 [service] — Test that `DeleteCloudFilesStep` loop tries all files when first batch all fail
  **File:** `service/src/cloud_sync/steps.rs` (test module)
  Analogous to T20 for cloud deletions: 15 files, first 10 fail, verify 11–15 are attempted.

- [x] T24 [database] — Test `get_cloud_files_pending_deletion` and `get_tombstones_pending_deletion`
  **File:** `database/src/repository/file_info_repository.rs` (test module)
  Insert a mix of `DeletionPending` records with and without `archive_file_name`. Verify each
  query returns only the expected subset.

- [x] T25 [service] — Test that `SyncCompleted` is sent even when no uploads are queued
  **File:** `service/src/cloud_sync/service.rs` or pipeline integration test
  A sync run with 0 uploads and > 0 deletions must emit `SyncStarted` then `SyncCompleted`.

- [x] T26 [service] — Test that `SyncFailed` is sent on pipeline abort
  **File:** `service/src/cloud_sync/service.rs` or pipeline integration test
  Simulate a DB error in the count phase. Verify `SyncFailed` is sent.

- [x] T27 [service] — Test that `partial_successful_uploads` is counted correctly
  **File:** `service/src/cloud_sync/steps.rs` (test module)
  Test at the context level: directly insert a `FileSyncResult` with
  `cloud_operation_success = true, db_update_success = false` into
  `context.upload_results`. Assert `context.partial_successful_uploads() == 1`.
  Also insert a result with both `true` and assert it does not count as partial.

- [x] T28 [cloud_storage] — Test `cloud_key` free function
  **File:** `cloud_storage/src/lib.rs` (test module)
  Verify correct output for `Rom` (`"rom/file.zst"`) and `DiskImage` (`"disk_image/file.zst"`)
  to confirm the underscore format and absence of the `to_lowercase()` bug.

- [x] T29 [service] — Test that `CleanupTombstonesStep` exits cleanly when DB has no tombstones at run time
  **File:** `service/src/cloud_sync/steps.rs` (test module)
  Set `context.tombstones_prepared_for_cleanup = 5` but insert no tombstone records
  in the DB (simulating concurrent deletion between count and step execution).
  Run `CleanupTombstonesStep`. Verify the first fetch returns empty, `batch_cleaned == 0`
  triggers the break, `context.tombstones_cleaned_up == 0`, and the step returns
  `StepAction::Continue` without hanging.

- [x] T30 [service] — Test that `SyncCancelled` is sent exactly once and not from a step
  **File:** `service/src/cloud_sync/service.rs` or pipeline integration test
  Start a sync run with pending files. Cancel via the cancel channel mid-run. Collect all
  events from the progress channel. Assert: exactly one `SyncCancelled` event appears, and no
  step-emitted `SyncCancelled` precedes the pipeline return (i.e. the count is 1, not 2).

- [x] T31 [service] — Test that `SyncStarted` carries correct upload and deletion counts
  **File:** `service/src/cloud_sync/service.rs` or pipeline integration test
  Insert 3 files pending upload and 2 files pending cloud deletion. Run sync. Assert
  `SyncStarted { total_upload_count: 3, total_deletion_count: 2 }` is the first event.

- [ ] T32 [service] — Test that `SyncCompleted` arrives after both uploads and deletions
  **File:** `service/src/cloud_sync/service.rs` or pipeline integration test
  Queue 2 pending uploads + 2 pending cloud deletions. Collect events. Assert the event
  sequence ends with `SyncCompleted` and that `FileDeletionCompleted` events appear before it
  (not after).

- [x] T33 [service] — Test that `SyncCompleted` is sent when there is nothing to sync
  **File:** `service/src/cloud_sync/service.rs` or pipeline integration test
  Run sync with an empty DB (no pending uploads, no pending deletions, no tombstones). Assert
  `SyncStarted { total_upload_count: 0, total_deletion_count: 0 }` is sent, followed by
  `SyncCompleted`.

- [x] T34 [service] — Test `CleanupTombstonesStep::should_execute` returns false when no tombstones
  **File:** `service/src/cloud_sync/steps.rs` (test module)
  Build a `SyncContext` with `tombstones_prepared_for_cleanup = 0`. Call
  `CleanupTombstonesStep::should_execute`. Assert it returns `false` with no DB calls made.

- [x] T35 [service] — Test `DeleteCloudFilesStep::should_execute` returns false when no cloud deletions
  **File:** `service/src/cloud_sync/steps.rs` (test module)
  Build a `SyncContext` with `cloud_files_prepared_for_deletion = 0`. Call
  `DeleteCloudFilesStep::should_execute`. Assert it returns `false` with no DB calls made.

## Phase 6 — Review Fixes (Round 1)

<!-- Tasks generated from review-1.md findings -->

- [x] T36 [database] — Extract shared `FileInfo → CloudSyncableFileInfo` conversion to private helper → R1
  **File:** `database/src/repository/file_info_repository.rs`
  Add a private `fn to_cloud_syncable(rows: Vec<FileInfo>, context: &str) -> Result<Vec<CloudSyncableFileInfo>, Error>`
  helper and replace the duplicate 5-line conversion at lines ~132–138 and ~204–210 with a single call each.

- [x] T37 [service] — Only emit `FileDeletionCompleted` when DB cleanup succeeds; emit `FileDeletionFailed` otherwise → R2
  **File:** `service/src/cloud_sync/steps.rs`
  After the `match (log_res, delete_res)` block, branch on `file_deletion_result.db_update_success`:
  - `true` → emit `FileDeletionCompleted` as now
  - `false` → emit `FileDeletionFailed` with `file_deletion_result.db_error` and increment `session_skip`

- [x] T38 [service] — Add struct-level and field-level doc comments to `SyncResult` → R3
  **File:** `service/src/cloud_sync/service.rs:L302–312`
  Add `///` doc comment on the struct and on the `tombstones_cleaned_up` field.

## Manual Verification Checklist

- [ ] Run a sync with files pending upload. Verify `SyncStarted` event appears in UI with correct upload and deletion counts.
- [ ] Trigger a cloud error mid-upload-batch. Verify remaining files are still attempted and `SyncCompleted` fires at the end.
- [ ] Mark a file `DeletionPending` that was never archived (`archive_file_name = NULL`). Verify it is cleaned up (removed from `file_info`) during sync without a cloud delete attempt.
- [ ] Run a sync with only deletions queued (no uploads). Verify `SyncStarted` and `SyncCompleted` are both emitted.
- [ ] Cancel a sync mid-run. Verify `SyncCancelled` (not `SyncCompleted`) is the terminal event, sent exactly once.
- [ ] Trigger a hard abort (e.g. DB unavailable during sync). Verify `SyncFailed` is sent.
