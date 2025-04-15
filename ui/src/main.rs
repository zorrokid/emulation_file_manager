use iced::{widget::text, Task};

fn main() -> iced::Result {
    iced::application(Ui::title, Ui::update, Ui::view).run_with(Ui::new)
}

struct Ui {}

#[derive(Debug, Clone)]
enum Message {}

impl Ui {
    fn new() -> (Self, Task<Message>) {
        (Self {}, iced::Task::none())
    }

    fn title(&self) -> String {
        "My Application".to_string()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        iced::Task::none()
    }

    fn view(&self) -> iced::Element<Message> {
        text!("TODO").into()
    }
}
