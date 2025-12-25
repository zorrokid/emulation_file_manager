use database::models::FileInfo;

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
        // TODO: add test with non existing file set
        let file_set_result = context
            .repository_manager
            .get_file_set_repository()
            .get_file_set(context.file_set_id)
            .await;

        match file_set_result {
            Ok(file_set) => {
                tracing::info!(
                    file_set_id = %context.file_set_id,
                    "File set exists in database"
                );
                context.file_set = Some(file_set);
            }
            Err(err) => {
                tracing::error!(
                    error = %err,
                    file_set_id = %context.file_set_id,
                    "Error checking if file set exists in database"
                );
                return StepAction::Abort(Error::DbError(format!(
                    "Error checking if file set exists: {}",
                    err,
                )));
            }
        }

        let zip_file_result = context.fs_ops.is_zip_archive(&context.file_path);
        match zip_file_result {
            Ok(is_zip) => {
                context.is_zip_archive = Some(is_zip);
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

pub struct AddFileInfoToDatabaseStep;

#[async_trait::async_trait]
impl PipelineStep<AddFileToFileSetContext> for AddFileInfoToDatabaseStep {
    fn name(&self) -> &'static str {
        "add_file_info_to_database"
    }

    fn should_execute(&self, context: &AddFileToFileSetContext) -> bool {
        !context
            .file_import_data
            .is_new_files_to_be_imported(&context.existing_files)
            && !context.imported_files.is_empty()
            && context.file_set.is_some()
    }

    async fn execute(&self, context: &mut AddFileToFileSetContext) -> StepAction {
        let file_type = context.file_set.as_ref().unwrap().file_type;
        for imported_file in context.imported_files.values() {
            let add_file_info_result = context
                .repository_manager
                .get_file_info_repository()
                .add_file_info(
                    &imported_file.sha1_checksum,
                    imported_file.file_size as i64,
                    &imported_file.archive_file_name,
                    file_type,
                )
                .await;
            match add_file_info_result {
                Ok(id) => {
                    tracing::info!(
                        file_count = context.file_info.len(),
                        "Added file info records to database"
                    );
                    context.new_files.push(FileInfo {
                        id,
                        sha1_checksum: imported_file.sha1_checksum.into(),
                        file_size: imported_file.file_size,
                        archive_file_name: imported_file.archive_file_name.clone(),
                        file_type,
                    });
                }
                Err(err) => {
                    tracing::error!(
                        error = %err,
                        "Error adding file info records to database"
                    );
                    // TODO: collect failed
                }
            }
        }
        StepAction::Continue
    }
}

pub struct UpdateFileSetStep;

#[async_trait::async_trait]
impl PipelineStep<AddFileToFileSetContext> for UpdateFileSetStep {
    fn name(&self) -> &'static str {
        "update_file_set"
    }

    async fn execute(&self, context: &mut AddFileToFileSetContext) -> StepAction {
        let file_info_ids_with_file_names = context.get_file_info_ids_with_file_names();
        let result = context
            .repository_manager
            .get_file_set_repository()
            .add_files_to_file_set(context.file_set_id, &file_info_ids_with_file_names)
            .await;

        match result {
            Ok(_) => {
                tracing::info!(
                    file_set_id = %context.file_set_id,
                    added_file_count = file_info_ids_with_file_names.len(),
                    "Added files to file set in database"
                );
            }
            Err(err) => {
                tracing::error!(
                    error = %err,
                    file_set_id = %context.file_set_id,
                    "Error adding files to file set in database"
                );
                // TODO: should files and file infos be removed?
                return StepAction::Abort(Error::DbError(format!(
                    "Error adding files to file set: {}",
                    err,
                )));
            }
        }
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
