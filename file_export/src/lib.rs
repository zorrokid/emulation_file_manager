pub mod file_outputter;
use std::{collections::HashMap, fs::File, io::Read, path::Path};

pub use file_outputter::ExportType;
use file_outputter::{output_files_combined_zip, output_files_individually_with_zip};
use sha1::{Digest, Sha1};

pub fn export_files(
    file_path: &Path,
    output_dir: &Path,
    // key is the archive file name, value is the output file name
    output_file_name_mapping: HashMap<String, String>,
    // key is the archive file name, value is the checksum
    filename_checksum_mapping: HashMap<String, String>,
    // TODO
    export_type: ExportType,
    container_name: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut output_file_names: Vec<String> = Vec::new();
    for (archive_file_name, output_file_name) in output_file_name_mapping {
        output_file_names.push(output_file_name.clone());
        // souce files are in zstd format
        let file_path = file_path.join(&archive_file_name).with_extension("zst");
        let output_file_path = output_dir.join(&output_file_name);
        decompress_zstd_file(&file_path, &output_file_path)?;
        if let Err(e) = check_file_checksum(
            &output_file_path,
            filename_checksum_mapping.get(&archive_file_name).unwrap(),
        ) {
            return Err(format!(
                "Checksum verification failed for file: {}. Error: {}",
                archive_file_name, e
            )
            .into());
        }
    }
    if export_type == ExportType::CombinedZipArhive {
        if let Some(container_name) = container_name {
            output_files_combined_zip(output_dir, output_file_names, container_name)?;
        } else {
            return Err("Container name is required for combined zip archive".into());
        }
    } else if export_type == ExportType::IndividualZipFiles {
        output_files_individually_with_zip(output_dir, output_file_names)?;
    }
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

fn check_file_checksum(
    file_path: &Path,
    expected_checksum: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let mut hasher = Sha1::new();
    let mut file = File::open(file_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    hasher.update(&buffer);
    let calculated_checksum = hasher.finalize();
    Ok(format!("{:x}", calculated_checksum) == expected_checksum)
}
