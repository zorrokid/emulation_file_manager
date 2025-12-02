use relm4::{
    gtk::{self},
    typed_view::list::RelmListItem,
};

pub trait HasId {
    fn id(&self) -> i64;
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ListItem {
    pub name: String,
    pub id: i64,
}

impl HasId for ListItem {
    fn id(&self) -> i64 {
        self.id
    }
}

pub struct ListItemWidgets {
    label: gtk::Label,
}

impl RelmListItem for ListItem {
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
