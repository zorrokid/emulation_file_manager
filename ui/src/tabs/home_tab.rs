use iced::{widget::text, Task};

pub struct HomeTab {}

#[derive(Debug, Clone)]
pub enum Message {}

impl HomeTab {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        Task::none()
    }

    pub fn view(&self) -> iced::Element<Message> {
        text("Home Tab").into()
    }
}
