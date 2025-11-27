use std::sync::Arc;

use crate::{
    error::Error,
    external_executable_runner::context::ExternalExecutableRunnerContext,
    file_set_download::service::DownloadService,
    pipeline::pipeline_step::{PipelineStep, StepAction},
    settings_service::SettingsService,
};

pub struct PrepareFilesStep;

#[async_trait::async_trait]
impl PipelineStep<ExternalExecutableRunnerContext> for PrepareFilesStep {
    fn name(&self) -> &'static str {
        "prepare_files"
    }

    async fn execute(&self, context: &mut ExternalExecutableRunnerContext) -> StepAction {
        let settings_service = Arc::new(SettingsService::new(context.repository_manager.clone()));

        let download_service = DownloadService::new_with_fs_ops(
            context.repository_manager.clone(),
            context.settings.clone(),
            settings_service,
            context.fs_ops.clone(),
        );

        let res = download_service
            .download_file_set(
                context.file_set_id,
                context.extract_files,
                None, /* TODO: maybe add progress channel */
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
