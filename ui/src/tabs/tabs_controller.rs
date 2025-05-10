use std::sync::Arc;

use database::repository_manager::RepositoryManager;
use iced::Task;
use service::view_model_service::ViewModelService;

use super::{add_release_tab, emulators_tab, home_tab, settings_tab};

#[derive(Debug, Clone, PartialEq)]
pub enum Tab {
    Home,
    Settings,
    AddRelease,
    Emulators,
}

#[derive(Debug, Clone)]
pub enum Message {
    Home(home_tab::Message),
    Settings(settings_tab::Message),
    AddRelease(add_release_tab::Message),
    Emulators(emulators_tab::Message),
}

pub struct TabsController {
    current_tab: Tab,
    home_tab: home_tab::HomeTab,
    settings_tab: settings_tab::SettingsTab,
    add_release_tab: add_release_tab::AddReleaseTab,
    emulators_tab: emulators_tab::EmulatorsTab,
}

impl TabsController {
    pub fn new(
        selected_tab: Option<Tab>,
        repositories: Arc<RepositoryManager>,
        view_model_service: Arc<ViewModelService>,
    ) -> (Self, Task<Message>) {
        let settings_tab = settings_tab::SettingsTab::new();
        let home_tab = home_tab::HomeTab::new();
        let (add_release_tab, task) = add_release_tab::AddReleaseTab::new(
            Arc::clone(&repositories),
            Arc::clone(&view_model_service),
        );
        let (emulators_tab, emulators_task) =
            emulators_tab::EmulatorsTab::new(repositories, view_model_service);

        let combined_task = Task::batch(vec![
            task.map(Message::AddRelease),
            emulators_task.map(Message::Emulators),
        ]);

        (
            Self {
                home_tab,
                settings_tab,
                current_tab: selected_tab.unwrap_or(Tab::Home),
                add_release_tab,
                emulators_tab,
            },
            combined_task,
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Home(message) => self.home_tab.update(message).map(Message::Home),
            Message::Settings(message) => self.settings_tab.update(message).map(Message::Settings),
            Message::AddRelease(message) => self
                .add_release_tab
                .update(message)
                .map(Message::AddRelease),
            Message::Emulators(message) => {
                self.emulators_tab.update(message).map(Message::Emulators)
            }
        }
    }

    pub fn view(&self) -> iced::Element<Message> {
        match self.current_tab {
            Tab::Home => self.home_tab.view().map(Message::Home),
            Tab::Settings => self.settings_tab.view().map(Message::Settings),
            Tab::AddRelease => self.add_release_tab.view().map(Message::AddRelease),
            Tab::Emulators => self.emulators_tab.view().map(Message::Emulators),
        }
    }

    pub fn switch_to_tab(&mut self, tab: Tab) -> Task<Message> {
        self.current_tab = tab;
        Task::none()
    }
}
