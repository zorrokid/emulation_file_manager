use std::{collections::HashMap, sync::Arc};

use async_std::channel::Sender;
use cloud_storage::{CloudStorageOps, DownloadEvent};
use database::{
    models::{FileInfo, FileSet, FileSetFileInfo},
    repository_manager::RepositoryManager,
};
use file_export::OutputFile;

use crate::{
    file_system_ops::FileSystemOps, pipeline::cloud_connection::CloudConnectionContext,
    settings_service::SettingsService, view_models::Settings,
};

pub struct DownloadContext<F: FileSystemOps> {
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
    pub settings_service: Arc<SettingsService>,
    pub progress_tx: Sender<DownloadEvent>,

    pub fs_ops: Arc<F>,

    // Lazy initialized by ConnectToCloudStep
    // Need to use dyn because CloudStorageOps is a trait
    // and trait is used so that we can have mock implementations for testing
    // and different cloud storage providers.
    pub cloud_ops: Option<Arc<dyn CloudStorageOps>>,

    pub file_set_id: i64,
    pub extract_files: bool,
    pub file_set: Option<FileSet>,
    pub files_in_set: Vec<FileSetFileInfo>,
    pub files_to_download: Vec<FileInfo>,
    pub file_download_results: Vec<FileDownloadResult>,
    pub file_output_mapping: HashMap<String, OutputFile>,
}

#[derive(Debug, Clone)]
pub struct FileDownloadResult {
    pub file_info_id: i64,
    pub cloud_key: String,

    pub cloud_operation_success: bool,
    pub file_write_success: bool,

    pub cloud_error: Option<String>,
    pub file_io_error: Option<String>,
}

impl<F: FileSystemOps> DownloadContext<F> {
    pub fn new(
        repository_manager: Arc<RepositoryManager>,
        settings: Arc<Settings>,
        settings_service: Arc<SettingsService>,
        progress_tx: Sender<DownloadEvent>,
        file_set_id: i64,
        extract_files: bool,
        cloud_ops: Option<Arc<dyn CloudStorageOps>>,
        fs_ops: Arc<F>,
    ) -> Self {
        Self {
            repository_manager,
            settings,
            settings_service,
            progress_tx,
            cloud_ops,
            file_set_id,
            extract_files,
            file_set: None,
            files_in_set: vec![],
            files_to_download: vec![],
            file_download_results: vec![],
            file_output_mapping: HashMap::new(),
            fs_ops,
        }
    }

    pub fn successful_downloads(&self) -> usize {
        self.file_download_results
            .iter()
            .filter(|result| result.cloud_operation_success && result.file_write_success)
            .count()
    }

    pub fn failed_downloads(&self) -> usize {
        self.file_download_results
            .iter()
            .filter(|result| !result.cloud_operation_success || !result.file_write_success)
            .count()
    }
}

impl<F: FileSystemOps> CloudConnectionContext for DownloadContext<F> {
    fn settings(&self) -> &Arc<Settings> {
        &self.settings
    }

    fn settings_service(&self) -> &Arc<SettingsService> {
        &self.settings_service
    }

    fn cloud_ops_mut(&mut self) -> &mut Option<Arc<dyn CloudStorageOps>> {
        &mut self.cloud_ops
    }

    fn should_connect(&self) -> bool {
        !self.files_to_download.is_empty()
    }
}
