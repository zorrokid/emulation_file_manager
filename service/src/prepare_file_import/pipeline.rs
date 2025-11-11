use crate::{
    pipeline::generic_pipeline::Pipeline,
    prepare_file_import::{
        context::PrepareFileImportContext,
        steps::{CollectFileContentStep, CollectFileMetadataStep, ProcessFileContentStep},
    },
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
