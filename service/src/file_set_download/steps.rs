use std::sync::Arc;

use crate::{
    error::Error,
    file_set_download::context::DownloadContext,
    pipeline::{PipelineStep, StepAction},
};

pub struct FetchFileSetStep;

#[async_trait::async_trait]
impl PipelineStep<DownloadContext> for FetchFileSetStep {
    fn name(&self) -> &'static str {
        "fetch_file_set"
    }

    async fn execute(&self, context: &mut DownloadContext) -> StepAction {
        let file_set_res = context
            .repository_manager
            .get_file_set_repository()
            .get_file_set(context.file_set_id)
            .await;
        match file_set_res {
            Ok(file_set) => {
                context.file_set = Some(file_set);
                StepAction::Continue
            }
            Err(e) => {
                eprintln!("Error fetching file set {}: {}", context.file_set_id, e);
                StepAction::Abort(e.into())
            }
        }
    }
}

pub struct FetchFileSetFileInfoStep;

#[async_trait::async_trait]
impl PipelineStep<DownloadContext> for FetchFileSetFileInfoStep {
    fn name(&self) -> &'static str {
        "fetch_file_set_file_info"
    }

    async fn execute(&self, context: &mut DownloadContext) -> StepAction {
        let file_set_file_infos_res = context
            .repository_manager
            .get_file_set_repository()
            .get_file_set_file_info(context.file_set_id)
            .await;

        match file_set_file_infos_res {
            Ok(file_set_file_infos) => {
                context.files_in_set = file_set_file_infos;

                StepAction::Continue
            }
            Err(e) => {
                eprintln!(
                    "Error fetching file infos for file set {}: {}",
                    context.file_set_id, e
                );
                StepAction::Abort(e.into())
            }
        }
    }
}

/// This step goes through each file in file set and collects info of those files that are not available locally - which need to be downloaded.
pub struct PrepareFileForDownloadStep;

#[async_trait::async_trait]
impl PipelineStep<DownloadContext> for PrepareFileForDownloadStep {
    fn name(&self) -> &'static str {
        "prepare_file_for_download"
    }

    fn should_execute(&self, context: &DownloadContext) -> bool {
        !context.files_in_set.is_empty() && context.file_set.is_some()
    }

    async fn execute(&self, context: &mut DownloadContext) -> StepAction {
        let mut files_to_download = vec![];

        if let Some(file_set) = &context.file_set {
            for file in context.files_in_set.iter() {
                let file_path = context
                    .settings
                    .get_file_path(&file_set.file_type, &file.archive_file_name);

                if !file_path.exists() {
                    files_to_download.push(file.clone());
                }
            }
        }

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
