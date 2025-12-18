use crate::{
    file_import::{
        common_steps::check_existing_files::CheckExistingFilesStep,
        prepare::{
            context::PrepareFileImportContext,
            steps::{CollectFileContentStep, CollectFileMetadataStep},
        },
    },
    pipeline::generic_pipeline::Pipeline,
};

impl Pipeline<PrepareFileImportContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(CollectFileMetadataStep),
            Box::new(CollectFileContentStep),
            Box::new(CheckExistingFilesStep::<PrepareFileImportContext>::new()),
        ])
    }
}
