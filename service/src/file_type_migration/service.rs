use std::sync::Arc;

use database::repository_manager::RepositoryManager;

use crate::{
    error::Error,
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

impl std::fmt::Debug for FileTypeMigrationService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileTypeMigrationService").finish()
    }
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

    pub async fn migrate_file_types(&self) -> Result<(), Error> {
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
                Ok(())
            }
            Err(e) => {
                tracing::error!("File type migration failed: {:?}", e);
                Err(e)
            }
        }
    }
}
