-- Change is_available DEFAULT from 1 to 0.
-- A file_info record is unavailable until confirmed; DEFAULT 1 was a leftover
-- from before missing-file support was introduced.
-- SQLite does not support ALTER COLUMN to change DEFAULT, so we recreate the table.

PRAGMA foreign_keys = OFF;

CREATE TABLE file_info_new (
    id                INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    sha1_checksum     BLOB    NOT NULL,
    file_size         INTEGER NOT NULL,
    archive_file_name TEXT,
    file_type         INTEGER,
    is_available      INTEGER NOT NULL DEFAULT 0
);

INSERT INTO file_info_new (id, sha1_checksum, file_size, archive_file_name, file_type, is_available)
    SELECT id, sha1_checksum, file_size, archive_file_name, file_type, is_available
    FROM file_info;

DROP TABLE file_info;
ALTER TABLE file_info_new RENAME TO file_info;

PRAGMA foreign_keys = ON;
