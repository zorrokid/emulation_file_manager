use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use core_types::ReadFile;
use file_metadata::{FileMetadataError, FileMetadataReader, MockFileMetadataReader};

pub fn create_mock_reader_factory(
    per_path: HashMap<PathBuf, Vec<ReadFile>>,
    failing_paths: Vec<PathBuf>,
) -> impl Fn(&Path) -> Result<Box<dyn FileMetadataReader>, FileMetadataError> + Send + Sync {
    move |path: &Path| {
        if failing_paths.contains(&path.to_path_buf()) {
            return Err(FileMetadataError::GeneralError {
                path: path.to_path_buf(),
                message: "Simulated failure for this path".to_string(),
            });
        }

        let metadata =
            per_path
                .get(path)
                .cloned()
                .ok_or_else(|| FileMetadataError::GeneralError {
                    path: path.to_path_buf(),
                    message: "No mock metadata for this path".to_string(),
                })?;

        let mock_reader = MockFileMetadataReader { metadata };
        Ok(Box::new(mock_reader))
    }
}
