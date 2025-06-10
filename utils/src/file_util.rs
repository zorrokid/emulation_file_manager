use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use core_types::Sha1Checksum;
use sha1::digest::{consts::U20, generic_array::GenericArray};

pub fn is_zip_file(path: &Path) -> Result<bool, Box<dyn std::error::Error>> {
    let mut file = File::open(path)?;
    let mut signature = [0u8; 4];
    file.read_exact(&mut signature)?;
    Ok(signature == [0x50, 0x4B, 0x03, 0x04]) // ZIP file signature
}

pub fn get_file_sha1(path: &PathBuf) -> Result<Sha1Checksum, Box<dyn std::error::Error>> {
    use sha1::{Digest, Sha1};
    let mut file = File::open(path)?;
    let mut hasher = Sha1::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    let sha1_checksum: GenericArray<u8, U20> = hasher.finalize();
    let sha1_checksum: Sha1Checksum = sha1_checksum.into();
    Ok(sha1_checksum)
}
