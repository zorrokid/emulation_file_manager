use crate::{
    file_import::{
        common_steps::{
            check_existing_files::CheckExistingFilesStep, collect_file_info::CollectFileInfoStep,
        },
        prepare::{context::PrepareFileImportContext, steps::CollectFileMetadataStep},
    },
    pipeline::generic_pipeline::Pipeline,
};

impl Pipeline<PrepareFileImportContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(CollectFileMetadataStep),
            Box::new(CollectFileInfoStep::<PrepareFileImportContext>::new()),
            Box::new(CheckExistingFilesStep::<PrepareFileImportContext>::new()),
        ])
    }
}
