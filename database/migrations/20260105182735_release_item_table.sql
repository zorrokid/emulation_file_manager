CREATE TABLE release_item (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    release_id INTEGER NOT NULL,
    item_type INTEGER NOT NULL,
    description TEXT,
    FOREIGN KEY (release_id) REFERENCES release(id) ON DELETE CASCADE
);
