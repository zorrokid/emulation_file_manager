use std::{collections::HashMap, path::PathBuf};

use core_types::{FileSize, FileType, Sha1Checksum};

/// Content of a file to be imported. If there is already an existing file with the same
/// checksum, the existing file info will be provided.
#[derive(Debug)]
pub struct ImportFileContent {
    pub file_name: String,
    pub sha1_checksum: Sha1Checksum,
    pub file_size: FileSize,

    pub existing_file_info_id: Option<i64>,
    pub existing_archive_file_name: Option<String>,
}

/// A file to be imported, containing its path and a mapping of its content by SHA1 checksum. When
/// imported file is an archive, it may contain multiple files inside it. When it's a single file,
/// the content will contain a single entry.
#[derive(Debug)]
pub struct ImportFile {
    pub file_set_name: String,      // TODO: is this needed?
    pub file_set_file_name: String, // TODO: is this needed?
    pub file_type: FileType,        // TODO: is this needed?
    pub path: PathBuf,
    pub content: HashMap<Sha1Checksum, ImportFileContent>,
}

#[derive(Debug)]
pub struct FileImportModel {
    pub path: PathBuf,
    pub content: HashMap<Sha1Checksum, ImportFileContent>,
}

#[derive(Debug)]
pub struct FileSetImportModel {
    pub import_files: Vec<FileImportModel>,
    // TODO: maybe just remove other files than selected
    // from import_files, so this can be removed?
    pub selected_files: Vec<Sha1Checksum>,
    pub system_ids: Vec<i64>,
    pub source: String,
    pub file_set_name: String,
    pub file_set_file_name: String,
    pub file_type: FileType,
}
