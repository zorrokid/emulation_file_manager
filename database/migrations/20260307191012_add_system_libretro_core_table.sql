CREATE TABLE system_libretro_core (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    system_id INTEGER NOT NULL REFERENCES system(id) ON DELETE CASCADE,
    core_name TEXT NOT NULL,
    UNIQUE(system_id, core_name)
);

