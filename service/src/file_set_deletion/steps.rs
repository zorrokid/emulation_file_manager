use crate::{
    error::Error,
    file_set_deletion::{context::DeletionContext, model::FileDeletionResult},
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

/// Step 1: Validate that the file set is not in use by any releases
pub struct ValidateFileSetNotInUseStep;

#[async_trait::async_trait]
impl PipelineStep<DeletionContext> for ValidateFileSetNotInUseStep {
    fn name(&self) -> &'static str {
        "validate_file_set_not_in_use"
    }

    async fn execute(&self, context: &mut DeletionContext) -> StepAction {
        tracing::info!(
            "Validating that file set with id {} is not in use",
            context.file_set_id
        );

        let file_set_id = context.file_set_id;

        let is_in_use_res = context
            .repository_manager
            .get_file_set_repository()
            .is_in_use(file_set_id)
            .await;
        match is_in_use_res {
            Err(e) => StepAction::Abort(Error::DbError(format!(
                "Failed to check if file set with id {} is in use: {}",
                file_set_id, e
            ))),
            Ok(in_use) => {
                if in_use {
                    tracing::warn!(
                        "File set with id {} is in use by one or more releases, aborting deletion",
                        file_set_id
                    );
                    StepAction::Abort(Error::DbError(
                        "File set is in use by one or more releases".to_string(),
                    ))
                } else {
                    tracing::info!(
                        "File set with id {} is not in use, proceeding with deletion",
                        file_set_id
                    );
                    StepAction::Continue
                }
            }
        }
    }
}

/// Step 2: Fetch all file infos for the file set
pub struct FetchFileInfosStep;

#[async_trait::async_trait]
impl PipelineStep<DeletionContext> for FetchFileInfosStep {
    fn name(&self) -> &'static str {
        "fetch_file_infos"
    }

    async fn execute(&self, context: &mut DeletionContext) -> StepAction {
        tracing::info!(
            "Fetching file infos for file set with id {}",
            context.file_set_id
        );

        let file_infos_res = context
            .repository_manager
            .get_file_info_repository()
            .get_file_infos_by_file_set(context.file_set_id)
            .await;

        match file_infos_res {
            Ok(file_infos) => {
                tracing::info!(
                    "Fetched {} file infos for file set {}",
                    file_infos.len(),
                    context.file_set_id
                );
                // even if file_infos is empty, continue to delete the file set
                context.deletion_results = file_infos
                    .into_iter()
                    .map(|fi| {
                        tracing::info!("Found file info with id {} for deletion", fi.id);
                        (fi.sha1_checksum.clone(), FileDeletionResult::new(fi))
                    })
                    .collect();

                StepAction::Continue
            }
            Err(e) => {
                tracing::error!(
                    "Failed to fetch file infos for file set {}: {}",
                    context.file_set_id,
                    e
                );
                StepAction::Abort(Error::DbError(format!("Failed to fetch file infos: {}", e)))
            }
        }
    }
}

/// Step 4: Delete the file set from database
pub struct DeleteFileSetStep;

#[async_trait::async_trait]
impl PipelineStep<DeletionContext> for DeleteFileSetStep {
    fn name(&self) -> &'static str {
        "delete_file_set"
    }

    async fn execute(&self, context: &mut DeletionContext) -> StepAction {
        tracing::info!("Deleting file set with id {}", context.file_set_id);

        let res = context
            .repository_manager
            .get_file_set_repository()
            .delete_file_set(context.file_set_id)
            .await;

        match res {
            Ok(_) => {
                if context.deletion_results.is_empty() {
                    tracing::info!(
                        "Deleted file set {} from database. No files associated with file set, skipping remaining steps",
                        context.file_set_id
                    );
                    // No files to process, can skip remaining steps
                    StepAction::Skip
                } else {
                    tracing::info!(
                        "Deleted file set {} from database, proceeding with file deletions",
                        context.file_set_id
                    );
                    StepAction::Continue
                }
            }
            Err(e) => {
                tracing::error!(
                    "Failed to delete file set {} from database: {}",
                    context.file_set_id,
                    e
                );
                StepAction::Abort(Error::DbError(format!("Failed to delete file set: {}", e)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf, sync::Arc};

    use core_types::{FileType, ImportedFile, Sha1Checksum};
    use database::{repository_manager::RepositoryManager, setup_test_db};

    use super::*;
    use crate::{file_system_ops::mock::MockFileSystemOps, view_models::Settings};

    #[async_std::test]
    async fn test_validate_not_in_use_step() {
        let TestSetup {
            settings,
            repo_manager,
            fs_ops,
            system_id,
            file1,
        } = prepare_test().await;

        let file2 = ImportedFile {
            original_file_name: "file2.zst".to_string(),
            archive_file_name: "file2.zst".to_string(),
            sha1_checksum: Sha1Checksum::from([1; 20]),
            file_size: 5678,
        };

        let file_set_id =
            prepare_file_set_with_files(&repo_manager, system_id, &[file1, file2]).await;

        let software_title_id = repo_manager
            .get_software_title_repository()
            .add_software_title("Test Software", None)
            .await
            .unwrap();

        // link file set to release
        let release_id = repo_manager
            .get_release_repository()
            .add_release_full(
                "Test Release",
                &[software_title_id],
                &[file_set_id],
                &[system_id],
            )
            .await
            .unwrap();

        let mut context = DeletionContext {
            file_set_id,
            repository_manager: repo_manager.clone(),
            settings: settings.clone(),
            fs_ops: fs_ops.clone(),
            deletion_results: HashMap::new(),
        };

        let step = ValidateFileSetNotInUseStep;

        let action = step.execute(&mut context).await;
        assert!(matches!(action, StepAction::Abort(_)));

        // Delete release - link to file set should have been deleted also and file set can be
        // deleted now
        repo_manager
            .get_release_repository()
            .delete_release(release_id)
            .await
            .unwrap();

        let action = step.execute(&mut context).await;
        assert!(matches!(action, StepAction::Continue));
    }

    #[async_std::test]
    async fn test_fetch_file_infos_step() {
        let TestSetup {
            settings,
            repo_manager,
            fs_ops,
            system_id,
            file1,
        } = prepare_test().await;

        let file_set_id = prepare_file_set_with_files(&repo_manager, system_id, &[file1]).await;

        let mut context = DeletionContext {
            file_set_id,
            repository_manager: repo_manager.clone(),
            settings: settings.clone(),
            fs_ops: fs_ops.clone(),
            deletion_results: HashMap::new(),
        };
        let step = FetchFileInfosStep;
        let action = step.execute(&mut context).await;
        assert!(matches!(action, StepAction::Continue));
        assert_eq!(context.deletion_results.len(), 1);
        let deletion_result = context.deletion_results.values().next().unwrap();
        assert_eq!(deletion_result.file_info.archive_file_name, "file1.zst");
        // at this point we don't know if the file is deletable yet
        assert!(!deletion_result.is_deletable);
    }

    struct TestSetup {
        settings: Arc<Settings>,
        repo_manager: Arc<RepositoryManager>,
        fs_ops: Arc<MockFileSystemOps>,
        system_id: i64,
        file1: ImportedFile,
    }

    async fn prepare_test() -> TestSetup {
        let pool = Arc::new(setup_test_db().await);
        let repo_manager = Arc::new(RepositoryManager::new(pool));
        let settings = Arc::new(Settings {
            collection_root_dir: PathBuf::from("/"),
            ..Default::default()
        });
        let fs_ops = Arc::new(MockFileSystemOps::new());

        let system_id = repo_manager
            .get_system_repository()
            .add_system("Test System")
            .await
            .unwrap();

        let file1 = ImportedFile {
            original_file_name: "file1.zst".to_string(),
            archive_file_name: "file1.zst".to_string(),
            sha1_checksum: Sha1Checksum::from([0; 20]),
            file_size: 1234,
        };

        TestSetup {
            settings,
            repo_manager,
            fs_ops,
            system_id,
            file1,
        }
    }

    async fn prepare_file_set_with_files(
        repo_manager: &RepositoryManager,
        system_id: i64,
        files: &[ImportedFile],
    ) -> i64 {
        repo_manager
            .get_file_set_repository()
            .add_file_set(
                "test_set",
                "file name",
                &FileType::Rom,
                "",
                files,
                &[system_id],
            )
            .await
            .unwrap()
    }
}
