use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
};

use database::{
    models::{FileType, PickedFileInfo},
    repository_manager::RepositoryManager,
};
use file_import::{CompressionMethod, FileImportError, ImportedFile, Sha1Checksum};
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
    current_picked_file_content: HashSet<String>,
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
    FileContentsRead(Result<HashSet<String>, FileImportError>),
    FileSelectionToggled(String),
    FilesImported(Result<HashMap<Sha1Checksum, ImportedFile>, FileImportError>),
}

impl FileAddWidget {
    pub fn new(collection_root_dir: PathBuf, repositories: Arc<RepositoryManager>) -> Self {
        Self {
            file_name: "".to_string(),
            selected_file_type: None,
            current_picked_file: None,
            current_picked_file_content: HashSet::new(),
            selected_files_from_current_picked_file: HashSet::new(),
            collection_root_dir,
            imported_files: HashMap::new(),
            repositories,
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::FileNameUpdated(name) => {
                self.file_name = name;
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
                        self.imported_files = files.clone();
                        let repo = Arc::clone(&self.repositories);
                        let file_name = self.file_name.clone();
                        let files = self
                            .selected_files_from_current_picked_file
                            .iter()
                            .map(|f| f.to_string())
                            .collect::<Vec<PickedFileInfo>>();
                        Task::perform(
                            {
                                repo.get_file_set_repository()
                                    .add_file_set(file_name, file_type, files)
                                    .await
                            },
                            Message::FilesSavedToDatabase,
                        )
                    }

                    // TODO: save imported files to database
                    // TODO: if files are saved successfully, clear the selected files
                    // TODO: if files are NOT saved successfully, delete the imported files and
                    // show an error message
                }
                Err(err) => {
                    eprintln!("Error importing files: {}", err);
                }
            },
            Message::CancelAddFile => {
                self.file_name = "".to_string();
                self.selected_file_type = None;
                self.current_picked_file = None;
                self.current_picked_file_content.clear();
                self.selected_files_from_current_picked_file.clear();
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
            Message::FileTypeSelected(file_type) => {
                self.selected_file_type = Some(file_type);
            }
            Message::FilePicked(file_handle) => {
                if let Some(handle) = file_handle {
                    println!("File selected: {:?}", handle.file_name());
                    self.file_name = handle.file_name();
                    let file_path = handle.path().to_path_buf();
                    self.current_picked_file = Some(handle.clone());

                    return Task::perform(
                        async move { file_import::read_zip_contents(file_path) },
                        Message::FileContentsRead,
                    );
                } else {
                    println!("No file selected");
                }
            }
            Message::FileContentsRead(result) => match result {
                Ok(files) => {
                    self.current_picked_file_content = files.clone();
                    self.selected_files_from_current_picked_file = files;
                }
                Err(err) => {
                    eprintln!("Error reading file contents: {}", err);
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
        for file_name in &self.current_picked_file_content {
            let is_selected = self
                .selected_files_from_current_picked_file
                .contains(file_name);
            let checkbox: checkbox::Checkbox<'_, Message> = checkbox(file_name, is_selected)
                .on_toggle(|_| Message::FileSelectionToggled(file_name.clone()));
            let row = row![checkbox]
                .spacing(DEFAULT_SPACING)
                .padding(DEFAULT_PADDING);
            rows.push(row.into());
        }
        Column::with_children(rows).into()
    }
}
