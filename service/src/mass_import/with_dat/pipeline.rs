use crate::{
    mass_import::{
        common_steps::steps::{ReadFileMetadataStep, ReadFilesStep},
        with_dat::{
            context::MassImportContext,
            steps::{
                CheckExistingDatFileStep, FilterExistingFileSetsStep, ImportDatFileStep,
                ImportFileSetsStep, LinkExistingFileSetsStep, StoreDatFileStep,
            },
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
            Box::new(ReadFilesStep::<MassImportContext>::new()),
            Box::new(ReadFileMetadataStep::<MassImportContext>::new()),
            Box::new(FilterExistingFileSetsStep),
            Box::new(ImportFileSetsStep),
            Box::new(LinkExistingFileSetsStep),
        ])
    }
}
