use relm4::{
    Component, ComponentParts, ComponentSender,
    gtk::{
        self,
        prelude::{BoxExt, OrientableExt, WidgetExt},
    },
    typed_view::list::{RelmListItem, TypedListView},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageStatus {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageListItem {
    pub message: String,
    pub status: MessageStatus,
}

pub struct ListItemWidgets {
    label: gtk::Label,
}

impl RelmListItem for MessageListItem {
    type Root = gtk::Box;
    type Widgets = ListItemWidgets;

    fn setup(_item: &gtk::ListItem) -> (gtk::Box, ListItemWidgets) {
        relm4::view! {
            my_box = gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                #[name = "label"]
                gtk::Label,
            }
        }

        let widgets = ListItemWidgets { label };

        (my_box, widgets)
    }

    fn bind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        let ListItemWidgets { label } = widgets;
        let status_icon = match self.status {
            MessageStatus::Info => "🟢",
            MessageStatus::Warning => "🟡",
            MessageStatus::Error => "🔴",
        };
        label.set_label(format!("{} {}", status_icon, self.message).as_str());
    }
}

#[derive(Debug)]
pub struct MessageListView {
    list_view_wrapper: relm4::typed_view::list::TypedListView<MessageListItem, gtk::NoSelection>,
}

#[derive(Debug)]
pub enum MessageListViewMsg {
    SetItems(Vec<MessageListItem>),
}

#[relm4::component(pub)]
impl Component for MessageListView {
    type Init = ();
    type Input = MessageListViewMsg;
    type Output = ();
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 4,
            gtk::ScrolledWindow {
                set_vexpand: true,
                set_hexpand: true,
                #[local_ref]
                list_view -> gtk::ListView {},
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let list_view_wrapper: TypedListView<MessageListItem, gtk::NoSelection> =
            TypedListView::new();

        let model = MessageListView { list_view_wrapper };
        let list_view = &model.list_view_wrapper.view;
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            MessageListViewMsg::SetItems(items) => {
                self.list_view_wrapper.extend_from_iter(items);
            }
        }
    }
}
