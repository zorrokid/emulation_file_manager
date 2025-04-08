use sqlx::FromRow;

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum CollectionFileType {
    Rom,
    DiskImage,
    TapeImage,
    Screenshot,
    Manual,
    CoverScan,
    MemorySnapshot,
}

impl From<CollectionFileType> for i64 {
    fn from(value: CollectionFileType) -> Self {
        match value {
            CollectionFileType::Rom => 1,
            CollectionFileType::DiskImage => 2,
            CollectionFileType::TapeImage => 3,
            CollectionFileType::Screenshot => 4,
            CollectionFileType::Manual => 5,
            CollectionFileType::CoverScan => 6,
            CollectionFileType::MemorySnapshot => 7,
        }
    }
}

impl From<i64> for CollectionFileType {
    fn from(value: i64) -> Self {
        match value {
            1 => CollectionFileType::Rom,
            2 => CollectionFileType::DiskImage,
            3 => CollectionFileType::TapeImage,
            4 => CollectionFileType::Screenshot,
            5 => CollectionFileType::Manual,
            6 => CollectionFileType::CoverScan,
            7 => CollectionFileType::MemorySnapshot,
            _ => panic!("Invalid CollectionFileType value"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct FileInfo {
    pub id: i64,
    pub file_name: String,
    pub sha1_checksum: String,
    pub file_size: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CollectionFile {
    pub id: i64,
    pub file_name: String,
    pub file_type: CollectionFileType,
}
