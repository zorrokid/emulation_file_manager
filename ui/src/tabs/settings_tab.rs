use iced::{widget::text, Task};

pub struct SettingsTab {}

#[derive(Debug, Clone)]
pub enum SettingsTabMessage {}

impl SettingsTab {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update(&mut self, message: SettingsTabMessage) -> Task<SettingsTabMessage> {
        Task::none()
    }

    pub fn view(&self) -> iced::Element<SettingsTabMessage> {
        text!("Settings").into()
    }
}
