use std::{fs::File, path::Path};

use zip::write::FileOptions;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExportType {
    CombinedZipArhive,
    IndividualZipFiles,
    IndividualFilesWithoutCompression,
}

pub fn output_files_individually_with_zip(
    output_dir: &Path,
    file_names: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    for file_name in file_names {
        // TODO: this part is identical to output_zip_compressed in file_import crate
        // => create a common library
        let output_path = output_dir.join(&file_name).with_extension("zip");
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = File::create(output_path)?;
        let mut zip_writer = zip::ZipWriter::new(file);
        let file_options: FileOptions<'_, ()> = FileOptions::default();
        zip_writer.start_file(file_name, file_options)?;
        zip_writer.finish()?;
    }
    Ok(())
}

pub fn output_files_combined_zip(
    output_dir: &Path,
    file_names: Vec<String>,
    container_name: String,
) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: this could be also in common library => compression
    let output_path = output_dir.join(container_name).with_extension("zip");
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = File::create(output_path)?;
    let mut zip_writer = zip::ZipWriter::new(file);
    let file_options: FileOptions<'_, ()> = FileOptions::default();
    for file_name in file_names {
        zip_writer.start_file(file_name.clone(), file_options)?;
    }
    zip_writer.finish()?;
    Ok(())
}
