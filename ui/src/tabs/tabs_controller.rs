use std::sync::Arc;

use database::repository_manager::RepositoryManager;
use iced::Task;
use service::view_model_service::ViewModelService;

use super::{
    add_release_tab::{self, AddReleaseTabMessage},
    emulators_tab::{self, EmulatorsTabMessage},
    home_tab::{self, HomeTabMessage},
    settings_tab::{self, SettingsTabMessage},
};

#[derive(Debug, Clone, PartialEq)]
pub enum Tab {
    Home,
    Settings,
    AddRelease,
    Emulators,
}

#[derive(Debug, Clone)]
pub enum TabsControllerMessage {
    // child messages
    Home(HomeTabMessage),
    Settings(SettingsTabMessage),
    AddRelease(AddReleaseTabMessage),
    Emulators(EmulatorsTabMessage),
    // local messages
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
    ) -> (Self, Task<TabsControllerMessage>) {
        let settings_tab = settings_tab::SettingsTab::new();
        let home_tab = home_tab::HomeTab::new();
        let (add_release_tab, task) = add_release_tab::AddReleaseTab::new(
            Arc::clone(&repositories),
            Arc::clone(&view_model_service),
        );
        let (emulators_tab, emulators_task) =
            emulators_tab::EmulatorsTab::new(repositories, view_model_service);

        let combined_task = Task::batch(vec![
            task.map(TabsControllerMessage::AddRelease),
            emulators_task.map(TabsControllerMessage::Emulators),
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

    pub fn update(&mut self, message: TabsControllerMessage) -> Task<TabsControllerMessage> {
        match message {
            TabsControllerMessage::Home(message) => self
                .home_tab
                .update(message)
                .map(TabsControllerMessage::Home),
            TabsControllerMessage::Settings(message) => self
                .settings_tab
                .update(message)
                .map(TabsControllerMessage::Settings),
            TabsControllerMessage::AddRelease(message) => self
                .add_release_tab
                .update(message)
                .map(TabsControllerMessage::AddRelease),
            TabsControllerMessage::Emulators(message) => self
                .emulators_tab
                .update(message)
                .map(TabsControllerMessage::Emulators),
        }
    }

    pub fn view(&self) -> iced::Element<TabsControllerMessage> {
        match self.current_tab {
            Tab::Home => self.home_tab.view().map(TabsControllerMessage::Home),
            Tab::Settings => self
                .settings_tab
                .view()
                .map(TabsControllerMessage::Settings),
            Tab::AddRelease => self
                .add_release_tab
                .view()
                .map(TabsControllerMessage::AddRelease),
            Tab::Emulators => self
                .emulators_tab
                .view()
                .map(TabsControllerMessage::Emulators),
        }
    }

    pub fn switch_to_tab(&mut self, tab: Tab) -> Task<TabsControllerMessage> {
        self.current_tab = tab;
        Task::none()
    }
}
