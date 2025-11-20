# HTTP Downloader

A simple asynchronous HTTP file downloader library for downloading files from URLs.

## Features

- Asynchronous file downloads using `async-std`
- Streaming downloads to handle large files efficiently
- Automatic filename extraction from URL or Content-Disposition headers
- Simple error handling

## Usage

```rust
use http_downloader::download_file;
use std::path::Path;

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = "https://example.com/file.zip";
    let target_dir = Path::new("/tmp");
    
    let result = download_file(url, target_dir).await?;
    println!("File downloaded to: {}", result.file_path.display());
    
    Ok(())
}
```

## Integration with File Import

The downloaded file can be used directly with the file import service:

```rust
use http_downloader::download_file;
use service::download_service::DownloadService;

let download_service = DownloadService::new(repository_manager, settings);
let prepare_result = download_service
    .download_and_prepare_import(url, file_type, temp_dir)
    .await?;
```

This will download the file and prepare it for import into the collection.

## Future Improvements

The following enhancements are planned for future releases:

### High Priority
- **Progress Reporting** - Show download progress (bytes downloaded, percentage, ETA)
- **Error Handling & Retry** - Better error messages, automatic retry on network failures
- **URL Validation** - Validate URL before starting download
- **Download Cancellation** - Allow users to cancel in-progress downloads

### Medium Priority
- **Resume Capability** - Support resuming interrupted downloads using HTTP Range headers
- **File Size Preview** - Show expected file size before downloading (from Content-Length header)
- **Multiple URLs** - Batch download multiple files at once
- **Download History** - Keep track of previously downloaded URLs

### Low Priority
- **Bandwidth Throttling** - Limit download speed if needed
- **Custom Headers** - Support for authentication/custom HTTP headers
- **Checksum Verification** - Verify downloaded file integrity
- **Mirror Support** - Try alternative URLs if primary fails
