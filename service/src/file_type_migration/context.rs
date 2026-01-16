use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use cloud_storage::CloudStorageOps;
use core_types::{FileType, item_type::ItemType};
use database::repository_manager::RepositoryManager;

use crate::{file_system_ops::FileSystemOps, view_models::Settings};

pub struct FileTypeMigration {
    pub old_file_type: FileType,
    pub new_file_type: FileType,
    pub item_type: Option<ItemType>,
}

pub struct FileTypeMigrationContext {
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
    pub fs_ops: Arc<dyn FileSystemOps>,
    pub cloud_storage_ops: Arc<dyn CloudStorageOps>,
    // Mapping of file_set_id to new FileType
    pub file_sets_to_migrate: HashMap<i64, FileTypeMigration>,
    pub file_ids_synced_to_cloud: HashSet<i64>,
    pub moved_local_file_ids: HashSet<i64>,
    pub moved_cloud_file_ids: HashSet<i64>,
    pub non_existing_local_file_ids: HashSet<i64>,
    pub updated_file_info_ids: HashSet<i64>,
    pub updated_file_set_ids: HashSet<i64>,
    pub is_dry_run: bool,
}

impl FileTypeMigrationContext {
    pub fn new(
        repository_manager: Arc<RepositoryManager>,
        settings: Arc<Settings>,
        fs_ops: Arc<dyn FileSystemOps>,
        cloud_storage_ops: Arc<dyn CloudStorageOps>,
        is_dry_run: bool,
    ) -> Self {
        Self {
            repository_manager,
            settings,
            fs_ops,
            cloud_storage_ops,
            file_sets_to_migrate: HashMap::new(),
            file_ids_synced_to_cloud: HashSet::new(),
            moved_local_file_ids: HashSet::new(),
            moved_cloud_file_ids: HashSet::new(),
            non_existing_local_file_ids: HashSet::new(),
            updated_file_info_ids: HashSet::new(),
            updated_file_set_ids: HashSet::new(),
            is_dry_run,
        }
    }
}
