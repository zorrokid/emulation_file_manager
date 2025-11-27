use std::sync::Arc;

use core_types::ArgumentType;
use database::repository_manager::RepositoryManager;
use emulator_runner::ops::EmulatorRunnerOps;

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
    pub executable_runner_ops: Arc<dyn EmulatorRunnerOps>,
    pub file_names: Vec<String>,
    pub was_successful: bool,
    pub error_message: Vec<String>,
    pub download_service_ops: Arc<dyn DownloadServiceOps>,
}
