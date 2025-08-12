use std::{
    collections::HashMap,
    fmt::Display,
    path::{Path, PathBuf},
};

use core_types::{ImportedFile, ReadFile, Sha1Checksum};
use database::models::FileInfo;
use utils::file_util;

#[derive(Debug)]
pub struct PickedFile {
    pub path: PathBuf,
    pub content: HashMap<Sha1Checksum, PickedFileContent>,
}

#[derive(Debug)]
pub struct PickedFileContent {
    pub file_info: ReadFile,
    pub is_selected: bool,
    pub is_new: bool,
    pub imported_file: Option<ImportedFile>,
}

#[derive(Debug)]
pub struct FileImporter {
    current_picked_files: Vec<PickedFile>,
}

impl Display for FileImporter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.current_picked_files.is_empty() {
            write!(f, "No file currently picked")
        } else {
            write!(
                f,
                "Current picked files: {}",
                self.current_picked_files
                    .iter()
                    .map(|f| f.path.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }
}

impl FileImporter {
    pub fn new() -> Self {
        Self {
            current_picked_files: vec![],
        }
    }
    pub fn get_current_picked_files(&self) -> &Vec<PickedFile> {
        self.current_picked_files.as_ref()
    }
    /*pub fn get_current_picked_file_content(&self) -> &HashMap<Sha1Checksum, ReadFile> {
        &self.current_picked_files_content
    }*/

    pub fn get_selected_files_from_current_picked_files_that_are_new(&self) -> Vec<ReadFile> {
        self.current_picked_files
            .iter()
            .flat_map(|file| {
                file.content
                    .iter()
                    .filter_map(|(_sha1_checksum, picked_content)| {
                        if picked_content.is_selected && picked_content.is_new {
                            Some(picked_content.file_info.clone())
                        } else {
                            None
                        }
                    })
            })
            .collect()
    }

    pub fn is_selected_files(&self) -> bool {
        !self.current_picked_files.is_empty()
            && self
                .current_picked_files
                .iter()
                .flat_map(|f| f.content.values())
                .any(|c| c.is_selected)
    }

    pub fn set_current_picked_file(&mut self, file: PickedFile) {
        self.current_picked_files.push(file);
    }

    /*pub fn get_current_picked_file_name(&self) -> Vec<String> {
        self.current_picked_files
            .iter()
            .map(|f| f.file_name())
            .and_then(|f| f.to_string_lossy().to_string())
    }*/

    pub fn get_file_set_name(&self, path: &PathBuf) -> Option<String> {
        path.file_stem()
            .map(|stem| stem.to_string_lossy().to_string())
    }

    pub fn get_file_set_file_name(&self, path: &PathBuf) -> Option<String> {
        path.file_name()
            .map(|name| name.to_string_lossy().to_string())
    }

    /*pub fn add_current_picked_file_content(&mut self, content: HashMap<Sha1Checksum, ReadFile>) {
        println!("Setting selected: {:?}", content.keys());
        self.selected_files_from_current_picked_files
            .extend(content.keys());
        self.current_picked_files_content.extend(content);
    }*/

    /*pub fn add_existing_files(&mut self, files: Vec<FileInfo>) {
        for file in files {
            let checksum = file
                .sha1_checksum
                .clone()
                .try_into()
                .expect("Invalid checksum length");
            let original_file_name = self
                .current_picked_files_content
                .get(&checksum)
                .and_then(|read_file| read_file.file_name.clone().into())
                .expect("File name not found in current picked file content");
            self.existing_files.insert(
                checksum,
                ImportedFile {
                    original_file_name,
                    archive_file_name: file.archive_file_name.clone(),
                    sha1_checksum: checksum,
                    file_size: file.file_size,
                },
            );
        }
    }*/
    pub fn set_imported_files(&mut self, files: HashMap<Sha1Checksum, ImportedFile>) {
        for (sha1_checksum, imported_file) in files.iter() {
            let file = self
                .current_picked_files
                .iter_mut()
                .find(|f| f.content.contains_key(sha1_checksum));

            if let Some(file) = file {
                if let Some(picked_content) = file.content.get_mut(sha1_checksum) {
                    picked_content.imported_file = Some(imported_file.clone());
                }
            }
        }
    }

    /*pub fn clear(&mut self) {
        self.current_picked_files = vec![];
    }*/

    pub fn get_files_selected_for_file_set(&self) -> Vec<ImportedFile> {
        self.current_picked_files
            .iter()
            .flat_map(|file| {
                file.content
                    .iter()
                    .filter_map(|(_sha1_checksum, picked_content)| {
                        if picked_content.is_selected {
                            picked_content.imported_file.clone()
                        } else {
                            None
                        }
                    })
            })
            .collect::<Vec<_>>()
    }

    /*pub fn is_file_selected(&self, sha1_checksum: &Sha1Checksum) -> bool {
        self.selected_files_from_current_picked_files
            .contains(sha1_checksum)
    }*/

    pub fn deselect_file(&mut self, sha1_checksum: &Sha1Checksum) {
        let file = self
            .current_picked_files
            .iter_mut()
            .find(|f| f.content.contains_key(sha1_checksum));

        if let Some(file) = file {
            if let Some(picked_content) = file.content.get_mut(sha1_checksum) {
                picked_content.is_selected = false;
                println!("Deselecting file: {}", picked_content.file_info.file_name);
            }
        }
    }

    pub fn select_file(&mut self, sha1_checksum: &Sha1Checksum) {
        let file = self
            .current_picked_files
            .iter_mut()
            .find(|f| f.content.contains_key(sha1_checksum));

        if let Some(file) = file {
            if let Some(picked_content) = file.content.get_mut(sha1_checksum) {
                picked_content.is_selected = true;
                println!("Selecting file: {}", picked_content.file_info.file_name);
            }
        }
    }

    /*pub fn toggle_file_selection(&mut self, sha1_checksum: Sha1Checksum) {
        if self.is_file_selected(&sha1_checksum) {
            self.deselect_file(&sha1_checksum);
        } else {
            self.select_file(&sha1_checksum);
        }
    }*/
    pub fn is_zip_file(&self, path: &Path) -> bool {
        file_util::is_zip_file(path).unwrap_or(false)
    }

    pub fn add_new_picked_file(
        &mut self,
        path: &Path,
        content: &HashMap<Sha1Checksum, ReadFile>,
        existing_files_file_info: &[FileInfo],
    ) {
        let picked_content = content
            .iter()
            .map(|(sha1, file_info)| {
                let existing_file = existing_files_file_info
                    .iter()
                    .find(|f| f.sha1_checksum == *sha1);
                let picked = PickedFileContent {
                    file_info: file_info.clone(),
                    is_selected: true,
                    is_new: existing_file.is_none(),
                    imported_file: existing_file.map(|f| ImportedFile {
                        original_file_name: file_info.file_name.clone(),
                        archive_file_name: f.archive_file_name.clone(),
                        sha1_checksum: *sha1,
                        file_size: f.file_size,
                    }),
                };

                (*sha1, picked)
            })
            .collect::<HashMap<_, _>>();

        let current_picked_file = PickedFile {
            path: PathBuf::from(path),
            content: picked_content,
        };

        self.current_picked_files.push(current_picked_file);
    }
}
