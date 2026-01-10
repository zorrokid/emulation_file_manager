use relm4::{
    Component, ComponentParts, ComponentSender,
    gtk::{
        self,
        prelude::{ButtonExt, OrientableExt, WidgetExt},
    },
    typed_view::list::TypedListView,
};

use crate::{
    list_item::ListItem,
    release_form::{get_item_ids, remove_selected},
};

#[derive(Debug)]
pub enum ItemListMsg {
    AddItem,
    EditItem,
    RemoveItem,
    ResetItems { items: Vec<ListItem> },
    SetReleaseId { release_id: Option<i64> },
}

#[derive(Debug)]
pub enum ItemListOutputMsg {
    ItemsChanged { item_ids: Vec<i64> },
    AddItem,
    EditItem { item_id: i64 },
    RemoveItem { item_id: i64 },
}

pub struct ItemListInit {
    pub release_id: Option<i64>,
}

#[derive(Debug)]
pub struct ItemList {
    release_id: Option<i64>,
    selected_items_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
}

#[relm4::component(pub)]
impl Component for ItemList {
    type Input = ItemListMsg;
    type Output = ItemListOutputMsg;
    type CommandOutput = ();
    type Init = ItemListInit;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,

            gtk::ScrolledWindow {
                set_min_content_height: 360,
                set_hexpand: true,
                #[local_ref]
                selected_items_list_view -> gtk::ListView {}
            },
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_width_request: 250,
                add_css_class: "button-group",

                gtk::Button {
                    set_label: "Add Item",
                    #[watch]
                    set_sensitive: model.release_id.is_some(),
                    connect_clicked => ItemListMsg::AddItem,
                },
                gtk::Button {
                    set_label: "Edit Item",
                    connect_clicked => ItemListMsg::EditItem,
                },
                gtk::Button {
                    set_label: "Delete Item",
                    connect_clicked => ItemListMsg::RemoveItem,
                },
            },
        }
    }

    fn init(
        init_model: Self::Init,
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let selected_items_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection> =
            TypedListView::new();

        let model = ItemList {
            release_id: init_model.release_id,
            selected_items_list_view_wrapper,
        };

        let selected_items_list_view = &model.selected_items_list_view_wrapper.view;

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            ItemListMsg::AddItem => {
                sender
                    .output(ItemListOutputMsg::AddItem)
                    .unwrap_or_else(|err| {
                        tracing::error!(error = ?err, "Error sending output message");
                    });
            }
            ItemListMsg::EditItem => {
                if let Some(item_id) =
                    get_selected_item_id(&self.selected_items_list_view_wrapper)
                {
                    sender
                        .output(ItemListOutputMsg::EditItem { item_id })
                        .unwrap_or_else(|err| {
                            tracing::error!(error = ?err, "Error sending output message");
                        });
                }
            }
            ItemListMsg::RemoveItem => {
                if let Some(item_id) =
                    get_selected_item_id(&self.selected_items_list_view_wrapper)
                {
                    remove_selected(&mut self.selected_items_list_view_wrapper);
                    sender
                        .output(ItemListOutputMsg::RemoveItem { item_id })
                        .unwrap_or_else(|err| {
                            tracing::error!(error = ?err, "Error sending output message");
                        });
                    self.notify_items_changed(&sender);
                }
            }
            ItemListMsg::ResetItems { items } => {
                self.selected_items_list_view_wrapper.clear();
                self.selected_items_list_view_wrapper
                    .extend_from_iter(items.into_iter());
                self.notify_items_changed(&sender);
            }
            ItemListMsg::SetReleaseId { release_id } => {
                self.release_id = release_id;
            }
        }
    }
}

impl ItemList {
    fn notify_items_changed(&self, sender: &ComponentSender<Self>) {
        let item_ids = get_item_ids(&self.selected_items_list_view_wrapper);
        sender
            .output(ItemListOutputMsg::ItemsChanged { item_ids })
            .unwrap_or_else(|err| {
                tracing::error!(error = ?err, "Error sending output message");
            });
    }
}

fn get_selected_item_id(
    list_view_wrapper: &TypedListView<ListItem, gtk::SingleSelection>,
) -> Option<i64> {
    let selected_position = list_view_wrapper.selection_model.selected();
    list_view_wrapper
        .get(selected_position)
        .map(|item| item.borrow().id)
}
