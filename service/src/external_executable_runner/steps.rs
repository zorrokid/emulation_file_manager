use crate::{
    error::Error,
    external_executable_runner::context::ExternalExecutableRunnerContext,
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

pub struct PrepareFilesStep;

#[async_trait::async_trait]
impl PipelineStep<ExternalExecutableRunnerContext> for PrepareFilesStep {
    fn name(&self) -> &'static str {
        "prepare_files"
    }

    async fn execute(&self, context: &mut ExternalExecutableRunnerContext) -> StepAction {
        let res = context
            .download_service_ops
            .download_file_set(
                context.file_set_id,
                context.extract_files,
                context.progress_tx.clone(),
            )
            .await;

        match res {
            Ok(download_result) => {
                tracing::info!("Files prepared successfully");
                if download_result.failed_downloads > 0 {
                    tracing::warn!("No files were downloaded successfully");
                    return StepAction::Abort(Error::DownloadError(
                        "Some files failed to download".to_string(),
                    ));
                }
                context.file_names = download_result.output_file_names;
                StepAction::Continue
            }
            Err(e) => {
                tracing::error!("Error preparing files: {:?}", e);
                StepAction::Abort(Error::DownloadError(e.to_string())) // TODO: maybe add more
                // specific error
            }
        }
    }
}

pub struct StartExecutableStep;

#[async_trait::async_trait]
impl PipelineStep<ExternalExecutableRunnerContext> for StartExecutableStep {
    fn name(&self) -> &'static str {
        "start_executable"
    }

    fn should_execute(&self, context: &ExternalExecutableRunnerContext) -> bool {
        !context.file_names.is_empty()
    }

    async fn execute(&self, context: &mut ExternalExecutableRunnerContext) -> StepAction {
        let temp_dir = context.settings.temp_output_dir.clone();

        let initial_file = if context.file_names.len() == 1 {
            context.file_names[0].clone()
        } else if let Some(initial_file) = &context.initial_file {
            initial_file.clone()
        } else if !context.file_names.is_empty() {
            context.file_names[0].clone()
        } else {
            return StepAction::Abort(Error::InvalidInput(
                "No files available to start executable".to_string(),
            ));
        };

        let res = context
            .executable_runner_ops
            .run_with_emulator(
                context.executable.clone(), // TODO: pass executable as reference
                &context.arguments,
                &context.file_names,
                initial_file,
                temp_dir,
            )
            .await;

        match res {
            Ok(_) => {
                tracing::info!("Executable executed and finished successfully");
                context.was_successful = true;
            }
            Err(e) => {
                tracing::error!("Error starting executable: {:?}", e);
                context.was_successful = false;
                context.error_message.push(e.to_string());
            }
        }
        // continue regardless of success or failure to clean up
        StepAction::Continue
    }
}

pub struct CleanupFilesStep;

#[async_trait::async_trait]
impl PipelineStep<ExternalExecutableRunnerContext> for CleanupFilesStep {
    fn name(&self) -> &'static str {
        "cleanup_files"
    }

    fn should_execute(&self, context: &ExternalExecutableRunnerContext) -> bool {
        !context.file_names.is_empty()
    }

    async fn execute(&self, context: &mut ExternalExecutableRunnerContext) -> StepAction {
        let path = &context.settings.temp_output_dir;
        tracing::info!("Cleaning up temporary files at {:?}", path);
        for file_name in &context.file_names {
            let file_path = path.join(file_name);
            if context.fs_ops.exists(&file_path) {
                match context.fs_ops.remove_file(&file_path) {
                    Ok(_) => {
                        tracing::info!("Deleted temporary file {:?}", file_path);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to delete temporary file {:?}: {:?}", file_path, e);
                        context.error_message.push(format!(
                            "Failed to delete temporary file {:?}: {:?}",
                            file_path, e
                        ));
                    }
                }
            }
        }
        StepAction::Continue
    }
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, sync::Arc};

    use database::{repository_manager::RepositoryManager, setup_test_db};
    use emulator_runner::ops::{EmulatorRunnerOps, MockEmulatorRunnerOps};

    use crate::{
        external_executable_runner::{
            context::ExternalExecutableRunnerContext, steps::PrepareFilesStep,
        },
        file_set_download::download_service_ops::{DownloadServiceOps, MockDownloadServiceOps},
        file_system_ops::{FileSystemOps, mock::MockFileSystemOps},
        pipeline::pipeline_step::{PipelineStep, StepAction},
        view_models::Settings,
    };

    #[async_std::test]
    async fn test_prepare_files_step_success() {
        let download_service_ops = Arc::new(MockDownloadServiceOps::new());

        let mut context = initialize_context(Some(download_service_ops.clone()), None, None).await;
        let step = PrepareFilesStep;
        let res = step.execute(&mut context).await;
        let download_calls = download_service_ops.download_calls();
        assert_eq!(download_calls.len(), 1);
        let call = &download_calls[0];
        assert_eq!(call.file_set_id, context.file_set_id);
        assert_eq!(call.extract_files, context.extract_files);
        assert!(matches!(res, StepAction::Continue));
    }

    #[async_std::test]
    async fn test_prepare_files_step_failure() {
        let download_service_ops = Arc::new(MockDownloadServiceOps::with_failure(
            "Simulated download failure",
        ));

        let mut context = initialize_context(Some(download_service_ops.clone()), None, None).await;
        let step = PrepareFilesStep;
        let res = step.execute(&mut context).await;
        let download_calls = download_service_ops.download_calls();
        assert_eq!(download_calls.len(), 1);
        let call = &download_calls[0];
        assert_eq!(call.file_set_id, context.file_set_id);
        assert_eq!(call.extract_files, context.extract_files);
        assert!(matches!(res, StepAction::Abort(_)));
    }

    #[async_std::test]
    async fn test_prepare_files_step_with_failed_downloads() {
        // if even one download fails, the step should abort
        let download_service_ops =
            Arc::new(MockDownloadServiceOps::with_successful_and_failed_downloads(1, 1));

        let mut context = initialize_context(Some(download_service_ops.clone()), None, None).await;
        let step = PrepareFilesStep;
        let res = step.execute(&mut context).await;
        let download_calls = download_service_ops.download_calls();
        assert_eq!(download_calls.len(), 1);
        let call = &download_calls[0];
        assert_eq!(call.file_set_id, context.file_set_id);
        assert_eq!(call.extract_files, context.extract_files);
        assert!(matches!(res, StepAction::Abort(_)));
    }

    #[async_std::test]
    async fn test_start_executable_step_with_success() {
        let executable_runner_ops = Arc::new(MockEmulatorRunnerOps::new());
        let mut context = initialize_context(None, Some(executable_runner_ops.clone()), None).await;
        context.file_names = vec!["file1".to_string(), "file2".to_string()];
        let step = crate::external_executable_runner::steps::StartExecutableStep;
        let res = step.execute(&mut context).await;
        assert!(matches!(res, StepAction::Continue));
        assert!(context.was_successful);
        assert!(context.error_message.is_empty());
        assert!(executable_runner_ops.total_calls() == 1);
        assert_eq!(
            executable_runner_ops.run_calls()[0].file_names,
            vec!["file1".to_string(), "file2".to_string()]
        );
        assert_eq!(
            executable_runner_ops.run_calls()[0].selected_file_name,
            "file1".to_string()
        );
    }

    #[async_std::test]
    async fn test_start_executable_step_success_with_start_file_defined() {
        let executable_runner_ops = Arc::new(MockEmulatorRunnerOps::new());
        let mut context = initialize_context(None, Some(executable_runner_ops.clone()), None).await;
        context.file_names = vec!["file1".to_string(), "file2".to_string()];
        context.initial_file = Some("file2".to_string());
        let step = crate::external_executable_runner::steps::StartExecutableStep;
        let res = step.execute(&mut context).await;
        assert!(matches!(res, StepAction::Continue));
        assert!(context.was_successful);
        assert!(context.error_message.is_empty());
        assert!(executable_runner_ops.total_calls() == 1);
        assert_eq!(
            executable_runner_ops.run_calls()[0].file_names,
            vec!["file1".to_string(), "file2".to_string()]
        );
        assert_eq!(
            executable_runner_ops.run_calls()[0].selected_file_name,
            "file2".to_string()
        );
    }

    #[async_std::test]
    async fn test_start_executable_failure_without_files() {
        let executable_runner_ops = Arc::new(MockEmulatorRunnerOps::new());
        let mut context = initialize_context(None, Some(executable_runner_ops.clone()), None).await;
        context.file_names = vec![];
        let step = crate::external_executable_runner::steps::StartExecutableStep;
        assert!(!step.should_execute(&context));
        let res = step.execute(&mut context).await;
        assert!(matches!(res, StepAction::Abort(_)));
    }

    #[async_std::test]
    async fn test_start_executable_step_with_failure() {
        let executable_runner_ops = Arc::new(MockEmulatorRunnerOps::with_failure(
            "Simulated emulator failure",
        ));
        let mut context = initialize_context(None, Some(executable_runner_ops.clone()), None).await;
        context.file_names = vec!["file1".to_string()];
        let step = crate::external_executable_runner::steps::StartExecutableStep;
        let res = step.execute(&mut context).await;
        // should continue even on failure to allow cleanup
        assert!(matches!(res, StepAction::Continue));
        assert!(!context.was_successful);
        assert!(!context.error_message.is_empty());
        assert!(executable_runner_ops.total_calls() == 1);
    }

    #[async_std::test]
    async fn test_cleanup_files_step_execution_success() {
        let fs_ops = Arc::new(MockFileSystemOps::new());
        fs_ops.add_file("/temp/file1");
        fs_ops.add_file("/temp/file2");

        let mut context = initialize_context(None, None, Some(fs_ops.clone())).await;
        context.file_names = vec!["file1".to_string(), "file2".to_string()];
        let step = crate::external_executable_runner::steps::CleanupFilesStep;
        let res = step.execute(&mut context).await;
        assert!(matches!(res, StepAction::Continue));
        assert!(fs_ops.was_deleted("/temp/file1"));
        assert!(fs_ops.was_deleted("/temp/file2"));
    }

    #[async_std::test]
    async fn test_cleanup_files_step_execution_failure() {
        let fs_ops = Arc::new(MockFileSystemOps::new());
        fs_ops.fail_delete_with("Simulated delete failure");

        let mut context = initialize_context(None, None, Some(fs_ops.clone())).await;
        context.file_names = vec!["file1".to_string(), "file2".to_string()];
        let step = crate::external_executable_runner::steps::CleanupFilesStep;
        let res = step.execute(&mut context).await;
        assert!(matches!(res, StepAction::Continue));
        context.error_message.iter().for_each(|msg| {
            assert!(msg.contains("Failed to delete temporary file"));
            assert!(msg.contains("Simulated delete failure"));
            assert!(msg.contains("file1") || msg.contains("file2"));
        });
    }

    async fn initialize_context(
        download_service_ops: Option<Arc<dyn DownloadServiceOps>>,
        executable_runner_ops: Option<Arc<dyn EmulatorRunnerOps>>,
        file_system_ops: Option<Arc<dyn FileSystemOps>>,
    ) -> ExternalExecutableRunnerContext {
        let pool = Arc::new(setup_test_db().await);
        let repository_manager = Arc::new(RepositoryManager::new(pool));
        let settings = Arc::new(Settings {
            collection_root_dir: PathBuf::from("/"),
            temp_output_dir: PathBuf::from("/temp"),
            ..Default::default()
        });

        ExternalExecutableRunnerContext {
            executable: "emulator".to_string(),
            arguments: vec![],
            extract_files: true,
            file_set_id: 1,
            settings,
            initial_file: None,
            fs_ops: file_system_ops.unwrap_or(Arc::new(MockFileSystemOps::new())),
            repository_manager,
            error_message: Vec::new(),
            file_names: Vec::new(),
            executable_runner_ops: executable_runner_ops
                .unwrap_or(Arc::new(MockEmulatorRunnerOps::new())),
            was_successful: false,
            download_service_ops: download_service_ops
                .unwrap_or(Arc::new(MockDownloadServiceOps::new())),
            progress_tx: None,
        }
    }
}
