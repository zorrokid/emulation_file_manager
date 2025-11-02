use cloud_storage::events::DownloadEvent;

use crate::{
    file_set_download::context::DownloadContext,
    pipeline::pipeline_step::{PipelineStep, StepAction},
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
            && context.cloud_ops.is_some()
            && context.file_set.is_some()
    }

    async fn execute(&self, context: &mut DownloadContext) -> StepAction {
        for file_info in context.files_to_download.iter() {
            let cloud_key = &file_info.generate_cloud_key();
            context
                .progress_tx
                .send(DownloadEvent::FileDownloadStarted {
                    key: cloud_key.clone(),
                })
                .await
                .ok();

            let target_path = context.settings.get_file_path(
                &context
                    .file_set
                    .as_ref()
                    .expect("This step should only execute if file_set is Some")
                    .file_type,
                &file_info.archive_file_name,
            );
            let download_res = context
                .cloud_ops
                .as_ref()
                .expect("This step should only execute if cloud_ops is Some")
                .download_file(
                    &cloud_key,
                    target_path.as_path(),
                    Some(&context.progress_tx),
                )
                .await;
            match download_res {
                Ok(_) => {
                    context
                        .progress_tx
                        .send(DownloadEvent::FileDownloadCompleted {
                            key: cloud_key.clone(),
                        })
                        .await
                        .ok();
                    context.file_download_results.push(
                        crate::file_set_download::context::FileDownloadResult {
                            file_info_id: file_info.id,
                            cloud_key: file_info.archive_file_name.clone(),
                            cloud_operation_success: true,
                            file_write_success: true,
                            cloud_error: None,
                            file_io_error: None,
                        },
                    );
                }
                Err(e) => {
                    context
                        .progress_tx
                        .send(DownloadEvent::FileDownloadFailed {
                            key: cloud_key.clone(),
                            error: format!("{}", e),
                        })
                        .await
                        .ok();

                    eprintln!(
                        "Error downloading file {}: {}",
                        file_info.archive_file_name, e
                    );
                    context.file_download_results.push(
                        crate::file_set_download::context::FileDownloadResult {
                            file_info_id: file_info.id,
                            cloud_key: file_info.archive_file_name.clone(),
                            cloud_operation_success: false,
                            file_write_success: false,
                            cloud_error: Some(format!("{}", e)),
                            file_io_error: None,
                        },
                    );
                }
            }
        }

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
