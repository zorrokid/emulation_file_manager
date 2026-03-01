use core_types::{FileSetEqualitySpecs, FileSetFileEqualitySpecs};

use crate::{
    error::Error,
    mass_import::{
        common_steps::context::MassImportContextOps,
        with_files_only::context::MassImportWithFilesOnlyContext,
    },
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

pub struct FilterExistingFileSetsStep;

/// Filter out files from file_metadata that already have file sets in the system so that they
/// won't be imported again.
#[async_trait::async_trait]
impl PipelineStep<MassImportWithFilesOnlyContext> for FilterExistingFileSetsStep {
    fn name(&self) -> &'static str {
        "filter_existing_file_sets"
    }

    fn should_execute(&self, context: &MassImportWithFilesOnlyContext) -> bool {
        !context.state.file_metadata.is_empty()
    }

    async fn execute(&self, context: &mut MassImportWithFilesOnlyContext) -> StepAction {
        let file_set_import_models = context.get_import_file_sets();
        let repository_manager = context.deps.repository_manager.clone();
        let file_type = context.input.file_type;
        for file_set_import_model in file_set_import_models {
            let mut file_set_file_info: Vec<FileSetFileEqualitySpecs> = Vec::new();

            let file_contents = file_set_import_model
                .import_files
                .iter()
                .flat_map(|import_file| import_file.content.values().clone())
                .collect::<Vec<_>>();

            for file in file_contents {
                file_set_file_info.push(FileSetFileEqualitySpecs {
                    file_name: file.file_name.clone(),
                    file_type,
                    sha1_checksum: file.sha1_checksum,
                });
            }

            let file_set_equality_specs = FileSetEqualitySpecs {
                file_set_name: file_set_import_model.file_set_name.clone(),
                file_set_file_name: file_set_import_model.file_set_file_name.clone(),
                file_type,
                source: context.input.source.clone(),
                file_set_file_info,
            };

            let existing_file_set_res = repository_manager
                .get_file_set_repository()
                .find_file_set(&file_set_equality_specs)
                .await;

            match existing_file_set_res {
                Ok(existing_file_set) => {
                    if existing_file_set.is_some() {
                        tracing::info!(
                            "File set '{}' already exists in the system, skipping import for file '{}'",
                            file_set_import_model.file_set_name,
                            file_set_import_model.import_files[0].path.display()
                        );
                        context
                            .state
                            .file_metadata
                            .remove(&file_set_import_model.import_files[0].path);
                    }
                }
                Err(e) => {
                    return StepAction::Abort(Error::DbError(format!(
                        "Error checking for existing file set: {}",
                        e
                    )));
                }
            }
        }

        StepAction::Continue
    }
}
