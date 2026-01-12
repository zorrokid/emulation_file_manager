use std::{collections::HashMap, sync::Arc};

use core_types::FileType;
use database::repository_manager::RepositoryManager;

pub struct FileTypeMigrationContext {
    pub repository_manager: Arc<RepositoryManager>,
    // Mapping of file_set_id to new FileType
    pub file_sets_to_migrate: HashMap<i64, FileType>,
}
