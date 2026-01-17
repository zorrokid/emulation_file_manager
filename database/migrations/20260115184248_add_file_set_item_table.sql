CREATE TABLE file_set_item_type (
    file_set_id INTEGER NOT NULL,
    item_type INTEGER NOT NULL,
    PRIMARY KEY (file_set_id, item_type),
    FOREIGN KEY (file_set_id) REFERENCES file_set(id) ON DELETE CASCADE
);
