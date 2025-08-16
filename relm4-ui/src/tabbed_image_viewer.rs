use std::sync::Arc;

use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmApp,
    gtk::{self, glib::clone, prelude::*},
    once_cell::sync::OnceCell,
    typed_view::list::TypedListView,
};
use service::view_models::Settings;

use crate::image_viewer::{ImageViewer, ImageViewerInit};

#[derive(Debug)]
pub enum TabbedImageViewerMsg {}

#[derive(Debug)]
pub struct TabbedImageViewer {
    box_scan_image_viewer: Controller<ImageViewer>,
    screenshots_image_viewer: Controller<ImageViewer>,
}

#[derive(Debug)]
pub struct TabbedImageViewerInit {
    pub settings: Arc<Settings>,
}

#[derive(Debug)]
pub struct AppWidgets {}

impl Component for TabbedImageViewer {
    type Input = TabbedImageViewerMsg;
    type Output = ();
    type CommandOutput = ();
    type Init = TabbedImageViewerInit;
    type Root = gtk::Notebook;
    type Widgets = AppWidgets;

    fn init_root() -> Self::Root {
        gtk::Notebook::new()
    }

    fn init(
        init_model: Self::Init,
        root: Self::Root,
        _: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let image_viewer_init = ImageViewerInit {
            settings: Arc::clone(&init_model.settings),
        };

        let box_scan_image_viewer = ImageViewer::builder()
            .launch(image_viewer_init.clone())
            .detach();
        let screenshots_image_viewer = ImageViewer::builder().launch(image_viewer_init).detach();

        let widgets = AppWidgets {};

        let box_scans_page = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        let screenshots_page = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(6)
            .build();

        let box_scans_page_label = gtk::Label::new(Some("Box Scans"));
        let screenshots_page_label = gtk::Label::new(Some("Screenshots"));

        box_scans_page.append(box_scan_image_viewer.widget());
        screenshots_page.append(screenshots_image_viewer.widget());

        root.append_page(&box_scans_page, Some(&box_scans_page_label));
        root.append_page(&screenshots_page, Some(&screenshots_page_label));

        ComponentParts {
            model: TabbedImageViewer {
                box_scan_image_viewer,
                screenshots_image_viewer,
            },
            widgets,
        }
    }
}
