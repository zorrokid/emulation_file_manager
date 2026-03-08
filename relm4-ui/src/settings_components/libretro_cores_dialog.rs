use std::sync::Arc;

use relm4::{
    Component, ComponentParts, ComponentSender,
    gtk::{
        self, glib,
        prelude::{GtkWindowExt, WidgetExt},
    },
};
use service::app_services::AppServices;

#[derive(Debug)]
pub struct LibretroCoresDialog {
    pub app_services: Arc<AppServices>,
}

pub struct LibretroCoredDialogInit {
    pub app_services: Arc<AppServices>,
}

#[derive(Debug)]
pub enum LibretroCoresDialogMsg {
    Show,
    Hide,
}

#[relm4::component(pub)]
impl Component for LibretroCoresDialog {
    type Init = LibretroCoredDialogInit;
    type Input = LibretroCoresDialogMsg;
    type Output = ();
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Window {
            set_default_width: 400,
            set_default_height: 600,
            set_title: Some("Libretro Cores"),
            connect_close_request[sender] => move |_| {
                sender.input(LibretroCoresDialogMsg::Hide);
                glib::Propagation::Proceed
            },

            gtk::Box {
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = LibretroCoresDialog {
            app_services: init.app_services,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            LibretroCoresDialogMsg::Show => {
                root.show();
            }
            LibretroCoresDialogMsg::Hide => {
                root.hide();
            }
        }
    }
}

impl LibretroCoresDialog {}
