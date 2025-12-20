use std::{collections::HashMap, path::PathBuf, sync::Arc};

use crate::file_import::{
    common_steps::collect_file_info::CollectFileInfoContext, model::FileImportData,
};
use core_types::{ImportedFile, ReadFile, Sha1Checksum};
use database::{models::FileInfo, repository_manager::RepositoryManager};
use file_import::FileImportOps;

use crate::{
    file_import::common_steps::import::FileImportContextOps, file_system_ops::FileSystemOps,
    view_models::Settings,
};

pub struct AddFileToFileSetContext {
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
    pub file_import_ops: Arc<dyn FileImportOps>,
    pub fs_ops: Arc<dyn FileSystemOps>,

    pub file_import_data: FileImportData,
    pub file_path: PathBuf,
    pub existing_files: Vec<FileInfo>,
    pub file_info: HashMap<Sha1Checksum, ReadFile>,
    pub is_zip_archive: Option<bool>,
    pub imported_files: HashMap<Sha1Checksum, ImportedFile>,
}

impl FileImportContextOps for AddFileToFileSetContext {
    fn set_imported_files(&mut self, imported_files: HashMap<Sha1Checksum, ImportedFile>) {
        self.imported_files = imported_files;
    }
    fn file_import_ops(&self) -> &Arc<dyn FileImportOps> {
        &self.file_import_ops
    }
    fn get_file_import_data(&self) -> &FileImportData {
        &self.file_import_data
    }
}

impl CollectFileInfoContext for AddFileToFileSetContext {
    fn is_zip_archive(&self) -> Option<bool> {
        self.is_zip_archive
    }

    fn file_import_ops(&self) -> Arc<dyn FileImportOps> {
        self.file_import_ops.clone()
    }

    fn set_file_info(&mut self, file_info: HashMap<Sha1Checksum, ReadFile>) {
        self.file_info = file_info;
    }

    fn file_path(&self) -> &PathBuf {
        &self.file_path
    }
}
