use iced::{
    alignment,
    widget::{button, row, text_input},
    Task,
};
use service::view_models::SoftwareTitleListModel;

use crate::defaults::{DEFAULT_PADDING, DEFAULT_SPACING};

pub struct SoftwareTitleAddWidget {
    software_title_name: String,
    software_title_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub enum SoftwareTitleAddWidgetMessage {
    SoftwareTitleNameUpdated(String),
    Submit,
    SetEditSoftwareTitle(i64, String),
    AddSoftwareTitle(String),
    UpdateSoftwareTitle(i64, String),
    Reset,
}

impl SoftwareTitleAddWidget {
    pub fn new() -> Self {
        Self {
            software_title_name: "".to_string(),
            software_title_id: None,
        }
    }

    pub fn update(
        &mut self,
        message: SoftwareTitleAddWidgetMessage,
    ) -> Task<SoftwareTitleAddWidgetMessage> {
        match message {
            SoftwareTitleAddWidgetMessage::SoftwareTitleNameUpdated(name) => {
                self.software_title_name = name;
                Task::none()
            }
            SoftwareTitleAddWidgetMessage::Submit => {
                let software_title_name = self.software_title_name.clone();
                let task = if let Some(id) = self.software_title_id {
                    SoftwareTitleAddWidgetMessage::UpdateSoftwareTitle(id, software_title_name)
                } else {
                    SoftwareTitleAddWidgetMessage::AddSoftwareTitle(software_title_name)
                };
                self.software_title_name.clear();
                Task::done(task)
            }
            SoftwareTitleAddWidgetMessage::SetEditSoftwareTitle(id, name) => {
                self.software_title_id = Some(id);
                self.software_title_name = name;
                Task::none()
            }
            SoftwareTitleAddWidgetMessage::Reset => {
                self.software_title_name.clear();
                self.software_title_id = None;
                Task::none()
            }
            _ => Task::none(),
        }
    }

    pub fn view(&self) -> iced::Element<SoftwareTitleAddWidgetMessage> {
        let name_input = text_input("Software title name", &self.software_title_name)
            .on_input(SoftwareTitleAddWidgetMessage::SoftwareTitleNameUpdated);

        let submit_button = button("Submit").on_press_maybe(
            (!self.software_title_name.is_empty()).then_some(SoftwareTitleAddWidgetMessage::Submit),
        );
        row![name_input, submit_button]
            .spacing(DEFAULT_SPACING)
            .padding(DEFAULT_PADDING)
            .align_y(alignment::Vertical::Center)
            .into()
    }
}
