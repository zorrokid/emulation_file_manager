use crate::{
    file_import::{
        add_file_to_file_set::{
            context::AddFileToFileSetContext,
            steps::{CollectFileContentStep, ValidateFileStep},
        },
        common_steps::import::ImportFilesStep,
    },
    pipeline::generic_pipeline::Pipeline,
};

impl Pipeline<AddFileToFileSetContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(ValidateFileStep),
            Box::new(CollectFileContentStep),
            Box::new(ImportFilesStep::<AddFileToFileSetContext>::new()),
        ])
    }
}
