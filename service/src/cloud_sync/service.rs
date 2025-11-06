use std::sync::Arc;

use async_std::channel::Sender;
use cloud_storage::SyncEvent;
use database::repository_manager::RepositoryManager;

use crate::{
    cloud_sync::context::SyncContext, error::Error, pipeline::generic_pipeline::Pipeline,
    view_models::Settings,
};

#[derive(Debug)]
pub struct CloudStorageSyncService {
    repository_manager: Arc<RepositoryManager>,
    settings: Arc<Settings>,
}

impl CloudStorageSyncService {
    pub fn new(repository_manager: Arc<RepositoryManager>, settings: Arc<Settings>) -> Self {
        Self {
            repository_manager,
            settings,
        }
    }

    #[tracing::instrument(skip_all, err)]
    pub async fn sync_to_cloud(&self, progress_tx: Sender<SyncEvent>) -> Result<SyncResult, Error> {
        tracing::info!("Starting cloud sync operation");
        let mut context = SyncContext::new(
            self.repository_manager.clone(),
            self.settings.clone(),
            progress_tx.clone(),
        );

        let pipeline = Pipeline::<SyncContext>::new();
        pipeline.execute(&mut context).await?;
        let successful_uploads = context.successful_uploads();
        let failed_uploads = context.failed_uploads();
        let successful_deletions = context.successful_deletions();
        let failed_deletions = context.failed_deletions();

        tracing::info!(
            successful_uploads,
            failed_uploads,
            successful_deletions,
            failed_deletions,
            "Cloud sync summary"
        );

        Ok(SyncResult {
            successful_uploads,
            failed_uploads,
            successful_deletions,
            failed_deletions,
        })
    }
}

#[derive(Debug)]
pub struct SyncResult {
    pub successful_uploads: usize,
    pub failed_uploads: usize,
    pub successful_deletions: usize,
    pub failed_deletions: usize,
}
