use std::collections::HashMap;

use cloud_storage::events::DownloadEvent;
use core_types::Sha1Checksum;
use file_export::{FileSetExportModel, OutputFile};

use crate::{
    file_set_download::context::{DownloadContext, FileDownloadResult},
    file_system_ops::FileSystemOps,
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

pub struct FetchFileSetStep;

#[async_trait::async_trait]
impl<F: FileSystemOps> PipelineStep<DownloadContext<F>> for FetchFileSetStep {
    fn name(&self) -> &'static str {
        "fetch_file_set"
    }

    async fn execute(&self, context: &mut DownloadContext<F>) -> StepAction {
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
impl<F: FileSystemOps> PipelineStep<DownloadContext<F>> for FetchFileSetFileInfoStep {
    fn name(&self) -> &'static str {
        "fetch_file_set_file_info"
    }

    async fn execute(&self, context: &mut DownloadContext<F>) -> StepAction {
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
impl<F: FileSystemOps> PipelineStep<DownloadContext<F>> for PrepareFileForDownloadStep {
    fn name(&self) -> &'static str {
        "prepare_file_for_download"
    }

    fn should_execute(&self, context: &DownloadContext<F>) -> bool {
        !context.files_in_set.is_empty() && context.file_set.is_some()
    }

    async fn execute(&self, context: &mut DownloadContext<F>) -> StepAction {
        if let Some(file_set) = &context.file_set {
            for file in context.files_in_set.iter() {
                let file_path = context
                    .settings
                    .get_file_path(&file_set.file_type, &file.archive_file_name);

                if !context.fs_ops.exists(&file_path) {
                    context.files_to_download.push(file.into());
                }
            }
        }

        StepAction::Continue
    }
}

pub struct DownloadFilesStep;
#[async_trait::async_trait]
impl<F: FileSystemOps> PipelineStep<DownloadContext<F>> for DownloadFilesStep {
    fn name(&self) -> &'static str {
        "download_files"
    }

    fn should_execute(&self, context: &DownloadContext<F>) -> bool {
        // only execute if there are files to download
        !context.files_to_download.is_empty()
            && context.cloud_ops.is_some()
            && context.file_set.is_some()
    }

    async fn execute(&self, context: &mut DownloadContext<F>) -> StepAction {
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
                    context.file_download_results.push(FileDownloadResult {
                        file_info_id: file_info.id,
                        cloud_key: cloud_key.clone(),
                        cloud_operation_success: true,
                        file_write_success: true,
                        cloud_error: None,
                        file_io_error: None,
                    });
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
                    context.file_download_results.push(FileDownloadResult {
                        file_info_id: file_info.id,
                        cloud_key: cloud_key.clone(),
                        cloud_operation_success: false,
                        file_write_success: false,
                        cloud_error: Some(format!("{}", e)),
                        file_io_error: None,
                    });
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
impl<F: FileSystemOps> PipelineStep<DownloadContext<F>> for ExportFilesStep {
    fn name(&self) -> &'static str {
        "export_files"
    }

    fn should_execute(&self, context: &DownloadContext<F>) -> bool {
        !context.files_in_set.is_empty() && context.file_set.is_some()
    }

    async fn execute(&self, context: &mut DownloadContext<F>) -> StepAction {
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

        let res = if context.extract_files {
            context.export_ops.export(&export_model)
        } else {
            context.export_ops.export_zipped(&export_model)
        };
        
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
    use core_types::{FileType, ImportedFile, Sha1Checksum};
    use database::{repository_manager::RepositoryManager, setup_test_db};

    use crate::{
        file_set_download::{
            context::DownloadContext,
            steps::{
                DownloadFilesStep, ExportFilesStep, FetchFileSetFileInfoStep, FetchFileSetStep,
                PrepareFileForDownloadStep,
            },
        },
        file_system_ops::mock::MockFileSystemOps,
        pipeline::pipeline_step::{PipelineStep, StepAction},
        settings_service::SettingsService,
        view_models::Settings,
    };

    #[async_std::test]
    async fn test_fetch_file_set_step_file_set_not_found() {
        let (mut context, _) = initialize_context(false).await;
        let step = FetchFileSetStep;
        let action = step.execute(&mut context).await;
        assert!(matches!(action, StepAction::Abort(_)));
    }

    #[async_std::test]
    async fn test_fetch_file_set_step_file_set_found() {
        let (mut context, _) = initialize_context(false).await;

        let file_set_id = prepare_file_set_with_files(
            &context.repository_manager,
            "some_cryptic_filename",
            &FileType::Rom,
        )
        .await;

        context.file_set_id = file_set_id;
        let step = FetchFileSetStep;
        let action = step.execute(&mut context).await;
        assert!(matches!(action, StepAction::Continue));
        assert!(context.file_set.is_some());
        assert_eq!(context.file_set.as_ref().unwrap().id, file_set_id);
    }

    #[async_std::test]
    async fn test_fetch_file_set_file_info_step() {
        let (mut context, _) = initialize_context(false).await;

        let file_set_id = prepare_file_set_with_files(
            &context.repository_manager,
            "some_cryptic_filename",
            &FileType::Rom,
        )
        .await;

        context.file_set_id = file_set_id;
        let step = FetchFileSetFileInfoStep;
        let should_execute = step.should_execute(&context);
        assert!(should_execute);
        let action = step.execute(&mut context).await;
        assert!(matches!(action, StepAction::Continue));
        assert_eq!(context.files_in_set.len(), 1);
    }

    #[async_std::test]
    async fn test_prepare_file_for_download_step_file_exists_locally() {
        let (mut context, _) = initialize_context(false).await;

        let archive_file_name = "some_cryptic_filename";
        let file_type = FileType::Rom;

        let _file_set_id =
            prepare_file_set_with_files(&context.repository_manager, archive_file_name, &file_type)
                .await;

        let file_path = context
            .settings
            .get_file_path(&file_type, archive_file_name);

        context
            .fs_ops
            .add_file(file_path.to_string_lossy().as_ref());

        context.file_set = Some(
            context
                .repository_manager
                .get_file_set_repository()
                .get_file_set(context.file_set_id)
                .await
                .unwrap(),
        );
        context.files_in_set = context
            .repository_manager
            .get_file_set_repository()
            .get_file_set_file_info(context.file_set_id)
            .await
            .unwrap();

        let step = PrepareFileForDownloadStep;
        let should_execute = step.should_execute(&context);
        assert!(should_execute);
        let action = step.execute(&mut context).await;
        assert!(matches!(action, StepAction::Continue));
        assert_eq!(context.files_to_download.len(), 0);
    }

    #[async_std::test]
    async fn test_prepare_file_for_download_step_file_does_not_exist_locally() {
        let (mut context, _) = initialize_context(false).await;

        let archive_file_name = "some_cryptic_filename";
        let file_type = FileType::Rom;

        let _file_set_id =
            prepare_file_set_with_files(&context.repository_manager, archive_file_name, &file_type)
                .await;

        let file_set = context
            .repository_manager
            .get_file_set_repository()
            .get_file_set(context.file_set_id)
            .await
            .unwrap();

        context.file_set = Some(file_set);
        context.files_in_set = context
            .repository_manager
            .get_file_set_repository()
            .get_file_set_file_info(context.file_set_id)
            .await
            .unwrap();

        let step = PrepareFileForDownloadStep;
        let should_execute = step.should_execute(&context);
        assert!(should_execute);
        let action = step.execute(&mut context).await;
        assert!(matches!(action, StepAction::Continue));
        assert_eq!(context.files_to_download.len(), 1);
        assert_eq!(
            context.files_to_download[0].archive_file_name,
            archive_file_name
        );
    }

    #[async_std::test]
    async fn test_download_files_step_with_successful_download() {
        let (mut context, _) = initialize_context(false).await;

        let archive_file_name = "some_cryptic_filename";
        let file_type = FileType::Rom;

        let _file_set_id =
            prepare_file_set_with_files(&context.repository_manager, archive_file_name, &file_type)
                .await;

        let file_set = context
            .repository_manager
            .get_file_set_repository()
            .get_file_set(context.file_set_id)
            .await
            .unwrap();

        context.file_set = Some(file_set);
        context.files_in_set = context
            .repository_manager
            .get_file_set_repository()
            .get_file_set_file_info(context.file_set_id)
            .await
            .unwrap();
        context.files_to_download = context.files_in_set.iter().map(|f| f.into()).collect();

        let file_path = context
            .settings
            .get_file_path(&file_type, archive_file_name);
        let key = context.files_to_download[0].generate_cloud_key();

        context
            .cloud_ops
            .clone()
            .unwrap()
            .upload_file(&file_path, &key, None)
            .await
            .unwrap();

        let step = DownloadFilesStep;
        let should_execute = step.should_execute(&context);
        assert!(should_execute);
        let action = step.execute(&mut context).await;
        assert!(matches!(action, StepAction::Continue));
        assert_eq!(context.successful_downloads(), 1);
        assert_eq!(context.failed_downloads(), 0);
        assert_eq!(
            context.file_download_results.first().unwrap().cloud_key,
            key
        );
    }

    #[async_std::test]
    async fn test_download_files_step_with_failed_download() {
        let (mut context, _) = initialize_context(false).await;

        let archive_file_name = "some_cryptic_filename";
        let file_type = FileType::Rom;

        let _file_set_id =
            prepare_file_set_with_files(&context.repository_manager, archive_file_name, &file_type)
                .await;

        let file_set = context
            .repository_manager
            .get_file_set_repository()
            .get_file_set(context.file_set_id)
            .await
            .unwrap();

        context.file_set = Some(file_set);
        context.files_in_set = context
            .repository_manager
            .get_file_set_repository()
            .get_file_set_file_info(context.file_set_id)
            .await
            .unwrap();
        context.files_to_download = context.files_in_set.iter().map(|f| f.into()).collect();

        let key = context.files_to_download[0].generate_cloud_key();

        let step = DownloadFilesStep;
        let should_execute = step.should_execute(&context);
        assert!(should_execute);
        let action = step.execute(&mut context).await;
        assert!(matches!(action, StepAction::Abort(_)));
        assert_eq!(context.successful_downloads(), 0);
        assert_eq!(context.failed_downloads(), 1);
        let download_result = context.file_download_results.first().unwrap();
        assert_eq!(download_result.cloud_key, key);
        assert!(!download_result.cloud_operation_success);
        assert!(!download_result.file_write_success);
    }

    #[async_std::test]
    async fn test_export_files_step_success() {
        let (mut context, mock_export_state) = initialize_context(false).await;

        let archive_file_name = "some_cryptic_filename";
        let file_type = FileType::Rom;

        let _file_set_id =
            prepare_file_set_with_files(&context.repository_manager, archive_file_name, &file_type)
                .await;

        let file_set = context
            .repository_manager
            .get_file_set_repository()
            .get_file_set(context.file_set_id)
            .await
            .unwrap();

        context.file_set = Some(file_set);
        context.files_in_set = context
            .repository_manager
            .get_file_set_repository()
            .get_file_set_file_info(context.file_set_id)
            .await
            .unwrap();

        let step = ExportFilesStep;
        let should_execute = step.should_execute(&context);
        assert!(should_execute);

        let action = step.execute(&mut context).await;
        assert!(matches!(action, StepAction::Continue));
        
        // Verify export was called via shared state
        assert_eq!(mock_export_state.total_calls(), 1);
        assert_eq!(mock_export_state.export_zipped_calls().len(), 1);
        
        let call = &mock_export_state.export_zipped_calls()[0];
        assert_eq!(call.output_file_names.len(), 1);
        assert!(!call.extract_files);
    }

    #[async_std::test]
    async fn test_export_files_step_with_extraction() {
        let (mut context, mock_export_state) = initialize_context(true).await;

        let archive_file_name = "some_cryptic_filename";
        let file_type = FileType::Rom;

        let _file_set_id =
            prepare_file_set_with_files(&context.repository_manager, archive_file_name, &file_type)
                .await;

        let file_set = context
            .repository_manager
            .get_file_set_repository()
            .get_file_set(context.file_set_id)
            .await
            .unwrap();

        context.file_set = Some(file_set);
        context.files_in_set = context
            .repository_manager
            .get_file_set_repository()
            .get_file_set_file_info(context.file_set_id)
            .await
            .unwrap();

        let step = ExportFilesStep;
        let action = step.execute(&mut context).await;
        assert!(matches!(action, StepAction::Continue));
        
        // Verify export (not export_zipped) was called via shared state
        assert_eq!(mock_export_state.total_calls(), 1);
        assert_eq!(mock_export_state.export_calls().len(), 1);
        
        let call = &mock_export_state.export_calls()[0];
        assert!(call.extract_files);
    }

    async fn initialize_context(
        extract_files: bool,
    ) -> (
        DownloadContext<MockFileSystemOps>,
        Arc<file_export::file_export_ops::MockState>,
    ) {
        let pool = Arc::new(setup_test_db().await);
        let repo_manager = Arc::new(RepositoryManager::new(pool));
        let settings = Arc::new(Settings {
            collection_root_dir: PathBuf::from("/"),
            ..Default::default()
        });

        let settings_service = Arc::new(SettingsService::new(repo_manager.clone()));
        let cloud_ops = Arc::new(MockCloudStorage::new());

        let (tx, _rx) = async_std::channel::unbounded();
        let fs_ops = Arc::new(MockFileSystemOps::new());
        
        let mock_export_state = Arc::new(file_export::file_export_ops::MockState::new());
        let export_ops = Arc::new(
            file_export::file_export_ops::MockFileExportOps::new_with_state(mock_export_state.clone())
        );

        let context = DownloadContext::new(
            repo_manager,
            settings,
            settings_service,
            tx,
            1,
            extract_files,
            Some(cloud_ops),
            fs_ops,
            export_ops,
        );
        
        (context, mock_export_state)
    }

    async fn prepare_file_set_with_files(
        repo_manager: &RepositoryManager,
        archive_file_name: &str,
        file_type: &FileType,
        //system_id: i64,
        //files: &[ImportedFile],
    ) -> i64 {
        let system_id = repo_manager
            .get_system_repository()
            .add_system("Test System")
            .await
            .unwrap();

        let file = ImportedFile {
            original_file_name: archive_file_name.to_string(),
            archive_file_name: archive_file_name.to_string(),
            sha1_checksum: Sha1Checksum::from([1; 20]),
            file_size: 5678,
        };

        repo_manager
            .get_file_set_repository()
            .add_file_set(
                "test_set",
                "file name",
                file_type,
                "",
                &[file],
                &[system_id],
            )
            .await
            .unwrap()
    }
}
