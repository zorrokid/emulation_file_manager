use core_types::events::HttpDownloadEvent;
use flume::{Receiver, Sender};
use futures::StreamExt;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::io::AsyncWriteExt;

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(String),
    #[error("File IO error: {0}")]
    FileIoError(String),
    #[error("Download cancelled by user")]
    Cancelled,
    #[error("Filename error: {0}")]
    FilenameError(#[from] FilenameError),
    #[error("Processing download chunk failed: {0}")]
    ChunkFailed(String),
    #[error("Cleanup failed: {0}")]
    CleanUp(String),
}

pub struct DownloadResult {
    pub file_path: PathBuf,
}

/// Download a file from a URL to a specified directory.
///
/// # Arguments
///
/// * `url` - The URL to download from.
/// * `target_dir` - The directory where the file will be saved.
/// * `progress_tx` - A channel sender to report download progress events.
/// * `cancel_rx` - A channel receiver to listen for cancellation signals.
///
/// # Returns
///
/// A `Result` containing the `DownloadResult` with the path to the downloaded file,
/// or a `DownloadError` if the operation fails.
pub async fn download_file(
    url: &str,
    target_dir: &Path,
    progress_tx: &Sender<HttpDownloadEvent>,
    cancel_rx: &Receiver<()>,
) -> Result<DownloadResult, DownloadError> {
    match download_file_internal_with_reqwest(url, target_dir, progress_tx, cancel_rx).await {
        Ok(result) => Ok(result),
        Err(e) => {
            // Send Failed event before returning error
            send_status_message(
                progress_tx,
                HttpDownloadEvent::Failed {
                    error: e.to_string(),
                },
            )
            .await;
            Err(e)
        }
    }
}

async fn download_file_internal_with_reqwest(
    url: &str,
    target_dir: &Path,
    progress_tx: &Sender<HttpDownloadEvent>,
    cancel_rx: &Receiver<()>,
) -> Result<DownloadResult, DownloadError> {
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| DownloadError::RequestFailed(e.to_string()))?
        .error_for_status()
        .map_err(|e| DownloadError::RequestFailed(e.to_string()))?;
    let total_size = response.content_length();

    send_status_message(progress_tx, HttpDownloadEvent::Started { total_size }).await;

    let file_name = extract_filename_from_headers_reqwest(&response)
        .map_err(DownloadError::FilenameError)?
        .or_else(|| extract_filename_from_url(url));

    let Some(file_name) = file_name else {
        return Err(DownloadError::FilenameError(FilenameError::Filename));
    };

    let file_path = target_dir.join(&file_name);

    let mut file: tokio::fs::File = tokio::fs::File::create(&file_path)
        .await
        .map_err(|e| DownloadError::FileIoError(format!("Error creating file: {}", e)))?;

    let mut stream = response.bytes_stream();
    let mut bytes_downloaded = 0u64;

    while let Some(chunk) = stream.next().await {
        if cancel_rx.try_recv().is_ok() {
            cleanup_partial_file(&file_path).await;
            return Err(DownloadError::Cancelled);
        }

        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(e) => {
                cleanup_partial_file(&file_path).await;
                return Err(DownloadError::ChunkFailed(e.to_string()));
            }
        };

        if let Err(e) = file.write_all(&chunk).await {
            cleanup_partial_file(&file_path).await;
            return Err(DownloadError::FileIoError(e.to_string()));
        }
        bytes_downloaded += chunk.len() as u64;
        send_status_message(
            progress_tx,
            HttpDownloadEvent::Progress { bytes_downloaded },
        )
        .await;
    }

    if let Err(e) = file.flush().await {
        cleanup_partial_file(&file_path).await;
        return Err(DownloadError::FileIoError(e.to_string()));
    }
    send_status_message(
        progress_tx,
        HttpDownloadEvent::Completed {
            file_path: file_path.clone(),
        },
    )
    .await;

    Ok(DownloadResult { file_path })
}

async fn cleanup_partial_file(file_path: &Path) {
    if let Err(e) = tokio::fs::remove_file(&file_path).await {
        tracing::warn!(error = ?e, path = ?file_path, "Failed to remove partial download");
    }
}

fn extract_filename_from_url(url: &str) -> Option<String> {
    url.split('/')
        .next_back()
        .filter(|s| !s.is_empty() && s.contains('.'))
        .map(|s| s.split('?').next().unwrap_or(s))
        .map(String::from)
}

#[derive(Error, Debug)]
pub enum FilenameError {
    #[error("invalid Content-Disposition header content")]
    ContentDispositionContent,
    #[error("invalid encoded UTF filename")]
    EncodedUtf,
    #[error("invalid filename")]
    Filename,
}

// TODO: tests
fn extract_filename_from_headers_reqwest(
    response: &reqwest::Response,
) -> Result<Option<String>, FilenameError> {
    use reqwest::header::CONTENT_DISPOSITION;
    use urlencoding::decode;

    let Some(value) = response.headers().get(CONTENT_DISPOSITION) else {
        return Ok(None);
    };

    let value_str = value
        .to_str()
        .map_err(|_| FilenameError::ContentDispositionContent)?;

    // If UTF-8 filename not found return plain ASCII filename as fallback
    let mut plain_filename: Option<String> = None;

    for part in value_str.split(';').map(str::trim) {
        if let Some((name, raw_value)) = part.split_once('=') {
            // UTF filename returned once found
            if name.eq_ignore_ascii_case("filename*") {
                let encoded = raw_value.trim_matches('"');
                let encoded = encoded
                    .strip_prefix("UTF-8''")
                    .or_else(|| encoded.strip_prefix("utf-8''"))
                    .unwrap_or(encoded);

                let decoded = decode(encoded).map_err(|_| FilenameError::EncodedUtf)?;

                let normalized = normalize_filename(&decoded)?;
                return Ok(Some(normalized));
            }
            // ASCII filename set if found
            if name.eq_ignore_ascii_case("filename") {
                let filename = raw_value.trim_matches('"');
                let normalized = normalize_filename(filename)?;
                if !normalized.is_empty() {
                    plain_filename = Some(normalized);
                }
            }
        }
    }
    Ok(plain_filename)
}

// TODO: tests
fn normalize_filename(raw: &str) -> Result<String, FilenameError> {
    let name = raw
        .rsplit(['/', '\\'])
        .next()
        .ok_or(FilenameError::Filename)?
        .trim();

    if name.is_empty() || name == "." || name == ".." {
        return Err(FilenameError::Filename);
    }

    let sanitized = sanitize_filename::sanitize(name);

    if sanitized.is_empty() {
        return Err(FilenameError::Filename);
    }

    Ok(sanitized)
}

async fn send_status_message(progress_tx: &Sender<HttpDownloadEvent>, event: HttpDownloadEvent) {
    if let Err(err) = progress_tx.send(event) {
        eprintln!("Failed to send download progress event: {}", err);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_filename_from_url() {
        assert_eq!(
            extract_filename_from_url("https://example.com/file.zip"),
            Some("file.zip".to_string())
        );
        assert_eq!(
            extract_filename_from_url("https://example.com/path/to/file.zip"),
            Some("file.zip".to_string())
        );
        assert_eq!(
            extract_filename_from_url("https://example.com/file.zip?query=param"),
            Some("file.zip".to_string())
        );
        assert_eq!(extract_filename_from_url("https://example.com/"), None);
    }
}
