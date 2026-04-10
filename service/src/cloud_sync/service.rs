use std::sync::Arc;

use core_types::events::SyncEvent;
use database::repository_manager::RepositoryManager;
use flume::{Receiver, Sender};

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
    pub async fn sync_to_cloud(
        &self,
        progress_tx: Sender<SyncEvent>,
        cancel_rx: Receiver<()>,
    ) -> Result<SyncResult, Error> {
        tracing::info!("Starting cloud sync operation");
        let mut context = SyncContext::new(
            self.repository_manager.clone(),
            self.settings.clone(),
            progress_tx.clone(),
            cancel_rx,
        );

        // Populate counts before the pipeline so SyncStarted carries accurate totals.
        let repo = self.repository_manager.get_file_info_repository();

        context.files_prepared_for_upload = repo
            .count_files_pending_upload()
            .await
            .map_err(|e| Error::DbError(e.to_string()))?;

        context.cloud_files_prepared_for_deletion = repo
            .count_cloud_files_pending_deletion()
            .await
            .map_err(|e| Error::DbError(e.to_string()))?;

        context.tombstones_prepared_for_cleanup = repo
            .count_tombstones_pending_deletion()
            .await
            .map_err(|e| Error::DbError(e.to_string()))?;

        let send_res = progress_tx.send(SyncEvent::SyncStarted {
            total_upload_count: context.files_prepared_for_upload,
            total_deletion_count: context.cloud_files_prepared_for_deletion,
        });
        if let Err(e) = send_res {
            tracing::error!("Failed to send SyncStarted event: {}", e);
        }

        let pipeline = Pipeline::<SyncContext>::new();
        let pipeline_result = pipeline.execute(&mut context).await;

        // Send terminal lifecycle event based on pipeline outcome.
        match &pipeline_result {
            Ok(_) => {
                let _ = progress_tx.send(SyncEvent::SyncCompleted);
            }
            Err(Error::OperationCancelled) => {
                let _ = progress_tx.send(SyncEvent::SyncCancelled);
            }
            Err(e) => {
                let _ = progress_tx.send(SyncEvent::SyncFailed {
                    error: e.to_string(),
                });
            }
        }

        // Propagate hard errors after sending the terminal event.
        pipeline_result?;

        let successful_uploads = context.successful_uploads();
        let failed_uploads = context.failed_uploads();
        let successful_deletions = context.successful_deletions();
        let failed_deletions = context.failed_deletions();
        let partial_successful_uploads = context.partial_successful_uploads();
        let tombstones_cleaned_up = context.tombstones_cleaned_up;

        if partial_successful_uploads > 0 {
            tracing::warn!(
                partial_successful_uploads,
                "Ghost uploads detected: files uploaded to cloud but DB not updated"
            );
        }

        tracing::info!(
            successful_uploads,
            failed_uploads,
            successful_deletions,
            failed_deletions,
            partial_successful_uploads,
            tombstones_cleaned_up,
            "Cloud sync summary"
        );

        Ok(SyncResult {
            successful_uploads,
            failed_uploads,
            successful_deletions,
            failed_deletions,
            partial_successful_uploads,
            tombstones_cleaned_up,
        })
    }
}

#[derive(Debug)]
pub struct SyncResult {
    pub successful_uploads: usize,
    pub failed_uploads: usize,
    pub successful_deletions: usize,
    pub failed_deletions: usize,
    /// Uploads where the cloud operation succeeded but the DB update failed.
    /// These files exist in cloud storage but remain `NotSynced` in the DB.
    pub partial_successful_uploads: usize,
    pub tombstones_cleaned_up: usize,
}
