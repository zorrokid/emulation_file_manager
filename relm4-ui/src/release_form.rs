use std::sync::Arc;

use database::{database_error::Error, repository_manager::RepositoryManager};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller,
    gtk::{
        self,
        prelude::{ButtonExt, GtkWindowExt, OrientableExt, WidgetExt},
    },
    typed_view::list::TypedListView,
};
use service::{
    view_model_service::ViewModelService,
    view_models::{
        FileSetListModel, ReleaseListModel, Settings, SoftwareTitleListModel, SystemListModel,
    },
};

use crate::{
    file_selector::{FileSelectInit, FileSelectModel, FileSelectOutputMsg},
    list_item::ListItem,
    software_title_selector::{
        SoftwareTitleSelectInit, SoftwareTitleSelectModel, SoftwareTitleSelectOutputMsg,
    },
    system_selector::{SystemSelectInit, SystemSelectModel, SystemSelectOutputMsg},
};

#[derive(Debug)]
pub enum ReleaseFormMsg {
    OpenSystemSelector,
    OpenFileSelector,
    SystemSelected(SystemListModel),
    FileSetSelected(FileSetListModel),
    SoftwareTitleSelected(SoftwareTitleListModel),
    StartSaveRelease,
    OpenSoftwareTitleSelector,
}

#[derive(Debug)]
pub enum ReleaseFormOutputMsg {
    ReleaseCreated(ReleaseListModel),
}

#[derive(Debug)]
pub enum CommandMsg {
    ReleaseCreated(Result<i64, Error>),
}

#[derive(Debug)]
pub struct ReleaseFormModel {
    view_model_service: Arc<ViewModelService>,
    repository_manager: Arc<RepositoryManager>,
    selected_sofware_titles: Vec<SoftwareTitleListModel>,
    selected_systems: Vec<SystemListModel>,
    selected_file_sets: Vec<FileSetListModel>,
    settings: Arc<Settings>,
    system_selector: Option<Controller<SystemSelectModel>>,
    file_selector: Option<Controller<FileSelectModel>>,
    software_title_selector: Option<Controller<SoftwareTitleSelectModel>>,
    selected_software_titles_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    selected_systems_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    selected_file_sets_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
}

pub struct ReleaseFormInit {
    pub view_model_service: Arc<ViewModelService>,
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
}

#[relm4::component(pub)]
impl Component for ReleaseFormModel {
    type Input = ReleaseFormMsg;
    type Output = ReleaseFormOutputMsg;
    type CommandOutput = CommandMsg;
    type Init = ReleaseFormInit;

    view! {
        #[root]
        gtk::Window {
            set_default_width: 800,
            set_default_height: 600,
            set_title: Some("Release Form"),
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                gtk::ScrolledWindow {
                    set_vexpand: true,
                    #[local_ref]
                    selected_software_titles_list_view -> gtk::ListView {}
                },
                gtk::Button {
                    set_label: "Select Software Title",
                    connect_clicked => ReleaseFormMsg::OpenSoftwareTitleSelector,
                },


                gtk::ScrolledWindow {
                    set_vexpand: true,
                    #[local_ref]
                    selected_systems_list_view -> gtk::ListView {}
                },
                gtk::Button {
                    set_label: "Select System",
                    connect_clicked => ReleaseFormMsg::OpenSystemSelector,
                },



               gtk::ScrolledWindow {
                    set_min_content_height: 360,
                    set_vexpand: true,

                    #[local_ref]
                    selected_file_sets_list_view -> gtk::ListView {}

                },
                gtk::Button {
                    set_label: "Select File Set",
                    connect_clicked => ReleaseFormMsg::OpenFileSelector,
                },


                gtk::Button {
                    set_label: "Submit Release",
                    connect_clicked => ReleaseFormMsg::StartSaveRelease,
                },
            },
        }
    }

    fn init(
        init_model: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let selected_systems_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection> =
            TypedListView::new();

        let selected_file_sets_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection> =
            TypedListView::new();

        let selected_software_titles_list_view_wrapper: TypedListView<
            ListItem,
            gtk::SingleSelection,
        > = TypedListView::new();

        let model = ReleaseFormModel {
            view_model_service: init_model.view_model_service,
            repository_manager: init_model.repository_manager,
            settings: init_model.settings,
            selected_systems: Vec::new(),
            system_selector: None,
            file_selector: None,
            software_title_selector: None,
            selected_software_titles_list_view_wrapper,
            selected_systems_list_view_wrapper,
            selected_file_sets_list_view_wrapper,
            selected_file_sets: Vec::new(),
            selected_sofware_titles: Vec::new(),
        };

        let selected_systems_list_view = &model.selected_systems_list_view_wrapper.view;
        let selected_file_sets_list_view = &model.selected_file_sets_list_view_wrapper.view;
        let selected_software_titles_list_view =
            &model.selected_software_titles_list_view_wrapper.view;
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            ReleaseFormMsg::OpenSystemSelector => {
                let init_model = SystemSelectInit {
                    view_model_service: Arc::clone(&self.view_model_service),
                    repository_manager: Arc::clone(&self.repository_manager),
                };
                let system_selector = SystemSelectModel::builder()
                    .transient_for(root)
                    .launch(init_model)
                    .forward(sender.input_sender(), |msg| match msg {
                        SystemSelectOutputMsg::SystemSelected(system_list_model) => {
                            ReleaseFormMsg::SystemSelected(system_list_model)
                        }
                    });
                self.system_selector = Some(system_selector);

                self.system_selector
                    .as_ref()
                    .expect("System selector should be set")
                    .widget()
                    .present();
            }
            ReleaseFormMsg::OpenFileSelector => {
                let init_model = FileSelectInit {
                    view_model_service: Arc::clone(&self.view_model_service),
                    repository_manager: Arc::clone(&self.repository_manager),
                    settings: Arc::clone(&self.settings),
                    selected_system_ids: self.selected_systems.iter().map(|s| s.id).collect(),
                };
                let file_selector = FileSelectModel::builder()
                    .transient_for(root)
                    .launch(init_model)
                    .forward(sender.input_sender(), |msg| match msg {
                        FileSelectOutputMsg::FileSetSelected(file_set_liset_model) => {
                            ReleaseFormMsg::FileSetSelected(file_set_liset_model)
                        }
                    });
                self.file_selector = Some(file_selector);

                self.file_selector
                    .as_ref()
                    .expect("File selector should be set")
                    .widget()
                    .present();
            }
            ReleaseFormMsg::OpenSoftwareTitleSelector => {
                let software_title_selector = SoftwareTitleSelectModel::builder()
                    .transient_for(root)
                    .launch(SoftwareTitleSelectInit {
                        view_model_service: Arc::clone(&self.view_model_service),
                        repository_manager: Arc::clone(&self.repository_manager),
                    })
                    .forward(sender.input_sender(), |msg| match msg {
                        SoftwareTitleSelectOutputMsg::SoftwareTitleSelected(software_title) => {
                            ReleaseFormMsg::SoftwareTitleSelected(software_title)
                        }
                    });
                self.software_title_selector = Some(software_title_selector);
                self.software_title_selector
                    .as_ref()
                    .expect("Software title selector should be set")
                    .widget()
                    .present();
            }

            ReleaseFormMsg::SystemSelected(system) => {
                println!("System selected: {:?}", &system);
                self.selected_systems_list_view_wrapper.append(ListItem {
                    name: system.name.clone(),
                    id: system.id,
                });
                self.selected_systems.push(system);
            }
            ReleaseFormMsg::FileSetSelected(file_set) => {
                println!("File set selected: {:?}", &file_set);
                self.selected_file_sets_list_view_wrapper.append(ListItem {
                    name: file_set.file_set_name.clone(),
                    id: file_set.id,
                });
                self.selected_file_sets.push(file_set);
            }
            ReleaseFormMsg::SoftwareTitleSelected(software_title) => {
                println!("Software title selected: {:?}", &software_title);
                self.selected_software_titles_list_view_wrapper
                    .append(ListItem {
                        name: software_title.name.clone(),
                        id: software_title.id,
                    });
                self.selected_sofware_titles.push(software_title);
            }
            ReleaseFormMsg::StartSaveRelease => {
                println!("Starting to save release with selected systems and file sets");
                let repository_manager = Arc::clone(&self.repository_manager);
                if self.selected_systems.is_empty() {
                    println!("No systems selected, cannot create release.");
                } else if self.selected_file_sets.is_empty() {
                    println!("No file sets selected, cannot create release.");
                } else if self.selected_sofware_titles.is_empty() {
                    println!("No software titles selected, cannot create release.");
                } else {
                    println!(
                        "Selected systems: {:?}, Selected file sets: {:?}",
                        self.selected_systems, self.selected_file_sets
                    );

                    let software_title_ids: Vec<i64> = self
                        .selected_sofware_titles
                        .iter()
                        .map(|title| title.id)
                        .collect();

                    let file_set_ids: Vec<i64> =
                        self.selected_file_sets.iter().map(|fs| fs.id).collect();

                    let system_ids: Vec<i64> = self
                        .selected_systems
                        .iter()
                        .map(|system| system.id)
                        .collect();

                    sender.oneshot_command(async move {
                        let release_list_model = repository_manager
                            .get_release_repository()
                            .add_release_full(
                                "".to_string(),
                                software_title_ids,
                                file_set_ids,
                                system_ids,
                            )
                            .await;
                        CommandMsg::ReleaseCreated(release_list_model)
                    });
                }
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            CommandMsg::ReleaseCreated(Ok(id)) => {
                println!("Release created with ID: {}", id);
                let release_list_model = ReleaseListModel {
                    id,
                    name: "New Release".to_string(),
                    system_names: self
                        .selected_systems
                        .iter()
                        .map(|s| s.name.clone())
                        .collect(),
                    file_types: self
                        .selected_file_sets
                        .iter()
                        .map(|fs| fs.file_type.to_string())
                        .collect(),
                };
                let res = sender.output(ReleaseFormOutputMsg::ReleaseCreated(release_list_model));
                if let Err(e) = res {
                    eprintln!("Failed to send output message: {:?}", e);
                    // TODO: show error to user
                } else {
                    println!("Output message sent successfully");
                    root.close();
                }
            }
            CommandMsg::ReleaseCreated(Err(err)) => {
                eprintln!("Failed to create release: {:?}", err);
                // TODO: show error to user
            }
        }
    }
}
