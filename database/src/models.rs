use std::fmt::Display;

use sqlx::FromRow;

// TODO move to better place
#[derive(Debug, Clone)]
pub struct PickedFileInfo {
    pub sha1_checksum: String,
    pub file_size: i64,
    pub file_name: String,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum FileType {
    Rom = 1,
    DiskImage = 2,
    TapeImage = 3,
    Screenshot = 4,
    Manual = 5,
    CoverScan = 6,
    MemorySnapshot = 7,
}

impl From<FileType> for i64 {
    fn from(value: FileType) -> Self {
        match value {
            FileType::Rom => 1,
            FileType::DiskImage => 2,
            FileType::TapeImage => 3,
            FileType::Screenshot => 4,
            FileType::Manual => 5,
            FileType::CoverScan => 6,
            FileType::MemorySnapshot => 7,
        }
    }
}

impl TryFrom<i64> for FileType {
    type Error = sqlx::Error;
    fn try_from(value: i64) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(FileType::Rom),
            2 => Ok(FileType::DiskImage),
            3 => Ok(FileType::TapeImage),
            4 => Ok(FileType::Screenshot),
            5 => Ok(FileType::Manual),
            6 => Ok(FileType::CoverScan),
            7 => Ok(FileType::MemorySnapshot),
            _ => Err(sqlx::Error::ColumnDecode {
                index: "file_type".into(),
                source: Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Invalid file type",
                )),
            }),
        }
    }
}

impl Display for FileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileType::Rom => write!(f, "Rom"),
            FileType::DiskImage => write!(f, "Disk Image"),
            FileType::TapeImage => write!(f, "Tape Image"),
            FileType::Screenshot => write!(f, "Screenshot"),
            FileType::Manual => write!(f, "Manual"),
            FileType::CoverScan => write!(f, "Cover Scan"),
            FileType::MemorySnapshot => write!(f, "Memory Snapshot"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct FileInfo {
    pub id: i64,
    pub sha1_checksum: String,
    pub file_size: i64,
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
}

#[derive(Debug, Clone, PartialEq)]
pub struct Release {
    pub id: i64,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct System {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct Emulator {
    pub id: i64,
    pub name: String,
    pub executable: String,
    pub extract_files: bool,
}

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct EmulatorSystem {
    pub system_id: i64,
    pub system_name: String,
    pub arguments: String,
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

pub enum SettingName {
    CollectionRootDir,
}

impl SettingName {
    pub fn as_str(&self) -> &'static str {
        match self {
            SettingName::CollectionRootDir => "collection_root_dir",
        }
    }
}
