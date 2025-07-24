use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use core_types::{ImportedFile, ReadFile, Sha1Checksum};
use database::models::FileInfo;
use utils::file_util;

#[derive(Debug)]
pub struct FileImporter {
    current_picked_file: Option<PathBuf>,
    current_picked_file_content: HashMap<Sha1Checksum, ReadFile>,
    existing_files: HashMap<Sha1Checksum, ImportedFile>,
    selected_files_from_current_picked_file: HashSet<Sha1Checksum>,
    imported_files: HashMap<Sha1Checksum, ImportedFile>,
}

impl FileImporter {
    pub fn new() -> Self {
        Self {
            current_picked_file: None,
            current_picked_file_content: HashMap::new(),
            existing_files: HashMap::new(),
            selected_files_from_current_picked_file: HashSet::new(),
            imported_files: HashMap::new(),
        }
    }
    pub fn get_current_picked_file(&self) -> Option<&PathBuf> {
        self.current_picked_file.as_ref()
    }
    pub fn get_current_picked_file_content(&self) -> &HashMap<Sha1Checksum, ReadFile> {
        &self.current_picked_file_content
    }
    pub fn get_selected_files_from_current_picked_file_that_are_new(&self) -> Vec<ReadFile> {
        let existing_files_checksums: HashSet<Sha1Checksum> =
            self.existing_files.keys().cloned().collect();
        let checksums_for_new_files: HashSet<Sha1Checksum> = self
            .selected_files_from_current_picked_file
            .difference(&existing_files_checksums)
            .cloned()
            .collect();

        self.current_picked_file_content
            .iter()
            .filter(|(sha1_checksum, _)| checksums_for_new_files.contains(*sha1_checksum))
            .map(|(_, read_file)| read_file.clone())
            .collect()
    }
    pub fn is_selected_files(&self) -> bool {
        self.current_picked_file.is_some()
            && !self.selected_files_from_current_picked_file.is_empty()
    }

    pub fn set_current_picked_file(&mut self, file: PathBuf) {
        self.clear();
        self.current_picked_file = Some(file);
    }
    pub fn set_current_picked_file_content(&mut self, content: HashMap<Sha1Checksum, ReadFile>) {
        self.selected_files_from_current_picked_file
            .extend(content.keys());
        self.current_picked_file_content = content;
    }
    pub fn set_existing_files(&mut self, files: Vec<FileInfo>) {
        let mut file_map: HashMap<Sha1Checksum, ImportedFile> = HashMap::new();
        for file in files {
            let checksum = file
                .sha1_checksum
                .clone()
                .try_into()
                .expect("Invalid checksum length");
            let original_file_name = self
                .current_picked_file_content
                .get(&checksum)
                .and_then(|read_file| read_file.file_name.clone().into())
                .expect("File name not found in current picked file content");
            file_map.insert(
                checksum,
                ImportedFile {
                    original_file_name,
                    archive_file_name: file.archive_file_name.clone(),
                    sha1_checksum: checksum,
                    file_size: file.file_size,
                },
            );
        }
        self.existing_files = file_map;
    }
    pub fn set_imported_files(&mut self, files: HashMap<Sha1Checksum, ImportedFile>) {
        self.imported_files = files;
    }
    pub fn clear(&mut self) {
        self.current_picked_file = None;
        self.current_picked_file_content.clear();
        self.existing_files.clear();
        self.selected_files_from_current_picked_file.clear();
        self.imported_files.clear();
    }

    pub fn is_file_selected(&self, sha1_checksum: &Sha1Checksum) -> bool {
        self.selected_files_from_current_picked_file
            .contains(sha1_checksum)
    }

    pub fn deselect_file(&mut self, sha1_checksum: &Sha1Checksum) {
        self.selected_files_from_current_picked_file
            .remove(sha1_checksum);
    }

    pub fn select_file(&mut self, sha1_checksum: &Sha1Checksum) {
        self.selected_files_from_current_picked_file
            .insert(*sha1_checksum);
    }

    pub fn toggle_file_selection(&mut self, sha1_checksum: Sha1Checksum) {
        if self.is_file_selected(&sha1_checksum) {
            self.deselect_file(&sha1_checksum);
        } else {
            self.select_file(&sha1_checksum);
        }
    }
    pub fn is_zip_file(&self) -> bool {
        if let Some(path) = self.get_current_picked_file() {
            return file_util::is_zip_file(path.as_path()).unwrap_or(false);
        }
        false
    }
}
