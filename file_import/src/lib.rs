pub mod file_outputter;
pub use file_outputter::{CompressionMethod, FileOutputter};
use sha1::{Digest, Sha1};
use std::{collections::HashMap, fs::File, io::Read, path::Path};

use zip::ZipArchive;

/// Reads the give zip file and imports it to the output directory in given compression method.
/// Calculate the checksum of each file in the zip archive and return a hash map with file names
/// and their checksums.
///
/// # Arguments
///
/// * `file_path` - The path to the zip file.
/// * `output_dir` - The directory where the files will be extracted.
/// * `compression_type` - The compression method to use for the output files.
///
/// # Returns
///
/// A `Result` containing a hash map with file names and their checksums, or an error if the
/// operation fails.
///
pub fn read_zip_file(
    file_path: &str,
    output_dir: &str,
    compression_type: CompressionMethod,
) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    let file = File::open(file_path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut file_name_to_checksum_map = HashMap::new();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;

        if file.is_file() {
            let mut hasher = Sha1::new();
            let mut buffer = [0u8; 8192]; // 8KB buffer
            loop {
                let bytes_read = file.read(&mut buffer)?;
                if bytes_read == 0 {
                    break; // EOF
                }
                hasher.update(&buffer[..bytes_read]);
            }
            let checksum = hasher.finalize();
            file_name_to_checksum_map.insert(file.name().to_string(), format!("{:x}", checksum));

            let output_path = Path::new(output_dir);
            compression_type.output(output_path, &mut file)?;
        }
    }
    Ok(file_name_to_checksum_map)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;
    use tempfile::tempdir;
    use zip::write::FileOptions;

    #[test]
    fn test_read_zip_file() {
        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path();
        let buffer = b"Hello, world!";
        let method = CompressionMethod::Zstd;
        let file_name = "test";

        let zip_file_path = output_path.join("test.zip");
        let zip_file = File::create(&zip_file_path).unwrap();
        let mut zip_writer = zip::ZipWriter::new(zip_file);
        let file_options: FileOptions<'_, ()> = FileOptions::default();
        zip_writer.start_file(file_name, file_options).unwrap();
        zip_writer.write_all(buffer).unwrap();
        zip_writer.finish().unwrap();
        let result = read_zip_file(
            zip_file_path.to_str().unwrap(),
            output_path.to_str().unwrap(),
            method,
        );
        assert!(result.is_ok());
        let hash_map = result.unwrap();
        assert_eq!(hash_map.len(), 1);
    }
}
