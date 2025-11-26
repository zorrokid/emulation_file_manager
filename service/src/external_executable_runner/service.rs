use std::sync::Arc;

use core_types::ArgumentType;

use crate::{
    error::Error,
    external_executable_runner::context::ExternalExecutableRunnerContext,
    file_system_ops::{FileSystemOps, StdFileSystemOps},
    pipeline::generic_pipeline::Pipeline,
    view_models::{FileSetViewModel, Settings},
};

pub struct ExternalExecutableRunnerService {
    settings: Arc<Settings>,
    fs_ops: Arc<dyn FileSystemOps>,
}

pub struct ExecutableRunnerModel {
    pub executable: String,
    pub arguments: Vec<ArgumentType>,
    pub extract_files: bool,
    pub file_set: FileSetViewModel,
    pub initial_file: Option<String>,
}

impl ExternalExecutableRunnerService {
    pub fn new(settings: Arc<Settings>) -> Self {
        Self::new_with_fs_ops(settings, Arc::new(StdFileSystemOps))
    }

    pub fn new_with_fs_ops(settings: Arc<Settings>, fs_ops: Arc<dyn FileSystemOps>) -> Self {
        Self { settings, fs_ops }
    }

    pub async fn run_executable(&self, model: ExecutableRunnerModel) -> Result<(), Error> {
        let mut context = ExternalExecutableRunnerContext {
            executable: model.executable,
            arguments: model.arguments,
            extract_files: model.extract_files,
            file_set: model.file_set,
            settinsgs: self.settings.clone(),
            initial_file: model.initial_file,
        };

        let pipeline = Pipeline::<ExternalExecutableRunnerContext>::new();
        pipeline.execute(&mut context).await?;
        Ok(())
    }
}
