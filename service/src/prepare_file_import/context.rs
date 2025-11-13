use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use core_types::{FileType, ImportedFile, ReadFile, Sha1Checksum};
use database::{models::FileInfo, repository_manager::RepositoryManager};
use file_import::FileImportOps;

use crate::{
    file_system_ops::FileSystemOps,
    prepare_file_import::model::{ImportFile, ImportFileContent},
};

pub struct PrepareFileImportContext {
    pub repository_manager: Arc<RepositoryManager>,
    pub file_path: PathBuf,
    pub file_type: FileType,
    pub import_metadata: Option<FileImportMetadata>,
    pub existing_files: Vec<FileInfo>,
    pub file_info: HashMap<Sha1Checksum, ReadFile>,
    pub fs_ops: Arc<dyn FileSystemOps>,
    pub file_import_ops: Arc<dyn FileImportOps>,
}

pub struct FileImportMetadata {
    pub file_set_name: Option<String>,
    pub file_set_file_name: Option<String>,
    pub is_zip_archive: bool,
}

impl PrepareFileImportContext {
    pub fn new(
        repository_manager: Arc<RepositoryManager>,
        file_path: &Path,
        file_type: FileType,
        fs_ops: Arc<dyn FileSystemOps>,
        file_import_ops: Arc<dyn FileImportOps>,
    ) -> Self {
        Self {
            repository_manager,
            file_path: file_path.to_path_buf(),
            file_type,
            import_metadata: None,
            existing_files: vec![],
            file_info: HashMap::new(),
            fs_ops,
            file_import_ops,
        }
    }

    pub fn get_imported_file_info(&self) -> ImportFile {
        let import_content = self
            .file_info
            .iter()
            .map(|(sha1, file_info)| {
                let existing_file = self
                    .existing_files
                    .iter()
                    .find(|f| f.sha1_checksum == *sha1);
                let picked = ImportFileContent {
                    file_info: file_info.clone(),
                    is_new: existing_file.is_none(),
                    existing_file: existing_file.map(|f| ImportedFile {
                        original_file_name: file_info.file_name.clone(),
                        archive_file_name: f.archive_file_name.clone(),
                        sha1_checksum: *sha1,
                        file_size: f.file_size,
                    }),
                };

                (*sha1, picked)
            })
            .collect::<HashMap<_, _>>();

        ImportFile {
            path: self.file_path.clone(),
            content: import_content,
            // TODO: maybe have file_set_name and file_set_file_name mandatory in context
            file_set_file_name: self
                .import_metadata
                .as_ref()
                .and_then(|m| m.file_set_file_name.clone())
                .unwrap_or_default(),
            file_set_name: self
                .import_metadata
                .as_ref()
                .and_then(|m| m.file_set_name.clone())
                .unwrap_or_default(),
        }
    }
}
