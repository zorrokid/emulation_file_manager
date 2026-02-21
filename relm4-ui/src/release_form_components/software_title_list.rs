use std::sync::Arc;

use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller,
    gtk::{
        self,
        prelude::{ButtonExt, OrientableExt, WidgetExt},
    },
    typed_view::list::TypedListView,
};
use service::{app_services::AppServices, view_models::SoftwareTitleListModel};

use crate::{
    list_item::ListItem,
    software_title_selector::{
        SoftwareTitleSelectInit, SoftwareTitleSelectModel, SoftwareTitleSelectMsg,
        SoftwareTitleSelectOutputMsg,
    },
    utils::typed_list_view_utils::{get_item_ids, remove_selected},
};

#[derive(Debug)]
pub enum SoftwareTitleListMsg {
    OpenSelector,
    SoftwareTitleSelected(SoftwareTitleListModel),
    SoftwareTitleCreated(SoftwareTitleListModel),
    SoftwareTitleUpdated(SoftwareTitleListModel),
    UnlinkSoftwareTitle,
    ResetItems { items: Vec<SoftwareTitleListModel> },
}

#[derive(Debug)]
pub enum SoftwareTitleListOutputMsg {
    ItemsChanged { software_title_ids: Vec<i64> },
    SoftwareTitleCreated(SoftwareTitleListModel),
    SoftwareTitleUpdated(SoftwareTitleListModel),
}

pub struct SoftwareTitleListInit {
    pub app_services: Arc<AppServices>,
}

#[derive(Debug)]
pub struct SoftwareTitleList {
    app_services: Arc<AppServices>,
    software_title_selector: Controller<SoftwareTitleSelectModel>,
    selected_software_titles_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
}

#[relm4::component(pub)]
impl Component for SoftwareTitleList {
    type Input = SoftwareTitleListMsg;
    type Output = SoftwareTitleListOutputMsg;
    type CommandOutput = ();
    type Init = SoftwareTitleListInit;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,

            gtk::ScrolledWindow {
                set_hexpand: true,
                #[local_ref]
                selected_software_titles_list_view -> gtk::ListView {}
            },
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_width_request: 250,
                add_css_class: "button-group",

                gtk::Button {
                    set_label: "Select Software Title",
                    connect_clicked => SoftwareTitleListMsg::OpenSelector,
                },
                gtk::Button {
                    set_label: "Unlink Software Title",
                    connect_clicked => SoftwareTitleListMsg::UnlinkSoftwareTitle,
                },
            },
        }
    }

    fn init(
        init_model: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let selected_software_titles_list_view_wrapper: TypedListView<
            ListItem,
            gtk::SingleSelection,
        > = TypedListView::new();

        let software_title_selector_init = SoftwareTitleSelectInit {
            app_services: Arc::clone(&init_model.app_services),
        };

        let software_title_selector = SoftwareTitleSelectModel::builder()
            .transient_for(&root)
            .launch(software_title_selector_init)
            .forward(sender.input_sender(), |msg| match msg {
                SoftwareTitleSelectOutputMsg::Selected(software_title) => {
                    SoftwareTitleListMsg::SoftwareTitleSelected(software_title)
                }
                SoftwareTitleSelectOutputMsg::Created(software_title) => {
                    SoftwareTitleListMsg::SoftwareTitleCreated(software_title)
                }
                SoftwareTitleSelectOutputMsg::Updated(software_title) => {
                    SoftwareTitleListMsg::SoftwareTitleUpdated(software_title)
                }
            });

        let model = SoftwareTitleList {
            app_services: init_model.app_services,
            software_title_selector,
            selected_software_titles_list_view_wrapper,
        };

        let selected_software_titles_list_view =
            &model.selected_software_titles_list_view_wrapper.view;

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            SoftwareTitleListMsg::OpenSelector => {
                self.software_title_selector
                    .emit(SoftwareTitleSelectMsg::Show {
                        selected_software_title_ids: get_item_ids(
                            &self.selected_software_titles_list_view_wrapper,
                        ),
                    });
            }
            SoftwareTitleListMsg::SoftwareTitleSelected(software_title) => {
                self.selected_software_titles_list_view_wrapper
                    .append(ListItem {
                        name: software_title.name.clone(),
                        id: software_title.id,
                    });
                self.notify_items_changed(&sender);
            }
            SoftwareTitleListMsg::SoftwareTitleCreated(software_title) => {
                sender
                    .output(SoftwareTitleListOutputMsg::SoftwareTitleCreated(
                        software_title,
                    ))
                    .unwrap_or_else(|err| {
                        tracing::error!(error = ?err, "Error sending output message");
                    });
            }
            SoftwareTitleListMsg::SoftwareTitleUpdated(software_title) => {
                sender
                    .output(SoftwareTitleListOutputMsg::SoftwareTitleUpdated(
                        software_title,
                    ))
                    .unwrap_or_else(|err| {
                        tracing::error!(error = ?err, "Error sending output message");
                    });
            }
            SoftwareTitleListMsg::UnlinkSoftwareTitle => {
                remove_selected(&mut self.selected_software_titles_list_view_wrapper);
                self.notify_items_changed(&sender);
            }
            SoftwareTitleListMsg::ResetItems { items } => {
                self.selected_software_titles_list_view_wrapper.clear();
                self.selected_software_titles_list_view_wrapper
                    .extend_from_iter(items.iter().map(|st| ListItem {
                        id: st.id,
                        name: st.name.clone(),
                    }));
                self.notify_items_changed(&sender);
            }
        }
    }
}

impl SoftwareTitleList {
    fn notify_items_changed(&self, sender: &ComponentSender<Self>) {
        let software_title_ids = get_item_ids(&self.selected_software_titles_list_view_wrapper);
        sender
            .output(SoftwareTitleListOutputMsg::ItemsChanged { software_title_ids })
            .unwrap_or_else(|err| {
                tracing::error!(error = ?err, "Error sending output message");
            });
    }
}
