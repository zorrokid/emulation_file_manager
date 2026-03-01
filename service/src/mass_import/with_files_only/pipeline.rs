use crate::{
    mass_import::{
        common_steps::steps::{ImportFileSetsStep, ReadFileMetadataStep, ReadFilesStep},
        with_files_only::context::MassImportWithFilesOnlyContext,
    },
    pipeline::generic_pipeline::Pipeline,
};

impl Pipeline<MassImportWithFilesOnlyContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(ReadFilesStep::<MassImportWithFilesOnlyContext>::new()),
            Box::new(ReadFileMetadataStep::<MassImportWithFilesOnlyContext>::new()),
            Box::new(ImportFileSetsStep::<MassImportWithFilesOnlyContext>::new()),
        ])
    }
}
