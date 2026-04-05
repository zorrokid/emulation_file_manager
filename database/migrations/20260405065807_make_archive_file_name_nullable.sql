-- Make archive_file_name nullable in file_info.
-- SQLite does not support DROP NOT NULL via ALTER COLUMN, so we use the
-- standard workaround: create new table → copy data → drop old → rename.
PRAGMA foreign_keys = OFF;

CREATE TABLE file_info_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    sha1_checksum BLOB NOT NULL,
    file_size INTEGER NOT NULL,
    archive_file_name TEXT,
    file_type INTEGER,  -- was already nullable (added via ALTER TABLE ADD COLUMN in a prior migration)
    is_available INTEGER NOT NULL DEFAULT 1
);

INSERT INTO file_info_new (id, sha1_checksum, file_size, archive_file_name, file_type, is_available)
SELECT
    id,
    sha1_checksum,
    file_size,
    NULLIF(archive_file_name, ''),
    file_type,
    is_available
FROM file_info;

DROP TABLE file_info;

ALTER TABLE file_info_new RENAME TO file_info;

PRAGMA foreign_keys = ON;
