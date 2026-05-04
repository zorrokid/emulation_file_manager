use crate::{
    libretro::error::LibretroPreflightError,
    libretro::runner::prepare::context::PrepareLaunchContext, pipeline::generic_pipeline::Pipeline,
};

impl Pipeline<PrepareLaunchContext, LibretroPreflightError> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(crate::libretro::runner::prepare::steps::DownloadFileSetStep),
            Box::new(crate::libretro::runner::prepare::steps::SelectLaunchFileStep),
            Box::new(crate::libretro::runner::prepare::steps::ValidateFirmwareStep),
            Box::new(crate::libretro::runner::prepare::steps::ValidateExtensionStep),
            Box::new(crate::libretro::runner::prepare::steps::BuildLaunchPathsStep),
        ])
    }
}
