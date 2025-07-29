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
    view_models::{ReleaseListModel, ReleaseViewModel, Settings},
};

use crate::{
    list_item::ListItem,
    release_form::{ReleaseFormInit, ReleaseFormModel, ReleaseFormOutputMsg},
};

#[derive(Debug)]
pub enum ReleasesMsg {
    SoftwareTitleSelected { id: i64 },
    ReleaseSelected { index: u32 },
    StartAddRelease,
    AddRelease(ReleaseListModel),
}

#[derive(Debug)]
pub enum CommandMsg {
    FetchedReleases(Result<Vec<ReleaseListModel>, Error>),
    FetchedRelease(Result<ReleaseViewModel, Error>),
}

#[derive(Debug)]
pub struct ReleasesModel {
    view_model_service: Arc<ViewModelService>,
    repository_manager: Arc<RepositoryManager>,
    settings: Arc<Settings>,
    form_window: Option<Controller<ReleaseFormModel>>,
    releases_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
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
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 5,
            set_margin_all: 5,

            gtk::ScrolledWindow {
                set_vexpand: true,
                #[local_ref]
                releases_list_view -> gtk::ListView {}
            },

            gtk::Button {
                set_label: "Add Release",
                connect_clicked => ReleasesMsg::StartAddRelease,
            },

        }
    }

    fn init(
        init_model: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = ReleasesModel {
            view_model_service: init_model.view_model_service,
            repository_manager: init_model.repository_manager,
            settings: init_model.settings,
            form_window: None,
            releases_list_view_wrapper: TypedListView::new(),
        };
        let releases_list_view = &model.releases_list_view_wrapper.view;
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            ReleasesMsg::SoftwareTitleSelected { id } => {
                println!("Software title selected with ID: {}", id);

                let view_model_service = Arc::clone(&self.view_model_service);

                sender.oneshot_command(async move {
                    let releases_result = view_model_service
                        .get_release_list_models(ReleaseFilter {
                            software_title_id: Some(id),
                            system_id: None,
                        })
                        .await;
                    println!("Fetched releases: {:?}", releases_result);
                    CommandMsg::FetchedReleases(releases_result)
                });
            }
            ReleasesMsg::ReleaseSelected { index } => {
                println!("Software title selected with index: {}", index);

                let selected = self.releases_list_view_wrapper.get(index);
                if let Some(item) = selected {
                    println!("Selected item: {:?}", item);
                    let selected_id = item.borrow().id;
                    let view_model_service = Arc::clone(&self.view_model_service);

                    sender.oneshot_command(async move {
                        let release = view_model_service.get_release_view_model(selected_id).await;
                        println!("Fetched release: {:?}", release);
                        CommandMsg::FetchedRelease(release)
                    });
                } else {
                    println!("No item found at index: {}", index);
                }
            }

            ReleasesMsg::StartAddRelease => {
                let release_form_init_model = ReleaseFormInit {
                    view_model_service: Arc::clone(&self.view_model_service),
                    repository_manager: Arc::clone(&self.repository_manager),
                    settings: Arc::clone(&self.settings),
                };
                let form_window = ReleaseFormModel::builder()
                    .transient_for(root)
                    .launch(release_form_init_model)
                    .forward(sender.input_sender(), |msg| match msg {
                        ReleaseFormOutputMsg::ReleaseCreated(release_list_model) => {
                            ReleasesMsg::AddRelease(release_list_model)
                        }
                    });

                self.form_window = Some(form_window);

                self.form_window
                    .as_ref()
                    .expect("Form window should be set already")
                    .widget()
                    .present();
                //form_window.connect_closed(...);
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
            CommandMsg::FetchedRelease(Ok(release)) => {
                println!("Release fetched successfully: {:?}", release);
                // Handle the fetched release, e.g., display it in a new window or dialog
            }
            CommandMsg::FetchedRelease(Err(err)) => {
                eprintln!("Error fetching release: {:?}", err);
                // TODO: show error to user
            }
        }
    }
}
