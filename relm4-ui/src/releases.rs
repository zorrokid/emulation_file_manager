use std::sync::Arc;

use database::repository_manager::RepositoryManager;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
    gtk::{self, prelude::*},
    typed_view::list::TypedListView,
};
use service::{
    error::Error,
    view_model_service::{ReleaseFilter, ViewModelService},
    view_models::{ReleaseListModel, Settings},
};

use crate::{
    list_item::ListItem,
    release::{ReleaseInitModel, ReleaseModel, ReleaseMsg},
    release_form::{ReleaseFormInit, ReleaseFormModel, ReleaseFormOutputMsg},
};

#[derive(Debug)]
pub enum ReleasesMsg {
    SoftwareTitleSelected { id: i64 },
    ReleaseSelected { index: u32 },
    StartAddRelease,
    AddRelease(ReleaseListModel),
    FetchReleases,
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

#[relm4::component(pub)]
impl Component for ReleasesModel {
    type Input = ReleasesMsg;
    type Output = ();
    type CommandOutput = CommandMsg;
    type Init = ReleasesInit;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,

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
        let release_model = ReleaseModel::builder().launch(release_init_model).detach();

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
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            ReleasesMsg::SoftwareTitleSelected { id } => {
                println!("Software title selected with ID: {}", id);
                self.selected_software_title_id = Some(id);
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
                println!("Software title selected with index: {}", index);

                let selected = self.releases_list_view_wrapper.get(index);
                if let Some(item) = selected {
                    println!("Selected item: {:?}", item);
                    let selected_id = item.borrow().id;
                    self.release
                        .sender()
                        .emit(ReleaseMsg::ReleaseSelected { id: selected_id });
                } else {
                    println!("No item found at index: {}", index);
                }
            }

            ReleasesMsg::StartAddRelease => {
                if let Some(software_title_id) = self.selected_software_title_id {
                    println!(
                        "Starting to add release for software title ID: {}",
                        software_title_id
                    );

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
                                ReleasesMsg::FetchReleases
                            }
                        });

                    self.form_window = Some(form_window);

                    self.form_window
                        .as_ref()
                        .expect("Form window should be set already")
                        .widget()
                        .present();
                } else {
                    eprintln!("No software title selected for adding a release.");
                    return;
                }
            }
            ReleasesMsg::AddRelease(release_list_model) => {
                println!("Release added: {:?}", release_list_model);
                self.releases_list_view_wrapper.append(ListItem {
                    id: release_list_model.id,
                    name: release_list_model.name,
                });
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
                            .map(|release| ListItem {
                                id: release.id,
                                name: release.name,
                            })
                            .collect();
                        self.releases_list_view_wrapper.clear();
                        self.releases_list_view_wrapper.extend_from_iter(items);
                        let index = self.releases_list_view_wrapper.selection_model.selected();
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
