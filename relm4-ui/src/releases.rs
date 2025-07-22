use std::sync::Arc;

use database::repository_manager::RepositoryManager;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
    gtk::{self, prelude::*},
    once_cell::sync::OnceCell,
};
use service::{
    error::Error,
    view_model_service::{ReleaseFilter, ViewModelService},
    view_models::ReleaseListModel,
};

use crate::release_form::{
    ReleaseFormInit, ReleaseFormModel, ReleaseFormMsg, ReleaseFormOutputMsg,
};

#[derive(Debug)]
pub enum ReleasesMsg {
    SoftwareTitleSelected { id: i64 },
    StartAddRelease,
    AddRelease(ReleaseListModel),
}

#[derive(Debug)]
pub enum CommandMsg {
    FetchedReleases(Result<Vec<ReleaseListModel>, Error>),
}

#[derive(Debug)]
pub struct ReleasesModel {
    view_model_service: Arc<ViewModelService>,
    repository_manager: Arc<RepositoryManager>,
    form_window: Option<Controller<ReleaseFormModel>>,
}

pub struct ReleasesInit {
    pub view_model_service: Arc<ViewModelService>,
    pub repository_manager: Arc<RepositoryManager>,
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

            gtk::Label {
                set_label: "Releases Component",
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
        let widgets = view_output!();

        let model = ReleasesModel {
            view_model_service: init_model.view_model_service,
            repository_manager: init_model.repository_manager,
            form_window: None,
        };
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _: &Self::Root) {
        match msg {
            ReleasesMsg::SoftwareTitleSelected { id } => {
                // Handle software title selection
                println!("Software title selected with ID: {}", id);

                let view_model_service = Arc::clone(&self.view_model_service);

                // TODO: use command with view_model_service to fetch releases for the selected software title
                sender.oneshot_command(async move {
                    // Simulate fetching releases
                    let releases_result = view_model_service
                        .get_release_list_models(ReleaseFilter {
                            software_title_id: Some(id),
                            system_id: None,
                        })
                        .await;
                    println!("Fetched releases: {:?}", releases_result);
                    CommandMsg::FetchedReleases(releases_result) // Replace with actual command message
                });
            }
            ReleasesMsg::StartAddRelease => {
                let release_form_init_model = ReleaseFormInit {
                    view_model_service: Arc::clone(&self.view_model_service),
                    repository_manager: Arc::clone(&self.repository_manager),
                };
                let form_window = ReleaseFormModel::builder()
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
                // Handle the added release
                println!("Release added: {:?}", release_list_model);
                // Here you would typically update the model or UI to reflect the new release
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
            CommandMsg::FetchedReleases(releases_result) => {
                match releases_result {
                    Ok(releases) => {
                        // Handle successful release fetching
                        println!("Releases fetched successfully: {:?}", releases);
                    }
                    Err(err) => {
                        // Handle error in fetching releases
                        eprintln!("Error fetching releases: {:?}", err);
                    }
                }
            }
        }
    }
}
