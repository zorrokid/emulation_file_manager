use std::sync::Arc;

use database::{models::FileType, repository_manager::RepositoryManager};
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
    error::Error,
    view_model_service::ViewModelService,
    view_models::{FileSetViewModel, ReleaseListModel, ReleaseViewModel, Settings},
};

use crate::{
    emulator_runner::{EmulatorRunnerInit, EmulatorRunnerModel},
    list_item::ListItem,
    release_form::{ReleaseFormInit, ReleaseFormModel, ReleaseFormOutputMsg},
};

#[derive(Debug)]
pub struct ReleaseModel {
    view_model_service: Arc<ViewModelService>,
    repository_manager: Arc<RepositoryManager>,
    settings: Arc<Settings>,

    selected_release: Option<ReleaseViewModel>,
    selected_release_system_names: String,
    file_set_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    selected_file_set: Option<FileSetViewModel>,
    emulator_runner: Option<Controller<EmulatorRunnerModel>>,
    form_window: Option<Controller<ReleaseFormModel>>,
}

#[derive(Debug)]
pub struct ReleaseInitModel {
    pub view_model_service: Arc<ViewModelService>,
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
}

#[derive(Debug)]
pub enum ReleaseMsg {
    ReleaseSelected { id: i64 },
    FetchRelease { id: i64 },
    StartEmulatorRunner,
    StartEditRelease,
    UpdateRelease(ReleaseListModel),
    Clear,
    FileSetSelected { index: u32 },
}

#[derive(Debug)]
pub enum ReleaseCommandMsg {
    FetchedRelease(Result<ReleaseViewModel, Error>),
}

#[relm4::component(pub)]
impl Component for ReleaseModel {
    type Input = ReleaseMsg;
    type Output = ();
    type CommandOutput = ReleaseCommandMsg;
    type Init = ReleaseInitModel;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            gtk::Label {
                set_label: "Release widget",
            },
            gtk::Label {
                #[watch]
                set_label: model.selected_release_system_names.as_str(),

            },

            #[local_ref]
            file_set_list_view -> gtk::ListView { },

            gtk::Button {
                set_label: "Run with Emulator",
                #[watch]
                set_sensitive: model.selected_file_set.is_some(),
                connect_clicked => ReleaseMsg::StartEmulatorRunner,
            },
            gtk::Button {
                set_label: "Edit",
                connect_clicked => ReleaseMsg::StartEditRelease,
            }
        }
    }

    fn init(
        init_model: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = ReleaseModel {
            view_model_service: init_model.view_model_service,
            repository_manager: init_model.repository_manager,
            settings: init_model.settings,

            selected_release: None,
            selected_release_system_names: String::new(),
            file_set_list_view_wrapper: TypedListView::new(),
            selected_file_set: None,
            emulator_runner: None,
            form_window: None,
        };

        let file_set_list_view = &model.file_set_list_view_wrapper.view;
        let selection_model = &model.file_set_list_view_wrapper.selection_model;
        selection_model.connect_selected_notify(clone!(
            #[strong]
            sender,
            move |s| {
                let index = s.selected();
                println!("Selected index: {}", index);
                sender.input(ReleaseMsg::FileSetSelected { index });
            }
        ));
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            ReleaseMsg::ReleaseSelected { id } => {
                sender.input(ReleaseMsg::FetchRelease { id });
            }
            ReleaseMsg::FetchRelease { id } => {
                let view_model_service = Arc::clone(&self.view_model_service);

                sender.oneshot_command(async move {
                    let release = view_model_service.get_release_view_model(id).await;
                    println!("Fetched release: {:?}", release);
                    ReleaseCommandMsg::FetchedRelease(release)
                });
            }
            ReleaseMsg::StartEmulatorRunner => {
                if let (Some(file_set), Some(release)) =
                    (&self.selected_file_set, &self.selected_release)
                {
                    println!("Starting emulator runner with file set: {:?}", file_set);
                    let init_model = EmulatorRunnerInit {
                        view_model_service: Arc::clone(&self.view_model_service),
                        repository_manager: Arc::clone(&self.repository_manager),
                        settings: Arc::clone(&self.settings),
                        file_set: file_set.clone(),
                        systems: release.systems.clone(),
                    };
                    let emulator_runner = EmulatorRunnerModel::builder()
                        .transient_for(root)
                        .launch(init_model)
                        .detach();

                    self.emulator_runner = Some(emulator_runner);
                    self.emulator_runner
                        .as_ref()
                        .expect("Emulator runner should be set already")
                        .widget()
                        .present();
                }
            }
            ReleaseMsg::UpdateRelease(release_list_model) => {
                println!("Updating release with model: {:?}", release_list_model);
                // TODO
            }
            ReleaseMsg::StartEditRelease => {
                if let Some(release) = &self.selected_release {
                    println!("Starting edit release for: {:?}", release);
                    let release_form_init_model = ReleaseFormInit {
                        view_model_service: Arc::clone(&self.view_model_service),
                        repository_manager: Arc::clone(&self.repository_manager),
                        settings: Arc::clone(&self.settings),
                        release: Some(release.clone()),
                    };
                    let form_window = ReleaseFormModel::builder()
                        .transient_for(root)
                        .launch(release_form_init_model)
                        .forward(sender.input_sender(), |msg| match msg {
                            ReleaseFormOutputMsg::ReleaseCreatedOrUpdated { id } => {
                                ReleaseMsg::FetchRelease { id }
                            }
                        });

                    self.form_window = Some(form_window);

                    self.form_window
                        .as_ref()
                        .expect("Form window should be set already")
                        .widget()
                        .present();
                }
            }
            ReleaseMsg::Clear => {
                println!("Clearing release model");
                self.selected_release = None;
                self.selected_release_system_names.clear();
                self.file_set_list_view_wrapper.clear();
                self.selected_file_set = None;
                self.emulator_runner = None;
                self.form_window = None;
            }
            ReleaseMsg::FileSetSelected { index } => {
                println!("File set selected with index: {}", index);
                let selected = self.file_set_list_view_wrapper.get(index);
                if let Some(file_set_list_item) = selected {
                    let file_set_id = file_set_list_item.borrow().id;
                    let file_set = self.selected_release.as_ref().and_then(|release| {
                        release
                            .file_sets
                            .iter()
                            .find(|fs| fs.id == file_set_id)
                            .cloned()
                    });
                    self.selected_file_set = file_set;
                    println!("Selected file set: {:?}", self.selected_file_set);
                } else {
                    println!("No file set found at index: {}", index);
                }
            }
            _ => (),
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _: &Self::Root,
    ) {
        match message {
            ReleaseCommandMsg::FetchedRelease(Ok(release)) => {
                println!("Release fetched successfully: {:?}", release);
                self.selected_release_system_names = release
                    .systems
                    .iter()
                    .map(|s| s.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ");

                let emulator_file_types = [
                    FileType::DiskImage,
                    FileType::TapeImage,
                    FileType::Rom,
                    FileType::MemorySnapshot,
                ];

                let emulator_file_sets = release
                    .file_sets
                    .iter()
                    .filter(|fs| emulator_file_types.contains(&fs.file_type))
                    .cloned()
                    .collect::<Vec<_>>();

                let emulator_file_set_list_items = emulator_file_sets.iter().map(|fs| ListItem {
                    id: fs.id,
                    name: fs.file_set_name.clone(),
                });

                self.file_set_list_view_wrapper.clear();
                self.file_set_list_view_wrapper
                    .extend_from_iter(emulator_file_set_list_items);

                let selected_index = self.file_set_list_view_wrapper.selection_model.selected();

                let selected_file_set_list_item =
                    self.file_set_list_view_wrapper.get(selected_index);
                if let Some(file_set_list_item) = selected_file_set_list_item {
                    let file_set = emulator_file_sets
                        .iter()
                        .find(|fs| fs.id == file_set_list_item.borrow().id);
                    self.selected_file_set = file_set.cloned();
                } else {
                    self.selected_file_set = None;
                }

                self.selected_release = Some(release);
            }
            ReleaseCommandMsg::FetchedRelease(Err(err)) => {
                eprintln!("Error fetching release: {:?}", err);
                // TODO: show error to user
            }
        }
    }
}
