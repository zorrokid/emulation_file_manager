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
        fs_ops: Arc<dyn FileSystemOps>,
    ) -> Self {
        let settings_service = Arc::new(SettingsService::new(repository_manager.clone()));
        Self {
            repository_manager,
            settings,
            settings_service,
            fs_ops,
        }
    }

    pub async fn migrate_file_types(&self, is_dry_run: bool) -> Result<(), Error> {
        let mut context = FileTypeMigrationContext::new(
            self.repository_manager.clone(),
            self.settings.clone(),
            self.settings_service.clone(),
            self.fs_ops.clone(),
            is_dry_run,
        );

        let pipeline = Pipeline::<FileTypeMigrationContext>::new();
        match pipeline.execute(&mut context).await {
            Ok(_) => {
                tracing::info!("File type migration completed successfully.");
                let migration_results = context.collect_migration_results();
                for (key, value) in migration_results {
                    tracing::info!("{}: {}", key, value);
                }
                Ok(())
            }
            Err(e) => {
                tracing::error!("File type migration failed: {:?}", e);
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use core_types::{FileType, ImportedFile, Sha1Checksum};
    use database::setup_test_db;

    use crate::file_system_ops::mock::MockFileSystemOps;

    use super::*;

    async fn insert_test_system(repository_manager: &RepositoryManager, name: &str) -> i64 {
        repository_manager
            .get_system_repository()
            .add_system(name)
            .await
            .unwrap()
    }

    async fn insert_test_file_set(
        repository_manager: &RepositoryManager,
        file_type: &FileType,
        file_sha1: Sha1Checksum,
        archive_file_name: String,
    ) -> i64 {
        let system_id = insert_test_system(repository_manager, "Test System").await;

        let imported_file = ImportedFile {
            sha1_checksum: file_sha1,
            file_size: 1234,
            archive_file_name,
            original_file_name: "original_test_file.rom".to_string(),
        };

        repository_manager
            .get_file_set_repository()
            .add_file_set(
                "Test FileSet",
                "Test Description",
                file_type,
                "source",
                &[imported_file],
                &[system_id],
            )
            .await
            .unwrap()
    }

    #[async_std::test]
    async fn test_migrate_file_types() {
        let pool = setup_test_db().await;
        let repository_manager = Arc::new(RepositoryManager::new(Arc::new(pool)));
        let settings = Arc::new(Settings {
            collection_root_dir: "/files".into(),
            ..Default::default()
        });

        // add file set to be migrated
        let file_checksum = Sha1Checksum::from([0; 20]);
        let archive_file_name = "123123.zst".to_string();
        let file_set_id = insert_test_file_set(
            &repository_manager,
            &FileType::ManualScan, // should be migrated to Scan
            file_checksum,
            archive_file_name.clone(),
        )
        .await;

        let fs_ops = Arc::new(MockFileSystemOps::new());
        let service = FileTypeMigrationService::new_with_ops(repository_manager, settings, fs_ops);

        let result = service.migrate_file_types(false).await;

        let file_set = service
            .repository_manager
            .get_file_set_repository()
            .get_file_set(file_set_id)
            .await
            .unwrap();

        assert!(result.is_ok());
        assert_eq!(file_set.file_type, FileType::Scan);

        let file_info = service
            .repository_manager
            .get_file_info_repository()
            .get_file_infos_by_file_set(file_set_id)
            .await
            .unwrap();

        assert_eq!(file_info.len(), 1);
        assert_eq!(file_info[0].file_type, FileType::Scan);

        let file_set_items = service
            .repository_manager
            .get_file_set_repository()
            .get_item_types_for_file_set(file_set_id)
            .await
            .unwrap();

        assert_eq!(file_set_items.len(), 1);
        assert_eq!(file_set_items[0], core_types::item_type::ItemType::Manual);
    }
}
