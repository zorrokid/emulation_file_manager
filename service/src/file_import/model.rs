use std::{collections::HashMap, path::PathBuf};

use core_types::{FileSize, FileType, Sha1Checksum};

#[derive(Debug, Clone)]
pub struct FileImportMetadata {
    pub file_set_name: String,
    pub file_set_file_name: String,
    pub is_zip_archive: bool,
}

/// Content of a file to be imported. If there is already an existing file with the same
/// checksum, the existing file info will be provided.
#[derive(Debug, Clone)]
pub struct ImportFileContent {
    pub file_name: String,
    pub sha1_checksum: Sha1Checksum,
    pub file_size: FileSize,

    pub existing_file_info_id: Option<i64>,
    pub existing_archive_file_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FileImportModel {
    /// Path to the file to be imported (e.g., zip archive)
    pub path: PathBuf,
    /// Mapping of SHA1 checksum to file content info
    pub content: HashMap<Sha1Checksum, ImportFileContent>,
}

#[derive(Debug)]
pub struct FileImportPrepareResult {
    pub import_model: FileImportModel,
    pub import_metadata: FileImportMetadata,
}

#[derive(Debug)]
pub struct FileSetImportModel {
    pub import_files: Vec<FileImportModel>,
    pub selected_files: Vec<Sha1Checksum>,
    pub system_ids: Vec<i64>,
    pub source: String,
    pub file_set_name: String,
    pub file_set_file_name: String,
    pub file_type: FileType,
}
