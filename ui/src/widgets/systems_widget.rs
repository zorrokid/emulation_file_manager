use iced::{widget::text, Task};

pub struct SystemsWidget {}

#[derive(Debug, Clone)]
pub enum Message {}

impl SystemsWidget {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        Task::none()
    }

    pub fn view(&self) -> iced::Element<Message> {
        text!("Systems").into()
    }
}
