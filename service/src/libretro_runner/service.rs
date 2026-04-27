use std::{fmt::Display, path::PathBuf, sync::Arc};

use crate::{
    error::Error,
    file_set_download::service::DownloadService,
    libretro_core::service::LibretroCoreInfo,
    libretro_runner::prepare::context::{
        PrepareLaunchContext, PrepareLaunchContextDeps, PrepareLaunchContextInput,
        PrepareLaunchContextState,
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

#[derive(Debug)]
pub struct LibretroLaunchPaths {
    pub rom_path: PathBuf,
    pub core_path: PathBuf,
    pub system_dir: PathBuf,
    /// Files to remove after the session ends.
    pub temp_files: Vec<String>,
}

pub struct LibretroRunnerService {
    settings: Arc<Settings>,
    download_service: Arc<DownloadService>,
}

impl std::fmt::Debug for LibretroRunnerService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LibretroRunnerService")
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub enum LibretroPreflightError {
    UnsupportedExtension(String),
    DownloadError(String),
    NoFileInFileSet,
    SystemDirNotSet,
    FirmwareNotAvailable(String),
    InvalidInitialFile(String),
    CoreDirNotSet,
    CoreNotRecognized(String),
    InfoParseError(String),
}

impl Display for LibretroPreflightError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LibretroPreflightError::UnsupportedExtension(ext) => {
                write!(
                    f,
                    "The file extension '{ext}' is not supported by the selected core."
                )
            }
            LibretroPreflightError::DownloadError(msg) => {
                write!(f, "Failed to download ROM: {msg}")
            }
            LibretroPreflightError::NoFileInFileSet => {
                write!(f, "The selected file set does not contain any files.")
            }
            LibretroPreflightError::SystemDirNotSet => {
                write!(f, "The system directory is not configured in settings.")
            }
            LibretroPreflightError::FirmwareNotAvailable(desc) => {
                write!(f, "Required firmware not available: {desc}")
            }
            LibretroPreflightError::InvalidInitialFile(file) => {
                write!(
                    f,
                    "The specified initial file '{file}' was not found in the file set."
                )
            }
            LibretroPreflightError::CoreDirNotSet => {
                write!(
                    f,
                    "The libretro core directory is not configured in settings."
                )
            }
            LibretroPreflightError::CoreNotRecognized(core) => {
                write!(
                    f,
                    "The specified core '{core}' was not recognized in the core info."
                )
            }
            LibretroPreflightError::InfoParseError(msg) => {
                write!(f, "Failed to parse core info: {msg}")
            }
        }
    }
}

impl LibretroRunnerService {
    pub fn new(settings: Arc<Settings>, download_service: Arc<DownloadService>) -> Self {
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
