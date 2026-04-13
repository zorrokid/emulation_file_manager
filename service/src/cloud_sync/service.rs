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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use core_types::{CloudSyncStatus, FileType, Sha1Checksum, events::SyncEvent};
    use database::setup_test_repository_manager;

    use crate::view_models::Settings;

    use super::CloudStorageSyncService;

    fn setup_service(
        repos: Arc<database::repository_manager::RepositoryManager>,
    ) -> CloudStorageSyncService {
        let settings = Arc::new(Settings::default());
        CloudStorageSyncService::new(repos, settings)
    }

    fn collect_events(rx: flume::Receiver<SyncEvent>) -> Vec<SyncEvent> {
        std::iter::from_fn(|| rx.try_recv().ok()).collect()
    }

    #[async_std::test]
    async fn test_sync_completed_sent_when_nothing_to_sync() {
        let repos = setup_test_repository_manager().await;
        let service = setup_service(repos);
        let (tx, rx) = flume::unbounded();
        let (_cancel_tx, cancel_rx) = flume::unbounded::<()>();

        let result = service.sync_to_cloud(tx, cancel_rx).await;

        assert!(result.is_ok());
        let events = collect_events(rx);
        assert_eq!(
            events.iter().filter(|e| matches!(e, SyncEvent::SyncCompleted)).count(),
            1
        );
        assert!(!events.iter().any(|e| matches!(e, SyncEvent::SyncFailed { .. })));
    }

    #[async_std::test]
    async fn test_sync_completed_sent_when_only_tombstones_cleaned() {
        let repos = setup_test_repository_manager().await;
        let id = repos
            .get_file_info_repository()
            .add_file_info(&Sha1Checksum::from([1u8; 20]), 1234, None, FileType::Rom)
            .await
            .unwrap();
        repos
            .get_file_info_repository()
            .update_cloud_sync_status(id, CloudSyncStatus::DeletionPending)
            .await
            .unwrap();

        let service = setup_service(repos);
        let (tx, rx) = flume::unbounded();
        let (_cancel_tx, cancel_rx) = flume::unbounded::<()>();

        let result = service.sync_to_cloud(tx, cancel_rx).await;

        assert!(result.is_ok());
        let sync_result = result.unwrap();
        assert_eq!(sync_result.tombstones_cleaned_up, 1);
        let events = collect_events(rx);
        assert_eq!(
            events.iter().filter(|e| matches!(e, SyncEvent::SyncCompleted)).count(),
            1
        );
        assert!(!events.iter().any(|e| matches!(e, SyncEvent::SyncFailed { .. })));
    }

    #[async_std::test]
    async fn test_sync_failed_sent_when_pipeline_aborts() {
        // Settings::default() has no S3 config. Adding a file pending upload triggers
        // ConnectToCloudStep, which aborts with a settings error.
        let repos = setup_test_repository_manager().await;
        repos
            .get_file_info_repository()
            .add_file_info(
                &Sha1Checksum::from([1u8; 20]),
                1234,
                Some("file.zst"),
                FileType::Rom,
            )
            .await
            .unwrap();

        let service = setup_service(repos);
        let (tx, rx) = flume::unbounded();
        let (_cancel_tx, cancel_rx) = flume::unbounded::<()>();

        let _ = service.sync_to_cloud(tx, cancel_rx).await;

        let events = collect_events(rx);
        assert!(events.iter().any(|e| matches!(e, SyncEvent::SyncFailed { .. })));
        assert!(!events.iter().any(|e| matches!(e, SyncEvent::SyncCompleted)));
    }

    #[async_std::test]
    async fn test_sync_started_carries_correct_upload_and_deletion_counts() {
        let repos = setup_test_repository_manager().await;
        let repo = repos.get_file_info_repository();

        // 2 files pending upload (NotSynced by default)
        repo.add_file_info(&Sha1Checksum::from([1u8; 20]), 1234, Some("file1.zst"), FileType::Rom)
            .await
            .unwrap();
        repo.add_file_info(&Sha1Checksum::from([2u8; 20]), 1234, Some("file2.zst"), FileType::Rom)
            .await
            .unwrap();

        // 1 cloud file pending deletion (has archive_file_name, status = DeletionPending)
        let del_id = repo
            .add_file_info(
                &Sha1Checksum::from([3u8; 20]),
                1234,
                Some("deleted.zst"),
                FileType::Rom,
            )
            .await
            .unwrap();
        repo.update_cloud_sync_status(del_id, CloudSyncStatus::DeletionPending)
            .await
            .unwrap();

        let service = setup_service(repos);
        let (tx, rx) = flume::unbounded();
        let (_cancel_tx, cancel_rx) = flume::unbounded::<()>();

        // The pipeline will fail (no S3 config) but SyncStarted is emitted before it runs.
        let _ = service.sync_to_cloud(tx, cancel_rx).await;

        let events = collect_events(rx);
        let started = events
            .iter()
            .find(|e| matches!(e, SyncEvent::SyncStarted { .. }))
            .expect("SyncStarted not found");
        assert!(matches!(
            started,
            SyncEvent::SyncStarted {
                total_upload_count: 2,
                total_deletion_count: 1
            }
        ));
    }

    #[async_std::test]
    async fn test_sync_cancelled_sent_exactly_once() {
        // Use a tombstone to trigger CleanupTombstonesStep (skips ConnectToCloudStep).
        // Pre-send cancel so the step's per-item check picks it up immediately.
        let repos = setup_test_repository_manager().await;
        let id = repos
            .get_file_info_repository()
            .add_file_info(&Sha1Checksum::from([1u8; 20]), 1234, None, FileType::Rom)
            .await
            .unwrap();
        repos
            .get_file_info_repository()
            .update_cloud_sync_status(id, CloudSyncStatus::DeletionPending)
            .await
            .unwrap();

        let service = setup_service(repos);
        let (tx, rx) = flume::unbounded();
        let (cancel_tx, cancel_rx) = flume::unbounded::<()>();
        cancel_tx.send(()).unwrap();

        let result = service.sync_to_cloud(tx, cancel_rx).await;

        assert!(result.is_err());
        let events = collect_events(rx);
        assert_eq!(
            events.iter().filter(|e| matches!(e, SyncEvent::SyncCancelled)).count(),
            1
        );
        assert!(!events.iter().any(|e| matches!(e, SyncEvent::SyncCompleted)));
        assert!(!events.iter().any(|e| matches!(e, SyncEvent::SyncFailed { .. })));
    }
}

/// Summary of a completed cloud sync operation returned by [`CloudStorageSyncService::sync_to_cloud`].
#[derive(Debug)]
pub struct SyncResult {
    pub successful_uploads: usize,
    pub failed_uploads: usize,
    pub successful_deletions: usize,
    pub failed_deletions: usize,
    /// Uploads where the cloud operation succeeded but the DB update failed.
    /// These files exist in cloud storage but remain `NotSynced` in the DB.
    pub partial_successful_uploads: usize,
    /// Tombstone records (`DeletionPending` with no `archive_file_name`) deleted from the DB.
    pub tombstones_cleaned_up: usize,
}
