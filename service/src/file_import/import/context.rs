use std::{collections::HashMap, sync::Arc};

use core_types::{FileType, ImportedFile, Sha1Checksum};
use database::repository_manager::RepositoryManager;

use crate::{file_import::model::FileImportModel, view_models::Settings};

pub struct FileImportContext {
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
    pub selected_files: Vec<Sha1Checksum>,
    pub file_type: FileType,
    // It is possible to create a file set from multiple sets of import files.
    // From this collection of files, the ones that are in selected_files and are new files, will be imported.
    // If the files are existing files, they will be just linked to the file set.
    // When files are imported, their archive file names will be populated here.
    pub import_files: Vec<FileImportModel>,
    pub system_ids: Vec<i64>,
    pub source: String,
    // File set nama and file name the will be created from the set of import files.
    pub file_set_name: String,
    pub file_set_file_name: String,
    pub imported_files: HashMap<Sha1Checksum, ImportedFile>,
    pub file_set_id: Option<i64>,
}
