use iced::{
    alignment::Vertical,
    widget::{pick_list, row, text},
    Task,
};
use service::view_models::SystemListModel;

use crate::defaults::{DEFAULT_LABEL_WIDTH, DEFAULT_PADDING, DEFAULT_SPACING};

pub struct SystemSelectWidget {
    systems: Vec<SystemListModel>,
    selected_system: Option<SystemListModel>,
}

#[derive(Debug, Clone)]
pub enum SystemSelectWidgetMessage {
    Reset,
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

    pub fn update(
        &mut self,
        message: SystemSelectWidgetMessage,
    ) -> Task<SystemSelectWidgetMessage> {
        match message {
            SystemSelectWidgetMessage::SystemSelected(system) => {
                Task::done(SystemSelectWidgetMessage::SystemSelected(system.clone()))
            }
            SystemSelectWidgetMessage::SetSystems(systems) => {
                self.systems = systems;
                self.selected_system = None;
                Task::none()
            }
            SystemSelectWidgetMessage::Reset => {
                self.selected_system = None;
                Task::none()
            }
        }
    }

    // TODO: pass systems to view from parent widget
    // [&SystemListModel]
    pub fn view(&self) -> iced::Element<SystemSelectWidgetMessage> {
        let system_select = pick_list(
            self.systems.as_slice(),
            self.selected_system.clone(),
            SystemSelectWidgetMessage::SystemSelected,
        );
        let label = text!("Select system").width(DEFAULT_LABEL_WIDTH);
        row![label, system_select]
            .spacing(DEFAULT_SPACING)
            .padding(DEFAULT_PADDING)
            .align_y(Vertical::Center)
            .into()
    }
}
