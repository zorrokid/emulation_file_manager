use core_types::item_type::ItemType;
use relm4::gtk;
use relm4::gtk::prelude::*;
use relm4::prelude::*;
use std::fmt::Display;
use std::marker::PhantomData;

/// Generic trait for items that can be displayed in a dropdown
pub trait DropDownItem: Clone + Display + PartialEq + std::fmt::Debug + 'static {
    /// Get all available items for the dropdown
    fn all_items() -> Vec<Self>;
}

/// Generic trait for messages that can be sent when an item is selected
pub trait DropDownMessage<T: DropDownItem>: std::fmt::Debug {
    /// Create a new selection message with the given item
    fn from_selection(item: T) -> Self;
}

#[derive(Debug)]
pub struct DropDown<T, M>
where
    T: DropDownItem,
    M: DropDownMessage<T> + 'static,
{
    items: Vec<T>,
    selected_index: Option<u32>,
    _phantom: PhantomData<M>,
}

#[derive(Debug)]
pub enum DropDownMsg<T, M>
where
    T: DropDownItem,
    M: DropDownMessage<T>,
{
    SelectionChanged(u32),
    SetSelected(T),
    _Phantom(PhantomData<(T, M)>),
}

#[derive(Debug)]
pub enum DropDownOutputMsg<T, M>
where
    T: DropDownItem,
    M: DropDownMessage<T> + 'static,
{
    ItemSelected(M),
    _Phantom(PhantomData<T>),
}

#[relm4::component(pub)]
impl<T, M> Component for DropDown<T, M>
where
    T: DropDownItem,
    M: DropDownMessage<T> + 'static,
{
    type Init = Option<T>;
    type Input = DropDownMsg<T, M>;
    type Output = DropDownOutputMsg<T, M>;
    type CommandOutput = ();

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 6,

            #[name = "dropdown"]
            gtk::DropDown {
                connect_selected_notify[sender] => move |dropdown| {
                    sender.input(DropDownMsg::SelectionChanged(dropdown.selected()));
                },
                #[watch]
                set_selected: model.selected_index.unwrap_or(0),
            },
        }
    }

    fn init(
        initial_selection: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let items = T::all_items();

        // Find the initial selection index
        let initial_index = initial_selection
            .as_ref()
            .and_then(|item| items.iter().position(|i| i == item))
            .map(|pos| pos as u32);

        let model = Self {
            items: items.clone(),
            selected_index: initial_index,
            _phantom: PhantomData,
        };

        let widgets = view_output!();

        // Setup dropdown with items
        let item_strings: Vec<String> = items.iter().map(|item| item.to_string()).collect();
        let string_refs: Vec<&str> = item_strings.iter().map(|s| s.as_str()).collect();
        let string_list = gtk::StringList::new(&string_refs);

        widgets.dropdown.set_model(Some(&string_list));

        // Set initial selection
        if let Some(index) = initial_index {
            widgets.dropdown.set_selected(index);
        } else if !items.is_empty() {
            widgets.dropdown.set_selected(0);
        }

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            DropDownMsg::SelectionChanged(index) => {
                self.selected_index = Some(index);

                if let Some(item) = self.items.get(index as usize) {
                    let message = M::from_selection(item.clone());
                    sender
                        .output(DropDownOutputMsg::ItemSelected(message))
                        .unwrap_or_else(|err| {
                            eprintln!("Error sending output message: {:?}", err);
                        });
                }
            }
            DropDownMsg::SetSelected(item) => {
                if let Some(index) = self.items.iter().position(|i| i == &item) {
                    self.selected_index = Some(index as u32);
                }
            }
            DropDownMsg::_Phantom(_) => {}
        }
    }
}

impl<T, M> DropDown<T, M>
where
    T: DropDownItem,
    M: DropDownMessage<T> + 'static,
{
    /// Get the currently selected item
    pub fn selected_item(&self) -> Option<&T> {
        self.selected_index
            .and_then(|idx| self.items.get(idx as usize))
    }

    /// Set the selected item by finding it in the list
    pub fn set_selected_item(&mut self, item: &T) {
        if let Some(index) = self.items.iter().position(|i| i == item) {
            self.selected_index = Some(index as u32);
        }
    }
}

// FileType-specific implementation
use core_types::{ACTIVE_FILE_TYPES, FileType};
use strum::IntoEnumIterator;

impl DropDownItem for FileType {
    fn all_items() -> Vec<Self> {
        ACTIVE_FILE_TYPES.to_vec()
    }
}

#[derive(Debug, Clone)]
pub enum FileTypeSelectedMsg {
    FileTypeSelected(FileType),
}

impl DropDownMessage<FileType> for FileTypeSelectedMsg {
    fn from_selection(item: FileType) -> Self {
        FileTypeSelectedMsg::FileTypeSelected(item)
    }
}

pub type FileTypeDropDown = DropDown<FileType, FileTypeSelectedMsg>;

// DocumentType-specific implementation

use core_types::DocumentType;

impl DropDownItem for DocumentType {
    fn all_items() -> Vec<Self> {
        DocumentType::iter().collect()
    }
}

#[derive(Debug, Clone)]
pub enum DocumentTypeSelectedMsg {
    DocumentTypeSelected(DocumentType),
}

impl DropDownMessage<DocumentType> for DocumentTypeSelectedMsg {
    fn from_selection(item: DocumentType) -> Self {
        DocumentTypeSelectedMsg::DocumentTypeSelected(item)
    }
}

pub type DocumentTypeDropDown = DropDown<DocumentType, DocumentTypeSelectedMsg>;

// ItemType-specific implementation
pub type ItemTypeDropDown = DropDown<ItemType, ItemTypeSelectedMsg>;

#[derive(Debug, Clone)]
pub enum ItemTypeSelectedMsg {
    ItemTypeSelected(ItemType),
}

impl DropDownItem for ItemType {
    fn all_items() -> Vec<Self> {
        ItemType::iter().collect()
    }
}

impl DropDownMessage<ItemType> for ItemTypeSelectedMsg {
    fn from_selection(item: ItemType) -> Self {
        ItemTypeSelectedMsg::ItemTypeSelected(item)
    }
}
