-- Remove the is_available column from file_info.
-- Availability is now derived from archive_file_name: a record is available iff
-- archive_file_name IS NOT NULL. The is_available column was always kept in sync
-- with this condition (see add_file_info and update_is_available), so removing it
-- is a pure structural simplification with no data loss.
--
-- SQLite does not support DROP COLUMN on older versions, so we use the standard
-- recreate-table pattern.

PRAGMA foreign_keys = OFF;

CREATE TABLE file_info_new (
    id                INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    sha1_checksum     BLOB    NOT NULL,
    file_size         INTEGER NOT NULL,
    archive_file_name TEXT,
    file_type         INTEGER,
    cloud_sync_status INTEGER NOT NULL DEFAULT 0
);

INSERT INTO file_info_new (id, sha1_checksum, file_size, archive_file_name, file_type, cloud_sync_status)
    SELECT id, sha1_checksum, file_size, archive_file_name, file_type, cloud_sync_status
    FROM file_info;

DROP TABLE file_info;
ALTER TABLE file_info_new RENAME TO file_info;

PRAGMA foreign_keys = ON;
