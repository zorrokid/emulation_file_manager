use std::sync::Arc;

use relm4::{
    Component, ComponentParts, ComponentSender, RelmWidgetExt,
    gtk::{self, glib::clone, prelude::*},
};
use service::{
    error::Error,
    view_model_service::{ReleaseFilter, ViewModelService},
    view_models::ReleaseListModel,
};

#[derive(Debug)]
pub enum ReleasesMsg {
    SomeMessage,
    SoftwareTitleSelected { id: i64 },
}

#[derive(Debug)]
pub enum CommandMsg {
    SomeMessage(Result<Vec<ReleaseListModel>, Error>),
}

#[derive(Debug)]
pub struct ReleasesModel {
    view_model_service: Arc<ViewModelService>,
}

#[relm4::component(pub)]
impl Component for ReleasesModel {
    type Input = ReleasesMsg;
    type Output = ();
    type CommandOutput = CommandMsg;
    type Init = Arc<ViewModelService>;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 5,
            set_margin_all: 5,

            gtk::Label {
                set_label: "Releases Component",
            },

        }
    }

    fn init(
        view_model_service: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let widgets = view_output!();
        let model = ReleasesModel { view_model_service };
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _: &Self::Root) {
        match msg {
            ReleasesMsg::SomeMessage => {
                // Handle the message
            }
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
                    CommandMsg::SomeMessage(releases_result) // Replace with actual command message
                });
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
            CommandMsg::SomeMessage(releases_result) => {
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
