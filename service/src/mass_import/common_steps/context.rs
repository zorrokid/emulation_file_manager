use std::{collections::HashMap, path::PathBuf, sync::Arc};

use core_types::ReadFile;
use database::repository_manager::RepositoryManager;
use file_metadata::SendReaderFactoryFn;
use flume::Sender;

use crate::{
    error::Error,
    file_import::{file_import_service_ops::FileImportServiceOps, model::FileSetImportModel},
    file_system_ops::FileSystemOps,
    mass_import::models::{FileSetImportResult, MassImportSyncEvent},
};

#[derive(Debug)]
pub struct MassImportDeps {
    pub repository_manager: Arc<RepositoryManager>,
}

#[derive(Default, Debug)]
pub struct CommonMassImportState {
    pub read_ok_files: Vec<std::path::PathBuf>,
    pub read_failed_files: Vec<std::path::PathBuf>,
    pub dir_scan_errors: Vec<crate::error::Error>,
    pub file_metadata: HashMap<PathBuf, Vec<ReadFile>>,
    pub import_results: Vec<FileSetImportResult>,
}

pub trait MassImportContextOps {
    fn common_state(&self) -> &CommonMassImportState;
    fn common_state_mut(&mut self) -> &mut CommonMassImportState;
    fn reader_factory_fn(&self) -> Arc<SendReaderFactoryFn>;
    fn fs_ops(&self) -> Arc<dyn FileSystemOps>;
    fn source_path(&self) -> &std::path::Path;
    fn read_ok_files_mut(&mut self) -> &mut Vec<PathBuf> {
        &mut self.common_state_mut().read_ok_files
    }
    fn read_ok_files(&self) -> &Vec<PathBuf> {
        &self.common_state().read_ok_files
    }
    fn read_failed_files(&self) -> &Vec<PathBuf> {
        &self.common_state().read_failed_files
    }
    fn read_failed_files_mut(&mut self) -> &mut Vec<PathBuf> {
        &mut self.common_state_mut().read_failed_files
    }
    fn dir_scan_errors(&mut self) -> &mut Vec<Error> {
        &mut self.common_state_mut().dir_scan_errors
    }
    fn get_non_failed_files(&self) -> Vec<PathBuf> {
        let mut non_failed_files = self.read_ok_files().clone();
        non_failed_files.retain(|file| !self.read_failed_files().contains(file));
        non_failed_files
    }
    fn file_metadata(&mut self) -> &mut HashMap<PathBuf, Vec<ReadFile>> {
        &mut self.common_state_mut().file_metadata
    }
    fn get_import_file_sets(&self) -> Vec<FileSetImportModel>;
    fn import_service_ops(&self) -> Arc<dyn FileImportServiceOps>;
    fn import_results(&mut self) -> &mut Vec<FileSetImportResult> {
        &mut self.common_state_mut().import_results
    }
    fn progress_tx(&self) -> &Option<Sender<MassImportSyncEvent>>;
    fn can_import_file_sets(&self) -> bool;
}
