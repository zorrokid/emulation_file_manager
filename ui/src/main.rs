mod tabs;

use iced::widget::column;
use iced::Task;
use tabs::{
    tabs_controller::TabsController,
    title_bar::{self, TitleBar},
};

fn main() -> iced::Result {
    iced::application(Ui::title, Ui::update, Ui::view).run_with(Ui::new)
}

struct Ui {
    title_bar: TitleBar,
    tabs_controller: TabsController,
}

#[derive(Debug, Clone)]
enum Message {
    TabsController(tabs::tabs_controller::Message),
    TitleBar(tabs::title_bar::Message),
}

impl Ui {
    fn new() -> (Self, Task<Message>) {
        let (tabs_controller, task) = TabsController::new(None);
        let title_bar = TitleBar::new();
        (
            Self {
                tabs_controller,
                title_bar,
            },
            task.map(Message::TabsController),
        )
    }

    fn title(&self) -> String {
        "Software Collection Manager".to_string()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TabsController(message) => self
                .tabs_controller
                .update(message)
                .map(Message::TabsController),
            Message::TitleBar(message) => {
                self.title_bar.update(message.clone());
                match message {
                    title_bar::Message::TabSelected(tab) => self
                        .tabs_controller
                        .switch_to_tab(tab)
                        .map(Message::TabsController),
                }
            }
        }
    }

    fn view(&self) -> iced::Element<Message> {
        let title_bar_view = self.title_bar.view().map(Message::TitleBar);
        let tab_view = self.tabs_controller.view().map(Message::TabsController);
        column![title_bar_view, tab_view].into()
    }
}
