use crate::{
    libretro_runner::{
        prepare::context::PrepareLaunchContext,
        service::{LibretroLaunchPaths, LibretroPreflightError},
    },
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

pub struct DownloadFileSetStep;
pub struct SelectLaunchFileStep;
pub struct ValidateFirmwareStep;
pub struct ValidateExtensionStep;
pub struct BuildLaunchPathsStep;

/// Downloads the file set and extracts it if necessary. If the download fails, an error is returned. If some files fail to download, the successful downloads are kept and an error is
/// returned with the count of failed downloads. The download results are stored in the context
/// state for use in subsequent steps.
#[async_trait::async_trait]
impl PipelineStep<PrepareLaunchContext, LibretroPreflightError> for DownloadFileSetStep {
    fn name(&self) -> &'static str {
        "download_file_set"
    }

    async fn execute(
        &self,
        context: &mut PrepareLaunchContext,
    ) -> StepAction<LibretroPreflightError> {
        let result = context
            .deps
            .download_service
            .download_file_set(
                context.input.file_set_id,
                context.input.extract_files,
                context.deps.progress_tx.clone(),
            )
            .await;
        match result {
            Ok(results) => {
                tracing::info!(
                    successful_downloads = results.successful_downloads,
                    failed_downloads = results.failed_downloads
                );
                if !results.errors.is_empty() {
                    tracing::warn!("Download completed with errors: {:?}", results.errors);
                }
                if results.failed_downloads > 0 {
                    return StepAction::Abort(LibretroPreflightError::DownloadError(format!(
                        "{} files failed to download",
                        results.failed_downloads
                    )));
                }
                context.state.download_results = Some(results);
                StepAction::Continue
            }
            Err(e) => StepAction::Abort(LibretroPreflightError::DownloadError(e.to_string())),
        }
    }
}

/// Selects the launch file from the downloaded file set. If an initial file is specified, it will
/// be selected if it exists in the file set. Otherwise, the first file in the set will be
/// selected. If no files are available, an error is returned.
#[async_trait::async_trait]
impl PipelineStep<PrepareLaunchContext, LibretroPreflightError> for SelectLaunchFileStep {
    fn name(&self) -> &'static str {
        "select_launch_file"
    }

    fn should_execute(&self, context: &PrepareLaunchContext) -> bool {
        context.state.selected_file.is_none() && context.state.download_results.is_some()
    }

    async fn execute(
        &self,
        context: &mut PrepareLaunchContext,
    ) -> StepAction<LibretroPreflightError> {
        let download_results = context
            .state
            .download_results
            .as_ref()
            .expect("download_results are tested in should_execute");

        let file_name = if let Some(initial) = &context.input.initial_file {
            if download_results.output_file_names.contains(initial) {
                initial.clone()
            } else {
                return StepAction::Abort(LibretroPreflightError::InvalidInitialFile(
                    initial.clone(),
                ));
            }
        } else if let Some(first) = download_results.output_file_names.first() {
            first.clone()
        } else {
            return StepAction::Abort(LibretroPreflightError::NoFileInFileSet);
        };

        context.state.selected_file = Some(file_name);
        StepAction::Continue
    }
}

#[async_trait::async_trait]
impl PipelineStep<PrepareLaunchContext, LibretroPreflightError> for ValidateFirmwareStep {
    fn name(&self) -> &'static str {
        "validate_firmware"
    }

    fn should_execute(&self, context: &PrepareLaunchContext) -> bool {
        context.input.core_info.firmware_info.iter().any(|f| !f.opt)
    }

    async fn execute(
        &self,
        context: &mut PrepareLaunchContext,
    ) -> StepAction<LibretroPreflightError> {
        // TODO: firmware availabilitys is pre-validated when reading core info but it could be
        // moved here instead.

        let non_available_firmware = context
            .input
            .core_info
            .firmware_info
            .iter()
            .filter(|f| !f.opt && !f.available)
            .map(|f| f.desc.clone())
            .collect::<Vec<_>>();
        if !non_available_firmware.is_empty() {
            return StepAction::Abort(LibretroPreflightError::FirmwareNotAvailable(
                non_available_firmware.join(", "),
            ));
        }
        StepAction::Continue
    }
}

#[async_trait::async_trait]
impl PipelineStep<PrepareLaunchContext, LibretroPreflightError> for ValidateExtensionStep {
    fn name(&self) -> &'static str {
        "validate_extension"
    }

    fn should_execute(&self, context: &PrepareLaunchContext) -> bool {
        context.state.selected_file.is_some()
            && !context.input.core_info.supported_extensions.is_empty()
    }

    async fn execute(
        &self,
        context: &mut PrepareLaunchContext,
    ) -> StepAction<LibretroPreflightError> {
        let selected_file = context
            .state
            .selected_file
            .as_ref()
            .expect("selected_file is tested in should_execute");
        let extension = std::path::Path::new(selected_file)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default()
            .to_lowercase();

        if !context
            .input
            .core_info
            .supported_extensions
            .iter()
            .any(|ext| ext.to_lowercase() == extension)
        {
            return StepAction::Abort(LibretroPreflightError::UnsupportedExtension(extension));
        }
        StepAction::Continue
    }
}

#[async_trait::async_trait]
impl PipelineStep<PrepareLaunchContext, LibretroPreflightError> for BuildLaunchPathsStep {
    fn name(&self) -> &'static str {
        "build_launch_paths"
    }

    fn should_execute(&self, context: &PrepareLaunchContext) -> bool {
        context.state.selected_file.is_some()
    }

    async fn execute(
        &self,
        context: &mut PrepareLaunchContext,
    ) -> StepAction<LibretroPreflightError> {
        let selected_file = context
            .state
            .selected_file
            .as_ref()
            .expect("selected_file is tested in should_execute");

        let system_dir = match &context.deps.settings.libretro_system_dir {
            Some(dir) => dir,
            None => {
                return StepAction::Abort(LibretroPreflightError::SystemDirNotSet);
            }
        };

        let launch_paths = LibretroLaunchPaths {
            rom_path: context.deps.settings.temp_output_dir.join(selected_file),
            core_path: context.input.core_path.clone(),
            system_dir: system_dir.clone(),
            temp_files: vec![selected_file.clone()],
        };

        context.state.launch_paths = Some(launch_paths);

        StepAction::Continue
    }
}
