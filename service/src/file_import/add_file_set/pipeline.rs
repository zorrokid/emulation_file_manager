use crate::{
    file_import::{
        add_file_set::{
            context::AddFileSetContext,
            steps::{AddFileSetItemTypesStep, CreateFileSetToDatabaseStep},
        },
        common_steps::{
            check_existing_file_set::CheckExistingFileSetStep,
            check_existing_files::CheckExistingFilesStep, import::ImportFilesStep,
        },
    },
    pipeline::generic_pipeline::Pipeline,
};

// NOTE: This add pipeline still handles cases where the file set may already exist,
// for example during import flows. That makes the flow overlap somewhat with update
// behavior. A cleaner design would move the add-vs-update routing into a higher-level
// service/orchestration layer after the shared pre-checks, so the add and update
// pipelines can have clearer responsibilities.
impl Pipeline<AddFileSetContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(CheckExistingFilesStep::<AddFileSetContext>::new()),
            Box::new(CheckExistingFileSetStep::<AddFileSetContext>::new()),
            Box::new(ImportFilesStep::<AddFileSetContext>::new()),
            Box::new(CreateFileSetToDatabaseStep),
            //Box::new(CreateReleaseForExistingFileSetStep),
            //Box::new(LinkExistingFileSetToReleaseStep),
            // TODO: since we check existing file set, we should probably skip this step if the
            // file set already exists (or check if item types are already set)
            Box::new(AddFileSetItemTypesStep),
        ])
    }
}
