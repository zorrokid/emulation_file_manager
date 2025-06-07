use iced::{
    alignment::Vertical,
    widget::{button, pick_list, row, text},
    Task,
};
use service::view_models::SystemListModel;

use crate::defaults::{
    DEFAULT_LABEL_WIDTH, DEFAULT_PADDING, DEFAULT_PICKER_WIDTH, DEFAULT_SPACING,
};

pub struct SystemFilterWidget {
    systems: Vec<SystemListModel>,
    selected_system: Option<SystemListModel>,
}

#[derive(Debug, Clone)]
pub enum SystemFilterWidgetMessage {
    SystemSelected(SystemListModel),
    SetSystems(Vec<SystemListModel>),
    SetSelectedSystem(Option<i64>),
    ClearSelection,
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
                Task::done(SystemFilterWidgetMessage::SetSelectedSystem(Some(
                    system.id,
                )))
            }
            SystemFilterWidgetMessage::SetSystems(systems) => {
                self.systems = systems;
                self.selected_system = None;
                Task::none()
            }
            SystemFilterWidgetMessage::ClearSelection => {
                self.selected_system = None;
                Task::done(SystemFilterWidgetMessage::SetSelectedSystem(None))
            }
            _ => Task::none(),
        }
    }

    pub fn view(&self) -> iced::Element<SystemFilterWidgetMessage> {
        let system_select = pick_list(
            self.systems.as_slice(),
            self.selected_system.clone(),
            SystemFilterWidgetMessage::SystemSelected,
        )
        .width(DEFAULT_PICKER_WIDTH)
        .placeholder("Select System");
        let label = text!("System").width(DEFAULT_LABEL_WIDTH);
        let clear_button = button("Clear").on_press(SystemFilterWidgetMessage::ClearSelection);
        row![label, system_select, clear_button]
            .spacing(DEFAULT_SPACING)
            .padding(DEFAULT_PADDING)
            .align_y(Vertical::Center)
            .into()
    }
}
