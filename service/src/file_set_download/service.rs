use std::sync::Arc;

use async_std::channel::Sender;
use cloud_storage::events::DownloadEvent;
use database::repository_manager::RepositoryManager;
use file_export::file_export_ops::DefaultFileExportOps;

use crate::{
    file_set_download::context::DownloadContext,
    file_system_ops::{FileSystemOps, StdFileSystemOps},
    pipeline::generic_pipeline::Pipeline,
    settings_service::SettingsService,
    view_models::Settings,
};

#[derive(Debug)]
pub struct DownloadService<F: FileSystemOps = StdFileSystemOps> {
    repository_manager: Arc<RepositoryManager>,
    settings: Arc<Settings>,
    settings_service: Arc<SettingsService>,
    fs_ops: Arc<F>,
}

impl DownloadService<StdFileSystemOps> {
    pub fn new(
        repository_manager: Arc<RepositoryManager>,
        settings: Arc<Settings>,
        settings_service: Arc<SettingsService>,
    ) -> Self {
        Self::new_with_fs_ops(
            repository_manager,
            settings,
            settings_service,
            Arc::new(StdFileSystemOps),
        )
    }
}

impl<F: FileSystemOps + 'static> DownloadService<F> {
    pub fn new_with_fs_ops(
        repository_manager: Arc<RepositoryManager>,
        settings: Arc<Settings>,
        settings_service: Arc<SettingsService>,
        fs_ops: Arc<F>,
    ) -> Self {
        Self {
            repository_manager,
            settings,
            settings_service,
            fs_ops,
        }
    }

    pub async fn download_file_set(
        &self,
        file_set_id: i64,
        extract_files: bool,
        progress_tx: Sender<DownloadEvent>,
    ) -> Result<DownloadResult, crate::error::Error> {
        let mut context = DownloadContext::new(
            self.repository_manager.clone(),
            self.settings.clone(),
            self.settings_service.clone(),
            progress_tx,
            file_set_id,
            extract_files,
            None, // this will be initialized in the pipeline
            self.fs_ops.clone(),
            Arc::new(DefaultFileExportOps),
        );
        let pipeline = Pipeline::<DownloadContext<F>>::new();
        pipeline.execute(&mut context).await?;
        let successful_downloads = context.successful_downloads();
        let failed_downloads = context.failed_downloads();
        Ok(DownloadResult {
            successful_downloads,
            failed_downloads,
        })
    }
}

#[derive(Debug)]
pub struct DownloadResult {
    pub successful_downloads: usize,
    pub failed_downloads: usize,
}
