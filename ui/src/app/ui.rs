use std::{cell::OnceCell, sync::Arc};

use crate::tabs::tabs_controller::TabsControllerMessage;
use crate::tabs::title_bar::TitleBarMessage;
use crate::tabs::{
    tabs_controller::TabsController,
    title_bar::{self, TitleBar},
};
use database::{get_db_pool, repository_manager::RepositoryManager};
use iced::widget::{column, text};
use iced::{Element, Task};
use service::error::Error;
use service::view_model_service::ViewModelService;

use super::effect::EffectResponse;

pub struct Ui {
    title_bar: TitleBar,
    tabs_controller: OnceCell<TabsController>,
    repositories: OnceCell<Arc<RepositoryManager>>,
    view_model_service: OnceCell<Arc<ViewModelService>>,
}

#[derive(Debug, Clone)]
pub enum MainMessage {
    // child messages
    TabsController(TabsControllerMessage),
    TitleBar(TitleBarMessage),
    // local messages
    RepositoriesInitialized(Result<Arc<RepositoryManager>, Error>),
    // new after refactor
    EffectResponse(EffectResponse),
}

impl Ui {
    pub fn new() -> (Self, Task<MainMessage>) {
        let initialize_task = Task::perform(
            async {
                match get_db_pool().await {
                    Ok(pool) => {
                        let repositories = Arc::new(RepositoryManager::new(pool));
                        Ok(repositories)
                    }
                    Err(err) => Err(Error::DbError(format!(
                        "Failed connecting to database: {}",
                        err
                    ))),
                }
            },
            MainMessage::RepositoriesInitialized,
        );
        let title_bar = TitleBar::new();
        (
            Self {
                tabs_controller: OnceCell::new(),
                title_bar,
                repositories: OnceCell::new(),
                view_model_service: OnceCell::new(),
            },
            initialize_task,
        )
    }

    pub fn title(&self) -> String {
        "Software Collection Manager".to_string()
    }

    pub fn update(&mut self, message: MainMessage) -> Task<MainMessage> {
        match message {
            MainMessage::EffectResponse(effect) => {
                self.route_effect(effect);
                // Handle any effects here if needed
                Task::none()
            }
            MainMessage::RepositoriesInitialized(result) => match result {
                Ok(repositories) => {
                    let view_model_service =
                        Arc::new(ViewModelService::new(Arc::clone(&repositories)));
                    let (tabs_controller, task) = TabsController::new(
                        None,
                        Arc::clone(&repositories),
                        Arc::clone(&view_model_service),
                    );
                    self.repositories.set(repositories).unwrap_or_else(|_| {
                        panic!("Failed to set repositories, already set?");
                    });
                    self.tabs_controller
                        .set(tabs_controller)
                        .unwrap_or_else(|_| {
                            panic!("Failed to set tabs controller, already set?");
                        });
                    self.view_model_service
                        .set(view_model_service)
                        .unwrap_or_else(|_| {
                            panic!("Failed to set view modelservice, already set?");
                        });
                    task.map(MainMessage::TabsController)
                }
                Err(err) => {
                    eprintln!("Failed connecting to database: {}", err);
                    Task::none()
                }
            },
            MainMessage::TabsController(message) => self
                .tabs_controller
                .get_mut()
                .expect("TabsControler expected to be initialized by now.")
                .update(message)
                .map(MainMessage::TabsController),
            MainMessage::TitleBar(message) => {
                self.title_bar.update(message.clone());
                match message {
                    title_bar::TitleBarMessage::TabSelected(tab) => self
                        .tabs_controller
                        .get_mut()
                        .expect("TabsControler expected to be initialized by now.")
                        .switch_to_tab(tab)
                        .map(MainMessage::TabsController),
                }
            }
        }
    }

    pub fn route_effect(&mut self, effect: EffectResponse) {
        match effect {
            EffectResponse::System(system_effect) => {
                if let Some(tabs_controller) = self.tabs_controller.get_mut() {
                    tabs_controller.handle_system_effect(system_effect);
                }
            } // Handle other effects here if needed
        }
    }

    pub fn view(&self) -> Element<MainMessage> {
        let title_bar_view = self.title_bar.view().map(MainMessage::TitleBar);
        let tab_view = if let Some(tabs_controller) = self.tabs_controller.get() {
            tabs_controller.view().map(MainMessage::TabsController)
        } else {
            text!("Initializing").into()
        };
        column![title_bar_view, tab_view].into()
    }
}
