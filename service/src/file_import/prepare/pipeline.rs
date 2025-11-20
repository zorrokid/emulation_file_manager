use crate::{
    file_import::prepare::{
        context::PrepareFileImportContext,
        steps::{CollectFileContentStep, CollectFileMetadataStep, ProcessFileContentStep},
    },
    pipeline::generic_pipeline::Pipeline,
};

impl Pipeline<PrepareFileImportContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(CollectFileMetadataStep),
            Box::new(CollectFileContentStep),
            Box::new(ProcessFileContentStep),
        ])
    }
}
