use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
};

use core_types::{ImportedFile, Sha1Checksum};
use database::{
    database_error::Error,
    models::{FileInfo, FileType},
    repository_manager::RepositoryManager,
};
use file_import::{CompressionMethod, FileImportError};
use iced::{
    alignment,
    widget::{button, checkbox, column, pick_list, row, scrollable, text_input, Column},
    Element, Task,
};
use rfd::FileHandle;
use service::view_models::FileSetListModel;

use crate::defaults::{DEFAULT_PADDING, DEFAULT_SPACING};

pub struct FileImporter {
    current_picked_file: Option<FileHandle>,
    current_picked_file_content: HashMap<Sha1Checksum, ImportedFile>,
    existing_files: HashMap<Sha1Checksum, FileInfo>,
    selected_files_from_current_picked_file: HashSet<String>,
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
    pub fn get_current_picked_file_content(&self) -> &HashMap<Sha1Checksum, ImportedFile> {
        &self.current_picked_file_content
    }
    pub fn get_selected_files_from_current_picked_file(&self) -> &HashSet<String> {
        &self.selected_files_from_current_picked_file
    }
    pub fn set_current_picked_file(&mut self, file: FileHandle) {
        self.current_picked_file = Some(file);
    }
    pub fn set_current_picked_file_content(
        &mut self,
        content: HashMap<Sha1Checksum, ImportedFile>,
    ) {
        self.selected_files_from_current_picked_file
            .extend(content.values().map(|f| f.file_name.clone()));
        self.current_picked_file_content = content;
    }
    pub fn set_existing_files(&mut self, files: Vec<FileInfo>) {
        let mut file_map: HashMap<Sha1Checksum, FileInfo> = HashMap::new();
        for file in files {
            let checksum = file
                .sha1_checksum
                .clone()
                .try_into()
                .expect("Invalid checksum length");
            file_map.insert(checksum, file);
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

    pub fn is_file_selected(&self, file_name: &str) -> bool {
        self.selected_files_from_current_picked_file
            .contains(file_name)
    }

    pub fn deselect_file(&mut self, file_name: &str) {
        self.selected_files_from_current_picked_file
            .remove(file_name);
    }

    pub fn select_file(&mut self, file_name: &str) {
        self.selected_files_from_current_picked_file
            .insert(file_name.to_string());
    }

    pub fn toggle_file_selection(&mut self, file_name: &str) {
        if self.is_file_selected(file_name) {
            self.deselect_file(file_name);
        } else {
            self.select_file(file_name);
        }
    }

    pub fn get_filtered_picked_file_content(&self) -> Vec<ImportedFile> {
        self.current_picked_file_content
            .values()
            .filter(|file| {
                self.selected_files_from_current_picked_file
                    .contains(&file.file_name)
            })
            .map(|file| ImportedFile {
                file_name: file.file_name.clone(),
                sha1_checksum: file.sha1_checksum,
                file_size: file.file_size,
            })
            .collect::<Vec<ImportedFile>>()
    }
}

pub struct FileAddWidget {
    file_name: String,
    selected_file_type: Option<FileType>,
    file_importer: FileImporter,
    collection_root_dir: PathBuf,
    repositories: Arc<RepositoryManager>,
}

#[derive(Debug, Clone)]
pub enum FileAddWidgetMessage {
    FileNameUpdated(String),
    CancelAddFile,
    Submit,
    StartFileSelection,
    FileTypeSelected(FileType),
    FilePicked(Option<FileHandle>),
    FileContentsRead(Result<HashMap<Sha1Checksum, ImportedFile>, FileImportError>),
    FileSelectionToggled(String),
    FilesImported(Result<HashMap<Sha1Checksum, ImportedFile>, FileImportError>),
    FilesSavedToDatabase(Result<i64, Error>),
    ExistingFilesRead(Result<Vec<FileInfo>, Error>),
    FileSetAdded(FileSetListModel),
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
            FileAddWidgetMessage::StartFileSelection => {
                if self.selected_file_type.is_none() {
                    return Task::none();
                }
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Choose a file")
                            // TODO: support other archive formats and non archived files
                            .add_filter("Zip archive", &["zip"])
                            .pick_file()
                            .await
                    },
                    FileAddWidgetMessage::FilePicked,
                );
            }
            FileAddWidgetMessage::FilePicked(file_handle) => {
                if let Some(handle) = file_handle {
                    self.file_name = handle.file_name();
                    let file_path = handle.path().to_path_buf();
                    self.file_importer.set_current_picked_file(handle.clone());

                    return Task::perform(
                        async move { file_import::read_zip_contents_with_checksums(file_path) },
                        FileAddWidgetMessage::FileContentsRead,
                    );
                } else {
                    println!("No file selected");
                }
            }
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
            FileAddWidgetMessage::ExistingFilesRead(result) => match result {
                Ok(existing_files) => {
                    self.file_importer.set_existing_files(existing_files);
                }
                Err(err) => {
                    eprintln!("Error reading existing files: {}", err);
                }
            },
            FileAddWidgetMessage::FileSelectionToggled(file_name) => {
                self.file_importer.toggle_file_selection(&file_name)
            }
            FileAddWidgetMessage::FileNameUpdated(name) => {
                self.file_name = name;
            }
            FileAddWidgetMessage::CancelAddFile => {
                self.file_name = "".to_string();
                self.selected_file_type = None;
                self.file_importer.clear();
            }
            FileAddWidgetMessage::Submit => {
                if let Some(handle) = &self.file_importer.get_current_picked_file() {
                    let file_path = handle.path().to_path_buf().clone();
                    let collection_root_dir = self.collection_root_dir.clone();
                    let file_filter = self
                        .file_importer
                        .get_selected_files_from_current_picked_file()
                        .clone();
                    return Task::perform(
                        async move {
                            file_import::import_files_from_zip(
                                file_path,
                                collection_root_dir,
                                CompressionMethod::Zstd,
                                file_filter,
                            )
                        },
                        FileAddWidgetMessage::FilesImported,
                    );
                } else {
                    eprintln!("No file selected");
                    return Task::none();
                }
            }

            FileAddWidgetMessage::FilesImported(result) => match result {
                Ok(files) => {
                    if let Some(file_type) = self.selected_file_type {
                        self.file_importer.set_imported_files(files);
                        let repo = Arc::clone(&self.repositories);
                        let file_name = self.file_name.clone();
                        let filtered_picked_file_content =
                            self.file_importer.get_filtered_picked_file_content();
                        return Task::perform(
                            async move {
                                repo.get_file_set_repository()
                                    .add_file_set(
                                        file_name,
                                        file_type,
                                        filtered_picked_file_content,
                                    )
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
            _ => (),
        }
        Task::none()
    }

    pub fn view(&self) -> iced::Element<FileAddWidgetMessage> {
        let name_input = text_input("File name", &self.file_name)
            .on_input(FileAddWidgetMessage::FileNameUpdated);

        let submit_button = button("Submit file")
            .on_press_maybe((!self.file_name.is_empty()).then_some(FileAddWidgetMessage::Submit));
        let cancel_button = button("Cancel").on_press(FileAddWidgetMessage::CancelAddFile);
        let file_picker = self.create_file_picker();
        let picked_file_contents = self.create_picked_file_contents();
        column![
            row![file_picker, name_input, submit_button, cancel_button]
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
        );
        let add_file_button = button("Add File").on_press_maybe(
            (self.selected_file_type.is_some()).then_some(FileAddWidgetMessage::StartFileSelection),
        );
        row![collection_file_type_picker, add_file_button].into()
    }

    fn create_picked_file_contents(&self) -> Element<FileAddWidgetMessage> {
        let mut rows: Vec<Element<FileAddWidgetMessage>> = Vec::new();
        for import_file in self
            .file_importer
            .get_current_picked_file_content()
            .values()
        {
            let is_selected = self.file_importer.is_file_selected(&import_file.file_name);
            let checkbox: checkbox::Checkbox<'_, FileAddWidgetMessage> =
                checkbox(&import_file.file_name, is_selected).on_toggle(move |_| {
                    FileAddWidgetMessage::FileSelectionToggled(import_file.file_name.clone())
                });
            let row = row![checkbox]
                .spacing(DEFAULT_SPACING)
                .padding(DEFAULT_PADDING);
            rows.push(row.into());
        }
        Column::with_children(rows).into()
    }
}
