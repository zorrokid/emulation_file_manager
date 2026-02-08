pub mod events;
pub mod item_type;

use hex::FromHex;
use serde::{Deserialize, Serialize};
use std::string::ToString;
use strum_macros::{Display, EnumIter};

pub type Sha1Checksum = [u8; 20];

pub fn sha1_bytes_to_hex_string(checksum: &Sha1Checksum) -> String {
    checksum.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn sha1_from_hex_string(hex_str: &str) -> Result<Sha1Checksum, CoreTypeError> {
    let bytes = <[u8; 20]>::from_hex(hex_str).map_err(|_| {
        CoreTypeError::ConversionError("Failed to convert hex string to Sha1Checksum".to_string())
    })?;
    Ok(bytes)
}

pub type FileSize = u64;

#[derive(Debug, Clone)]
pub enum CoreTypeError {
    ConversionError(String),
    InvalidArgumentType(String),
}

impl std::fmt::Display for CoreTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoreTypeError::ConversionError(msg) => write!(f, "Conversion Error: {}", msg),
            CoreTypeError::InvalidArgumentType(msg) => write!(f, "Invalid Argument Type: {}", msg),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Copy, EnumIter)]
pub enum DocumentType {
    Pdf = 1,
}

impl From<DocumentType> for i64 {
    fn from(value: DocumentType) -> Self {
        match value {
            DocumentType::Pdf => 1,
        }
    }
}

impl TryFrom<i64> for DocumentType {
    type Error = CoreTypeError;
    fn try_from(value: i64) -> Result<Self, CoreTypeError> {
        match value {
            1 => Ok(DocumentType::Pdf),
            _ => Err(CoreTypeError::ConversionError(
                "Failed convert to DocumentType".to_string(),
            )),
        }
    }
}

impl std::fmt::Display for DocumentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DocumentType::Pdf => write!(f, "PDF"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportedFile {
    pub original_file_name: String,
    pub archive_file_name: String,
    pub sha1_checksum: Sha1Checksum,
    pub file_size: FileSize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReadFile {
    pub file_name: String,
    pub sha1_checksum: Sha1Checksum,
    pub file_size: FileSize,
}

#[derive(Debug, Clone, PartialEq, Copy, EnumIter, Display, Eq, Ord, PartialOrd, Hash)]
#[repr(u8)]
pub enum FileType {
    Rom = 1,
    #[strum(serialize = "Disk Image")]
    DiskImage = 2,
    #[strum(serialize = "Tape Image")]
    TapeImage = 3,
    Screenshot = 4, // This file type doesn't have associated ItemType
    /// Manual document file (e.g. pdf)
    // TODO: will be deprecated, use generic Document type instead
    Manual = 5,
    #[strum(serialize = "Cover Scan")]
    // TODO: will be deprecated, use generic Scan type instead
    CoverScan = 6, // This file type doesn't have associated ItemType (can be from box, inlay,
    // manual, etc)
    #[strum(serialize = "Memory Snapshot")]
    MemorySnapshot = 7, // This file type doesn't have associated ItemType
    // TODO: will be deprecated, use generic Screenshot type instead
    #[strum(serialize = "Loading Screen")]
    LoadingScreen = 8, // This file type doesn't have associated ItemType
    // TODO: will be deprecated, use generic Screenshot type instead
    #[strum(serialize = "Title Screen")]
    TitleScreen = 9, // This file type doesn't have associated ItemType
    #[strum(serialize = "Manual Scan")]
    // TODO: will be deprecated, use generic Scan type instead
    ManualScan = 10,
    #[strum(serialize = "Media Scan")]
    // TODO: will be deprecated, use generic Scan type instead
    MediaScan = 11,
    // This is not currently used, use BoxScan or InlayScan instead instead instead instead
    // #[strum(serialize = "Package Scan")]
    //PackageScan = 12,
    #[strum(serialize = "Inlay Scan")]
    // TODO: will be deprecated, use generic Scan type instead
    InlayScan = 13,
    #[strum(serialize = "Box Scan")]
    // TODO: will be deprecated, use generic Scan type instead
    BoxScan = 14,
    /// Box document file (e.g. pdf)
    // TODO: will be deprecated, use generic Document type instead
    Box = 15,
    Document = 16, // Generic document type (e.g. pdf)
    Scan = 17,     // Generic scan type (e.g. jpg, png)
}

impl FileType {
    pub fn dir_name(&self) -> &'static str {
        match self {
            FileType::Rom => "rom",
            FileType::DiskImage => "disk_image",
            FileType::TapeImage => "tape_image",
            FileType::Screenshot => "screenshot",
            FileType::Manual => "manual",
            FileType::CoverScan => "cover_scan",
            FileType::MemorySnapshot => "memory_snapshot",
            FileType::LoadingScreen => "loading_screen",
            FileType::TitleScreen => "title_screen",
            FileType::ManualScan => "manual_scan",
            FileType::MediaScan => "media_scan",
            //FileType::PackageScan => "package_scan",
            FileType::InlayScan => "inlay_scan",
            FileType::BoxScan => "box_scan",
            FileType::Box => "box",
            FileType::Document => "document",
            FileType::Scan => "scan",
        }
    }

    pub fn to_db_int(&self) -> u8 {
        *self as u8
    }

    pub fn from_db_int(value: u8) -> Result<Self, CoreTypeError> {
        match value {
            1 => Ok(FileType::Rom),
            2 => Ok(FileType::DiskImage),
            3 => Ok(FileType::TapeImage),
            4 => Ok(FileType::Screenshot),
            5 => Ok(FileType::Manual),
            6 => Ok(FileType::CoverScan),
            7 => Ok(FileType::MemorySnapshot),
            8 => Ok(FileType::LoadingScreen),
            9 => Ok(FileType::TitleScreen),
            10 => Ok(FileType::ManualScan),
            11 => Ok(FileType::MediaScan),
            //12 => Ok(FileType::PackageScan),
            13 => Ok(FileType::InlayScan),
            14 => Ok(FileType::BoxScan),
            15 => Ok(FileType::Box),
            16 => Ok(FileType::Document),
            17 => Ok(FileType::Scan),
            _ => Err(CoreTypeError::ConversionError(
                "Failed convert to FileType".to_string(),
            )),
        }
    }

    pub fn is_media_type(&self) -> bool {
        matches!(
            self,
            FileType::DiskImage | FileType::TapeImage | FileType::Rom | FileType::MemorySnapshot
        )
    }
}

pub const EMULATOR_FILE_TYPES: &[FileType] = &[
    FileType::DiskImage,
    FileType::TapeImage,
    FileType::Rom,
    FileType::MemorySnapshot,
];

pub const IMAGE_FILE_TYPES: &[FileType] = &[
    FileType::ManualScan,
    FileType::CoverScan,
    FileType::Screenshot,
    FileType::MediaScan,
    FileType::LoadingScreen,
    FileType::TitleScreen,
    //FileType::PackageScan,
    FileType::InlayScan,
    FileType::BoxScan,
    FileType::Scan,
];

pub const ACTIVE_FILE_TYPES: &[FileType] = &[
    FileType::DiskImage,
    FileType::TapeImage,
    FileType::Rom,
    FileType::MemorySnapshot,
    FileType::Screenshot,
    FileType::Document,
    FileType::Scan,
];

pub const DOCUMENT_FILE_TYPES: &[FileType] = &[FileType::Manual, FileType::Box, FileType::Document];

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Display, Serialize, Deserialize)]
pub enum ArgumentType {
    #[strum(to_string = "{name}")]
    Flag { name: String },
    #[strum(to_string = "{name} {value}")]
    FlagWithValue { name: String, value: String },
    #[strum(to_string = "{name}={value}")]
    FlagEqualsValue { name: String, value: String },
    // TODO: add more types as needed
}

impl TryFrom<&str> for ArgumentType {
    type Error = CoreTypeError;
    fn try_from(argument_string: &str) -> Result<Self, Self::Error> {
        parse_argument(argument_string)
    }
}

fn parse_argument(argument_string: &str) -> Result<ArgumentType, CoreTypeError> {
    if argument_string.contains('=') {
        // Handle flag with value (e.g. --flag=value)
        let parts: Vec<&str> = argument_string.splitn(2, '=').collect();
        if parts.len() != 2 {
            return Err(CoreTypeError::InvalidArgumentType(
                "Invalid flag equals value format".to_string(),
            ));
        }
        return Ok(ArgumentType::FlagEqualsValue {
            name: parts[0].to_string(),
            value: parts[1].to_string(),
        });
    } else if argument_string.contains(' ') {
        // Handle flag with value (e.g. -f 1 -f 1 --flag 1)
        let parts: Vec<&str> = argument_string.splitn(2, ' ').collect();
        if parts.len() != 2 {
            return Err(CoreTypeError::InvalidArgumentType(
                "Invalid flag with value format".to_string(),
            ));
        }
        return Ok(ArgumentType::FlagWithValue {
            name: parts[0].to_string(),
            value: parts[1].to_string(),
        });
    }

    Ok(ArgumentType::Flag {
        name: argument_string.to_string(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SettingName {
    CollectionRootDir,
    S3EndPoint,
    S3Region,
    S3Bucket,
    S3FileSyncEnabled,
}

impl SettingName {
    pub fn as_str(&self) -> &'static str {
        match self {
            SettingName::CollectionRootDir => "collection_root_dir",
            SettingName::S3EndPoint => "s3_endpoint",
            SettingName::S3Region => "s3_region",
            SettingName::S3Bucket => "s3_bucket",
            SettingName::S3FileSyncEnabled => "s3_file_sync_enabled",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u8)]
pub enum FileSyncStatus {
    UploadPending,
    UploadInProgress,
    UploadCompleted,
    UploadFailed,
    DeletionPending,
    DeletionInProgress,
    DeletionCompleted,
    DeletionFailed,
}

impl FileSyncStatus {
    pub fn to_db_int(&self) -> u8 {
        *self as u8
    }

    pub fn from_db_int(value: u8) -> Result<Self, CoreTypeError> {
        match value {
            0 => Ok(FileSyncStatus::UploadPending),
            1 => Ok(FileSyncStatus::UploadInProgress),
            2 => Ok(FileSyncStatus::UploadCompleted),
            3 => Ok(FileSyncStatus::UploadFailed),
            4 => Ok(FileSyncStatus::DeletionPending),
            5 => Ok(FileSyncStatus::DeletionInProgress),
            6 => Ok(FileSyncStatus::DeletionCompleted),
            7 => Ok(FileSyncStatus::DeletionFailed),
            _ => Err(CoreTypeError::ConversionError(
                "Failed convert to FileSyncStatus".to_string(),
            )),
        }
    }
}

pub struct FileSetEqualitySpecs {
    pub file_set_name: String,
    pub file_set_file_name: String,
    pub file_type: FileType,
    pub source: String,
    pub file_set_file_info: Vec<FileSetFileEqualitySpecs>,
}

pub struct FileSetFileEqualitySpecs {
    pub file_name: String,
    pub file_type: FileType,
    pub sha1_checksum: Sha1Checksum,
}

// TODO add test for From<&str> for ArgumentType
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse_argument() {
        assert_eq!(
            parse_argument("--flag").unwrap(),
            ArgumentType::Flag {
                name: "--flag".to_string()
            }
        );
        assert_eq!(
            parse_argument("-f 1").unwrap(),
            ArgumentType::FlagWithValue {
                name: "-f".to_string(),
                value: "1".to_string()
            }
        );
        assert_eq!(
            parse_argument("--flag=1").unwrap(),
            ArgumentType::FlagEqualsValue {
                name: "--flag".to_string(),
                value: "1".to_string()
            }
        );
    }
    #[test]
    fn test_argument_type_from_str() {
        assert_eq!(
            ArgumentType::try_from("--flag").unwrap(),
            ArgumentType::Flag {
                name: "--flag".to_string()
            }
        );
        assert_eq!(
            ArgumentType::try_from("-f 1").unwrap(),
            ArgumentType::FlagWithValue {
                name: "-f".to_string(),
                value: "1".to_string()
            }
        );
        assert_eq!(
            ArgumentType::try_from("--flag=1").unwrap(),
            ArgumentType::FlagEqualsValue {
                name: "--flag".to_string(),
                value: "1".to_string()
            }
        );
    }

    #[test]
    fn test_is_media_type() {
        assert!(FileType::Rom.is_media_type());
        assert!(FileType::DiskImage.is_media_type());
        assert!(FileType::TapeImage.is_media_type());
        assert!(!FileType::Screenshot.is_media_type());
        assert!(!FileType::Manual.is_media_type());
        assert!(!FileType::CoverScan.is_media_type());
        assert!(FileType::MemorySnapshot.is_media_type());
        assert!(!FileType::LoadingScreen.is_media_type());
        assert!(!FileType::TitleScreen.is_media_type());
        assert!(!FileType::ManualScan.is_media_type());
        assert!(!FileType::MediaScan.is_media_type());
        //assert!(!FileType::PackageScan.is_media_type());
        assert!(!FileType::InlayScan.is_media_type());
    }
}
