use crate::{
    mass_import::{
        context::MassImportContext,
        steps::{ImportDatFileStep, ReadFileMetadataStep, ReadFilesStep},
    },
    pipeline::generic_pipeline::Pipeline,
};

impl Pipeline<MassImportContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(ImportDatFileStep),
            Box::new(ReadFilesStep),
            Box::new(ReadFileMetadataStep),
        ])
    }
}
