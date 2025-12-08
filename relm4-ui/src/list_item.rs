use core_types::FileType;
use relm4::{
    gtk::{self},
    typed_view::list::RelmListItem,
};

// TODO: Maybe use ListModels directly instead of these?

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ListItem {
    pub name: String,
    pub id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeletableListItem {
    pub name: String,
    pub id: i64,
    pub can_delete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileSetListItem {
    pub name: String,
    pub id: i64,
    pub file_type: FileType,
    pub can_delete: bool,
}

pub trait HasId {
    fn id(&self) -> i64;
}

pub struct ListItemWidgets {
    label: gtk::Label,
}

macro_rules! impl_list_item_traits {
    ($ty:ty) => {
        impl HasId for $ty {
            fn id(&self) -> i64 {
                self.id
            }
        }
        impl RelmListItem for $ty {
            type Root = gtk::Box;
            type Widgets = ListItemWidgets;
            fn setup(_item: &gtk::ListItem) -> (gtk::Box, ListItemWidgets) {
                relm4::view! {
                    my_box = gtk::Box {
                        #[name = "label"]
                        gtk::Label,
                    }
                }
                let widgets = ListItemWidgets { label };
                (my_box, widgets)
            }
            fn bind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
                let ListItemWidgets { label } = widgets;
                label.set_label(&self.name);
            }
        }
    };
}

impl_list_item_traits!(ListItem);
impl_list_item_traits!(DeletableListItem);
impl_list_item_traits!(FileSetListItem);
