use std::sync::Arc;

use core_types::Sha1Checksum;
use database::repository_manager::RepositoryManager;

use crate::{
    error::Error,
    file_set::FileSetServiceOps,
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

pub trait CheckExistingFileSetContext {
    fn has_existing_files(&self) -> bool {
        !self.get_existing_file_sha1_checksums().is_empty()
    }
    fn all_files_in_file_set_exist(&self) -> bool;
    fn get_existing_file_sha1_checksums(&self) -> Vec<Sha1Checksum>;
    fn repository_manager(&self) -> Arc<RepositoryManager>;
    fn set_file_set_id(&mut self, file_set_id: Option<i64>);
    fn get_file_set_service(&self) -> Arc<dyn FileSetServiceOps>;
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

        let existing_file_set = context
            .get_file_set_service()
            .find_file_set_by_files(context.get_existing_file_sha1_checksums())
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
    use super::*;
    use crate::file_set::{mock_file_set_service::MockFileSetService, FileSetServiceError};
    use crate::pipeline::pipeline_step::PipelineStep;
    use database::setup_test_db;
    use std::sync::Arc;

    /// Test context implementing CheckExistingFileSetContext
    struct TestContext {
        has_existing_files: bool,
        all_files_in_file_set_exist: bool,
        existing_checksums: Vec<Sha1Checksum>,
        file_set_id: Option<i64>,
        file_set_service: Arc<dyn FileSetServiceOps>,
        repository_manager: Arc<RepositoryManager>,
    }

    impl TestContext {
        async fn new(file_set_service: Arc<dyn FileSetServiceOps>) -> Self {
            // Create a test repository manager with in-memory database
            let pool = Arc::new(setup_test_db().await);
            let repo_manager = Arc::new(RepositoryManager::new(pool));

            Self {
                has_existing_files: true,
                all_files_in_file_set_exist: true,
                existing_checksums: vec![],
                file_set_id: None,
                file_set_service,
                repository_manager: repo_manager,
            }
        }

        fn with_checksums(mut self, checksums: Vec<Sha1Checksum>) -> Self {
            self.existing_checksums = checksums;
            self
        }

        fn with_has_existing_files(mut self, has_existing: bool) -> Self {
            self.has_existing_files = has_existing;
            self
        }

        fn with_all_files_exist(mut self, all_exist: bool) -> Self {
            self.all_files_in_file_set_exist = all_exist;
            self
        }
    }

    impl CheckExistingFileSetContext for TestContext {
        fn has_existing_files(&self) -> bool {
            self.has_existing_files
        }

        fn all_files_in_file_set_exist(&self) -> bool {
            self.all_files_in_file_set_exist
        }

        fn get_existing_file_sha1_checksums(&self) -> Vec<Sha1Checksum> {
            self.existing_checksums.clone()
        }

        fn repository_manager(&self) -> Arc<RepositoryManager> {
            self.repository_manager.clone()
        }

        fn set_file_set_id(&mut self, file_set_id: Option<i64>) {
            self.file_set_id = file_set_id;
        }

        fn get_file_set_service(&self) -> Arc<dyn FileSetServiceOps> {
            self.file_set_service.clone()
        }
    }

    fn create_test_checksums() -> Vec<Sha1Checksum> {
        vec![[1; 20], [2; 20], [3; 20]]
    }

    #[test]
    fn test_step_name() {
        let step = CheckExistingFileSetStep::<TestContext>::new();
        assert_eq!(step.name(), "check_existing_file_set");
    }

    #[async_std::test]
    async fn test_should_execute_when_has_files_and_all_exist() {
        let mock = Arc::new(MockFileSetService::new());
        let context = TestContext::new(mock)
            .await
            .with_has_existing_files(true)
            .with_all_files_exist(true);

        let step = CheckExistingFileSetStep::new();
        assert!(step.should_execute(&context));
    }

    #[async_std::test]
    async fn test_should_not_execute_when_no_existing_files() {
        let mock = Arc::new(MockFileSetService::new());
        let context = TestContext::new(mock)
            .await
            .with_has_existing_files(false)
            .with_all_files_exist(true);

        let step = CheckExistingFileSetStep::new();
        assert!(!step.should_execute(&context));
    }

    #[async_std::test]
    async fn test_should_not_execute_when_not_all_files_exist() {
        let mock = Arc::new(MockFileSetService::new());
        let context = TestContext::new(mock)
            .await
            .with_has_existing_files(true)
            .with_all_files_exist(false);

        let step = CheckExistingFileSetStep::new();
        assert!(!step.should_execute(&context));
    }

    #[async_std::test]
    async fn test_should_not_execute_when_no_files_and_not_all_exist() {
        let mock = Arc::new(MockFileSetService::new());
        let context = TestContext::new(mock)
            .await
            .with_has_existing_files(false)
            .with_all_files_exist(false);

        let step = CheckExistingFileSetStep::new();
        assert!(!step.should_execute(&context));
    }

    #[async_std::test]
    async fn test_execute_finds_existing_file_set() {
        let mock = Arc::new(MockFileSetService::new());
        let checksums = create_test_checksums();

        // Configure mock to return a file set ID
        mock.add_file_set_lookup(checksums.clone(), 42);

        let mut context = TestContext::new(mock).await.with_checksums(checksums);

        let step = CheckExistingFileSetStep::new();
        let result = step.execute(&mut context).await;

        // Should continue and set file_set_id
        assert!(matches!(result, StepAction::Continue));
        assert_eq!(context.file_set_id, Some(42));
    }

    #[async_std::test]
    async fn test_execute_no_existing_file_set_found() {
        let mock = Arc::new(MockFileSetService::new());
        let checksums = create_test_checksums();

        // Don't configure any lookup - mock will return None

        let mut context = TestContext::new(mock).await.with_checksums(checksums);

        let step = CheckExistingFileSetStep::new();
        let result = step.execute(&mut context).await;

        // Should continue with None
        assert!(matches!(result, StepAction::Continue));
        assert_eq!(context.file_set_id, None);
    }

    #[async_std::test]
    async fn test_execute_handles_database_error() {
        let mock = Arc::new(MockFileSetService::new());
        let checksums = create_test_checksums();

        // Configure mock to fail
        mock.fail_find_for(checksums.clone());

        let mut context = TestContext::new(mock).await.with_checksums(checksums);

        let step = CheckExistingFileSetStep::new();
        let result = step.execute(&mut context).await;

        // Should abort with error
        match result {
            StepAction::Abort(Error::DbError(msg)) => {
                assert!(msg.contains("Error checking for existing file set"));
            }
            _ => panic!("Expected Abort with DbError"),
        }
    }

    #[async_std::test]
    async fn test_execute_with_multiple_file_sets() {
        let mock = Arc::new(MockFileSetService::new());
        
        let checksums1 = vec![[1; 20]];
        let checksums2 = vec![[2; 20]];
        
        // Configure different file sets for different checksums
        mock.add_file_set_lookup(checksums1.clone(), 100);
        mock.add_file_set_lookup(checksums2.clone(), 200);

        // Test first set
        let mut context1 = TestContext::new(mock.clone()).await.with_checksums(checksums1);
        let step = CheckExistingFileSetStep::new();
        step.execute(&mut context1).await;
        assert_eq!(context1.file_set_id, Some(100));

        // Test second set
        let mut context2 = TestContext::new(mock).await.with_checksums(checksums2);
        step.execute(&mut context2).await;
        assert_eq!(context2.file_set_id, Some(200));
    }

    #[async_std::test]
    async fn test_execute_with_empty_checksums() {
        let mock = Arc::new(MockFileSetService::new());

        let mut context = TestContext::new(mock).await.with_checksums(vec![]);

        let step = CheckExistingFileSetStep::new();
        let result = step.execute(&mut context).await;

        // Should handle empty checksums gracefully
        assert!(matches!(result, StepAction::Continue));
    }
}
