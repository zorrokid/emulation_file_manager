use crate::{
    file_import::import::{
        context::FileImportContext,
        steps::{ImportFilesStep, UpdateDatabaseStep},
    },
    pipeline::generic_pipeline::Pipeline,
};

impl Pipeline<FileImportContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(ImportFilesStep),
            Box::new(UpdateDatabaseStep),
        ])
    }
}
