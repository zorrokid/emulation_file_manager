use serde::{Deserialize, Serialize};
use std::string::ToString;
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter};

pub type Sha1Checksum = [u8; 20];
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

#[derive(Debug, Clone, PartialEq, Copy, EnumIter, Display)]
#[repr(u8)]
pub enum FileType {
    Rom = 1,
    #[strum(serialize = "Disk Image")]
    DiskImage = 2,
    #[strum(serialize = "Tape Image")]
    TapeImage = 3,
    Screenshot = 4,
    Manual = 5,
    #[strum(serialize = "Cover Scan")]
    CoverScan = 6,
    #[strum(serialize = "Memory Snapshot")]
    MemorySnapshot = 7,
    #[strum(serialize = "Loading Screen")]
    LoadingScreen = 8,
    #[strum(serialize = "Title Screen")]
    TitleScreen = 9,
    #[strum(serialize = "Manual Scan")]
    ManualScan = 10,
    #[strum(serialize = "Media Scan")]
    MediaScan = 11,
    #[strum(serialize = "Package Scan")]
    PackageScan = 12,
    #[strum(serialize = "Inlay Scan")]
    InlayScan = 13,
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
            FileType::PackageScan => "package_scan",
            FileType::InlayScan => "inlay_scan",
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
            12 => Ok(FileType::PackageScan),
            13 => Ok(FileType::InlayScan),
            _ => Err(CoreTypeError::ConversionError(
                "Failed convert to FileType".to_string(),
            )),
        }
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
    FileType::PackageScan,
    FileType::InlayScan,
];

pub const DOCUMENT_FILE_TYPES: &[FileType] = &[FileType::Manual];

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
}
