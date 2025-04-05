use std::{
    fs::{create_dir_all, File},
    io::Write,
    path::Path,
    str::FromStr,
};
use zstd::Encoder;

use zip::write::FileOptions;

#[derive(Clone, Debug)]
pub enum CompressionMethod {
    Zip,
    Zstd,
    None,
}

impl FromStr for CompressionMethod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "zstd" => Ok(CompressionMethod::Zstd),
            "zip" => Ok(CompressionMethod::Zip),
            "none" => Ok(CompressionMethod::None),
            _ => Err(format!("Invalid compression method: {}", s)),
        }
    }
}

pub trait FileOutputter {
    fn output(
        &self,
        output_dir: &Path,
        buffer: &[u8],
        file_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>>;
}

impl FileOutputter for CompressionMethod {
    fn output(
        &self,
        output_path: &Path,
        buffer: &[u8],
        file_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            CompressionMethod::Zip => output_zip_compressed(output_path, buffer, file_name),
            CompressionMethod::Zstd => output_zstd_compressed(output_path, buffer, file_name),
            CompressionMethod::None => output_without_compression(output_path, buffer, file_name),
        }
    }
}

fn output_without_compression(
    output_dir: &Path,
    buffer: &[u8],
    file_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = output_dir.parent() {
        create_dir_all(parent)?;
    }
    let mut output_file = File::create(output_dir.join(file_name))?;
    output_file.write_all(buffer)?;
    Ok(())
}

fn output_zstd_compressed(
    output_dir: &Path,
    buffer: &[u8],
    file_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let zstd_file_path = output_dir.join(file_name).with_extension("zst");
    if let Some(parent) = zstd_file_path.parent() {
        create_dir_all(parent)?;
    }
    let zstd_file = File::create(zstd_file_path)?;
    let mut encoder = Encoder::new(zstd_file, 0)?;
    encoder.write_all(buffer)?;
    encoder.finish()?;
    Ok(())
}

fn output_zip_compressed(
    output_dir: &Path,
    buffer: &[u8],
    file_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let zip_file_path = output_dir.join(file_name).with_extension("zip");
    if let Some(parent) = zip_file_path.parent() {
        create_dir_all(parent)?;
    }
    let zip_file = File::create(zip_file_path)?;
    let mut zip_writer = zip::ZipWriter::new(zip_file);
    let file_options: FileOptions<'_, ()> = FileOptions::default();
    zip_writer.start_file(file_name, file_options)?;
    zip_writer.write_all(buffer)?;
    zip_writer.finish()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn test_output_without_compression() {
        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path();
        let buffer = b"Hello, world!";
        let file_name = "test";

        let method = CompressionMethod::None;

        method
            .output(output_path, buffer, file_name)
            .expect("Failed to write file");

        let output_data = fs::read(output_path.join(file_name)).expect("Failed to read file");
        assert_eq!(output_data, buffer);
    }

    #[test]
    fn test_output_zstd_compressed() {
        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path();
        let buffer = b"Hello, world!";
        let method = CompressionMethod::Zstd;
        let file_name = "test";

        method
            .output(output_path, buffer, file_name)
            .expect("Failed to write file");

        let output_data =
            fs::read(output_path.join("test").with_extension("zst")).expect("Failed to read file");
        assert!(!output_data.is_empty());
    }

    #[test]
    fn test_output_zip_compressed() {
        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path();
        let buffer = b"Hello, world!";
        let method = CompressionMethod::Zip;
        let file_name = "test";

        method
            .output(output_path, buffer, file_name)
            .expect("Failed to write file");

        let output_data = fs::read(output_path.join(file_name).with_extension("zip"))
            .expect("Failed to read file");
        assert!(!output_data.is_empty());
    }
}
