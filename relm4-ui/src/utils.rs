use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use core_types::{FileType, Sha1Checksum};
use file_export::FileSetExportModel;
use service::view_models::FileSetViewModel;

pub fn resolve_file_type_path(root_path: &Path, file_type: &FileType) -> PathBuf {
    let mut path = PathBuf::from(root_path);
    path.push(file_type.dir_name());
    path
}

pub fn prepare_fileset_for_export(
    file_set: &FileSetViewModel,
    collection_root_dir: &Path,
    temp_dir: &Path,
    extract_files: bool,
) -> FileSetExportModel {
    let source_file_path = resolve_file_type_path(collection_root_dir, &file_set.file_type.into());
    let output_file_name_mapping = file_set
        .files
        .iter()
        .map(|f| (f.archive_file_name.clone(), f.file_name.clone()))
        .collect::<HashMap<_, _>>();

    let filename_checksum_mapping = file_set
        .files
        .iter()
        .map(|f| {
            let checksum: Sha1Checksum = f
                .sha1_checksum
                .clone()
                .try_into()
                .expect("Failed to convert to Sha1Checksum");
            (f.archive_file_name.clone(), checksum)
        })
        .collect::<HashMap<String, Sha1Checksum>>();

    let exported_zip_file_name = file_set.file_set_name.clone();
    FileSetExportModel {
        output_file_name_mapping,
        filename_checksum_mapping,
        source_file_path,
        output_dir: temp_dir.to_path_buf(),
        extract_files,
        exported_zip_file_name,
    }
}
