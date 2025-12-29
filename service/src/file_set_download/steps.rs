use std::collections::HashMap;

use core_types::{IMAGE_FILE_TYPES, Sha1Checksum, events::DownloadEvent};
use file_export::{FileSetExportModel, OutputFile};

use crate::{
    file_set_download::context::{DownloadContext, FileDownloadResult},
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

pub struct FetchFileSetStep;

#[async_trait::async_trait]
impl PipelineStep<DownloadContext> for FetchFileSetStep {
    fn name(&self) -> &'static str {
        "fetch_file_set"
    }

    async fn execute(&self, context: &mut DownloadContext) -> StepAction {
        tracing::debug!(file_set_id = context.file_set_id, "Fetching file set");

        let file_set_res = context
            .repository_manager
            .get_file_set_repository()
            .get_file_set(context.file_set_id)
            .await;
        match file_set_res {
            Ok(file_set) => {
                tracing::info!(
                    file_set_id = context.file_set_id,
                    file_set_name = %file_set.name,
                    "File set found"
                );
                context.file_set = Some(file_set);
                StepAction::Continue
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    file_set_id = context.file_set_id,
                    "Failed to fetch file set"
                );
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

/// This step downloads the files that were identified as needing download in the previous step.
/// It reports progress via the progress_tx channel if available, and records the results of
/// each download attempt.
/// If any downloads fail, the step aborts the pipeline with an error.
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
        tracing::info!(
            file_count = context.files_to_download.len(),
            "Starting file downloads"
        );

        for file_info in context.files_to_download.iter() {
            let cloud_key = &file_info.generate_cloud_key();

            tracing::debug!(
                cloud_key = %cloud_key,
                archive_file_name = %file_info.archive_file_name,
                "Downloading file"
            );

            if let Some(tx) = &context.progress_tx {
                tx.send(DownloadEvent::FileDownloadStarted {
                    key: cloud_key.clone(),
                })
                .await
                .ok(); // TODO: Handle send error?
            }

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
                    cloud_key,
                    target_path.as_path(),
                    context.progress_tx.as_ref(),
                )
                .await;
            match download_res {
                Ok(_) => {
                    tracing::debug!(
                        cloud_key = %cloud_key,
                        "File downloaded successfully"
                    );

                    if let Some(tx) = &context.progress_tx {
                        tx.send(DownloadEvent::FileDownloadCompleted {
                            key: cloud_key.clone(),
                        })
                        .await
                        .ok(); // TODO: Handle send error?
                    }

                    context.file_download_results.push(FileDownloadResult {
                        file_info_id: file_info.id,
                        cloud_key: cloud_key.clone(),
                        cloud_operation_success: true,
                        file_write_success: true,
                        error: None,
                    });
                }
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        cloud_key = %cloud_key,
                        archive_file_name = %file_info.archive_file_name,
                        "File download failed"
                    );

                    if let Some(tx) = &context.progress_tx {
                        tx.send(DownloadEvent::FileDownloadFailed {
                            key: cloud_key.clone(),
                            error: format!("{}", e),
                        })
                        .await
                        .ok(); // TODO: Handle send error?
                    }

                    context.file_download_results.push(FileDownloadResult {
                        file_info_id: file_info.id,
                        cloud_key: cloud_key.clone(),
                        cloud_operation_success: false,
                        file_write_success: false,
                        error: Some(e.to_string()),
                    });
                }
            }
        }

        let failed = context.failed_downloads();
        let successful = context.successful_downloads();

        if failed > 0 {
            tracing::warn!(successful, failed, "Some downloads failed");
            // TODO: parse errors, example message:
            // <?xml version="1.0" encoding="UTF-8" standalone="yes"?>
            // <Error>
            //     <Code>NoSuchKey</Code>
            //     <Message>Key not found</Message>
            // </Error>
            let errors = context
                .file_download_results
                .iter()
                .filter_map(|result| result.error.clone())
                .collect::<Vec<String>>();

            StepAction::Abort(crate::error::Error::DownloadError(format!(
                "{} files failed to download out of {} attempted. Errors: {:?}",
                failed,
                successful + failed,
                errors
            )))
        } else {
            tracing::info!(successful, "All downloads completed successfully");
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

        tracing::info!(
            file_set_id = file_set.id,
            file_set_name = %file_set.name,
            extract_files = context.extract_files,
            file_count = context.files_in_set.len(),
            "Exporting files"
        );

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
            tracing::debug!("Exporting files individually");
            context.export_ops.export(&export_model)
        } else {
            tracing::debug!("Exporting files as zip");
            context.export_ops.export_zipped(&export_model)
        };

        match res {
            Ok(_) => {
                if context.extract_files {
                    context.output_file_names = export_model
                        .output_mapping
                        .values()
                        .map(|f| f.output_file_name.clone())
                        .collect();
                } else {
                    context.output_file_names = vec![export_model.exported_zip_file_name.clone()];
                }
                context.file_output_mapping = export_model.output_mapping;
                StepAction::Continue
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    file_set_id = file_set.id,
                    "Failed to export files"
                );
                return StepAction::Abort(e.into());
            }
        }
    }
}

pub struct PrepareThumbnailsStep;

#[async_trait::async_trait]
impl PipelineStep<DownloadContext> for PrepareThumbnailsStep {
    fn name(&self) -> &'static str {
        "prepare_thumbnails"
    }

    fn should_execute(&self, context: &DownloadContext) -> bool {
        if let Some(file_set) = &context.file_set
            && IMAGE_FILE_TYPES.contains(&file_set.file_type)
            && context.extract_files
            && !context.file_output_mapping.is_empty()
        {
            true
        } else {
            tracing::info!("Skipping thumbnail preparation step");
            false
        }
    }

    async fn execute(&self, context: &mut DownloadContext) -> StepAction {
        tracing::info!("Preparing thumbnails for image file set");
        let thumnail_dir = context.settings.get_thumbnails_path();
        let output_dir = &context.settings.temp_output_dir;
        let output_mapping = &context.file_output_mapping;
        tracing::debug!(
            thumbnail_dir = %thumnail_dir.to_string_lossy(),
            output_dir = %output_dir.to_string_lossy(),
            "Thumbnail preparation paths"
        );
        let res = context.thumbnail_generator.prepare_thumbnails(
            &thumnail_dir,
            output_dir,
            output_mapping,
        );
        match res {
            Ok(thumbnail_map) => {
                tracing::info!(
                    thumbnail_count = thumbnail_map.len(),
                    "Thumbnails prepared successfully"
                );
                context.thumbnail_path_map = thumbnail_map;
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    "Failed to prepare thumbnails"
                );
                // No need to abort the whole process for thumbnail generation failure
            }
        }
        StepAction::Continue
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf, sync::Arc};

    use cloud_storage::mock::MockCloudStorage;
    use core_types::{FileType, ImportedFile, Sha1Checksum};
    use database::{models::FileSet, repository_manager::RepositoryManager, setup_test_db};
    use file_export::{OutputFile, file_export_ops::MockFileExportOps};

    use crate::{
        file_set_download::{
            context::{DownloadContext, DownloadContextSettings},
            steps::{
                DownloadFilesStep, ExportFilesStep, FetchFileSetFileInfoStep, FetchFileSetStep,
                PrepareFileForDownloadStep, PrepareThumbnailsStep,
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

        let fs_ops = Arc::new(MockFileSystemOps::new());
        fs_ops.add_file(file_path.to_string_lossy().as_ref());
        context.fs_ops = fs_ops;

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
        let (mut context, mock_export_ops) = initialize_context(false).await;

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
        assert_eq!(mock_export_ops.total_calls(), 1);
        assert_eq!(mock_export_ops.export_zipped_calls().len(), 1);

        let call = &mock_export_ops.export_zipped_calls()[0];
        assert_eq!(call.output_file_names.len(), 1);
        assert!(!call.extract_files);
    }

    #[async_std::test]
    async fn test_export_files_step_with_extraction() {
        let (mut context, mock_export_ops) = initialize_context(true).await;

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
        assert_eq!(mock_export_ops.total_calls(), 1);
        assert_eq!(mock_export_ops.export_calls().len(), 1);

        let call = &mock_export_ops.export_calls()[0];
        assert!(call.extract_files);
    }

    #[async_std::test]
    async fn test_export_files_step_failure() {
        let (mut context, _) = initialize_context(false).await;

        context.export_ops = Arc::new(MockFileExportOps::with_failure("Disk full"));

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
        assert!(matches!(action, StepAction::Abort(_)));
        assert!(context.file_output_mapping.is_empty());
    }

    #[async_std::test]
    async fn test_prepare_thumbnails_step_skipped_when_extract_files_is_false() {
        let file_set = FileSet {
            id: 1,
            name: "Test File Set".to_string(),
            file_type: FileType::Screenshot,
            file_name: "test_file.zst".to_string(),
            source: "".to_string(),
        };

        let (mut context, _file_export_ops) = initialize_context(false).await;
        context.file_set = Some(file_set);
        context.file_output_mapping = HashMap::from([(
            "archive_file_name".to_string(),
            OutputFile {
                output_file_name: "file_name".to_string(),
                checksum: Sha1Checksum::from([1; 20]),
            },
        )]);

        let step = PrepareThumbnailsStep;
        let should_execute = step.should_execute(&context);
        assert!(!should_execute);
    }

    #[async_std::test]
    async fn test_prepare_thumbnails_step_skipped_for_non_image_file_type() {
        let file_set = FileSet {
            id: 1,
            name: "Test File Set".to_string(),
            file_type: FileType::Rom,
            file_name: "test_file.zst".to_string(),
            source: "".to_string(),
        };

        let (mut context, _file_export_ops) = initialize_context(true).await;
        context.file_set = Some(file_set);
        context.file_output_mapping = HashMap::from([(
            "archive_file_name".to_string(),
            OutputFile {
                output_file_name: "file_name".to_string(),
                checksum: Sha1Checksum::from([1; 20]),
            },
        )]);

        let step = PrepareThumbnailsStep;
        let should_execute = step.should_execute(&context);
        assert!(!should_execute);
    }

    #[async_std::test]
    async fn test_prepare_thumbnails_step_executed_for_image_file_type() {
        let file_set = FileSet {
            id: 1,
            name: "Test File Set".to_string(),
            file_type: FileType::Screenshot,
            file_name: "test_file.zst".to_string(),
            source: "".to_string(),
        };

        let (mut context, _file_export_ops) = initialize_context(true).await;
        context.file_set = Some(file_set);
        context.file_output_mapping = HashMap::from([(
            "archive_file_name".to_string(),
            OutputFile {
                output_file_name: "file_name".to_string(),
                checksum: Sha1Checksum::from([1; 20]),
            },
        )]);

        let step = PrepareThumbnailsStep;
        let should_execute = step.should_execute(&context);
        assert!(should_execute);
        let action = step.execute(&mut context).await;
        assert!(matches!(action, StepAction::Continue));
        assert!(context.thumbnail_path_map.len() == 1);
    }

    async fn initialize_context(extract_files: bool) -> (DownloadContext, Arc<MockFileExportOps>) {
        let pool = Arc::new(setup_test_db().await);
        let repository_manager = Arc::new(RepositoryManager::new(pool));
        let settings = Arc::new(Settings {
            collection_root_dir: PathBuf::from("/"),
            ..Default::default()
        });

        let settings_service = Arc::new(SettingsService::new(repository_manager.clone()));
        let cloud_ops = Arc::new(MockCloudStorage::new());

        let (tx, _rx) = async_std::channel::unbounded();
        let fs_ops = Arc::new(MockFileSystemOps::new());

        let export_ops = Arc::new(MockFileExportOps::new());
        let thumbnail_generator = Arc::new(thumbnails::ThumbnailGeneratorMock);

        let settings = DownloadContextSettings {
            repository_manager,
            settings,
            settings_service,
            progress_tx: Some(tx),
            file_set_id: 1,
            extract_files,
            cloud_ops: Some(cloud_ops),
            fs_ops,
            export_ops: export_ops.clone(),
            thumbnail_generator,
        };

        let context = DownloadContext::new(settings);

        (context, export_ops)
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
