use core_types::Sha1Checksum;
use utils::file_util;

use crate::{
    error::Error,
    pipeline::pipeline_step::{PipelineStep, StepAction},
    prepare_file_import::context::{FileImportMetadata, PrepareFileImportContext},
};

pub struct CollectFileMetadataStep;

#[async_trait::async_trait]
impl PipelineStep<PrepareFileImportContext> for CollectFileMetadataStep {
    fn name(&self) -> &'static str {
        "collect_file_metadata"
    }

    async fn execute(&self, context: &mut PrepareFileImportContext) -> StepAction {
        // Implementation for collecting file metadata goes here.
        let file_set_name = context
            .file_path
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_string());
        let file_set_file_name = context
            .file_path
            .file_name()
            .map(|name| name.to_string_lossy().to_string());

        let zip_file_result = file_util::is_zip_file(&context.file_path);

        match zip_file_result {
            Ok(is_zip_archive) => {
                context.import_metadata = Some(FileImportMetadata {
                    file_set_name,
                    file_set_file_name,
                    is_zip_archive,
                });
                StepAction::Continue
            }
            Err(err) => {
                tracing::error!(
                    error = %err,
                    file_path = %context.file_path.display(),
                    "Failed to check if file is zip archive"
                );

                StepAction::Abort(Error::IoError(
                    "Failed to determine if file is zip archive".into(),
                ))
            }
        }
    }
}

pub struct CollectFileContentStep;

#[async_trait::async_trait]
impl PipelineStep<PrepareFileImportContext> for CollectFileContentStep {
    fn name(&self) -> &'static str {
        "collect_file_metadata"
    }

    fn should_execute(&self, context: &PrepareFileImportContext) -> bool {
        context.import_metadata.is_some()
    }

    async fn execute(&self, context: &mut PrepareFileImportContext) -> StepAction {
        let is_zip = context.import_metadata.as_ref().unwrap().is_zip_archive;
        let file_contents_res = match is_zip {
            true => file_import::read_zip_contents_with_checksums(&context.file_path),
            false => file_import::read_file_checksum(&context.file_path),
        };

        match file_contents_res {
            Ok(file_contents) => {
                context.file_info = file_contents;
            }
            Err(err) => {
                tracing::error!(
                    error = %err,
                    file_path = %context.file_path.display(),
                    "Failed to read file contents and checksums"
                );

                return StepAction::Abort(Error::IoError(
                    "Failed to read file contents and checksums".into(),
                ));
            }
        }

        StepAction::Continue
    }
}

pub struct ProcessFileContentStep;

#[async_trait::async_trait]
impl PipelineStep<PrepareFileImportContext> for ProcessFileContentStep {
    fn name(&self) -> &'static str {
        "process_file_content"
    }

    fn should_execute(&self, context: &PrepareFileImportContext) -> bool {
        !context.file_info.is_empty()
    }

    async fn execute(&self, context: &mut PrepareFileImportContext) -> StepAction {
        let file_checksums = context
            .file_info
            .keys()
            .cloned()
            .collect::<Vec<Sha1Checksum>>();

        let existing_files_res = context
            .repository_manager
            .get_file_info_repository()
            .get_file_infos_by_sha1_checksums(file_checksums, context.file_type)
            .await;

        match existing_files_res {
            Ok(existing_files_file_info) => {
                tracing::info!(
                    existing_file_count = existing_files_file_info.len(),
                    "Fetched existing file info from repository"
                );
                context.existing_files = existing_files_file_info;

                StepAction::Continue
            }
            Err(err) => {
                tracing::error!(
                    error = %err,
                    "Failed to fetch existing file info from repository"
                );
                StepAction::Abort(Error::DbError("Failed to fetch existing file info".into()))
            }
        }
    }
}
