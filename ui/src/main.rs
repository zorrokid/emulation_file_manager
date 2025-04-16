mod error;
mod tabs;

use std::{cell::OnceCell, sync::Arc};

use database::{get_db_pool, repository_manager::RepositoryManager};
use error::Error;
use iced::widget::{column, text};
use iced::Task;
use service::view_model_service::ViewModelService;
use tabs::{
    tabs_controller::TabsController,
    title_bar::{self, TitleBar},
};

fn main() -> iced::Result {
    iced::application(Ui::title, Ui::update, Ui::view).run_with(Ui::new)
}

struct Ui {
    title_bar: TitleBar,
    tabs_controller: OnceCell<TabsController>,
    repositories: OnceCell<Arc<RepositoryManager>>,
    view_model_service: OnceCell<Arc<ViewModelService>>,
}

#[derive(Debug, Clone)]
enum Message {
    TabsController(tabs::tabs_controller::Message),
    TitleBar(tabs::title_bar::Message),
    RepositoriesInitialized(Result<Arc<RepositoryManager>, Error>),
}

impl Ui {
    fn new() -> (Self, Task<Message>) {
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
            Message::RepositoriesInitialized,
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

    fn title(&self) -> String {
        "Software Collection Manager".to_string()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::RepositoriesInitialized(result) => match result {
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
                    task.map(Message::TabsController)
                }
                Err(err) => {
                    eprintln!("Failed connecting to database: {}", err);
                    Task::none()
                }
            },
            Message::TabsController(message) => self
                .tabs_controller
                .get_mut()
                .expect("TabsControler expected to be initialized by now.")
                .update(message)
                .map(Message::TabsController),
            Message::TitleBar(message) => {
                self.title_bar.update(message.clone());
                match message {
                    title_bar::Message::TabSelected(tab) => self
                        .tabs_controller
                        .get_mut()
                        .expect("TabsControler expected to be initialized by now.")
                        .switch_to_tab(tab)
                        .map(Message::TabsController),
                }
            }
        }
    }

    fn view(&self) -> iced::Element<Message> {
        let title_bar_view = self.title_bar.view().map(Message::TitleBar);
        let tab_view = if let Some(tabs_controller) = self.tabs_controller.get() {
            tabs_controller.view().map(Message::TabsController)
        } else {
            text!("Initializing").into()
        };
        column![title_bar_view, tab_view].into()
    }
}
