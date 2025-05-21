use iced::{alignment::Vertical, widget::pick_list};
use service::view_models::EmulatorListModel;

use crate::defaults::{DEFAULT_PADDING, DEFAULT_SPACING};

pub struct EmulatorSelectWidget {
    emulators: Vec<EmulatorListModel>,
    selected_emulator: Option<EmulatorListModel>,
}

#[derive(Debug, Clone)]
pub enum EmulatorSelectWidgetMessage {
    EmulatorSelected(EmulatorListModel),
    SetEmulators(Vec<EmulatorListModel>),
}

impl EmulatorSelectWidget {
    pub fn new() -> Self {
        Self {
            emulators: vec![],
            selected_emulator: None,
        }
    }

    pub fn update(
        &mut self,
        message: EmulatorSelectWidgetMessage,
    ) -> iced::Task<EmulatorSelectWidgetMessage> {
        match message {
            EmulatorSelectWidgetMessage::EmulatorSelected(emulator) => {
                self.selected_emulator = Some(emulator.clone());
                iced::Task::none()
            }
            EmulatorSelectWidgetMessage::SetEmulators(emulators) => {
                self.emulators = emulators;
                self.selected_emulator = None;
                iced::Task::none()
            }
        }
    }

    pub fn view(&self) -> iced::Element<EmulatorSelectWidgetMessage> {
        let emulator_select = pick_list(
            self.emulators.as_slice(),
            self.selected_emulator.clone(),
            EmulatorSelectWidgetMessage::EmulatorSelected,
        );
        let label = iced::widget::text("Select emulator");
        iced::widget::row![label, emulator_select]
            .spacing(DEFAULT_SPACING)
            .padding(DEFAULT_PADDING)
            .align_y(Vertical::Center)
            .into()
    }
}
