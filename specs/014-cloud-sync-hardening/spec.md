# 014 — Cloud Sync Hardening

## Status
Complete

## Affected Crates
- `core_types` — add `SyncFailed` event variant; update `SyncStarted` fields
- `cloud_storage` — add `cloud_key(file_type, archive_file_name)` free function; remove `generate_cloud_key` from `database` models
- `database` — new `CloudSyncableFileInfo` model; new `get_cloud_files_pending_deletion` / `get_tombstones_pending_deletion` queries and counts; updated `get_files_pending_upload` return type; remove `generate_cloud_key` from `FileInfo`
- `service` — fix loop termination in upload/delete steps; split `DeleteMarkedFilesStep` into two; move lifecycle events to `service.rs`; surface partial successes in `SyncResult`; update all callers of `generate_cloud_key`

## Problem

Four independent hardening gaps were found in `service/src/cloud_sync/`:

### H1 — Loop termination bug silently abandons untried files

Both `UploadPendingFilesStep` and `DeleteMarkedFilesStep` use a paginated loop that breaks
when `batch_uploaded == 0` / `batch_deleted == 0`. The intent is to avoid an infinite loop
when files have no `archive_file_name`, but the same condition fires when all files in a batch
fail at the cloud layer — causing files 11+ (from subsequent pages) to be silently skipped
for the rest of the sync session.

Additionally, in `DeleteMarkedFilesStep`, tombstones with `archive_file_name = NULL` (files
marked `DeletionPending` that were never archived) are skipped via `continue` but never
removed from `file_info`. They re-appear in every sync run forever.

For the upload path, `get_files_pending_upload` already filters
`WHERE archive_file_name IS NOT NULL`, making the `generate_cloud_key()` skip guard dead
code. The type system should reflect this invariant.

### H2 — `SyncCompleted` event sent at wrong time and sometimes never sent

`SyncCompleted` is emitted inside `UploadPendingFilesStep::execute`, before
`DeleteMarkedFilesStep` runs. The UI therefore receives "sync complete" and then continues
to get `FileDeletionStarted/Completed` events.

If `files_prepared_for_upload == 0`, `UploadPendingFilesStep` is skipped entirely
(`should_execute` returns `false`). In this case — e.g. only deletions queued, or nothing
to do at all — `SyncCompleted` is never sent, leaving the caller's progress channel open
with no terminal event.

### H3 — Partial successes (ghost uploads) silently ignored

When a file uploads to cloud successfully but the subsequent DB update fails,
`FileSyncResult::is_partial_success()` returns `true`. The code tracks this in
`SyncContext::partial_successful_uploads()` but the count is not included in `SyncResult`
and `service.rs` emits no warning. The file stays `NotSynced` and is re-uploaded on every
subsequent sync until the DB recovers.

### H4 — `SyncStarted` event only reflects upload count; missing for deletion-only runs

`SyncStarted { total_files_count }` is emitted inside `UploadPendingFilesStep` with only
the upload count. If there are zero uploads (only deletions), `SyncStarted` is never sent.
The progress consumer has no upfront summary of the full sync work to be done.

## Proposed Solution

### H1a — Introduce `CloudSyncableFileInfo` with `archive_file_name: String`

Add a new model `CloudSyncableFileInfo` in `database/src/models.rs` with
`archive_file_name: String` (non-optional). Change `get_files_pending_upload` to return
`Vec<CloudSyncableFileInfo>`. This removes the dead-code skip guard in
`UploadPendingFilesStep` and makes the type system enforce the `archive_file_name IS NOT NULL`
invariant already present in the SQL query.

`CloudSyncableFileInfo` does **not** have a `generate_cloud_key` method — callers use the
`cloud_storage::cloud_key` free function (see H1d below).

### H1d — Move `generate_cloud_key` to `cloud_storage` as a single free function

There is currently one `generate_cloud_key` method on `FileInfo` in `database/src/models.rs`.
It belongs in `cloud_storage` because the cloud key format is S3-specific, not a database
concern. There should be exactly one implementation in the entire codebase.

Add to `cloud_storage/src/lib.rs`:

```rust
/// Compute the S3 cloud key for a file given its type and archive file name.
/// Format: `{file_type_dir}/{archive_file_name}` (e.g. `"rom/game.zst"`, `"disk_image/disk.zst"`).
pub fn cloud_key(file_type: FileType, archive_file_name: &str) -> String {
    format!("{}/{}", file_type.dir_name(), archive_file_name)
}
```

`cloud_storage` already depends on `core_types` (for `FileType`), so no new dependency is
needed. Note: the existing `FileInfo::generate_cloud_key` incorrectly uses
`file_type.to_string().to_lowercase()`, which produces `"disk image"` (space) instead of
`"disk_image"` (underscore) for `FileType::DiskImage`. The new function fixes this by using
`FileType::dir_name()`.

Remove `generate_cloud_key` from `FileInfo`. Update all callers in `service` to use
`cloud_storage::cloud_key(file.file_type, &file.archive_file_name)`.

### H1b — Split deletion into cloud-deletion and tombstone-cleanup

Add two new repository methods alongside the existing `get_files_pending_deletion`:
- `get_cloud_files_pending_deletion(limit, offset)` → `Vec<CloudSyncableFileInfo>`:
  `WHERE cloud_sync_status = DeletionPending AND archive_file_name IS NOT NULL`
- `get_tombstones_pending_deletion(limit, offset)` → `Vec<FileInfo>`:
  `WHERE cloud_sync_status = DeletionPending AND archive_file_name IS NULL`
- Matching `count_cloud_files_pending_deletion()` and `count_tombstones_pending_deletion()`.

Split `DeleteMarkedFilesStep` into two:
- `DeleteCloudFilesStep`: uses `get_cloud_files_pending_deletion`, performs cloud delete +
  `delete_file_info`. All files are guaranteed to have a cloud key — no skip guard needed.
- `CleanupTombstonesStep`: uses `get_tombstones_pending_deletion`, calls only
  `delete_file_info` (no cloud op). These records were never uploaded, nothing to delete
  from cloud.

Add `tombstones_cleaned_up` to `SyncContext` so the service can report it.

### H1c — Fix loop termination with session offset

For `UploadPendingFilesStep` and `DeleteCloudFilesStep`, files that fail a cloud operation
stay in their pending state and re-appear at `OFFSET 0` on the next fetch. The original
`batch_uploaded == 0` break misfires when an entire batch fails, abandoning all subsequent
pages.

Fix: track `session_skip: i64 = 0` before the loop. Pass it as the offset to each batch
fetch. After processing each file, increment `session_skip` only for **failed** files (they
stay pending and occupy offset positions). Successful files become `Synced`/deleted and
disappear from the pending set naturally, so they don't consume offset slots. Break when
the fetched batch is empty (no more pending files at or beyond `session_skip`).

```
session_skip = 0
loop:
    batch = get_files_pending_upload(LIMIT 10, OFFSET session_skip)
    if batch.is_empty(): break
    for file in batch:
        attempt upload
        if failed: session_skip += 1
```

For `CleanupTombstonesStep`, loop termination uses `batch_cleaned == 0`: if no tombstone
was successfully deleted in a batch (e.g. all `delete_file_info` calls failed), break to
avoid an infinite loop. Successful deletions remove the record, so the next fetch at
`OFFSET 0` returns the next unprocessed tombstone naturally.

### H2 — Move lifecycle events to `service.rs`; steps send no terminal events

Remove **all** terminal event sends from steps. Steps must only return `StepAction` — they
never send `SyncStarted`, `SyncCompleted`, `SyncCancelled`, or `SyncFailed`.

Currently `UploadPendingFilesStep` sends `SyncCompleted` and the cancellation check sends
`SyncCancelled` before returning `Abort`. Both must be removed.

In `CloudStorageSyncService::sync_to_cloud`, **before** calling `pipeline.execute()`:

1. Run the three count queries (`count_files_pending_upload`,
   `count_cloud_files_pending_deletion`, `count_tombstones_pending_deletion`) directly and
   populate context fields.
2. Send `SyncEvent::SyncStarted { total_upload_count, total_deletion_count }`.
3. Remove `GetSyncFileCountsStep` from the pipeline (its work is now done in service.rs).
   Pipeline becomes: `[ConnectToCloud, Upload, DeleteCloud, CleanupTombstones]`.

After `pipeline.execute()` returns:
- `Ok(_)` → send `SyncEvent::SyncCompleted`
- `Err(Error::OperationCancelled)` → send `SyncEvent::SyncCancelled`
- `Err(e)` → send `SyncEvent::SyncFailed { error: e.to_string() }`

Add `SyncFailed { error: String }` to `SyncEvent` in `core_types`.

### H3 — Surface partial successes in `SyncResult`

Add `partial_successful_uploads: usize` to `SyncResult`. Populate it from
`context.partial_successful_uploads()` in `service.rs`. Emit a `tracing::warn!` when this
count is > 0.

### H4 — Send `SyncStarted` from `service.rs` with full counts

Remove `SyncEvent::SyncStarted` from `UploadPendingFilesStep`.

In `CloudStorageSyncService::sync_to_cloud`, after the context is populated by
`GetSyncFileCountsStep` (i.e., after the pipeline finishes the first step), send
`SyncStarted` with both upload and deletion counts. Since `GetSyncFileCountsStep` is the
first pipeline step and runs unconditionally, the cleanest approach is to send `SyncStarted`
from `service.rs` *after* running only `GetSyncFileCountsStep` and *before* the rest of the
pipeline — or alternatively, restructure `SyncStarted` to be sent at the very beginning with
combined counts from both `files_prepared_for_upload` and `files_prepared_for_deletion` that
were set in `GetSyncFileCountsStep`.

Because the current `Pipeline::execute` runs all steps atomically, the simplest approach is
to send `SyncStarted { total_upload_count, total_deletion_count }` from `service.rs` *before*
calling `pipeline.execute()`, using a count query run ahead of time (reusing the existing
repository calls). Update `SyncEvent::SyncStarted` to carry both counts.

## Key Decisions

| Decision | Rationale |
|---|---|
| `CloudSyncableFileInfo` is a separate model, not a trait | Simplest; avoids generic gymnastics. The struct mirrors `FileInfo` but with `archive_file_name: String`. |
| `CloudSyncableFileInfo` has no `generate_cloud_key` method | Callers use `cloud_storage::cloud_key` free function — cloud key format is not the model's concern. |
| `generate_cloud_key` moves to `cloud_storage` as a free function | Cloud key format is S3-specific. `cloud_storage` already depends on `core_types`, so `FileType` is accessible with no new dep. One implementation, one place. |
| Use `FileType::dir_name()` in `cloud_key`, not `to_string().to_lowercase()` | `DiskImage.to_string()` gives `"Disk Image"` (strum serialize); `.to_lowercase()` gives `"disk image"` (space), not `"disk_image"`. `dir_name()` is already correct. |
| Keep `get_files_pending_deletion` as-is | Existing callers (if any) are not broken; new split queries are additions. |
| Session-offset loop termination, not a "seen" set | A seen-set with `batch_tried == 0` breaks before reaching page 2 when all of page 1 fails. Offset-based pagination correctly advances past failed files. |
| Steps never send terminal events; `service.rs` is sole sender | Sending `SyncCancelled` in the step and again in service.rs would duplicate the event. Service owns the sync lifecycle; steps own individual operations. |
| `GetSyncFileCountsStep` removed from pipeline; counts run in `service.rs` | Service needs counts before the pipeline runs (to send `SyncStarted`). Running them in `service.rs` first and removing the step avoids duplicate DB queries and is cleaner. |
| `SyncFailed` event added to `core_types` | Consumers (UI) need a terminal error event to exit "syncing" state cleanly. |
| `SyncStarted` updated to carry both upload and deletion counts | UI needs a complete picture of the sync run from the start. |

## Acceptance Criteria

- [ ] A sync run where the first batch all fail cloud upload still attempts all remaining files.
- [ ] Tombstones (`DeletionPending` + `archive_file_name = NULL`) are removed from `file_info` during sync; they no longer re-appear on subsequent runs.
- [ ] A tombstone cleanup loop with a DB failure on every record breaks out rather than looping forever.
- [ ] `SyncCompleted` is always sent after the full pipeline finishes (uploads + deletions), not mid-pipeline.
- [ ] `SyncCompleted` is sent even when there are zero uploads and/or zero deletions.
- [ ] `SyncCancelled` is sent exactly once when sync is cancelled; steps do not send it.
- [ ] `SyncFailed` is sent if the pipeline aborts with a non-cancellation error.
- [ ] `SyncResult.partial_successful_uploads` is populated; a `tracing::warn!` is emitted when > 0.
- [ ] `SyncStarted` carries both upload count and deletion count and is always sent at the start.
- [ ] `cloud_storage::cloud_key` is the single implementation of cloud key generation; `FileInfo::generate_cloud_key` is removed.
- [ ] Cloud keys for multi-word file types (e.g. `DiskImage`) use underscores, not spaces.
- [ ] All existing tests pass; new tests cover the fixed/new behaviours.

## As Implemented

Implemented as proposed with the following notes:

- **H1 (loop termination):** Replaced the `batch_uploaded == 0` / `batch_deleted == 0` break guard with a session-offset algorithm. A `session_skip: i64` counter tracks how many files in the current session have failed (cloud error); the fetch offset advances by `session_skip`, ensuring the loop always makes forward progress and terminates when the offset returns an empty batch.

- **H2 (tombstone split):** `DeleteMarkedFilesStep` was split into two independent steps: `DeleteCloudFilesStep` (processes `DeletionPending` files with `archive_file_name IS NOT NULL` — requires a cloud delete + DB remove) and `CleanupTombstonesStep` (processes `DeletionPending` files with `archive_file_name IS NULL` — DB remove only). `ConnectToCloudStep` skips connection when only tombstones are queued.

- **H3 (pre-pipeline counts):** `sync_to_cloud` in `service.rs` now queries `count_files_pending_upload`, `count_cloud_files_pending_deletion`, and `count_tombstones_pending_deletion` before the pipeline runs, storing them in `SyncContext`. `SyncStarted` is emitted with `total_upload_count` and `total_deletion_count` before the pipeline executes.

- **H4 (lifecycle events):** Terminal events (`SyncCompleted`, `SyncCancelled`, `SyncFailed`) are now emitted exclusively in `service.rs` after the pipeline returns, matching the pipeline result. Per-step `SyncCompleted` emissions were removed.

- **`generate_cloud_key` consolidation:** The method was removed from `FileInfo` and replaced with a free function `cloud_key(file_type, archive_file_name)` in the `cloud_storage` crate. All callers updated.

- **`CloudSyncableFileInfo` model:** New struct in `database::models` guarantees `archive_file_name` is non-optional for upload/deletion paths. `TryFrom<FileInfo>` conversion used in repository methods.

- **Phase 6 review fixes:** DRY violation in repository (duplicate conversion) extracted to private helper; `FileDeletionCompleted` now only emitted when both cloud and DB operations succeed; doc comments added to `SyncResult`.
