use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use core_types::{FileType, ReadFile, Sha1Checksum};
use database::{models::FileInfo, repository_manager::RepositoryManager};
use file_metadata::file_metadata_ops::FileMetadataOps;

use crate::{
    file_import::{
        common_steps::collect_file_info::CollectFileInfoContext,
        model::{FileImportMetadata, FileImportSource, ImportFileContent},
    },
    file_system_ops::FileSystemOps,
};

pub struct PrepareFileImportContext {
    pub repository_manager: Arc<RepositoryManager>,
    pub file_path: PathBuf,
    pub file_type: FileType,
    pub import_metadata: Option<FileImportMetadata>,
    pub existing_files: Vec<FileInfo>,
    pub file_info: HashMap<Sha1Checksum, ReadFile>,
    pub fs_ops: Arc<dyn FileSystemOps>,
    pub file_metadata_ops: Arc<dyn FileMetadataOps>,
}

impl PrepareFileImportContext {
    pub fn new(
        repository_manager: Arc<RepositoryManager>,
        file_path: &Path,
        file_type: FileType,
        fs_ops: Arc<dyn FileSystemOps>,
        file_metadata_ops: Arc<dyn FileMetadataOps>,
    ) -> Self {
        Self {
            repository_manager,
            file_path: file_path.to_path_buf(),
            file_type,
            import_metadata: None,
            existing_files: vec![],
            file_info: HashMap::new(),
            fs_ops,
            file_metadata_ops,
        }
    }

    pub fn get_imported_file_info(&self) -> FileImportSource {
        let import_content = self
            .file_info
            .iter()
            .map(|(sha1, file_info)| {
                let file = ImportFileContent {
                    file_name: file_info.file_name.clone(),
                    sha1_checksum: *sha1,
                    file_size: file_info.file_size,
                };

                (*sha1, file)
            })
            .collect::<HashMap<_, _>>();

        FileImportSource {
            path: self.file_path.clone(),
            content: import_content,
        }
    }
}

impl CollectFileInfoContext for PrepareFileImportContext {
    fn file_metadata_ops(&self) -> Arc<dyn FileMetadataOps> {
        self.file_metadata_ops.clone()
    }

    fn set_file_info(&mut self, file_info: HashMap<Sha1Checksum, ReadFile>) {
        self.file_info = file_info;
    }

    fn file_path(&self) -> &PathBuf {
        &self.file_path
    }

    fn fs_ops(&self) -> Arc<dyn FileSystemOps> {
        self.fs_ops.clone()
    }
}
