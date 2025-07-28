use std::string::ToString;
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter};

pub type Sha1Checksum = [u8; 20];
pub type FileSize = u64;

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
