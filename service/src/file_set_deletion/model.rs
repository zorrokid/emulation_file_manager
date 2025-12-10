use database::models::FileInfo;

#[derive(Debug, Clone)]
pub struct FileDeletionResult {
    pub file_info: FileInfo,
    pub file_path: Option<String>,
    pub file_deletion_success: bool,
    pub error_messages: Vec<String>,
    pub is_deletable: bool,
    pub was_deleted_from_db: bool,
    pub cloud_sync_marked: bool,
}

impl FileDeletionResult {
    pub fn new(file_info: FileInfo) -> Self {
        Self {
            file_info,
            file_path: None,
            file_deletion_success: false,
            error_messages: vec![],
            is_deletable: false,
            was_deleted_from_db: false,
            cloud_sync_marked: false,
        }
    }
}
