use std::{path::PathBuf, sync::Arc};

use file_export::{FileExportError, FileSetExportModel, export_files};
use relm4::{
    Component, ComponentParts, ComponentSender, RelmWidgetExt,
    gtk::{
        self,
        gio::File,
        glib::{self, clone},
        prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt},
    },
    typed_view::grid::{RelmGridItem, TypedGridView},
};
use service::{
    export_service::prepare_fileset_for_export,
    view_models::{FileSetViewModel, Settings},
};
use thumbnails::{ThumbnailPathMap, get_image_size, prepare_thumbnails};

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
    ZoomIn,
    Show { file_set: FileSetViewModel },
    Hide,
}

#[derive(Debug)]
pub enum ImageFileSetViewerCommandMsg {
    ExportedImageFileSet(Result<(), FileExportError>, FileSetExportModel),
    ThumbnailsPrepared(ThumbnailPathMap),
}

pub struct ImageFileSetViewerInit {
    pub settings: Arc<Settings>,
}

#[derive(Debug)]
pub struct ImageFilesetViewer {
    file_set: Option<FileSetViewModel>,
    settings: Arc<Settings>,
    thumbnails_mapping: ThumbnailPathMap,
    grid_view_wrapper: TypedGridView<MyGridItem, gtk::SingleSelection>,
    selected_image: PathBuf,
    image_dimensions: (u32, u32),
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
            connect_close_request[sender] => move |_| {
                sender.input(ImageFilesetViewerMsg::Hide);
                glib::Propagation::Proceed
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                gtk::Label {
                    set_label: &format!("Viewing fileset: {}", model.file_set.as_ref().map_or("None", |fs| &fs.file_set_name)),
                },

                gtk::Paned {
                    set_orientation: gtk::Orientation::Vertical,
                    set_start_child: Some(&thumbnails_grid),
                    set_end_child: Some(&image_view),
                },

                gtk::Button {
                    set_label: "Zoom in",
                    connect_clicked => ImageFilesetViewerMsg::ZoomIn,
                },

                #[name = "thumbnails_grid"]
                gtk::ScrolledWindow {
                    #[local_ref]
                    my_view -> gtk::GridView {
                        set_orientation: gtk::Orientation::Vertical,
                        set_max_columns: 3,
                    },
                },

                #[name = "image_view"]
                gtk::ScrolledWindow {
                    set_vexpand: true,
                    set_hexpand: true,
                    #[name = "selected_image"]
                    gtk::Picture{
                        #[watch]
                        set_file: Some(&File::for_path(&model.selected_image)),
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
            file_set: None,
            settings: init.settings,
            thumbnails_mapping: ThumbnailPathMap::new(),
            grid_view_wrapper,
            selected_image: PathBuf::new(),
            image_dimensions: (0, 0),
        };
        let my_view = &model.grid_view_wrapper.view;
        let widgets = view_output!();
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
                    let image_size = get_image_size(&path).unwrap_or((0, 0));

                    self.selected_image = path;
                    self.image_dimensions = image_size;
                } else {
                    println!("No item found at index {}", index);
                }
            }
            ImageFilesetViewerMsg::ZoomIn => {}
            ImageFilesetViewerMsg::Show { file_set } => {
                let export_model = prepare_fileset_for_export(
                    &file_set,
                    &self.settings.collection_root_dir,
                    &self.settings.temp_output_dir,
                    true,
                );
                sender.spawn_command(move |sender| {
                    let res = export_files(&export_model);
                    sender.emit(ImageFileSetViewerCommandMsg::ExportedImageFileSet(
                        res,
                        export_model,
                    ));
                });

                self.file_set = Some(file_set);
                root.show();
            }
            ImageFilesetViewerMsg::Hide => {
                root.close();
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
