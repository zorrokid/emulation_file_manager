use iced::{
    alignment::Vertical,
    widget::{pick_list, row, text},
    Task,
};
use service::view_models::SystemListModel;

use crate::defaults::{DEFAULT_LABEL_WIDTH, DEFAULT_PADDING, DEFAULT_SPACING};

pub struct SystemSelectWidget {
    selected_system: Option<SystemListModel>,
}

#[derive(Debug, Clone)]
pub enum SystemSelectWidgetMessage {
    Reset,
    SystemSelected(SystemListModel),
}

impl SystemSelectWidget {
    pub fn new() -> Self {
        Self {
            selected_system: None,
        }
    }

    pub fn update(
        &mut self,
        message: SystemSelectWidgetMessage,
    ) -> Task<SystemSelectWidgetMessage> {
        match message {
            SystemSelectWidgetMessage::SystemSelected(system) => {
                self.selected_system = Some(system.clone());
                Task::none()
            }
            SystemSelectWidgetMessage::Reset => {
                self.selected_system = None;
                Task::none()
            }
        }
    }

    pub fn view<'a>(
        &self,
        systems: &'a [SystemListModel],
    ) -> iced::Element<'a, SystemSelectWidgetMessage> {
        let system_select = pick_list(
            systems,
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
