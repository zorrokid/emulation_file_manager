use iced::{widget::text, Task};

pub struct AddReleaseTab {}

#[derive(Debug, Clone)]
pub enum Message {}

impl AddReleaseTab {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        Task::none()
    }

    pub fn view(&self) -> iced::Element<Message> {
        text("Add Release Tab").into()
    }
}
