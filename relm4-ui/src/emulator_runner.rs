use std::sync::Arc;

use crate::{
    emulator_form::{EmulatorFormInit, EmulatorFormModel, EmulatorFormMsg, EmulatorFormOutputMsg},
    list_item::ListItem,
};
use database::{
    database_error::Error,
    models::{FileSetFileInfo, System},
    repository_manager::RepositoryManager,
};
use emulator_runner::{error::EmulatorRunnerError, run_with_emulator};
use file_export::export_files_zipped_or_non_zipped;
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
    export_service::prepare_fileset_for_export,
    view_model_service::ViewModelService,
    view_models::{EmulatorListModel, EmulatorViewModel, FileSetViewModel, Settings},
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

    RunEmulator,

    Show {
        file_set: FileSetViewModel,
        systems: Vec<System>,
    },
    Hide,
    Ignore,
}

#[derive(Debug)]
pub enum EmulatorRunnerCommandMsg {
    EmulatorsFetched(Result<Vec<EmulatorViewModel>, ServiceError>),
    FinishedRunningEmulator(Result<(), EmulatorRunnerError>),
    EmulatorDeleted(Result<i64, Error>),
}

pub struct EmulatorRunnerInit {
    pub view_model_service: Arc<ViewModelService>,
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
}

#[derive(Debug)]
pub struct EmulatorRunnerModel {
    // services
    view_model_service: Arc<ViewModelService>,
    repository_manager: Arc<RepositoryManager>,

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
    settings: Arc<Settings>,
    file_set: Option<FileSetViewModel>,
    selected_file: Option<FileSetFileInfo>,
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
                    connect_clicked => EmulatorRunnerMsg::RunEmulator,
                    #[watch]
                    set_sensitive: model.selected_emulator.is_some() && model.selected_file.is_some(),
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
            view_model_service: Arc::clone(&init.view_model_service),
            repository_manager: Arc::clone(&init.repository_manager),
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
            view_model_service: init.view_model_service,
            repository_manager: init.repository_manager,

            systems: Vec::new(),
            emulators: Vec::new(),
            settings: init.settings,
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
            EmulatorRunnerMsg::RunEmulator => self.run_emulator(&sender),
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
                    println!("Editing Emulator: {}", selected_emulator.name);
                    sender.input(EmulatorRunnerMsg::OpenEmulatorForm {
                        editable_emulator: self.selected_emulator.clone(),
                    });
                } else {
                    eprintln!("No emulator selected for editing");
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
                let view_model_service = Arc::clone(&self.view_model_service);
                sender.oneshot_command(async move {
                    let emulators_result = view_model_service
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
                    let repository_manager = Arc::clone(&self.repository_manager);
                    sender.oneshot_command(async move {
                        let res = repository_manager
                            .get_emulator_repository()
                            .delete_emulator(emulator_id)
                            .await;
                        EmulatorRunnerCommandMsg::EmulatorDeleted(res)
                    });
                }
            }
            _ => {}
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            EmulatorRunnerCommandMsg::EmulatorsFetched(Ok(emulator_view_models)) => {
                println!("Emulators fetched successfully: {:?}", emulator_view_models);
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
                eprintln!("Error fetching emulators: {:?}", error);
                // TODO: Handle error appropriately, e.g., show a dialog or log the error
            }
            EmulatorRunnerCommandMsg::FinishedRunningEmulator(Ok(())) => {
                println!("Emulator ran successfully");
                sender.input(EmulatorRunnerMsg::Hide);
            }
            EmulatorRunnerCommandMsg::FinishedRunningEmulator(Err(error)) => {
                eprintln!("Error running emulator: {:?}", error);
            }
            EmulatorRunnerCommandMsg::EmulatorDeleted(Ok(deleted_id)) => {
                println!("Emulator with ID {} deleted successfully", deleted_id);
                // TODO: instead of fetching maybe just remove from the list
                if let Some(system) = &self.selected_system {
                    sender.input(EmulatorRunnerMsg::FetchEmulators {
                        system_id: system.id,
                    });
                }
            }
            EmulatorRunnerCommandMsg::EmulatorDeleted(Err(error)) => {
                eprintln!("Error deleting emulator: {:?}", error);
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
    pub fn run_emulator(&self, sender: &ComponentSender<Self>) {
        if let (Some(emulator), Some(selected_file), Some(file_set)) = (
            self.selected_emulator.clone(),
            self.selected_file.clone(),
            self.file_set.clone(),
        ) {
            let temp_dir = std::env::temp_dir();
            let export_model = prepare_fileset_for_export(
                &file_set,
                &self.settings.collection_root_dir,
                temp_dir.as_path(),
                emulator.extract_files,
            );

            println!("Export model prepared: {:?}", export_model);

            let files_in_fileset = file_set
                .files
                .iter()
                .map(|f| f.file_name.clone())
                .collect::<Vec<_>>();

            let extract_files = emulator.extract_files;
            println!(
                "Extract files: {}, Files in fileset: {:?}",
                extract_files, files_in_fileset
            );
            let starting_file = if extract_files {
                selected_file.file_name.clone()
            } else {
                export_model.exported_zip_file_name.clone()
            };

            let executable = emulator.executable.clone();
            let arguments = emulator.arguments.clone();

            sender.oneshot_command(async move {
                let res = match export_files_zipped_or_non_zipped(&export_model) {
                    Ok(()) => {
                        run_with_emulator(
                            executable,
                            &arguments,
                            &files_in_fileset,
                            starting_file,
                            temp_dir,
                        )
                        .await
                    }
                    Err(e) => Err(EmulatorRunnerError::IoError(format!(
                        "Failed to export files: {}",
                        e
                    ))),
                };
                EmulatorRunnerCommandMsg::FinishedRunningEmulator(res)
            });
        } else {
            // Handle the case where no emulator or file is selected
            eprintln!("No emulator or file selected");
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
