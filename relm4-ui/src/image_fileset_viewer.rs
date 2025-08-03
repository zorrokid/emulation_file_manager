use std::sync::Arc;

use database::models::FileSet;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller,
    gtk::{
        self,
        glib::clone,
        prelude::{ButtonExt, GtkWindowExt, OrientableExt, WidgetExt},
    },
    typed_view::list::TypedListView,
};
use service::view_models::{FileSetViewModel, Settings};

#[derive(Debug)]
pub enum ImageFilesetViewerMsg {
    FileSelected,
}

#[derive(Debug)]
pub enum ImageFileSetViewerCommandMsg {
    FileSetExtracted,
}

pub struct ImageFileSetViewerInit {
    pub file_set: FileSetViewModel,
    pub settings: Arc<Settings>,
}

#[derive(Debug)]
pub struct ImageFilesetViewer {
    file_set: FileSetViewModel,
    settings: Arc<Settings>,
}

#[relm4::component(pub)]
impl Component for ImageFilesetViewer {
    type Init = ImageFileSetViewerInit;
    type Input = ImageFilesetViewerMsg;
    type Output = ();
    type CommandOutput = ImageFileSetViewerCommandMsg;

    view! {
        gtk::Window {
            set_title: Some("Image Fileset Viewer"),
            set_default_width: 800,
            set_default_height: 600,
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                gtk::Label {
                    set_label: &format!("Viewing fileset: {}", model.file_set.file_set_name),
                },
           },
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = ImageFilesetViewer {
            file_set: init.file_set,
            settings: init.settings,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }
}
