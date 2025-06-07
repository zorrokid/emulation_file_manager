use iced::{
    alignment::Vertical,
    widget::{pick_list, row, text},
    Task,
};
use service::view_models::FileSetListModel;

use crate::defaults::{
    DEFAULT_LABEL_WIDTH, DEFAULT_PADDING, DEFAULT_PICKER_WIDTH, DEFAULT_SPACING,
};

pub struct FileSelectWidget {
    selected_file: Option<FileSetListModel>,
}

#[derive(Debug, Clone)]
pub enum FileSelectWidgetMessage {
    Reset,
    FileSelected(FileSetListModel),
}

impl FileSelectWidget {
    pub fn new() -> Self {
        Self {
            selected_file: None,
        }
    }

    pub fn update(&mut self, message: FileSelectWidgetMessage) -> Task<FileSelectWidgetMessage> {
        match message {
            FileSelectWidgetMessage::FileSelected(file) => {
                self.selected_file = Some(file.clone());
                Task::none()
            }
            FileSelectWidgetMessage::Reset => {
                self.selected_file = None;
                Task::none()
            }
        }
    }

    pub fn view<'a>(
        &self,
        files: &'a [FileSetListModel],
    ) -> iced::Element<'a, FileSelectWidgetMessage> {
        let file_select = pick_list(
            files,
            self.selected_file.clone(),
            FileSelectWidgetMessage::FileSelected,
        )
        .width(DEFAULT_PICKER_WIDTH)
        .placeholder("Select file set");
        let label = text!("File set").width(DEFAULT_LABEL_WIDTH);
        row![label, file_select]
            .spacing(DEFAULT_SPACING)
            .padding(DEFAULT_PADDING)
            .align_y(Vertical::Center)
            .into()
    }
}
