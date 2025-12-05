use relm4::{
    gtk::{self},
    typed_view::list::RelmListItem,
};

pub trait HasId {
    fn id(&self) -> i64;
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ListItem {
    pub name: String,
    pub id: i64,
}

pub struct ListItemWidgets {
    label: gtk::Label,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeletableListItem {
    pub name: String,
    pub id: i64,
    pub can_delete: bool,
}
