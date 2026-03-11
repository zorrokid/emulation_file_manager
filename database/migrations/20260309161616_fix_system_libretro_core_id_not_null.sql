CREATE TABLE system_libretro_core_new (
    id        INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    system_id INTEGER NOT NULL REFERENCES system(id) ON DELETE CASCADE,
    core_name TEXT NOT NULL,
    UNIQUE(system_id, core_name)
);

INSERT INTO system_libretro_core_new SELECT id, system_id, core_name FROM system_libretro_core;

DROP TABLE system_libretro_core;

ALTER TABLE system_libretro_core_new RENAME TO system_libretro_core;
