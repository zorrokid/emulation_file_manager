use std::sync::Arc;

use core_types::IMAGE_FILE_TYPES;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller,
    gtk::{self, prelude::*},
};
use service::{
    file_set_download::service::DownloadService,
    view_models::{FileSetViewModel, Settings},
};

use crate::image_viewer::{ImageViewer, ImageViewerInit, ImageViewerOutputMsg};

#[derive(Debug)]
pub enum TabbedImageViewerMsg {
    SetFileSets { file_sets: Vec<FileSetViewModel> },
    Clear,
    ShowError(String),
}

#[derive(Debug)]
pub struct TabbedImageViewer {
    viewers: Vec<Controller<ImageViewer>>,
    settings: Arc<Settings>,
    page_numbers: Vec<u32>,
    download_service: Arc<DownloadService>,
}

#[derive(Debug)]
pub struct TabbedImageViewerInit {
    pub settings: Arc<Settings>,
    pub download_service: Arc<DownloadService>,
}

#[derive(Debug)]
pub enum TabbedImageViewerOutputMsg {
    ShowError(String),
}

#[derive(Debug)]
pub struct AppWidgets {}

impl Component for TabbedImageViewer {
    type Input = TabbedImageViewerMsg;
    type Output = TabbedImageViewerOutputMsg;
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
                settings: init_model.settings,
                page_numbers: Vec::new(),
                download_service: init_model.download_service,
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
                    // NOTE: currently only the first file set of each type is used
                    if let Some(file_set) = file_sets.iter().find(|fs| fs.file_type == *file_type) {
                        if file_set.files.is_empty() {
                            continue; // Skip empty file sets
                        }
                        let image_viewer_init = ImageViewerInit {
                            settings: Arc::clone(&self.settings),
                            file_set: Some(file_set.clone()),
                            download_service: Arc::clone(&self.download_service),
                        };

                        let box_scan_image_viewer = ImageViewer::builder()
                            .launch(image_viewer_init)
                            .forward(sender.input_sender(), |output_msg| match output_msg {
                                ImageViewerOutputMsg::ShowError(msg) => {
                                    TabbedImageViewerMsg::ShowError(msg)
                                }
                            });

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
                for page_number in self.page_numbers.iter().rev() {
                    root.remove_page(Some(*page_number));
                }
                self.page_numbers.clear();
            }
            TabbedImageViewerMsg::ShowError(error_msg) => {
                sender
                    .output(TabbedImageViewerOutputMsg::ShowError(error_msg))
                    .unwrap_or_else(|e| {
                        tracing::error!(error = ?e, "Failed to send output message");
                    });
            }
        }
    }
}
