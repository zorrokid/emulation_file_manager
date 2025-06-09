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
