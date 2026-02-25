use std::{collections::HashMap, path::PathBuf, sync::Arc};

use async_std::channel::Sender;
use core_types::ReadFile;
use database::repository_manager::RepositoryManager;

use crate::{
    error::Error,
    file_import::{file_import_service_ops::FileImportServiceOps, model::FileSetImportModel},
    file_system_ops::FileSystemOps,
    mass_import::models::{FileSetImportResult, MassImportSyncEvent},
};

/// Type alias for a Send-able (can be safely transferred between threads)
/// metadata reader factory function.
///
/// Send-able:
/// The + Send + Sync bounds ensure that the closure can be shared and used across threads.
pub type SendReaderFactoryFn = dyn Fn(
        &std::path::Path,
    ) -> Result<Box<dyn file_metadata::FileMetadataReader>, file_metadata::FileMetadataError>
    + Send
    + Sync;

#[derive(Debug)]
pub struct MassImportDeps {
    pub repository_manager: Arc<RepositoryManager>,
}

pub trait MassImportContextOps {
    fn reader_factory_fn(&self) -> Arc<SendReaderFactoryFn>;
    fn fs_ops(&self) -> Arc<dyn FileSystemOps>;
    fn source_path(&self) -> &std::path::Path;
    fn read_ok_files_mut(&mut self) -> &mut Vec<PathBuf>;
    fn read_ok_files(&self) -> &Vec<PathBuf>;
    fn read_failed_files(&self) -> &Vec<PathBuf>;
    fn read_failed_files_mut(&mut self) -> &mut Vec<PathBuf>;
    fn dir_scan_errors(&mut self) -> &mut Vec<Error>;
    fn get_non_failed_files(&self) -> Vec<PathBuf> {
        let mut non_failed_files = self.read_ok_files().clone();
        non_failed_files.retain(|file| !self.read_failed_files().contains(file));
        non_failed_files
    }
    fn file_metadata(&mut self) -> &mut HashMap<PathBuf, Vec<ReadFile>>;
    fn get_import_file_sets(&self) -> Vec<FileSetImportModel>;
    fn import_service_ops(&self) -> Arc<dyn FileImportServiceOps>;
    fn import_results(&mut self) -> &mut Vec<FileSetImportResult>;
    fn progress_tx(&self) -> &Option<Sender<MassImportSyncEvent>>;
}
