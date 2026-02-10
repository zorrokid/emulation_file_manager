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

// NOTE: currently this pipeline is designed so that same file set could tried to be added multiple times for example as part of import operations. That makes logic somewhat complicated and this pipeline has characteristics of both add file set and update file set pipelines. We also have update pipeline. So better design would be that caller would check if file set already exists and then decide whether to call add file set pipeline or update file set pipeline. That way we could simplify both pipelines and have more clear separation of concerns.
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
