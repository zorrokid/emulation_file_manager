use std::{collections::HashMap, sync::Arc};

use database::repository_manager::RepositoryManager;

use crate::{
    file_set_deletion::model::FileDeletionResult, file_system_ops::FileSystemOps,
    view_models::Settings,
};

/// Context object that flows through the pipeline, accumulating state
pub struct DeletionContext {
    pub file_set_id: i64,
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
    pub fs_ops: Arc<dyn FileSystemOps>,

    // Accumulated state as pipeline progresses
    pub deletion_results: HashMap<Vec<u8>, FileDeletionResult>,
}
