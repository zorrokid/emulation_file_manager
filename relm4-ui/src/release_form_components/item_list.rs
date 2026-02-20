use std::sync::Arc;

use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller,
    gtk::{
        self,
        glib::clone,
        prelude::{ButtonExt, OrientableExt, WidgetExt},
    },
    once_cell::sync::OnceCell,
    typed_view::list::TypedListView,
};
use service::{app_services::AppServices, error::Error as ServiceError, view_models::ReleaseItemListModel};

use crate::{
    list_item::ListItem,
    release_form_components::item_form::{ItemForm, ItemFormInit, ItemFormMsg, ItemFormOutputMsg},
    utils::typed_list_view_utils::{get_item_ids, get_selected_item_id, remove_by_id},
};

#[derive(Debug)]
pub enum ItemListMsg {
    AddItem,
    EditItem,
    RemoveItem,
    ResetItems { items: Vec<ListItem> },
    SetReleaseId { release_id: Option<i64> },
    ItemAdded(ReleaseItemListModel),
    ItemUpdated(ReleaseItemListModel),
    SelectionChanged,
}

#[derive(Debug)]
pub enum ItemListOutputMsg {
    ItemsChanged { item_ids: Vec<i64> },
}

#[derive(Debug)]
pub enum ItemListCommandMsg {
    ProcessDeleteItemResult(Result<(), ServiceError>),
}

pub struct ItemListInit {
    pub app_services: Arc<AppServices>,
    pub release_id: Option<i64>,
}

#[derive(Debug)]
pub struct ItemList {
    app_services: Arc<AppServices>,
    release_id: Option<i64>,
    selected_items_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    item_form: OnceCell<Controller<ItemForm>>,
    selected_item: Option<i64>,
}

#[relm4::component(pub)]
impl Component for ItemList {
    type Input = ItemListMsg;
    type Output = ItemListOutputMsg;
    type CommandOutput = ItemListCommandMsg;
    type Init = ItemListInit;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,

            if model.release_id.is_none() {
                gtk::Label {
                    set_label: "Please create a release first to manage its items.",
                }
            } else {
                gtk::ScrolledWindow {
                    set_min_content_height: 360,
                    set_hexpand: true,
                    #[local_ref]
                    selected_items_list_view -> gtk::ListView {}
                }
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
                    #[watch]
                    set_sensitive: model.selected_item.is_some(),
                },
                gtk::Button {
                    set_label: "Delete Item",
                    connect_clicked => ItemListMsg::RemoveItem,
                    #[watch]
                    set_sensitive:  model.selected_item.is_some(),
                },
            },
        }
    }

    fn init(
        init_model: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let selected_items_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection> =
            TypedListView::new();

        let model = ItemList {
            app_services: init_model.app_services,
            release_id: init_model.release_id,
            selected_items_list_view_wrapper,
            item_form: OnceCell::new(),
            selected_item: None,
        };

        let selected_items_list_view = &model.selected_items_list_view_wrapper.view;
        let selection_model = &model.selected_items_list_view_wrapper.selection_model;
        selection_model.connect_selected_notify(clone!(
            #[strong]
            sender,
            move |_| {
                sender.input(ItemListMsg::SelectionChanged);
            }
        ));

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            ItemListMsg::AddItem => {
                if let Some(release_id) = self.release_id {
                    tracing::info!(release_id, "Opening item form to add new item");
                    self.ensure_item_form(root, &sender);
                    self.item_form
                        .get()
                        .expect("Item form should be initialized")
                        .emit(ItemFormMsg::Show {
                            release_id,
                            edit_item_id: None,
                        });
                }
            }
            ItemListMsg::EditItem => {
                if let (Some(item_id), Some(release_id)) = (self.selected_item, self.release_id) {
                    tracing::info!(item_id, "Opening item form to edit item");
                    self.ensure_item_form(root, &sender);
                    self.item_form
                        .get()
                        .expect("Item form should be initialized")
                        .emit(ItemFormMsg::Show {
                            release_id,
                            edit_item_id: Some(item_id),
                        });
                }
            }
            ItemListMsg::RemoveItem => {
                if let Some(item_id) = self.selected_item {
                    remove_by_id(&mut self.selected_items_list_view_wrapper, item_id);
                    let app_services = Arc::clone(&self.app_services);
                    sender.oneshot_command(async move {
                        tracing::info!(item_id, "Removing release item with ID");
                        let result = app_services.release_item.delete_item(item_id).await;
                        ItemListCommandMsg::ProcessDeleteItemResult(result)
                    });
                }
            }
            ItemListMsg::ResetItems { items } => {
                self.selected_items_list_view_wrapper.clear();
                self.selected_items_list_view_wrapper
                    .extend_from_iter(items);
                self.notify_items_changed(&sender);
            }
            ItemListMsg::SetReleaseId { release_id } => {
                self.release_id = release_id;
            }
            ItemListMsg::ItemAdded(item) => {
                self.selected_items_list_view_wrapper.append(ListItem {
                    id: item.id,
                    name: item.item_type.to_string(),
                });
                self.notify_items_changed(&sender);
            }
            ItemListMsg::ItemUpdated(item) => {
                // Update the item in the list by removing and re-inserting
                for i in 0..self.selected_items_list_view_wrapper.len() {
                    if let Some(list_item) = self.selected_items_list_view_wrapper.get(i)
                        && list_item.borrow().id == item.id
                    {
                        tracing::info!(item_id = item.id, "Updating item in the list");
                        let updated_item = ListItem {
                            id: item.id,
                            name: item.item_type.to_string(),
                        };
                        self.selected_items_list_view_wrapper.remove(i);
                        self.selected_items_list_view_wrapper
                            .insert(i, updated_item);
                        break;
                    }
                }
                self.notify_items_changed(&sender);
            }
            ItemListMsg::SelectionChanged => {
                let selected_item_id = get_selected_item_id(&self.selected_items_list_view_wrapper);
                tracing::info!(selected_item_id = ?selected_item_id, "Item selection changed");
                self.selected_item = selected_item_id;
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            ItemListCommandMsg::ProcessDeleteItemResult(result) => {
                match result {
                    Ok(_) => {
                        tracing::info!("Item deleted successfully");
                        self.notify_items_changed(&sender);
                    }
                    Err(err) => {
                        tracing::error!(error = ?err, "Failed to delete item");
                        // TODO: show error dialog
                    }
                }
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

    fn ensure_item_form(&self, root: &gtk::Box, sender: &ComponentSender<Self>) {
        if self.item_form.get().is_none() {
            let app_services = Arc::clone(&self.app_services);
            let item_form_init = ItemFormInit { app_services };

            let item_form = ItemForm::builder()
                .transient_for(root)
                .launch(item_form_init)
                .forward(sender.input_sender(), |msg| match msg {
                    ItemFormOutputMsg::ItemAdded(item) => ItemListMsg::ItemAdded(item),
                    ItemFormOutputMsg::ItemUpdated(item) => ItemListMsg::ItemUpdated(item),
                    _ => unreachable!(),
                });
            self.item_form.set(item_form).unwrap_or_else(|err| {
                tracing::error!(
                        error = ?err,
                        "Failed to set item_form in ItemList")
            });
        }
    }
}
