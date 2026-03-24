use crate::{
    mass_import::{
        common_steps::steps::{ImportFileSetsStep, ReadFileMetadataStep, ReadFilesStep},
        with_dat::{
            context::DatFileMassImportContext,
            steps::{
                CheckExistingDatFileStep, FilterExistingFileSetsStep, ImportDatFileStep,
                LinkExistingFileSetsStep, StoreDatFileStep,
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
            // Read files from given source path
            Box::new(ReadFilesStep::<DatFileMassImportContext>::new()),
            // Read file metadata for read files using reader based on type of file
            // for example either metadata of a single file or metadata of each file in a container file like
            // zip
            Box::new(ReadFileMetadataStep::<DatFileMassImportContext>::new()),
            // Filter out file sets that already exist in the database
            Box::new(FilterExistingFileSetsStep),
            Box::new(ImportFileSetsStep::<DatFileMassImportContext>::new()),
            Box::new(LinkExistingFileSetsStep),
        ])
    }
}
