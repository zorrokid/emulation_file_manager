use std::sync::Arc;

use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller,
    gtk::{
        self,
        prelude::{ButtonExt, OrientableExt, WidgetExt},
    },
    typed_view::list::TypedListView,
};
use service::{app_services::AppServices, view_models::SystemListModel};

use crate::{
    list_item::ListItem,
    system_selector::{
        SystemSelectInit, SystemSelectModel, SystemSelectMsg, SystemSelectOutputMsg,
    },
    utils::typed_list_view_utils::{get_item_ids, remove_selected},
};

#[derive(Debug)]
pub enum SystemListMsg {
    OpenSelector,
    SystemSelected(SystemListModel),
    UnlinkSystem,
    ResetItems { items: Vec<SystemListModel> },
}

#[derive(Debug)]
pub enum SystemListOutputMsg {
    ItemsChanged { system_ids: Vec<i64> },
}

pub struct SystemListInit {
    pub app_services: Arc<AppServices>,
}

#[derive(Debug)]
pub struct SystemList {
    app_services: Arc<AppServices>,
    system_selector: Controller<SystemSelectModel>,
    selected_systems_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
}

#[relm4::component(pub)]
impl Component for SystemList {
    type Input = SystemListMsg;
    type Output = SystemListOutputMsg;
    type CommandOutput = ();
    type Init = SystemListInit;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,

            gtk::ScrolledWindow {
                set_hexpand: true,
                #[local_ref]
                selected_systems_list_view -> gtk::ListView {}
            },
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_width_request: 250,
                add_css_class: "button-group",

                gtk::Button {
                    set_label: "Select System",
                    connect_clicked => SystemListMsg::OpenSelector,
                },
                gtk::Button {
                    set_label: "Unlink System",
                    connect_clicked => SystemListMsg::UnlinkSystem,
                },
            },
        }
    }

    fn init(
        init_model: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let selected_systems_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection> =
            TypedListView::new();

        let system_selector_init_model = SystemSelectInit {
            app_services: Arc::clone(&init_model.app_services),
        };

        let system_selector = SystemSelectModel::builder()
            .transient_for(&root)
            .launch(system_selector_init_model)
            .forward(sender.input_sender(), |msg| match msg {
                SystemSelectOutputMsg::SystemSelected(system_list_model) => {
                    SystemListMsg::SystemSelected(system_list_model)
                }
            });

        let model = SystemList {
            app_services: init_model.app_services,
            system_selector,
            selected_systems_list_view_wrapper,
        };

        let selected_systems_list_view = &model.selected_systems_list_view_wrapper.view;

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            SystemListMsg::OpenSelector => {
                self.system_selector.emit(SystemSelectMsg::Show {
                    selected_system_ids: get_item_ids(&self.selected_systems_list_view_wrapper),
                });
            }
            SystemListMsg::SystemSelected(system) => {
                self.selected_systems_list_view_wrapper.append(ListItem {
                    name: system.name.clone(),
                    id: system.id,
                });
                self.notify_items_changed(&sender);
            }
            SystemListMsg::UnlinkSystem => {
                remove_selected(&mut self.selected_systems_list_view_wrapper);
                self.notify_items_changed(&sender);
            }
            SystemListMsg::ResetItems { items } => {
                self.selected_systems_list_view_wrapper.clear();
                self.selected_systems_list_view_wrapper
                    .extend_from_iter(items.iter().map(|s| ListItem {
                        id: s.id,
                        name: s.name.clone(),
                    }));
                self.notify_items_changed(&sender);
            }
        }
    }
}

impl SystemList {
    fn notify_items_changed(&self, sender: &ComponentSender<Self>) {
        let system_ids = get_item_ids(&self.selected_systems_list_view_wrapper);
        sender
            .output(SystemListOutputMsg::ItemsChanged { system_ids })
            .unwrap_or_else(|err| {
                tracing::error!(error = ?err, "Error sending output message");
            });
    }
}
