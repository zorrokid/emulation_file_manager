use std::sync::Arc;

use database::repository_manager::RepositoryManager;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller,
    gtk::{
        self,
        prelude::{ButtonExt, GtkWindowExt, OrientableExt, WidgetExt},
    },
    typed_view::list::TypedListView,
};
use service::{
    error::Error,
    view_model_service::ViewModelService,
    view_models::{FileSetViewModel, ReleaseViewModel, Settings},
};

use crate::{
    emulator_runner::{EmulatorRunnerInit, EmulatorRunnerModel},
    list_item::ListItem,
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
}

#[derive(Debug)]
pub struct ReleaseInitModel {
    pub view_model_service: Arc<ViewModelService>,
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
}

#[derive(Debug)]
pub enum ReleaseMsg {
    ReleaseSelected { release_id: i64 },
    StartEmulatorRunner,
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
            }

        }
    }

    fn init(
        init_model: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
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
        };

        let file_set_list_view = &model.file_set_list_view_wrapper.view;
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            ReleaseMsg::ReleaseSelected { release_id } => {
                let view_model_service = Arc::clone(&self.view_model_service);

                sender.oneshot_command(async move {
                    let release = view_model_service.get_release_view_model(release_id).await;
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
                self.selected_release = Some(release);
                let system_names = self.selected_release.as_ref().map_or(String::new(), |r| {
                    r.systems
                        .iter()
                        .map(|s| s.name.clone())
                        .collect::<Vec<_>>()
                        .join(", ")
                });
                self.selected_release_system_names = system_names;
                self.file_set_list_view_wrapper.clear();
                self.file_set_list_view_wrapper.extend_from_iter(
                    self.selected_release.as_ref().map_or(vec![], |r| {
                        r.file_sets
                            .iter()
                            .map(|fs| ListItem {
                                id: fs.id,
                                name: fs.file_set_name.clone(),
                            })
                            .collect()
                    }),
                );

                let selected_index = self.file_set_list_view_wrapper.selection_model.selected();
                let selected_file_set_list_item =
                    self.file_set_list_view_wrapper.get(selected_index);
                if let (Some(file_set_list_item), Some(release)) =
                    (selected_file_set_list_item, &self.selected_release)
                {
                    let file_set = release
                        .file_sets
                        .iter()
                        .find(|fs| fs.id == file_set_list_item.borrow().id);
                    self.selected_file_set = file_set.cloned();
                } else {
                    self.selected_file_set = None;
                }
            }
            ReleaseCommandMsg::FetchedRelease(Err(err)) => {
                eprintln!("Error fetching release: {:?}", err);
                // TODO: show error to user
            }
        }
    }
}
