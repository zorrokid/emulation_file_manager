use std::{path::PathBuf, sync::Arc};

use crate::{
    file_set_download::download_service_ops::DownloadServiceOps,
    libretro::{
        core::service::LibretroCoreInfo,
        error::LibretroPreflightError,
        runner::prepare::context::{
            PrepareLaunchContext, PrepareLaunchContextDeps, PrepareLaunchContextInput,
            PrepareLaunchContextState,
        },
    },
    pipeline::generic_pipeline::Pipeline,
    view_models::Settings,
};

#[derive(Debug)]
pub struct LibretroLaunchModel {
    pub file_set_id: i64,
    pub initial_file: Option<String>,
    pub core_path: PathBuf,
    pub core_info: LibretroCoreInfo,
}

#[derive(Debug, Eq, PartialEq)]
pub struct LibretroLaunchPaths {
    pub rom_path: PathBuf,
    pub core_path: PathBuf,
    pub system_dir: PathBuf,
    /// Files to remove after the session ends.
    pub temp_files: Vec<String>,
}

pub struct LibretroRunnerService {
    settings: Arc<Settings>,
    download_service: Arc<dyn DownloadServiceOps>,
}

impl std::fmt::Debug for LibretroRunnerService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LibretroRunnerService")
            .finish_non_exhaustive()
    }
}

impl LibretroRunnerService {
    pub fn new(settings: Arc<Settings>, download_service: Arc<dyn DownloadServiceOps>) -> Self {
        Self {
            settings,
            download_service,
        }
    }

    /// Download and extract the ROM to the temp directory.
    /// Returns paths ready to pass to LibretroCore::load().
    pub async fn prepare_rom(
        &self,
        model: LibretroLaunchModel,
    ) -> Result<LibretroLaunchPaths, LibretroPreflightError> {
        let mut context = PrepareLaunchContext {
            deps: PrepareLaunchContextDeps {
                download_service: self.download_service.clone(),
                settings: self.settings.clone(),
                progress_tx: None,
            },
            input: PrepareLaunchContextInput {
                extract_files: true,
                file_set_id: model.file_set_id,
                initial_file: model.initial_file.clone(),
                core_info: model.core_info.clone(),
                core_path: model.core_path.clone(),
            },
            state: PrepareLaunchContextState::default(),
        };

        let pipeline = Pipeline::<PrepareLaunchContext, LibretroPreflightError>::new();
        match pipeline.execute(&mut context).await {
            Ok(_) => {
                if let Some(paths) = context.state.launch_paths {
                    Ok(paths)
                } else {
                    Err(LibretroPreflightError::DownloadError(
                        "Pipeline completed but launch paths not set".to_string(),
                    ))
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Resolve the full path for a core by name.
    /// `core_name` must be provided WITHOUT extension (e.g. `fceumm_libretro`).
    /// The `.so` extension is appended automatically.
    pub fn resolve_core_path(&self, core_name: &str) -> Result<PathBuf, LibretroPreflightError> {
        let core_dir = self
            .settings
            .libretro_core_dir
            .as_ref()
            .ok_or(LibretroPreflightError::CoreDirNotSet)?;
        Ok(core_dir.join(format!("{core_name}.so")))
    }

    /// Remove a list of temp files by name from the temp output directory.
    /// Called by the GUI when it receives SessionEnded from LibretroWindowModel.
    pub fn cleanup_files(&self, files: &[String]) {
        for file in files {
            let p = self.settings.temp_output_dir.join(file);
            if p.exists() {
                let _ = std::fs::remove_file(&p);
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use std::sync::Mutex;

    use libretro_runner::supported_cores::InputProfile;

    use crate::{
        error::Error,
        file_set_download::download_service_ops::{
            ConfiguredOutcome, MockDownloadServiceOps, MockState,
        },
        libretro::core::service::LibretroFirmwareInfo,
    };

    use super::*;

    fn get_mock_state(outcome: ConfiguredOutcome) -> Arc<Mutex<MockState>> {
        Arc::new(Mutex::new(MockState {
            outcome,
            ..Default::default()
        }))
    }

    fn get_libretro_runner_service(
        settings: Settings,
        state: &Arc<Mutex<MockState>>,
    ) -> LibretroRunnerService {
        LibretroRunnerService::new(
            Arc::new(settings),
            Arc::new(MockDownloadServiceOps::with_state(Arc::clone(state))),
        )
    }

    fn create_launch_model() -> LibretroLaunchModel {
        LibretroLaunchModel {
            file_set_id: 1,
            initial_file: None,
            core_path: PathBuf::from("/opt/libretro/cores"),
            core_info: LibretroCoreInfo {
                core_name: "test".into(),
                is_available: false, // TODO: do we check if core is available or not?
                firmware_info: vec![],
                input_profile: InputProfile::Standard,
                supported_extensions: vec![],
            },
        }
    }

    #[async_std::test]
    async fn test_prepare_rom_when_download_fails_with_error() {
        let settings = Settings::default();
        let mock_state = get_mock_state(ConfiguredOutcome {
            result: Err(Error::DownloadError("Download error".into())),
            ..Default::default()
        });
        let libretro_runner_service = get_libretro_runner_service(settings, &mock_state);

        let launch_model = create_launch_model();

        let result = libretro_runner_service.prepare_rom(launch_model).await;

        let state_guard = mock_state.lock().expect("lock poisoned");
        assert_eq!(state_guard.download_calls.len(), 1);
        let call = state_guard.download_calls[0].clone();
        assert_eq!(call.file_set_id, 1);
        assert!(call.extract_files);
        assert!(!call.had_progress_tx);

        assert!(result.is_err());
        let error = result.err();
        assert!(matches!(
            error,
            Some(LibretroPreflightError::DownloadError(_))
        ));
    }

    #[async_std::test]
    async fn test_prepare_rom_when_no_files_in_file_set() {
        let settings = Settings::default();
        let mock_state = get_mock_state(ConfiguredOutcome {
            result: Ok(crate::file_set_download::service::DownloadResult {
                successful_downloads: 1,
                ..Default::default()
            }),
            ..Default::default()
        });
        let libretro_runner_service = get_libretro_runner_service(settings, &mock_state);

        let launch_model = create_launch_model();

        let result = libretro_runner_service.prepare_rom(launch_model).await;

        let state_guard = mock_state.lock().expect("lock poisoned");
        assert_eq!(state_guard.download_calls.len(), 1);
        let call = state_guard.download_calls[0].clone();
        assert_eq!(call.file_set_id, 1);
        assert!(call.extract_files);
        assert!(!call.had_progress_tx);

        assert!(result.is_err());
        let error = result.err();
        assert!(matches!(
            error,
            Some(LibretroPreflightError::NoFileInFileSet)
        ));
    }

    #[async_std::test]
    async fn test_prepare_rom_when_invalid_initial_file() {
        let settings = Settings::default();
        let initial_file = "initial_file".to_string();

        let mock_state = get_mock_state(ConfiguredOutcome {
            result: Ok(crate::file_set_download::service::DownloadResult {
                successful_downloads: 1,
                output_file_names: vec!["not_initial_file".to_string()],
                ..Default::default()
            }),
            ..Default::default()
        });

        let libretro_runner_service = get_libretro_runner_service(settings, &mock_state);

        let mut launch_model = create_launch_model();
        launch_model.initial_file = Some(initial_file);

        let result = libretro_runner_service.prepare_rom(launch_model).await;

        let state_guard = mock_state.lock().expect("lock poisoned");
        assert_eq!(state_guard.download_calls.len(), 1);
        let call = state_guard.download_calls[0].clone();
        assert_eq!(call.file_set_id, 1);
        assert!(call.extract_files);
        assert!(!call.had_progress_tx);

        assert!(result.is_err());
        let error = result.err();
        assert!(matches!(
            error,
            Some(LibretroPreflightError::InvalidInitialFile(_))
        ));
    }

    #[async_std::test]
    async fn test_prepare_rom_when_firmware_not_available() {
        let settings = Settings::default();
        let mock_state = get_mock_state(ConfiguredOutcome {
            result: Ok(crate::file_set_download::service::DownloadResult {
                successful_downloads: 1,
                output_file_names: vec!["file".to_string()],
                ..Default::default()
            }),
            ..Default::default()
        });
        let libretro_runner_service = get_libretro_runner_service(settings, &mock_state);

        let mut launch_model = create_launch_model();
        launch_model.core_info.firmware_info = vec![LibretroFirmwareInfo {
            desc: "".to_string(),
            path: "bios.bin".to_string(),
            opt: false,
            available: false,
        }];

        let result = libretro_runner_service.prepare_rom(launch_model).await;

        let state_guard = mock_state.lock().expect("lock poisoned");
        assert_eq!(state_guard.download_calls.len(), 1);
        let call = state_guard.download_calls[0].clone();
        assert_eq!(call.file_set_id, 1);
        assert!(call.extract_files);
        assert!(!call.had_progress_tx);

        assert!(result.is_err());
        let error = result.err();
        assert!(matches!(
            error,
            Some(LibretroPreflightError::FirmwareNotAvailable(_))
        ));
    }

    #[async_std::test]
    async fn test_prepare_rom_when_unsupported_extension() {
        let settings = Settings::default();
        let mock_state = get_mock_state(ConfiguredOutcome {
            result: Ok(crate::file_set_download::service::DownloadResult {
                successful_downloads: 1,
                output_file_names: vec!["file.img".to_string()],
                ..Default::default()
            }),
            ..Default::default()
        });
        let libretro_runner_service = get_libretro_runner_service(settings, &mock_state);

        let mut launch_model = create_launch_model();
        launch_model.core_info.supported_extensions = vec!["dsk".to_string()];

        let result = libretro_runner_service.prepare_rom(launch_model).await;

        let state_guard = mock_state.lock().expect("lock poisoned");
        assert_eq!(state_guard.download_calls.len(), 1);
        let call = state_guard.download_calls[0].clone();
        assert_eq!(call.file_set_id, 1);
        assert!(call.extract_files);
        assert!(!call.had_progress_tx);

        assert!(result.is_err());
        let error = result.err();
        assert!(matches!(
            error,
            Some(LibretroPreflightError::UnsupportedExtension(_))
        ));
    }

    #[async_std::test]
    async fn test_prepare_rom_when_system_dir_not_set() {
        let settings = Settings::default();
        let mock_state = get_mock_state(ConfiguredOutcome {
            result: Ok(crate::file_set_download::service::DownloadResult {
                successful_downloads: 1,
                output_file_names: vec!["file.img".to_string()],
                ..Default::default()
            }),
            ..Default::default()
        });
        let libretro_runner_service = get_libretro_runner_service(settings, &mock_state);

        let launch_model = create_launch_model();

        let result = libretro_runner_service.prepare_rom(launch_model).await;

        let state_guard = mock_state.lock().expect("lock poisoned");
        assert_eq!(state_guard.download_calls.len(), 1);
        let call = state_guard.download_calls[0].clone();
        assert_eq!(call.file_set_id, 1);
        assert!(call.extract_files);
        assert!(!call.had_progress_tx);

        assert!(result.is_err());
        let error = result.err();
        assert!(matches!(
            error,
            Some(LibretroPreflightError::SystemDirNotSet)
        ));
    }

    #[async_std::test]
    async fn test_prepare_rom_happy_path_returns_launch_paths() {
        let settings = Settings {
            libretro_system_dir: Some(PathBuf::from("/opt/libretro/system_dir")),
            temp_output_dir: PathBuf::from("/tmp/"),
            ..Default::default()
        };

        let mock_state = get_mock_state(ConfiguredOutcome {
            result: Ok(crate::file_set_download::service::DownloadResult {
                successful_downloads: 1,
                output_file_names: vec!["file.img".to_string()],
                ..Default::default()
            }),
            ..Default::default()
        });

        let libretro_runner_service = get_libretro_runner_service(settings, &mock_state);

        let launch_model = create_launch_model();

        let result = libretro_runner_service.prepare_rom(launch_model).await;

        let state_guard = mock_state.lock().expect("lock poisoned");
        assert_eq!(state_guard.download_calls.len(), 1);
        let call = state_guard.download_calls[0].clone();
        assert_eq!(call.file_set_id, 1);
        assert!(call.extract_files);
        assert!(!call.had_progress_tx);

        assert!(result.is_ok());
        let result = result.ok();
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(
            result,
            LibretroLaunchPaths {
                rom_path: PathBuf::from("/tmp/file.img"),
                core_path: PathBuf::from("/opt/libretro/cores"),
                system_dir: PathBuf::from("/opt/libretro/system_dir"),
                temp_files: vec!["file.img".to_string()],
            }
        );
    }
}
