use crate::{
    mass_import::{
        context::MassImportContext,
        steps::{ImportDatFileStep, ImportFileSetsStep, ReadFileMetadataStep, ReadFilesStep},
    },
    pipeline::generic_pipeline::Pipeline,
};

impl Pipeline<MassImportContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(ImportDatFileStep),
            Box::new(ReadFilesStep),
            Box::new(ReadFileMetadataStep),
            Box::new(ImportFileSetsStep),
        ])
    }
}
