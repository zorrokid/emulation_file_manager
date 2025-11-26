use std::sync::Arc;

use core_types::ArgumentType;

use crate::view_models::{FileSetViewModel, Settings};

pub struct ExternalExecutableRunnerContext {
    pub executable: String,
    pub arguments: Vec<ArgumentType>,
    pub extract_files: bool,
    pub file_set: FileSetViewModel,
    pub settinsgs: Arc<Settings>,
    pub initial_file: Option<String>,
}
