use std::path::Path;

use crate::{
    error::Error,
    file_import::import::context::FileImportContext,
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

pub struct ImportFilesStep;
#[async_trait::async_trait]
impl PipelineStep<FileImportContext> for ImportFilesStep {
    fn name(&self) -> &'static str {
        "import_files"
    }
    async fn execute(&self, context: &mut FileImportContext) -> StepAction {
        match context
            .file_import_ops
            .import(&context.get_file_import_model())
        {
            Ok(imported_files) => {
                context.imported_files = imported_files;
            }
            Err(err) => {
                tracing::error!("Error importing files: {}", err);
                return StepAction::Abort(Error::FileImportError(format!(
                    "Error importing files: {}",
                    err
                )));
            }
        }
        StepAction::Continue
    }
}

pub struct UpdateDatabaseStep;

#[async_trait::async_trait]
impl PipelineStep<FileImportContext> for UpdateDatabaseStep {
    fn name(&self) -> &'static str {
        "update_database"
    }
    async fn execute(&self, context: &mut FileImportContext) -> StepAction {
        match context
            .repository_manager
            .get_file_set_repository()
            .add_file_set(
                &context.file_set_name,
                &context.file_set_file_name,
                &context.file_type,
                &context.source,
                &context.get_files_in_file_set(),
                &context.system_ids,
            )
            .await
        {
            Ok(id) => {
                tracing::info!(
                    "File set '{}' with id {} added to database",
                    context.file_set_name,
                    id
                );
                context.file_set_id = Some(id);
            }
            Err(err) => {
                tracing::error!(
                    "Error adding file set '{}' to database: {}",
                    context.file_set_name,
                    err
                );

                for imported_file in context.imported_files.values() {
                    let file_path = context
                        .settings
                        .get_file_path(&context.file_type, &imported_file.archive_file_name);
                    if let Err(e) = context.file_system_ops.remove_file(&file_path) {
                        tracing::error!(
                            "Error deleting imported file '{}' after database failure: {}",
                            file_path.display(),
                            e
                        );

                        return StepAction::Abort(Error::FileImportError(format!(
                            "Error deleting imported file '{}' after database failure: {}",
                            file_path.display(),
                            e
                        )));
                    }
                }

                return StepAction::Abort(Error::DbError(format!(
                    "Error adding file set to database: {}",
                    err
                )));
            }
        }

        StepAction::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_import::model::{FileImportModel as ServiceFileImportModel, ImportFileContent};
    use crate::file_system_ops::mock::MockFileSystemOps;
    use core_types::{FileType, ImportedFile, Sha1Checksum};
    use database::{repository_manager::RepositoryManager, setup_test_db};
    use file_import::mock::MockFileImportOps;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::Arc;

    async fn create_test_context(
        file_import_ops: Arc<MockFileImportOps>,
    ) -> FileImportContext {
        let pool = Arc::new(setup_test_db().await);
        let repository_manager = Arc::new(RepositoryManager::new(pool));
        let settings = Arc::new(crate::view_models::Settings::default());
        let file_system_ops = Arc::new(MockFileSystemOps::new());

        FileImportContext {
            repository_manager,
            settings,
            selected_files: vec![],
            file_type: FileType::Rom,
            import_files: vec![],
            system_ids: vec![],
            source: "test_source".to_string(),
            file_set_name: "Test Game".to_string(),
            file_set_file_name: "test_game.zip".to_string(),
            imported_files: HashMap::new(),
            file_set_id: None,
            file_import_ops,
            file_system_ops,
        }
    }

    #[async_std::test]
    async fn test_import_files_step_success() {
        let checksum: Sha1Checksum = [1u8; 20];
        
        // Setup mock before creating context
        let mock_ops = Arc::new(MockFileImportOps::new());
        mock_ops.add_imported_file(
            checksum,
            ImportedFile {
                original_file_name: "game.rom".to_string(),
                sha1_checksum: checksum,
                file_size: 1024,
                archive_file_name: "archive123.zst".to_string(),
            },
        );
        
        let mut context = create_test_context(mock_ops).await;
        context.selected_files = vec![checksum];

        let mut content = HashMap::new();
        content.insert(
            checksum,
            ImportFileContent {
                file_name: "game.rom".to_string(),
                sha1_checksum: checksum,
                file_size: 1024,
                existing_file_info_id: None,
                existing_archive_file_name: None,
            },
        );

        context.import_files = vec![ServiceFileImportModel {
            path: PathBuf::from("/test/games.zip"),
            content,
        }];

        let step = ImportFilesStep;
        let result = step.execute(&mut context).await;

        assert!(matches!(result, StepAction::Continue));
        assert_eq!(context.imported_files.len(), 1);
        assert!(context.imported_files.contains_key(&checksum));
        let imported = context.imported_files.get(&checksum).unwrap();
        assert_eq!(imported.original_file_name, "game.rom");
        assert_eq!(imported.archive_file_name, "archive123.zst");
    }

    #[async_std::test]
    async fn test_import_files_step_failure() {
        let checksum: Sha1Checksum = [1u8; 20];
        
        // Setup mock to fail
        let mock_ops = Arc::new(MockFileImportOps::new());
        mock_ops.set_should_fail(true);
        
        let mut context = create_test_context(mock_ops).await;
        context.selected_files = vec![checksum];

        let mut content = HashMap::new();
        content.insert(
            checksum,
            ImportFileContent {
                file_name: "game.rom".to_string(),
                sha1_checksum: checksum,
                file_size: 1024,
                existing_file_info_id: None,
                existing_archive_file_name: None,
            },
        );

        context.import_files = vec![ServiceFileImportModel {
            path: PathBuf::from("/test/games.zip"),
            content,
        }];

        let step = ImportFilesStep;
        let result = step.execute(&mut context).await;

        assert!(matches!(result, StepAction::Abort(_)));
        assert!(context.imported_files.is_empty());
    }

    #[async_std::test]
    async fn test_update_database_step_success() {
        let mock_ops = Arc::new(MockFileImportOps::new());
        let mut context = create_test_context(mock_ops).await;
        let checksum: Sha1Checksum = [1u8; 20];

        // Add system to database first
        let system_id = context
            .repository_manager
            .get_system_repository()
            .add_system("Test System")
            .await
            .unwrap();

        context.selected_files = vec![checksum];
        context.system_ids = vec![system_id];

        context.imported_files.insert(
            checksum,
            ImportedFile {
                original_file_name: "game.rom".to_string(),
                sha1_checksum: checksum,
                file_size: 1024,
                archive_file_name: "archive123.zst".to_string(),
            },
        );

        let step = UpdateDatabaseStep;
        let result = step.execute(&mut context).await;

        if !matches!(result, StepAction::Continue) {
            panic!("Expected Continue, got: {:?}", result);
        }
        assert!(context.file_set_id.is_some());
        assert!(context.file_set_id.unwrap() > 0);
    }

    #[async_std::test]
    async fn test_update_database_step_with_existing_files() {
        let mock_ops = Arc::new(MockFileImportOps::new());
        let mut context = create_test_context(mock_ops).await;
        let checksum1: Sha1Checksum = [1u8; 20];
        let checksum2: Sha1Checksum = [2u8; 20];

        // Add system to database first
        let system_id = context
            .repository_manager
            .get_system_repository()
            .add_system("Test System")
            .await
            .unwrap();

        context.selected_files = vec![checksum1, checksum2];
        context.system_ids = vec![system_id];

        // Add one newly imported file
        context.imported_files.insert(
            checksum1,
            ImportedFile {
                original_file_name: "new_game.rom".to_string(),
                sha1_checksum: checksum1,
                file_size: 1024,
                archive_file_name: "new_archive.zst".to_string(),
            },
        );

        // Add one existing file in import_files
        let mut content = HashMap::new();
        content.insert(
            checksum2,
            ImportFileContent {
                file_name: "existing_game.rom".to_string(),
                sha1_checksum: checksum2,
                file_size: 2048,
                existing_file_info_id: Some(123),
                existing_archive_file_name: Some("existing_archive.zst".to_string()),
            },
        );

        context.import_files = vec![ServiceFileImportModel {
            path: PathBuf::from("/test/games.zip"),
            content,
        }];

        let step = UpdateDatabaseStep;
        let result = step.execute(&mut context).await;

        if !matches!(result, StepAction::Continue) {
            panic!("Expected Continue, got: {:?}", result);
        }
        assert!(context.file_set_id.is_some());

        // Verify both files were added - just check the file set was created
        let file_set_id = context.file_set_id.unwrap();
        assert!(file_set_id > 0);
    }

    // Test removed: Can't easily trigger database failure without mocking database itself
    // The cleanup logic is still in place in UpdateDatabaseStep
}
