use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use cloud_storage::CloudStorageOps;
use core_types::{FileType, item_type::ItemType};
use database::repository_manager::RepositoryManager;

use crate::{
    file_system_ops::FileSystemOps, pipeline::cloud_connection::CloudConnectionContext,
    settings_service::SettingsService, view_models::Settings,
};

pub struct FileTypeMigration {
    pub old_file_type: FileType,
    pub new_file_type: FileType,
    pub item_type: Option<ItemType>,
}

pub struct FileTypeMigrationContext {
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
    pub settings_service: Arc<SettingsService>,
    pub fs_ops: Arc<dyn FileSystemOps>,
    // Lazy initialized by ConnectToCloudStep
    pub cloud_ops: Option<Arc<dyn CloudStorageOps>>,
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
        settings_service: Arc<SettingsService>,
        fs_ops: Arc<dyn FileSystemOps>,
        is_dry_run: bool,
    ) -> Self {
        Self {
            repository_manager,
            settings,
            fs_ops,
            file_sets_to_migrate: HashMap::new(),
            file_ids_synced_to_cloud: HashSet::new(),
            moved_local_file_ids: HashSet::new(),
            moved_cloud_file_ids: HashSet::new(),
            non_existing_local_file_ids: HashSet::new(),
            updated_file_info_ids: HashSet::new(),
            updated_file_set_ids: HashSet::new(),
            is_dry_run,
            cloud_ops: None,
            settings_service,
        }
    }

    pub fn collect_migration_results(&self) -> HashMap<&'static str, usize> {
        let mut results = HashMap::new();
        results.insert("file_sets_to_migrate", self.file_sets_to_migrate.len());
        results.insert(
            "file_ids_synced_to_cloud",
            self.file_ids_synced_to_cloud.len(),
        );
        results.insert("moved_local_file_ids", self.moved_local_file_ids.len());
        results.insert("moved_cloud_file_ids", self.moved_cloud_file_ids.len());
        results.insert(
            "non_existing_local_file_ids",
            self.non_existing_local_file_ids.len(),
        );
        results.insert("updated_file_info_ids", self.updated_file_info_ids.len());
        results.insert("updated_file_set_ids", self.updated_file_set_ids.len());
        results
    }
}

impl CloudConnectionContext for FileTypeMigrationContext {
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
        !self.file_ids_synced_to_cloud.is_empty()
    }
}
