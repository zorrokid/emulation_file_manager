use core_types::item_type::ItemType;
use relm4::{
    Component, ComponentParts, ComponentSender,
    gtk::{
        self,
        glib::clone,
        prelude::{BoxExt, ButtonExt, OrientableExt},
    },
};
use ui_components::DropDownItem;

#[derive(Debug)]
pub struct ItemTypeDropdown {
    item_type_dropdown: gtk::DropDown,
    items: Vec<ItemType>,
    selected_item_type: Option<ItemType>,
}

#[derive(Debug)]
pub enum ItemTypeDropDownMsg {
    ClearItemTypeSelection,
    ItemTypeChanged(u32),
    SetSelectedItemType(Option<ItemType>),
}

#[derive(Debug)]
pub enum ItemTypeDropDownOutputMsg {
    ItemTypeChanged(Option<ItemType>),
}

#[relm4::component(pub)]
impl Component for ItemTypeDropdown {
    type Input = ItemTypeDropDownMsg;
    type Output = ItemTypeDropDownOutputMsg;
    type Init = ();
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 5,
            gtk::Label {
                set_label: "Item Type:",
            },
            #[local_ref]
            item_type_dropdown -> gtk::DropDown,
            gtk::Button {
                set_label: "Clear Selection",
                connect_clicked[sender] => move |_| {
                    sender.input(ItemTypeDropDownMsg::ClearItemTypeSelection);
                }
            }
        }
    }

    fn init(
        _init_model: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let items: Vec<ItemType> = ItemType::all_items();
        let items_for_dropdown = items
            .iter()
            .map(|item| item.to_string())
            .collect::<Vec<String>>();
        let items_for_dropdown: Vec<&str> = items_for_dropdown.iter().map(|s| s.as_str()).collect();

        let items_string_list = gtk::StringList::new(&items_for_dropdown);

        let item_type_dropdown =
            gtk::DropDown::new(Some(items_string_list), None::<gtk::Expression>);

        item_type_dropdown.set_selected(gtk::INVALID_LIST_POSITION);
        item_type_dropdown.connect_selected_notify(clone!(
            #[strong]
            sender,
            move |dropdown| {
                let selected_index = dropdown.selected();
                tracing::info!("Index selected: {}", selected_index);
                sender.input(ItemTypeDropDownMsg::ItemTypeChanged(selected_index));
            }
        ));

        let model = ItemTypeDropdown {
            item_type_dropdown,
            items,
            selected_item_type: None,
        };

        let item_type_dropdown = &model.item_type_dropdown;

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            ItemTypeDropDownMsg::ItemTypeChanged(index) => {
                tracing::info!("Simple clearable dropdown changed: {}", index);
                let selected_item = if index != gtk::INVALID_LIST_POSITION {
                    Some(self.items.get(index as usize).cloned())
                } else {
                    None
                };
                if let Some(item) = selected_item {
                    tracing::info!("Selected item: {:?}", item);
                    self.selected_item_type = item;
                    sender
                        .output(ItemTypeDropDownOutputMsg::ItemTypeChanged(
                            self.selected_item_type,
                        ))
                        .unwrap_or_else(|e| {
                            tracing::error!(error = ?e, "Failed to send output message");
                        });
                } else {
                    tracing::info!("No item selected");
                }
            }
            ItemTypeDropDownMsg::ClearItemTypeSelection => {
                tracing::info!("Clearing simple clearable dropdown selection");
                self.item_type_dropdown
                    .set_selected(gtk::INVALID_LIST_POSITION);
                self.selected_item_type = None;
                sender
                    .output(ItemTypeDropDownOutputMsg::ItemTypeChanged(
                        self.selected_item_type,
                    ))
                    .unwrap_or_else(|e| {
                        tracing::error!(error = ?e, "Failed to send output message");
                    });
            }
            ItemTypeDropDownMsg::SetSelectedItemType(item_type_option) => {
                tracing::info!("Setting selected item type to: {:?}", item_type_option);
                if let Some(item_type) = item_type_option {
                    if let Some(index) = self.items.iter().position(|it| *it == item_type) {
                        self.item_type_dropdown.set_selected(index as u32);
                        self.selected_item_type = Some(item_type);
                    } else {
                        tracing::warn!("Item type {:?} not found in items list", item_type);
                    }
                } else {
                    self.item_type_dropdown
                        .set_selected(gtk::INVALID_LIST_POSITION);
                    self.selected_item_type = None;
                }
            }
        }
    }
}
