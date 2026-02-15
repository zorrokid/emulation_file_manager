PRAGMA foreign_keys = ON;

-- Add system_id column to dat_file table
ALTER TABLE dat_file ADD COLUMN system_id INTEGER NOT NULL REFERENCES system(id) ON DELETE CASCADE;

-- Create index for system_id
CREATE INDEX idx_dat_file_system_id ON dat_file(system_id);
