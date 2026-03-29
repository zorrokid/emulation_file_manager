use crate::{
    mass_import::{
        common_steps::steps::{ImportFileSetsStep, ReadFileMetadataStep, ReadFilesStep},
        with_dat::{
            context::DatFileMassImportContext,
            steps::{
                CategorizeFileSetsForImportStep, CheckExistingDatFileStep,
                HandleExistingFileSetsStep, ImportDatFileStep, StoreDatFileStep,
            },
        },
    },
    pipeline::generic_pipeline::Pipeline,
};

impl Pipeline<DatFileMassImportContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(ImportDatFileStep),
            Box::new(CheckExistingDatFileStep),
            Box::new(StoreDatFileStep),
            Box::new(ReadFilesStep::<DatFileMassImportContext>::new()),
            Box::new(ReadFileMetadataStep::<DatFileMassImportContext>::new()),
            Box::new(CategorizeFileSetsForImportStep),
            Box::new(ImportFileSetsStep::<DatFileMassImportContext>::new()),
            Box::new(HandleExistingFileSetsStep),
        ])
    }
}
