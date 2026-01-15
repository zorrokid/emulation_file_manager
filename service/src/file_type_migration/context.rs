use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use cloud_storage::CloudStorageOps;
use core_types::{FileType, Sha1Checksum};
use database::repository_manager::RepositoryManager;

use crate::{file_system_ops::FileSystemOps, view_models::Settings};

pub struct FileTypeMigration {
    pub old_file_type: FileType,
    pub new_file_type: FileType,
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
