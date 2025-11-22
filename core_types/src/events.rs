use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum HttpDownloadEvent {
    Started { total_size: Option<u64> },
    Progress { bytes_downloaded: u64 },
    Completed { file_path: PathBuf },
    Failed { error: String },
}

#[derive(Debug, Clone)]
pub enum SyncEvent {
    // TODO: use same events for upload and deletion, add process type field
    SyncStarted {
        total_files_count: i64,
    },
    FileUploadStarted {
        key: String,
        file_number: i64,
        total_files: i64,
    },
    PartUploaded {
        key: String,
        part: u32,
    },
    PartUploadFailed {
        key: String,
        error: String,
    },
    FileUploadCompleted {
        key: String,
        file_number: i64,
        total_files: i64,
    },
    FileUploadFailed {
        key: String,
        error: String,
        file_number: i64,
        total_files: i64,
    },
    SyncCompleted {},
    FileDeletionStarted {
        key: String,
        file_number: i64,
        total_files: i64,
    },
    FileDeletionCompleted {
        key: String,
        file_number: i64,
        total_files: i64,
    },
    FileDeletionFailed {
        key: String,
        error: String,
        file_number: i64,
        total_files: i64,
    },
}

#[derive(Debug, Clone)]
pub enum DownloadEvent {
    DownloadStarted { number_of_files: usize },
    FileDownloadStarted { key: String },
    FileDownloadProgress { key: String, bytes_downloaded: u64 },
    FileDownloadCompleted { key: String },
    FileDownloadFailed { key: String, error: String },
    DownloadCompleted,
}
