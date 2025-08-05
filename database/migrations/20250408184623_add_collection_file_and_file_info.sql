PRAGMA foreign_keys = ON;

CREATE TABLE system (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    name TEXT NOT NULL
);

CREATE TABLE emulator (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    name TEXT NOT NULL,
    executable TEXT NOT NULL,
    extract_files INTEGER NOT NULL
);

CREATE TABLE emulator_system (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    emulator_id INTEGER NOT NULL,
    system_id INTEGER NOT NULL,
    arguments TEXT,
    FOREIGN key (emulator_id) REFERENCES emulator(id) ON DELETE CASCADE,
    FOREIGN KEY (system_id) REFERENCES system(id) ON DELETE CASCADE
);

CREATE TABLE franchise (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    name TEXT NOT NULL
);

CREATE TABLE software_title (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    name TEXT NOT NULL,
    franchise_id INTEGER,
    FOREIGN KEY (franchise_id) REFERENCES franchise(id)
);

CREATE TABLE release (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    name TEXT NOT NULL
);

CREATE TABLE release_system (
    release_id INTEGER NOT NULL,
    system_id INTEGER NOT NULL,
    PRIMARY KEY (release_id, system_id),
    FOREIGN KEY (release_id) REFERENCES release(id),
    FOREIGN KEY (system_id) REFERENCES system(id)
);

CREATE TABLE release_software_title (
    release_id INTEGER NOT NULL,
    software_title_id INTEGER NOT NULL,
    PRIMARY KEY (release_id, software_title_id),
    FOREIGN KEY (release_id) REFERENCES release(id),
    FOREIGN KEY (software_title_id) REFERENCES software_title(id)
);

CREATE TABLE file_info (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    sha1_checksum BLOB NOT NULL,
    file_size INTEGER NOT NULL,
    archive_file_name TEXT NOT NULL
);

CREATE TABLE file_info_system (
    file_info_id INTEGER NOT NULL,
    system_id INTEGER NOT NULL,
    PRIMARY KEY (file_info_id, system_id),
    FOREIGN KEY (file_info_id) REFERENCES file_info(id),
    FOREIGN KEY (system_id) REFERENCES system(id)
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

CREATE TABLE setting (
    key TEXT NOT NULL PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE document_viewer (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    name TEXT NOT NULL,
    executable TEXT NOT NULL,
    document_type INTEGER NOT NULL
);
