use iced::{
    alignment,
    widget::{button, row, text_input},
    Task,
};

use crate::defaults::{DEFAULT_PADDING, DEFAULT_SPACING};

pub struct SystemAddWidget {
    system_name: String,
    system_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub enum SystemAddWidgetMessage {
    SystemNameUpdated(String),
    Submit,
    AddSystem(String),
    UpdateSystem(i64, String),
    SetEditSystem(i64, String),
}

impl SystemAddWidget {
    pub fn new() -> Self {
        Self {
            system_name: "".to_string(),
            system_id: None,
        }
    }

    pub fn update(&mut self, message: SystemAddWidgetMessage) -> Task<SystemAddWidgetMessage> {
        match message {
            SystemAddWidgetMessage::SystemNameUpdated(name) => {
                self.system_name = name;
                Task::none()
            }
            SystemAddWidgetMessage::Submit => {
                let system_name = self.system_name.clone();
                let task = if let Some(id) = self.system_id {
                    SystemAddWidgetMessage::UpdateSystem(id, system_name)
                } else {
                    SystemAddWidgetMessage::AddSystem(system_name)
                };

                self.system_name.clear();
                Task::done(task)
            }
            SystemAddWidgetMessage::SetEditSystem(id, name) => {
                self.system_id = Some(id);
                self.system_name = name;
                Task::none()
            }
            _ => Task::none(),
        }
    }

    pub fn view(&self) -> iced::Element<SystemAddWidgetMessage> {
        let name_input = text_input("System name", &self.system_name)
            .on_input(SystemAddWidgetMessage::SystemNameUpdated);

        let submit_button = button("Submit system").on_press_maybe(
            (!self.system_name.is_empty()).then_some(SystemAddWidgetMessage::Submit),
        );
        row![name_input, submit_button]
            .spacing(DEFAULT_SPACING)
            .padding(DEFAULT_PADDING)
            .align_y(alignment::Vertical::Center)
            .into()
    }
}
