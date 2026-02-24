use crate::{
    mass_import::with_files_only::context::MassImportWithFilesOnlyContext,
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

pub struct BuildImportItemsFromFileNamesStep;

#[async_trait::async_trait]
impl PipelineStep<MassImportWithFilesOnlyContext> for BuildImportItemsFromFileNamesStep {
    fn name(&self) -> &'static str {
        "build_import_items_from_file_names"
    }
    async fn execute(&self, context: &mut MassImportWithFilesOnlyContext) -> StepAction {
        StepAction::Continue
    }
}

pub struct ImportFileSetsStep;

#[async_trait::async_trait]
impl PipelineStep<MassImportWithFilesOnlyContext> for ImportFileSetsStep {
    fn name(&self) -> &'static str {
        "import_file_sets"
    }
    async fn execute(&self, context: &mut MassImportWithFilesOnlyContext) -> StepAction {
        StepAction::Continue
    }
}
