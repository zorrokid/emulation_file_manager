CREATE TABLE file_sync_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    file_info_id INTEGER NOT NULL,
    sync_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    status TEXT NOT NULL,
    message TEXT,
    FOREIGN KEY (file_info_id) REFERENCES file_info(id)
);


