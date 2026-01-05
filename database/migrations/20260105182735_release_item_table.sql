CREATE TABLE release_item (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    release_id INTEGER NOT NULL,
    item_type INTEGER NOT NULL,
    notes TEXT,
    FOREIGN KEY (release_id) REFERENCES release(id) ON DELETE CASCADE
);

CREATE TABLE file_set_item (
    file_set_id INTEGER NOT NULL,
    item_id INTEGER NOT NULL,
    PRIMARY KEY (file_set_id, item_id),
    FOREIGN KEY (file_set_id) REFERENCES file_set(id) ON DELETE CASCADE,
    FOREIGN KEY (item_id) REFERENCES release_item(id) ON DELETE CASCADE
);

