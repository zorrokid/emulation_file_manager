pub mod file_outputter;
pub use file_outputter::{CompressionMethod, FileOutputter};
use sha1::{Digest, Sha1};
use std::{collections::HashMap, fs::File, io::Read, path::Path};

use zip::ZipArchive;

pub fn read_zip_file(
    file_path: &str,
    output_dir: &str,
    compression_type: CompressionMethod,
) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    println!("Reading zip file: {}", file_path);
    let file = File::open(file_path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut hash_map = HashMap::new();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        println!("File: {}", file.name());

        if file.is_file() {
            let mut hasher = Sha1::new();
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;
            hasher.update(&buffer);
            let checksum = hasher.finalize();
            println!("Checksum: {:x}", checksum);
            hash_map.insert(file.name().to_string(), format!("{:x}", checksum));

            let output_path = Path::new(output_dir);
            println!("Output path: {}", output_path.display());
            compression_type.output(output_path, &buffer, file.name())?;
        }
    }
    Ok(hash_map)
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
