use std::sync::Arc;

use database::{database_error::Error, repository_manager::RepositoryManager};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller,
    gtk::{
        self, gio,
        glib::clone,
        prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt},
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
    system_selector::{SystemSelectInit, SystemSelectModel, SystemSelectOutputMsg},
};

#[derive(Debug)]
pub enum ReleaseFormMsg {
    OpenSystemSelector,
    OpenFileSelector,
    SystemSelected(SystemListModel),
    FileSetSelected(FileSetListModel),
    StartSaveRelease,
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
    selected_systems: Vec<SystemListModel>,
    selected_file_sets: Vec<FileSetListModel>,
    settings: Arc<Settings>,
    system_selector: Option<Controller<SystemSelectModel>>,
    file_selector: Option<Controller<FileSelectModel>>,
    selected_systems_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    selected_file_sets_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    selected_sofware_titles: Vec<SoftwareTitleListModel>,
}

#[derive(Debug)]
pub struct Widgets {}

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
    //type Widgets = Widgets;
    //type Root = gtk::Window;

    /*fn init_root() -> Self::Root {
        gtk::Window::builder()
            .title("Release Form")
            .default_width(800)
            .default_height(800)
            .build()
    }*/

    view! {
        #[root]
        gtk::Window {
            set_default_width: 800,
            set_default_height: 600,
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                gtk::Label {
                    set_label: "Release Form Component",
                },

                gtk::Button {
                    set_label: "Select System",
                    connect_clicked => ReleaseFormMsg::OpenSystemSelector,
                },

                gtk::ScrolledWindow {
                    set_vexpand: true,
                    #[local_ref]
                    selected_systems_list_view -> gtk::ListView {}
                },

                gtk::Button {
                    set_label: "Select File Set",
                    connect_clicked => ReleaseFormMsg::OpenFileSelector,
                },


               gtk::ScrolledWindow {
                    set_min_content_height: 360,
                    set_vexpand: true,

                    #[local_ref]
                    selected_file_sets_list_view -> gtk::ListView {}

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
        // TODO: add software title selector and possibly convert to use component macro
        /*let v_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        let label = gtk::Label::new(Some("Release Form Component"));
        v_box.append(&label);*/

        let selected_systems_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection> =
            TypedListView::new();

        /*v_box.append(&selected_systems_list_view_wrapper.view);

        // TODO: disable when window is opened
        let select_system_button = gtk::Button::with_label("Select System");
        select_system_button.connect_clicked(clone!(
            #[strong]
            sender,
            move |_| {
                sender.input(ReleaseFormMsg::OpenSystemSelector);
                println!("Select System button clicked");
            }
        ));

        v_box.append(&select_system_button);*/

        let selected_file_sets_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection> =
            TypedListView::new();

        /*v_box.append(&selected_file_sets_list_view_wrapper.view);

        // TODO: disable when window is opened
        let select_file_button = gtk::Button::with_label("Select File Set");
        select_file_button.connect_clicked(clone!(
            #[strong]
            sender,
            move |_| {
                sender.input(ReleaseFormMsg::OpenFileSelector);
                println!("Select File Set button clicked");
            }
        ));

        v_box.append(&select_file_button);

        let submit_button = gtk::Button::with_label("Submit Release");
        submit_button.connect_clicked(clone!(
            #[strong]
            sender,
            move |_| {
                println!("Submit Release button clicked");
                sender.input(ReleaseFormMsg::StartSaveRelease);
            }
        ));

        root.set_child(Some(&v_box));*/

        //let widgets = Widgets {};

        let model = ReleaseFormModel {
            view_model_service: init_model.view_model_service,
            repository_manager: init_model.repository_manager,
            settings: init_model.settings,
            selected_systems: Vec::new(),
            system_selector: None,
            file_selector: None,
            selected_systems_list_view_wrapper,
            selected_file_sets_list_view_wrapper,
            selected_file_sets: Vec::new(),
            selected_sofware_titles: Vec::new(),
        };

        let selected_systems_list_view = &model.selected_systems_list_view_wrapper.view;
        let selected_file_sets_list_view = &model.selected_file_sets_list_view_wrapper.view;
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
                } else {
                    println!("Output message sent successfully");
                    root.close();
                }
            }
            CommandMsg::ReleaseCreated(Err(err)) => {
                eprintln!("Failed to create release: {:?}", err);
                // Handle error, maybe show a dialog or log it
            }
        }
    }
}
