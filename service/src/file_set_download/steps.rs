use std::collections::HashMap;

use cloud_storage::events::DownloadEvent;
use core_types::Sha1Checksum;
use file_export::{export_files_zipped_or_non_zipped, FileSetExportModel, OutputFile};

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
                .download_file(cloud_key, target_path.as_path(), Some(&context.progress_tx))
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

        if context.failed_downloads() > 0 {
            StepAction::Abort(crate::error::Error::DownloadError(format!(
                "{} files failed to download",
                context.failed_downloads(),
            )))
        } else {
            StepAction::Continue
        }
    }
}

pub struct ExportFilesStep;
#[async_trait::async_trait]
impl PipelineStep<DownloadContext> for ExportFilesStep {
    fn name(&self) -> &'static str {
        "export_files"
    }

    fn should_execute(&self, context: &DownloadContext) -> bool {
        !context.files_in_set.is_empty() && context.file_set.is_some()
    }

    async fn execute(&self, context: &mut DownloadContext) -> StepAction {
        let file_set = context
            .file_set
            .as_ref()
            .expect("This step should only execute if file_set is Some");

        let source_file_path = context.settings.get_file_type_path(&file_set.file_type);

        let output_mapping = context
            .files_in_set
            .iter()
            .map(|f| {
                let checksum: Sha1Checksum = f
                    .sha1_checksum
                    .clone()
                    .try_into()
                    .expect("Failed to convert to Sha1Checksum");
                (
                    f.archive_file_name.clone(),
                    OutputFile {
                        output_file_name: f.file_name.clone(),
                        checksum,
                    },
                )
            })
            .collect::<HashMap<String, OutputFile>>();

        let exported_zip_file_name = file_set.name.clone();

        let export_model = FileSetExportModel {
            output_mapping,
            source_file_path,
            output_dir: context.settings.temp_output_dir.clone(),
            extract_files: context.extract_files,
            exported_zip_file_name,
        };

        let res = export_files_zipped_or_non_zipped(&export_model);
        match res {
            Ok(_) => {
                context.file_output_mapping = export_model.output_mapping;
                StepAction::Continue
            }
            Err(e) => {
                eprintln!("Error exporting files for file set {}: {}", file_set.id, e);
                return StepAction::Abort(e.into());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, sync::Arc};

    use cloud_storage::mock::MockCloudStorage;
    use database::{repository_manager::RepositoryManager, setup_test_db};

    use crate::{
        file_set_download::{context::DownloadContext, steps::FetchFileSetStep},
        pipeline::pipeline_step::{PipelineStep, StepAction},
        settings_service::SettingsService,
        view_models::Settings,
    };

    #[async_std::test]
    async fn test_fetch_file_set_step_file_set_not_found() {
        let mut context = initialize_context(false).await;
        let step = FetchFileSetStep;
        let action = step.execute(&mut context).await;
        assert!(matches!(action, StepAction::Abort(_)));
    }

    #[async_std::test]
    async fn test_fetch_file_set_step_file_set_found() {}

    async fn initialize_context(extract_files: bool) -> DownloadContext {
        let pool = Arc::new(setup_test_db().await);
        let repo_manager = Arc::new(RepositoryManager::new(pool));
        let settings = Arc::new(Settings {
            collection_root_dir: PathBuf::from("/"),
            ..Default::default()
        });

        let settings_service = Arc::new(SettingsService::new(repo_manager.clone()));
        let cloud_ops = Arc::new(MockCloudStorage::new());

        let (tx, _rx) = async_std::channel::unbounded();

        DownloadContext::new(
            repo_manager,
            settings,
            settings_service,
            tx,
            1,
            extract_files,
            Some(cloud_ops),
        )
    }
}
