use std::sync::Arc;

use cloud_storage::{CloudStorageOps, S3CloudStorage};
use database::repository_manager::RepositoryManager;

use crate::{
    file_system_ops::{FileSystemOps, StdFileSystemOps},
    file_type_migration::context::FileTypeMigrationContext,
    pipeline::generic_pipeline::Pipeline,
    settings_service::SettingsService,
    view_models::Settings,
};

pub struct FileTypeMigrationService {
    repository_manager: Arc<RepositoryManager>,
    settings: Arc<Settings>,
    settings_service: Arc<SettingsService>,
    fs_ops: Arc<dyn FileSystemOps>,
}

impl FileTypeMigrationService {
    pub fn new(repository_manager: Arc<RepositoryManager>, settings: Arc<Settings>) -> Self {
        let settings_service = Arc::new(SettingsService::new(repository_manager.clone()));
        Self {
            repository_manager,
            settings,
            settings_service,
            fs_ops: Arc::new(StdFileSystemOps),
        }
    }

    pub fn new_with_ops(
        repository_manager: Arc<RepositoryManager>,
        settings: Arc<Settings>,
        settings_service: Arc<SettingsService>,
        fs_ops: Arc<dyn FileSystemOps>,
    ) -> Self {
        Self {
            repository_manager,
            settings,
            settings_service,
            fs_ops,
        }
    }

    pub async fn migrate_file_types(&self) {
        let mut context = FileTypeMigrationContext::new(
            self.repository_manager.clone(),
            self.settings.clone(),
            self.settings_service.clone(),
            self.fs_ops.clone(),
            false,
        );

        let pipeline = Pipeline::<FileTypeMigrationContext>::new();
        match pipeline.execute(&mut context).await {
            Ok(_) => {
                tracing::info!("File type migration completed successfully.");
            }
            Err(e) => {
                tracing::error!("File type migration failed: {:?}", e);
            }
        }
    }
}
