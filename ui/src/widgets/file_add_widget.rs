use std::collections::HashMap;

use database::models::FileType;
use iced::{
    alignment,
    widget::{button, column, pick_list, row, text, text_input, Column},
    Element, Task,
};
use rfd::FileHandle;

use crate::defaults::{DEFAULT_PADDING, DEFAULT_SPACING};

pub struct FileAddWidget {
    file_name: String,
    selected_file_type: Option<FileType>,
    current_picked_file: Option<FileHandle>,
    current_picked_file_content: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    FileNameUpdated(String),
    CancelAddFile,
    Submit,
    StartFileSelection,
    FileTypeSelected(FileType),
    FilePicked(Option<FileHandle>),
}

impl FileAddWidget {
    pub fn new() -> Self {
        Self {
            file_name: "".to_string(),
            selected_file_type: None,
            current_picked_file: None,
            current_picked_file_content: HashMap::new(),
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::FileNameUpdated(name) => {
                self.file_name = name;
            }
            Message::Submit => {
                // TODO
            }
            Message::CancelAddFile => {
                // TODO
                println!("Cancel");
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
                    let file_path = handle.path();
                    let file_map = file_import::read_zip_contents(file_path);
                    println!("File map: {:?}", file_map);
                    if let Ok(file_map) = file_map {
                        self.current_picked_file = Some(handle);
                        self.current_picked_file_content = file_map;
                    } else {
                        // TODO: submit error
                        eprintln!("Error reading file contents");
                    }
                } else {
                    println!("No file selected");
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
            picked_file_contents,
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
        for (file_name, checksum) in &self.current_picked_file_content {
            let row = row![
                text!("File name: {}", file_name),
                text!("Checksum: {}", checksum)
            ]
            .spacing(DEFAULT_SPACING)
            .padding(DEFAULT_PADDING);
            rows.push(row.into());
        }
        Column::with_children(rows).into()
    }
}
