use std::sync::Arc;

use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
    gtk::{
        self, glib,
        prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt},
    },
};
use service::app_services::AppServices;
use ui_components::string_list_view::{
    StringListView, StringListViewInit, StringListViewMsg, StringListViewOutput,
};

use crate::utils::dialog_utils::show_error_dialog;

#[derive(Debug)]
pub struct LibretroCoresDialog {
    pub app_services: Arc<AppServices>,
    available_cores_list: Controller<StringListView>,
}

pub struct LibretroCoredDialogInit {
    pub app_services: Arc<AppServices>,
}

#[derive(Debug)]
pub enum LibretroCoresDialogMsg {
    Show,
    Hide,
    CoresFetched(Result<Vec<String>, service::error::Error>),
    AvailableCoreSelected(Option<String>),
}

#[relm4::component(pub)]
impl Component for LibretroCoresDialog {
    type Init = LibretroCoredDialogInit;
    type Input = LibretroCoresDialogMsg;
    type Output = ();
    type CommandOutput = LibretroCoresDialogMsg;

    view! {
        #[root]
        gtk::Window {
            set_default_width: 600,
            set_default_height: 500,
            set_title: Some("Manage Core Mappings"),
            connect_close_request[sender] => move |_| {
                sender.input(LibretroCoresDialogMsg::Hide);
                glib::Propagation::Proceed
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 10,
                set_spacing: 10,

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 10,
                    set_vexpand: true,

                    model.available_cores_list.widget(),
                },

                gtk::Button {
                    set_label: "Close",
                    set_halign: gtk::Align::End,
                    connect_clicked => LibretroCoresDialogMsg::Hide,
                },
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let available_cores_list = StringListView::builder()
            .launch(StringListViewInit {
                title: "Available Cores".to_string(),
            })
            .forward(sender.input_sender(), |msg| match msg {
                StringListViewOutput::SelectionChanged(name) => {
                    LibretroCoresDialogMsg::AvailableCoreSelected(name)
                }
            });

        let model = LibretroCoresDialog {
            app_services: init.app_services,
            available_cores_list,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(
        &mut self,
        msg: Self::Input,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match msg {
            LibretroCoresDialogMsg::Show => {
                root.show();
                let app_services = Arc::clone(&self.app_services);
                sender.oneshot_command(async move {
                    let result = app_services.libretro_core().list_cores();
                    LibretroCoresDialogMsg::CoresFetched(result)
                });
            }
            LibretroCoresDialogMsg::Hide => {
                root.hide();
            }
            LibretroCoresDialogMsg::CoresFetched(Ok(cores)) => {
                self.available_cores_list
                    .emit(StringListViewMsg::SetItems(cores));
            }
            LibretroCoresDialogMsg::CoresFetched(Err(e)) => {
                tracing::error!(error = ?e, "Failed to list libretro cores");
                show_error_dialog("Failed to list libretro cores".into(), root);
            }
            LibretroCoresDialogMsg::AvailableCoreSelected(_name) => {
                // TODO: load mapped systems for selected core
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: Self::CommandOutput,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        self.update(msg, sender, root);
    }
}
