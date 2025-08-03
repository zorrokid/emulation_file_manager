use std::{collections::HashMap, sync::Arc};

use crate::{
    emulator_form::{EmulatorFormInit, EmulatorFormModel, EmulatorFormOutputMsg},
    list_item::ListItem,
    utils::{prepare_fileset_for_export, resolve_file_type_path},
};
use core_types::Sha1Checksum;
use database::{
    models::{FileSetFileInfo, System},
    repository_manager::RepositoryManager,
};
use emulator_runner::{error::EmulatorRunnerError, run_with_emulator};
use file_export::{export_files, export_files_zipped, export_files_zipped_or_non_zipped};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller,
    gtk::{
        self,
        glib::clone,
        prelude::{ButtonExt, GtkWindowExt, OrientableExt, WidgetExt},
    },
    typed_view::list::TypedListView,
};
use service::{
    error::Error as ServiceError,
    view_model_service::ViewModelService,
    view_models::{EmulatorListModel, EmulatorViewModel, FileSetViewModel, Settings},
};

#[derive(Debug)]
pub enum EmulatorRunnerMsg {
    FetchEmulators { system_id: i64 },

    // list selection messages
    FileSelected { index: u32 },
    EmulatorSelected { index: u32 },
    SystemSelected { index: u32 },

    OpenEmulatorForm,
    AddEmulator(EmulatorListModel),

    RunEmulator,
}

#[derive(Debug)]
pub enum EmulatorRunnerCommandMsg {
    EmulatorsFetched(Result<Vec<EmulatorViewModel>, ServiceError>),
    FinishedRunningEmulator(Result<(), EmulatorRunnerError>),
}

pub struct EmulatorRunnerInit {
    pub view_model_service: Arc<ViewModelService>,
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
    pub systems: Vec<System>,
    pub file_set: FileSetViewModel,
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
    emulator_form: Option<Controller<EmulatorFormModel>>,

    // data
    emulators: Vec<EmulatorViewModel>,
    systems: Vec<System>,

    // needed for running the emulator:
    settings: Arc<Settings>,
    file_set: FileSetViewModel,
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
                    connect_clicked => EmulatorRunnerMsg::OpenEmulatorForm,

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
        let mut file_list_view_wrapper = TypedListView::<ListItem, gtk::SingleSelection>::new();
        let emulator_list_view_wrapper = TypedListView::<ListItem, gtk::SingleSelection>::new();

        let file_list_items = init
            .file_set
            .files
            .iter()
            .map(|file| ListItem {
                id: file.file_info_id,
                name: file.file_name.clone(),
            })
            .collect::<Vec<_>>();

        file_list_view_wrapper.extend_from_iter(file_list_items);

        let mut system_list_view_wrapper = TypedListView::<ListItem, gtk::SingleSelection>::new();

        let system_list_items = init
            .systems
            .iter()
            .map(|system| ListItem {
                id: system.id,
                name: system.name.clone(),
            })
            .collect::<Vec<_>>();
        system_list_view_wrapper.extend_from_iter(system_list_items);

        let model = EmulatorRunnerModel {
            view_model_service: init.view_model_service,
            repository_manager: init.repository_manager,

            systems: init.systems,
            emulators: Vec::new(),
            settings: init.settings,
            file_set: init.file_set,

            file_list_view_wrapper,
            emulator_list_view_wrapper,
            system_list_view_wrapper,

            selected_file: None,
            selected_emulator: None,
            emulator_form: None,
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
            EmulatorRunnerMsg::RunEmulator => {
                if let (Some(emulator), Some(selected_file), Some(system)) = (
                    self.selected_emulator.clone(),
                    self.selected_file.clone(),
                    self.selected_system.clone(),
                ) {
                    let emulator_system =
                        emulator.systems.iter().find(|s| s.system_id == system.id);

                    if let Some(emulator_system) = emulator_system {
                        let temp_dir = std::env::temp_dir();
                        let export_model = prepare_fileset_for_export(
                            &self.file_set,
                            &self.settings.collection_root_dir,
                            temp_dir.as_path(),
                            emulator.extract_files,
                        );

                        println!("Export model prepared: {:?}", export_model);

                        let files_in_fileset = self
                            .file_set
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
                        let arguments = emulator_system.arguments.clone();

                        sender.oneshot_command(async move {
                            let res = match export_files_zipped_or_non_zipped(&export_model) {
                                Ok(()) => {
                                    run_with_emulator(
                                        executable,
                                        arguments,
                                        files_in_fileset,
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
                    }
                } else {
                    // Handle the case where no emulator or file is selected
                    eprintln!("No emulator or file selected");
                }
            }
            EmulatorRunnerMsg::FileSelected { index } => {
                println!("File selected at index: {}", index);
                let file_list_item = self.file_list_view_wrapper.get(index);
                if let Some(item) = file_list_item {
                    let id = item.borrow().id;
                    let file_info = self.file_set.files.iter().find(|f| f.file_info_id == id);
                    self.selected_file = file_info.cloned();
                }
            }
            EmulatorRunnerMsg::EmulatorSelected { index } => {
                println!("Emulator selected at index: {}", index);
                let emulator_list_item = self.emulator_list_view_wrapper.get(index);
                if let Some(item) = emulator_list_item {
                    let id = item.borrow().id;
                    let emulator = self.emulators.iter().find(|e| e.id == id);
                    self.selected_emulator = emulator.cloned();
                }
            }
            EmulatorRunnerMsg::SystemSelected { index } => {
                println!("System selected at index: {}", index);
                let system_list_item = self.system_list_view_wrapper.get(index);
                if let Some(item) = system_list_item {
                    let id = item.borrow().id;
                    let system = self.systems.iter().find(|s| s.id == id);
                    self.selected_system = system.cloned();
                    sender.input(EmulatorRunnerMsg::FetchEmulators { system_id: id });
                }
            }
            EmulatorRunnerMsg::OpenEmulatorForm => {
                println!("Open Emulator Form");
                let init_model = EmulatorFormInit {
                    view_model_service: Arc::clone(&self.view_model_service),
                    repository_manager: Arc::clone(&self.repository_manager),
                };
                let emulator_form = EmulatorFormModel::builder()
                    .transient_for(root)
                    .launch(init_model)
                    .forward(sender.input_sender(), |msg| match msg {
                        EmulatorFormOutputMsg::EmulatorAdded(emulator_list_model) => {
                            EmulatorRunnerMsg::AddEmulator(emulator_list_model)
                        }
                    });

                self.emulator_form = Some(emulator_form);
                self.emulator_form
                    .as_ref()
                    .expect("Emulator form should be initialized")
                    .widget()
                    .present();
            }
            EmulatorRunnerMsg::AddEmulator(_emulator_list_model) => {
                if let Some(system) = &self.selected_system {
                    sender.input(EmulatorRunnerMsg::FetchEmulators {
                        system_id: system.id,
                    });
                }
            }
            EmulatorRunnerMsg::FetchEmulators { system_id } => {
                println!("Fetching emulators for systems: {:?}", system_id);
                let view_model_service = Arc::clone(&self.view_model_service);
                sender.oneshot_command(async move {
                    let emulators_result = view_model_service
                        .get_emulator_view_models_for_systems(&[system_id])
                        .await;
                    EmulatorRunnerCommandMsg::EmulatorsFetched(emulators_result)
                });
            }
            _ => {}
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        root: &Self::Root,
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
                self.emulator_list_view_wrapper
                    .extend_from_iter(emulator_list_items);
            }
            EmulatorRunnerCommandMsg::EmulatorsFetched(Err(error)) => {
                eprintln!("Error fetching emulators: {:?}", error);
                // TODO: Handle error appropriately, e.g., show a dialog or log the error
            }
            EmulatorRunnerCommandMsg::FinishedRunningEmulator(Ok(())) => {
                println!("Emulator ran successfully");
                root.close();
            }
            EmulatorRunnerCommandMsg::FinishedRunningEmulator(Err(error)) => {
                eprintln!("Error running emulator: {:?}", error);
            }
        }
    }
}
