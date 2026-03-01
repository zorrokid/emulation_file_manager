use crate::{
    mass_import::{
        common_steps::steps::{ImportFileSetsStep, ReadFileMetadataStep, ReadFilesStep},
        with_files_only::{context::FilesOnlyMassImportContext, steps::FilterExistingFileSetsStep},
    },
    pipeline::generic_pipeline::Pipeline,
};

impl Pipeline<FilesOnlyMassImportContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(ReadFilesStep::<FilesOnlyMassImportContext>::new()),
            Box::new(ReadFileMetadataStep::<FilesOnlyMassImportContext>::new()),
            Box::new(FilterExistingFileSetsStep),
            Box::new(ImportFileSetsStep::<FilesOnlyMassImportContext>::new()),
        ])
    }
}
