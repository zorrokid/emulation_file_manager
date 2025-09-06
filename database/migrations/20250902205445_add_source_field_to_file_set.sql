-- Add source field to file_set table
ALTER TABLE file_set ADD COLUMN source TEXT NOT NULL DEFAULT '';