CREATE TABLE file_set_dat_file_link (
    file_set_id INTEGER NOT NULL,
    dat_file_id INTEGER NOT NULL,
    PRIMARY KEY (file_set_id, dat_file_id),
    FOREIGN KEY (file_set_id) REFERENCES file_set(id) ON DELETE CASCADE,
    FOREIGN KEY (dat_file_id) REFERENCES dat_file(id) ON DELETE CASCADE
);
