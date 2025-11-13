use std::sync::Arc;

use async_std::channel::Sender;
use cloud_storage::events::DownloadEvent;
use database::repository_manager::RepositoryManager;
use file_export::file_export_ops::DefaultFileExportOps;
use thumbnails::{ThumbnailGenerator, ThumbnailPathMap};

use crate::{
    error::Error,
    file_set_download::context::{DownloadContext, DownloadContextSettings},
    file_system_ops::{FileSystemOps, StdFileSystemOps},
    pipeline::generic_pipeline::Pipeline,
    settings_service::SettingsService,
    view_models::Settings,
};

pub struct DownloadService {
    repository_manager: Arc<RepositoryManager>,
    settings: Arc<Settings>,
    settings_service: Arc<SettingsService>,
    fs_ops: Arc<dyn FileSystemOps>,
}

impl std::fmt::Debug for DownloadService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DownloadService").finish_non_exhaustive()
    }
}

impl DownloadService {
    pub fn new(repository_manager: Arc<RepositoryManager>, settings: Arc<Settings>) -> Self {
        let settings_service = Arc::new(SettingsService::new(repository_manager.clone()));
        Self::new_with_fs_ops(
            repository_manager,
            settings,
            settings_service,
            Arc::new(StdFileSystemOps),
        )
    }

    pub fn new_with_fs_ops(
        repository_manager: Arc<RepositoryManager>,
        settings: Arc<Settings>,
        settings_service: Arc<SettingsService>,
        fs_ops: Arc<dyn FileSystemOps>,
    ) -> Self {
        Self {
            repository_manager,
            settings,
            settings_service,
            fs_ops,
        }
    }

    #[tracing::instrument(skip(self, progress_tx), fields(file_set_id, extract_files), err)]
    pub async fn download_file_set(
        &self,
        file_set_id: i64,
        extract_files: bool,
        progress_tx: Option<Sender<DownloadEvent>>,
    ) -> Result<DownloadResult, Error> {
        tracing::info!("Starting file set download");

        let settings = DownloadContextSettings {
            repository_manager: self.repository_manager.clone(),
            settings: self.settings.clone(),
            settings_service: self.settings_service.clone(),
            progress_tx: progress_tx.clone(),
            file_set_id,
            extract_files,
            cloud_ops: None,
            fs_ops: self.fs_ops.clone(),
            export_ops: Arc::new(DefaultFileExportOps),
            thumbnail_generator: Arc::new(ThumbnailGenerator),
        };
        let mut context = DownloadContext::new(settings);

        let pipeline = Pipeline::<DownloadContext>::new();
        match pipeline.execute(&mut context).await {
            Ok(_) => {
                let successful_downloads = context.successful_downloads();
                let failed_downloads = context.failed_downloads();

                tracing::info!(
                    successful = successful_downloads,
                    failed = failed_downloads,
                    "Download completed"
                );

                Ok(DownloadResult {
                    successful_downloads,
                    failed_downloads,
                    thumbnail_path_map: context.thumbnail_path_map,
                })
            }
            Err(e) => Err(e),
        }
    }
}

#[derive(Debug)]
pub struct DownloadResult {
    pub successful_downloads: usize,
    pub failed_downloads: usize,
    pub thumbnail_path_map: ThumbnailPathMap,
}
