use iced::{widget::text, Task};

pub struct HomeTab {}

#[derive(Debug, Clone)]
pub enum HomeTabMessage {}

impl HomeTab {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update(&mut self, message: HomeTabMessage) -> Task<HomeTabMessage> {
        Task::none()
    }

    pub fn view(&self) -> iced::Element<HomeTabMessage> {
        text("Home Tab").into()
    }
}
