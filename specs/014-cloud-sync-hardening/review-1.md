# Review 1 — Cloud Sync Hardening

## Summary

The implementation is solid overall. The session-offset loop algorithm and the tombstone/cloud-deletion split are correct, the pipeline structure is clean, and test coverage is thorough. Three findings require attention: one DRY violation in the repository, one misleading event emission in `DeleteCloudFilesStep`, and two missing doc comments. Tasks T36–T38 added to tasks.md Phase 6.

## Findings

### Major

#### R1 — DRY: identical `FileInfo → CloudSyncableFileInfo` conversion in two repository methods → T36
**File:** `database/src/repository/file_info_repository.rs:L132–138` and `L204–210`

`get_files_pending_upload` and `get_cloud_files_pending_deletion` both end with:
```rust
rows.into_iter()
    .map(CloudSyncableFileInfo::try_from)
    .collect::<Result<Vec<_>, _>>()
    .map_err(|_| Error::DecodeError("...".into()))
```
Five lines, copy-pasted verbatim. The only difference is the error message string. When the conversion logic changes (or a new paginated query is added), both sites must be updated in sync.

**Fix:** Extract a private helper:
```rust
fn to_cloud_syncable(rows: Vec<FileInfo>, context: &str) -> Result<Vec<CloudSyncableFileInfo>, Error> {
    rows.into_iter()
        .map(CloudSyncableFileInfo::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| Error::DecodeError(
            format!("Unexpected null archive_file_name in {context}").into(),
        ))
}
```
Call sites become one line each.

---

### Minor

#### R2 — `FileDeletionCompleted` emitted even when DB cleanup fails → T37
**File:** `service/src/cloud_sync/steps.rs:L344–381`

When the cloud delete succeeds but the audit log write or `delete_file_info` fails, `db_update_success` is set to `false` — but `FileDeletionCompleted` is still emitted unconditionally (lines 373–381). The UI consumer (status bar) increments its deletion counter, telling the user "file deleted successfully". However the `file_info` record is still in `DeletionPending` in the DB. On the next sync session this file will be re-fetched, the cloud delete will fail (object no longer exists), and `FileDeletionFailed` will be emitted for the same file. The user sees contradictory signals.

**Fix:** Only emit `FileDeletionCompleted` when `db_update_success` is true; fall back to `FileDeletionFailed` with a DB-level error message otherwise:
```rust
if file_deletion_result.db_update_success {
    send_progress_event(SyncEvent::FileDeletionCompleted { ... }, &context.progress_tx).await;
} else {
    send_progress_event(SyncEvent::FileDeletionFailed {
        key: cloud_key.clone(),
        error: file_deletion_result.db_error.clone().unwrap_or_default(),
        ...
    }, &context.progress_tx).await;
    // Advance past this file so it isn't immediately re-fetched in this session.
    session_skip += 1;
}
```

#### R3 — `SyncResult` missing struct-level and field-level doc comments → T38
**File:** `service/src/cloud_sync/service.rs:L302–312`

`SyncResult` is a `pub` struct with no `///` doc comment at the struct level. The field `tombstones_cleaned_up` also has no doc comment (unlike `partial_successful_uploads` which has a clear explanation). Project convention requires all public items to have doc comments.

**Fix:**
```rust
/// Summary of a completed cloud sync operation returned by [`CloudStorageSyncService::sync_to_cloud`].
#[derive(Debug)]
pub struct SyncResult {
    pub successful_uploads: usize,
    pub failed_uploads: usize,
    pub successful_deletions: usize,
    pub failed_deletions: usize,
    /// Uploads where the cloud operation succeeded but the DB update failed.
    /// These files exist in cloud storage but remain `NotSynced` in the DB.
    pub partial_successful_uploads: usize,
    /// Tombstone records (DeletionPending, no archive_file_name) deleted from the DB.
    pub tombstones_cleaned_up: usize,
}
```

---

### Spec / Process

#### R4 — `spec.md` Status and `## As Implemented` not updated ✅ resolved
**File:** `specs/014-cloud-sync-hardening/spec.md`

Status is still `Planning` and `## As Implemented` is `_(Pending)_`. These must be filled in before the spec is considered complete. This is a process gate, not a code fix — no task added; update as part of closing Phase 6.

---

## Spec Compliance

- [x] AC1: Upload loop processes all pending files without skipping any ✅
- [x] AC2: Session-offset algorithm prevents getting stuck on a repeatedly-failing file ✅
- [x] AC3: Cloud deletions and tombstone cleanup are split into separate steps ✅
- [x] AC4: `generate_cloud_key` is consolidated in `cloud_storage` crate ✅
- [x] AC5: `SyncStarted` carries accurate total counts before pipeline runs ✅
- [x] AC6: Terminal lifecycle events (`SyncCompleted`, `SyncCancelled`, `SyncFailed`) are always sent ✅
- [x] AC7: Pipeline correctly handles cancellation at each per-file checkpoint ✅

## Verdict

- **Blocking issues**: 0
- **Non-blocking issues**: 3 (R1 Major DRY, R2 Minor event correctness, R3 Minor doc)
- **Ready to merge**: After fixes
