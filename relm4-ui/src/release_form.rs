use relm4::{
    Component, ComponentParts, ComponentSender,
    gtk::{self, prelude::GtkWindowExt},
};

#[derive(Debug)]
pub enum ReleaseFormMsg {}

#[derive(Debug)]
pub enum CommandMsg {}

#[derive(Debug)]
pub struct ReleaseFormModel;

#[relm4::component(pub)]
impl Component for ReleaseFormModel {
    type Input = ReleaseFormMsg;
    type Output = ();
    type CommandOutput = CommandMsg;
    type Init = ();

    view! {
        #[root]
        gtk::Window {
            set_title: Some("Release Form"),
            gtk::Box {
                gtk::Label {
                    set_label: "Release Form Component",
                },
            }
        }
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let widgets = view_output!();
        let model = ReleaseFormModel {};
        ComponentParts { model, widgets }
    }
    fn update(&mut self, _msg: Self::Input, _sender: ComponentSender<Self>, _: &Self::Root) {}
    fn update_cmd(
        &mut self,
        _message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _: &Self::Root,
    ) {
    }
}
