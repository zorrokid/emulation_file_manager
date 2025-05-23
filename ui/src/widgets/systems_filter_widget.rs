use iced::{
    alignment::Vertical,
    widget::{pick_list, row, text},
    Task,
};
use service::view_models::SystemListModel;

use crate::defaults::{DEFAULT_PADDING, DEFAULT_SPACING};

pub struct SystemFilterWidget {
    systems: Vec<SystemListModel>,
    selected_system: Option<SystemListModel>,
}

#[derive(Debug, Clone)]
pub enum SystemFilterWidgetMessage {
    SystemSelected(SystemListModel),
    SetSystems(Vec<SystemListModel>),
    SetSelectedSystem(SystemListModel),
}

impl SystemFilterWidget {
    pub fn new() -> Self {
        Self {
            systems: vec![],
            selected_system: None,
        }
    }

    pub fn update(
        &mut self,
        message: SystemFilterWidgetMessage,
    ) -> Task<SystemFilterWidgetMessage> {
        match message {
            SystemFilterWidgetMessage::SystemSelected(system) => {
                self.selected_system = Some(system.clone());
                Task::done(SystemFilterWidgetMessage::SetSelectedSystem(system.clone()))
            }
            SystemFilterWidgetMessage::SetSystems(systems) => {
                self.systems = systems;
                self.selected_system = None;
                Task::none()
            }
            _ => Task::none(),
        }
    }

    pub fn view(&self) -> iced::Element<SystemFilterWidgetMessage> {
        let system_select = pick_list(
            self.systems.as_slice(),
            self.selected_system.clone(),
            SystemFilterWidgetMessage::SystemSelected,
        );
        let label = text!("Select system");
        row![label, system_select]
            .spacing(DEFAULT_SPACING)
            .padding(DEFAULT_PADDING)
            .align_y(Vertical::Center)
            .into()
    }
}
