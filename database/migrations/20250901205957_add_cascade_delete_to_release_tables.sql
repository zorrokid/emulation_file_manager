-- Add CASCADE DELETE to release_system and release_software_title tables

-- Fix release_system table
CREATE TABLE release_system_new (
    release_id INTEGER NOT NULL,
    system_id INTEGER NOT NULL,
    PRIMARY KEY (release_id, system_id),
    FOREIGN KEY (release_id) REFERENCES release(id) ON DELETE CASCADE,
    FOREIGN KEY (system_id) REFERENCES system(id) ON DELETE CASCADE
);

INSERT INTO release_system_new (release_id, system_id)
SELECT release_id, system_id FROM release_system;

DROP TABLE release_system;
ALTER TABLE release_system_new RENAME TO release_system;

-- Fix release_software_title table  
CREATE TABLE release_software_title_new (
    release_id INTEGER NOT NULL,
    software_title_id INTEGER NOT NULL,
    PRIMARY KEY (release_id, software_title_id),
    FOREIGN KEY (release_id) REFERENCES release(id) ON DELETE CASCADE,
    FOREIGN KEY (software_title_id) REFERENCES software_title(id) ON DELETE CASCADE
);

INSERT INTO release_software_title_new (release_id, software_title_id)
SELECT release_id, software_title_id FROM release_software_title;

DROP TABLE release_software_title;
ALTER TABLE release_software_title_new RENAME TO release_software_title;