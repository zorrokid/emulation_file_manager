use std::sync::Arc;

use crate::{
    emulator_form::{EmulatorFormInit, EmulatorFormModel, EmulatorFormMsg, EmulatorFormOutputMsg},
    list_item::ListItem,
    utils::dialog_utils::show_error_dialog,
};
use database::models::System; // TODO: replace with view model
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller,
    gtk::{
        self,
        glib::{self, clone},
        prelude::{ButtonExt, GtkWindowExt, OrientableExt, WidgetExt},
    },
    typed_view::list::TypedListView,
};
use service::{
    error::Error as ServiceError,
    external_executable_runner::service::{ExecutableRunnerModel, ExternalExecutableRunnerService},
    view_models::{
        EmulatorListModel, EmulatorViewModel, FileSetFileInfoViewModel, FileSetViewModel, Settings,
    },
};
use ui_components::confirm_dialog::{
    ConfirmDialog, ConfirmDialogInit, ConfirmDialogMsg, ConfirmDialogOutputMsg,
};

#[derive(Debug)]
pub enum EmulatorRunnerMsg {
    FetchEmulators {
        system_id: i64,
    },

    // list selection messages
    FileSelected {
        index: u32,
    },
    EmulatorSelected {
        index: u32,
    },
    SystemSelected {
        index: u32,
    },

    StartAddEmulator,
    StartEditEmulator,
    DeleteEmulator,
    DeleteConfirmed,
    OpenEmulatorForm {
        editable_emulator: Option<EmulatorViewModel>,
    },
    AddEmulator(EmulatorListModel),
    UpdateEmulator(EmulatorListModel),

    Show {
        file_set: FileSetViewModel,
        systems: Vec<System>,
    },
    Hide,
    Ignore,
    StartEmulator,
}

#[derive(Debug)]
pub enum EmulatorRunnerCommandMsg {
    EmulatorsFetched(Result<Vec<EmulatorViewModel>, ServiceError>),
    FinishedRunningEmulator(Result<(), ServiceError>),
    EmulatorDeleted(Result<i64, ServiceError>),
}

pub struct EmulatorRunnerInit {
    pub app_services: Arc<service::app_services::AppServices>,
    pub settings: Arc<Settings>,
}

#[derive(Debug)]
pub struct EmulatorRunnerModel {
    // services
    app_services: Arc<service::app_services::AppServices>,

    // list views
    file_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    emulator_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    system_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,

    // controllers
    emulator_form: Controller<EmulatorFormModel>,
    confirm_dialog_controller: Controller<ConfirmDialog>,

    // data
    emulators: Vec<EmulatorViewModel>,
    systems: Vec<System>,

    // needed for running the emulator:
    file_set: Option<FileSetViewModel>,
    selected_file: Option<FileSetFileInfoViewModel>,
    selected_system: Option<System>,
    selected_emulator: Option<EmulatorViewModel>,
}

#[relm4::component(pub)]
impl Component for EmulatorRunnerModel {
    type Input = EmulatorRunnerMsg;
    type Output = ();
    type CommandOutput = EmulatorRunnerCommandMsg;
    type Init = EmulatorRunnerInit;

    view! {
        gtk::Window {
            connect_close_request[sender] => move |_| {
                sender.input(EmulatorRunnerMsg::Hide);
                glib::Propagation::Proceed
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                #[local_ref]
                file_list_view -> gtk::ListView,

                #[local_ref]
                system_list_view -> gtk::ListView,

                #[local_ref]
                emulator_list_view -> gtk::ListView,

                gtk::Button {
                    set_label: "Add emulator",
                    connect_clicked => EmulatorRunnerMsg::StartAddEmulator,
                },

                gtk::Button {
                    set_label: "Edit emulator",
                    connect_clicked => EmulatorRunnerMsg::StartEditEmulator,
                    #[watch]
                    set_sensitive: model.selected_emulator.is_some()
                },

                gtk::Button {
                    set_label: "Delete emulator",
                    connect_clicked => EmulatorRunnerMsg::DeleteEmulator,
                    #[watch]
                    set_sensitive: model.selected_emulator.is_some()
                },

                gtk::Button {
                    set_label: "Run Emulator",
                    connect_clicked => EmulatorRunnerMsg::StartEmulator,
                    #[watch]
                    set_sensitive: model.selected_emulator.is_some() && model.selected_file.is_some() && model.file_set.is_some(),
                },
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let file_list_view_wrapper = TypedListView::<ListItem, gtk::SingleSelection>::new();
        let emulator_list_view_wrapper = TypedListView::<ListItem, gtk::SingleSelection>::new();
        let system_list_view_wrapper = TypedListView::<ListItem, gtk::SingleSelection>::new();

        let init_model = EmulatorFormInit {
            app_services: Arc::clone(&init.app_services),
        };

        let emulator_form = EmulatorFormModel::builder()
            .transient_for(&root)
            .launch(init_model)
            .forward(sender.input_sender(), |msg| match msg {
                EmulatorFormOutputMsg::EmulatorAdded(emulator_list_model) => {
                    EmulatorRunnerMsg::AddEmulator(emulator_list_model)
                }
                EmulatorFormOutputMsg::EmulatorUpdated(emulator_list_model) => {
                    EmulatorRunnerMsg::UpdateEmulator(emulator_list_model)
                }
            });

        let confirm_dialog_controller = ConfirmDialog::builder()
            .transient_for(&root)
            .launch(ConfirmDialogInit {
                title: "Confirm Deletion".to_string(),
                visible: false,
            })
            .forward(sender.input_sender(), |msg| match msg {
                ConfirmDialogOutputMsg::Confirmed => EmulatorRunnerMsg::DeleteConfirmed,
                ConfirmDialogOutputMsg::Canceled => EmulatorRunnerMsg::Ignore,
            });

        let model = EmulatorRunnerModel {
            app_services: init.app_services,

            systems: Vec::new(),
            emulators: Vec::new(),
            file_set: None,

            file_list_view_wrapper,
            emulator_list_view_wrapper,
            system_list_view_wrapper,

            selected_file: None,
            selected_emulator: None,
            emulator_form,
            confirm_dialog_controller,
            selected_system: None,
        };

        let file_list_view = &model.file_list_view_wrapper.view;
        let emulator_list_view = &model.emulator_list_view_wrapper.view;
        let system_list_view = &model.system_list_view_wrapper.view;

        model
            .file_list_view_wrapper
            .selection_model
            .connect_selected_notify(clone!(
                #[strong]
                sender,
                move |selection| {
                    let selected = selection.selected();
                    sender.input(EmulatorRunnerMsg::FileSelected { index: selected });
                }
            ));

        model
            .emulator_list_view_wrapper
            .selection_model
            .connect_selected_notify(clone!(
                #[strong]
                sender,
                move |selection| {
                    let selected = selection.selected();
                    sender.input(EmulatorRunnerMsg::EmulatorSelected { index: selected });
                }
            ));

        model
            .system_list_view_wrapper
            .selection_model
            .connect_selected_notify(clone!(
                #[strong]
                sender,
                move |selection| {
                    let selected = selection.selected();
                    sender.input(EmulatorRunnerMsg::SystemSelected { index: selected });
                }
            ));

        let widgets = view_output!();
        sender.input(EmulatorRunnerMsg::SystemSelected { index: 0 });
        sender.input(EmulatorRunnerMsg::FileSelected { index: 0 });
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            EmulatorRunnerMsg::FileSelected { index } => {
                self.handle_file_selection(index);
            }
            EmulatorRunnerMsg::EmulatorSelected { index } => {
                self.handle_emulator_selection(index);
            }
            EmulatorRunnerMsg::SystemSelected { index } => {
                self.handle_system_selection(index, &sender);
            }
            EmulatorRunnerMsg::StartAddEmulator => {
                sender.input(EmulatorRunnerMsg::OpenEmulatorForm {
                    editable_emulator: None,
                });
            }
            EmulatorRunnerMsg::StartEditEmulator => {
                if let Some(selected_emulator) = &self.selected_emulator {
                    tracing::info!(id = selected_emulator.id, "Starting edit emulator");
                    sender.input(EmulatorRunnerMsg::OpenEmulatorForm {
                        editable_emulator: self.selected_emulator.clone(),
                    });
                }
            }
            EmulatorRunnerMsg::OpenEmulatorForm { editable_emulator } => {
                self.emulator_form
                    .emit(EmulatorFormMsg::Show { editable_emulator });
            }
            EmulatorRunnerMsg::AddEmulator(_emulator_list_model) => {
                if let Some(system) = &self.selected_system {
                    sender.input(EmulatorRunnerMsg::FetchEmulators {
                        system_id: system.id,
                    });
                }
            }
            EmulatorRunnerMsg::UpdateEmulator(_emulator_list_model) => {
                if let Some(system) = &self.selected_system {
                    sender.input(EmulatorRunnerMsg::FetchEmulators {
                        system_id: system.id,
                    });
                }
            }
            EmulatorRunnerMsg::FetchEmulators { system_id } => {
                let app_services = Arc::clone(&self.app_services);
                sender.oneshot_command(async move {
                    let emulators_result = app_services
                        .view_model
                        .get_emulator_view_models_for_systems(&[system_id])
                        .await;
                    EmulatorRunnerCommandMsg::EmulatorsFetched(emulators_result)
                });
            }
            EmulatorRunnerMsg::Show { file_set, systems } => {
                self.init_with_new_data(file_set, systems, &sender);
                root.show();
            }
            EmulatorRunnerMsg::Hide => {
                root.hide();
            }
            EmulatorRunnerMsg::DeleteEmulator => {
                self.confirm_dialog_controller.emit(ConfirmDialogMsg::Show);
            }
            EmulatorRunnerMsg::DeleteConfirmed => {
                if let Some(selected_emulator) = &self.selected_emulator {
                    let emulator_id = selected_emulator.id;
                    let app_services = Arc::clone(&self.app_services);
                    sender.oneshot_command(async move {
                        let res = app_services.emulator.delete_emulator(emulator_id).await;
                        EmulatorRunnerCommandMsg::EmulatorDeleted(res)
                    });
                }
            }
            EmulatorRunnerMsg::StartEmulator => {
                self.start_emulator(&sender);
            }
            _ => {}
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            EmulatorRunnerCommandMsg::EmulatorsFetched(Ok(emulator_view_models)) => {
                tracing::info!("Emulators fetched successfully");
                let emulator_list_items = emulator_view_models
                    .iter()
                    .map(|emulator| ListItem {
                        id: emulator.id,
                        name: emulator.name.clone(),
                    })
                    .collect::<Vec<_>>();
                self.emulators = emulator_view_models;
                self.emulator_list_view_wrapper.clear();
                self.emulator_list_view_wrapper
                    .extend_from_iter(emulator_list_items);
            }
            EmulatorRunnerCommandMsg::EmulatorsFetched(Err(error)) => {
                tracing::error!(
                    error = ?error,
                    "Error fetching emulators"
                );
                show_error_dialog(format!("Error Fetching Emulators {:?}", error), root);
            }
            EmulatorRunnerCommandMsg::FinishedRunningEmulator(Ok(())) => {
                tracing::info!("Emulator executed successfully");
                sender.input(EmulatorRunnerMsg::Hide);
            }
            EmulatorRunnerCommandMsg::FinishedRunningEmulator(Err(error)) => {
                show_error_dialog(format!("Error running emulator: {:?}", error), root);
            }
            EmulatorRunnerCommandMsg::EmulatorDeleted(Ok(deleted_id)) => {
                tracing::info!(id = deleted_id, "Emulator deleted successfully");
                // TODO: instead of fetching maybe just remove from the list
                if let Some(system) = &self.selected_system {
                    sender.input(EmulatorRunnerMsg::FetchEmulators {
                        system_id: system.id,
                    });
                }
            }
            EmulatorRunnerCommandMsg::EmulatorDeleted(Err(error)) => {
                show_error_dialog(format!("Error deleting emulator: {:?}", error), root);
            }
        }
    }
}

impl EmulatorRunnerModel {
    pub fn handle_file_selection(&mut self, index: u32) {
        let file_list_item = self.file_list_view_wrapper.get(index);
        if let (Some(item), Some(file_set)) = (file_list_item, &self.file_set) {
            let id = item.borrow().id;
            let file_info = file_set.files.iter().find(|f| f.file_info_id == id);
            self.selected_file = file_info.cloned();
        }
    }
    pub fn handle_emulator_selection(&mut self, index: u32) {
        let emulator_list_item = self.emulator_list_view_wrapper.get(index);
        if let Some(item) = emulator_list_item {
            let id = item.borrow().id;
            let emulator = self.emulators.iter().find(|e| e.id == id);
            self.selected_emulator = emulator.cloned();
        }
    }
    pub fn handle_system_selection(&mut self, index: u32, sender: &ComponentSender<Self>) {
        let system_list_item = self.system_list_view_wrapper.get(index);
        if let Some(item) = system_list_item {
            let id = item.borrow().id;
            let system = self.systems.iter().find(|s| s.id == id);
            self.selected_system = system.cloned();
            if let Some(system) = system {
                sender.input(EmulatorRunnerMsg::FetchEmulators {
                    system_id: system.id,
                });
            }
        }
    }

    pub fn start_emulator(&self, sender: &ComponentSender<Self>) {
        if let (Some(emulator), Some(selected_file), Some(file_set)) =
            (&self.selected_emulator, &self.selected_file, &self.file_set)
        {
            let executable = emulator.executable.clone();
            let arguments = emulator.arguments.clone();

            let starting_file = if emulator.extract_files {
                selected_file.file_name.clone()
            } else {
                file_set.file_set_name.clone()
            };

            let executable_runner_service = self.app_services.runner.clone();

            let executable_runner_model = ExecutableRunnerModel {
                executable,
                arguments,
                extract_files: emulator.extract_files,
                file_set_id: file_set.id,
                initial_file: Some(starting_file.clone()),
                // Emulators block until closed, cleanup after
                // TODO: make this configurable
                skip_cleanup: false,
            };

            sender.oneshot_command(async move {
                let res = executable_runner_service
                    .run_executable(executable_runner_model, None)
                    .await;
                EmulatorRunnerCommandMsg::FinishedRunningEmulator(res)
            });
        }
    }

    pub fn init_with_new_data(
        &mut self,
        file_set: FileSetViewModel,
        systems: Vec<System>,
        sender: &ComponentSender<Self>,
    ) {
        let file_list_items = file_set
            .files
            .iter()
            .map(|file| ListItem {
                id: file.file_info_id,
                name: file.file_name.clone(),
            })
            .collect::<Vec<_>>();

        self.file_list_view_wrapper.clear();
        self.file_list_view_wrapper
            .extend_from_iter(file_list_items);

        sender.input(EmulatorRunnerMsg::FileSelected {
            index: self.file_list_view_wrapper.selection_model.selected(),
        });

        let system_list_items = systems
            .iter()
            .map(|system| ListItem {
                id: system.id,
                name: system.name.clone(),
            })
            .collect::<Vec<_>>();

        self.system_list_view_wrapper.clear();
        self.system_list_view_wrapper
            .extend_from_iter(system_list_items);

        sender.input(EmulatorRunnerMsg::FetchEmulators {
            system_id: systems.first().map_or(0, |s| s.id),
        });

        self.systems = systems;
        self.file_set = Some(file_set);
    }
}
