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

use crate::defaults::{DEFAULT_PADDING, DEFAULT_SPACING};

pub struct FileAddWidget {
    file_name: String,
    selected_file_type: Option<FileType>,
    current_picked_file: Option<FileHandle>,
    current_picked_file_content: HashMap<Sha1Checksum, ImportedFile>,
    existing_files: HashMap<Sha1Checksum, FileInfo>,
    selected_files_from_current_picked_file: HashSet<String>,
    collection_root_dir: PathBuf,
    imported_files: HashMap<Sha1Checksum, ImportedFile>,
    repositories: Arc<RepositoryManager>,
}

#[derive(Debug, Clone)]
pub enum Message {
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
}

impl FileAddWidget {
    pub fn new(collection_root_dir: PathBuf, repositories: Arc<RepositoryManager>) -> Self {
        Self {
            file_name: "".to_string(),
            selected_file_type: None,
            current_picked_file: None,
            current_picked_file_content: HashMap::new(),
            existing_files: HashMap::new(),
            selected_files_from_current_picked_file: HashSet::new(),
            collection_root_dir,
            imported_files: HashMap::new(),
            repositories,
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::FileTypeSelected(file_type) => {
                self.selected_file_type = Some(file_type);
            }
            Message::StartFileSelection => {
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
                    Message::FilePicked,
                );
            }
            Message::FilePicked(file_handle) => {
                if let Some(handle) = file_handle {
                    println!("File selected: {:?}", handle.file_name());
                    self.file_name = handle.file_name();
                    let file_path = handle.path().to_path_buf();
                    self.current_picked_file = Some(handle.clone());

                    return Task::perform(
                        async move { file_import::read_zip_contents_with_checksums(file_path) },
                        Message::FileContentsRead,
                    );
                } else {
                    println!("No file selected");
                }
            }
            Message::FileContentsRead(result) => match result {
                Ok(files) => {
                    self.current_picked_file_content = files.clone();
                    self.selected_files_from_current_picked_file =
                        files.values().map(|f| f.file_name.clone()).collect();

                    let repo = Arc::clone(&self.repositories);

                    return Task::perform(
                        async move {
                            repo.get_file_info_repository()
                                .get_file_infos_by_sha1_checksums(files.keys().cloned().collect())
                                .await
                        },
                        Message::ExistingFilesRead,
                    );
                }
                Err(err) => {
                    eprintln!("Error reading file contents: {}", err);
                }
            },
            Message::ExistingFilesRead(result) => match result {
                Ok(existing_files) => {
                    // TODO: these files user cannot toggle but they should be filtered out from
                    // import_files_from_zip call
                    self.existing_files.clear();
                    for file in existing_files {
                        let checksum = file
                            .sha1_checksum
                            .clone()
                            .try_into()
                            .expect("Invalid checksum length");
                        self.existing_files.insert(checksum, file);
                    }
                }
                Err(err) => {
                    eprintln!("Error reading existing files: {}", err);
                }
            },
            Message::FileSelectionToggled(file_name) => {
                if self
                    .selected_files_from_current_picked_file
                    .contains(&file_name)
                {
                    self.selected_files_from_current_picked_file
                        .retain(|f| f != &file_name);
                } else {
                    self.selected_files_from_current_picked_file
                        .insert(file_name);
                }
            }
            Message::FileNameUpdated(name) => {
                self.file_name = name;
            }
            Message::CancelAddFile => {
                self.file_name = "".to_string();
                self.selected_file_type = None;
                self.current_picked_file = None;
                self.current_picked_file_content.clear();
                self.selected_files_from_current_picked_file.clear();
            }
            Message::Submit => {
                if let Some(handle) = &self.current_picked_file {
                    let file_path = handle.path().to_path_buf().clone();
                    let collection_root_dir = self.collection_root_dir.clone();
                    let file_filter = self.selected_files_from_current_picked_file.clone();
                    return Task::perform(
                        async move {
                            file_import::import_files_from_zip(
                                file_path,
                                collection_root_dir,
                                CompressionMethod::Zstd,
                                file_filter,
                            )
                        },
                        Message::FilesImported,
                    );
                } else {
                    eprintln!("No file selected");
                    return Task::none();
                }
            }

            Message::FilesImported(result) => match result {
                Ok(files) => {
                    if let Some(file_type) = self.selected_file_type {
                        self.imported_files = files;
                        let repo = Arc::clone(&self.repositories);
                        let file_name = self.file_name.clone();
                        let filtered_picked_file_content = self
                            .current_picked_file_content
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
                            .collect::<Vec<ImportedFile>>();
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
                            Message::FilesSavedToDatabase,
                        );
                    }
                }
                Err(err) => {
                    eprintln!("Error importing files: {}", err);
                }
            },
            Message::FilesSavedToDatabase(result) => match result {
                Ok(file_set_id) => {
                    println!("Files saved to database with id: {}", file_set_id);
                    self.file_name = "".to_string();
                    self.selected_file_type = None;
                    self.current_picked_file = None;
                    self.current_picked_file_content.clear();
                    self.selected_files_from_current_picked_file.clear();
                    self.imported_files.clear();
                }
                Err(err) => {
                    eprintln!("Error saving files to database: {}", err);
                    // TODO: delete imported files and show error message
                }
            },
        }
        Task::none()
    }

    pub fn view(&self) -> iced::Element<Message> {
        let name_input =
            text_input("File name", &self.file_name).on_input(Message::FileNameUpdated);

        let submit_button = button("Submit file")
            .on_press_maybe((!self.file_name.is_empty()).then_some(Message::Submit));
        let cancel_button = button("Cancel").on_press(Message::CancelAddFile);
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

    fn create_file_picker(&self) -> Element<Message> {
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
            Message::FileTypeSelected,
        );
        let add_file_button = button("Add File").on_press_maybe(
            (self.selected_file_type.is_some()).then_some(Message::StartFileSelection),
        );
        row![collection_file_type_picker, add_file_button].into()
    }

    fn create_picked_file_contents(&self) -> Element<Message> {
        let mut rows: Vec<Element<Message>> = Vec::new();
        for (_, import_file) in &self.current_picked_file_content {
            let is_selected = self
                .selected_files_from_current_picked_file
                .contains(&import_file.file_name);
            let checkbox: checkbox::Checkbox<'_, Message> =
                checkbox(&import_file.file_name, is_selected).on_toggle(move |_| {
                    Message::FileSelectionToggled(import_file.file_name.clone())
                });
            let row = row![checkbox]
                .spacing(DEFAULT_SPACING)
                .padding(DEFAULT_PADDING);
            rows.push(row.into());
        }
        Column::with_children(rows).into()
    }
}
