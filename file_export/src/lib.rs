use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

use sha1::{Digest, Sha1};

#[derive(Clone, Debug)]
pub enum ExportType {
    CombinedZipArhive,
    IndividualZipFiles,
    IndividualFilesWithoutCompression,
}

pub fn export_files(
    file_path: &Path,
    output_dir: &Path,
    // key is the archive file name, value is the output file name
    output_file_name_mapping: HashMap<String, String>,
    // key is the archive file name, value is the checksum
    filename_checksum_mapping: HashMap<String, String>,
    // TODO
    _export_type: ExportType,
) -> Result<(), Box<dyn std::error::Error>> {
    for (archive_file_name, output_file_name) in output_file_name_mapping {
        println!(
            "Exporting file: {} to {}",
            archive_file_name, output_file_name
        );
        // souce files are in zstd format
        let file_path = file_path.join(&archive_file_name).with_extension("zst");
        println!(
            "File path: {}",
            file_path.to_str().unwrap_or("Invalid path")
        );
        let file = File::open(&file_path)?;
        // create decoder to decompress the zstd file
        let mut zstd_reader = zstd::Decoder::new(file)?;
        let output_file_path = output_dir.join(&output_file_name);
        println!(
            "Output file path: {}",
            output_file_path.to_str().unwrap_or("Invalid path")
        );
        if let Some(parent) = output_file_path.parent() {
            println!(
                "Creating parent directory: {}",
                parent.to_str().unwrap_or("Invalid path")
            );
            std::fs::create_dir_all(parent)?;
        }
        let mut output_file = File::create(&output_file_path)?;

        // copy the decompressed data to the output file
        std::io::copy(&mut zstd_reader, &mut output_file)?;

        // create hasher and buffer for calculating checksum
        let mut hasher = Sha1::new();
        let mut buffer = Vec::new();
        println!("Calculating checksum for file: {}", archive_file_name);
        let mut output_file = File::open(&output_file_path)?;
        output_file.seek(SeekFrom::Start(0))?;
        output_file.read_to_end(&mut buffer)?;
        hasher.update(&buffer);
        let calculated_checksum = hasher.finalize();
        if format!("{:x}", calculated_checksum)
            != *filename_checksum_mapping.get(&archive_file_name).unwrap()
        {
            return Err(format!(
                "Checksum verification failed for file: {}",
                archive_file_name
            )
            .into());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_export_files() {
        // Create a temporary directory for input and output
        let temp_dir = tempdir().unwrap();
        let input_dir = temp_dir.path().join("input");
        let output_dir = temp_dir.path().join("output");
        fs::create_dir_all(&input_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();

        // Create a sample compressed file
        let file_name = "test_file";
        let test_file_content = "Hello, world!";
        let test_file_content_sha1 = "943a702d06f34599aee1f8da8ef9f7296031d699";
        let compressed_file_path = input_dir.join(format!("{}.zst", file_name));
        let mut encoder =
            zstd::Encoder::new(File::create(&compressed_file_path).unwrap(), 0).unwrap();
        write!(encoder, "{}", test_file_content).unwrap();
        encoder.finish().unwrap();

        // Prepare file mappings
        let mut output_file_name_mapping = HashMap::new();
        output_file_name_mapping.insert(file_name.to_string(), "output_file".to_string());
        let mut filename_checksum_mapping = HashMap::new();
        filename_checksum_mapping.insert(file_name.to_string(), test_file_content_sha1.to_string());

        export_files(
            &input_dir,
            &output_dir,
            output_file_name_mapping,
            filename_checksum_mapping,
            ExportType::IndividualFilesWithoutCompression,
        )
        .unwrap();

        let output_file_path = output_dir.join("output_file");
        assert!(output_file_path.exists());
        let content = fs::read_to_string(output_file_path).unwrap();
        assert_eq!(content, "Hello, world!");
    }
}
