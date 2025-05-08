use std::sync::Arc;

use database::repository_manager::RepositoryManager;
use iced::widget::{column, row, text, Column};
use iced::Task;
use service::error::Error;
use service::view_model_service::ViewModelService;
use service::view_models::{EmulatorListModel, EmulatorViewModel};

use crate::defaults::{DEFAULT_PADDING, DEFAULT_SPACING};

use super::{
    emulator_add_widget::{self, EmulatorAddWidget},
    emulator_select_widget::{self, EmulatorSelectWidget},
};

pub struct EmulatorsWidget {
    emulators: Vec<EmulatorListModel>,
    selected_emulator: Option<EmulatorViewModel>,
    emulator_add_widget: EmulatorAddWidget,
    emulator_select_widget: EmulatorSelectWidget,
    view_model_service: Arc<ViewModelService>,
}

#[derive(Debug, Clone)]
pub enum Message {
    EmulatorsFetched(Result<Vec<EmulatorListModel>, Error>),
    EmulatorAdd(emulator_add_widget::Message),
    EmulatorSelect(emulator_select_widget::Message),
    SelectedEmulatorLoaded(Result<EmulatorViewModel, Error>),
}

impl EmulatorsWidget {
    pub fn new(
        repositories: Arc<RepositoryManager>,
        view_model_service: Arc<ViewModelService>,
    ) -> (Self, Task<Message>) {
        let view_model_service_clone = Arc::clone(&view_model_service);
        let fech_emulators_task = Task::perform(
            async move { view_model_service_clone.get_emulator_list_models().await },
            Message::EmulatorsFetched,
        );
        (
            Self {
                emulators: vec![],
                selected_emulator: None,
                emulator_add_widget: EmulatorAddWidget::new(repositories),
                emulator_select_widget: EmulatorSelectWidget::new(),
                view_model_service,
            },
            fech_emulators_task,
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::EmulatorsFetched(result) => match result {
                Ok(emulators) => {
                    self.emulators = emulators;
                    return self
                        .emulator_select_widget
                        .update(emulator_select_widget::Message::SetEmulators(
                            self.emulators.clone(),
                        ))
                        .map(Message::EmulatorSelect);
                }
                Err(error) => {
                    eprintln!("Error fetching emulators: {:?}", error);
                }
            },
            Message::EmulatorAdd(msg) => {
                if let emulator_add_widget::Message::EmulatorAdded(emulator_list_model) =
                    msg.clone()
                {
                    println!("Emulator added: {:?}", emulator_list_model);
                    let view_model_service = Arc::clone(&self.view_model_service);
                    let fetch_emulator_task = Task::perform(
                        async move {
                            view_model_service
                                .get_emulator_view_model(emulator_list_model.id)
                                .await
                        },
                        Message::SelectedEmulatorLoaded,
                    );
                    self.emulators.push(emulator_list_model);
                    return fetch_emulator_task;
                }
                println!("Updating emulator add widget with message: {:?}", msg);
                return self
                    .emulator_add_widget
                    .update(msg)
                    .map(Message::EmulatorAdd);
            }
            Message::EmulatorSelect(msg) => {
                let update_task = self
                    .emulator_select_widget
                    .update(msg.clone())
                    .map(Message::EmulatorSelect);
                if let emulator_select_widget::Message::EmulatorSelected(emulator_list_model) = &msg
                {
                    let view_model_service = Arc::clone(&self.view_model_service);
                    let id = emulator_list_model.id;
                    let fetch_emulator_task = Task::perform(
                        async move { view_model_service.get_emulator_view_model(id).await },
                        Message::SelectedEmulatorLoaded,
                    );
                    return Task::batch(vec![fetch_emulator_task, update_task]);
                }
                return update_task;
            }
            Message::SelectedEmulatorLoaded(result) => match result {
                Ok(emulator) => {
                    self.selected_emulator = Some(emulator);
                }
                Err(error) => {
                    eprintln!("Error loading selected emulator: {:?}", error);
                }
            },
        }
        Task::none()
    }

    pub fn view(&self) -> iced::Element<Message> {
        let emulator_add_view = self.emulator_add_widget.view().map(Message::EmulatorAdd);
        let emulator_select_view = self
            .emulator_select_widget
            .view()
            .map(Message::EmulatorSelect);
        column![
            self.create_selected_emulator_view(),
            emulator_add_view,
            emulator_select_view,
        ]
        .into()
    }

    pub fn create_selected_emulator_view(&self) -> iced::Element<Message> {
        let row_content = match &self.selected_emulator {
            Some(emulator) => text!("{}", emulator.name.clone()),
            None => text("No emulator selected"),
        };
        row![row_content]
            .spacing(DEFAULT_SPACING)
            .padding(DEFAULT_PADDING)
            .into()
    }
}
