use gtk::prelude::*;
use relm4::{prelude::*, Sender};

pub struct ConfirmDialog {
    title: String,
}

#[derive(Debug)]
pub enum ConfirmDialogMsg {
    Accept,
    Cancel,
}

pub struct ConfirmDialogInit {
    pub title: String,
}

#[relm4::component(pub)]
impl SimpleComponent for ConfirmDialog {
    type Init = ConfirmDialogInit;
    type Input = ConfirmDialogMsg;
    type Output = bool;
    type Widgets = ConfirmDialogWidgets;

    view! {
        #[root]
        dialog = gtk::MessageDialog {
            set_margin_all: 10,
            set_modal: true,
            set_text: Some(model.title.as_str()),
            add_button: ("Cancel", gtk::ResponseType::Cancel),
            add_button: ("Confirm", gtk::ResponseType::Accept),
            present: (),

            connect_response[sender] => move |dialog, resp| {
                dialog.set_visible(false);
                sender.input(if resp == gtk::ResponseType::Accept {
                    ConfirmDialogMsg::Accept
                } else {
                    ConfirmDialogMsg::Cancel
                });
            }
        },
        dialog.content_area() -> gtk::Box {
            gtk::Label{
                set_label: "Are you sure?",
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = ConfirmDialog { title: init.title };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            ConfirmDialogMsg::Accept => {
                sender.output(true).unwrap();
            }
            ConfirmDialogMsg::Cancel => {
                sender.output(false).unwrap();
            }
        }
    }

    fn shutdown(&mut self, _widgets: &mut Self::Widgets, _output: Sender<Self::Output>) {
        println!("ConfirmDialog shutdown");
    }
}
