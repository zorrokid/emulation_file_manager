use std::sync::Arc;

use async_std::channel::Sender;
use core_types::{ArgumentType, events::DownloadEvent};
use database::repository_manager::RepositoryManager;
use executable_runner::ops::ExecutableRunnerOps;

use crate::{
    file_set_download::download_service_ops::DownloadServiceOps, file_system_ops::FileSystemOps,
    view_models::Settings,
};

pub struct ExternalExecutableRunnerContext {
    pub repository_manager: Arc<RepositoryManager>,
    pub executable: String,
    pub arguments: Vec<ArgumentType>,
    pub extract_files: bool,
    pub file_set_id: i64,
    pub settings: Arc<Settings>,
    pub initial_file: Option<String>,
    pub fs_ops: Arc<dyn FileSystemOps>,
    pub executable_runner_ops: Arc<dyn ExecutableRunnerOps>,
    pub file_names: Vec<String>,
    pub was_successful: bool,
    pub error_message: Vec<String>,
    pub download_service_ops: Arc<dyn DownloadServiceOps>,
    pub progress_tx: Option<Sender<DownloadEvent>>,
    /// Whether to skip automatic cleanup of temporary files.
    /// Set to true for viewers that spawn child processes (like xdg-open)
    /// where the parent returns immediately.
    pub skip_cleanup: bool,
}
