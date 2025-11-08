use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use file_export::{FileSetExportModel, OutputFile};
use image::GenericImageView;

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

pub trait ThumbnailOps: Send + Sync {
    fn prepare_thumbnails(
        &self,
        thumbnails_dir: &Path,
        output_dir: &Path,
        output_mapping: &HashMap<String, OutputFile>,
    ) -> Result<ThumbnailPathMap, ThumbnailsError>;
}

pub struct ThumbnailGenerator;

impl ThumbnailOps for ThumbnailGenerator {
    fn prepare_thumbnails(
        &self,
        thumbnails_dir: &Path,
        output_dir: &Path,
        output_mapping: &HashMap<String, OutputFile>,
    ) -> Result<ThumbnailPathMap, ThumbnailsError> {
        prepare_thumbnails_from_output_dir(thumbnails_dir, output_dir, output_mapping)
    }
}

#[derive(Default)]
pub struct ThumbnailGeneratorMock;

impl ThumbnailOps for ThumbnailGeneratorMock {
    fn prepare_thumbnails(
        &self,
        thumbnails_dir: &Path,
        _output_dir: &Path,
        output_mapping: &HashMap<String, OutputFile>,
    ) -> Result<ThumbnailPathMap, ThumbnailsError> {
        println!(
            "Mock preparing thumbnails in directory: {}",
            thumbnails_dir.display()
        );
        let mut thumbnail_path_mapp: HashMap<String, PathBuf> = HashMap::new();
        for (archive_file_name, output_file) in output_mapping {
            println!(
                "Mock generating thumbnail for archive file name '{}' at '{}'",
                archive_file_name,
                thumbnails_dir.display()
            );
            let thumbnail_path = thumbnails_dir.join(format!("{}.png", archive_file_name));
            thumbnail_path_mapp.insert(output_file.output_file_name.clone(), thumbnail_path);
        }
        Ok(thumbnail_path_mapp)
    }
}

pub fn prepare_thumbnails(
    export_model: &FileSetExportModel,
    collection_root_dir: &Path,
) -> Result<ThumbnailPathMap, ThumbnailsError> {
    let thumbnails_dir = collection_root_dir.join("thumbnails");
    prepare_thumbnails_from_output_dir(
        thumbnails_dir.as_path(),
        &export_model.output_dir,
        &export_model.output_mapping,
    )
}

pub fn prepare_thumbnails_from_output_dir(
    thumbnails_dir: &Path,
    output_dir: &Path,
    output_mapping: &HashMap<String, OutputFile>,
) -> Result<ThumbnailPathMap, ThumbnailsError> {
    let exported_files_dir = &output_dir;
    let mut thumbnail_path_mapp: HashMap<String, PathBuf> = HashMap::new();
    for (archive_file_name, output_file) in output_mapping {
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
            std::fs::create_dir_all(thumbnails_dir).map_err(|_| {
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

pub fn get_image_size(image_path: &Path) -> Result<(u32, u32), ThumbnailsError> {
    let image = image::open(image_path).map_err(|err| {
        ThumbnailsError::IoError(format!(
            "Failed opening image {} with error: {}",
            image_path.display(),
            &err
        ))
    })?;
    Ok(image.dimensions())
}
