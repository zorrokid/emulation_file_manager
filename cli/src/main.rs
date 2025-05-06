use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use async_std::task;
use clap::Parser;
use core_types::ImportedFile;
use database::{get_db_pool, models::FileType, repository_manager::RepositoryManager};
use emulator_runner::run_with_emulator;
use file_export::{export_files, export_files_zipped};
use file_import::{import_files_from_zip, read_zip_contents, CompressionMethod};

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
        let repository_manager = RepositoryManager::new(Arc::clone(&db_pool));

        let file_path = PathBuf::from(args.input_file);
        let file_name = file_path
            .file_name()
            .expect("Failed to get file name")
            .to_str()
            .expect("Failed to convert file name to string")
            .to_string();
        let output_directory = PathBuf::from(args.output_directory);
        let file_name_filter =
            read_zip_contents(file_path.clone()).expect("Failed to read zip contents");
        match import_files_from_zip(
            file_path.clone(),
            output_directory.clone(),
            args.compression_method,
            file_name_filter,
        ) {
            Ok(hash_map) => {
                // let't try inserting file set to database
                let file_type = FileType::Rom;
                let picked_files = hash_map
                    .values()
                    .map(|v| ImportedFile {
                        file_name: v.file_name.clone(),
                        sha1_checksum: v.sha1_checksum,
                        file_size: v.file_size,
                    })
                    .collect::<Vec<ImportedFile>>();

                let file_set_id = repository_manager
                    .get_file_set_repository()
                    .add_file_set(file_name, file_type, picked_files)
                    .await
                    .expect("Failed to insert file set to database");

                // let's try fetching file set from database
                let file_set = repository_manager
                    .get_file_set_repository()
                    .get_file_sets(vec![file_set_id])
                    .await
                    .expect("Failed to fetch file set from database");

                println!("File set: {:?}", file_set);

                // let's try fetching file info from database
                let file_info = repository_manager
                    .get_file_info_repository()
                    .get_file_infos_by_file_set(file_set_id)
                    .await
                    .expect("Failed to fetch file info from database");

                println!("File info: {:?}", file_info);

                // let's try exporting the files...
                let input_path = Path::new(&output_directory);
                let output_path = Path::new(&output_directory).join("export");
                let output_filename_mapping = hash_map
                    .values()
                    .map(|v| {
                        (
                            v.file_name.clone(),
                            format!("{}-exported", v.file_name.clone()),
                        )
                    })
                    .collect::<HashMap<_, _>>();
                let filename_checksum_mapping = hash_map
                    .values()
                    .map(|v| (v.file_name.clone(), v.sha1_checksum))
                    .collect::<HashMap<_, _>>();
                export_files(
                    input_path,
                    &output_path,
                    output_filename_mapping.clone(),
                    filename_checksum_mapping.clone(),
                )
                .expect("Failed to export files");
                export_files_zipped(
                    input_path,
                    &output_path,
                    output_filename_mapping,
                    filename_checksum_mapping,
                    "exported_files.zip".to_string(),
                )
                .expect("Failed to export files");

                // let's try running an emulator
                let executable = "x64".to_string();
                let arguments = "".to_string();
                let file_names = vec!["test.d64".to_string()];
                let selected_file_name = "test.d64".to_string();
                let file_path = std::env::current_dir()
                    .expect("Failed to get current directory")
                    .join("test_files")
                    .to_path_buf();

                run_with_emulator(
                    executable,
                    arguments,
                    file_names,
                    selected_file_name,
                    file_path,
                )
                .await
                .expect("Failed to run emulator");
            }
            Err(e) => {
                eprintln!("Error reading zip file: {}", e);
            }
        }
        Ok(())
    })
}
