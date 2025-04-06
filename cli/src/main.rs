use std::{collections::HashMap, path::Path};

use clap::Parser;
use file_compress::{read_zip_file, CompressionMethod};
use file_export::{export_files, ExportType};

#[derive(Parser, Debug)]
struct Cli {
    /// Input file (zip archive)
    input_file: String,

    /// Output directory, where files are outputted individually using selected compression method
    output_directory: String,

    /// Compression method (zip, zstd or none)
    #[arg(value_enum)]
    compression_method: CompressionMethod,
}

fn main() {
    let args = Cli::parse();
    match read_zip_file(
        args.input_file.as_str(),
        args.output_directory.as_str(),
        args.compression_method,
    ) {
        Ok(hash_map) => {
            // let's try exporting the files...
            let input_path = Path::new(&args.output_directory);
            let output_path = Path::new(&args.output_directory).join("export");
            let output_filename_mapping = hash_map
                .iter()
                .map(|(k, v)| (k.clone(), format!("{}-{}-exported", k.clone(), v.clone())))
                .collect::<HashMap<_, _>>();
            export_files(
                input_path,
                &output_path,
                output_filename_mapping,
                hash_map.clone(),
                ExportType::IndividualFilesWithoutCompression,
            )
            .expect("Failed to export files");
        }
        Err(e) => {
            eprintln!("Error reading zip file: {}", e);
        }
    }
}
