use iced::{
    alignment::Vertical,
    widget::{pick_list, row, text},
    Task,
};
use service::view_models::FileSetListModel;

use crate::defaults::{DEFAULT_PADDING, DEFAULT_SPACING};

pub struct FileSelectWidget {
    files: Vec<FileSetListModel>,
    selected_file: Option<FileSetListModel>,
}

#[derive(Debug, Clone)]
pub enum FileSelectWidgetMessage {
    FileSelected(FileSetListModel),
    SetFiles(Vec<FileSetListModel>),
}

impl FileSelectWidget {
    pub fn new() -> Self {
        Self {
            files: vec![],
            selected_file: None,
        }
    }

    pub fn update(&mut self, message: FileSelectWidgetMessage) -> Task<FileSelectWidgetMessage> {
        match message {
            //Message::FileSelected(file) => Task::done(Message::FileSelected(file.clone())),
            FileSelectWidgetMessage::SetFiles(files) => {
                self.files = files;
                self.selected_file = None;
            }
            _ => (),
        }
        Task::none()
    }

    pub fn view(&self) -> iced::Element<FileSelectWidgetMessage> {
        let file_select = pick_list(
            self.files.as_slice(),
            self.selected_file.clone(),
            FileSelectWidgetMessage::FileSelected,
        );
        let label = text!("Select file");
        row![label, file_select]
            .spacing(DEFAULT_SPACING)
            .padding(DEFAULT_PADDING)
            .align_y(Vertical::Center)
            .into()
    }
}
