PRAGMA foreign_keys = OFF;

ALTER TABLE file_sync_log RENAME TO file_sync_log_old;

CREATE TABLE file_sync_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    file_info_id INTEGER NOT NULL,
    sync_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    status INTEGER NOT NULL,
    message TEXT NOT NULL DEFAULT '',
    cloud_key TEXT NOT NULL
);

INSERT INTO file_sync_log (id, file_info_id, sync_time, status, message, cloud_key)
SELECT id, file_info_id, sync_time, status, message, cloud_key FROM file_sync_log_old;

DROP TABLE file_sync_log_old;

PRAGMA foreign_keys = ON;
