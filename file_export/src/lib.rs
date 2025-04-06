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
