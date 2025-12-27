use std::{collections::HashMap, sync::Arc};

use async_std::channel::Sender;
use cloud_storage::CloudStorageOps;
use core_types::events::DownloadEvent;
use database::{
    models::{FileInfo, FileSet, FileSetFileInfo},
    repository_manager::RepositoryManager,
};
use file_export::{OutputFile, file_export_ops::FileExportOps};
use thumbnails::{ThumbnailOps, ThumbnailPathMap};

use crate::{
    file_system_ops::FileSystemOps,
    pipeline::{
        cloud_connection::CloudConnectionContext, 
    },
    settings_service::SettingsService,
    view_models::Settings,
};

// TODO: FileSystemOps generic parameter might not be needed here, use dyn instead?
pub struct DownloadContext {
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
    pub settings_service: Arc<SettingsService>,
    pub progress_tx: Option<Sender<DownloadEvent>>,

    pub fs_ops: Arc<dyn FileSystemOps>,
    pub export_ops: Arc<dyn FileExportOps>,
    pub thumbnail_generator: Arc<dyn ThumbnailOps>,

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
    /// Depending if extract_files is true or false, this maps to either
    /// the extracted files or contents of the output zip-file.
    pub file_output_mapping: HashMap<String, OutputFile>,
    pub thumbnail_path_map: ThumbnailPathMap,
    pub output_file_names: Vec<String>,
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

pub struct DownloadContextSettings {
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
    pub settings_service: Arc<SettingsService>,
    pub progress_tx: Option<Sender<DownloadEvent>>,

    pub file_set_id: i64,
    pub extract_files: bool,

    pub cloud_ops: Option<Arc<dyn CloudStorageOps>>,
    pub fs_ops: Arc<dyn FileSystemOps>,
    pub export_ops: Arc<dyn FileExportOps>,
    pub thumbnail_generator: Arc<dyn ThumbnailOps>,
}

impl DownloadContext {
    pub fn new(settings: DownloadContextSettings) -> Self {
        Self {
            repository_manager: settings.repository_manager,
            settings: settings.settings,
            settings_service: settings.settings_service,
            progress_tx: settings.progress_tx,
            cloud_ops: settings.cloud_ops,
            file_set_id: settings.file_set_id,
            extract_files: settings.extract_files,
            file_set: None,
            files_in_set: vec![],
            files_to_download: vec![],
            file_download_results: vec![],
            file_output_mapping: HashMap::new(),
            fs_ops: settings.fs_ops,
            export_ops: settings.export_ops,
            thumbnail_generator: settings.thumbnail_generator,
            thumbnail_path_map: ThumbnailPathMap::new(),
            output_file_names: vec![],
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

impl CloudConnectionContext for DownloadContext {
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
