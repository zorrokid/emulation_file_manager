pub type Sha1Checksum = [u8; 20];
pub type FileSize = u64;

#[derive(Debug, Clone, PartialEq)]
pub struct ImportedFile {
    pub file_name: String,
    pub sha1_checksum: Sha1Checksum,
    pub file_size: FileSize,
}
