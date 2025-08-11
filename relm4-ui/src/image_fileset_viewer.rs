use std::{path::PathBuf, sync::Arc};

use file_export::{FileExportError, FileSetExportModel, export_files};
use relm4::{
    Component, ComponentParts, ComponentSender, RelmWidgetExt,
    gtk::{
        self,
        glib::clone,
        prelude::{BoxExt, GtkWindowExt, OrientableExt, WidgetExt},
    },
    typed_view::grid::{RelmGridItem, TypedGridView},
};
use service::view_models::{FileSetViewModel, Settings};
use thumbnails::{ThumbnailPathMap, prepare_thumbnails};

use crate::utils::prepare_fileset_for_export;

// grid
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct MyGridItem {
    thumbnail_path: PathBuf,
    image_path: PathBuf,
}

impl MyGridItem {
    fn new(thumbnail_path: PathBuf, image_path: PathBuf) -> Self {
        Self {
            thumbnail_path,
            image_path,
        }
    }
}

struct Widgets {
    thumbnail: gtk::Image,
    button: gtk::Button,
}

impl RelmGridItem for MyGridItem {
    type Root = gtk::Box;
    type Widgets = Widgets;

    fn setup(_item: &gtk::ListItem) -> (gtk::Box, Widgets) {
        relm4::view! {
            my_box = gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_margin_all: 2,
                set_spacing: 5,


                #[name = "thumbnail"]
                gtk::Image {
                    set_pixel_size: 100,
                    set_valign: gtk::Align::Center,

                },

               #[name = "button"]
                gtk::Button,
            }
        }

        let widgets = Widgets { thumbnail, button };

        (my_box, widgets)
    }

    fn bind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        let Widgets { thumbnail, button } = widgets;
        thumbnail.set_from_file(Some(&self.thumbnail_path));
    }
}

//

#[derive(Debug)]
pub enum ImageFilesetViewerMsg {
    FileSelected { index: u32 },
}

#[derive(Debug)]
pub enum ImageFileSetViewerCommandMsg {
    ExportedImageFileSet(Result<(), FileExportError>, FileSetExportModel),
    ThumbnailsPrepared(ThumbnailPathMap),
}

pub struct ImageFileSetViewerInit {
    pub file_set: FileSetViewModel,
    pub settings: Arc<Settings>,
}

#[derive(Debug)]
pub struct ImageFilesetViewer {
    file_set: FileSetViewModel,
    settings: Arc<Settings>,
    thumbnails_mapping: ThumbnailPathMap,
    grid_view_wrapper: TypedGridView<MyGridItem, gtk::SingleSelection>,
    selected_image: PathBuf,
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
            set_default_size: (800, 600),
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                gtk::Label {
                    set_label: &format!("Viewing fileset: {}", model.file_set.file_set_name),
                },

                gtk::ScrolledWindow {
                    set_vexpand: true,

                    #[local_ref]
                    my_view -> gtk::GridView {
                        set_orientation: gtk::Orientation::Vertical,
                        set_max_columns: 3,
                    }
                },

                gtk::ScrolledWindow {
                    set_vexpand: true,
                    set_hexpand: true,
                    #[name = "selected_image"]
                    gtk::Image {
                         #[watch]
                        set_from_file: Some(&model.selected_image),
                        set_pixel_size: 2200,
                        set_valign: gtk::Align::Start,
                        set_halign: gtk::Align::Start,
                    }
                }

           },
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let export_model = prepare_fileset_for_export(
            &init.file_set,
            &init.settings.collection_root_dir,
            // TODO: temp_dir should come from settings
            std::env::temp_dir().as_path(),
            true,
        );

        let grid_view_wrapper: TypedGridView<MyGridItem, gtk::SingleSelection> =
            TypedGridView::new();

        let selection_model = &grid_view_wrapper.selection_model;
        selection_model.connect_selected_notify(clone!(
            #[strong]
            sender,
            move |selection| {
                sender.input(ImageFilesetViewerMsg::FileSelected {
                    index: selection.selected(),
                });
            }
        ));

        let model = ImageFilesetViewer {
            file_set: init.file_set,
            settings: init.settings,
            thumbnails_mapping: ThumbnailPathMap::new(),
            grid_view_wrapper,
            selected_image: PathBuf::new(),
        };
        let my_view = &model.grid_view_wrapper.view;

        let widgets = view_output!();

        sender.spawn_command(move |sender| {
            let res = export_files(&export_model);
            sender.emit(ImageFileSetViewerCommandMsg::ExportedImageFileSet(
                res,
                export_model,
            ));
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            ImageFilesetViewerMsg::FileSelected { index } => {
                if let Some(item) = self.grid_view_wrapper.get(index) {
                    // Handle the selected item, e.g., show the image in a larger view
                    let image_path = item.borrow().image_path.clone();
                    println!("Selected file: {:?}", image_path);
                    let temp_dir = std::env::temp_dir();
                    let path = temp_dir.join(&image_path);
                    self.selected_image = path;
                } else {
                    println!("No item found at index {}", index);
                }
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
            ImageFileSetViewerCommandMsg::ExportedImageFileSet(Ok(()), export_model) => {
                // Handle successful export, e.g., show a success message
                println!("Fileset exported successfully: {:?}", export_model);
                let collection_root_dir = self.settings.collection_root_dir.clone();
                sender.spawn_command(move |sender| {
                    let res = prepare_thumbnails(&export_model, &collection_root_dir);
                    // You can emit a message to update the UI or notify the user
                    match res {
                        Ok(thumbnails_mapping) => {
                            println!("Thumbnails prepared successfully: {:?}", thumbnails_mapping);
                            sender.emit(ImageFileSetViewerCommandMsg::ThumbnailsPrepared(
                                thumbnails_mapping,
                            ));
                        }
                        Err(e) => {
                            eprintln!("Failed to prepare thumbnails: {}", e);
                        }
                    }
                });
            }
            ImageFileSetViewerCommandMsg::ExportedImageFileSet(Err(e), _) => {
                // Handle export error, e.g., show an error message
                eprintln!("Failed to export fileset: {}", e);
            }
            ImageFileSetViewerCommandMsg::ThumbnailsPrepared(thumbnails_mapping) => {
                // Handle thumbnails prepared, e.g., update the UI to show thumbnails
                println!("Thumbnails prepared successfully.");

                let grid_items = thumbnails_mapping
                    .iter()
                    .map(|(file_name, thumbnail_path)| {
                        MyGridItem::new(thumbnail_path.clone(), file_name.clone().into())
                    });

                self.grid_view_wrapper.extend_from_iter(grid_items);

                self.thumbnails_mapping = thumbnails_mapping;
            }
        }
    }
}
