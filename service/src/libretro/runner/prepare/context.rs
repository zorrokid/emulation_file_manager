use std::{path::PathBuf, sync::Arc};

use core_types::events::DownloadEvent;
use flume::Sender;

use crate::{
    file_set_download::{download_service_ops::DownloadServiceOps, service::DownloadResult},
    libretro::core::service::LibretroCoreInfo,
    libretro::runner::service::LibretroLaunchPaths,
    view_models::Settings,
};

pub struct PrepareLaunchContextDeps {
    pub download_service: Arc<dyn DownloadServiceOps>,
    pub settings: Arc<Settings>,
    pub progress_tx: Option<Sender<DownloadEvent>>,
}

pub struct PrepareLaunchContextInput {
    pub extract_files: bool,
    pub file_set_id: i64,
    pub initial_file: Option<String>,
    pub core_info: LibretroCoreInfo,
    pub core_path: PathBuf,
}

#[derive(Default)]
pub struct PrepareLaunchContextState {
    pub download_results: Option<DownloadResult>,
    pub selected_file: Option<String>,
    pub launch_paths: Option<LibretroLaunchPaths>,
}

pub struct PrepareLaunchContext {
    pub deps: PrepareLaunchContextDeps,
    pub input: PrepareLaunchContextInput,
    pub state: PrepareLaunchContextState,
}
