use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmApp,
    gtk::{self, glib::clone, prelude::*},
    once_cell::sync::OnceCell,
    typed_view::list::TypedListView,
};

#[derive(Debug)]
pub enum TabbedImageViewerMsg {}

#[derive(Debug)]
pub struct TabbedImageViewer {}

#[derive(Debug)]
pub struct AppWidgets {}

impl Component for TabbedImageViewer {
    type Input = TabbedImageViewerMsg;
    type Output = ();
    type CommandOutput = ();
    type Init = ();
    type Root = gtk::Notebook;
    type Widgets = AppWidgets;

    fn init_root() -> Self::Root {
        gtk::Notebook::new()
    }

    fn init(_: Self::Init, root: Self::Root, _: ComponentSender<Self>) -> ComponentParts<Self> {
        let widgets = AppWidgets {};
        let page_1 = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(6)
            .build();
        page_1.append(&gtk::Label::new(Some("Page 1 Content")));
        let label_1 = gtk::Label::new(Some("Page 1"));
        root.append_page(&page_1, Some(&label_1));
        let page_2 = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(6)
            .build();
        page_2.append(&gtk::Label::new(Some("Page 2 Content")));
        let label_2 = gtk::Label::new(Some("Page 2"));
        root.append_page(&page_2, Some(&label_2));
        ComponentParts {
            model: TabbedImageViewer {},
            widgets,
        }
    }
}
