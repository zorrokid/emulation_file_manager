use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex};

use async_std::channel::Sender;
use async_trait::async_trait;
use core_types::events::{DownloadEvent, SyncEvent};

use crate::{CloudStorageError, ops::CloudStorageOps};

/// Internal state for MockCloudStorage.
///
/// Groups all mutable state into a single struct for simplified locking.
#[derive(Default)]
struct MockState {
    /// Stores uploaded files (cloud_key -> file content)
    uploaded_files: HashMap<String, Vec<u8>>,
    /// Tracks which files were deleted
    deleted_files: HashSet<String>,
    /// Keys that should fail on upload
    fail_upload_keys: HashSet<String>,
    /// Keys that should fail on deletion
    fail_delete_keys: HashSet<String>,
    /// Number of parts to simulate in multipart upload (default: 3)
    simulate_part_count: u32,
}

/// Mock implementation of CloudStorageOps for testing
///
/// This mock allows you to:
/// - Simulate file uploads and deletions
/// - Test failure scenarios
/// - Verify what operations were performed
/// - Simulate progress events
#[derive(Clone)]
pub struct MockCloudStorage {
    state: Arc<Mutex<MockState>>,
}

impl Default for MockCloudStorage {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(MockState {
                simulate_part_count: 3,
                ..Default::default()
            })),
        }
    }
}

impl MockCloudStorage {
    /// Create a new mock cloud storage
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a file that already exists in cloud storage (for testing)
    pub fn add_file(&self, cloud_key: impl Into<String>, content: Vec<u8>) {
        let mut state = self.state.lock().unwrap();
        state.uploaded_files.insert(cloud_key.into(), content);
    }

    /// Add a file with dummy content
    pub fn add_file_dummy(&self, cloud_key: impl Into<String>) {
        let key = cloud_key.into();
        let content = format!("mock-content-for-{}", key).into_bytes();
        let mut state = self.state.lock().unwrap();
        state.uploaded_files.insert(key, content);
    }

    /// Make upload fail for a specific key
    pub fn fail_upload_for(&self, cloud_key: impl Into<String>) {
        let mut state = self.state.lock().unwrap();
        state.fail_upload_keys.insert(cloud_key.into());
    }

    /// Make deletion fail for a specific key
    pub fn fail_delete_for(&self, cloud_key: impl Into<String>) {
        let mut state = self.state.lock().unwrap();
        state.fail_delete_keys.insert(cloud_key.into());
    }

    /// Set how many parts to simulate in multipart upload
    pub fn set_part_count(&self, count: u32) {
        let mut state = self.state.lock().unwrap();
        state.simulate_part_count = count;
    }

    /// Check if a file was uploaded
    pub fn was_uploaded(&self, cloud_key: &str) -> bool {
        let state = self.state.lock().unwrap();
        state.uploaded_files.contains_key(cloud_key)
    }

    /// Check if a file was deleted
    pub fn was_deleted(&self, cloud_key: &str) -> bool {
        let state = self.state.lock().unwrap();
        state.deleted_files.contains(cloud_key)
    }

    /// Get the content of an uploaded file
    pub fn get_uploaded_content(&self, cloud_key: &str) -> Option<Vec<u8>> {
        let state = self.state.lock().unwrap();
        state.uploaded_files.get(cloud_key).cloned()
    }

    /// Get all uploaded file keys
    pub fn get_uploaded_keys(&self) -> Vec<String> {
        let state = self.state.lock().unwrap();
        state.uploaded_files.keys().cloned().collect()
    }

    /// Get all deleted file keys
    pub fn get_deleted_keys(&self) -> Vec<String> {
        let state = self.state.lock().unwrap();
        state.deleted_files.iter().cloned().collect()
    }

    /// Get the number of uploaded files
    pub fn uploaded_count(&self) -> usize {
        let state = self.state.lock().unwrap();
        state.uploaded_files.len()
    }

    /// Get the number of deleted files
    pub fn deleted_count(&self) -> usize {
        let state = self.state.lock().unwrap();
        state.deleted_files.len()
    }

    /// Clear all state (useful between tests)
    pub fn clear(&self) {
        let mut state = self.state.lock().unwrap();
        *state = MockState {
            simulate_part_count: 3,
            ..Default::default()
        };
    }
}

#[async_trait]
impl CloudStorageOps for MockCloudStorage {
    async fn upload_file(
        &self,
        file_path: &Path,
        cloud_key: &str,
        progress_tx: Option<&Sender<SyncEvent>>,
    ) -> Result<(), CloudStorageError> {
        // Check if we should fail this upload
        let should_fail = {
            let state = self.state.lock().unwrap();
            state.fail_upload_keys.contains(cloud_key)
        };

        if should_fail {
            // Send failure event if requested
            if let Some(tx) = progress_tx {
                tx.send(SyncEvent::PartUploadFailed {
                    key: cloud_key.to_string(),
                    error: "Mock upload failure".to_string(),
                })
                .await
                .ok();
            }

            return Err(CloudStorageError::Other(format!(
                "Mock upload failure for key: {}",
                cloud_key
            )));
        }

        // Simulate multipart upload progress
        let part_count = {
            let state = self.state.lock().unwrap();
            state.simulate_part_count
        };

        if let Some(tx) = progress_tx {
            for part in 1..=part_count {
                tx.send(SyncEvent::PartUploaded {
                    key: cloud_key.to_string(),
                    part,
                })
                .await
                .ok();
            }
        }

        // Read the actual file content (or use dummy data if file doesn't exist)
        // This allows testing without creating actual files
        let content = async_std::fs::read(file_path)
            .await
            .unwrap_or_else(|_| format!("mock-content-for-{}", file_path.display()).into_bytes());

        // Store the uploaded file
        let mut state = self.state.lock().unwrap();
        state.uploaded_files.insert(cloud_key.to_string(), content);

        Ok(())
    }

    async fn delete_file(&self, cloud_key: &str) -> Result<(), CloudStorageError> {
        let mut state = self.state.lock().unwrap();

        // Check if we should fail this deletion
        if state.fail_delete_keys.contains(cloud_key) {
            return Err(CloudStorageError::Other(format!(
                "Mock deletion failure for key: {}",
                cloud_key
            )));
        }

        // Mark as deleted
        state.deleted_files.insert(cloud_key.to_string());

        // Remove from uploaded files (if it was there)
        state.uploaded_files.remove(cloud_key);

        Ok(())
    }

    async fn file_exists(&self, cloud_key: &str) -> Result<bool, CloudStorageError> {
        let state = self.state.lock().unwrap();
        // A file exists if it's in uploaded_files and not in deleted_files
        let is_uploaded = state.uploaded_files.contains_key(cloud_key);
        let is_deleted = state.deleted_files.contains(cloud_key);

        Ok(is_uploaded && !is_deleted)
    }

    // TODO: simulate download progress events
    async fn download_file(
        &self,
        cloud_key: &str,
        _destination_path: &Path,
        _progress_tx: Option<&Sender<DownloadEvent>>,
    ) -> Result<(), CloudStorageError> {
        let state = self.state.lock().unwrap();
        if state.uploaded_files.contains_key(cloud_key) {
            Ok(())
        } else {
            Err(CloudStorageError::Other(format!(
                "Mock download failed, key not found: {}",
                cloud_key
            )))
        }
    }

    async fn move_file(
        &self,
        source_cloud_key: &str,
        destination_cloud_key: &str,
    ) -> Result<(), CloudStorageError> {
        let mut state = self.state.lock().unwrap();
        if let Some(content) = state.uploaded_files.remove(source_cloud_key) {
            state.uploaded_files.insert(destination_cloud_key.to_string(), content);
            Ok(())
        } else {
            Err(CloudStorageError::Other(format!(
                "Mock move failed, source key not found: {}",
                source_cloud_key
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[async_std::test]
    async fn test_mock_upload() {
        let mock = MockCloudStorage::new();

        mock.upload_file(Path::new("/test/file.zst"), "rom/game.zst", None)
            .await
            .unwrap();

        assert!(mock.was_uploaded("rom/game.zst"));
        assert_eq!(mock.uploaded_count(), 1);
    }

    #[async_std::test]
    async fn test_mock_upload_failure() {
        let mock = MockCloudStorage::new();
        mock.fail_upload_for("rom/game.zst");

        let result = mock
            .upload_file(Path::new("/test/file.zst"), "rom/game.zst", None)
            .await;

        assert!(result.is_err());
        assert!(!mock.was_uploaded("rom/game.zst"));
    }

    #[async_std::test]
    async fn test_mock_delete() {
        let mock = MockCloudStorage::new();
        mock.add_file_dummy("rom/game.zst");

        mock.delete_file("rom/game.zst").await.unwrap();

        assert!(mock.was_deleted("rom/game.zst"));
        assert!(!mock.was_uploaded("rom/game.zst"));
    }

    #[async_std::test]
    async fn test_mock_delete_failure() {
        let mock = MockCloudStorage::new();
        mock.add_file_dummy("rom/game.zst");
        mock.fail_delete_for("rom/game.zst");

        let result = mock.delete_file("rom/game.zst").await;

        assert!(result.is_err());
        assert!(!mock.was_deleted("rom/game.zst"));
        assert!(mock.was_uploaded("rom/game.zst")); // Still there
    }

    #[async_std::test]
    async fn test_file_exists() {
        let mock = MockCloudStorage::new();

        // File doesn't exist initially
        assert!(!mock.file_exists("rom/game.zst").await.unwrap());

        // Add file
        mock.add_file_dummy("rom/game.zst");
        assert!(mock.file_exists("rom/game.zst").await.unwrap());

        // Delete file
        mock.delete_file("rom/game.zst").await.unwrap();
        assert!(!mock.file_exists("rom/game.zst").await.unwrap());
    }

    #[async_std::test]
    async fn test_upload_with_progress_events() {
        let mock = MockCloudStorage::new();
        mock.set_part_count(5);

        let (tx, rx) = async_std::channel::unbounded();

        mock.upload_file(Path::new("/test/file.zst"), "rom/game.zst", Some(&tx))
            .await
            .unwrap();

        // Count part uploaded events
        let mut part_count = 0;
        while let Ok(event) = rx.try_recv() {
            if matches!(event, SyncEvent::PartUploaded { .. }) {
                part_count += 1;
            }
        }

        assert_eq!(part_count, 5);
        assert!(mock.was_uploaded("rom/game.zst"));
    }

    #[async_std::test]
    async fn test_clear() {
        let mock = MockCloudStorage::new();

        mock.add_file_dummy("file1.zst");
        mock.add_file_dummy("file2.zst");
        mock.delete_file("file1.zst").await.unwrap();

        assert_eq!(mock.uploaded_count(), 1);
        assert_eq!(mock.deleted_count(), 1);

        mock.clear();

        assert_eq!(mock.uploaded_count(), 0);
        assert_eq!(mock.deleted_count(), 0);
    }

    #[async_std::test]
    async fn test_move_file() {
        let mock = MockCloudStorage::new();
        mock.add_file_dummy("rom/old_game.zst");
        mock.move_file("rom/old_game.zst", "rom/new_game.zst")
            .await
            .unwrap();
        assert!(!mock.was_uploaded("rom/old_game.zst"));
        assert!(mock.was_uploaded("rom/new_game.zst"));
    }
}
