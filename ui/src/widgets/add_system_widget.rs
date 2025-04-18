use iced::widget::{button, row, text_input};

pub struct AddSystemWidget {
    system_name: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    SystemNameUpdated(String),
    CancelAddSystem,
    Submit,
}

pub enum Action {
    AddSystem(String),
    None,
}

impl AddSystemWidget {
    pub fn new() -> Self {
        Self {
            system_name: "".to_string(),
        }
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::SystemNameUpdated(name) => self.system_name = name,
            Message::Submit => return Action::AddSystem(self.system_name.clone()),
            Message::CancelAddSystem => println!("Cancel"),
        }
        Action::None
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
