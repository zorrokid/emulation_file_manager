CREATE TABLE release (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    name TEXT NOT NULL
);
 
CREATE TABLE file_info (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    sha1_checksum TEXT NOT NULL,
    file_size INTEGER NOT NULL
);

CREATE TABLE file_set (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    file_name TEXT NOT NULL,
    file_type INTEGER NOT NULL
 );

CREATE TABLE file_set_file_info (
    file_set_id INTEGER NOT NULL,
    file_info_id INTEGER NOT NULL,
    -- same file can have different names in different file sets 
    file_name TEXT NOT NULL,
    PRIMARY KEY (file_set_id, file_info_id),
    FOREIGN KEY (file_set_id) REFERENCES file_set(id),
    FOREIGN KEY (file_info_id) REFERENCES file_info(id)
);

CREATE TABLE release_file_set (
    release_id INTEGER NOT NULL,
    file_set_id INTEGER NOT NULL,
    PRIMARY KEY (release_id, file_set_id),
    FOREIGN KEY (release_id) REFERENCES release(id)
);
 
