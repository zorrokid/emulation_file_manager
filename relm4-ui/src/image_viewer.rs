use std::{path::PathBuf, sync::Arc};

use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller,
    gtk::{
        self,
        prelude::{ButtonExt, OrientableExt, WidgetExt},
    },
};
use service::{
    error::Error as ServiceError, file_set_download::service::DownloadResult,
    view_models::FileSetViewModel,
};

use crate::image_fileset_viewer::{
    ImageFileSetViewerInit, ImageFileSetViewerOutputMsg, ImageFilesetViewer, ImageFilesetViewerMsg,
};

#[derive(Debug)]
pub enum ImageViewerMsg {
    ShowPrevious,
    ShowNext,
    SetFileSet { file_set: FileSetViewModel },
    Clear,
    View,
    ShowError(String),
}

#[derive(Debug)]
pub enum ImageViewerCommandMsg {
    HandleDownloadResult(Result<DownloadResult, ServiceError>),
}

#[derive(Debug)]
pub enum ImageViewerOutputMsg {
    ShowError(String),
}

#[derive(Debug, Clone)]
pub struct ImageViewerInit {
    pub file_set: Option<FileSetViewModel>,
    pub app_services: Arc<service::app_services::AppServices>,
}

#[derive(Debug)]
pub struct ImageViewer {
    file_set: Option<FileSetViewModel>,
    current_file_index: Option<usize>,
    selected_image: PathBuf,
    image_file_set_viewer: Controller<ImageFilesetViewer>,
    app_services: Arc<service::app_services::AppServices>,
}

#[relm4::component(pub)]
impl Component for ImageViewer {
    type Init = ImageViewerInit;
    type Input = ImageViewerMsg;
    type Output = ImageViewerOutputMsg;
    type CommandOutput = ImageViewerCommandMsg;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            gtk::ScrolledWindow {
                set_vexpand: true,
                set_hexpand: true,
                #[name = "selected_image"]
                gtk::Image {
                    #[watch]
                    set_from_file: Some(&model.selected_image),
                }
            },
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_hexpand: true,
                gtk::Button {
                    set_label: "<",
                    connect_clicked => ImageViewerMsg::ShowPrevious,
                    #[watch]
                    set_sensitive: model.current_file_index.unwrap_or(0) > 0,
                },
                gtk::Button {
                    set_hexpand: true,
                    set_label: "View",
                    connect_clicked => ImageViewerMsg::View,
                },
                gtk::Button {
                    set_label: ">",
                    connect_clicked => ImageViewerMsg::ShowNext,
                    #[watch] set_sensitive: model.current_file_index.unwrap_or(usize::MAX) <
                        model.file_set.as_ref().map_or(0, |fs| fs.files.len() - 1),
                },

            },
       },
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let init_model = ImageFileSetViewerInit {
            app_services: Arc::clone(&init.app_services),
        };
        let image_file_set_viewer = ImageFilesetViewer::builder()
            .transient_for(&root)
            .launch(init_model)
            .forward(sender.input_sender(), |msg| match msg {
                ImageFileSetViewerOutputMsg::ShowError(msg) => ImageViewerMsg::ShowError(msg),
            });

        let model = ImageViewer {
            file_set: None,
            selected_image: PathBuf::new(),
            current_file_index: None,
            image_file_set_viewer,
            app_services: Arc::clone(&init.app_services),
        };
        if let Some(file_set) = init.file_set {
            sender.input(ImageViewerMsg::SetFileSet { file_set });
        }

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            ImageViewerMsg::SetFileSet { file_set } => {
                self.current_file_index = if !file_set.files.is_empty() {
                    Some(0)
                } else {
                    None
                };
                let file_set_id = file_set.id;
                self.file_set = Some(file_set);

                let download_service = Arc::clone(&self.app_services.file_set_download());
                sender.oneshot_command(async move {
                    let res = download_service
                        .download_file_set(file_set_id, true, None)
                        .await;

                    ImageViewerCommandMsg::HandleDownloadResult(res)
                });
            }
            ImageViewerMsg::ShowNext => {
                if let (Some(index), Some(file_set)) = (self.current_file_index, &self.file_set)
                    && index + 1 < file_set.files.len()
                {
                    let new_index = index + 1;
                    self.current_file_index = Some(new_index);
                    let next_image = file_set.files[new_index].file_name.clone();
                    let file_path = self
                        .app_services
                        .app_settings()
                        .temp_output_dir
                        .join(&next_image);
                    self.selected_image = file_path;
                }
            }
            ImageViewerMsg::ShowPrevious => {
                if let (Some(index), Some(file_set)) = (self.current_file_index, &self.file_set)
                    && index > 0
                {
                    let new_index = index - 1;
                    self.current_file_index = Some(new_index);
                    let previous_image = file_set.files[new_index].file_name.clone();
                    let file_path = self
                        .app_services
                        .app_settings()
                        .temp_output_dir
                        .join(&previous_image);
                    self.selected_image = file_path;
                }
            }
            ImageViewerMsg::Clear => {
                self.file_set = None;
                self.current_file_index = None;
                self.selected_image = PathBuf::new();
            }
            ImageViewerMsg::View => {
                if let Some(file_set) = &self.file_set {
                    self.image_file_set_viewer
                        .emit(ImageFilesetViewerMsg::Show {
                            file_set: file_set.clone(),
                        });
                }
            }
            ImageViewerMsg::ShowError(msg) => {
                sender
                    .output(ImageViewerOutputMsg::ShowError(msg))
                    .unwrap_or_else(|err| {
                        tracing::error!(
                            error = ?err,
                            "Failed sending output message ImageViewerOutputMsg::ShowError");
                    });
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            ImageViewerCommandMsg::HandleDownloadResult(Ok(_)) => {
                if let Some(file_set) = &self.file_set {
                    let selected_image_name = self
                        .current_file_index
                        .and_then(|index| file_set.files.get(index))
                        .map(|f| f.file_name.clone())
                        .unwrap_or_default();

                    if let Some(selected_file) = file_set
                        .files
                        .iter()
                        .find(|f| f.file_name == selected_image_name)
                    {
                        let temp_output_dir = &self.app_services.app_settings().temp_output_dir;
                        let image_path = temp_output_dir.join(&selected_file.file_name);

                        self.selected_image = image_path;
                    } else {
                        tracing::error!("Selected file not found in the file set.");
                    }
                }
            }
            ImageViewerCommandMsg::HandleDownloadResult(Err(e)) => {
                let message = format!("Failed to download fileset: {}", e);
                tracing::error!(message);
                sender
                    .output(ImageViewerOutputMsg::ShowError(message))
                    .unwrap_or_else(|err| {
                        tracing::error!("Failed to send output message: {:?}", err);
                    });
            }
        }
    }
}
