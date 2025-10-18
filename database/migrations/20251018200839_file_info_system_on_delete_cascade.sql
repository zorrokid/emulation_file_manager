PRAGMA foreign_keys = OFF;

ALTER TABLE file_info_system RENAME TO file_info_system_old;

CREATE TABLE file_info_system (
    file_info_id INTEGER NOT NULL,
    system_id INTEGER NOT NULL,
    PRIMARY KEY (file_info_id, system_id),
    FOREIGN KEY (file_info_id) REFERENCES file_info(id) ON DELETE CASCADE,
    FOREIGN KEY (system_id) REFERENCES system(id)
);

INSERT INTO file_info_system (file_info_id, system_id)
SELECT file_info_id, system_id FROM file_info_system_old;

DROP TABLE file_info_system_old;

PRAGMA foreign_keys = ON;
