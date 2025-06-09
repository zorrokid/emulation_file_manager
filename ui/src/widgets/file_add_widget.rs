use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
};

use core_types::{ImportedFile, ReadFile, Sha1Checksum};
use database::{
    database_error::Error,
    models::{FileInfo, FileType},
    repository_manager::RepositoryManager,
};
use file_import::FileImportError;
use iced::{
    alignment,
    widget::{button, checkbox, column, pick_list, row, scrollable, text, text_input, Column},
    Element, Task,
};
use rfd::FileHandle;
use service::view_models::FileSetListModel;

use crate::{
    defaults::{DEFAULT_LABEL_WIDTH, DEFAULT_PADDING, DEFAULT_PICKER_WIDTH, DEFAULT_SPACING},
    util::file_paths::resolve_file_type_path,
};

pub struct FileImporter {
    current_picked_file: Option<FileHandle>,
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
    pub fn get_current_picked_file(&self) -> Option<&FileHandle> {
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

    pub fn set_current_picked_file(&mut self, file: FileHandle) {
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
}

pub struct FileAddWidget {
    file_name: String,
    selected_file_type: Option<FileType>, // TODO: use core FileType?
    file_importer: FileImporter,
    collection_root_dir: PathBuf,
    repositories: Arc<RepositoryManager>,
}

#[derive(Debug, Clone)]
pub enum FileAddWidgetMessage {
    FileNameUpdated(String),
    Submit,
    StartFileSelection,
    FileTypeSelected(FileType),
    FilePicked(Option<FileHandle>),
    FileContentsRead(Result<HashMap<Sha1Checksum, ReadFile>, FileImportError>),
    FileSelectionToggled(Sha1Checksum),
    FilesImported(Result<HashMap<Sha1Checksum, ImportedFile>, FileImportError>),
    FilesSavedToDatabase(Result<i64, Error>),
    ExistingFilesRead(Result<Vec<FileInfo>, Error>),
    FileSetAdded(FileSetListModel),
    Reset,
}

impl FileAddWidget {
    pub fn new(collection_root_dir: PathBuf, repositories: Arc<RepositoryManager>) -> Self {
        Self {
            file_name: "".to_string(),
            selected_file_type: None,
            collection_root_dir,
            repositories,
            file_importer: FileImporter::new(),
        }
    }

    pub fn update(&mut self, message: FileAddWidgetMessage) -> Task<FileAddWidgetMessage> {
        match message {
            FileAddWidgetMessage::FileTypeSelected(file_type) => {
                self.selected_file_type = Some(file_type);
            }
            // 1. Start file selection by opening a file dialog
            FileAddWidgetMessage::StartFileSelection => {
                if self.selected_file_type.is_none() {
                    return Task::none();
                }
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Choose a file")
                            // TODO: filter supported file types based on selected file type
                            .pick_file()
                            .await
                    },
                    FileAddWidgetMessage::FilePicked,
                );
            }
            // 2. If a file is picked, read its contents and checksums
            FileAddWidgetMessage::FilePicked(file_handle) => {
                if let Some(handle) = file_handle {
                    self.file_name = handle.file_name();
                    let file_path = handle.path().to_path_buf();
                    self.file_importer.set_current_picked_file(handle.clone());

                    return Task::perform(
                        // TODO: create new method in file_import to handle either single file or
                        // zip archive and use that instead
                        async move { file_import::read_zip_contents_with_checksums(file_path) },
                        FileAddWidgetMessage::FileContentsRead,
                    );
                } else {
                    println!("No file selected");
                }
            }
            // 3. When contents have been read, check for existing files in the database
            FileAddWidgetMessage::FileContentsRead(result) => match result {
                Ok(files) => {
                    let file_checksums = files.keys().cloned().collect::<Vec<Sha1Checksum>>();
                    self.file_importer.set_current_picked_file_content(files);
                    let repo = Arc::clone(&self.repositories);

                    return Task::perform(
                        async move {
                            repo.get_file_info_repository()
                                .get_file_infos_by_sha1_checksums(file_checksums)
                                .await
                        },
                        FileAddWidgetMessage::ExistingFilesRead,
                    );
                }
                Err(err) => {
                    eprintln!("Error reading file contents: {}", err);
                }
            },
            // 4. When existing files are read, set them in the file importer
            FileAddWidgetMessage::ExistingFilesRead(result) => match result {
                Ok(existing_files) => {
                    self.file_importer.set_existing_files(existing_files);
                }
                Err(err) => {
                    eprintln!("Error reading existing files: {}", err);
                }
            },
            FileAddWidgetMessage::FileSelectionToggled(sha1_checksum) => {
                self.file_importer.toggle_file_selection(sha1_checksum)
            }
            FileAddWidgetMessage::FileNameUpdated(name) => {
                self.file_name = name;
            }
            // This starts the actual import process
            FileAddWidgetMessage::Submit => {
                if let (Some(handle), Some(file_type)) = (
                    &self.file_importer.get_current_picked_file(),
                    self.selected_file_type,
                ) {
                    let file_path = handle.path().to_path_buf().clone();
                    let target_path = resolve_file_type_path(&self.collection_root_dir, &file_type);
                    let file_filter = self
                        .file_importer
                        .get_selected_files_from_current_picked_file_that_are_new()
                        .iter()
                        .map(|file| file.file_name.clone())
                        .collect::<HashSet<String>>();
                    return Task::perform(
                        async move {
                            file_import::import_files_from_zip(
                                file_path,
                                target_path,
                                file_filter,
                                file_type.into(),
                            )
                        },
                        FileAddWidgetMessage::FilesImported,
                    );
                } else {
                    eprintln!("No file selected");
                    return Task::none();
                }
            }

            FileAddWidgetMessage::FilesImported(result) => match &result {
                // Note: imported_files_map contains only the new files, not all the selected files
                // for the file set - some of the files in file set may have been already imported with another file set
                Ok(imported_files_map) => {
                    if let Some(file_type) = self.selected_file_type {
                        self.file_importer
                            .set_imported_files(imported_files_map.clone());
                        let repo = Arc::clone(&self.repositories);
                        let file_name = self.file_name.clone();

                        // combine the newly imported files with the existing files
                        let mut imported_files = imported_files_map
                            .values()
                            .cloned()
                            .collect::<Vec<ImportedFile>>();

                        imported_files.extend(self.file_importer.existing_files.values().cloned());
                        return Task::perform(
                            async move {
                                repo.get_file_set_repository()
                                    .add_file_set(file_name, file_type, imported_files)
                                    .await
                            },
                            FileAddWidgetMessage::FilesSavedToDatabase,
                        );
                    }
                }
                Err(err) => {
                    eprintln!("Error importing files: {}", err);
                }
            },
            FileAddWidgetMessage::FilesSavedToDatabase(result) => match result {
                Ok(file_set_id) => {
                    if let Some(file_type) = self.selected_file_type {
                        self.file_importer.set_imported_files(HashMap::new());
                        self.file_importer.clear();
                        let list_model = FileSetListModel {
                            id: file_set_id,
                            file_set_name: self.file_name.clone(),
                            file_type,
                        };
                        self.file_name = "".to_string();
                        self.selected_file_type = None;
                        self.file_importer.clear();
                        return Task::done(FileAddWidgetMessage::FileSetAdded(list_model));
                    }
                }
                Err(err) => {
                    eprintln!("Error saving files to database: {}", err);
                    // TODO: delete imported files and show error message
                }
            },
            FileAddWidgetMessage::Reset => {
                self.file_name = "".to_string();
                self.selected_file_type = None;
                self.file_importer.clear();
            }
            _ => (),
        }
        Task::none()
    }

    pub fn view(&self) -> iced::Element<FileAddWidgetMessage> {
        let name_input = text_input("File name", &self.file_name)
            .on_input(FileAddWidgetMessage::FileNameUpdated);

        let submit_button = button("Submit file").on_press_maybe(
            (!self.file_name.is_empty()
                && self.selected_file_type.is_some()
                && self.file_importer.is_selected_files())
            .then_some(FileAddWidgetMessage::Submit),
        );
        let file_picker = self.create_file_picker();
        let picked_file_contents = self.create_picked_file_contents();
        column![
            row![file_picker, name_input, submit_button]
                .spacing(DEFAULT_SPACING)
                .padding(DEFAULT_PADDING)
                .align_y(alignment::Vertical::Center),
            scrollable(picked_file_contents),
        ]
        .into()
    }

    fn create_file_picker(&self) -> Element<FileAddWidgetMessage> {
        let collection_file_type_picker = pick_list(
            vec![
                FileType::Rom,
                FileType::DiskImage,
                FileType::CoverScan,
                FileType::Manual,
                FileType::Screenshot,
                FileType::TapeImage,
            ],
            self.selected_file_type,
            FileAddWidgetMessage::FileTypeSelected,
        )
        .width(DEFAULT_PICKER_WIDTH)
        .placeholder("Select file type");
        let file_type_label = text("File type").width(DEFAULT_LABEL_WIDTH);
        let add_file_button = button("Select file").on_press_maybe(
            (self.selected_file_type.is_some()).then_some(FileAddWidgetMessage::StartFileSelection),
        );
        row![
            file_type_label,
            collection_file_type_picker,
            add_file_button
        ]
        .spacing(DEFAULT_SPACING)
        .into()
    }

    fn create_picked_file_contents(&self) -> Element<FileAddWidgetMessage> {
        let mut rows: Vec<Element<FileAddWidgetMessage>> = Vec::new();
        for read_file in self
            .file_importer
            .get_current_picked_file_content()
            .values()
        {
            let is_selected = self
                .file_importer
                .is_file_selected(&read_file.sha1_checksum);
            let checkbox: checkbox::Checkbox<'_, FileAddWidgetMessage> =
                checkbox(&read_file.file_name, is_selected).on_toggle(move |_| {
                    FileAddWidgetMessage::FileSelectionToggled(read_file.sha1_checksum)
                });
            let row = row![checkbox]
                .spacing(DEFAULT_SPACING)
                .padding(DEFAULT_PADDING);
            rows.push(row.into());
        }
        Column::with_children(rows).into()
    }
}
