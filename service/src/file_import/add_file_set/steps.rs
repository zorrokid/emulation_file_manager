use crate::{
    error::Error,
    file_import::{
        add_file_set::context::AddFileSetContext, common_steps::import::AddFileSetContextOps,
    },
    file_set_service::FileSetServiceOps,
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

pub struct UpdateDatabaseStep;

#[async_trait::async_trait]
impl PipelineStep<AddFileSetContext> for UpdateDatabaseStep {
    fn name(&self) -> &'static str {
        "update_database"
    }
    async fn execute(&self, context: &mut AddFileSetContext) -> StepAction {
        let files_in_file_set = context.get_files_in_file_set();
        if files_in_file_set.is_empty() {
            tracing::error!("No files in file set.");
            return StepAction::Abort(Error::FileImportError("No files in file set.".to_string()));
        }

        let file_type = context.get_file_import_model().file_type;
        let file_set_service = context.get_file_set_service();
        let file_set_service_result = file_set_service
            .create_file_set(context.to_create_file_set_params())
            .await;

        match file_set_service_result {
            Ok(res) => {
                let id = res.file_set_id;
                tracing::info!(
                    "File set '{}' with id {} added to database",
                    context.input.file_set_name,
                    id
                );
                context.state.file_set_id = Some(id);
                context.state.release_id = res.release_id;
            }
            Err(err) => {
                tracing::error!(
                    "Error adding file set '{}' to database: {}",
                    context.input.file_set_name,
                    err
                );

                for imported_file in context.state.imported_files.values() {
                    let file_path = context
                        .deps
                        .settings
                        .get_file_path(&file_type, &imported_file.archive_file_name);
                    if let Err(e) = context.ops.fs_ops.remove_file(&file_path) {
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

pub struct AddFileSetItemTypesStep;

#[async_trait::async_trait]
impl PipelineStep<AddFileSetContext> for AddFileSetItemTypesStep {
    fn name(&self) -> &'static str {
        "add_file_set_item_types"
    }

    fn should_execute(&self, context: &AddFileSetContext) -> bool {
        !context.state.item_types.is_empty() && context.state.file_set_id.is_some()
    }

    async fn execute(&self, context: &mut AddFileSetContext) -> StepAction {
        let file_set_id = context.state.file_set_id.unwrap();
        let res = context
            .deps
            .repository_manager
            .get_file_set_repository()
            .add_item_types_to_file_set(&file_set_id, &context.state.item_types)
            .await;

        match res {
            Ok(_) => tracing::info!("Item types added to file set"),
            Err(err) => {
                tracing::error!(error = %err,
                    "Add item types to file set operation failed.");
                // No point to abort here, add to failed steps and continue
                context.state.failed_steps.insert(
                    self.name().to_string(),
                    Error::DbError(format!("Error adding item types to file set: {}", err)),
                );
            }
        }

        StepAction::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_import::add_file_set::context::{
        AddFileSetDeps, AddFileSetInput, AddFileSetOps,
    };
    use crate::file_import::model::{FileImportData, FileImportSource, ImportFileContent};
    use crate::file_system_ops::mock::MockFileSystemOps;
    use core_types::item_type::ItemType;
    use core_types::{FileType, ImportedFile, Sha1Checksum};
    use database::{repository_manager::RepositoryManager, setup_test_db};
    use file_import::mock::MockFileImportOps;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::Arc;

    async fn create_test_context(file_import_data: Option<FileImportData>) -> AddFileSetContext {
        let pool = Arc::new(setup_test_db().await);
        let repository_manager = Arc::new(RepositoryManager::new(pool));
        let settings = Arc::new(crate::view_models::Settings::default());
        let file_system_ops = Arc::new(MockFileSystemOps::new());

        let ops = AddFileSetOps {
            file_import_ops: Arc::new(MockFileImportOps::new()),
            fs_ops: file_system_ops.clone(),
        };

        let input = AddFileSetInput {
            file_import_data: file_import_data.unwrap_or(create_file_import_data(vec![], vec![])),
            system_ids: vec![],
            source: "test_source".to_string(),
            file_set_name: "Test Game".to_string(),
            file_set_file_name: "test_game.zip".to_string(),
            create_release: false,
        };

        let deps = AddFileSetDeps {
            repository_manager: repository_manager.clone(),
            settings: settings.clone(),
        };

        AddFileSetContext::new(ops, deps, input)
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
        let checksum: Sha1Checksum = [1u8; 20];
        let file_import_data = create_file_import_data(vec![checksum], vec![]);
        let mut context = create_test_context(Some(file_import_data)).await;

        // Add system to database first
        let system_id = context
            .deps
            .repository_manager
            .get_system_repository()
            .add_system("Test System")
            .await
            .unwrap();

        context.input.system_ids = vec![system_id];

        context.state.imported_files.insert(
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
        assert!(context.state.file_set_id.is_some());
        assert!(context.state.file_set_id.unwrap() > 0);
    }

    #[async_std::test]
    async fn test_update_database_step_with_existing_files() {
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

        let mut context = create_test_context(Some(file_import_data)).await;
        // Add system to database first
        let system_id = context
            .deps
            .repository_manager
            .get_system_repository()
            .add_system("Test System")
            .await
            .unwrap();

        context.input.system_ids = vec![system_id];

        // Add one newly imported file
        context.state.imported_files.insert(
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
        assert!(context.state.file_set_id.is_some());

        // Verify both files were added - just check the file set was created
        let file_set_id = context.state.file_set_id.unwrap();
        assert!(file_set_id > 0);
    }

    #[async_std::test]
    async fn test_add_file_set_item_types_step() {
        let mut context = create_test_context(None).await;
        context.state.item_types = vec![ItemType::Manual, ItemType::Box];
        let file_set_id = context
            .deps
            .repository_manager
            .get_file_set_repository()
            .add_file_set("", "", &FileType::Rom, "", &[], &[])
            .await
            .unwrap();
        context.state.file_set_id = Some(file_set_id);

        let step = AddFileSetItemTypesStep;
        assert!(step.should_execute(&context));

        let res = step.execute(&mut context).await;
        assert_eq!(res, StepAction::Continue);

        let item_types_in_db = context
            .deps
            .repository_manager
            .get_file_set_repository()
            .get_item_types_for_file_set(file_set_id)
            .await
            .unwrap();
        assert_eq!(item_types_in_db.len(), 2);
        assert!(item_types_in_db.contains(&ItemType::Manual));
        assert!(item_types_in_db.contains(&ItemType::Box));
    }
}
