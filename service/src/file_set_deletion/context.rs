use std::{collections::HashMap, sync::Arc};

use core_types::Sha1Checksum;
use database::repository_manager::RepositoryManager;

use crate::{
    file_import::common_steps::file_deletion_steps::FileDeletionStepsContext,
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
    pub deletion_results: HashMap<Sha1Checksum, FileDeletionResult>,
}

impl FileDeletionStepsContext for DeletionContext {
    fn repository_manager(&self) -> Arc<RepositoryManager> {
        self.repository_manager.clone()
    }

    fn file_set_id(&self) -> i64 {
        self.file_set_id
    }

    fn has_deletion_candidates(&self) -> bool {
        !self.deletion_results.is_empty()
    }

    fn deletion_results_mut(&mut self) -> &mut HashMap<Sha1Checksum, FileDeletionResult> {
        &mut self.deletion_results
    }

    fn deletion_results(&self) -> &HashMap<Sha1Checksum, FileDeletionResult> {
        &self.deletion_results
    }

    fn fs_ops(&self) -> Arc<dyn FileSystemOps> {
        self.fs_ops.clone()
    }

    fn settings(&self) -> Arc<Settings> {
        self.settings.clone()
    }
}
