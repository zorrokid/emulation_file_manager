use std::{collections::HashMap, sync::Arc};

use async_std::channel::Sender;
use cloud_storage::CloudStorageOps;
use core_types::events::SyncEvent;
use database::repository_manager::RepositoryManager;

use crate::{
    pipeline::cloud_connection::CloudConnectionContext, settings_service::SettingsService,
    view_models::Settings,
};

pub struct SyncContext {
    pub repository_manager: Arc<RepositoryManager>,
    pub settings_service: Arc<SettingsService>,
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

    pub cloud_operation_success: bool,
    pub db_update_success: bool,

    pub cloud_error: Option<String>,
    pub db_error: Option<String>,
}

impl FileSyncResult {
    pub fn is_complete_success(&self) -> bool {
        self.cloud_operation_success && self.db_update_success
    }

    /// Partial success - cloud worked but DB didn't (DANGEROUS!)
    pub fn is_partial_success(&self) -> bool {
        self.cloud_operation_success && !self.db_update_success
    }

    /// Clean failure - cloud operation failed
    pub fn is_clean_failure(&self) -> bool {
        !self.cloud_operation_success
    }

    /// Get the main error message
    pub fn error_message(&self) -> Option<String> {
        match (&self.cloud_error, &self.db_error) {
            (Some(cloud), Some(db)) => Some(format!("Cloud: {}; DB: {}", cloud, db)),
            (Some(cloud), None) => Some(cloud.clone()),
            (None, Some(db)) => Some(format!("DB update failed: {}", db)),
            (None, None) => None,
        }
    }
}

impl SyncContext {
    pub fn new(
        repository_manager: Arc<RepositoryManager>,
        settings: Arc<Settings>,
        progress_tx: Sender<SyncEvent>,
    ) -> Self {
        let settings_service = Arc::new(SettingsService::new(repository_manager.clone()));
        Self {
            repository_manager,
            settings,
            progress_tx,
            cloud_ops: None, // Will be filled by ConnectToCloudStep
            files_prepared_for_upload: 0,
            upload_results: HashMap::new(),
            files_prepared_for_deletion: 0,
            deletion_results: HashMap::new(),
            settings_service,
        }
    }

    // upload stats
    pub fn successful_uploads(&self) -> usize {
        self.upload_results
            .values()
            .filter(|r| r.is_complete_success())
            .count()
    }

    pub fn failed_uploads(&self) -> usize {
        self.upload_results
            .values()
            .filter(|r| !r.is_complete_success())
            .count()
    }

    /// CRITICAL: Files uploaded to cloud but not tracked in DB
    pub fn partial_successful_uploads(&self) -> usize {
        self.upload_results
            .values()
            .filter(|r| r.is_partial_success())
            .count()
    }

    pub fn get_partial_successful_uploads(&self) -> Vec<&FileSyncResult> {
        self.upload_results
            .values()
            .filter(|r| r.is_partial_success())
            .collect()
    }

    // deletion stats

    pub fn successful_deletions(&self) -> usize {
        self.deletion_results
            .values()
            .filter(|r| r.cloud_operation_success)
            .count()
    }

    pub fn failed_deletions(&self) -> usize {
        self.deletion_results
            .values()
            .filter(|r| !r.cloud_operation_success)
            .count()
    }
}

impl CloudConnectionContext for SyncContext {
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
        self.cloud_ops.is_none()
            && (self.files_prepared_for_upload > 0 || self.files_prepared_for_deletion > 0)
    }
}
