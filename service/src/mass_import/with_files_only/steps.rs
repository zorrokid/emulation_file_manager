use domain::title_normalizer::get_canonical_software_title;

use crate::{
    file_import::model::{
        CreateReleaseParams, FileImportSource, FileSetImportModel, ImportFileContent,
    },
    mass_import::{
        common_steps::context::MassImportContextOps,
        with_files_only::context::MassImportWithFilesOnlyContext,
    },
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

pub struct BuildImportItemsFromFileNamesStep;

#[async_trait::async_trait]
impl PipelineStep<MassImportWithFilesOnlyContext> for BuildImportItemsFromFileNamesStep {
    fn name(&self) -> &'static str {
        "build_import_items_from_file_names"
    }
    async fn execute(&self, context: &mut MassImportWithFilesOnlyContext) -> StepAction {
        let system_id = context.input.system_id;
        let file_type = context.input.file_type;
        let item_type = context.input.item_type;
        let source = context.input.source.clone();
        let file_metadata = &context.file_metadata();
        let mut file_import_sets: Vec<FileSetImportModel> = vec![];
        for (file_path, metadata) in file_metadata.iter() {
            let file_name = file_path.file_name().unwrap().to_string_lossy().to_string();
            let file_name_without_extension =
                file_path.file_stem().unwrap().to_string_lossy().to_string();

            let software_title = get_canonical_software_title(&file_name_without_extension);

            let file_set_import_model = FileSetImportModel {
                file_set_name: file_path.file_stem().unwrap().to_string_lossy().to_string(),
                file_set_file_name: file_path.file_name().unwrap().to_string_lossy().to_string(),
                import_files: vec![FileImportSource {
                    path: file_path.clone(),
                    content: metadata
                        .iter()
                        .map(|f| {
                            (
                                f.sha1_checksum,
                                ImportFileContent {
                                    file_name: file_name.clone(),
                                    file_size: f.file_size,
                                    sha1_checksum: f.sha1_checksum,
                                },
                            )
                        })
                        .collect(),
                }],
                selected_files: metadata.iter().map(|meta| meta.sha1_checksum).collect(),
                system_ids: vec![system_id],
                source: source.clone(),
                file_type,
                item_ids: vec![],
                item_types: item_type.into_iter().collect(),
                create_release: Some(CreateReleaseParams {
                    software_title_name: software_title,
                    release_name: "".to_string(), // TODO: improve later,
                }),
                dat_file_id: None,
            };
            file_import_sets.push(file_set_import_model);
        }
        StepAction::Continue
    }
}
