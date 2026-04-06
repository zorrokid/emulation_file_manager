use crate::{
    error::Error,
    file_import::{
        add_file_set::context::AddFileSetContext, common_steps::import::AddFileSetContextOps,
    },
    file_set::FileSetServiceOps,
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

/// Pipeline step that creates a new file set in the database:
/// - creates a file set record in the database
/// - links it to the release and dat file if needed
///
/// If the file set is successfully created, its ID is stored in the context for use in later steps.
///
/// If an error occurs, the step will attempt to clean up any imported files and abort the pipeline
/// with an appropriate error message.
pub struct CreateFileSetToDatabaseStep;

#[async_trait::async_trait]
impl PipelineStep<AddFileSetContext> for CreateFileSetToDatabaseStep {
    fn name(&self) -> &'static str {
        "update_database"
    }

    fn should_execute(&self, context: &AddFileSetContext) -> bool {
        context.state.file_set_id.is_none()
    }

    async fn execute(&self, context: &mut AddFileSetContext) -> StepAction {
        let files_in_file_set = context.get_files_in_file_set();
        if files_in_file_set.is_empty() {
            tracing::error!("No files in file set.");
            return StepAction::Abort(Error::FileImportError("No files in file set.".to_string()));
        }

        // TODO: check if we already have file set id set from previous step check
        // maybe add file set id to CreateFileSetParams
        // - check if release and software title creation is needed
        // - check if dat file linking is needed
        // The point of those all being in the same service is that we want to do all that in a
        // single transaction and roll back if any of it fails, otherwise we might end up with a
        // release without a file set or a file set without files, etc.
        // TODO: if file set exists, we should still check that it's linked to dat file if dat file
        // id is provided.
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

                for imported_file in context
                    .state
                    .imported_files
                    .values()
                    .filter(|f| f.is_available)
                {
                    if let Some(archive_name) = &imported_file.archive_file_name {
                        let file_path = context
                            .deps
                            .settings
                            .get_file_path(&file_type, archive_name);
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
                    } else {
                        tracing::warn!(
                            file_name = %imported_file.original_file_name,
                            "Imported file does not have an archive name although marked as available, skipping deletion after database failure",
                        );
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
        // check existing linking
        let existing_item_types = context
            .deps
            .repository_manager
            .get_file_set_repository()
            .get_item_types_for_file_set(file_set_id)
            .await;

        match existing_item_types {
            Ok(existing) => {
                let new_item_types: Vec<_> = context
                    .state
                    .item_types
                    .iter()
                    .filter(|it| !existing.contains(it))
                    .cloned()
                    .collect();

                if new_item_types.is_empty() {
                    tracing::info!("No new item types to add to file set");
                    return StepAction::Continue;
                }

                let res = context
                    .deps
                    .repository_manager
                    .get_file_set_repository()
                    .add_item_types_to_file_set(&file_set_id, &new_item_types)
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
            }
            Err(err) => {
                tracing::error!(error = %err,
                    "Error checking existing item types for file set, aborting add item types step.");
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
    use crate::file_set::mock_file_set_service::MockFileSetService;
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

        let file_set_service_ops = Arc::new(MockFileSetService::new());

        let ops = AddFileSetOps {
            file_import_ops: Arc::new(MockFileImportOps::new()),
            fs_ops: file_system_ops.clone(),
            file_set_service_ops,
        };

        let input = AddFileSetInput {
            file_import_data: file_import_data.unwrap_or(create_file_import_data(vec![], vec![])),
            system_ids: vec![],
            source: "test_source".to_string(),
            file_set_name: "Test Game".to_string(),
            file_set_file_name: "test_game.zip".to_string(),
            create_release: None,
            dat_file_id: None,
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
            missing_files: vec![],
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
                archive_file_name: Some("archive123.zst".to_string()),
                is_available: true,
            },
        );

        let step = CreateFileSetToDatabaseStep;
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
                archive_file_name: Some("new_archive.zst".to_string()),
                is_available: true,
            },
        );

        let step = CreateFileSetToDatabaseStep;
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

    #[async_std::test]
    async fn test_same_sha1_imported_for_two_file_sets_creates_single_file_info_record() {
        // Arrange: a file_info already exists in DB (from a previous import of file set A)
        let sha1_shared: Sha1Checksum = [42u8; 20];
        let mut context = create_test_context(None).await;
        let repo = context.deps.repository_manager.clone();

        let existing_id = repo
            .get_file_info_repository()
            .add_file_info(
                &sha1_shared,
                2048,
                Some("archive_from_set_a.zst"),
                FileType::Rom,
            )
            .await
            .unwrap();

        // Act: run CheckExistingFilesStep for file set B importing the same SHA1
        let file_import_data = create_file_import_data(
            vec![sha1_shared],
            vec![FileImportSource {
                path: PathBuf::from("/roms/game.zip"),
                content: [(
                    sha1_shared,
                    ImportFileContent {
                        file_name: "game.rom".to_string(),
                        sha1_checksum: sha1_shared,
                        file_size: 2048,
                    },
                )]
                .into_iter()
                .collect(),
            }],
        );
        context.input.file_import_data = file_import_data;

        let step = crate::file_import::common_steps::check_existing_files::CheckExistingFilesStep::<
            AddFileSetContext,
        >::new();
        let action = step.execute(&mut context).await;

        // Assert: step continues and the existing file_info is found
        assert!(matches!(action, StepAction::Continue));
        assert_eq!(context.state.existing_files.len(), 1);
        assert_eq!(context.state.existing_files[0].id, existing_id);
        assert_eq!(context.state.existing_files[0].sha1_checksum, sha1_shared);

        // Assert: no new import needed — the SHA1 is already in the DB
        assert!(
            !context
                .input
                .file_import_data
                .is_new_files_to_be_imported(&context.state.existing_files),
            "Expected no new files to import since SHA1 already exists in DB"
        );

        // Assert: DB still has exactly one file_info record for this SHA1
        let infos = repo
            .get_file_info_repository()
            .get_file_infos_by_sha1_checksums(&[sha1_shared], FileType::Rom)
            .await
            .unwrap();
        assert_eq!(
            infos.len(),
            1,
            "Expected exactly one file_info row for SHA1 — no duplicates should be created"
        );
    }

    #[async_std::test]
    async fn test_create_file_set_database_step_cleanup_skips_when_archive_file_name_missing() {
        // Arrange: trigger a real DB failure via a non-existent system_id (FK constraint violation).
        // The imported file has is_available=true with archive_file_name=None — an invariant
        // violation. The cleanup path should warn and skip it rather than attempt FS deletion.
        let fs_ops = Arc::new(MockFileSystemOps::new());
        let checksum: Sha1Checksum = [1u8; 20];

        let mut context = create_test_context(Some(create_file_import_data(
            vec![checksum],
            vec![],
        )))
        .await;

        // Wire in the fs_ops we can inspect, and point to a non-existent system (FK failure)
        context.ops.fs_ops = fs_ops.clone();
        context.input.system_ids = vec![999];
        context.state.imported_files.insert(
            checksum,
            ImportedFile {
                original_file_name: "missing.rom".to_string(),
                sha1_checksum: checksum,
                file_size: 0,
                archive_file_name: None,
                is_available: true,
            },
        );

        // Act
        let step = CreateFileSetToDatabaseStep;
        let result = step.execute(&mut context).await;

        // Assert: step aborts (DB failed) but no FS deletion was attempted
        assert!(
            matches!(result, StepAction::Abort(_)),
            "Expected Abort due to DB failure, got: {:?}",
            result
        );
        assert_eq!(
            fs_ops.get_deleted_files(),
            vec![] as Vec<String>,
            "No files should be deleted when archive_file_name is None"
        );
    }
}
