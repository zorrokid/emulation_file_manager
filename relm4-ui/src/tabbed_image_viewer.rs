use std::sync::Arc;

use core_types::{FileType, IMAGE_FILE_TYPES};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller,
    gtk::{self, prelude::*},
};
use service::view_models::{FileSetViewModel, Settings};

use crate::image_viewer::{ImageViewer, ImageViewerInit};

#[derive(Debug)]
pub enum TabbedImageViewerMsg {
    SetFileSets { file_sets: Vec<FileSetViewModel> },
    Clear,
}

#[derive(Debug)]
pub struct TabbedImageViewer {
    viewers: Vec<Controller<ImageViewer>>,
    settings: Arc<Settings>,
    page_numbers: Vec<u32>,
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
        _root: Self::Root,
        _: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let widgets = AppWidgets {};

        ComponentParts {
            model: TabbedImageViewer {
                viewers: vec![],
                settings: Arc::clone(&init_model.settings),
                page_numbers: Vec::new(),
            },
            widgets,
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            TabbedImageViewerMsg::SetFileSets { file_sets } => {
                self.viewers = Vec::new();
                for page_number in self.page_numbers.iter().rev() {
                    root.remove_page(Some(*page_number));
                }
                self.page_numbers.clear();

                for file_type in IMAGE_FILE_TYPES {
                    // NOTE: currentely only the first file set of each type is used
                    if let Some(file_set) = file_sets
                        .iter()
                        .find(|fs| fs.file_type == (*file_type).into())
                    {
                        if file_set.files.is_empty() {
                            continue; // Skip empty file sets
                        }
                        let image_viewer_init = ImageViewerInit {
                            settings: Arc::clone(&self.settings),
                            file_set: Some(file_set.clone()),
                        };

                        let box_scan_image_viewer =
                            ImageViewer::builder().launch(image_viewer_init).detach();

                        let box_scans_page = gtk::Box::builder()
                            .orientation(gtk::Orientation::Vertical)
                            .build();

                        let box_scans_page_label =
                            gtk::Label::new(Some(file_type.to_string().as_str()));
                        box_scans_page.append(box_scan_image_viewer.widget());
                        let page_number =
                            root.append_page(&box_scans_page, Some(&box_scans_page_label));
                        self.page_numbers.push(page_number);
                        self.viewers.push(box_scan_image_viewer);
                    }
                }
            }
            TabbedImageViewerMsg::Clear => {
                for page_number in self.page_numbers.iter() {
                    root.remove_page(Some(*page_number));
                }
                self.page_numbers.clear();
            }
        }
    }
}
