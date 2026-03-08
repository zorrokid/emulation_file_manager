use std::sync::Arc;

use relm4::{
    Component, ComponentParts, ComponentSender,
    gtk::{
        self, glib,
        prelude::{GtkWindowExt, WidgetExt},
    },
};
use service::app_services::AppServices;

use crate::utils::dialog_utils::show_error_dialog;

#[derive(Debug)]
pub struct LibretroCoresDialog {
    pub app_services: Arc<AppServices>,
    pub available_cores: Vec<String>,
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
            available_cores: Vec::new(),
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            LibretroCoresDialogMsg::Show => {
                root.show();
                // TODO: should be async
                let available_cores_result = self.app_services.libretro_core().list_cores();
                match available_cores_result {
                    Ok(cores) => {
                        tracing::info!(cores = ?cores, "Available libretro cores");
                        self.available_cores = cores;
                    }
                    Err(e) => {
                        tracing::error!(error = ?e, "Failed to list libretro cores");
                        show_error_dialog("Failed to list libretro cores".into(), root);
                    }
                }
            }
            LibretroCoresDialogMsg::Hide => {
                root.hide();
            }
        }
    }
}

impl LibretroCoresDialog {}
