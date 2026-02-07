use crate::{
    file_import::{
        add_file_set::{
            context::AddFileSetContext,
            steps::{AddFileSetItemTypesStep, UpdateDatabaseStep},
        },
        common_steps::{
            check_existing_file_set::CheckExistingFileSetStep,
            check_existing_files::CheckExistingFilesStep, import::ImportFilesStep,
        },
    },
    pipeline::generic_pipeline::Pipeline,
};

impl Pipeline<AddFileSetContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(CheckExistingFilesStep::<AddFileSetContext>::new()),
            Box::new(CheckExistingFileSetStep::<AddFileSetContext>::new()),
            Box::new(ImportFilesStep::<AddFileSetContext>::new()),
            Box::new(UpdateDatabaseStep),
            // TODO: since we check existing file set, we should probably skip this step if the
            // file set already exists (or check if item types are already set)
            Box::new(AddFileSetItemTypesStep),
        ])
    }
}
