use crate::{
    file_import::{
        common_steps::{check_existing_files::CheckExistingFilesStep, import::ImportFilesStep},
        import::{context::FileImportContext, steps::UpdateDatabaseStep},
    },
    pipeline::generic_pipeline::Pipeline,
};

impl Pipeline<FileImportContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(CheckExistingFilesStep::<FileImportContext>::new()),
            Box::new(ImportFilesStep::<FileImportContext>::new()),
            Box::new(UpdateDatabaseStep),
        ])
    }
}
