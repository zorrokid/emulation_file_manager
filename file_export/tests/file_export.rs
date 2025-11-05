use std::io::{Read, Write};
use std::{
    collections::HashMap,
    fs::{self, File},
};

use file_export::{export_files, export_files_zipped, FileSetExportModel, OutputFile};
use tempfile::tempdir;
use utils::test_utils::get_sha1_and_size;

const TEST_FILE_CONTENT: &str = "Hello, world!";
const TEST_FILE_NAME: &str = "test_file";
const TEST_OUTPUT_FILE_NAME: &str = "output_file";
const TEST_INPUT_FOLDER: &str = "input";
const TEST_OUTPUT_FOLDER: &str = "output";

#[test]
fn test_export_files() {
    // Create a temporary directory for input and output
    let temp_dir = tempdir().unwrap();
    let input_dir = temp_dir.path().join(TEST_INPUT_FOLDER);
    let output_dir = temp_dir.path().join(TEST_OUTPUT_FOLDER);
    fs::create_dir_all(&input_dir).unwrap();
    fs::create_dir_all(&output_dir).unwrap();

    create_sample_compressed_file(&input_dir, TEST_FILE_NAME);
    let output_mapping = prepare_file_mappings();
    let output_file_path = output_dir.join(TEST_OUTPUT_FILE_NAME);

    let export_model = FileSetExportModel {
        output_mapping,
        source_file_path: input_dir,
        extract_files: false,
        exported_zip_file_name: TEST_OUTPUT_FILE_NAME.to_string(),
        output_dir,
    };

    export_files(&export_model).unwrap();

    assert!(output_file_path.exists());
    let content = fs::read_to_string(output_file_path).unwrap();
    assert_eq!(content, TEST_FILE_CONTENT);
}

#[test]
fn test_export_files_zipped() {
    // Create a temporary directory for input and output
    let temp_dir = tempdir().unwrap();
    let input_dir = temp_dir.path().join(TEST_INPUT_FOLDER);
    let output_dir = temp_dir.path().join(TEST_OUTPUT_FOLDER);
    fs::create_dir_all(&input_dir).unwrap();
    fs::create_dir_all(&output_dir).unwrap();

    create_sample_compressed_file(&input_dir, TEST_FILE_NAME);

    let output_mapping = prepare_file_mappings();
    let zip_file_path = output_dir.join("exported_files.zip");

    let export_model = FileSetExportModel {
        output_mapping,
        source_file_path: input_dir.clone(),
        extract_files: true,
        exported_zip_file_name: "exported_files.zip".to_string(),
        output_dir,
    };

    export_files_zipped(&export_model).unwrap();

    // Check if the zip file was created
    assert!(zip_file_path.exists());
    // Check if the zip file contains the expected file
    let mut zip_reader = zip::ZipArchive::new(File::open(zip_file_path).unwrap()).unwrap();
    let mut file = zip_reader.by_name(TEST_OUTPUT_FILE_NAME).unwrap();
    let mut zip_content = String::new();
    file.read_to_string(&mut zip_content).unwrap();
    assert_eq!(zip_content, TEST_FILE_CONTENT);
    // Note: The temporary directory will be automatically deleted when it goes out of scope
}

fn create_sample_compressed_file(
    input_dir: &std::path::Path,
    file_name: &str,
) -> std::path::PathBuf {
    let compressed_file_path = input_dir.join(format!("{}.zst", file_name));
    let mut encoder = zstd::Encoder::new(File::create(&compressed_file_path).unwrap(), 0).unwrap();
    write!(encoder, "{}", TEST_FILE_CONTENT).unwrap();
    encoder.finish().unwrap();
    compressed_file_path
}

fn prepare_file_mappings() -> HashMap<String, OutputFile> {
    let mut output_mapping = HashMap::new();
    let (checksum, _) = get_sha1_and_size(TEST_FILE_CONTENT);
    output_mapping.insert(
        TEST_FILE_NAME.to_string(),
        OutputFile {
            output_file_name: TEST_OUTPUT_FILE_NAME.to_string(),
            checksum,
        },
    );
    output_mapping
}
