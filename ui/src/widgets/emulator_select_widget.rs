use iced::{alignment::Vertical, widget::pick_list};
use service::view_models::EmulatorListModel;

use crate::defaults::{DEFAULT_PADDING, DEFAULT_SPACING};

pub struct EmulatorSelectWidget {
    emulators: Vec<EmulatorListModel>,
    selected_emulator: Option<EmulatorListModel>,
}

#[derive(Debug, Clone)]
pub enum Message {
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

    pub fn update(&mut self, message: Message) -> iced::Task<Message> {
        match message {
            Message::EmulatorSelected(emulator) => {
                self.selected_emulator = Some(emulator.clone());
                iced::Task::none()
            }
            Message::SetEmulators(emulators) => {
                self.emulators = emulators;
                self.selected_emulator = None;
                iced::Task::none()
            }
        }
    }

    pub fn view(&self) -> iced::Element<Message> {
        let emulator_select = pick_list(
            self.emulators.as_slice(),
            self.selected_emulator.clone(),
            Message::EmulatorSelected,
        );
        let label = iced::widget::text("Select emulator");
        iced::widget::row![label, emulator_select]
            .spacing(DEFAULT_SPACING)
            .padding(DEFAULT_PADDING)
            .align_y(Vertical::Center)
            .into()
    }
}
