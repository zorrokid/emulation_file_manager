PRAGMA foreign_keys = OFF;

ALTER TABLE file_set_file_info RENAME TO file_set_file_info_old;

CREATE TABLE file_set_file_info (
    file_set_id INTEGER NOT NULL,
    file_info_id INTEGER NOT NULL,
    -- same file can have different names in different file sets
    file_name TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (file_set_id, file_info_id),
    FOREIGN KEY (file_set_id) REFERENCES file_set(id) ON DELETE CASCADE,
    FOREIGN KEY (file_info_id) REFERENCES file_info(id) ON DELETE CASCADE
);

INSERT INTO file_set_file_info (file_set_id, file_info_id, file_name, sort_order)
SELECT file_set_id, file_info_id, file_name, sort_order FROM file_set_file_info_old;

DROP TABLE file_set_file_info_old;

PRAGMA foreign_keys = ON;
