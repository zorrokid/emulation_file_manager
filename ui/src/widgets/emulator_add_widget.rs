use iced::{
    widget::{checkbox, text_input},
    Element, Task,
};

use crate::defaults::{DEFAULT_PADDING, DEFAULT_SPACING};

pub struct EmulatorAddWidget {
    emulator_name: String,
    emulator_executable: String,
    emulator_extract_files: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    EmulatorNameChanged(String),
    EmulatorExecutableChanged(String),
    EmulatorExtractFilesChanged(bool),
    Submit,
}

impl EmulatorAddWidget {
    pub fn new() -> Self {
        Self {
            emulator_name: String::new(),
            emulator_executable: String::new(),
            emulator_extract_files: false,
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::EmulatorNameChanged(name) => self.emulator_name = name,
            Message::EmulatorExecutableChanged(executable) => self.emulator_executable = executable,
            Message::EmulatorExtractFilesChanged(extract_files) => {
                self.emulator_extract_files = extract_files
            }
            Message::Submit => {
                // Handle submission logic here
            }
        }
        Task::none()
    }

    pub fn view(&self) -> Element<Message> {
        let name_input =
            text_input("Emulator name", &self.emulator_name).on_input(Message::EmulatorNameChanged);
        let executable_input =
            iced::widget::text_input("Emulator executable", &self.emulator_executable)
                .on_input(Message::EmulatorExecutableChanged);
        let extract_files_checkbox = checkbox("Extract files", self.emulator_extract_files)
            .on_toggle(Message::EmulatorExtractFilesChanged);

        let submit_button = iced::widget::button("Submit").on_press(Message::Submit);

        iced::widget::column![
            name_input,
            executable_input,
            extract_files_checkbox,
            submit_button,
        ]
        .spacing(DEFAULT_SPACING)
        .padding(DEFAULT_PADDING)
        .into()
    }
}
