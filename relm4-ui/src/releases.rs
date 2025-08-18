use std::sync::Arc;

use database::repository_manager::RepositoryManager;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
    gtk::{self, glib::clone, prelude::*},
    typed_view::list::TypedListView,
};
use service::{
    error::Error,
    view_model_service::{ReleaseFilter, ViewModelService},
    view_models::{ReleaseListModel, Settings, SoftwareTitleListModel},
};

use crate::{
    list_item::ListItem,
    release::{ReleaseInitModel, ReleaseModel, ReleaseMsg, ReleaseOutputMsg},
    release_form::{ReleaseFormInit, ReleaseFormModel, ReleaseFormOutputMsg},
};

#[derive(Debug)]
pub enum ReleasesMsg {
    SoftwareTitleSelected { id: i64 },
    ReleaseSelected { index: u32 },
    StartAddRelease,
    AddRelease(ReleaseListModel),
    FetchReleases,
    ReleaseCreatedOrUpdated { id: i64 },
    SofwareTitleCreated(SoftwareTitleListModel),
}

#[derive(Debug)]
pub enum CommandMsg {
    FetchedReleases(Result<Vec<ReleaseListModel>, Error>),
}

#[derive(Debug)]
pub struct ReleasesModel {
    view_model_service: Arc<ViewModelService>,
    repository_manager: Arc<RepositoryManager>,
    settings: Arc<Settings>,
    form_window: Option<Controller<ReleaseFormModel>>,
    releases_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    selected_software_title_id: Option<i64>,

    release: Controller<ReleaseModel>,
}

pub struct ReleasesInit {
    pub view_model_service: Arc<ViewModelService>,
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
}

#[derive(Debug)]
pub enum ReleasesOutputMsg {
    SoftwareTitleCreated {
        software_title_list_model: SoftwareTitleListModel,
    },
    ReleaseSelected {
        id: i64,
    },
}

#[relm4::component(pub)]
impl Component for ReleasesModel {
    type Input = ReleasesMsg;
    type Output = ReleasesOutputMsg;
    type CommandOutput = CommandMsg;
    type Init = ReleasesInit;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_hexpand: true,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 10,
                set_margin_all: 10,

                gtk::Label {
                    set_label: "Releases",
                },

                gtk::ScrolledWindow {
                    set_vexpand: true,
                    #[local_ref]
                    releases_list_view -> gtk::ListView {}
                },

                gtk::Button {
                    set_label: "Add Release",
                    connect_clicked => ReleasesMsg::StartAddRelease,
                },

            },

            gtk::Box {
                append = model.release.widget(),
            }
        }
    }

    fn init(
        init_model: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let release_init_model = ReleaseInitModel {
            view_model_service: Arc::clone(&init_model.view_model_service),
            repository_manager: Arc::clone(&init_model.repository_manager),
            settings: Arc::clone(&init_model.settings),
        };
        let release_model = ReleaseModel::builder().launch(release_init_model).forward(
            sender.input_sender(),
            |msg| match msg {
                ReleaseOutputMsg::SoftwareTitleCreated(software_title_list_model) => {
                    ReleasesMsg::SofwareTitleCreated(software_title_list_model)
                }
            },
        );

        let model = ReleasesModel {
            view_model_service: init_model.view_model_service,
            repository_manager: init_model.repository_manager,
            settings: init_model.settings,
            form_window: None,
            releases_list_view_wrapper: TypedListView::new(),
            release: release_model,
            selected_software_title_id: None,
        };
        let releases_list_view = &model.releases_list_view_wrapper.view;
        let selection_model = &model.releases_list_view_wrapper.selection_model;
        selection_model.connect_selected_notify(clone!(
            #[strong]
            sender,
            move |selection| {
                sender.input(ReleasesMsg::ReleaseSelected {
                    index: selection.selected(),
                });
            }
        ));

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            ReleasesMsg::SoftwareTitleSelected { id } => {
                println!("Software title selected with ID: {}", id);
                self.selected_software_title_id = Some(id);
                self.release.sender().emit(ReleaseMsg::Clear);
                sender.input(ReleasesMsg::FetchReleases);
            }
            ReleasesMsg::FetchReleases => {
                if let Some(software_title_id) = self.selected_software_title_id {
                    println!(
                        "Fetching releases for software title ID: {}",
                        software_title_id
                    );

                    let view_model_service = Arc::clone(&self.view_model_service);
                    sender.oneshot_command(async move {
                        let releases_result = view_model_service
                            .get_release_list_models(ReleaseFilter {
                                software_title_id: Some(software_title_id),
                                system_id: None,
                            })
                            .await;
                        CommandMsg::FetchedReleases(releases_result)
                    });
                } else {
                    eprintln!("No software title selected to fetch releases.");
                }
            }
            ReleasesMsg::ReleaseSelected { index } => {
                println!("Release selected with index: {}", index);

                let selected = self.releases_list_view_wrapper.get(index);
                if let Some(item) = selected {
                    println!("Selected item: {:?}", item);
                    let selected_id = item.borrow().id;
                    self.release
                        .sender()
                        .emit(ReleaseMsg::ReleaseSelected { id: selected_id });
                    let res = sender.output(ReleasesOutputMsg::ReleaseSelected { id: selected_id });
                    if let Err(err) = res {
                        eprintln!("Error sending ReleaseSelected message: {:?}", err);
                    }
                } else {
                    println!("No item found at index: {}", index);
                }
            }

            ReleasesMsg::StartAddRelease => {
                let release_form_init_model = ReleaseFormInit {
                    view_model_service: Arc::clone(&self.view_model_service),
                    repository_manager: Arc::clone(&self.repository_manager),
                    settings: Arc::clone(&self.settings),
                    release: None,
                };

                let form_window = ReleaseFormModel::builder()
                    .transient_for(root)
                    .launch(release_form_init_model)
                    .forward(sender.input_sender(), |msg| match msg {
                        ReleaseFormOutputMsg::ReleaseCreatedOrUpdated { id } => {
                            println!("Release created or updated with ID: {}", id);
                            //ReleasesMsg::FetchReleases
                            ReleasesMsg::ReleaseCreatedOrUpdated { id }
                        }
                        ReleaseFormOutputMsg::SoftwareTitleCreated(software_title_list_model) => {
                            println!("Software title created: {:?}", software_title_list_model);
                            ReleasesMsg::SofwareTitleCreated(software_title_list_model)
                        }
                    });

                self.form_window = Some(form_window);

                self.form_window
                    .as_ref()
                    .expect("Form window should be set already")
                    .widget()
                    .present();
            }
            ReleasesMsg::AddRelease(release_list_model) => {
                println!("Release added: {:?}", release_list_model);
                self.releases_list_view_wrapper.append(ListItem {
                    id: release_list_model.id,
                    name: release_list_model.name,
                });
            }
            ReleasesMsg::ReleaseCreatedOrUpdated { id } => {
                println!("Release created or updated with ID: {}", id);
                // TODO fetch only the created of rupdated release
                sender.input(ReleasesMsg::FetchReleases);
            }
            ReleasesMsg::SofwareTitleCreated(software_title_list_model) => {
                let res = sender.output(ReleasesOutputMsg::SoftwareTitleCreated {
                    software_title_list_model,
                });
                if let Err(err) = res {
                    eprintln!("Error sending SoftwareTitleCreated message: {:?}", err);
                }
            }
        }
    }
    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _: &Self::Root,
    ) {
        match message {
            CommandMsg::FetchedReleases(releases_result) => {
                match releases_result {
                    Ok(releases) => {
                        println!("Releases fetched successfully: {:?}", releases);
                        let items: Vec<ListItem> = releases
                            .into_iter()
                            .map(|release| {
                                let name_string = if !release.name.is_empty() {
                                    format!("{} ", release.name)
                                } else {
                                    String::new()
                                };

                                ListItem {
                                    id: release.id,
                                    name: format!(
                                        "{}{} {}",
                                        release.system_names.join(", "),
                                        release.file_types.join(", "),
                                        name_string,
                                    ),
                                }
                            })
                            .collect();
                        self.releases_list_view_wrapper.clear();
                        self.releases_list_view_wrapper.extend_from_iter(items);
                        let index = self.releases_list_view_wrapper.selection_model.selected();
                        println!("Selected index after fetching releases: {}", index);
                        sender.input(ReleasesMsg::ReleaseSelected { index });
                    }
                    Err(err) => {
                        eprintln!("Error fetching releases: {:?}", err);
                        // TODO: show error to user
                    }
                }
            }
        }
    }
}
