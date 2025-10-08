CREATE TABLE file_sync_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    file_info_id INTEGER NOT NULL,
    sync_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    status INTEGER NOT NULL,
    message TEXT NOT NULL DEFAULT '',
    cloud_key TEXT NOT NULL,
    FOREIGN KEY (file_info_id) REFERENCES file_info(id)
);

