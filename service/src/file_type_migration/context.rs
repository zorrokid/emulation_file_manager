use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

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
    // Mapping of file_set_id to new FileType
    pub file_sets_to_migrate: HashMap<i64, FileTypeMigration>,
    pub moved_local_file_sha1_checksums: HashSet<Sha1Checksum>,
    pub non_existing_local_file_sha1_checksums: HashSet<Sha1Checksum>,
    pub is_dry_run: bool,
}
