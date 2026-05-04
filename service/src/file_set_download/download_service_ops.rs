use crate::error::Error;
use crate::file_set_download::service::{DownloadResult, DownloadService};
use core_types::events::DownloadEvent;
use flume::Sender;
use std::sync::{Arc, Mutex};

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
    /// Did caller pass progress sender
    pub had_progress_tx: bool,
}

pub struct ConfiguredOutcome {
    /// Result returned from `download_file_set`.
    pub result: Result<DownloadResult, Error>,
    /// Progress events emitted.
    pub progress_events: Vec<DownloadEvent>,
}

impl Default for ConfiguredOutcome {
    fn default() -> Self {
        Self {
            result: Ok(DownloadResult::default()),
            progress_events: vec![],
        }
    }
}

/// Internal state for MockDownloadServiceOps.
///
/// Groups all mutable state into a single struct for simplified locking.
#[derive(Default)]
pub struct MockState {
    pub download_calls: Vec<DownloadCall>,
    pub outcome: ConfiguredOutcome,
}

/// Mock implementation for testing download service operations.
///
/// This mock tracks all download calls and can simulate failures, allowing comprehensive
/// testing without performing actual downloads.
///
/// # Examples
///
/// ```
/// use service::file_set_download::download_service_ops::{ConfiguredOutcome, DownloadServiceOps, MockDownloadServiceOps};
/// use service::file_set_download::service::DownloadResult;
///
/// #[async_std::main]
/// async fn main() {
///     // Test successful download
///     let mock = MockDownloadServiceOps::with_outcome(ConfiguredOutcome {
///         result: Ok(DownloadResult {
///             successful_downloads: 1,
///             ..Default::default()
///             }),
///         ..Default::default()
///     });
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
#[derive(Clone)]
pub struct MockDownloadServiceOps {
    state: Arc<Mutex<MockState>>,
}

impl Default for MockDownloadServiceOps {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(MockState::default())),
        }
    }
}

impl MockDownloadServiceOps {
    /// Creates a new mock that succeeds on all download operations.
    ///
    /// Use this for testing happy path scenarios where downloads should succeed.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates new mock with given state.
    pub fn with_state(state: Arc<Mutex<MockState>>) -> Self {
        Self { state }
    }

    /// Creates new mock with given outcome.
    pub fn with_outcome(outcome: ConfiguredOutcome) -> Self {
        Self {
            state: Arc::new(Mutex::new(MockState {
                outcome,
                ..Default::default()
            })),
        }
    }

    /// Returns all calls made to the `download_file_set` method.
    pub fn download_calls(&self) -> Vec<DownloadCall> {
        let state = self.state.lock().unwrap();
        state.download_calls.clone()
    }

    /// Returns the total number of download calls made.
    pub fn total_calls(&self) -> usize {
        let state = self.state.lock().unwrap();
        state.download_calls.len()
    }

    /// Clear all state (useful between tests)
    pub fn clear(&self) {
        let mut state = self.state.lock().unwrap();
        *state = MockState::default();
    }
}

#[async_trait::async_trait]
impl DownloadServiceOps for MockDownloadServiceOps {
    async fn download_file_set(
        &self,
        file_set_id: i64,
        extract_files: bool,
        progress_tx: Option<Sender<DownloadEvent>>,
    ) -> Result<DownloadResult, Error> {
        let (events, result) = {
            let mut state = self.state.lock().unwrap();

            let call = DownloadCall {
                file_set_id,
                extract_files,
                had_progress_tx: progress_tx.is_some(),
            };
            state.download_calls.push(call);
            (
                state.outcome.progress_events.clone(),
                state.outcome.result.clone(),
            )
        };

        if let Some(tx) = progress_tx.as_ref() {
            for event in &events {
                if tx.send(event.clone()).is_err() {
                    return Err(Error::DownloadError(
                        "mock: failed to send progress event".into(),
                    ));
                }
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[async_std::test]
    async fn test_mock_download_service_ops_success() {
        let events = [
            DownloadEvent::DownloadStarted { number_of_files: 1 },
            DownloadEvent::FileDownloadStarted { key: "key".into() },
            DownloadEvent::FileDownloadProgress {
                key: "key".into(),
                bytes_downloaded: 123,
            },
            DownloadEvent::FileDownloadProgress {
                key: "key".into(),
                bytes_downloaded: 123,
            },
            DownloadEvent::FileDownloadCompleted { key: "key".into() },
            DownloadEvent::DownloadCompleted,
        ];
        let outcome = ConfiguredOutcome {
            result: Ok(DownloadResult {
                successful_downloads: 1,
                ..Default::default()
            }),
            progress_events: events.clone().into(),
        };

        let mock = MockDownloadServiceOps::with_outcome(outcome);
        let (tx, rx) = flume::unbounded();

        let result = mock.download_file_set(123, true, Some(tx)).await;

        let mut event_count = 0;
        while let Ok(event) = rx.try_recv() {
            assert_eq!(event, events[event_count]);
            event_count += 1;
        }

        assert!(result.is_ok());

        // Verify the call was tracked
        assert_eq!(mock.total_calls(), 1);
        let calls = mock.download_calls();
        assert_eq!(calls.len(), 1);

        let call = &calls[0];
        assert_eq!(call.file_set_id, 123);
        assert!(call.extract_files);
        assert!(call.had_progress_tx);

        let download_result = result.unwrap();
        assert_eq!(download_result.successful_downloads, 1);
        assert_eq!(download_result.failed_downloads, 0);
    }

    #[async_std::test]
    async fn test_mock_download_service_ops_with_state_construction() {
        let outcome = ConfiguredOutcome {
            result: Ok(DownloadResult {
                successful_downloads: 1,
                ..Default::default()
            }),
            ..Default::default()
        };

        let mock_state = Arc::new(Mutex::new(MockState {
            outcome,
            ..Default::default()
        }));

        let mock = MockDownloadServiceOps::with_state(Arc::clone(&mock_state));
        let result = mock.download_file_set(123, true, None).await;

        assert!(result.is_ok());

        // Verify the call was tracked
        let state_guard = mock_state.lock().expect("poisoned lock");
        assert_eq!(state_guard.download_calls.len(), 1);

        let call = &state_guard.download_calls[0].clone();
        assert_eq!(call.file_set_id, 123);
        assert!(call.extract_files);
        assert!(!call.had_progress_tx);

        let download_result = result.unwrap();
        assert_eq!(download_result.successful_downloads, 1);
        assert_eq!(download_result.failed_downloads, 0);
    }

    #[async_std::test]
    async fn test_mock_download_service_ops_failure() {
        let events = [
            DownloadEvent::DownloadStarted { number_of_files: 1 },
            DownloadEvent::FileDownloadStarted { key: "key".into() },
            DownloadEvent::FileDownloadFailed {
                key: "key".into(),
                error: "Download error".into(),
            },
        ];
        let outcome = ConfiguredOutcome {
            result: Err(Error::DownloadError("Download error".into())),
            progress_events: events.clone().into(),
        };

        let mock = MockDownloadServiceOps::with_outcome(outcome);
        let (tx, rx) = flume::unbounded();

        let result = mock.download_file_set(123, true, Some(tx)).await;

        let mut event_count = 0;
        while let Ok(event) = rx.try_recv() {
            assert_eq!(event, events[event_count]);
            event_count += 1;
        }

        assert!(result.is_err());

        // Verify the call was tracked
        assert_eq!(mock.total_calls(), 1);
        let calls = mock.download_calls();
        assert_eq!(calls.len(), 1);

        let call = &calls[0];
        assert_eq!(call.file_set_id, 123);
        assert!(call.extract_files);
        assert!(call.had_progress_tx);

        let download_result = result.err().unwrap();
        assert_eq!(
            download_result,
            Error::DownloadError("Download error".into())
        );
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
        assert!(calls[0].extract_files);
        assert_eq!(calls[1].file_set_id, 2);
        assert!(!calls[1].extract_files);
        assert_eq!(calls[2].file_set_id, 3);
        assert!(calls[2].extract_files);
    }
}
