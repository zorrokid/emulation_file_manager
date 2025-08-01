use relm4::{
    gtk::{self, prelude::*},
    typed_view::list::RelmListItem,
};

#[derive(Debug, Clone)]
pub struct ListItem {
    pub name: String,
    pub id: i64,
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
        label.set_label(&format!("Name: {} ", self.name));
    }
}
