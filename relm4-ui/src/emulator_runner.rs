use std::sync::Arc;

use crate::{
    emulator_form::{EmulatorFormInit, EmulatorFormModel, EmulatorFormOutputMsg},
    list_item::ListItem,
};
use database::{
    models::{FileInfo, FileSetFileInfo},
    repository_manager::RepositoryManager,
};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller,
    gtk::{
        self,
        glib::clone,
        prelude::{ButtonExt, GtkWindowExt, OrientableExt},
    },
    typed_view::list::TypedListView,
};
use service::{
    error::Error as ServiceError,
    view_model_service::ViewModelService,
    view_models::{
        EmulatorListModel, EmulatorViewModel, FileSetListModel, FileSetViewModel, Settings,
    },
};

#[derive(Debug)]
pub enum EmulatorRunnerMsg {
    FetchFileSets,
    RunEmulator,
    FileSelected { index: u32 },
    EmulatorSelected { index: u32 },
    OpenEmulatorForm,
    FetchEmulators,
    AddEmulator(EmulatorListModel),
}

#[derive(Debug)]
pub enum EmulatorRunnerOutputMsg {
    EmulatorAndStartFileSelected {
        emulator: EmulatorViewModel,
        file_info: FileInfo,
    },
}

#[derive(Debug)]
pub enum EmulatorRunnerCommandMsg {
    FileSetsFetched(Result<Vec<FileSetListModel>, ServiceError>),
    EmulatorsFetched(Result<Vec<EmulatorViewModel>, ServiceError>),
}

pub struct EmulatorRunnerInit {
    pub view_model_service: Arc<ViewModelService>,
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
    pub system_ids: Vec<i64>,
    pub file_set: FileSetViewModel,
}

#[derive(Debug)]
pub struct EmulatorRunnerModel {
    view_model_service: Arc<ViewModelService>,
    repository_manager: Arc<RepositoryManager>,
    settings: Arc<Settings>,
    file_set: FileSetViewModel,
    selected_file: Option<FileSetFileInfo>,
    file_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    emulator_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    system_ids: Vec<i64>,
    emulators: Vec<EmulatorViewModel>,
    selected_emulator: Option<EmulatorViewModel>,
    emulator_form: Option<Controller<EmulatorFormModel>>,
}

#[relm4::component(pub)]
impl Component for EmulatorRunnerModel {
    type Input = EmulatorRunnerMsg;
    type Output = EmulatorRunnerOutputMsg;
    type CommandOutput = EmulatorRunnerCommandMsg;
    type Init = EmulatorRunnerInit;

    view! {
        gtk::Window {
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                #[local_ref]
                file_list_view -> gtk::ListView,

                #[local_ref]
                emulator_list_view -> gtk::ListView,

                gtk::Button {
                    set_label: "Add emulator",
                    connect_clicked => EmulatorRunnerMsg::OpenEmulatorForm,
                },

                gtk::Button {
                    set_label: "Run Emulator",
                    connect_clicked => EmulatorRunnerMsg::RunEmulator,
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

        let model = EmulatorRunnerModel {
            view_model_service: init.view_model_service,
            repository_manager: init.repository_manager,
            settings: init.settings,
            file_set: init.file_set,
            selected_file: None,
            file_list_view_wrapper,
            emulator_list_view_wrapper,
            system_ids: init.system_ids,
            emulators: Vec::new(),
            selected_emulator: None,
            emulator_form: None,
        };

        let file_list_view = &model.file_list_view_wrapper.view;

        let emulator_list_view = &model.emulator_list_view_wrapper.view;
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
        let widgets = view_output!();
        sender.input(EmulatorRunnerMsg::FetchEmulators);
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            EmulatorRunnerMsg::RunEmulator => {
                // Logic to run the emulator with the selected file set and file
            }
            EmulatorRunnerMsg::FileSelected { index } => {
                let file_list_item = self.file_list_view_wrapper.get(index);
                if let Some(item) = file_list_item {
                    let id = item.borrow().id;
                    let file_info = self.file_set.files.iter().find(|f| f.file_info_id == id);
                    self.selected_file = file_info.cloned();
                }
            }
            EmulatorRunnerMsg::EmulatorSelected { index } => {
                let emulator_list_item = self.emulator_list_view_wrapper.get(index);
                if let Some(item) = emulator_list_item {
                    let id = item.borrow().id;
                    let emulator = self.emulators.iter().find(|e| e.id == id);
                    self.selected_emulator = emulator.cloned();
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
            EmulatorRunnerMsg::FetchEmulators => {
                let view_model_service = Arc::clone(&self.view_model_service);
                let system_ids = self.system_ids.clone();
                sender.oneshot_command(async move {
                    let emulators_result = view_model_service
                        .get_emulator_view_models_for_systems(system_ids)
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
        _: &Self::Root,
    ) {
        match message {
            EmulatorRunnerCommandMsg::EmulatorsFetched(Ok(emulator_view_models)) => {
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
            _ => {
                // Handle error or other cases
            }
        }
    }
}
