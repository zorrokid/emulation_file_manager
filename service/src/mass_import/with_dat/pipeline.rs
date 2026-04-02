use crate::{
    mass_import::{
        common_steps::steps::{ReadFileMetadataStep, ReadFilesStep},
        with_dat::{
            context::DatFileMassImportContext,
            route_and_process_step::RouteAndProcessFileSetsStep,
            steps::{
                CategorizeFileSetsForImportStep, CheckExistingDatFileStep, ImportDatFileStep,
                StoreDatFileStep,
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
            Box::new(RouteAndProcessFileSetsStep),
        ])
    }
}
