use std::{collections::HashMap, path::Path, sync::Arc};

use async_std::task;
use clap::Parser;
use database::{
    get_db_pool,
    models::{FileType, PickedFileInfo},
    repository_manager::RepositoryManager,
};
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

        let file_name = args.input_file;
        let file_path = Path::new(&file_name);
        let output_directory = args.output_directory;
        let file_name_to_checksum_filter =
            read_zip_contents(file_path).expect("Failed to read zip contents");
        match import_files_from_zip(
            &file_name,
            &output_directory,
            args.compression_method,
            file_name_to_checksum_filter,
        ) {
            Ok(hash_map) => {
                // let't try inserting file set to database
                let file_type = FileType::Rom;
                let picked_files = hash_map
                    .iter()
                    .map(|(k, v)| PickedFileInfo {
                        file_name: k.clone(),
                        file_size: 0, // TODO: set the correct file size
                        sha1_checksum: v.clone(),
                    })
                    .collect();

                let file_set_id = repository_manager
                    .get_file_set_repository()
                    .add_file_set(&file_name, &file_type, &picked_files)
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
