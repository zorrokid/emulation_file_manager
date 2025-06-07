use std::sync::Arc;

use database::repository_manager::RepositoryManager;
use iced::Task;
use service::view_model_service::ViewModelService;

use super::{
    emulators_tab::{self, EmulatorsTabMessage},
    home_tab::{self, HomeTabMessage},
    releases_tab::{self, ReleasesTabMessage},
    settings_tab::{self, SettingsTabMessage},
};

#[derive(Debug, Clone, PartialEq)]
pub enum Tab {
    Home,
    Settings,
    Releases,
    Emulators,
}

#[derive(Debug, Clone)]
pub enum TabsControllerMessage {
    // child messages
    Home(HomeTabMessage),
    Settings(SettingsTabMessage),
    AddRelease(ReleasesTabMessage),
    Emulators(EmulatorsTabMessage),
    // local messages
}

pub struct TabsController {
    current_tab: Tab,
    home_tab: home_tab::HomeTab,
    settings_tab: settings_tab::SettingsTab,
    releases_tab: releases_tab::ReleasesTab,
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
        let (add_release_tab, task) = releases_tab::ReleasesTab::new(
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
                releases_tab: add_release_tab,
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
                .releases_tab
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
            Tab::Releases => self
                .releases_tab
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
