use std::sync::Arc;

use core_types::FileType;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmApp,
    gtk::{self, prelude::*},
};
use service::view_models::{FileSetViewModel, Settings};

use crate::image_viewer::{ImageViewer, ImageViewerInit, ImageViewerMsg};

#[derive(Debug)]
pub enum TabbedImageViewerMsg {
    SetFileSets { file_sets: Vec<FileSetViewModel> },
    Clear,
}

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

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            TabbedImageViewerMsg::SetFileSets { file_sets } => {
                // NOTE: currentely only the first file set of each type is used
                if let Some(file_set) = file_sets
                    .iter()
                    .find(|fs| fs.file_type == FileType::CoverScan.into())
                {
                    self.box_scan_image_viewer
                        .sender()
                        .send(ImageViewerMsg::SetFileSet {
                            file_set: file_set.clone(),
                        })
                        .unwrap();
                }
                if let Some(file_set) = file_sets
                    .iter()
                    .find(|fs| fs.file_type == FileType::Screenshot.into())
                {
                    self.screenshots_image_viewer
                        .sender()
                        .send(ImageViewerMsg::SetFileSet {
                            file_set: file_set.clone(),
                        })
                        .unwrap();
                }
            }
            TabbedImageViewerMsg::Clear => {
                self.box_scan_image_viewer
                    .sender()
                    .send(ImageViewerMsg::Clear)
                    .unwrap();
                self.screenshots_image_viewer
                    .sender()
                    .send(ImageViewerMsg::Clear)
                    .unwrap();
            }
        }
    }
}
