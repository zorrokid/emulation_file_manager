use std::{path::PathBuf, sync::Arc};

use crate::{
    error::Error,
    file_set_download::service::DownloadService,
    view_models::Settings,
};

#[derive(Debug)]
pub struct LibretroLaunchModel {
    pub file_set_id: i64,
    pub initial_file: Option<String>,
    pub core_path: PathBuf,
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
        f.debug_struct("LibretroRunnerService").finish_non_exhaustive()
    }
}

impl LibretroRunnerService {
    pub fn new(settings: Arc<Settings>, download_service: Arc<DownloadService>) -> Self {
        Self { settings, download_service }
    }

    /// Download and extract the ROM to the temp directory.
    /// Returns paths ready to pass to LibretroCore::load().
    pub async fn prepare_rom(
        &self,
        model: LibretroLaunchModel,
    ) -> Result<LibretroLaunchPaths, Error> {
        let result = self
            .download_service
            .download_file_set(model.file_set_id, true, None)
            .await?;

        // Pick the initial file if specified, otherwise take the first output file.
        let file_name = model
            .initial_file
            .or_else(|| result.output_file_names.into_iter().next())
            .ok_or_else(|| Error::InvalidInput("No ROM file found in file set".into()))?;

        Ok(LibretroLaunchPaths {
            rom_path: self.settings.temp_output_dir.join(&file_name),
            core_path: model.core_path,
            system_dir: self.settings.temp_output_dir.clone(),
            temp_files: vec![file_name],
        })
    }

    /// Resolve the full path for a core by name.
    pub fn resolve_core_path(&self, core_name: &str) -> Result<PathBuf, Error> {
        let core_dir = self.settings.libretro_core_dir.as_ref().ok_or_else(|| {
            Error::SettingsError("Libretro core directory is not set".to_string())
        })?;
        Ok(core_dir.join(core_name))
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
