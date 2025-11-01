use std::sync::Arc;

use async_std::channel::Sender;
use database::repository_manager::RepositoryManager;

use crate::{
    file_set_download::context::DownloadContext, pipeline::Pipeline,
    settings_service::SettingsService, view_models::Settings,
};

#[derive(Debug)]
pub struct DownloadService {
    repository_manager: Arc<RepositoryManager>,
    settings: Arc<Settings>,
    settings_service: Arc<SettingsService>,
}

pub enum DownloadEvent {
    DownloadStarted {
        file_set_id: i64,
        number_of_files: usize,
    },
    FileDownloadStarted {
        file_info_id: i64,
    },
    FileDownloadProgress {
        file_info_id: i64,
        bytes_downloaded: u64,
    },
    FileDownloadCompleted {
        file_info_id: i64,
    },
    DownloadCompleted {
        file_set_id: i64,
    },
}

impl DownloadService {
    pub fn new(repository_manager: Arc<RepositoryManager>, settings: Arc<Settings>) -> Self {
        let settings_service = Arc::new(SettingsService::new(repository_manager.clone()));
        Self {
            repository_manager,
            settings,
            settings_service,
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
        );
        let pipeline = Pipeline::<DownloadContext>::new();
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
