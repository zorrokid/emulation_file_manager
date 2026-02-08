use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use core_types::{FileSetEqualitySpecs, Sha1Checksum};

use crate::file_set::{
    CreateFileSetParams, CreateFileSetResult, FileSetServiceError, FileSetServiceOps,
};

/// Internal state for MockFileSetService
#[derive(Default)]
struct MockState {
    next_file_set_id: i64,
    next_release_id: i64,
    created_file_sets: HashMap<i64, CreateFileSetParams>,
    checksum_to_file_set: HashMap<BTreeSet<Sha1Checksum>, i64>,
    fail_create_for: Vec<String>,
    fail_find_for: Vec<BTreeSet<Sha1Checksum>>,
}

/// Mock implementation of FileSetServiceOps for testing
///
/// This mock allows you to:
/// - Simulate file set creation with configurable IDs
/// - Test failure scenarios
/// - Pre-configure file set lookups by checksums
/// - Verify what operations were performed
#[derive(Clone)]
pub struct MockFileSetService {
    state: Arc<Mutex<MockState>>,
}

impl Default for MockFileSetService {
    fn default() -> Self {
        Self::new()
    }
}

impl MockFileSetService {
    /// Create a new mock file set service
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MockState {
                next_file_set_id: 1,
                next_release_id: 1,
                ..Default::default()
            })),
        }
    }

    /// Add a pre-configured file set lookup result
    pub fn add_file_set_lookup(&self, checksums: Vec<Sha1Checksum>, file_set_id: i64) {
        let checksum_set: BTreeSet<Sha1Checksum> = checksums.into_iter().collect();
        self.state
            .lock()
            .unwrap()
            .checksum_to_file_set
            .insert(checksum_set, file_set_id);
    }

    /// Make create_file_set fail for a specific file set name
    pub fn fail_create_for(&self, file_set_name: impl Into<String>) {
        self.state
            .lock()
            .unwrap()
            .fail_create_for
            .push(file_set_name.into());
    }

    /// Make find_file_set_by_files fail for specific checksums
    pub fn fail_find_for(&self, checksums: Vec<Sha1Checksum>) {
        let checksum_set: BTreeSet<Sha1Checksum> = checksums.into_iter().collect();
        self.state.lock().unwrap().fail_find_for.push(checksum_set);
    }

    /// Get all created file sets
    pub fn get_created_file_sets(&self) -> Vec<(i64, CreateFileSetParams)> {
        let state = self.state.lock().unwrap();
        state
            .created_file_sets
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
        self.state.lock().unwrap().created_file_sets.len()
    }

    /// Check if a file set was created with a specific name
    pub fn was_created(&self, file_set_name: &str) -> bool {
        self.state
            .lock()
            .unwrap()
            .created_file_sets
            .values()
            .any(|params| params.file_set_name == file_set_name)
    }

    /// Set the next file set ID to be returned
    pub fn set_next_file_set_id(&self, id: i64) {
        self.state.lock().unwrap().next_file_set_id = id;
    }

    /// Set the next release ID to be returned
    pub fn set_next_release_id(&self, id: i64) {
        self.state.lock().unwrap().next_release_id = id;
    }

    /// Clear all state (useful between tests)
    pub fn clear(&self) {
        let mut state = self.state.lock().unwrap();
        *state = MockState {
            next_file_set_id: 1,
            next_release_id: 1,
            ..Default::default()
        };
    }
}

#[async_trait]
impl FileSetServiceOps for MockFileSetService {
    async fn create_file_set(
        &self,
        file_set_params: CreateFileSetParams,
    ) -> Result<CreateFileSetResult, FileSetServiceError> {
        let mut state = self.state.lock().unwrap();

        // Check if we should fail this operation
        if state
            .fail_create_for
            .contains(&file_set_params.file_set_name)
        {
            return Err(FileSetServiceError::DatabaseError(format!(
                "Mock create failure for file set: {}",
                file_set_params.file_set_name
            )));
        }

        // Generate IDs
        let file_set_id = state.next_file_set_id;
        state.next_file_set_id += 1;

        let release_id = if file_set_params.create_release.is_some() {
            let id = state.next_release_id;
            state.next_release_id += 1;
            Some(id)
        } else {
            None
        };

        // Store the created file set
        state.created_file_sets.insert(file_set_id, file_set_params);

        Ok(CreateFileSetResult {
            file_set_id,
            release_id,
        })
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
    }
}
