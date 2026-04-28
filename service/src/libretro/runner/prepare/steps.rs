use crate::{
    libretro::error::LibretroPreflightError,
    libretro::runner::{prepare::context::PrepareLaunchContext, service::LibretroLaunchPaths},
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

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, sync::Arc};

    use database::setup_test_repository_manager;
    use libretro_runner::supported_cores::InputProfile;

    use super::*;
    use crate::{
        file_set_download::{
            download_service_ops::{DownloadServiceOps, MockDownloadServiceOps},
            service::{DownloadResult, DownloadService},
        },
        libretro::core::service::{LibretroCoreInfo, LibretroFirmwareInfo},
        libretro::runner::prepare::context::{
            PrepareLaunchContextDeps, PrepareLaunchContextInput, PrepareLaunchContextState,
        },
        view_models::Settings,
    };

    fn create_core_info(
        supported_extensions: Vec<&str>,
        firmware_info: Vec<LibretroFirmwareInfo>,
    ) -> LibretroCoreInfo {
        LibretroCoreInfo {
            core_name: "freeintv_libretro".to_string(),
            is_available: true,
            firmware_info,
            input_profile: InputProfile::Standard,
            supported_extensions: supported_extensions
                .into_iter()
                .map(str::to_string)
                .collect(),
        }
    }

    async fn create_test_context() -> PrepareLaunchContext {
        let repository_manager = setup_test_repository_manager().await;
        let settings = Arc::new(Settings {
            temp_output_dir: PathBuf::from("/tmp/libretro-test-output"),
            libretro_system_dir: Some(PathBuf::from("/tmp/libretro-system")),
            ..Default::default()
        });

        let download_service = Arc::new(DownloadService::new(repository_manager, settings.clone()));
        create_test_context_with_download_service(settings, download_service)
    }

    fn create_test_context_with_download_service(
        settings: Arc<Settings>,
        download_service: Arc<dyn DownloadServiceOps>,
    ) -> PrepareLaunchContext {
        PrepareLaunchContext {
            deps: PrepareLaunchContextDeps {
                download_service,
                settings,
                progress_tx: None,
            },
            input: PrepareLaunchContextInput {
                extract_files: true,
                file_set_id: 123,
                initial_file: None,
                core_info: create_core_info(vec!["bin", "int"], vec![]),
                core_path: PathBuf::from("/cores/freeintv_libretro.so"),
            },
            state: PrepareLaunchContextState::default(),
        }
    }

    fn create_test_settings() -> Arc<Settings> {
        Arc::new(Settings {
            temp_output_dir: PathBuf::from("/tmp/libretro-test-output"),
            libretro_system_dir: Some(PathBuf::from("/tmp/libretro-system")),
            ..Default::default()
        })
    }

    fn set_download_results(context: &mut PrepareLaunchContext, output_file_names: Vec<&str>) {
        context.state.download_results = Some(DownloadResult {
            successful_downloads: output_file_names.len(),
            failed_downloads: 0,
            thumbnail_path_map: Default::default(),
            output_file_names: output_file_names.into_iter().map(str::to_string).collect(),
            errors: vec![],
        });
    }

    #[async_std::test]
    async fn test_download_file_set_stores_results_on_success() {
        let step = DownloadFileSetStep;
        let download_service = Arc::new(MockDownloadServiceOps::new());
        let settings = create_test_settings();
        let mut context = create_test_context_with_download_service(settings, download_service);

        let result = step.execute(&mut context).await;

        assert!(matches!(result, StepAction::Continue));
        let results = context
            .state
            .download_results
            .as_ref()
            .expect("download results should be stored");
        assert_eq!(results.successful_downloads, 1);
        assert_eq!(results.failed_downloads, 0);
    }

    #[async_std::test]
    async fn test_download_file_set_returns_download_error_when_service_fails() {
        let step = DownloadFileSetStep;
        let download_service = Arc::new(MockDownloadServiceOps::with_failure("Network error"));
        let settings = create_test_settings();
        let mut context = create_test_context_with_download_service(settings, download_service);

        let result = step.execute(&mut context).await;

        assert!(matches!(
            result,
            StepAction::Abort(LibretroPreflightError::DownloadError(message))
            if message == "Download error: Network error"
        ));
        assert!(context.state.download_results.is_none());
    }

    #[async_std::test]
    async fn test_download_file_set_returns_download_error_when_some_downloads_fail() {
        let step = DownloadFileSetStep;
        let download_service =
            Arc::new(MockDownloadServiceOps::with_successful_and_failed_downloads(1, 2));
        let settings = create_test_settings();
        let mut context = create_test_context_with_download_service(settings, download_service);

        let result = step.execute(&mut context).await;

        assert!(matches!(
            result,
            StepAction::Abort(LibretroPreflightError::DownloadError(message))
            if message == "2 files failed to download"
        ));
        assert!(context.state.download_results.is_none());
    }

    #[async_std::test]
    async fn test_select_launch_file_should_execute_only_when_file_not_selected_and_results_exist()
    {
        let step = SelectLaunchFileStep;
        let mut context = create_test_context().await;

        assert!(!step.should_execute(&context));

        set_download_results(&mut context, vec!["game.bin"]);
        assert!(step.should_execute(&context));

        context.state.selected_file = Some("game.bin".to_string());
        assert!(!step.should_execute(&context));
    }

    #[async_std::test]
    async fn test_select_launch_file_uses_initial_file_when_present_in_download_results() {
        let step = SelectLaunchFileStep;
        let mut context = create_test_context().await;
        context.input.initial_file = Some("alt.bin".to_string());
        set_download_results(&mut context, vec!["game.bin", "alt.bin"]);

        let result = step.execute(&mut context).await;

        assert!(matches!(result, StepAction::Continue));
        assert_eq!(context.state.selected_file.as_deref(), Some("alt.bin"));
    }

    #[async_std::test]
    async fn test_select_launch_file_uses_first_downloaded_file_when_initial_file_not_set() {
        let step = SelectLaunchFileStep;
        let mut context = create_test_context().await;
        set_download_results(&mut context, vec!["game.bin", "alt.bin"]);

        let result = step.execute(&mut context).await;

        assert!(matches!(result, StepAction::Continue));
        assert_eq!(context.state.selected_file.as_deref(), Some("game.bin"));
    }

    #[async_std::test]
    async fn test_select_launch_file_returns_invalid_initial_file_when_requested_file_missing() {
        let step = SelectLaunchFileStep;
        let mut context = create_test_context().await;
        context.input.initial_file = Some("missing.bin".to_string());
        set_download_results(&mut context, vec!["game.bin"]);

        let result = step.execute(&mut context).await;

        assert!(matches!(
            result,
            StepAction::Abort(LibretroPreflightError::InvalidInitialFile(file))
            if file == "missing.bin"
        ));
        assert!(context.state.selected_file.is_none());
    }

    #[async_std::test]
    async fn test_select_launch_file_returns_no_file_in_file_set_when_download_results_are_empty() {
        let step = SelectLaunchFileStep;
        let mut context = create_test_context().await;
        set_download_results(&mut context, vec![]);

        let result = step.execute(&mut context).await;

        assert!(matches!(
            result,
            StepAction::Abort(LibretroPreflightError::NoFileInFileSet)
        ));
        assert!(context.state.selected_file.is_none());
    }

    #[async_std::test]
    async fn test_validate_firmware_should_execute_only_when_required_firmware_exists() {
        let step = ValidateFirmwareStep;
        let mut context = create_test_context().await;

        assert!(!step.should_execute(&context));

        context.input.core_info.firmware_info = vec![LibretroFirmwareInfo {
            desc: "ECS".to_string(),
            path: "ecs.bin".to_string(),
            opt: false,
            available: true,
        }];
        assert!(step.should_execute(&context));
    }

    #[async_std::test]
    async fn test_validate_firmware_continues_when_all_required_firmware_is_available() {
        let step = ValidateFirmwareStep;
        let mut context = create_test_context().await;
        context.input.core_info.firmware_info = vec![
            LibretroFirmwareInfo {
                desc: "Exec".to_string(),
                path: "exec.bin".to_string(),
                opt: false,
                available: true,
            },
            LibretroFirmwareInfo {
                desc: "Optional Overlay".to_string(),
                path: "overlay.bin".to_string(),
                opt: true,
                available: false,
            },
        ];

        let result = step.execute(&mut context).await;

        assert!(matches!(result, StepAction::Continue));
    }

    #[async_std::test]
    async fn test_validate_firmware_aborts_when_required_firmware_is_missing() {
        let step = ValidateFirmwareStep;
        let mut context = create_test_context().await;
        context.input.core_info.firmware_info = vec![
            LibretroFirmwareInfo {
                desc: "Exec".to_string(),
                path: "exec.bin".to_string(),
                opt: false,
                available: false,
            },
            LibretroFirmwareInfo {
                desc: "GROM".to_string(),
                path: "grom.bin".to_string(),
                opt: false,
                available: false,
            },
        ];

        let result = step.execute(&mut context).await;

        assert!(matches!(
            result,
            StepAction::Abort(LibretroPreflightError::FirmwareNotAvailable(message))
            if message == "Exec, GROM"
        ));
    }

    #[async_std::test]
    async fn test_validate_extension_should_execute_only_when_selected_file_exists_and_extensions_configured()
     {
        let step = ValidateExtensionStep;
        let mut context = create_test_context().await;

        assert!(!step.should_execute(&context));

        context.state.selected_file = Some("game.bin".to_string());
        assert!(step.should_execute(&context));

        context.input.core_info.supported_extensions.clear();
        assert!(!step.should_execute(&context));
    }

    #[async_std::test]
    async fn test_validate_extension_continues_when_selected_file_extension_is_supported_case_insensitively()
     {
        let step = ValidateExtensionStep;
        let mut context = create_test_context().await;
        context.state.selected_file = Some("GAME.BIN".to_string());
        context.input.core_info.supported_extensions = vec!["bin".to_string()];

        let result = step.execute(&mut context).await;

        assert!(matches!(result, StepAction::Continue));
    }

    #[async_std::test]
    async fn test_validate_extension_aborts_when_selected_file_extension_is_not_supported() {
        let step = ValidateExtensionStep;
        let mut context = create_test_context().await;
        context.state.selected_file = Some("game.rom".to_string());
        context.input.core_info.supported_extensions = vec!["bin".to_string(), "int".to_string()];

        let result = step.execute(&mut context).await;

        assert!(matches!(
            result,
            StepAction::Abort(LibretroPreflightError::UnsupportedExtension(extension))
            if extension == "rom"
        ));
    }

    #[async_std::test]
    async fn test_build_launch_paths_should_execute_only_when_selected_file_exists() {
        let step = BuildLaunchPathsStep;
        let mut context = create_test_context().await;

        assert!(!step.should_execute(&context));

        context.state.selected_file = Some("game.bin".to_string());
        assert!(step.should_execute(&context));
    }

    #[async_std::test]
    async fn test_build_launch_paths_sets_launch_paths_when_system_dir_is_configured() {
        let step = BuildLaunchPathsStep;
        let mut context = create_test_context().await;
        context.state.selected_file = Some("game.bin".to_string());

        let result = step.execute(&mut context).await;

        assert!(matches!(result, StepAction::Continue));

        let launch_paths = context
            .state
            .launch_paths
            .as_ref()
            .expect("launch paths should be set");

        assert_eq!(
            launch_paths.rom_path,
            PathBuf::from("/tmp/libretro-test-output/game.bin")
        );
        assert_eq!(
            launch_paths.core_path,
            PathBuf::from("/cores/freeintv_libretro.so")
        );
        assert_eq!(
            launch_paths.system_dir,
            PathBuf::from("/tmp/libretro-system")
        );
        assert_eq!(launch_paths.temp_files, vec!["game.bin".to_string()]);
    }

    #[async_std::test]
    async fn test_build_launch_paths_aborts_when_system_dir_is_not_configured() {
        let step = BuildLaunchPathsStep;
        let mut context = create_test_context().await;
        context.state.selected_file = Some("game.bin".to_string());
        Arc::make_mut(&mut context.deps.settings).libretro_system_dir = None;

        let result = step.execute(&mut context).await;

        assert!(matches!(
            result,
            StepAction::Abort(LibretroPreflightError::SystemDirNotSet)
        ));
        assert!(context.state.launch_paths.is_none());
    }
}
