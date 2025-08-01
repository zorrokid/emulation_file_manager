use iced::{widget::text, Task};
use service::view_models::SystemListModel;

pub struct SystemsTab {
    systems: Vec<SystemListModel>,
}

#[derive(Debug, Clone)]
pub enum SystemsTabMessage {
    FetchSystems,
    SetSystems(Vec<SystemListModel>),
}

impl SystemsTab {
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
        }
    }

    pub fn update(&mut self, message: SystemsTabMessage) -> Task<SystemsTabMessage> {
        match message {
            SystemsTabMessage::FetchSystems => {
                // Here you would typically fetch the systems from a service or repository
                // For now, we will just simulate it with an empty task
                Task::none()
            }
            SystemsTabMessage::SetSystems(systems) => {
                self.systems = systems;
                Task::none()
            }
        }
    }

    pub fn view(&self) -> iced::Element<SystemsTabMessage> {
        if self.systems.is_empty() {
            text!("No systems available").into()
        } else {
            // Here you would typically create a view for each system
            // For simplicity, we will just display the count of systems
            text!(format!("{} systems available", self.systems.len())).into()
        }
    }
}
