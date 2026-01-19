use crate::{
    mass_import::{context::MassImportContext, steps::ReadFilesStep},
    pipeline::generic_pipeline::Pipeline,
};

impl Pipeline<MassImportContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![Box::new(ReadFilesStep)])
    }
}
