-- This migration adds the 'is_available' column to the 'file_info' table to indicate whether a file is available or not.
-- Setting to default to 1 (available) for existing records.
ALTER TABLE file_info ADD COLUMN is_available INTEGER NOT NULL DEFAULT 1;
