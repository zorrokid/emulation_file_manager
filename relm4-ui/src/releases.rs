use relm4::{
    Component, ComponentParts, ComponentSender, RelmWidgetExt,
    gtk::{self, prelude::*},
};

#[derive(Debug)]
pub enum ReleasesMsg {
    SomeMessage,
}

#[derive(Debug)]
pub enum CommandMsg {}

#[derive(Debug, Default)]
pub struct ReleasesModel {}

#[relm4::component(pub)]
impl Component for ReleasesModel {
    type Input = ReleasesMsg;
    type Output = ();
    type CommandOutput = CommandMsg;
    type Init = ();

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
        _: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let widgets = view_output!();
        let model = ReleasesModel {};
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>, _: &Self::Root) {
        match msg {
            ReleasesMsg::SomeMessage => {
                // Handle the message
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
