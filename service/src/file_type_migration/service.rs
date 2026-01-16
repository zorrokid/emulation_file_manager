use std::sync::Arc;

use cloud_storage::{CloudStorageOps, S3CloudStorage};
use database::repository_manager::RepositoryManager;

use crate::{
    file_system_ops::{FileSystemOps, StdFileSystemOps},
    file_type_migration::context::FileTypeMigrationContext,
    view_models::Settings,
};

pub struct FileTypeMigrationService {
    repository_manager: Arc<RepositoryManager>,
    settings: Arc<Settings>,
    fs_ops: Arc<dyn FileSystemOps>,
    cloud_storage_ops: Arc<dyn CloudStorageOps>,
}
impl FileTypeMigrationService {
    pub fn new(repository_manager: Arc<RepositoryManager>, settings: Arc<Settings>) -> Self {
        Self {
            repository_manager,
            settings,
            fs_ops: Arc::new(StdFileSystemOps),
            cloud_storage_ops: Arc::new(S3CloudStorage),
        }
    }

    pub fn new_with_ops(
        repository_manager: Arc<RepositoryManager>,
        settings: Arc<Settings>,
        fs_ops: Arc<dyn FileSystemOps>,
        cloud_storage_ops: Arc<dyn CloudStorageOps>,
    ) -> Self {
        Self {
            repository_manager,
            settings,
            fs_ops,
            cloud_storage_ops,
        }
    }

    pub fn migrate_file_types(&self) {
        let context = FileTypeMigrationContext::new(
            self.repository_manager.clone(),
            self.settings.clone(),
            self.fs_ops.clone(),
            self.cloud_storage_ops.clone(),
            false,
        );

        let pipeline = Pipeline::<FileTypeMigrationContext>::new();
        match pipeline.run(context) {
            Ok(_) => {
                tracing::info!("File type migration completed successfully.");
            }
            Err(e) => {
                tracing::error!("File type migration failed: {:?}", e);
            }
        }
    }
}
