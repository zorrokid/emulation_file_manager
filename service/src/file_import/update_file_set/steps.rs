use core_types::Sha1Checksum;
use database::models::FileInfo;

use crate::{
    error::Error,
    file_import::{
        common_steps::import::AddFileSetContextOps, update_file_set::context::UpdateFileSetContext,
    },
    file_set_deletion::model::FileDeletionResult,
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

pub struct FetchFileSetStep;

#[async_trait::async_trait]
impl PipelineStep<UpdateFileSetContext> for FetchFileSetStep {
    fn name(&self) -> &'static str {
        "fetch_file_set"
    }

    async fn execute(&self, context: &mut UpdateFileSetContext) -> StepAction {
        println!("Fetching file set with id {}", context.file_set_id);
        // TODO: add test with non existing file set
        let file_set_result = context
            .repository_manager
            .get_file_set_repository()
            .get_file_set(context.file_set_id)
            .await;

        match file_set_result {
            Ok(file_set) => {
                tracing::info!(
                    file_set_id = %context.file_set_id,
                    "File set exists in database"
                );
                context.file_set = Some(file_set);
            }
            Err(err) => {
                tracing::error!(
                    error = %err,
                    file_set_id = %context.file_set_id,
                    "Error checking if file set exists in database"
                );
                return StepAction::Abort(Error::DbError(format!(
                    "Error checking if file set exists: {}",
                    err,
                )));
            }
        }

        StepAction::Continue
    }
}

pub struct FetchFilesInFileSetStep;
#[async_trait::async_trait]
impl PipelineStep<UpdateFileSetContext> for FetchFilesInFileSetStep {
    fn name(&self) -> &'static str {
        "fetch_files_in_file_set"
    }

    async fn execute(&self, context: &mut UpdateFileSetContext) -> StepAction {
        println!("Fetching files in file set with id {}", context.file_set_id);
        let files_result = context
            .repository_manager
            .get_file_info_repository()
            .get_file_infos_by_file_set(context.file_set_id)
            .await;

        match files_result {
            Ok(files) => {
                tracing::info!(
                    file_set_id = %context.file_set_id,
                    file_count = files.len(),
                    "Fetched files in file set from database"
                );
                context.files_in_file_set = files;
            }
            Err(err) => {
                tracing::error!(
                    error = %err,
                    file_set_id = %context.file_set_id,
                    "Error fetching files in file set from database"
                );
                return StepAction::Abort(Error::DbError(format!(
                    "Error fetching files in file set: {}",
                    err,
                )));
            }
        }

        StepAction::Continue
    }
}

pub struct UpdateFileInfoToDatabaseStep;

#[async_trait::async_trait]
impl PipelineStep<UpdateFileSetContext> for UpdateFileInfoToDatabaseStep {
    fn name(&self) -> &'static str {
        "add_file_info_to_database"
    }

    fn should_execute(&self, context: &UpdateFileSetContext) -> bool {
        context.is_new_files_to_be_imported()
            && !context.imported_files.is_empty()
            && context.file_set.is_some()
    }

    async fn execute(&self, context: &mut UpdateFileSetContext) -> StepAction {
        println!("Adding file info records to database...");
        let file_type = context.file_set.as_ref().unwrap().file_type;
        for imported_file in context.imported_files.values() {
            let add_file_info_result = context
                .repository_manager
                .get_file_info_repository()
                .add_file_info(
                    &imported_file.sha1_checksum,
                    imported_file.file_size as i64,
                    &imported_file.archive_file_name,
                    file_type,
                )
                .await;
            match add_file_info_result {
                Ok(id) => {
                    println!(
                        "Added file info record with id {} for file '{}'",
                        id, imported_file.archive_file_name
                    );
                    tracing::info!(
                        file_count = context.imported_files.len(),
                        "Added file info records to database"
                    );
                    context.new_files.push(FileInfo {
                        id,
                        sha1_checksum: imported_file.sha1_checksum.into(),
                        file_size: imported_file.file_size,
                        archive_file_name: imported_file.archive_file_name.clone(),
                        file_type,
                    });
                }
                Err(err) => {
                    println!(
                        "Error adding file info record for file '{}': {}",
                        imported_file.archive_file_name, err
                    );
                    tracing::error!(
                        error = %err,
                        "Error adding file info records to database"
                    );
                    // TODO: collect failed
                }
            }
        }
        StepAction::Continue
    }
}

pub struct UpdateFileSetFilesStep;

#[async_trait::async_trait]
impl PipelineStep<UpdateFileSetContext> for UpdateFileSetFilesStep {
    fn name(&self) -> &'static str {
        "update_file_set_files"
    }

    async fn execute(&self, context: &mut UpdateFileSetContext) -> StepAction {
        println!("Adding files to file set with id {}", context.file_set_id);
        let file_info_ids_with_file_names = context.get_file_info_ids_with_file_names();
        let result = context
            .repository_manager
            .get_file_set_repository()
            .add_files_to_file_set(context.file_set_id, &file_info_ids_with_file_names)
            .await;

        match result {
            Ok(_) => {
                println!(
                    "Added {} files to file set in database",
                    file_info_ids_with_file_names.len()
                );
                tracing::info!(
                    file_set_id = %context.file_set_id,
                    added_file_count = file_info_ids_with_file_names.len(),
                    "Added files to file set in database"
                );

                StepAction::Continue
            }
            Err(err) => {
                println!("Error adding files to file set in database: {}", err);
                tracing::error!(
                    error = %err,
                    file_set_id = %context.file_set_id,
                    "Error adding files to file set in database"
                );
                // TODO: should files and file infos be removed?
                StepAction::Abort(Error::DbError(format!(
                    "Error adding files to file set: {}",
                    err,
                )))
            }
        }
    }
}

pub struct CollectDeletionCandidatesStep;

#[async_trait::async_trait]
impl PipelineStep<UpdateFileSetContext> for CollectDeletionCandidatesStep {
    fn name(&self) -> &'static str {
        "collect_deletion_candidates"
    }

    fn should_execute(&self, context: &UpdateFileSetContext) -> bool {
        context.has_removed_files()
    }

    async fn execute(&self, context: &mut UpdateFileSetContext) -> StepAction {
        println!(
            "Collecting files unlinked from file set with id {} for deletion candidates",
            context.file_set_id
        );
        let removed_files = context
            .get_removed_files()
            .iter()
            .map(|file| file.sha1_checksum)
            .collect::<Vec<Sha1Checksum>>();

        let repository = context.repository_manager.get_file_info_repository();
        let result = repository
            .get_file_infos_by_sha1_checksums(
                &removed_files,
                context.file_set.as_ref().unwrap().file_type,
            )
            .await;
        match result {
            Ok(files) => {
                tracing::info!(
                    file_set_id = %context.file_set_id,
                    removed_file_count = files.len(),
                    "Collected files unlinked from file set for deletion candidates"
                );
                context.deletion_results = files
                    .into_iter()
                    .map(|file| (file.sha1_checksum, FileDeletionResult::new(file)))
                    .collect();
            }
            Err(err) => {
                tracing::error!(
                    error = %err,
                    file_set_id = %context.file_set_id,
                    "Error collecting files unlinked from file set for deletion candidates"
                );
                return StepAction::Abort(Error::DbError(format!(
                    "Error collecting files unlinked from file set for deletion candidates: {}",
                    err,
                )));
            }
        }
        StepAction::Continue
    }
}

pub struct UnlinkFilesFromFileSetStep;

#[async_trait::async_trait]
impl PipelineStep<UpdateFileSetContext> for UnlinkFilesFromFileSetStep {
    fn name(&self) -> &'static str {
        "unlink_files_from_file_set"
    }

    fn should_execute(&self, context: &UpdateFileSetContext) -> bool {
        context.has_removed_files()
    }

    async fn execute(&self, context: &mut UpdateFileSetContext) -> StepAction {
        println!(
            "Unlinking files from file set with id {}",
            context.file_set_id
        );
        let removed_files = context
            .get_removed_files()
            .iter()
            .map(|file| file.id)
            .collect::<Vec<i64>>();

        let result = context
            .repository_manager
            .get_file_set_repository()
            .remove_files_from_file_set(context.file_set_id, &removed_files)
            .await;

        match result {
            Ok(_) => {
                tracing::info!(
                    file_set_id = %context.file_set_id,
                    removed_file_count = removed_files.len(),
                    "Unlinked files from file set in database"
                );
                StepAction::Continue
            }
            Err(err) => {
                tracing::error!(
                    error = %err,
                    file_set_id = %context.file_set_id,
                    "Error unlinking files from file set in database"
                );
                StepAction::Abort(Error::DbError(format!(
                    "Error unlinking files from file set: {}",
                    err,
                )))
            }
        }
    }
}

pub struct UpdateFileSetStep;

#[async_trait::async_trait]
impl PipelineStep<UpdateFileSetContext> for UpdateFileSetStep {
    fn name(&self) -> &'static str {
        "update_file_set"
    }

    fn should_execute(&self, context: &UpdateFileSetContext) -> bool {
        context.file_set.is_some()
    }

    async fn execute(&self, context: &mut UpdateFileSetContext) -> StepAction {
        let repository = context.repository_manager.get_file_set_repository();
        let original_file_type = context.file_set.as_ref().unwrap().file_type;
        let result = repository
            .update_file_set(
                context.file_set_id,
                &context.file_set_file_name,
                &context.file_set_name,
                &context.source,
                // TODO: currently we cannot update the file type because of the folder structure
                // in local storage and S3. We need to implement a way to move files when the file
                // type is changed. For now, we just keep the original file type and if file type
                // is needed to change the file set need to be deleted and re-imported.
                &original_file_type, // &context.file_import_data.file_type,
            )
            .await;
        match result {
            Ok(_) => {
                tracing::info!(
                    file_set_id = %context.file_set_id,
                    "Updated file set metadata in database"
                );
                StepAction::Continue
            }
            Err(err) => {
                tracing::error!(
                    error = %err,
                    file_set_id = %context.file_set_id,
                    "Error updating file set metadata in database"
                );
                StepAction::Abort(Error::DbError(format!("Error updating file set: {}", err,)))
            }
        }
    }
}

pub struct MarkNewFilesForCloudSyncStep;

#[async_trait::async_trait]
impl PipelineStep<UpdateFileSetContext> for MarkNewFilesForCloudSyncStep {
    fn name(&self) -> &'static str {
        "mark_new_files_for_cloud_sync"
    }

    fn should_execute(&self, context: &UpdateFileSetContext) -> bool {
        context.settings.s3_sync_enabled && !context.imported_files.is_empty()
    }
    async fn execute(&self, context: &mut UpdateFileSetContext) -> StepAction {
        println!("Marking new files for cloud sync...");
        let file_info_ids: Vec<(i64, String)> = context
            .new_files
            .iter()
            .map(|file| (file.id, file.generate_cloud_key()))
            .collect();

        let result = context
            .repository_manager
            .get_file_sync_log_repository()
            .mark_files_for_cloud_sync(&file_info_ids)
            .await;

        match result {
            Ok(_) => {
                tracing::info!(
                    file_count = file_info_ids.len(),
                    "Marked files for cloud sync"
                );
            }
            Err(err) => {
                tracing::error!(
                    error = %err,
                    "Error marking files for cloud sync"
                );

                // No point aborting here, the import was successful and new files are also marked
                // for syncing when cloud synd is triggered
            }
        }

        StepAction::Continue
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf, sync::Arc};

    use core_types::{FileType, ImportedFile, Sha1Checksum};
    use database::{models::FileSet, repository_manager::RepositoryManager, setup_test_db};
    use file_import::mock::MockFileImportOps;

    use crate::{
        file_import::{
            model::{FileImportData, FileImportSource, ImportFileContent},
            update_file_set::context::UpdateFileSetContext,
        },
        file_system_ops::mock::MockFileSystemOps,
        pipeline::pipeline_step::{PipelineStep, StepAction},
    };

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

    async fn create_test_context(
        file_system_ops: Option<Arc<MockFileSystemOps>>,
    ) -> UpdateFileSetContext {
        let pool = Arc::new(setup_test_db().await);
        let repository_manager = Arc::new(RepositoryManager::new(pool));
        let settings = Arc::new(crate::view_models::Settings::default());
        let file_system_ops = file_system_ops.unwrap_or(Arc::new(MockFileSystemOps::new()));
        let file_import_ops = Arc::new(MockFileImportOps::new());
        let file_import_data = create_file_import_data(vec![], vec![]);

        UpdateFileSetContext {
            repository_manager,
            settings,
            fs_ops: file_system_ops,
            file_import_ops,
            file_import_data,
            file_set_id: 0,
            imported_files: HashMap::new(),
            existing_files: Vec::new(),
            new_files: Vec::new(),
            file_set: None,
            files_in_file_set: Vec::new(),
            file_set_name: "Test File Set".to_string(),
            file_set_file_name: "test_game".to_string(),
            source: "test_source".to_string(),
            deletion_results: HashMap::new(),
        }
    }

    async fn create_context_and_test_file_set() -> (UpdateFileSetContext, Sha1Checksum) {
        let file_1_checksum: Sha1Checksum = [1u8; 20];
        // Add file to test db
        let file_system_ops = Arc::new(MockFileSystemOps::new());
        let path = "/test/games.zip".to_string();
        file_system_ops.add_file(path.clone());
        let mut context = create_test_context(Some(file_system_ops)).await;
        let repository_manager = context.repository_manager.clone();
        let _file_info_1_id = repository_manager
            .get_file_info_repository()
            .add_file_info(&file_1_checksum, 1024, "test_archive_name_1", FileType::Rom)
            .await
            .unwrap();

        let system_id = repository_manager
            .get_system_repository()
            .add_system("Test System")
            .await
            .unwrap();

        let files_in_file_set = vec![ImportedFile {
            original_file_name: "original file name".to_string(),
            archive_file_name: "archive_file_name".to_string(),
            sha1_checksum: file_1_checksum,
            file_size: 1024,
        }];

        let file_set_id = repository_manager
            .get_file_set_repository()
            .add_file_set(
                "Test File Set",
                "test_game",
                &FileType::Rom,
                "test_source",
                &files_in_file_set,
                &[system_id],
            )
            .await
            .unwrap();

        context.file_set_id = file_set_id;

        let file_import_data = FileImportData::new(FileType::Rom, PathBuf::from("/imported/files"))
            .with_selected_file(file_1_checksum)
            .with_file_import_source(FileImportSource::new(PathBuf::from(path)).with_content(
                ImportFileContent {
                    file_name: "game1.rom".to_string(),
                    sha1_checksum: file_1_checksum,
                    file_size: 1024,
                },
            ));

        context.file_import_data = file_import_data;
        (context, file_1_checksum)
    }

    #[async_std::test]
    async fn test_fetch_file_set_step() {
        let (mut context, _) = create_context_and_test_file_set().await;
        let step = super::FetchFileSetStep;
        let action = step.execute(&mut context).await;
        assert!(matches!(action, StepAction::Continue));
        assert!(context.file_set.is_some());
    }

    #[async_std::test]
    async fn test_fetch_files_in_file_set_step() {
        let (mut context, file_1_checksum) = create_context_and_test_file_set().await;
        let step = super::FetchFilesInFileSetStep;
        let action = step.execute(&mut context).await;
        assert!(matches!(action, StepAction::Continue));
        assert!(
            context
                .files_in_file_set
                .iter()
                .any(|f| f.sha1_checksum == file_1_checksum)
        );
    }

    #[async_std::test]
    async fn test_update_file_info_to_database_step() {
        let mut context = create_test_context(None).await;
        context.file_set = Some(FileSet {
            id: 1,
            name: "Test File Set".to_string(),
            file_name: "test_game".to_string(),
            file_type: FileType::Rom,
            source: "test_source".to_string(),
        });

        let file_1_checksum: Sha1Checksum = [1u8; 20];

        context.imported_files.insert(
            file_1_checksum,
            ImportedFile {
                original_file_name: "game1.rom".to_string(),
                sha1_checksum: file_1_checksum,
                file_size: 1024,
                archive_file_name: "archive123.zst".to_string(),
            },
        );

        let step = super::UpdateFileInfoToDatabaseStep;
        let action = step.execute(&mut context).await;
        assert!(matches!(action, StepAction::Continue));
        assert!(!context.new_files.is_empty());
        assert_eq!(context.new_files[0].sha1_checksum, file_1_checksum);
    }

    #[async_std::test]
    async fn test_update_file_set_files_step() {
        let (mut context, file_1_checksum) = create_context_and_test_file_set().await;

        // Fetch the existing file info for file_1
        let file_1_infos = context
            .repository_manager
            .get_file_info_repository()
            .get_file_infos_by_sha1_checksums(&[file_1_checksum], FileType::Rom)
            .await
            .unwrap();

        // Add file_1 to existing_files
        context.existing_files = file_1_infos;

        // Add a new file to be imported
        let file_2_checksum: Sha1Checksum = [2u8; 20];
        let file_2_id = context
            .repository_manager
            .get_file_info_repository()
            .add_file_info(&file_2_checksum, 2048, "test_archive_name_2", FileType::Rom)
            .await
            .unwrap();

        context.new_files.push(database::models::FileInfo {
            id: file_2_id,
            sha1_checksum: file_2_checksum.into(),
            file_size: 2048,
            archive_file_name: "test_archive_name_2".to_string(),
            file_type: FileType::Rom,
        });

        // Update file_import_data to only include the new file (file_2)
        // file_1 is already in the file set
        context.file_import_data =
            FileImportData::new(FileType::Rom, PathBuf::from("/imported/files"))
                .with_selected_file(file_2_checksum)
                .with_file_import_source(
                    FileImportSource::new(PathBuf::from("/test/games.zip")).with_content(
                        ImportFileContent {
                            file_name: "game2.rom".to_string(),
                            sha1_checksum: file_2_checksum,
                            file_size: 2048,
                        },
                    ),
                );

        let step = super::UpdateFileSetFilesStep;
        let action = step.execute(&mut context).await;
        assert!(matches!(action, StepAction::Continue));

        // Verify the new file was added to the file set (total should be 2)
        let files_in_set = context
            .repository_manager
            .get_file_info_repository()
            .get_file_infos_by_file_set(context.file_set_id)
            .await
            .unwrap();
        assert_eq!(files_in_set.len(), 2);
        assert!(
            files_in_set
                .iter()
                .any(|f| f.sha1_checksum == file_1_checksum)
        );
        assert!(
            files_in_set
                .iter()
                .any(|f| f.sha1_checksum == file_2_checksum)
        );
    }

    #[async_std::test]
    async fn test_collect_deletion_candidates_step() {
        let (mut context, file_1_checksum) = create_context_and_test_file_set().await;

        // Fetch file set
        let step_fetch_set = super::FetchFileSetStep;
        let _ = step_fetch_set.execute(&mut context).await;

        // Fetch files currently in file set
        let step_fetch = super::FetchFilesInFileSetStep;
        let _ = step_fetch.execute(&mut context).await;

        // Swet file_import_data without selected files (simulating removal)
        context.file_import_data =
            FileImportData::new(FileType::Rom, PathBuf::from("/imported/files"));

        let step = super::CollectDeletionCandidatesStep;

        // Verify should_execute returns true when there are removed files
        assert!(step.should_execute(&context));

        let action = step.execute(&mut context).await;
        assert!(matches!(action, StepAction::Continue));

        // Verify deletion results were collected
        assert_eq!(context.deletion_results.len(), 1);
        assert!(context.deletion_results.contains_key(&file_1_checksum));
    }

    #[async_std::test]
    async fn test_unlink_files_from_file_set_step() {
        let (mut context, file_1_checksum) = create_context_and_test_file_set().await;

        // Fetch files currently in file set
        let step_fetch = super::FetchFilesInFileSetStep;
        let _ = step_fetch.execute(&mut context).await;

        // Update file_import_data to have no selected files (simulating removal)
        context.file_import_data =
            FileImportData::new(FileType::Rom, PathBuf::from("/imported/files"));

        let step = super::UnlinkFilesFromFileSetStep;

        // Verify should_execute returns true when there are removed files
        assert!(step.should_execute(&context));

        let action = step.execute(&mut context).await;
        assert!(matches!(action, StepAction::Continue));

        // Verify files were removed from the file set
        let files_in_set = context
            .repository_manager
            .get_file_info_repository()
            .get_file_infos_by_file_set(context.file_set_id)
            .await
            .unwrap();
        assert_eq!(files_in_set.len(), 0);

        // Verify the file info still exists in database (only unlinked from file set)
        let file_info = context
            .repository_manager
            .get_file_info_repository()
            .get_file_infos_by_sha1_checksums(&[file_1_checksum], FileType::Rom)
            .await
            .unwrap();
        assert_eq!(file_info.len(), 1);
    }

    #[async_std::test]
    async fn test_update_file_set_step() {
        let (mut context, _) = create_context_and_test_file_set().await;

        // Update file set metadata
        context.file_set_name = "Updated File Set Name".to_string();
        context.file_set_file_name = "updated_game".to_string();
        context.source = "updated_source".to_string();
        context.file_import_data.file_type = FileType::Rom;

        let step = super::UpdateFileSetStep;
        let action = step.execute(&mut context).await;
        assert!(matches!(action, StepAction::Continue));

        // Verify file set was updated
        let updated_file_set = context
            .repository_manager
            .get_file_set_repository()
            .get_file_set(context.file_set_id)
            .await
            .unwrap();

        assert_eq!(updated_file_set.file_name, "updated_game");
        assert_eq!(updated_file_set.name, "Updated File Set Name");
        assert_eq!(updated_file_set.source, "updated_source");
        assert_eq!(updated_file_set.file_type, FileType::Rom);
    }
}
