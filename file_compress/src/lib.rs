pub mod file_outputter;
pub use file_outputter::{CompressionMethod, FileOutputter};
use sha1::{Digest, Sha1};
use std::{fs::File, io::Read, path::Path};

use zip::ZipArchive;

pub fn read_zip_file(
    file_path: &str,
    output_dir: &str,
    compression_type: CompressionMethod,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(file_path)?;
    let mut archive = ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        println!("Filename: {}", file.name());

        if file.is_file() {
            let mut hasher = Sha1::new();
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;
            hasher.update(&buffer);
            let checksum = hasher.finalize();
            println!("SHA-1 checksum: {:x}", checksum);

            let output_path = Path::new(output_dir).join(file.name());
            compression_type.output(&output_path, &buffer, file.name())?;
        }
    }
    Ok(())
}
