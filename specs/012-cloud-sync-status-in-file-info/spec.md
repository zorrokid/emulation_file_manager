# Spec 012 – Cloud Sync Status in `file_info`

## Status
<!-- Planning | In Progress | Complete | Abandoned -->
In Progress

## Affected Crates
- `core_types` — new `CloudSyncStatus` enum
- `database` — migration adding `cloud_sync_status` column, new repository methods
- `service` — cloud sync pipeline rewrite, deletion pipeline changes, import pipeline simplification
- `file_import` — remove `MarkNewFilesForCloudSyncStep`

## Problem

The current cloud sync design tracks "pending" states through `file_sync_log` entries:

- Before uploading, a `PrepareFilesForUploadStep` writes `UploadPending` log entries for every
  `file_info` record that has no sync log.
- Before deletion, `MarkForCloudDeletionStep` writes a `DeletionPending` log entry for any
  synced file that is being deleted locally.

This creates two problems:

1. **Unnecessary write amplification.** Every new file triggers a pre-upload log write before
   any actual cloud operation happens.
2. **Latent correctness bug.** The `DeleteMarkedFilesStep` joins `file_sync_log` with
   `file_info` via INNER JOIN. Because `DeleteFileInfosStep` deletes the `file_info` record
   *immediately* (even for `DeletionPending` files), the join silently drops those rows. The
   orphaned `DeletionPending` log entries are then swept away by `CleanupOrphanedSyncLogsStep`,
   meaning **the cloud file is never actually deleted**.

## Proposed Solution

Add a `cloud_sync_status` column to `file_info`. This column becomes the single source of
truth for where a file stands in the cloud lifecycle. The `file_sync_log` table is retained
for **audit/history only** (completed uploads, completed deletions, failures).

### New `CloudSyncStatus` type (in `core_types`)

| Value | Int | Meaning |
|---|---|---|
| `NotSynced` | 0 | Default. Never successfully uploaded, or a previous upload failed and will be retried. |
| `Synced` | 1 | Successfully uploaded to cloud. |
| `DeletionPending` | 2 | Local file deleted; cloud copy still needs to be deleted. |

> `UploadFailed` is **not** a separate status. On upload failure the record stays `NotSynced`
> so it is automatically retried on the next sync run. The failure is still recorded in
> `file_sync_log` for diagnosis.

### New Cloud Sync Pipeline

| # | Step | Change |
|---|---|---|
| 1 | ~~PrepareFilesForUploadStep~~ | **Removed.** New `file_info` records start as `NotSynced` by default. |
| 2 | `GetSyncFileCountsStep` | Now counts `file_info` rows by `cloud_sync_status` instead of log entries. |
| 3 | `ConnectToCloudStep` | Unchanged. |
| 4 | `UploadPendingFilesStep` | Queries `file_info WHERE cloud_sync_status = NotSynced AND is_available = 1`. On success sets `cloud_sync_status = Synced`. On failure leaves `NotSynced`. |
| 5 | `DeleteMarkedFilesStep` | Queries `file_info WHERE cloud_sync_status = DeletionPending`. On success deletes the `file_info` record. On failure leaves `DeletionPending`. |
| 6 | ~~CleanupOrphanedSyncLogsStep~~ | **Removed.** In the new tombstone design the only logs that become "orphaned" are `DeletionCompleted` entries written by `DeleteMarkedFilesStep` moments earlier — running cleanup immediately erases the audit trail. Since `file_sync_log` has no FK to `file_info`, these rows are intentional audit records. |

### File Deletion Pipeline Changes

`MarkForCloudDeletionStep`:
- If `file_info.cloud_sync_status = Synced` → set `cloud_sync_status = DeletionPending`. **Do
  not delete `file_info`**; it acts as a tombstone for the pending cloud deletion.
- If `file_info.cloud_sync_status = NotSynced` → no cloud action needed; proceed to
  `DeleteFileInfosStep` as normal.

`DeleteFileInfosStep`:
- Only deletes `file_info` records where `cloud_sync_status = NotSynced`. Files with
  `DeletionPending` are retained until the cloud deletion is confirmed.

### Import Pipeline Changes

`MarkNewFilesForCloudSyncStep` (in `update_file_set` pipeline) is **removed**. New `file_info`
records default to `NotSynced`, so they are picked up automatically on the next sync.

### Database Migration

A new migration adds the column:

```sql
ALTER TABLE file_info ADD COLUMN cloud_sync_status INTEGER NOT NULL DEFAULT 0;
```

Data migration in the same migration: set `cloud_sync_status` based on the latest
`file_sync_log` entry for each `file_info`:

- Latest status is `UploadCompleted` → `Synced` (1)
- Latest status is `DeletionPending`, `DeletionInProgress`, or `DeletionCompleted` → `DeletionPending` (2)
- All others (or no log) → `NotSynced` (0)

Existing `UploadPending` log entries may be left in the log table; they
will be ignored by the new pipeline. `DeletionCompleted` entries written by
`DeleteMarkedFilesStep` are retained as intentional audit records (no cleanup step).

## Acceptance Criteria

1. When a new `file_info` is created, `cloud_sync_status` defaults to `NotSynced`.
2. Triggering cloud sync uploads all `NotSynced + is_available` files and sets them `Synced`
   on success.
3. If an upload fails, the file remains `NotSynced` and a failure entry is written to
   `file_sync_log`.
4. Deleting a file set where a file has `cloud_sync_status = Synced` sets that file's status
   to `DeletionPending` without deleting the `file_info` record.
5. Triggering cloud sync deletes all `DeletionPending` cloud files and, on success, deletes the
   `file_info` record.
6. If a cloud deletion fails, `file_info` remains with `DeletionPending` and will be retried.
7. Deleting a file set where a file has `cloud_sync_status = NotSynced` immediately deletes
   the `file_info` record (no cloud action needed).
8. `file_type_migration` `CollectCloudFileSetsStep` correctly identifies synced files using
   the new `cloud_sync_status = Synced` column instead of `file_sync_log`.
9. All existing tests pass after the migration.
10. `PrepareFilesForUploadStep`, `MarkNewFilesForCloudSyncStep`, and `CleanupOrphanedSyncLogsStep` no longer exist.
11. `DeletionCompleted` audit log entries written by `DeleteMarkedFilesStep` are persisted and not automatically cleaned up.

## As Implemented
_(Pending — T19–T26 still open)_
