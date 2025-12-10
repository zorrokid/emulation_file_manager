use database::models::FileInfo;

#[derive(Debug, Clone)]
pub struct FileDeletionResult {
    pub file_info: FileInfo,
    pub file_path: Option<String>,
    /// Accumulated error messages during deletion process
    pub error_messages: Vec<String>,
    /// Whether the file is eligible for deletion (when file is not used in any file sets)
    pub is_deletable: bool,
    /// None = not attempted, Some(true) = success, Some(false) = failed
    pub file_deletion_success: Option<bool>,
    /// None = not attempted, Some(true) = success, Some(false) = failed
    pub db_deletion_success: Option<bool>,
    /// None = not attempted, Some(true) = success, Some(false) = failed
    pub cloud_delete_marked_successfully: Option<bool>,
}

impl FileDeletionResult {
    pub fn new(file_info: FileInfo) -> Self {
        Self {
            file_info,
            file_path: None,
            file_deletion_success: None,
            error_messages: Vec::new(),
            is_deletable: false,
            db_deletion_success: None,
            cloud_delete_marked_successfully: None,
        }
    }
}
