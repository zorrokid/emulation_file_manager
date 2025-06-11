use core::fmt;
use core_types::FileType as CoreFileType;
use std::fmt::{Display, Formatter};

use sqlx::FromRow;

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum FileType {
    Rom = 1,
    DiskImage = 2,
    TapeImage = 3,
    Screenshot = 4,
    Manual = 5,
    CoverScan = 6,
    MemorySnapshot = 7,
    LoadingScreen = 8,
    TitleScreen = 9,
    ManualScan = 10,
}

impl From<FileType> for CoreFileType {
    fn from(value: FileType) -> Self {
        match value {
            FileType::Rom => CoreFileType::Rom,
            FileType::DiskImage => CoreFileType::DiskImage,
            FileType::TapeImage => CoreFileType::TapeImage,
            FileType::Screenshot => CoreFileType::Screenshot,
            FileType::Manual => CoreFileType::Manual,
            FileType::CoverScan => CoreFileType::CoverScan,
            FileType::MemorySnapshot => CoreFileType::MemorySnapshot,
            FileType::LoadingScreen => CoreFileType::LoadingScreen,
            FileType::TitleScreen => CoreFileType::TitleScreen,
            FileType::ManualScan => CoreFileType::ManualScan,
        }
    }
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
        }
    }
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
            FileType::LoadingScreen => 8,
            FileType::TitleScreen => 9,
            FileType::ManualScan => 10,
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
            8 => Ok(FileType::LoadingScreen),
            9 => Ok(FileType::TitleScreen),
            10 => Ok(FileType::ManualScan),
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
            FileType::LoadingScreen => write!(f, "Loading Screen"),
            FileType::TitleScreen => write!(f, "Title Screen"),
            FileType::ManualScan => write!(f, "Manual Scan"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct FileInfo {
    pub id: i64,
    pub sha1_checksum: Vec<u8>,
    pub file_size: u64,
    pub archive_file_name: String,
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
pub struct FileSetFileInfo {
    pub file_set_id: i64,
    pub file_info_id: i64,
    pub file_name: String,
    pub sha1_checksum: Vec<u8>,
    pub file_size: i64,
    pub archive_file_name: String,
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
}

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct EmulatorSystem {
    pub id: i64,
    pub system_id: i64,
    pub system_name: String,
    pub arguments: String,
}

pub struct EmulatorSystemUpdateModel {
    pub id: Option<i64>,
    pub system_id: i64,
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
