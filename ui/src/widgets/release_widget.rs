use iced::{widget::text, Element, Task};

pub struct ReleaseWidget {}

pub enum Message {}

impl ReleaseWidget {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {}
    }

    pub fn view(&self) -> Element<Message> {
        text!("Release Widget").into()
    }
}
