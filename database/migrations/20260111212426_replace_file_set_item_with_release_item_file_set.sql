-- Drop file_set_item table
DROP TABLE file_set_item;

-- Create release_item_file_set table for linking multiple file_sets to release_item
CREATE TABLE release_item_file_set (
    release_item_id INTEGER NOT NULL,
    file_set_id INTEGER NOT NULL,
    PRIMARY KEY (release_item_id, file_set_id),
    FOREIGN KEY (release_item_id) REFERENCES release_item(id) ON DELETE CASCADE,
    FOREIGN KEY (file_set_id) REFERENCES file_set(id) ON DELETE CASCADE
);
