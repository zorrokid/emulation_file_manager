use crate::{
    file_import::{
        add_file_set::{
            context::AddFileSetContext,
            steps::{AddFileSetItemTypesStep, UpdateDatabaseStep},
        },
        common_steps::{check_existing_files::CheckExistingFilesStep, import::ImportFilesStep},
    },
    pipeline::generic_pipeline::Pipeline,
};

impl Pipeline<AddFileSetContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(CheckExistingFilesStep::<AddFileSetContext>::new()),
            // TODO: check if identical file set already exists and skip import if so and just
            // return the existing file set id
            // probably FileSetItemTypes should be checked as well
            //Box::new(CheckExistingFileSetStep),
            Box::new(ImportFilesStep::<AddFileSetContext>::new()),
            Box::new(UpdateDatabaseStep),
            Box::new(AddFileSetItemTypesStep),
        ])
    }
}
