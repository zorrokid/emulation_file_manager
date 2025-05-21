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
    HomeTab(HomeTabMessage),
    SettingsTab(SettingsTabMessage),
    AddReleaseTab(AddReleaseTabMessage),
    EmulatorsTab(EmulatorsTabMessage),
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
            task.map(TabsControllerMessage::AddReleaseTab),
            emulators_task.map(TabsControllerMessage::EmulatorsTab),
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
            TabsControllerMessage::HomeTab(message) => self
                .home_tab
                .update(message)
                .map(TabsControllerMessage::HomeTab),
            TabsControllerMessage::SettingsTab(message) => self
                .settings_tab
                .update(message)
                .map(TabsControllerMessage::SettingsTab),
            TabsControllerMessage::AddReleaseTab(message) => self
                .add_release_tab
                .update(message)
                .map(TabsControllerMessage::AddReleaseTab),
            TabsControllerMessage::EmulatorsTab(message) => self
                .emulators_tab
                .update(message)
                .map(TabsControllerMessage::EmulatorsTab),
        }
    }

    pub fn view(&self) -> iced::Element<TabsControllerMessage> {
        match self.current_tab {
            Tab::Home => self.home_tab.view().map(TabsControllerMessage::HomeTab),
            Tab::Settings => self
                .settings_tab
                .view()
                .map(TabsControllerMessage::SettingsTab),
            Tab::AddRelease => self
                .add_release_tab
                .view()
                .map(TabsControllerMessage::AddReleaseTab),
            Tab::Emulators => self
                .emulators_tab
                .view()
                .map(TabsControllerMessage::EmulatorsTab),
        }
    }

    pub fn switch_to_tab(&mut self, tab: Tab) -> Task<TabsControllerMessage> {
        self.current_tab = tab;
        Task::none()
    }
}
