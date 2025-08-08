use std::{path::PathBuf, sync::Arc};

use file_export::{FileExportError, FileSetExportModel, export_files};
use relm4::{
    Component, ComponentParts, ComponentSender,
    gtk::{
        self,
        prelude::{ButtonExt, OrientableExt, WidgetExt},
    },
};
use service::view_models::{FileSetViewModel, Settings};

use crate::utils::prepare_fileset_for_export;

#[derive(Debug)]
pub enum ImageViewerMsg {
    ShowPrevious,
    ShowNext,
    SetFileSet { file_set: FileSetViewModel },
    Clear,
}

#[derive(Debug)]
pub enum ImageViewerCommandMsg {
    ExportedImageFileSet(Result<(), FileExportError>, FileSetExportModel),
}

pub struct ImageViewerInit {
    pub settings: Arc<Settings>,
}

#[derive(Debug)]
pub struct ImageViewer {
    file_set: Option<FileSetViewModel>,
    current_file_index: Option<usize>,
    settings: Arc<Settings>,
    selected_image: PathBuf,
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
            set_orientation: gtk::Orientation::Horizontal,
            gtk::Button {
                set_label: "<",
                connect_clicked => ImageViewerMsg::ShowPrevious,
            },
            gtk::ScrolledWindow {
                set_vexpand: true,
                set_hexpand: true,
                #[name = "selected_image"]
                gtk::Image {
                    #[watch]
                    set_from_file: Some(&model.selected_image),
                }
            },
            gtk::Button {
                set_label: ">",
                connect_clicked => ImageViewerMsg::ShowNext,
            },
       },
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = ImageViewer {
            file_set: None,
            settings: init.settings,
            selected_image: PathBuf::new(),
            current_file_index: None,
        };

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
                        let next_image = file_set.files[new_index].archive_file_name.clone();
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
                        let previous_image = file_set.files[new_index].archive_file_name.clone();
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
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
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
