use std::{collections::HashMap, path::Path, sync::Arc};

use async_std::task;
use clap::Parser;
use database::{get_db_pool, repository::file_info_repository::FileInfoRepository};
use file_export::{export_files, export_files_zipped};
use file_import::{read_zip_file, CompressionMethod};

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    task::block_on(async {
        let args = Cli::parse();
        let db_pool = get_db_pool().await.unwrap();

        let _file_info_repository = FileInfoRepository::new(Arc::clone(&db_pool));

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
                    output_filename_mapping.clone(),
                    hash_map.clone(),
                )
                .expect("Failed to export files");
                export_files_zipped(
                    input_path,
                    &output_path,
                    output_filename_mapping,
                    hash_map.clone(),
                    "exported_files.zip".to_string(),
                )
                .expect("Failed to export files");
            }
            Err(e) => {
                eprintln!("Error reading zip file: {}", e);
            }
        }
        Ok(())
    })
}
