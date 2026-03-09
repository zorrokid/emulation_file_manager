use std::sync::Arc;

use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
    gtk::{
        self, glib,
        prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt},
    },
};
use service::{
    app_services::AppServices,
    libretro_core::service::SystemCoreMappingModel,
};
use ui_components::string_list_view::{
    StringListView, StringListViewInit, StringListViewMsg, StringListViewOutput,
};

use crate::utils::dialog_utils::show_error_dialog;

#[derive(Debug)]
pub struct LibretroCoresDialog {
    pub app_services: Arc<AppServices>,
    available_cores_list: Controller<StringListView>,
    mapped_systems_list: Controller<StringListView>,
    mapped_systems: Vec<SystemCoreMappingModel>,
}

pub struct LibretroCoredDialogInit {
    pub app_services: Arc<AppServices>,
}

#[derive(Debug)]
pub enum LibretroCoresDialogMsg {
    Show,
    Hide,
    AvailableCoreSelected(Option<String>),
    MappedSystemSelected(Option<String>),
}

#[derive(Debug)]
pub enum LibretroCoresDialogCmd {
    CoresFetched(Result<Vec<String>, service::error::Error>),
    MappedSystemsFetched(Result<Vec<SystemCoreMappingModel>, service::error::Error>),
}

#[relm4::component(pub)]
impl Component for LibretroCoresDialog {
    type Init = LibretroCoredDialogInit;
    type Input = LibretroCoresDialogMsg;
    type Output = ();
    type CommandOutput = LibretroCoresDialogCmd;

    view! {
        #[root]
        gtk::Window {
            set_default_width: 700,
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
                    model.mapped_systems_list.widget(),
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

        let mapped_systems_list = StringListView::builder()
            .launch(StringListViewInit {
                title: "Mapped Systems".to_string(),
            })
            .forward(sender.input_sender(), |msg| match msg {
                StringListViewOutput::SelectionChanged(name) => {
                    LibretroCoresDialogMsg::MappedSystemSelected(name)
                }
            });

        let model = LibretroCoresDialog {
            app_services: init.app_services,
            available_cores_list,
            mapped_systems_list,
            mapped_systems: Vec::new(),
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
                    LibretroCoresDialogCmd::CoresFetched(
                        app_services.libretro_core().list_cores(),
                    )
                });
            }
            LibretroCoresDialogMsg::Hide => {
                root.hide();
            }
            LibretroCoresDialogMsg::AvailableCoreSelected(Some(core_name)) => {
                let app_services = Arc::clone(&self.app_services);
                sender.oneshot_command(async move {
                    LibretroCoresDialogCmd::MappedSystemsFetched(
                        app_services
                            .libretro_core()
                            .get_systems_for_core(&core_name)
                            .await,
                    )
                });
            }
            LibretroCoresDialogMsg::AvailableCoreSelected(None) => {
                self.mapped_systems_list
                    .emit(StringListViewMsg::SetItems(vec![]));
                self.mapped_systems.clear();
            }
            LibretroCoresDialogMsg::MappedSystemSelected(_name) => {
                // TODO: used to enable/disable Remove button
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match msg {
            LibretroCoresDialogCmd::CoresFetched(Ok(cores)) => {
                self.available_cores_list
                    .emit(StringListViewMsg::SetItems(cores));
                self.mapped_systems_list
                    .emit(StringListViewMsg::SetItems(vec![]));
                self.mapped_systems.clear();
            }
            LibretroCoresDialogCmd::CoresFetched(Err(e)) => {
                tracing::error!(error = ?e, "Failed to list libretro cores");
                show_error_dialog("Failed to list libretro cores".into(), root);
            }
            LibretroCoresDialogCmd::MappedSystemsFetched(Ok(systems)) => {
                let names = systems.iter().map(|s| s.system_name.clone()).collect();
                self.mapped_systems = systems;
                self.mapped_systems_list
                    .emit(StringListViewMsg::SetItems(names));
            }
            LibretroCoresDialogCmd::MappedSystemsFetched(Err(e)) => {
                tracing::error!(error = ?e, "Failed to fetch mapped systems");
                show_error_dialog("Failed to fetch mapped systems".into(), root);
            }
        }
    }
}
