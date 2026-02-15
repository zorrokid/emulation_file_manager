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

// This is generic so that can be used in both add file set and update file set pipelines.
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

#[cfg(test)]
mod tests {
    use core_types::{FileSetFileEqualitySpecs, ImportedFile, Sha1Checksum};

    use super::*;

    struct TestContext {
        repository_manager: Arc<RepositoryManager>,
        file_set_service: Arc<dyn FileSetServiceOps>,
        files_in_file_set_already_exist: bool,
        file_set_equality_specs: FileSetEqualitySpecs,
        file_set_id: Option<i64>,
    }

    impl CheckExistingFileSetContext for TestContext {
        fn repository_manager(&self) -> Arc<RepositoryManager> {
            self.repository_manager.clone()
        }

        fn get_file_set_service(&self) -> Arc<dyn FileSetServiceOps> {
            self.file_set_service.clone()
        }

        fn files_in_file_set_already_exist(&self) -> bool {
            self.files_in_file_set_already_exist
        }

        fn file_set_equality_specs(&self) -> FileSetEqualitySpecs {
            self.file_set_equality_specs.clone()
        }

        fn set_file_set_id(&mut self, file_set_id: Option<i64>) {
            self.file_set_id = file_set_id;
        }
    }

    #[async_std::test]
    async fn test_check_existing_file_set_step() {
        // Arrange
        // create test db and add a file set to it
        let test_pool = Arc::new(database::setup_test_db().await);
        let repository_manager = Arc::new(RepositoryManager::new(test_pool));

        let system_id = repository_manager
            .get_system_repository()
            .add_system("Test System")
            .await
            .expect("Failed to add system to repository");

        let file_set_repository = repository_manager.get_file_set_repository();
        let sha1_checksum: Sha1Checksum = [0; 20]; // dummy SHA1 checksum
        let files_in_file_set = vec![ImportedFile {
            original_file_name: "test_file".to_string(),
            archive_file_name: "test_file".to_string(),
            file_size: 1234,
            sha1_checksum,
        }];

        let file_set_id = file_set_repository
            .add_file_set(
                "Test Set",
                "test.zip",
                &core_types::FileType::Rom,
                "Test",
                &files_in_file_set,
                &[system_id],
            )
            .await
            .expect("Failed to add file set to repository");

        // create a context that returns the equality specs for the file set we just added
        let mut test_context = TestContext {
            repository_manager: repository_manager.clone(),
            file_set_service: Arc::new(
                crate::file_set::mock_file_set_service::MockFileSetService::new(),
            ),
            files_in_file_set_already_exist: true,
            file_set_equality_specs: FileSetEqualitySpecs {
                file_set_name: "Test Set".to_string(),
                file_set_file_name: "test.zip".to_string(),
                file_type: core_types::FileType::Rom,
                source: "Test".to_string(),
                file_set_file_info: vec![FileSetFileEqualitySpecs {
                    file_name: "test_file".to_string(),
                    file_type: core_types::FileType::Rom,
                    sha1_checksum,
                }],
            },
            file_set_id: None,
        };

        // create the step
        let step = CheckExistingFileSetStep::<TestContext>::new();

        // Act
        // execute the step with the context
        let result = step.execute(&mut test_context).await;
        // Assert
        // check that the context's file set id was set to the id of the file set
        assert_eq!(result, StepAction::Continue);
        // assert that the file set id in the context was set to the id of the file set we added to
        // the repository
        assert_eq!(test_context.file_set_id, Some(file_set_id));
    }
}
