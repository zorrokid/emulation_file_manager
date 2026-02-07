use std::sync::Arc;

use core_types::Sha1Checksum;
use database::repository_manager::RepositoryManager;

use crate::{
    file_set_service::{FileSetService, FileSetServiceOps},
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

pub trait CheckExistingFileSetContext {
    fn has_existing_files(&self) -> bool {
        !self.get_existing_file_sha1_checksums().is_empty()
    }
    fn all_files_in_file_set_exist(&self) -> bool;
    fn get_existing_file_sha1_checksums(&self) -> Vec<Sha1Checksum>;
    fn repository_manager(&self) -> Arc<RepositoryManager>;
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
        context.has_existing_files() && context.all_files_in_file_set_exist()
    }

    async fn execute(&self, context: &mut T) -> StepAction {
        println!("Checking for existing file set in the database...");
        let repository_manager = context.repository_manager();

        let file_set_service = FileSetService::new(repository_manager.clone());

        let existing_file_set = file_set_service
            .find_file_set_by_files(context.get_existing_file_sha1_checksums())
            .await;

        StepAction::Continue
    }
}
