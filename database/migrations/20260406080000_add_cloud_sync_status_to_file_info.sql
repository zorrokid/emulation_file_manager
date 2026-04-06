-- Add cloud_sync_status to file_info as the source of truth for cloud sync state.
-- Values: 0 = NotSynced, 1 = Synced, 2 = DeletionPending
ALTER TABLE file_info ADD COLUMN cloud_sync_status INTEGER NOT NULL DEFAULT 0;

-- Migrate existing state from file_sync_log.
-- For each file_info, set status based on the most recent log entry:
--   UploadCompleted (2)                          → Synced (1)
--   DeletionPending/DeletionInProgress/
--   DeletionCompleted/DeletionFailed (4,5,6,7)   → DeletionPending (2)
--   anything else (or no log)                    → NotSynced (0, already the default)
UPDATE file_info
SET cloud_sync_status = 1  -- Synced
WHERE id IN (
    SELECT log.file_info_id
    FROM file_sync_log log
    INNER JOIN (
        SELECT file_info_id, MAX(id) AS max_id
        FROM file_sync_log
        GROUP BY file_info_id
    ) latest ON log.file_info_id = latest.file_info_id AND log.id = latest.max_id
    WHERE log.status = 2  -- UploadCompleted
);

UPDATE file_info
SET cloud_sync_status = 2  -- DeletionPending
WHERE id IN (
    SELECT log.file_info_id
    FROM file_sync_log log
    INNER JOIN (
        SELECT file_info_id, MAX(id) AS max_id
        FROM file_sync_log
        GROUP BY file_info_id
    ) latest ON log.file_info_id = latest.file_info_id AND log.id = latest.max_id
    WHERE log.status IN (4, 5, 6, 7)  -- DeletionPending/InProgress/Completed/Failed
-- Note: DeletionCompleted (6) maps to DeletionPending (2) here. This is safe because
-- the old INNER JOIN bug in DeleteMarkedFilesStep meant cloud deletion never actually
-- executed, so no file_info with a DeletionCompleted log entry was ever truly deleted
-- from the cloud. Mapping to DeletionPending ensures these files will be retried.
);
