use async_std::fs::File;
use async_std::io::WriteExt;
use futures::StreamExt;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(String),
    #[error("File IO error: {0}")]
    FileIoError(String),
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
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
///
/// # Returns
///
/// A `Result` containing the `DownloadResult` with the path to the downloaded file,
/// or a `DownloadError` if the operation fails.
pub async fn download_file(url: &str, target_dir: &Path) -> Result<DownloadResult, DownloadError> {
    let client = reqwest::Client::new();
    
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| DownloadError::RequestFailed(format!("Failed to send request: {}", e)))?;

    if !response.status().is_success() {
        return Err(DownloadError::RequestFailed(format!(
            "HTTP error: {}",
            response.status()
        )));
    }

    let file_name = extract_filename_from_url(url)
        .or_else(|| extract_filename_from_headers(&response))
        .unwrap_or_else(|| "downloaded_file".to_string());
    
    let file_path = target_dir.join(&file_name);
    
    let mut file = File::create(&file_path)
        .await
        .map_err(|e| DownloadError::FileIoError(format!("Failed to create file: {}", e)))?;

    let mut stream = response.bytes_stream();
    
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| {
            DownloadError::RequestFailed(format!("Failed to read chunk: {}", e))
        })?;
        file.write_all(&chunk)
            .await
            .map_err(|e| DownloadError::FileIoError(format!("Failed to write chunk: {}", e)))?;
    }

    file.flush()
        .await
        .map_err(|e| DownloadError::FileIoError(format!("Failed to flush file: {}", e)))?;

    Ok(DownloadResult { file_path })
}

fn extract_filename_from_url(url: &str) -> Option<String> {
    url.split('/')
        .last()
        .filter(|s| !s.is_empty() && s.contains('.'))
        .map(|s| s.split('?').next().unwrap_or(s))
        .map(String::from)
}

fn extract_filename_from_headers(response: &reqwest::Response) -> Option<String> {
    response
        .headers()
        .get(reqwest::header::CONTENT_DISPOSITION)?
        .to_str()
        .ok()?
        .split("filename=")
        .nth(1)?
        .trim_matches(|c| c == '"' || c == '\'')
        .split(';')
        .next()
        .map(String::from)
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
        assert_eq!(
            extract_filename_from_url("https://example.com/"),
            None
        );
    }
}
