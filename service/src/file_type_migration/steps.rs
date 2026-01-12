use crate::{
    file_type_migration::context::FileTypeMigrationContext,
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

pub struct CollectFilesStep;

#[async_trait::async_trait]
impl PipelineStep<FileTypeMigrationContext> for CollectFilesStep {
    fn name(&self) -> &'static str {
        "collect_files_step"
    }

    async fn execute(&self, context: &mut FileTypeMigrationContext) -> StepAction {
        // TODO: get files from the database and populate context.old_file_type
        /*let files = context
        .repository_manager
        .get_file_info_repository()
        .get_all_files()
        .await;*/
        StepAction::Continue
    }
}

pub struct MapOldFileTypesToNewStep;

pub struct MoveLocalFilesStep;

pub struct MoveCloudFilesStep;

pub struct UpdateDatabaseStep;
