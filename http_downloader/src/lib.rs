use async_std::io::{ReadExt, WriteExt};
use async_std::{channel::{Receiver, Sender}, fs::File};
use core_types::events::HttpDownloadEvent;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(String),
    #[error("File IO error: {0}")]
    FileIoError(String),
    #[error("Download cancelled by user")]
    Cancelled,
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
    let url_string = url.to_string();
    
    match download_file_internal(url, target_dir, progress_tx, cancel_rx).await {
        Ok(result) => Ok(result),
        Err(e) => {
            // Send Failed event before returning error
            send_status_message(
                progress_tx,
                HttpDownloadEvent::Failed {
                    url: url_string,
                    error: e.to_string(),
                },
            )
            .await;
            Err(e)
        }
    }
}

async fn download_file_internal(
    url: &str,
    target_dir: &Path,
    progress_tx: &Sender<HttpDownloadEvent>,
    cancel_rx: &Receiver<()>,
) -> Result<DownloadResult, DownloadError> {
    let url_string = url.to_string(); // Clone once for reuse
    let buffer_size = 8192; // 8KB buffer
    let mut bytes_downloaded = 0;
    let mut last_event_reported = 0;
    let report_interval = 10 * buffer_size;

    // Create a client with default middleware (includes redirects)
    let client = surf::client().with(surf::middleware::Redirect::default());

    let mut response = client
        .get(url)
        .await
        .map_err(|e| DownloadError::RequestFailed(format!("Failed to send request: {}", e)))?;

    if !response.status().is_success() {
        return Err(DownloadError::RequestFailed(format!(
            "HTTP error: {}",
            response.status()
        )));
    }

    // Extract total size from Content-Length header
    let total_size = response
        .header("content-length")
        .and_then(|h| h.as_str().parse::<u64>().ok());

    send_status_message(
        progress_tx,
        HttpDownloadEvent::Started {
            url: url_string.clone(),
            total_size,
        },
    )
    .await;

    let file_name = extract_filename_from_url(url)
        .or_else(|| extract_filename_from_headers(&response))
        .unwrap_or_else(|| "downloaded_file".to_string());

    let file_path = target_dir.join(&file_name);

    let mut file = File::create(&file_path)
        .await
        .map_err(|e| DownloadError::FileIoError(format!("Failed to create file: {}", e)))?;

    // Take the body as an AsyncRead stream
    let mut body = response.take_body();

    // Stream the response body in chunks
    let mut buffer = vec![0u8; buffer_size];

    loop {
        // Check for cancellation
        if cancel_rx.try_recv().is_ok() {
            return Err(DownloadError::Cancelled);
        }

        let bytes_read = body
            .read(&mut buffer)
            .await
            .map_err(|e| DownloadError::RequestFailed(format!("Failed to read chunk: {}", e)))?;

        if bytes_read == 0 {
            break; // EOF
        }

        file.write_all(&buffer[..bytes_read])
            .await
            .map_err(|e| DownloadError::FileIoError(format!("Failed to write chunk: {}", e)))?;

        bytes_downloaded += bytes_read as u64;

        if bytes_downloaded - last_event_reported >= report_interval as u64 {
            last_event_reported = bytes_downloaded;
            send_status_message(
                progress_tx,
                HttpDownloadEvent::Progress {
                    url: url_string.clone(),
                    bytes_downloaded,
                },
            )
            .await;
        }
    }

    file.flush()
        .await
        .map_err(|e| DownloadError::FileIoError(format!("Failed to flush file: {}", e)))?;

    send_status_message(
        progress_tx,
        HttpDownloadEvent::Completed {
            url: url_string,
            file_path: file_path.clone(),
        },
    )
    .await;

    Ok(DownloadResult { file_path })
}

fn extract_filename_from_url(url: &str) -> Option<String> {
    url.split('/')
        .next_back()
        .filter(|s| !s.is_empty() && s.contains('.'))
        .map(|s| s.split('?').next().unwrap_or(s))
        .map(String::from)
}

fn extract_filename_from_headers(response: &surf::Response) -> Option<String> {
    response
        .header("content-disposition")?
        .as_str()
        .split("filename=")
        .nth(1)?
        .trim_matches(|c| c == '"' || c == '\'')
        .split(';')
        .next()
        .map(String::from)
}

async fn send_status_message(progress_tx: &Sender<HttpDownloadEvent>, event: HttpDownloadEvent) {
    if let Err(err) = progress_tx.send(event).await {
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
