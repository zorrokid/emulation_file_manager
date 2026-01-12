use std::{collections::HashMap, sync::Arc};

use core_types::FileType;
use database::repository_manager::RepositoryManager;

pub struct FileTypeMigrationContext {
    pub repository_manager: Arc<RepositoryManager>,
    pub old_file_type: HashMap<i64, FileType>,
    pub new_file_type: HashMap<i64, FileType>,
}
