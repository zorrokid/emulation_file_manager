use iced::{
    alignment::Vertical,
    widget::{pick_list, row, text},
    Task,
};
use service::view_models::SoftwareTitleListModel;

use crate::defaults::{DEFAULT_PADDING, DEFAULT_SPACING};

pub struct SoftwareTitleSelectWidget {
    software_titles: Vec<SoftwareTitleListModel>,
    selected_software_title: Option<SoftwareTitleListModel>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SoftwareTitleSelected(SoftwareTitleListModel),
    SetSoftwareTitles(Vec<SoftwareTitleListModel>),
}

impl SoftwareTitleSelectWidget {
    pub fn new() -> Self {
        Self {
            software_titles: vec![],
            selected_software_title: None,
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SoftwareTitleSelected(software_title) => {
                Task::done(Message::SoftwareTitleSelected(software_title.clone()))
            }
            Message::SetSoftwareTitles(software_titles) => {
                self.software_titles = software_titles;
                self.selected_software_title = None;
                Task::none()
            }
        }
    }

    pub fn view(&self) -> iced::Element<Message> {
        let software_title_select = pick_list(
            self.software_titles.as_slice(),
            self.selected_software_title.clone(),
            Message::SoftwareTitleSelected,
        );
        let label = text!("Select software title");
        row![label, software_title_select]
            .spacing(DEFAULT_SPACING)
            .padding(DEFAULT_PADDING)
            .align_y(Vertical::Center)
            .into()
    }
}
