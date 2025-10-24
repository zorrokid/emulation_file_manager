use std::{collections::HashMap, sync::Arc};

use async_std::channel::Sender;
use cloud_storage::{CloudStorageOps, SyncEvent};
use database::repository_manager::RepositoryManager;

use crate::view_models::Settings;

pub struct SyncContext {
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
    pub progress_tx: Sender<SyncEvent>,

    // Lazy initialized by ConnectToCloudStep
    // Need to use dyn because CloudStorageOps is a trait
    // and trait is used so that we can have mock implementations for testing
    // and different cloud storage providers.
    pub cloud_ops: Option<Arc<dyn CloudStorageOps>>,

    // Upload state
    pub files_prepared_for_upload: i64,
    pub upload_results: HashMap<String, FileSyncResult>,

    // Deletion state
    pub files_prepared_for_deletion: i64,
    pub deletion_results: HashMap<String, FileSyncResult>,
}

#[derive(Debug, Clone)]
pub struct FileSyncResult {
    pub file_info_id: i64,
    pub cloud_key: String,
    pub success: bool,
    pub error_message: Option<String>,
}

impl SyncContext {
    pub fn new(
        repository_manager: Arc<RepositoryManager>,
        settings: Arc<Settings>,
        progress_tx: Sender<SyncEvent>,
    ) -> Self {
        Self {
            repository_manager,
            settings,
            progress_tx,
            cloud_ops: None, // Will be filled by ConnectToCloudStep
            files_prepared_for_upload: 0,
            upload_results: HashMap::new(),
            files_prepared_for_deletion: 0,
            deletion_results: HashMap::new(),
        }
    }

    pub fn successful_uploads(&self) -> usize {
        self.upload_results.values().filter(|r| r.success).count()
    }

    pub fn failed_uploads(&self) -> usize {
        self.upload_results.values().filter(|r| !r.success).count()
    }

    pub fn successful_deletions(&self) -> usize {
        self.deletion_results.values().filter(|r| r.success).count()
    }

    pub fn failed_deletions(&self) -> usize {
        self.deletion_results
            .values()
            .filter(|r| !r.success)
            .count()
    }
}
