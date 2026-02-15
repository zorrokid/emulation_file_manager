use chrono::NaiveDateTime;
use core::fmt;
use core_types::{DocumentType, FileSyncStatus, FileType, Sha1Checksum, item_type::ItemType};
use std::fmt::{Display, Formatter};

use sqlx::FromRow;

#[derive(Debug, Clone, PartialEq)]
pub struct FileInfo {
    pub id: i64,
    pub sha1_checksum: Sha1Checksum,
    pub file_size: u64,
    pub archive_file_name: String,
    pub file_type: FileType,
}

impl From<&FileSetFileInfo> for FileInfo {
    fn from(file_set_file_info: &FileSetFileInfo) -> Self {
        Self {
            id: file_set_file_info.file_info_id,
            sha1_checksum: file_set_file_info.sha1_checksum,
            file_size: file_set_file_info.file_size as u64,
            archive_file_name: file_set_file_info.archive_file_name.clone(),
            file_type: file_set_file_info.file_type,
        }
    }
}

impl FileInfo {
    pub fn generate_cloud_key(&self) -> String {
        format!(
            "{}/{}",
            self.file_type.to_string().to_lowercase(),
            self.archive_file_name
        )
    }
}

/// FileSet is a container of files related to a single software title release.
/// For example a rom set, set of disk images, set of scanned
/// documents or screen shots.
///
/// When collection file is exported from
/// the system, it's exported as a single file, which is a zip archive containing all the files
/// related to the collection file and name of the zip arhive if the file_name field.
///
/// Each file in the collection file is represented by a FileInfo object and they can belong to
/// multiple collection files.
#[derive(Debug, Clone, PartialEq)]
pub struct FileSet {
    pub id: i64,
    pub file_name: String,
    pub file_type: FileType,
    pub name: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileSetFileInfo {
    pub file_set_id: i64,
    pub file_info_id: i64,
    pub file_name: String,
    pub sha1_checksum: Sha1Checksum,
    pub file_size: i64,
    pub archive_file_name: String,
    pub file_type: FileType,
    pub sort_order: i64,
}

impl Display for FileSetFileInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.file_name)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Release {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReleaseExtended {
    pub id: i64,
    pub name: String,
    pub system_names: Vec<String>,
    pub software_title_names: Vec<String>,
    pub file_types: Vec<FileType>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct System {
    pub id: i64,
    pub name: String,
}

impl Display for System {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct Emulator {
    pub id: i64,
    pub name: String,
    pub executable: String,
    pub extract_files: bool,
    pub system_id: i64,
    pub arguments: String, // as JSON string
}

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct DocumentViewer {
    pub id: i64,
    pub name: String,
    pub executable: String,
    pub document_type: DocumentType,
    pub arguments: String, // as JSON string
    pub cleanup_temp_files: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Franchise {
    pub id: i64,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SoftwareTitle {
    pub id: i64,
    pub name: String,
    pub franchise_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct FileSyncLog {
    pub id: i64,
    pub file_info_id: i64,
    pub sync_time: NaiveDateTime,
    pub status: FileSyncStatus,
    pub message: String,
    pub cloud_key: String,
}

pub struct FileSyncLogWithFileInfo {
    pub id: i64,
    pub file_info_id: i64,
    pub sync_time: NaiveDateTime,
    pub status: FileSyncStatus,
    pub message: String,
    pub cloud_key: String,
    pub sha1_checksum: Vec<u8>,
    pub file_size: i64,
    pub archive_file_name: String,
    pub file_type: FileType,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReleaseItem {
    pub id: i64,
    pub release_id: i64,
    pub item_type: ItemType,
    pub notes: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DatFile {
    pub id: i64,
    pub dat_id: i64,
    pub name: String,
    pub description: String,
    pub version: String,
    pub date: Option<String>,
    pub author: String,
    pub homepage: Option<String>,
    pub url: Option<String>,
    pub subset: Option<String>,
    pub system_id: i64,
    pub imported_at: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DatGame {
    pub id: i64,
    pub dat_file_id: i64,
    pub name: String,
    pub game_id: Option<String>,
    pub description: String,
    pub cloneof: Option<String>,
    pub cloneofid: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DatRom {
    pub id: i64,
    pub dat_game_id: i64,
    pub name: String,
    pub size: i64,
    pub crc: String,
    pub md5: String,
    pub sha1: String,
    pub sha256: Option<String>,
    pub status: Option<String>,
    pub serial: Option<String>,
    pub header: Option<String>,
}
