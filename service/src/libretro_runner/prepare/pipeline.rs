use crate::{
    libretro_runner::{prepare::context::PrepareLaunchContext, service::LibretroPreflightError},
    pipeline::generic_pipeline::Pipeline,
};

impl Pipeline<PrepareLaunchContext, LibretroPreflightError> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(crate::libretro_runner::prepare::steps::DownloadFileSetStep),
            Box::new(crate::libretro_runner::prepare::steps::SelectLaunchFileStep),
            Box::new(crate::libretro_runner::prepare::steps::ValidateFirmwareStep),
            Box::new(crate::libretro_runner::prepare::steps::ValidateExtensionStep),
            Box::new(crate::libretro_runner::prepare::steps::BuildLaunchPathsStep),
        ])
    }
}
