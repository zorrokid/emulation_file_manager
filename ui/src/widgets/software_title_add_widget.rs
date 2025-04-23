use iced::{
    alignment,
    widget::{button, row, text_input},
};

use crate::defaults::{DEFAULT_PADDING, DEFAULT_SPACING};

pub struct SoftwareTitleAddWidget {
    software_title_name: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    SoftwareTitleNameUpdated(String),
    CancelAddSoftwareTitle,
    Submit,
}

pub enum Action {
    AddSoftwareTitle(String),
    None,
}

impl SoftwareTitleAddWidget {
    pub fn new() -> Self {
        Self {
            software_title_name: "".to_string(),
        }
    }

    // TODO: maybe return Task<Message> instead of Action
    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::SoftwareTitleNameUpdated(name) => self.software_title_name = name,
            Message::Submit => return Action::AddSoftwareTitle(self.software_title_name.clone()),
            Message::CancelAddSoftwareTitle => println!("Cancel"),
        }
        Action::None
    }

    pub fn view(&self) -> iced::Element<Message> {
        let name_input = text_input("SoftwareTitle name", &self.software_title_name)
            .on_input(Message::SoftwareTitleNameUpdated);

        let submit_button = button("Submit software_title")
            .on_press_maybe((!self.software_title_name.is_empty()).then_some(Message::Submit));
        let cancel_button = button("Cancel").on_press(Message::CancelAddSoftwareTitle);
        row![name_input, submit_button, cancel_button]
            .spacing(DEFAULT_SPACING)
            .padding(DEFAULT_PADDING)
            .align_y(alignment::Vertical::Center)
            .into()
    }
}
