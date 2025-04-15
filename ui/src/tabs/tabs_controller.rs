use iced::Task;

use super::{home_tab, settings_tab};

#[derive(Debug, Clone, PartialEq)]
pub enum Tab {
    Home,
    Settings,
}

#[derive(Debug, Clone)]
pub enum Message {
    Home(home_tab::Message),
    Settings(settings_tab::Message),
}

pub struct TabsController {
    current_tab: Tab,
    home_tab: home_tab::HomeTab,
    settings_tab: settings_tab::SettingsTab,
}

impl TabsController {
    pub fn new(selected_tab: Option<Tab>) -> (Self, Task<Message>) {
        let settings_tab = settings_tab::SettingsTab::new();
        let home_tab = home_tab::HomeTab::new();

        (
            Self {
                home_tab,
                settings_tab,
                current_tab: selected_tab.unwrap_or(Tab::Home),
            },
            Task::none(),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Home(message) => self.home_tab.update(message).map(Message::Home),
            Message::Settings(message) => self.settings_tab.update(message).map(Message::Settings),
        }
    }

    pub fn view(&self) -> iced::Element<Message> {
        match self.current_tab {
            Tab::Home => self.home_tab.view().map(Message::Home),
            Tab::Settings => self.settings_tab.view().map(Message::Settings),
        }
    }

    pub fn switch_to_tab(&mut self, tab: Tab) -> Task<Message> {
        self.current_tab = tab;
        Task::none()
    }
}
