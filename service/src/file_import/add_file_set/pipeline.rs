use crate::{
    file_import::{
        add_file_set::{context::AddFileSetContext, steps::UpdateDatabaseStep},
        common_steps::{check_existing_files::CheckExistingFilesStep, import::ImportFilesStep},
    },
    pipeline::generic_pipeline::Pipeline,
};

impl Pipeline<AddFileSetContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(CheckExistingFilesStep::<AddFileSetContext>::new()),
            Box::new(ImportFilesStep::<AddFileSetContext>::new()),
            Box::new(UpdateDatabaseStep),
        ])
    }
}
