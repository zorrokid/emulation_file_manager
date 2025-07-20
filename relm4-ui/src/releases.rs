use std::sync::Arc;

use relm4::{
    Component, ComponentParts, ComponentSender, RelmWidgetExt,
    gtk::{self, prelude::*},
};
use service::view_model_service::ViewModelService;

#[derive(Debug)]
pub enum ReleasesMsg {
    SomeMessage,
    SoftwareTitleSelected { id: i64 },
}

#[derive(Debug)]
pub enum CommandMsg {}

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

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>, _: &Self::Root) {
        match msg {
            ReleasesMsg::SomeMessage => {
                // Handle the message
            }
            ReleasesMsg::SoftwareTitleSelected { id } => {
                // Handle software title selection
                println!("Software title selected with ID: {}", id);
            }
        }
    }
    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _: &Self::Root,
    ) {
        match message {}
    }
}
