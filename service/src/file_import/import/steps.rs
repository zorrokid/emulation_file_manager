use crate::{
    error::Error,
    file_import::import::context::FileImportContext,
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

pub struct ImportFilesStep;
#[async_trait::async_trait]
impl PipelineStep<FileImportContext> for ImportFilesStep {
    fn name(&self) -> &'static str {
        "import_files"
    }
    async fn execute(&self, context: &mut FileImportContext) -> StepAction {
        match context
            .file_import_ops
            .import(&context.get_file_import_model())
        {
            Ok(imported_files) => {
                context.imported_files = imported_files;
            }
            Err(err) => {
                tracing::error!("Error importing files: {}", err);
                return StepAction::Abort(Error::FileImportError(format!(
                    "Error importing files: {}",
                    err
                )));
            }
        }
        StepAction::Continue
    }
}

pub struct UpdateDatabaseStep;

#[async_trait::async_trait]
impl PipelineStep<FileImportContext> for UpdateDatabaseStep {
    fn name(&self) -> &'static str {
        "update_database"
    }
    async fn execute(&self, context: &mut FileImportContext) -> StepAction {
        match context
            .repository_manager
            .get_file_set_repository()
            .add_file_set(
                &context.file_set_name,
                &context.file_set_file_name,
                &context.file_type,
                &context.source,
                &context.get_files_in_file_set(),
                &context.system_ids,
            )
            .await
        {
            Ok(id) => {
                tracing::info!(
                    "File set '{}' with id {} added to database",
                    context.file_set_name,
                    id
                );
                context.file_set_id = Some(id);
            }
            Err(err) => {
                tracing::error!(
                    "Error adding file set '{}' to database: {}",
                    context.file_set_name,
                    err
                );
                // TODO: if this fails, we should probably roll back the imported files because
                // they are now orphaned.
                return StepAction::Abort(Error::DbError(format!(
                    "Error adding file set to database: {}",
                    err
                )));
            }
        }

        StepAction::Continue
    }
}
