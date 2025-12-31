use database::models::FileInfo;

use crate::{
    error::Error,
    file_import::{
        common_steps::import::AddFileSetContextOps, update_file_set::context::UpdateFileSetContext,
    },
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

pub struct FetchFileSetStep;

#[async_trait::async_trait]
impl PipelineStep<UpdateFileSetContext> for FetchFileSetStep {
    fn name(&self) -> &'static str {
        "fetch_file_set"
    }

    async fn execute(&self, context: &mut UpdateFileSetContext) -> StepAction {
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
                context.files_in_file_set = files
                    .iter()
                    .map(|file| {
                        file.sha1_checksum
                            .clone()
                            .try_into()
                            .expect("Invalid SHA1 checksum length")
                    })
                    .collect();
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

pub struct RemovedFilesStep;

#[async_trait::async_trait]
impl PipelineStep<UpdateFileSetContext> for RemovedFilesStep {
    fn name(&self) -> &'static str {
        "removed_files"
    }

    fn should_execute(&self, context: &UpdateFileSetContext) -> bool {
        context.has_removed_files()
    }

    async fn execute(&self, _context: &mut UpdateFileSetContext) -> StepAction {
        // TODO: check if there are files to be deleted
        // - if file is not part of any other file set, the actual file should be deleted too and
        // if cloud sync is enabled, the file should be marked to be deleted from cloud storage too
        // - file_info should be unlinked from file_set
        // - file_info entry should be deleted from database
        // - maybe deletion could bne done in a separate step
        //
        StepAction::Continue
    }
}

pub struct UpdateFileSetStep;

#[async_trait::async_trait]
impl PipelineStep<UpdateFileSetContext> for UpdateFileSetStep {
    fn name(&self) -> &'static str {
        "update_file_set"
    }

    async fn execute(&self, context: &mut UpdateFileSetContext) -> StepAction {
        let file_info_ids_with_file_names = context.get_file_info_ids_with_file_names();
        let result = context
            .repository_manager
            .get_file_set_repository()
            .add_files_to_file_set(context.file_set_id, &file_info_ids_with_file_names)
            .await;

        match result {
            Ok(_) => {
                tracing::info!(
                    file_set_id = %context.file_set_id,
                    added_file_count = file_info_ids_with_file_names.len(),
                    "Added files to file set in database"
                );
            }
            Err(err) => {
                tracing::error!(
                    error = %err,
                    file_set_id = %context.file_set_id,
                    "Error adding files to file set in database"
                );
                // TODO: should files and file infos be removed?
                return StepAction::Abort(Error::DbError(format!(
                    "Error adding files to file set: {}",
                    err,
                )));
            }
        }
        StepAction::Continue
    }
}

pub struct MarkFilesForCloudSyncStep;

#[async_trait::async_trait]
impl PipelineStep<UpdateFileSetContext> for MarkFilesForCloudSyncStep {
    fn name(&self) -> &'static str {
        "mark_files_for_cloud_sync"
    }

    fn should_execute(&self, context: &UpdateFileSetContext) -> bool {
        context.settings.s3_sync_enabled && !context.imported_files.is_empty()
    }
    async fn execute(&self, context: &mut UpdateFileSetContext) -> StepAction {
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
    use database::{repository_manager::RepositoryManager, setup_test_db};
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
        }
    }

    #[async_std::test]
    async fn test_fetch_file_set_step() {
        let file_1_checksum: Sha1Checksum = [1u8; 20];
        // Add file to test db
        let file_system_ops = Arc::new(MockFileSystemOps::new());
        let path = "/test/games.zip".to_string();
        file_system_ops.add_file(path.clone());
        let mut context = create_test_context(Some(file_system_ops)).await;
        let repository_manager = context.repository_manager.clone();
        let file_info_1_id = repository_manager
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
        let step = super::FetchFileSetStep;
        let action = step.execute(&mut context).await;
        assert!(matches!(action, StepAction::Continue));
        assert!(context.file_set.is_some());
    }

    #[async_std::test]
    async fn test_fetch_files_in_file_set_step() {
        let file_1_checksum: Sha1Checksum = [1u8; 20];
        // Add file to test db
        let file_system_ops = Arc::new(MockFileSystemOps::new());
        let path = "/test/games.zip".to_string();
        file_system_ops.add_file(path.clone());
        let mut context = create_test_context(Some(file_system_ops)).await;
        let repository_manager = context.repository_manager.clone();
        let file_info_1_id = repository_manager
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
        let step = super::FetchFilesInFileSetStep;
        let action = step.execute(&mut context).await;
        assert!(matches!(action, StepAction::Continue));
        assert!(context.files_in_file_set.contains(&file_1_checksum));
    }
}
