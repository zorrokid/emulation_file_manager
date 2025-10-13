use core_types::FileType;

pub enum CompressionLevel {
    Default,
    Fast,
    Good,
}

impl CompressionLevel {
    pub fn to_zstd_level(&self) -> i32 {
        match self {
            CompressionLevel::Fast => 1,
            CompressionLevel::Default => 3,
            CompressionLevel::Good => 6,
        }
    }
}

impl From<FileType> for CompressionLevel {
    fn from(file_type: FileType) -> Self {
        get_compression_level(&file_type)
    }
}

pub fn get_compression_level(file_type: &FileType) -> CompressionLevel {
    match file_type {
        FileType::Rom | FileType::DiskImage | FileType::TapeImage | FileType::MemorySnapshot => {
            CompressionLevel::Good
        }
        FileType::Screenshot
        | FileType::Manual
        | FileType::CoverScan
        | FileType::TitleScreen
        | FileType::LoadingScreen => CompressionLevel::Fast,
        _ => CompressionLevel::Default,
    }
}
