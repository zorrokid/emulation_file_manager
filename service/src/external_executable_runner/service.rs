use std::sync::Arc;

use async_std::channel::Sender;
use core_types::{ArgumentType, events::DownloadEvent};
use database::repository_manager::RepositoryManager;
use emulator_runner::ops::{DefaultEmulatorRunnerOps, EmulatorRunnerOps};

use crate::{
    error::Error,
    external_executable_runner::context::ExternalExecutableRunnerContext,
    file_set_download::{download_service_ops::DownloadServiceOps, service::DownloadService},
    file_system_ops::{FileSystemOps, StdFileSystemOps},
    pipeline::generic_pipeline::Pipeline,
    view_models::Settings,
};

pub struct ExternalExecutableRunnerService {
    repository_manager: Arc<RepositoryManager>,
    settings: Arc<Settings>,
    fs_ops: Arc<dyn FileSystemOps>,
    executable_runner_ops: Arc<dyn EmulatorRunnerOps>,
    download_service_ops: Arc<dyn DownloadServiceOps>,
}

impl std::fmt::Debug for ExternalExecutableRunnerService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExternalExecutableRunnerService")
            .finish_non_exhaustive()
    }
}

pub struct ExecutableRunnerModel {
    pub executable: String,
    pub arguments: Vec<ArgumentType>,
    pub extract_files: bool,
    pub file_set_id: i64,
    pub initial_file: Option<String>,
    /// Whether to skip automatic cleanup of temporary files.
    /// Set to true for viewers that spawn child processes (like xdg-open)
    /// where the parent returns immediately.
    pub skip_cleanup: bool,
}

impl ExternalExecutableRunnerService {
    pub fn new(settings: Arc<Settings>, repository_manager: Arc<RepositoryManager>) -> Self {
        let download_service = Arc::new(DownloadService::new(
            repository_manager.clone(),
            settings.clone(),
        ));
        Self::new_with_ops(
            repository_manager,
            settings,
            Arc::new(StdFileSystemOps),
            Arc::new(DefaultEmulatorRunnerOps),
            download_service,
        )
    }

    pub fn new_with_ops(
        repository_manager: Arc<RepositoryManager>,
        settings: Arc<Settings>,
        fs_ops: Arc<dyn FileSystemOps>,
        executable_runner_ops: Arc<dyn EmulatorRunnerOps>,
        download_service_ops: Arc<dyn DownloadServiceOps>,
    ) -> Self {
        Self {
            repository_manager,
            settings,
            fs_ops,
            executable_runner_ops,
            download_service_ops,
        }
    }

    pub async fn run_executable(
        &self,
        model: ExecutableRunnerModel,
        progress_tx: Option<Sender<DownloadEvent>>,
    ) -> Result<(), Error> {
        let mut context = ExternalExecutableRunnerContext {
            executable: model.executable,
            arguments: model.arguments,
            extract_files: model.extract_files,
            file_set_id: model.file_set_id,
            settings: self.settings.clone(),
            initial_file: model.initial_file,
            fs_ops: self.fs_ops.clone(),
            repository_manager: self.repository_manager.clone(),
            error_message: Vec::new(),
            file_names: Vec::new(),
            executable_runner_ops: self.executable_runner_ops.clone(),
            was_successful: false,
            download_service_ops: self.download_service_ops.clone(),
            progress_tx,
            skip_cleanup: model.skip_cleanup,
        };

        let pipeline = Pipeline::<ExternalExecutableRunnerContext>::new();
        pipeline.execute(&mut context).await?;
        Ok(())
    }
}
