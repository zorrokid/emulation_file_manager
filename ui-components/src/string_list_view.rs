use relm4::{
    Component, ComponentParts, ComponentSender,
    gtk::{
        self,
        glib::clone,
        prelude::{BoxExt, OrientableExt, WidgetExt},
    },
    typed_view::list::{RelmListItem, TypedListView},
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StringListItem<T: std::fmt::Display> {
    pub name: T,
}

pub struct StringListItemWidgets {
    label: gtk::Label,
}

impl<T: std::fmt::Display + Clone + 'static> RelmListItem for StringListItem<T> {
    type Root = gtk::Box;
    type Widgets = StringListItemWidgets;

    fn setup(_item: &gtk::ListItem) -> (gtk::Box, StringListItemWidgets) {
        relm4::view! {
            my_box = gtk::Box {
                #[name = "label"]
                gtk::Label {
                    set_xalign: 0.0,
                    set_margin_start: 6,
                },
            }
        }
        (my_box, StringListItemWidgets { label })
    }

    fn bind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        widgets.label.set_label(&self.name.to_string());
    }
}

#[derive(Debug)]
pub struct StringListView<T: std::fmt::Display> {
    list_view_wrapper: TypedListView<StringListItem<T>, gtk::SingleSelection>,
    title: String,
}

pub struct StringListViewInit {
    pub title: String,
}

#[derive(Debug)]
pub enum StringListViewMsg<T: std::fmt::Display> {
    SetItems(Vec<T>),
    SelectionChanged,
}

#[derive(Debug)]
pub enum StringListViewOutput<T: std::fmt::Display> {
    SelectionChanged(Option<T>),
}

#[relm4::component(pub)]
impl<T: std::fmt::Display + Clone + std::fmt::Debug + 'static> Component for StringListView<T> {
    type Init = StringListViewInit;
    type Input = StringListViewMsg<T>;
    type Output = StringListViewOutput<T>;
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 4,

            gtk::Label {
                #[watch]
                set_label: &model.title,
                set_xalign: 0.0,
            },

            gtk::ScrolledWindow {
                set_vexpand: true,
                #[local_ref]
                list_view -> gtk::ListView {},
            },
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let list_view_wrapper: TypedListView<StringListItem<T>, gtk::SingleSelection> =
            TypedListView::new();

        list_view_wrapper
            .selection_model
            .connect_selected_notify(clone!(
                #[strong]
                sender,
                move |_| sender.input(StringListViewMsg::SelectionChanged)
            ));

        let model = StringListView {
            list_view_wrapper,
            title: init.title,
        };

        let list_view = &model.list_view_wrapper.view;
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            StringListViewMsg::SetItems(items) => {
                self.list_view_wrapper.clear();
                self.list_view_wrapper
                    .extend_from_iter(items.into_iter().map(|name| StringListItem { name }));
            }
            StringListViewMsg::SelectionChanged => {
                let selected = self.get_selected();
                sender
                    .output(StringListViewOutput::SelectionChanged(selected))
                    .unwrap_or_default();
            }
        }
    }
}

impl<T: std::fmt::Display + Clone + 'static> StringListView<T> {
    pub fn get_selected(&self) -> Option<T> {
        let idx = self.list_view_wrapper.selection_model.selected();
        self.list_view_wrapper
            .get_visible(idx)
            .map(|item| item.borrow().name.clone())
    }
}
