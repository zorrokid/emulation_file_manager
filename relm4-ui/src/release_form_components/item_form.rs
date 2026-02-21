use std::sync::Arc;

use core_types::item_type::ItemType;
use domain::models::ReleaseItem;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
    gtk::{
        self, glib,
        prelude::{
            BoxExt, ButtonExt, EditableExt, EntryBufferExtManual, EntryExt, GtkWindowExt,
            OrientableExt, WidgetExt,
        },
    },
};
use service::{
    app_services::AppServices, error::Error as ServiceError, view_models::ReleaseItemListModel,
};
use ui_components::{
    DropDownMsg, DropDownOutputMsg,
    drop_down::{ItemTypeDropDown, ItemTypeSelectedMsg},
};

use crate::utils::dialog_utils::show_error_dialog;

#[derive(Debug)]
pub struct ItemForm {
    pub app_services: Arc<AppServices>,
    pub item_type: Option<ItemType>,
    pub notes: String,
    pub release_item_id: Option<i64>,
    pub release_id: Option<i64>,
    pub item_type_dropdown: Controller<ItemTypeDropDown>,
}

#[derive(Debug)]
pub enum ItemFormMsg {
    UpdateItemType(ItemType),
    UpdateNotes(String),
    CreateOrUpdateItem,
    Hide,
    Show {
        release_id: i64,
        edit_item_id: Option<i64>,
    },
    UpdateFields,
}

#[derive(Debug)]
pub enum ItemFormOutputMsg {
    ItemAdded(ReleaseItemListModel),
    ItemUpdated(ReleaseItemListModel),
}

#[derive(Debug)]
pub enum ItemFormCommandMsg {
    ItemSubmitted(Result<i64, ServiceError>),
    ProcessGetEditItemResult(Result<ReleaseItem, ServiceError>),
}

#[derive(Debug)]
pub struct ItemFormInit {
    pub app_services: Arc<AppServices>,
}

#[relm4::component(pub)]
impl Component for ItemForm {
    type Input = ItemFormMsg;
    type Output = ItemFormOutputMsg;
    type CommandOutput = ItemFormCommandMsg;
    type Init = ItemFormInit;

    view! {
        #[root]
        gtk::Window {
            set_default_width: 800,
            set_default_height: 600,
            set_margin_all: 10,

            #[watch]
            set_title: if model.release_item_id.is_some() {
                Some("Edit Release Item")
            } else {
                Some("Create Release Item")
            },

            connect_close_request[sender] => move |_| {
                sender.input(ItemFormMsg::Hide);
                glib::Propagation::Proceed
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 5,
                set_margin_all: 5,
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 5,

                    gtk::Label {
                        set_label: "Item Type:",
                    },

                    #[local_ref]
                    item_type_dropdown -> gtk::Box,
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 5,

                    gtk::Label {
                        set_label: "Notes:",
                    },

                    #[name = "notes_entry"]
                    gtk::Entry {
                        set_hexpand: true,
                        set_placeholder_text: Some("Enter notes about the item"),
                        connect_changed[sender] => move |entry| {
                            let buffer = entry.buffer();
                            sender.input(
                                ItemFormMsg::UpdateNotes(buffer.text().into()),
                            );
                        },
                    },
                },
                gtk::Button {
                    set_label: "Submit",
                    #[watch]
                    set_sensitive: model.item_type.is_some(),
                    connect_clicked => ItemFormMsg::CreateOrUpdateItem,
                },
            }
        }
    }

    fn init(
        init_model: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let item_type_dropdown = Self::create_item_type_dropdown(None, &sender);
        let model = ItemForm {
            item_type: None,
            notes: String::new(),
            app_services: init_model.app_services,
            item_type_dropdown,
            release_item_id: None,
            release_id: None,
        };

        let item_type_dropdown = model.item_type_dropdown.widget();

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            ItemFormMsg::UpdateItemType(item_type) => {
                tracing::info!("Updating item type: {:?}", item_type);
                self.item_type = Some(item_type);
            }
            ItemFormMsg::UpdateNotes(notes) => {
                println!("Updating notes: {}", notes);
                self.notes = notes;
            }
            ItemFormMsg::CreateOrUpdateItem => {
                if let (Some(item_type), Some(release_id)) = (self.item_type, self.release_id) {
                    let notes = if self.notes.is_empty() {
                        None
                    } else {
                        Some(self.notes.clone())
                    };

                    println!(
                        "Creating or updating item: type={:?}, notes={:?}",
                        item_type, notes
                    );

                    let app_services = Arc::clone(&self.app_services);

                    if let Some(edit_item_id) = self.release_item_id {
                        sender.oneshot_command(async move {
                            tracing::info!(item_id = edit_item_id, item_type = ?item_type, "Updating release item");
                            let result = app_services
                                .release_item
                                .update_item(edit_item_id, item_type, notes)
                                .await;
                            ItemFormCommandMsg::ItemSubmitted(result)
                        });
                    } else {
                        sender.oneshot_command(async move {
                            tracing::info!(item_type = ?item_type, "Adding new release item");
                            let result = app_services
                                .release_item
                                .create_item(release_id, item_type, notes)
                                .await;
                            ItemFormCommandMsg::ItemSubmitted(result)
                        });
                    }
                }
            }
            ItemFormMsg::Hide => {
                root.hide();
            }
            ItemFormMsg::Show {
                release_id,
                edit_item_id,
            } => {
                self.release_id = Some(release_id);
                self.item_type = None;
                self.notes.clear();
                self.release_item_id = None;

                if let Some(edit_item_id) = edit_item_id {
                    tracing::info!(item_id = edit_item_id, "Preparing to edit release item");
                    let app_services = Arc::clone(&self.app_services);
                    sender.oneshot_command(async move {
                        tracing::info!(item_id = edit_item_id, "Fetching release item for editing");
                        let result = app_services.release_item.get_item(edit_item_id).await;
                        ItemFormCommandMsg::ProcessGetEditItemResult(result)
                    });
                    // Don't show yet - wait until data is loaded
                } else {
                    // Clear the entry for new items
                    widgets.notes_entry.set_text("");
                    root.show();
                }
            }
            ItemFormMsg::UpdateFields => {
                widgets.notes_entry.set_text(&self.notes);
                if let Some(item_type) = self.item_type {
                    self.item_type_dropdown
                        .emit(DropDownMsg::SetSelected(item_type));
                }
            }
        }
        // This is essential with update_with_view:
        self.update_view(widgets, sender);
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            ItemFormCommandMsg::ItemSubmitted(Ok(item_id)) => {
                if let Some(item_type) = self.item_type {
                    let item = ReleaseItemListModel {
                        id: item_id,
                        item_type,
                    };
                    if self.release_item_id.is_some() {
                        tracing::info!(item_id, "Release item updated successfully");
                        sender
                            .output(ItemFormOutputMsg::ItemUpdated(item))
                            .unwrap_or_else(|err| {
                                tracing::error!(error = ?err, "Error sending output message");
                            });
                    } else {
                        tracing::info!(item_id, "Release item created successfully");
                        sender
                            .output(ItemFormOutputMsg::ItemAdded(item))
                            .unwrap_or_else(|err| {
                                tracing::error!(error = ?err, "Error sending output message");
                            });
                    }
                    root.close();
                }
            }
            ItemFormCommandMsg::ItemSubmitted(Err(err)) => {
                tracing::error!(error = ?err, "Error submitting item");
                show_error_dialog(
                    format!("An error occurred while submitting the item: {}", err),
                    root,
                );
            }
            ItemFormCommandMsg::ProcessGetEditItemResult(Ok(edit_item)) => {
                tracing::info!(item_id = edit_item.id, "Fetched item for editing");
                dbg!(&edit_item);
                self.item_type = Some(edit_item.item_type);
                self.notes = edit_item.notes;
                self.release_item_id = Some(edit_item.id);
                sender.input(ItemFormMsg::UpdateFields);
                root.show();
            }
            ItemFormCommandMsg::ProcessGetEditItemResult(Err(err)) => {
                tracing::error!(error = ?err, "Error fetching item for editing");
                show_error_dialog(
                    format!("An error occurred while fetching the item: {}", err),
                    root,
                );
            }
        }
    }
}

impl ItemForm {
    fn create_item_type_dropdown(
        initial_selection: Option<ItemType>,
        sender: &ComponentSender<Self>,
    ) -> Controller<ItemTypeDropDown> {
        ItemTypeDropDown::builder()
            .launch(initial_selection)
            .forward(sender.input_sender(), |msg| match msg {
                DropDownOutputMsg::ItemSelected(ItemTypeSelectedMsg::ItemTypeSelected(
                    item_type,
                )) => ItemFormMsg::UpdateItemType(item_type),
                _ => unreachable!(),
            })
    }
}
