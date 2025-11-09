use std::{collections::HashMap, sync::Arc};

use database::{models::FileInfo, repository_manager::RepositoryManager};

use crate::{file_system_ops::FileSystemOps, view_models::Settings};

/// Context object that flows through the pipeline, accumulating state
pub struct DeletionContext {
    pub file_set_id: i64,
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
    pub fs_ops: Arc<dyn FileSystemOps>,

    // Accumulated state as pipeline progresses
    pub deletion_results: HashMap<Vec<u8>, FileDeletionResult>,
}

#[derive(Debug, Clone)]
pub struct FileDeletionResult {
    pub file_info: FileInfo,
    pub file_path: Option<String>,
    pub file_deletion_success: bool,
    pub error_messages: Vec<String>,
    pub is_deletable: bool,
    pub was_deleted_from_db: bool,
    pub cloud_sync_marked: bool,
}
