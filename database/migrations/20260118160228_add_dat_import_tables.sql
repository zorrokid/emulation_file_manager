PRAGMA foreign_keys = ON;

-- Stores No-Intro DAT file metadata
CREATE TABLE dat_file (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    dat_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    version TEXT NOT NULL,
    date TEXT,
    author TEXT NOT NULL,
    homepage TEXT,
    url TEXT,
    subset TEXT,
    imported_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Stores game entries from DAT files
CREATE TABLE dat_game (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    dat_file_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    game_id TEXT,
    description TEXT NOT NULL,
    cloneof TEXT,
    cloneofid TEXT,
    FOREIGN KEY (dat_file_id) REFERENCES dat_file(id) ON DELETE CASCADE
);

-- Stores ROM entries for each game
CREATE TABLE dat_rom (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    dat_game_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    size INTEGER NOT NULL,
    crc TEXT NOT NULL,
    md5 TEXT NOT NULL,
    sha1 TEXT NOT NULL,
    sha256 TEXT,
    status TEXT,
    serial TEXT,
    header TEXT,
    FOREIGN KEY (dat_game_id) REFERENCES dat_game(id) ON DELETE CASCADE
);

-- Create indexes for common queries
CREATE INDEX idx_dat_game_dat_file_id ON dat_game(dat_file_id);
CREATE INDEX idx_dat_rom_dat_game_id ON dat_rom(dat_game_id);
CREATE INDEX idx_dat_rom_sha1 ON dat_rom(sha1);
CREATE INDEX idx_dat_rom_md5 ON dat_rom(md5);
CREATE INDEX idx_dat_rom_crc ON dat_rom(crc);
