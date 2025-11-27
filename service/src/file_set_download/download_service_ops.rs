use crate::error::Error;
use crate::file_set_download::service::{DownloadResult, DownloadService};
use async_std::channel::Sender;
use core_types::events::DownloadEvent;
use std::sync::{Arc, Mutex};
use thumbnails::ThumbnailPathMap;

/// Trait for download service operations.
///
/// This trait abstracts file set download functionality to allow for different implementations,
/// including mocks for testing purposes.
#[async_trait::async_trait]
pub trait DownloadServiceOps: Send + Sync {
    /// Downloads a file set and prepares it for use.
    ///
    /// # Arguments
    /// * `file_set_id` - ID of the file set to download
    /// * `extract_files` - Whether to extract files from archives
    /// * `progress_tx` - Optional channel for progress events
    ///
    /// # Returns
    /// * `Ok(DownloadResult)` on successful download
    /// * `Err(Error)` if download fails
    async fn download_file_set(
        &self,
        file_set_id: i64,
        extract_files: bool,
        progress_tx: Option<Sender<DownloadEvent>>,
    ) -> Result<DownloadResult, Error>;
}

/// Default implementation that performs actual file set downloads.
#[async_trait::async_trait]
impl DownloadServiceOps for DownloadService {
    async fn download_file_set(
        &self,
        file_set_id: i64,
        extract_files: bool,
        progress_tx: Option<Sender<DownloadEvent>>,
    ) -> Result<DownloadResult, Error> {
        self.download_file_set(file_set_id, extract_files, progress_tx)
            .await
    }
}

/// Represents a recorded call to a download service operation.
///
/// Used by `MockDownloadServiceOps` to track and verify download calls in tests.
#[derive(Debug, Clone)]
pub struct DownloadCall {
    /// File set ID that was requested
    pub file_set_id: i64,
    /// Whether files should be extracted
    pub extract_files: bool,
}

/// Mock implementation for testing download service operations.
///
/// This mock tracks all download calls and can simulate failures, allowing comprehensive
/// testing without performing actual downloads.
///
/// # Examples
///
/// ```
/// use service::file_set_download::download_service_ops::{DownloadServiceOps, MockDownloadServiceOps};
///
/// #[async_std::main]
/// async fn main() {
///     // Test successful download
///     let mock = MockDownloadServiceOps::new();
///     let result = mock.download_file_set(1, true, None).await;
///     assert!(result.is_ok());
///
///     // Verify calls
///     assert_eq!(mock.total_calls(), 1);
///     let calls = mock.download_calls();
///     assert_eq!(calls[0].file_set_id, 1);
///     assert_eq!(calls[0].extract_files, true);
/// }
/// ```
#[derive(Clone, Default)]
pub struct MockDownloadServiceOps {
    should_fail: bool,
    error_message: Option<String>,
    download_calls: Arc<Mutex<Vec<DownloadCall>>>,
}

impl MockDownloadServiceOps {
    /// Creates a new mock that succeeds on all download operations.
    ///
    /// Use this for testing happy path scenarios where downloads should succeed.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new mock that fails on all download operations with the given error message.
    ///
    /// Use this for testing error handling paths in your code.
    ///
    /// # Arguments
    /// * `error_msg` - The error message to return when download operations fail
    ///
    /// # Examples
    ///
    /// ```
    /// use service::file_set_download::download_service_ops::MockDownloadServiceOps;
    ///
    /// let mock = MockDownloadServiceOps::with_failure("Network error");
    /// // All download operations will now fail with "Network error" error
    /// ```
    pub fn with_failure(error_msg: impl Into<String>) -> Self {
        Self {
            should_fail: true,
            error_message: Some(error_msg.into()),
            ..Default::default()
        }
    }

    /// Returns all calls made to the `download_file_set` method.
    pub fn download_calls(&self) -> Vec<DownloadCall> {
        self.download_calls.lock().unwrap().clone()
    }

    /// Returns the total number of download calls made.
    pub fn total_calls(&self) -> usize {
        self.download_calls.lock().unwrap().len()
    }
}

#[async_trait::async_trait]
impl DownloadServiceOps for MockDownloadServiceOps {
    async fn download_file_set(
        &self,
        file_set_id: i64,
        extract_files: bool,
        _progress_tx: Option<Sender<DownloadEvent>>,
    ) -> Result<DownloadResult, Error> {
        let call = DownloadCall {
            file_set_id,
            extract_files,
        };
        self.download_calls.lock().unwrap().push(call);

        if self.should_fail {
            return Err(Error::DownloadError(
                self.error_message
                    .clone()
                    .unwrap_or_else(|| "Mock download failed".to_string()),
            ));
        }

        Ok(DownloadResult {
            successful_downloads: 1,
            failed_downloads: 0,
            thumbnail_path_map: ThumbnailPathMap::new(),
            output_file_names: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[async_std::test]
    async fn test_mock_download_service_ops_success() {
        let mock = MockDownloadServiceOps::new();

        let result = mock.download_file_set(123, true, None).await;

        assert!(result.is_ok());

        // Verify the call was tracked
        assert_eq!(mock.total_calls(), 1);
        let calls = mock.download_calls();
        assert_eq!(calls.len(), 1);

        let call = &calls[0];
        assert_eq!(call.file_set_id, 123);
        assert_eq!(call.extract_files, true);

        let download_result = result.unwrap();
        assert_eq!(download_result.successful_downloads, 1);
        assert_eq!(download_result.failed_downloads, 0);
    }

    #[async_std::test]
    async fn test_mock_download_service_ops_failure() {
        let mock = MockDownloadServiceOps::with_failure("Simulated network error");

        let result = mock.download_file_set(456, false, None).await;

        assert!(result.is_err());

        // Verify the call was tracked even though it failed
        assert_eq!(mock.total_calls(), 1);

        match result {
            Err(Error::DownloadError(msg)) => {
                assert_eq!(msg, "Simulated network error");
            }
            _ => panic!("Expected DownloadError"),
        }
    }

    #[async_std::test]
    async fn test_mock_tracks_multiple_calls() {
        let mock = MockDownloadServiceOps::new();

        mock.download_file_set(1, true, None).await.unwrap();
        mock.download_file_set(2, false, None).await.unwrap();
        mock.download_file_set(3, true, None).await.unwrap();

        assert_eq!(mock.total_calls(), 3);
        let calls = mock.download_calls();
        assert_eq!(calls[0].file_set_id, 1);
        assert_eq!(calls[0].extract_files, true);
        assert_eq!(calls[1].file_set_id, 2);
        assert_eq!(calls[1].extract_files, false);
        assert_eq!(calls[2].file_set_id, 3);
        assert_eq!(calls[2].extract_files, true);
    }
}
