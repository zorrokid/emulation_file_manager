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
            Ok(_) => {
                tracing::info!("Files prepared successfully");
                // TODO: store result to context?
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

    async fn execute(&self, _context: &mut ExternalExecutableRunnerContext) -> StepAction {
        // Implementation for starting the executable goes here
        StepAction::Continue
    }
}

pub struct CleanupFilesStep;

#[async_trait::async_trait]
impl PipelineStep<ExternalExecutableRunnerContext> for CleanupFilesStep {
    fn name(&self) -> &'static str {
        "cleanup_files"
    }

    async fn execute(&self, _context: &mut ExternalExecutableRunnerContext) -> StepAction {
        // Implementation for cleaning up files goes here
        StepAction::Continue
    }
}
