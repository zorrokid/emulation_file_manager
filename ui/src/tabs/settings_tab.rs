use iced::{widget::text, Task};

pub struct SettingsTab {}

#[derive(Debug, Clone)]
pub enum Message {}

impl SettingsTab {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        Task::none()
    }

    pub fn view(&self) -> iced::Element<Message> {
        text!("Settings").into()
    }
}
