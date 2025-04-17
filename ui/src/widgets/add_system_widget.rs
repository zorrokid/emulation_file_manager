use iced::{
    widget::{button, row, text_input},
    Task,
};

pub struct AddSystemWidget {
    system_name: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    SystemNameUpdated(String),
    CancelAddSystem,
    Submit,
}

impl AddSystemWidget {
    pub fn new() -> Self {
        Self {
            system_name: "".to_string(),
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SystemNameUpdated(name) => self.system_name = name,
            Message::Submit => println!("Submit"),
            Message::CancelAddSystem => println!("Cancel"),
        }
        Task::none()
    }

    pub fn view(&self) -> iced::Element<Message> {
        let name_input =
            text_input("System name", &self.system_name).on_input(Message::SystemNameUpdated);

        let submit_button = button("Submit system")
            .on_press_maybe((!self.system_name.is_empty()).then_some(Message::Submit));
        let cancel_button = button("Cancel").on_press(Message::CancelAddSystem);
        row![name_input, submit_button, cancel_button].into()
    }
}
