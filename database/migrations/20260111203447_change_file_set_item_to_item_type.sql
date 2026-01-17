-- Drop the old file_set_item table
DROP TABLE file_set_item;

-- Recreate file_set_item with item_type instead of item_id
CREATE TABLE file_set_item (
    file_set_id INTEGER NOT NULL,
    item_type INTEGER NOT NULL,
    PRIMARY KEY (file_set_id, item_type),
    FOREIGN KEY (file_set_id) REFERENCES file_set(id) ON DELETE CASCADE
);
