use iced::{
    alignment::Vertical,
    widget::{pick_list, row, text},
    Task,
};
use service::view_models::SystemListModel;

use crate::defaults::{DEFAULT_PADDING, DEFAULT_SPACING};

pub struct SystemSelectWidget {
    systems: Vec<SystemListModel>,
    selected_system: Option<SystemListModel>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SystemSelected(SystemListModel),
    SetSystems(Vec<SystemListModel>),
}

impl SystemSelectWidget {
    pub fn new() -> Self {
        Self {
            systems: vec![],
            selected_system: None,
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        println!("SystemSelectWidget update: {:?}", message);
        match message {
            Message::SystemSelected(system) => Task::done(Message::SystemSelected(system.clone())),
            Message::SetSystems(systems) => {
                self.systems = systems;
                self.selected_system = None;
                Task::none()
            }
        }
    }

    pub fn view(&self) -> iced::Element<Message> {
        let system_select = pick_list(
            self.systems.as_slice(),
            self.selected_system.clone(),
            Message::SystemSelected,
        );
        let label = text!("Select system");
        row![label, system_select]
            .spacing(DEFAULT_SPACING)
            .padding(DEFAULT_PADDING)
            .align_y(Vertical::Center)
            .into()
    }
}
