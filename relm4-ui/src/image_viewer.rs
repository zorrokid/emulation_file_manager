use std::{path::PathBuf, sync::Arc};

use file_export::{FileExportError, FileSetExportModel, export_files};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller,
    gtk::{
        self,
        prelude::{ButtonExt, GtkWindowExt, OrientableExt, WidgetExt},
    },
};
use service::view_models::{FileSetViewModel, Settings};

use crate::{
    image_fileset_viewer::{ImageFileSetViewerInit, ImageFilesetViewer, ImageFilesetViewerMsg},
    utils::prepare_fileset_for_export,
};

#[derive(Debug)]
pub enum ImageViewerMsg {
    ShowPrevious,
    ShowNext,
    SetFileSet { file_set: FileSetViewModel },
    Clear,
    View,
}

#[derive(Debug)]
pub enum ImageViewerCommandMsg {
    ExportedImageFileSet(Result<(), FileExportError>, FileSetExportModel),
}

#[derive(Debug, Clone)]
pub struct ImageViewerInit {
    pub settings: Arc<Settings>,
    pub file_set: Option<FileSetViewModel>,
}

#[derive(Debug)]
pub struct ImageViewer {
    file_set: Option<FileSetViewModel>,
    current_file_index: Option<usize>,
    settings: Arc<Settings>,
    selected_image: PathBuf,
    image_file_set_viewer: Controller<ImageFilesetViewer>,
}

#[relm4::component(pub)]
impl Component for ImageViewer {
    type Init = ImageViewerInit;
    type Input = ImageViewerMsg;
    type Output = ();
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
            settings: Arc::clone(&init.settings),
        };
        let image_file_set_viewer = ImageFilesetViewer::builder()
            .transient_for(&root)
            .launch(init_model)
            .detach();

        let model = ImageViewer {
            file_set: None,
            settings: init.settings,
            selected_image: PathBuf::new(),
            current_file_index: None,
            image_file_set_viewer,
        };
        if let Some(file_set) = init.file_set {
            sender.input(ImageViewerMsg::SetFileSet { file_set });
        }

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            ImageViewerMsg::SetFileSet { file_set } => {
                self.current_file_index = if !file_set.files.is_empty() {
                    Some(0)
                } else {
                    None
                };
                let export_model = prepare_fileset_for_export(
                    &file_set,
                    &self.settings.collection_root_dir,
                    &self.settings.temp_output_dir,
                    true,
                );
                self.file_set = Some(file_set);

                sender.spawn_command(move |sender| {
                    let res = export_files(&export_model);
                    sender.emit(ImageViewerCommandMsg::ExportedImageFileSet(
                        res,
                        export_model,
                    ));
                });
            }
            ImageViewerMsg::ShowNext => {
                if let (Some(index), Some(file_set)) = (self.current_file_index, &self.file_set) {
                    if index + 1 < file_set.files.len() {
                        let new_index = index + 1;
                        self.current_file_index = Some(new_index);
                        let next_image = file_set.files[new_index].file_name.clone();
                        let file_path = self.settings.temp_output_dir.join(&next_image);
                        self.selected_image = file_path;
                    }
                }
            }
            ImageViewerMsg::ShowPrevious => {
                if let (Some(index), Some(file_set)) = (self.current_file_index, &self.file_set) {
                    if index > 0 {
                        let new_index = index - 1;
                        self.current_file_index = Some(new_index);
                        let previous_image = file_set.files[new_index].file_name.clone();
                        let file_path = self.settings.temp_output_dir.join(&previous_image);
                        self.selected_image = file_path;
                    }
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
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            ImageViewerCommandMsg::ExportedImageFileSet(Ok(()), export_model) => {
                println!("Fileset exported successfully: {:?}", export_model);

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
                        println!("Selected file: {:?}", selected_file);
                        let image_path = export_model.output_dir.join(&selected_file.file_name);

                        self.selected_image = image_path;
                    } else {
                        eprintln!("Selected file not found in the file set.");
                    }
                }
            }
            ImageViewerCommandMsg::ExportedImageFileSet(Err(e), _) => {
                // Handle export error, e.g., show an error message
                eprintln!("Failed to export fileset: {}", e);
            }
        }
    }
}
