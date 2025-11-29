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

    async fn execute(&self, context: &mut ExternalExecutableRunnerContext) -> StepAction {
        let temp_dir = context.settings.temp_output_dir.clone();

        let initial_file = if context.file_names.len() == 1 {
            context.file_names[0].clone()
        } else if let Some(initial_file) = &context.initial_file {
            initial_file.clone()
        } else if !context.file_names.is_empty() {
            context.file_names[0].clone()
        } else {
            "".to_string()
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
        file_system_ops::mock::MockFileSystemOps,
        pipeline::pipeline_step::{PipelineStep, StepAction},
        view_models::Settings,
    };

    #[async_std::test]
    async fn test_prepare_files_step_success() {
        let download_service_ops = Arc::new(MockDownloadServiceOps::new());

        let mut context = initialize_context(Some(download_service_ops.clone()), None).await;
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

        let mut context = initialize_context(Some(download_service_ops.clone()), None).await;
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

        let mut context = initialize_context(Some(download_service_ops.clone()), None).await;
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
        let mut context = initialize_context(None, Some(executable_runner_ops)).await;
        context.file_names = vec!["file1".to_string(), "file2".to_string()];
        let step = crate::external_executable_runner::steps::StartExecutableStep;
        let res = step.execute(&mut context).await;
        assert!(matches!(res, StepAction::Continue));
        assert!(context.was_successful);
        assert!(context.error_message.is_empty());
    }

    async fn initialize_context(
        download_service_ops: Option<Arc<dyn DownloadServiceOps>>,
        executable_runner_ops: Option<Arc<dyn EmulatorRunnerOps>>,
    ) -> ExternalExecutableRunnerContext {
        let pool = Arc::new(setup_test_db().await);
        let repository_manager = Arc::new(RepositoryManager::new(pool));
        let settings = Arc::new(Settings {
            collection_root_dir: PathBuf::from("/"),
            ..Default::default()
        });
        let fs_ops = Arc::new(MockFileSystemOps::new());

        ExternalExecutableRunnerContext {
            executable: "emulator".to_string(),
            arguments: vec![],
            extract_files: true,
            file_set_id: 1,
            settings,
            initial_file: None,
            fs_ops,
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
