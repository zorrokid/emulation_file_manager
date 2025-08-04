use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use file_export::FileSetExportModel;

use std::fmt::{Display, Formatter, Result as FmtResult};

#[derive(Debug, Clone)]
pub enum ThumbnailsError {
    IoError(String),
}

impl Display for ThumbnailsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            ThumbnailsError::IoError(message) => {
                write!(f, "IO error when preparing thubnails: {}", message)
            }
        }
    }
}

pub type ThumbnailPathMap = HashMap<String, PathBuf>;

pub fn prepare_thumbnails(
    export_model: &FileSetExportModel,
    collection_root_dir: &Path,
) -> Result<ThumbnailPathMap, ThumbnailsError> {
    println!(
        "Preparing thumbnails for fileset: {} in directory: {}",
        export_model.exported_zip_file_name,
        collection_root_dir.display()
    );
    let thumbnails_dir = collection_root_dir.join("thumbnails");
    let exported_files_dir = &export_model.output_dir;
    let mut thumbnail_path_mapp: HashMap<String, PathBuf> = HashMap::new();
    for (archive_file_name, output_file) in &export_model.output_mapping {
        let thumbnail_path = thumbnails_dir.join(format!("{}.png", archive_file_name));
        let exported_file_path = exported_files_dir.join(&output_file.output_file_name);

        println!(
            "Generating thumbnail for archive file name '{}' output file name '{}' at '{}'",
            archive_file_name,
            exported_file_path.display(),
            thumbnail_path.display()
        );

        if thumbnail_path.exists() {
            thumbnail_path_mapp.insert(output_file.output_file_name.clone(), thumbnail_path);
        } else {
            let image = image::open(&exported_file_path).map_err(|err| {
                ThumbnailsError::IoError(format!(
                    "Failed opening image {} with error: {}",
                    exported_file_path.display(),
                    &err
                ))
            })?;

            let thumbnail = image.thumbnail(100, 100);
            std::fs::create_dir_all(&thumbnails_dir).map_err(|_| {
                ThumbnailsError::IoError(format!(
                    "Failed creating directory: {}",
                    &thumbnails_dir.display()
                ))
            })?;

            thumbnail.save(&thumbnail_path).map_err(|err| {
                ThumbnailsError::IoError(format!(
                    "Failed saving thumbnail to {} with error: {}",
                    thumbnail_path.display(),
                    &err
                ))
            })?;
            thumbnail_path_mapp.insert(output_file.output_file_name.clone(), thumbnail_path);
        }
    }
    Ok(thumbnail_path_mapp)
}
