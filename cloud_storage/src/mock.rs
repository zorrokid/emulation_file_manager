use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex};

use async_std::channel::Sender;
use async_trait::async_trait;

use crate::events::DownloadEvent;
use crate::{CloudStorageError, SyncEvent, ops::CloudStorageOps};

/// Mock implementation of CloudStorageOps for testing
///
/// This mock allows you to:
/// - Simulate file uploads and deletions
/// - Test failure scenarios
/// - Verify what operations were performed
/// - Simulate progress events
#[derive(Clone, Default)]
pub struct MockCloudStorage {
    /// Stores uploaded files (cloud_key -> file content)
    uploaded_files: Arc<Mutex<HashMap<String, Vec<u8>>>>,

    /// Tracks which files were deleted
    deleted_files: Arc<Mutex<HashSet<String>>>,

    /// Keys that should fail on upload
    fail_upload_keys: Arc<Mutex<HashSet<String>>>,

    /// Keys that should fail on deletion
    fail_delete_keys: Arc<Mutex<HashSet<String>>>,

    /// Number of parts to simulate in multipart upload (default: 3)
    simulate_part_count: Arc<Mutex<u32>>,
}

impl MockCloudStorage {
    /// Create a new mock cloud storage
    pub fn new() -> Self {
        Self {
            uploaded_files: Arc::new(Mutex::new(HashMap::new())),
            deleted_files: Arc::new(Mutex::new(HashSet::new())),
            fail_upload_keys: Arc::new(Mutex::new(HashSet::new())),
            fail_delete_keys: Arc::new(Mutex::new(HashSet::new())),
            simulate_part_count: Arc::new(Mutex::new(3)),
        }
    }

    /// Add a file that already exists in cloud storage (for testing)
    pub fn add_file(&self, cloud_key: impl Into<String>, content: Vec<u8>) {
        self.uploaded_files
            .lock()
            .unwrap()
            .insert(cloud_key.into(), content);
    }

    /// Add a file with dummy content
    pub fn add_file_dummy(&self, cloud_key: impl Into<String>) {
        let key = cloud_key.into();
        let content = format!("mock-content-for-{}", key).into_bytes();
        self.uploaded_files.lock().unwrap().insert(key, content);
    }

    /// Make upload fail for a specific key
    pub fn fail_upload_for(&self, cloud_key: impl Into<String>) {
        self.fail_upload_keys
            .lock()
            .unwrap()
            .insert(cloud_key.into());
    }

    /// Make deletion fail for a specific key
    pub fn fail_delete_for(&self, cloud_key: impl Into<String>) {
        self.fail_delete_keys
            .lock()
            .unwrap()
            .insert(cloud_key.into());
    }

    /// Set how many parts to simulate in multipart upload
    pub fn set_part_count(&self, count: u32) {
        *self.simulate_part_count.lock().unwrap() = count;
    }

    /// Check if a file was uploaded
    pub fn was_uploaded(&self, cloud_key: &str) -> bool {
        self.uploaded_files.lock().unwrap().contains_key(cloud_key)
    }

    /// Check if a file was deleted
    pub fn was_deleted(&self, cloud_key: &str) -> bool {
        self.deleted_files.lock().unwrap().contains(cloud_key)
    }

    /// Get the content of an uploaded file
    pub fn get_uploaded_content(&self, cloud_key: &str) -> Option<Vec<u8>> {
        self.uploaded_files.lock().unwrap().get(cloud_key).cloned()
    }

    /// Get all uploaded file keys
    pub fn get_uploaded_keys(&self) -> Vec<String> {
        self.uploaded_files
            .lock()
            .unwrap()
            .keys()
            .cloned()
            .collect()
    }

    /// Get all deleted file keys
    pub fn get_deleted_keys(&self) -> Vec<String> {
        self.deleted_files.lock().unwrap().iter().cloned().collect()
    }

    /// Get the number of uploaded files
    pub fn uploaded_count(&self) -> usize {
        self.uploaded_files.lock().unwrap().len()
    }

    /// Get the number of deleted files
    pub fn deleted_count(&self) -> usize {
        self.deleted_files.lock().unwrap().len()
    }

    /// Clear all state (useful between tests)
    pub fn clear(&self) {
        self.uploaded_files.lock().unwrap().clear();
        self.deleted_files.lock().unwrap().clear();
        self.fail_upload_keys.lock().unwrap().clear();
        self.fail_delete_keys.lock().unwrap().clear();
        *self.simulate_part_count.lock().unwrap() = 3;
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
        if self.fail_upload_keys.lock().unwrap().contains(cloud_key) {
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
        let part_count = *self.simulate_part_count.lock().unwrap();
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
        self.uploaded_files
            .lock()
            .unwrap()
            .insert(cloud_key.to_string(), content);

        Ok(())
    }

    async fn delete_file(&self, cloud_key: &str) -> Result<(), CloudStorageError> {
        // Check if we should fail this deletion
        if self.fail_delete_keys.lock().unwrap().contains(cloud_key) {
            return Err(CloudStorageError::Other(format!(
                "Mock deletion failure for key: {}",
                cloud_key
            )));
        }

        // Mark as deleted
        self.deleted_files
            .lock()
            .unwrap()
            .insert(cloud_key.to_string());

        // Remove from uploaded files (if it was there)
        self.uploaded_files.lock().unwrap().remove(cloud_key);

        Ok(())
    }

    async fn file_exists(&self, cloud_key: &str) -> Result<bool, CloudStorageError> {
        // A file exists if it's in uploaded_files and not in deleted_files
        let is_uploaded = self.uploaded_files.lock().unwrap().contains_key(cloud_key);
        let is_deleted = self.deleted_files.lock().unwrap().contains(cloud_key);

        Ok(is_uploaded && !is_deleted)
    }

    // TODO: simulate download progress events
    async fn download_file(
        &self,
        cloud_key: &str,
        _destination_path: &Path,
        _progress_tx: Option<&Sender<DownloadEvent>>,
    ) -> Result<(), CloudStorageError> {
        let uploaded_files = self.uploaded_files.lock().unwrap();
        if uploaded_files.contains_key(cloud_key) {
            Ok(())
        } else {
            Err(CloudStorageError::Other(format!(
                "Mock download failed, key not found: {}",
                cloud_key
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
}
