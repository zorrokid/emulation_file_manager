use std::sync::Arc;

use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
    gtk::{
        self,
        glib::{self, clone},
        prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt},
    },
    typed_view::list::TypedListView,
};
use service::{app_services::AppServices, libretro_core::service::SystemCoreMappingModel};
use ui_components::string_list_view::{
    StringListView, StringListViewInit, StringListViewMsg, StringListViewOutput,
};

use crate::{
    list_item::ListItem,
    system_selector::{
        SystemSelectInit, SystemSelectModel, SystemSelectMsg, SystemSelectOutputMsg,
    },
    utils::dialog_utils::show_error_dialog,
};

#[derive(Debug)]
pub struct LibretroCoresDialog {
    app_services: Arc<AppServices>,
    available_cores_list: Controller<StringListView<String>>,
    mapped_systems_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    system_selector: Controller<SystemSelectModel>,
    mapped_systems: Vec<SystemCoreMappingModel>,
    selected_core: Option<String>,
    selected_mapping_id: Option<i64>,
}

pub struct LibretroCoredDialogInit {
    pub app_services: Arc<AppServices>,
}

#[derive(Debug)]
pub enum LibretroCoresDialogMsg {
    Show,
    Hide,
    AvailableCoreSelected(Option<String>),
    MappedSystemSelectionChanged,
    AddSystemClicked,
    RemoveSystemClicked,
    SystemChosen(service::view_models::SystemListModel),
}

#[derive(Debug)]
pub enum LibretroCoresDialogCmd {
    CoresFetched(Result<Vec<String>, service::error::Error>),
    MappedSystemsFetched(Result<Vec<SystemCoreMappingModel>, service::error::Error>),
    MappingAdded(Result<i64, service::error::Error>),
    MappingRemoved(Result<(), service::error::Error>),
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

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 4,
                        set_hexpand: true,

                        gtk::Label {
                            set_label: "Mapped Systems",
                            set_xalign: 0.0,
                        },
                        gtk::ScrolledWindow {
                            set_vexpand: true,
                            #[local_ref]
                            mapped_systems_view -> gtk::ListView {},
                        },
                    },
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 6,
                    set_halign: gtk::Align::End,

                    gtk::Button {
                        set_label: "Add System",
                        #[watch]
                        set_sensitive: model.selected_core.is_some(),
                        connect_clicked => LibretroCoresDialogMsg::AddSystemClicked,
                    },
                    gtk::Button {
                        set_label: "Remove System",
                        #[watch]
                        set_sensitive: model.selected_mapping_id.is_some(),
                        connect_clicked => LibretroCoresDialogMsg::RemoveSystemClicked,
                    },
                    gtk::Button {
                        set_label: "Close",
                        connect_clicked => LibretroCoresDialogMsg::Hide,
                    },
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

        let mapped_systems_wrapper: TypedListView<ListItem, gtk::SingleSelection> =
            TypedListView::new();

        mapped_systems_wrapper
            .selection_model
            .connect_selected_notify(clone!(
                #[strong]
                sender,
                move |_| sender.input(LibretroCoresDialogMsg::MappedSystemSelectionChanged)
            ));

        let system_selector = SystemSelectModel::builder()
            .transient_for(&root)
            .launch(SystemSelectInit {
                app_services: Arc::clone(&init.app_services),
            })
            .forward(sender.input_sender(), |msg| match msg {
                SystemSelectOutputMsg::SystemSelected(system) => {
                    LibretroCoresDialogMsg::SystemChosen(system)
                }
            });

        let model = LibretroCoresDialog {
            app_services: init.app_services,
            available_cores_list,
            mapped_systems_wrapper,
            system_selector,
            mapped_systems: Vec::new(),
            selected_core: None,
            selected_mapping_id: None,
        };

        let mapped_systems_view = &model.mapped_systems_wrapper.view;
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            LibretroCoresDialogMsg::Show => {
                root.show();
                let app_services = Arc::clone(&self.app_services);
                sender.oneshot_command(async move {
                    LibretroCoresDialogCmd::CoresFetched(app_services.libretro_core().list_cores())
                });
            }
            LibretroCoresDialogMsg::Hide => {
                root.hide();
            }
            LibretroCoresDialogMsg::AvailableCoreSelected(core_name) => {
                self.selected_core = core_name.clone();
                self.selected_mapping_id = None;
                match core_name {
                    Some(core_name) => {
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
                    None => {
                        self.mapped_systems_wrapper.clear();
                        self.mapped_systems.clear();
                    }
                }
            }
            LibretroCoresDialogMsg::MappedSystemSelectionChanged => {
                let idx = self.mapped_systems_wrapper.selection_model.selected();
                self.selected_mapping_id = self
                    .mapped_systems_wrapper
                    .get_visible(idx)
                    .map(|item| item.borrow().id);
            }
            LibretroCoresDialogMsg::AddSystemClicked => {
                let already_mapped: Vec<i64> =
                    self.mapped_systems.iter().map(|s| s.system_id).collect();
                self.system_selector.emit(SystemSelectMsg::Show {
                    selected_system_ids: already_mapped,
                });
            }
            LibretroCoresDialogMsg::RemoveSystemClicked => {
                if let Some(mapping_id) = self.selected_mapping_id {
                    let app_services = Arc::clone(&self.app_services);
                    sender.oneshot_command(async move {
                        LibretroCoresDialogCmd::MappingRemoved(
                            app_services
                                .libretro_core()
                                .remove_core_mapping(mapping_id)
                                .await,
                        )
                    });
                }
            }
            LibretroCoresDialogMsg::SystemChosen(system) => {
                if let Some(core_name) = self.selected_core.clone() {
                    let app_services = Arc::clone(&self.app_services);
                    sender.oneshot_command(async move {
                        LibretroCoresDialogCmd::MappingAdded(
                            app_services
                                .libretro_core()
                                .add_core_mapping(system.id, &core_name)
                                .await,
                        )
                    });
                }
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: Self::CommandOutput,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match msg {
            LibretroCoresDialogCmd::CoresFetched(Ok(cores)) => {
                self.available_cores_list
                    .emit(StringListViewMsg::SetItems(cores));
                self.mapped_systems_wrapper.clear();
                self.mapped_systems.clear();
                self.selected_core = None;
                self.selected_mapping_id = None;
            }
            LibretroCoresDialogCmd::CoresFetched(Err(e)) => {
                tracing::error!(error = ?e, "Failed to list libretro cores");
                show_error_dialog("Failed to list libretro cores".into(), root);
            }
            LibretroCoresDialogCmd::MappedSystemsFetched(Ok(systems)) => {
                self.mapped_systems_wrapper.clear();
                self.mapped_systems_wrapper
                    .extend_from_iter(systems.iter().map(|s| ListItem {
                        id: s.id,
                        name: s.system_name.clone(),
                    }));
                self.mapped_systems = systems;
                self.selected_mapping_id = None;
            }
            LibretroCoresDialogCmd::MappedSystemsFetched(Err(e)) => {
                tracing::error!(error = ?e, "Failed to fetch mapped systems");
                show_error_dialog("Failed to fetch mapped systems".into(), root);
            }
            LibretroCoresDialogCmd::MappingAdded(Ok(_)) => {
                if let Some(core_name) = self.selected_core.clone() {
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
            }
            LibretroCoresDialogCmd::MappingAdded(Err(e)) => {
                tracing::error!(error = ?e, "Failed to add core mapping");
                show_error_dialog(format!("Failed to add mapping: {e}"), root);
            }
            LibretroCoresDialogCmd::MappingRemoved(Ok(())) => {
                self.selected_mapping_id = None;
                if let Some(core_name) = self.selected_core.clone() {
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
            }
            LibretroCoresDialogCmd::MappingRemoved(Err(e)) => {
                tracing::error!(error = ?e, "Failed to remove core mapping");
                show_error_dialog(format!("Failed to remove mapping: {e}"), root);
            }
        }
    }
}
