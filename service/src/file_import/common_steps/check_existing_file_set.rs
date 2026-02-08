use std::sync::Arc;

use core_types::FileSetEqualitySpecs;
use database::repository_manager::RepositoryManager;

use crate::{
    error::Error,
    file_set::FileSetServiceOps,
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

pub trait CheckExistingFileSetContext {
    fn repository_manager(&self) -> Arc<RepositoryManager>;
    fn get_file_set_service(&self) -> Arc<dyn FileSetServiceOps>;

    fn files_in_file_set_already_exist(&self) -> bool;
    fn file_set_equality_specs(&self) -> FileSetEqualitySpecs;

    /// if maching existing file set was found set its id here
    fn set_file_set_id(&mut self, file_set_id: Option<i64>);
}

pub struct CheckExistingFileSetStep<T: CheckExistingFileSetContext> {
    // `PhantomData<T>` is required to satisfy Rust's type system for generic structs that don't store their generic type directly.
    _phantom: std::marker::PhantomData<T>,
}

impl<T: CheckExistingFileSetContext> Default for CheckExistingFileSetStep<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: CheckExistingFileSetContext> CheckExistingFileSetStep<T> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<T: CheckExistingFileSetContext + Send + Sync> PipelineStep<T> for CheckExistingFileSetStep<T> {
    fn name(&self) -> &'static str {
        "check_existing_file_set"
    }

    fn should_execute(&self, context: &T) -> bool {
        context.files_in_file_set_already_exist()
    }

    async fn execute(&self, context: &mut T) -> StepAction {
        println!("Checking for existing file set in the database...");

        let existing_file_set = context
            .repository_manager()
            .get_file_set_repository()
            .find_file_set(&context.file_set_equality_specs())
            .await;

        match existing_file_set {
            Ok(file_set_id) => {
                tracing::info!(
                    file_set_id = file_set_id,
                    "Got result for existing file set in repository"
                );
                context.set_file_set_id(file_set_id);
                StepAction::Continue
            }
            Err(e) => {
                tracing::error!("Error checking for existing file set: {e}");
                StepAction::Abort(Error::DbError(format!(
                    "Error checking for existing file set: {}",
                    e
                )))
            }
        }
    }
}
