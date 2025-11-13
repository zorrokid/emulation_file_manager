use std::{collections::HashMap, path::PathBuf};

use core_types::{ImportedFile, ReadFile, Sha1Checksum};

/// Content of a file to be imported. If there is already an existing file with the same
/// checksum, it will be marked as not new and the existing file info will be provided.
#[derive(Debug)]
pub struct ImportFileContent {
    pub file_info: ReadFile,
    pub is_new: bool,
    pub existing_file: Option<ImportedFile>,
}

/// A file to be imported, containing its path and a mapping of its content by SHA1 checksum. When
/// imported file is an archive, it may contain multiple files inside it. When it's a single file,
/// the content will contain a single entry.
#[derive(Debug)]
pub struct ImportFile {
    pub file_set_name: String,
    pub file_set_file_name: String,
    pub path: PathBuf,
    pub content: HashMap<Sha1Checksum, ImportFileContent>,
}
