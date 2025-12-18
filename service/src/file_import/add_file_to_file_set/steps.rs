use crate::{
    error::Error,
    file_import::add_file_to_file_set::context::AddFileToFileSetContext,
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

pub struct ValidateFileStep;

#[async_trait::async_trait]
impl PipelineStep<AddFileToFileSetContext> for ValidateFileStep {
    fn name(&self) -> &'static str {
        "validate_file"
    }

    async fn execute(&self, context: &mut AddFileToFileSetContext) -> StepAction {
        let zip_file_result = context.fs_ops.is_zip_archive(&context.file_path);
        match zip_file_result {
            Ok(is_zip) => {
                context.is_zip_archive = is_zip;
            }
            Err(err) => {
                tracing::error!(
                    error = %err,
                    file_path = %context.file_path.display(),
                    "Failed to check if file is a zip archive"
                );
                return StepAction::Abort(Error::IoError(format!(
                    "Failed to determine if file is a zip archive: {}",
                    err,
                )));
            }
        }

        StepAction::Continue
    }
}

// Note: this is almost identical to CollectFileMetadataStep in file_import/pipeline/steps.rs
// TODO: maybe share a common step implementation if possible
pub struct CollectFileContentStep;

#[async_trait::async_trait]
impl PipelineStep<AddFileToFileSetContext> for CollectFileContentStep {
    fn name(&self) -> &'static str {
        "collect_file_metadata"
    }

    async fn execute(&self, context: &mut AddFileToFileSetContext) -> StepAction {
        let file_contents_res = if context.is_zip_archive {
            context
                .file_import_ops
                .read_zip_contents_with_checksums(&context.file_path)
        } else {
            context
                .file_import_ops
                .read_file_checksum(&context.file_path)
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

                return StepAction::Abort(Error::IoError(format!(
                    "Failed to read file contents and checksums: {}",
                    err,
                )));
            }
        }

        StepAction::Continue
    }
}

pub struct UpdateDatabaseRecordsStep;

#[async_trait::async_trait]
impl PipelineStep<AddFileToFileSetContext> for UpdateDatabaseRecordsStep {
    fn name(&self) -> &'static str {
        "update_database_records"
    }
    fn should_execute(&self, context: &AddFileToFileSetContext) -> bool {
        !context.file_info.is_empty()
    }
    async fn execute(&self, context: &mut AddFileToFileSetContext) -> StepAction {
        // TODO: add file(s) to file set
        StepAction::Continue
    }
}

pub struct MarkFilesForCloudSyncStep;

#[async_trait::async_trait]
impl PipelineStep<AddFileToFileSetContext> for MarkFilesForCloudSyncStep {
    fn name(&self) -> &'static str {
        "mark_files_for_cloud_sync"
    }

    fn should_execute(&self, context: &AddFileToFileSetContext) -> bool {
        !context.file_info.is_empty()
    }
    async fn execute(&self, context: &mut AddFileToFileSetContext) -> StepAction {
        StepAction::Continue
    }
}
