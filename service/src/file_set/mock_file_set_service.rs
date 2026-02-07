use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use core_types::Sha1Checksum;

use crate::file_set::{
    CreateFileSetParams, CreateFileSetResult, FileSetServiceError, FileSetServiceOps,
};

/// Mock implementation of FileSetServiceOps for testing
///
/// This mock allows you to:
/// - Simulate file set creation with configurable IDs
/// - Test failure scenarios
/// - Pre-configure file set lookups by checksums
/// - Verify what operations were performed
#[derive(Clone, Default)]
pub struct MockFileSetService {
    /// Counter for generating file set IDs
    next_file_set_id: Arc<Mutex<i64>>,

    /// Counter for generating release IDs
    next_release_id: Arc<Mutex<i64>>,

    /// Track created file sets (file_set_id -> params used)
    created_file_sets: Arc<Mutex<HashMap<i64, CreateFileSetParams>>>,

    /// Pre-configured lookups: checksums -> file_set_id
    /// BTreeSet ensures order independence and prevents duplicates
    checksum_to_file_set: Arc<Mutex<HashMap<BTreeSet<Sha1Checksum>, i64>>>,

    /// Keys that should fail on create_file_set
    fail_create_for: Arc<Mutex<Vec<String>>>,

    /// Keys that should fail on find_file_set_by_files
    fail_find_for: Arc<Mutex<Vec<BTreeSet<Sha1Checksum>>>>,
}

impl MockFileSetService {
    /// Create a new mock file set service
    pub fn new() -> Self {
        Self {
            next_file_set_id: Arc::new(Mutex::new(1)),
            next_release_id: Arc::new(Mutex::new(1)),
            created_file_sets: Arc::new(Mutex::new(HashMap::new())),
            checksum_to_file_set: Arc::new(Mutex::new(HashMap::new())),
            fail_create_for: Arc::new(Mutex::new(Vec::new())),
            fail_find_for: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Add a pre-configured file set lookup result
    pub fn add_file_set_lookup(&self, checksums: Vec<Sha1Checksum>, file_set_id: i64) {
        let checksum_set: BTreeSet<Sha1Checksum> = checksums.into_iter().collect();
        self.checksum_to_file_set
            .lock()
            .unwrap()
            .insert(checksum_set, file_set_id);
    }

    /// Make create_file_set fail for a specific file set name
    pub fn fail_create_for(&self, file_set_name: impl Into<String>) {
        self.fail_create_for
            .lock()
            .unwrap()
            .push(file_set_name.into());
    }

    /// Make find_file_set_by_files fail for specific checksums
    pub fn fail_find_for(&self, checksums: Vec<Sha1Checksum>) {
        let checksum_set: BTreeSet<Sha1Checksum> = checksums.into_iter().collect();
        self.fail_find_for.lock().unwrap().push(checksum_set);
    }

    /// Get all created file sets
    pub fn get_created_file_sets(&self) -> Vec<(i64, CreateFileSetParams)> {
        self.created_file_sets
            .lock()
            .unwrap()
            .iter()
            .map(|(id, params)| {
                (
                    *id,
                    CreateFileSetParams {
                        file_set_name: params.file_set_name.clone(),
                        file_set_file_name: params.file_set_file_name.clone(),
                        source: params.source.clone(),
                        file_type: params.file_type,
                        system_ids: params.system_ids.clone(),
                        files_in_file_set: params.files_in_file_set.clone(),
                        create_release: params.create_release.clone(),
                        dat_file_id: params.dat_file_id,
                    },
                )
            })
            .collect()
    }

    /// Get the number of created file sets
    pub fn created_count(&self) -> usize {
        self.created_file_sets.lock().unwrap().len()
    }

    /// Check if a file set was created with a specific name
    pub fn was_created(&self, file_set_name: &str) -> bool {
        self.created_file_sets
            .lock()
            .unwrap()
            .values()
            .any(|params| params.file_set_name == file_set_name)
    }

    /// Set the next file set ID to be returned
    pub fn set_next_file_set_id(&self, id: i64) {
        *self.next_file_set_id.lock().unwrap() = id;
    }

    /// Set the next release ID to be returned
    pub fn set_next_release_id(&self, id: i64) {
        *self.next_release_id.lock().unwrap() = id;
    }

    /// Clear all state (useful between tests)
    pub fn clear(&self) {
        *self.next_file_set_id.lock().unwrap() = 1;
        *self.next_release_id.lock().unwrap() = 1;
        self.created_file_sets.lock().unwrap().clear();
        self.checksum_to_file_set.lock().unwrap().clear();
        self.fail_create_for.lock().unwrap().clear();
        self.fail_find_for.lock().unwrap().clear();
    }
}

#[async_trait]
impl FileSetServiceOps for MockFileSetService {
    async fn create_file_set(
        &self,
        file_set_params: CreateFileSetParams,
    ) -> Result<CreateFileSetResult, FileSetServiceError> {
        // Check if we should fail this operation
        if self
            .fail_create_for
            .lock()
            .unwrap()
            .contains(&file_set_params.file_set_name)
        {
            return Err(FileSetServiceError::DatabaseError(format!(
                "Mock create failure for file set: {}",
                file_set_params.file_set_name
            )));
        }

        // Generate IDs
        let file_set_id = {
            let mut next_id = self.next_file_set_id.lock().unwrap();
            let id = *next_id;
            *next_id += 1;
            id
        };

        let release_id = if file_set_params.create_release.is_some() {
            let mut next_id = self.next_release_id.lock().unwrap();
            let id = *next_id;
            *next_id += 1;
            Some(id)
        } else {
            None
        };

        // Store the created file set
        self.created_file_sets
            .lock()
            .unwrap()
            .insert(file_set_id, file_set_params);

        Ok(CreateFileSetResult {
            file_set_id,
            release_id,
        })
    }

    async fn find_file_set_by_files(
        &self,
        files: Vec<Sha1Checksum>,
    ) -> Result<Option<i64>, FileSetServiceError> {
        // Convert to BTreeSet for order-independent comparison
        let file_set: BTreeSet<Sha1Checksum> = files.into_iter().collect();

        // Check if we should fail this operation
        if self.fail_find_for.lock().unwrap().contains(&file_set) {
            return Err(FileSetServiceError::DatabaseError(
                "Mock find failure".to_string(),
            ));
        }

        // Look up in pre-configured results
        let result = self
            .checksum_to_file_set
            .lock()
            .unwrap()
            .get(&file_set)
            .copied();

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use core_types::FileType;

    use super::*;
    use crate::file_import::model::CreateReleaseParams;

    #[async_std::test]
    async fn test_mock_create_file_set() {
        let mock = MockFileSetService::new();

        let params = CreateFileSetParams {
            file_set_name: "Test Set".to_string(),
            file_set_file_name: "test.zip".to_string(),
            source: "Test".to_string(),
            file_type: FileType::Rom,
            system_ids: vec![1],
            files_in_file_set: vec![],
            create_release: None,
            dat_file_id: None,
        };

        let result = mock.create_file_set(params).await.unwrap();

        assert_eq!(result.file_set_id, 1);
        assert_eq!(result.release_id, None);
        assert!(mock.was_created("Test Set"));
        assert_eq!(mock.created_count(), 1);
    }

    #[async_std::test]
    async fn test_mock_create_file_set_with_release() {
        let mock = MockFileSetService::new();

        let params = CreateFileSetParams {
            file_set_name: "Test Set".to_string(),
            file_set_file_name: "test.zip".to_string(),
            source: "Test".to_string(),
            file_type: FileType::Rom,
            system_ids: vec![1],
            files_in_file_set: vec![],
            create_release: Some(CreateReleaseParams {
                release_name: "Test Release".to_string(),
                software_title_name: "Test Title".to_string(),
            }),
            dat_file_id: None,
        };

        let result = mock.create_file_set(params).await.unwrap();

        assert_eq!(result.file_set_id, 1);
        assert_eq!(result.release_id, Some(1));
    }

    #[async_std::test]
    async fn test_mock_create_file_set_failure() {
        let mock = MockFileSetService::new();
        mock.fail_create_for("Test Set");

        let params = CreateFileSetParams {
            file_set_name: "Test Set".to_string(),
            file_set_file_name: "test.zip".to_string(),
            source: "Test".to_string(),
            file_type: FileType::Rom,
            system_ids: vec![1],
            files_in_file_set: vec![],
            create_release: None,
            dat_file_id: None,
        };

        let result = mock.create_file_set(params).await;

        assert!(result.is_err());
        assert_eq!(mock.created_count(), 0);
    }

    #[async_std::test]
    async fn test_mock_find_file_set_by_files() {
        let mock = MockFileSetService::new();

        let checksums = vec![[1u8; 20], [2u8; 20]];
        mock.add_file_set_lookup(checksums.clone(), 42);

        let result = mock.find_file_set_by_files(checksums).await.unwrap();

        assert_eq!(result, Some(42));
    }

    #[async_std::test]
    async fn test_mock_find_file_set_by_files_not_found() {
        let mock = MockFileSetService::new();

        let checksums = vec![[1u8; 20], [2u8; 20]];

        let result = mock.find_file_set_by_files(checksums).await.unwrap();

        assert_eq!(result, None);
    }

    #[async_std::test]
    async fn test_mock_find_file_set_by_files_order_independent() {
        let mock = MockFileSetService::new();

        let checksums1 = vec![[1u8; 20], [2u8; 20]];
        let checksums2 = vec![[2u8; 20], [1u8; 20]]; // Different order

        mock.add_file_set_lookup(checksums1.clone(), 42);

        let result = mock.find_file_set_by_files(checksums2).await.unwrap();

        assert_eq!(result, Some(42));
    }

    #[async_std::test]
    async fn test_mock_find_file_set_by_files_failure() {
        let mock = MockFileSetService::new();

        let checksums = vec![[1u8; 20], [2u8; 20]];
        mock.fail_find_for(checksums.clone());

        let result = mock.find_file_set_by_files(checksums).await;

        assert!(result.is_err());
    }

    #[async_std::test]
    async fn test_custom_ids() {
        let mock = MockFileSetService::new();
        mock.set_next_file_set_id(100);
        mock.set_next_release_id(200);

        let params = CreateFileSetParams {
            file_set_name: "Test Set".to_string(),
            file_set_file_name: "test.zip".to_string(),
            source: "Test".to_string(),
            file_type: FileType::Rom,
            system_ids: vec![1],
            files_in_file_set: vec![],
            create_release: Some(CreateReleaseParams {
                release_name: "Test Release".to_string(),
                software_title_name: "Test Title".to_string(),
            }),
            dat_file_id: None,
        };

        let result = mock.create_file_set(params).await.unwrap();

        assert_eq!(result.file_set_id, 100);
        assert_eq!(result.release_id, Some(200));
    }

    #[async_std::test]
    async fn test_clear() {
        let mock = MockFileSetService::new();

        let params = CreateFileSetParams {
            file_set_name: "Test Set".to_string(),
            file_set_file_name: "test.zip".to_string(),
            source: "Test".to_string(),
            file_type: FileType::Rom,
            system_ids: vec![1],
            files_in_file_set: vec![],
            create_release: None,
            dat_file_id: None,
        };

        mock.create_file_set(params).await.unwrap();
        mock.add_file_set_lookup(vec![[1u8; 20]], 42);

        assert_eq!(mock.created_count(), 1);

        mock.clear();

        assert_eq!(mock.created_count(), 0);
        let result = mock
            .find_file_set_by_files(vec![[1u8; 20]])
            .await
            .unwrap();
        assert_eq!(result, None);
    }
}
