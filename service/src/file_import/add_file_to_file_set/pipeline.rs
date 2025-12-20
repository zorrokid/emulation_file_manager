use crate::{
    file_import::{
        add_file_to_file_set::{context::AddFileToFileSetContext, steps::ValidateFileStep},
        common_steps::{collect_file_info::CollectFileInfoStep, import::ImportFilesStep},
    },
    pipeline::generic_pipeline::Pipeline,
};

impl Pipeline<AddFileToFileSetContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(ValidateFileStep),
            Box::new(CollectFileInfoStep::<AddFileToFileSetContext>::new()),
            Box::new(ImportFilesStep::<AddFileToFileSetContext>::new()),
        ])
    }
}
