use relm4::{
    Component, ComponentParts, ComponentSender,
    gtk::{
        self,
        prelude::{BoxExt, GtkWindowExt},
    },
};
use service::view_models::ReleaseListModel;

#[derive(Debug)]
pub enum ReleaseFormMsg {}

#[derive(Debug)]
pub enum ReleaseFormOutputMsg {
    ReleaseCreated(ReleaseListModel),
}

#[derive(Debug)]
pub enum CommandMsg {}

#[derive(Debug)]
pub struct ReleaseFormModel;

pub struct Widgets {}

//#[relm4::component(pub)]
impl Component for ReleaseFormModel {
    type Input = ReleaseFormMsg;
    type Output = ReleaseFormOutputMsg;
    type CommandOutput = CommandMsg;
    type Init = ();
    type Widgets = Widgets;
    type Root = gtk::Window;

    /*view! {
        #[root]
        gtk::Window {
            set_title: Some("Release Form"),
            gtk::Box {
                gtk::Label {
                    set_label: "Release Form Component",
                },
                gtk::DropDown {
                },
            }
        }
    }*/

    fn init_root() -> Self::Root {
        gtk::Window::builder()
            .title("Release Form")
            .default_width(800)
            .default_height(800)
            .build()
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        //let widgets = view_output!();
        let v_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        let label = gtk::Label::new(Some("Release Form Component"));

        v_box.append(&label);

        root.set_child(Some(&v_box));

        let widgets = Widgets {};

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
