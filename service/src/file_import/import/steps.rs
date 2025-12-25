use crate::{
    error::Error,
    file_import::{common_steps::import::FileImportContextOps, import::context::FileImportContext},
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

pub struct UpdateDatabaseStep;

#[async_trait::async_trait]
impl PipelineStep<FileImportContext> for UpdateDatabaseStep {
    fn name(&self) -> &'static str {
        "update_database"
    }
    async fn execute(&self, context: &mut FileImportContext) -> StepAction {
        let files_in_file_set = context.get_files_in_file_set();
        if files_in_file_set.is_empty() {
            tracing::error!("No files in file set.");
            return StepAction::Abort(Error::FileImportError("No files in file set.".to_string()));
        }

        let file_type = context.get_file_import_model().file_type;
        match context
            .repository_manager
            .get_file_set_repository()
            .add_file_set(
                &context.file_set_name,
                &context.file_set_file_name,
                &file_type,
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
                        .get_file_path(&file_type, &imported_file.archive_file_name);
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
    use crate::file_import::model::{FileImportData, FileImportSource, ImportFileContent};
    use crate::file_system_ops::mock::MockFileSystemOps;
    use core_types::{FileType, ImportedFile, Sha1Checksum};
    use database::{repository_manager::RepositoryManager, setup_test_db};
    use file_import::mock::MockFileImportOps;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::Arc;

    async fn create_test_context(
        file_import_ops: Arc<MockFileImportOps>,
        file_import_data: FileImportData,
    ) -> FileImportContext {
        let pool = Arc::new(setup_test_db().await);
        let repository_manager = Arc::new(RepositoryManager::new(pool));
        let settings = Arc::new(crate::view_models::Settings::default());
        let file_system_ops = Arc::new(MockFileSystemOps::new());

        FileImportContext {
            repository_manager,
            settings,
            file_import_data,
            system_ids: vec![],
            source: "test_source".to_string(),
            file_set_name: "Test Game".to_string(),
            file_set_file_name: "test_game.zip".to_string(),
            imported_files: HashMap::new(),
            file_set_id: None,
            file_import_ops,
            file_system_ops,
            existing_files: vec![],
        }
    }

    fn create_file_import_data(
        selected_files: Vec<Sha1Checksum>,
        import_files: Vec<FileImportSource>,
    ) -> FileImportData {
        FileImportData {
            file_type: FileType::Rom,
            selected_files,
            output_dir: PathBuf::from("/imported/files"),
            import_files,
        }
    }

    #[async_std::test]
    async fn test_update_database_step_success() {
        let mock_ops = Arc::new(MockFileImportOps::new());
        let checksum: Sha1Checksum = [1u8; 20];
        let file_import_data = create_file_import_data(vec![checksum], vec![]);
        let mut context = create_test_context(mock_ops, file_import_data).await;

        // Add system to database first
        let system_id = context
            .repository_manager
            .get_system_repository()
            .add_system("Test System")
            .await
            .unwrap();

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
        let checksum1: Sha1Checksum = [1u8; 20];
        let checksum2: Sha1Checksum = [2u8; 20];

        // Add one existing file in import_files
        let mut content = HashMap::new();
        content.insert(
            checksum2,
            ImportFileContent {
                file_name: "existing_game.rom".to_string(),
                sha1_checksum: checksum2,
                file_size: 2048,
            },
        );

        let file_import_data = create_file_import_data(
            vec![checksum1, checksum2],
            vec![FileImportSource {
                path: PathBuf::from("/test/games.zip"),
                content,
            }],
        );

        let mut context = create_test_context(mock_ops, file_import_data).await;
        // Add system to database first
        let system_id = context
            .repository_manager
            .get_system_repository()
            .add_system("Test System")
            .await
            .unwrap();

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
}
