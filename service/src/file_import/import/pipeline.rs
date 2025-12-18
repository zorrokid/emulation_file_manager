use crate::{
    file_import::{
        common_steps::import::ImportFilesStep,
        import::{context::FileImportContext, steps::UpdateDatabaseStep},
    },
    pipeline::generic_pipeline::Pipeline,
};

impl Pipeline<FileImportContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(ImportFilesStep::<FileImportContext>::new()),
            Box::new(UpdateDatabaseStep),
        ])
    }
}
