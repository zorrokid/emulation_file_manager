use iced::{
    alignment,
    widget::{button, row, text_input},
    Task,
};

use crate::defaults::{DEFAULT_PADDING, DEFAULT_SPACING};

pub struct SystemAddWidget {
    system_name: String,
}

#[derive(Debug, Clone)]
pub enum SystemAddWidgetMessage {
    SystemNameUpdated(String),
    Submit,
    AddSystem(String),
}

impl SystemAddWidget {
    pub fn new() -> Self {
        Self {
            system_name: "".to_string(),
        }
    }

    pub fn update(&mut self, message: SystemAddWidgetMessage) -> Task<SystemAddWidgetMessage> {
        match message {
            SystemAddWidgetMessage::SystemNameUpdated(name) => {
                println!("System name updated: {}", name);
                self.system_name = name;
            }
            SystemAddWidgetMessage::Submit => {
                println!("Submitting system: {}", self.system_name);
                return Task::done(SystemAddWidgetMessage::AddSystem(self.system_name.clone()));
            }
            _ => {}
        }
        Task::none()
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
