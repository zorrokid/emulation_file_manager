use crate::{
    file_set_download::context::DownloadContext,
    pipeline::{PipelineStep, StepAction},
};

pub struct FetchFileInfoStep;

#[async_trait::async_trait]
impl PipelineStep<DownloadContext> for FetchFileInfoStep {
    fn name(&self) -> &'static str {
        "fetch_file_info"
    }

    async fn execute(&self, context: &mut DownloadContext) -> StepAction {
        // fetch file metadata from database and store in context

        StepAction::Continue
    }
}

pub struct PrepareFileForDownloadStep;

#[async_trait::async_trait]
impl PipelineStep<DownloadContext> for PrepareFileForDownloadStep {
    fn name(&self) -> &'static str {
        "prepare_file_for_download"
    }

    async fn execute(&self, context: &mut DownloadContext) -> StepAction {
        // go through each file and collect which files are not available locally

        StepAction::Continue
    }
}

pub struct ConnectToCloudStep; // TODO: can we use same ConnectToCloudStep from upload?
#[async_trait::async_trait]
impl PipelineStep<DownloadContext> for ConnectToCloudStep {
    fn name(&self) -> &'static str {
        "connect_to_cloud"
    }
    fn should_execute(&self, context: &DownloadContext) -> bool {
        // only execute if there are files to download
        !context.files_to_download.is_empty()
    }
    async fn execute(&self, context: &mut DownloadContext) -> StepAction {
        // connect to cloud storage and store client in context

        StepAction::Continue
    }
}

pub struct DownloadFilesStep;
#[async_trait::async_trait]
impl PipelineStep<DownloadContext> for DownloadFilesStep {
    fn name(&self) -> &'static str {
        "download_files"
    }

    fn should_execute(&self, context: &DownloadContext) -> bool {
        // only execute if there are files to download
        !context.files_to_download.is_empty()
    }

    async fn execute(&self, context: &mut DownloadContext) -> StepAction {
        // download missing files from cloud storage

        StepAction::Continue
    }
}

pub struct ExportFilesStep;
#[async_trait::async_trait]
impl PipelineStep<DownloadContext> for ExportFilesStep {
    fn name(&self) -> &'static str {
        "export_files"
    }
    fn should_execute(&self, context: &DownloadContext) -> bool {
        // only execute if there are files to download
        !context.files_in_set.is_empty()
    }
    async fn execute(&self, context: &mut DownloadContext) -> StepAction {
        // export files to temp directory

        StepAction::Continue
    }
}
