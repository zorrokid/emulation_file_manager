use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use core_types::Sha1Checksum;
use sha1::{Digest, Sha1};
use zip::write::FileOptions;

#[derive(Debug, Clone)]
pub enum FileExportError {
    ZipError(String),
    FileIoError(String),
}

impl std::fmt::Display for FileExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileExportError::ZipError(err) => write!(f, "Zip error: {}", err),
            FileExportError::FileIoError(err) => write!(f, "File IO error: {}", err),
        }
    }
}

/// Exports files from a given zstd archive directory to an output directory decompressed and with given output file name
/// mapping. Files are also checked for their SHA1 checksums provided in filename checksum map.
///
/// # Arguments
///
/// * `file_path` - The path to the directory containing the archived collection files.
/// * `output_dir` - The directory where the files will be exported.
/// * `output_file_name_mapping` - A hash map where the key is the archive file name and the value is the output file name.
/// * `filename_checksum_mapping` - A hash map where the key is the archive file name and the value is the SHA1 checksum.
///
/// # Returns
///
/// A `Result` indicating success or failure of the operation.
///
pub fn export_files(
    file_path: PathBuf,
    output_dir: PathBuf,
    // key is the archive file name, value is the output file name
    output_file_name_mapping: HashMap<String, String>,
    // key is the archive file name, value is the checksum
    filename_checksum_mapping: HashMap<String, Sha1Checksum>,
) -> Result<(), FileExportError> {
    let mut output_file_names: Vec<String> = Vec::new();
    for (archive_file_name, output_file_name) in output_file_name_mapping {
        output_file_names.push(output_file_name.clone());
        // souce files are in zstd format
        let file_path = file_path.join(&archive_file_name).with_extension("zst");
        let output_file_path = output_dir.join(&output_file_name);
        decompress_zstd_file(&file_path, &output_file_path).map_err(|err| {
            FileExportError::ZipError(format!("Failed decompressing zstd file: {}", err))
        })?;

        check_file_checksum(
            &output_file_path,
            filename_checksum_mapping.get(&archive_file_name).unwrap(),
        )
        .map_err(|e| {
            FileExportError::FileIoError(format!(
                "Checksum verification failed for file: {}. Error: {}",
                archive_file_name, e
            ))
        })?;
    }
    Ok(())
}

/// Exports files from a given zstd archive directory to an output directory compressed to a zip
/// archive containing the files to be exported with given output file name mapping. Files are also checked for their SHA1 checksums provided in filename checksum map.
///
/// # Arguments
///
/// * `file_path` - The path to the directory containing the archived collection files.
/// * `output_dir` - The directory where the files will be exported.
/// * `output_file_name_mapping` - A hash map where the key is the archive file name and the value is the output file name.
/// * `filename_checksum_mapping` - A hash map where the key is the archive file name and the value is the SHA1 checksum.
/// * `container_name` - The name of the zip file to be created.
///
/// # Returns
///
/// A `Result` indicating success or failure of the operation.
pub fn export_files_zipped(
    file_path: PathBuf,
    output_dir: PathBuf,
    // key is the archive file name, value is the output file name
    output_file_name_mapping: HashMap<String, String>,
    // key is the archive file name, value is the checksum
    filename_checksum_mapping: HashMap<String, Sha1Checksum>,
    container_name: String,
) -> Result<(), FileExportError> {
    let zip_path = output_dir.join(container_name);
    let zip_file = File::create(zip_path)
        .map_err(|e| FileExportError::ZipError(format!("Failed creating zip file {}", e)))?;
    let mut zip_writer = zip::ZipWriter::new(zip_file);
    let file_options: FileOptions<'_, ()> = FileOptions::default();

    for (archive_file_name, output_file_name) in output_file_name_mapping {
        let file_path = file_path.join(&archive_file_name).with_extension("zst");

        // Add to combined zip archive

        zip_writer
            .start_file(&output_file_name, file_options)
            .map_err(|e| {
                FileExportError::ZipError(format!("Failed starting the zip file: {}", e))
            })?;
        decompress_zstd_to_writer(&file_path, &mut zip_writer).map_err(|e| {
            FileExportError::ZipError(format!("Failed decompressing zstd to writer: {}", e))
        })?;

        // Verify checksum
        if let Err(e) = check_file_checksum(
            &file_path,
            filename_checksum_mapping.get(&archive_file_name).unwrap(),
        ) {
            return Err(FileExportError::FileIoError(format!(
                "Checksum verification failed for file: {}. Error: {}",
                archive_file_name, e
            )));
        }
    }

    zip_writer
        .finish()
        .map_err(|e| FileExportError::ZipError(format!("Failed finishing zip writer: {}", e)))?;

    Ok(())
}

fn decompress_zstd_file(
    input_path: &Path,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(input_path)?;
    let mut zstd_reader = zstd::Decoder::new(file)?;
    if let Some(parent) = output_path.parent() {
        println!(
            "Creating parent directory: {}",
            parent.to_str().unwrap_or("Invalid path")
        );
        std::fs::create_dir_all(parent)?;
    }
    let mut output_file = File::create(output_path)?;
    std::io::copy(&mut zstd_reader, &mut output_file)?;
    Ok(())
}

fn decompress_zstd_to_writer(
    input_path: &Path,
    output_writer: &mut dyn std::io::Write,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(input_path)?;
    let mut zstd_reader = zstd::Decoder::new(file)?;
    std::io::copy(&mut zstd_reader, output_writer)?;
    Ok(())
}

fn check_file_checksum(
    file_path: &Path,
    expected_checksum: &Sha1Checksum,
) -> Result<bool, Box<dyn std::error::Error>> {
    let mut hasher = Sha1::new();
    let mut file = File::open(file_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    hasher.update(&buffer);
    let calculated_checksum = hasher.finalize();
    let calculated_checksum = calculated_checksum.as_slice();
    Ok(calculated_checksum == *expected_checksum)
}
