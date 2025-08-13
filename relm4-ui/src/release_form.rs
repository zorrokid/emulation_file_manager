use std::sync::Arc;

use database::{
    database_error::{DatabaseError, Error},
    repository_manager::RepositoryManager,
};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller,
    gtk::{
        self,
        glib::clone,
        prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt},
    },
    typed_view::list::TypedListView,
};
use service::{
    view_model_service::ViewModelService,
    view_models::{
        FileSetListModel, ReleaseViewModel, Settings, SoftwareTitleListModel, SystemListModel,
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
    SoftwareTitleCreated(SoftwareTitleListModel),
    RemoveSoftwareTitle,
    SoftwareTitleSelectedFromList { index: u32 },
}

#[derive(Debug)]
pub enum ReleaseFormOutputMsg {
    ReleaseCreatedOrUpdated { id: i64 },
    SoftwareTitleCreated(SoftwareTitleListModel),
}

#[derive(Debug)]
pub enum CommandMsg {
    ReleaseCreatedOrUpdated(Result<i64, Error>),
}

#[derive(Debug)]
pub struct ReleaseFormModel {
    view_model_service: Arc<ViewModelService>,
    repository_manager: Arc<RepositoryManager>,
    settings: Arc<Settings>,
    system_selector: Option<Controller<SystemSelectModel>>,
    file_selector: Option<Controller<FileSelectModel>>,
    software_title_selector: Option<Controller<SoftwareTitleSelectModel>>,
    selected_software_titles_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    selected_systems_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    selected_file_sets_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    selected_software_title: Option<i64>,
    release: Option<ReleaseViewModel>,
}

pub struct ReleaseFormInit {
    pub view_model_service: Arc<ViewModelService>,
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
    pub release: Option<ReleaseViewModel>,
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
                set_spacing: 10,

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    gtk::Label {
                        set_label: "Software titles",
                    },
                    gtk::ScrolledWindow {
                        set_vexpand: true,
                        #[local_ref]
                        selected_software_titles_list_view -> gtk::ListView {}
                    },
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,

                        gtk::Button {
                            set_label: "Select Software Title",
                            connect_clicked => ReleaseFormMsg::OpenSoftwareTitleSelector,
                        },
                        gtk::Button {
                            set_label: "Remove Software Title",
                            connect_clicked => ReleaseFormMsg::RemoveSoftwareTitle,
                            set_sensitive: model.selected_software_title.is_some()
                        },
                    },
                },


                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    gtk::Label {
                        set_label: "Systems",
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
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    gtk::Label {
                        set_label: "File sets",
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
        let mut selected_systems = vec![];
        let mut selected_file_sets = vec![];
        let mut selected_software_titles = vec![];
        if let Some(release) = &init_model.release {
            selected_systems = release
                .systems
                .iter()
                .map(|s| SystemListModel {
                    id: s.id,
                    name: s.name.clone(),
                    can_delete: false,
                })
                .collect();

            selected_file_sets = release
                .file_sets
                .iter()
                .map(|fs| FileSetListModel {
                    id: fs.id,
                    file_set_name: fs.file_set_name.clone(),
                    file_type: fs.file_type,
                    file_name: fs.file_name.clone(),
                })
                .collect();
            selected_software_titles = release
                .software_titles
                .iter()
                .map(|st| SoftwareTitleListModel {
                    id: st.id,
                    name: st.name.clone(),
                    can_delete: false,
                })
                .collect();
        }

        let mut selected_systems_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection> =
            TypedListView::new();

        selected_systems_list_view_wrapper.extend_from_iter(selected_systems.iter().map(|s| {
            ListItem {
                id: s.id,
                name: s.name.clone(),
            }
        }));

        let mut selected_file_sets_list_view_wrapper: TypedListView<
            ListItem,
            gtk::SingleSelection,
        > = TypedListView::new();

        selected_file_sets_list_view_wrapper.extend_from_iter(selected_file_sets.iter().map(
            |fs| ListItem {
                id: fs.id,
                name: fs.file_set_name.clone(),
            },
        ));

        let mut selected_software_titles_list_view_wrapper: TypedListView<
            ListItem,
            gtk::SingleSelection,
        > = TypedListView::new();

        selected_software_titles_list_view_wrapper.extend_from_iter(
            selected_software_titles.iter().map(|st| ListItem {
                id: st.id,
                name: st.name.clone(),
            }),
        );

        let software_titles_selection_model =
            &selected_software_titles_list_view_wrapper.selection_model;

        software_titles_selection_model.connect_selected_notify(clone!(
            #[strong]
            sender,
            move |selection| {
                sender.input(ReleaseFormMsg::SoftwareTitleSelectedFromList {
                    index: selection.selected(),
                });
            }
        ));

        let selected_software_title = selected_software_titles_list_view_wrapper
            .get(software_titles_selection_model.selected())
            .map(|t| t.borrow().id);

        let model = ReleaseFormModel {
            view_model_service: init_model.view_model_service,
            repository_manager: init_model.repository_manager,
            settings: init_model.settings,
            release: init_model.release,
            system_selector: None,
            file_selector: None,
            software_title_selector: None,
            selected_software_titles_list_view_wrapper,
            selected_systems_list_view_wrapper,
            selected_file_sets_list_view_wrapper,
            selected_software_title,
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
                    selected_system_ids: get_item_ids(&self.selected_systems_list_view_wrapper),
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
                    selected_system_ids: get_item_ids(&self.selected_systems_list_view_wrapper),
                    selected_file_set_ids: get_item_ids(&self.selected_file_sets_list_view_wrapper),
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
                        selected_software_title_ids: get_item_ids(
                            &self.selected_software_titles_list_view_wrapper,
                        ),
                    })
                    .forward(sender.input_sender(), |msg| match msg {
                        SoftwareTitleSelectOutputMsg::SoftwareTitleSelected(software_title) => {
                            ReleaseFormMsg::SoftwareTitleSelected(software_title)
                        }
                        SoftwareTitleSelectOutputMsg::SoftwareTitleCreated(software_title) => {
                            ReleaseFormMsg::SoftwareTitleCreated(software_title)
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
            }
            ReleaseFormMsg::FileSetSelected(file_set) => {
                println!("File set selected: {:?}", &file_set);
                self.selected_file_sets_list_view_wrapper.append(ListItem {
                    name: file_set.file_set_name.clone(),
                    id: file_set.id,
                });
            }
            ReleaseFormMsg::SoftwareTitleSelected(software_title) => {
                println!("Software title selected: {:?}", &software_title);
                self.selected_software_titles_list_view_wrapper
                    .append(ListItem {
                        name: software_title.name.clone(),
                        id: software_title.id,
                    });
            }
            ReleaseFormMsg::StartSaveRelease => {
                println!("Starting to save release with selected systems and file sets");
                let repository_manager = Arc::clone(&self.repository_manager);
                let software_title_ids =
                    get_item_ids(&self.selected_software_titles_list_view_wrapper);
                let system_ids = get_item_ids(&self.selected_systems_list_view_wrapper);

                let file_set_ids = get_item_ids(&self.selected_file_sets_list_view_wrapper);

                if system_ids.is_empty() {
                    println!("No systems selected, cannot create release.");
                } else if file_set_ids.is_empty() {
                    println!("No file sets selected, cannot create release.");
                } else if software_title_ids.is_empty() {
                    println!("No software titles selected, cannot create release.");
                } else {
                    let release_id = self.release.as_ref().map(|r| r.id);

                    sender.oneshot_command(async move {
                        let res = match release_id {
                            Some(id) => {
                                println!("Editing existing release with id: {}", id);
                                repository_manager
                                    .get_release_repository()
                                    .update_release_full(
                                        id,
                                        "".to_string(),
                                        software_title_ids,
                                        file_set_ids,
                                        system_ids,
                                    )
                                    .await
                            }
                            _ => {
                                println!("Creating new release");
                                repository_manager
                                    .get_release_repository()
                                    .add_release_full(
                                        "".to_string(),
                                        software_title_ids,
                                        file_set_ids,
                                        system_ids,
                                    )
                                    .await
                            }
                        };
                        CommandMsg::ReleaseCreatedOrUpdated(res)
                    });
                }
            }
            ReleaseFormMsg::SoftwareTitleCreated(software_title) => {
                println!("Software title created: {:?}", &software_title);
                let res = sender.output(ReleaseFormOutputMsg::SoftwareTitleCreated(software_title));
                if let Err(msg) = res {
                    eprintln!("Error in sending message {:?}", msg);
                }
            }
            ReleaseFormMsg::SoftwareTitleSelectedFromList { index } => {
                self.selected_software_title = self
                    .selected_software_titles_list_view_wrapper
                    .get(index)
                    .map(|t| t.borrow().id);
            }
            ReleaseFormMsg::RemoveSoftwareTitle => {
                let selected_position = self
                    .selected_software_titles_list_view_wrapper
                    .selection_model
                    .selected();
                self.selected_software_titles_list_view_wrapper
                    .remove(selected_position);
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
            CommandMsg::ReleaseCreatedOrUpdated(Ok(id)) => {
                println!("Release created or updated with ID: {}", id);
                let res = sender.output(ReleaseFormOutputMsg::ReleaseCreatedOrUpdated { id });
                if let Err(e) = res {
                    eprintln!("Failed to send output message: {:?}", e);
                    // TODO: show error to user
                } else {
                    println!("Output message sent successfully");
                    root.close();
                }
            }
            CommandMsg::ReleaseCreatedOrUpdated(Err(err)) => {
                eprintln!("Failed to create release: {:?}", err);
                // TODO: show error to user
            }
        }
    }
}

fn get_item_ids(list_view_wrapper: &TypedListView<ListItem, gtk::SingleSelection>) -> Vec<i64> {
    (0..list_view_wrapper.len())
        .filter_map(|i| list_view_wrapper.get(i).map(|st| st.borrow().id))
        .collect()
}
