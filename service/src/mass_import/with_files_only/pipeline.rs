use crate::{
    mass_import::{
        common_steps::steps::{ImportFileSetsStep, ReadFileMetadataStep, ReadFilesStep},
        with_files_only::{
            context::MassImportWithFilesOnlyContext, steps::BuildImportItemsFromFileNamesStep,
        },
    },
    pipeline::generic_pipeline::Pipeline,
};

impl Pipeline<MassImportWithFilesOnlyContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(ReadFilesStep::<MassImportWithFilesOnlyContext>::new()),
            Box::new(ReadFileMetadataStep::<MassImportWithFilesOnlyContext>::new()),
            Box::new(BuildImportItemsFromFileNamesStep),
            Box::new(ImportFileSetsStep::<MassImportWithFilesOnlyContext>::new()),
        ])
    }
}
