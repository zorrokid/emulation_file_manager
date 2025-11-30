-- Add cleanup_temp_files column to document_viewer table
-- This controls whether temporary files should be cleaned up after the viewer is launched
-- Set to false by default for safety (viewers that spawn child processes need files to persist)
ALTER TABLE document_viewer ADD COLUMN cleanup_temp_files BOOLEAN NOT NULL DEFAULT FALSE;
