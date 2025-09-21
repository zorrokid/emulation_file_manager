-- Add file_type field to file_info table and populate from file_set
ALTER TABLE file_info ADD COLUMN file_type INTEGER;

-- Populate file_type from file_set table via file_set_file_info relationship
-- This handles the case where a file_info might be linked to multiple file_sets
-- We'll use the first file_type found for each file_info
UPDATE file_info
SET file_type = (
    SELECT fs.file_type
    FROM file_set fs
    INNER JOIN file_set_file_info fsfi ON fs.id = fsfi.file_set_id
    WHERE fsfi.file_info_id = file_info.id
    LIMIT 1
)
WHERE EXISTS (
    SELECT 1
    FROM file_set_file_info fsfi
    WHERE fsfi.file_info_id = file_info.id
);