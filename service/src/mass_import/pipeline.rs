use crate::{
    mass_import::{
        context::MassImportContext,
        steps::{
            CheckExistingDatFileStep, ImportDatFileStep, ImportFileSetsStep, ReadFileMetadataStep,
            ReadFilesStep, StoreDatFileStep,
        },
    },
    pipeline::generic_pipeline::Pipeline,
};

impl Pipeline<MassImportContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(ImportDatFileStep),
            Box::new(CheckExistingDatFileStep),
            Box::new(StoreDatFileStep),
            Box::new(ReadFilesStep),
            Box::new(ReadFileMetadataStep),
            Box::new(ImportFileSetsStep),
        ])
    }
}
