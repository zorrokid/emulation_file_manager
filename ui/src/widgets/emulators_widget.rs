use database::database_error::Error;
use iced::widget::column;
use iced::Task;
use service::view_models::EmulatorListModel;

use super::{
    emulator_add_widget::{self, EmulatorAddWidget},
    emulator_select_widget::{self, EmulatorSelectWidget},
};

pub struct EmulatorsWidget {
    emulators: Vec<EmulatorListModel>,
    selected_emulator: Option<i64>,
    emulator_add_widget: EmulatorAddWidget,
    emulator_select_widget: EmulatorSelectWidget,
}

#[derive(Debug, Clone)]
pub enum Message {
    EmulatorsFetched(Result<Vec<EmulatorListModel>, Error>),
    EmulatorAdd(emulator_add_widget::Message),
    EmulatorSelect(emulator_select_widget::Message),
    EmulatorSelected(i64),
}

impl EmulatorsWidget {
    pub fn new() -> Self {
        Self {
            emulators: vec![],
            selected_emulator: None,
            emulator_add_widget: EmulatorAddWidget::new(),
            emulator_select_widget: EmulatorSelectWidget::new(),
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::EmulatorsFetched(result) => match result {
                Ok(emulators) => {
                    self.emulators = emulators;
                    return self
                        .emulator_select_widget
                        .update(emulator_select_widget::Message::SetEmulators(
                            self.emulators.clone(),
                        ))
                        .map(Message::EmulatorSelect);
                }
                Err(error) => {
                    eprintln!("Error fetching emulators: {:?}", error);
                }
            },
            Message::EmulatorAdd(msg) => {
                return self
                    .emulator_add_widget
                    .update(msg)
                    .map(Message::EmulatorAdd)
            }
            Message::EmulatorSelect(msg) => {
                return self
                    .emulator_select_widget
                    .update(msg)
                    .map(Message::EmulatorSelect);
            }
            Message::EmulatorSelected(id) => {
                self.selected_emulator = Some(id);
            }
        }
        Task::none()
    }

    pub fn view(&self) -> iced::Element<Message> {
        let emulator_add_view = self.emulator_add_widget.view().map(Message::EmulatorAdd);
        let emulator_select_view = self
            .emulator_select_widget
            .view()
            .map(Message::EmulatorSelect);
        column![emulator_add_view, emulator_select_view].into()
    }
}
