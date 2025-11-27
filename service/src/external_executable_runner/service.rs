use std::sync::Arc;

use core_types::ArgumentType;
use database::repository_manager::RepositoryManager;
use emulator_runner::ops::{DefaultEmulatorRunnerOps, EmulatorRunnerOps};

use crate::{
    error::Error,
    external_executable_runner::context::ExternalExecutableRunnerContext,
    file_system_ops::{FileSystemOps, StdFileSystemOps},
    pipeline::generic_pipeline::Pipeline,
    view_models::Settings,
};

pub struct ExternalExecutableRunnerService {
    settings: Arc<Settings>,
    fs_ops: Arc<dyn FileSystemOps>,
    executable_runner_ops: Arc<dyn EmulatorRunnerOps>,
}

pub struct ExecutableRunnerModel {
    pub repository_manager: Arc<RepositoryManager>,
    pub executable: String,
    pub arguments: Vec<ArgumentType>,
    pub extract_files: bool,
    pub file_set_id: i64,
    pub initial_file: Option<String>,
}

impl ExternalExecutableRunnerService {
    pub fn new(settings: Arc<Settings>) -> Self {
        Self::new_with_ops(
            settings,
            Arc::new(StdFileSystemOps),
            Arc::new(DefaultEmulatorRunnerOps),
        )
    }

    pub fn new_with_ops(
        settings: Arc<Settings>,
        fs_ops: Arc<dyn FileSystemOps>,
        executable_runner_ops: Arc<dyn EmulatorRunnerOps>,
    ) -> Self {
        Self {
            settings,
            fs_ops,
            executable_runner_ops,
        }
    }

    pub async fn run_executable(&self, model: ExecutableRunnerModel) -> Result<(), Error> {
        let mut context = ExternalExecutableRunnerContext {
            executable: model.executable,
            arguments: model.arguments,
            extract_files: model.extract_files,
            file_set_id: model.file_set_id,
            settings: self.settings.clone(),
            initial_file: model.initial_file,
            fs_ops: self.fs_ops.clone(),
            repository_manager: model.repository_manager.clone(),
            error_message: Vec::new(),
            file_names: Vec::new(),
            executable_runner_ops: Arc::new(emulator_runner::ops::DefaultEmulatorRunnerOps {}),
            was_successful: false,
        };

        let pipeline = Pipeline::<ExternalExecutableRunnerContext>::new();
        pipeline.execute(&mut context).await?;
        Ok(())
    }
}
