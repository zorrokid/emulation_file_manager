use std::{path::PathBuf, sync::Arc};

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
use service::error::Error as ServiceError;
use service::{
    file_set_download::service::{DownloadResult, DownloadService},
    view_models::FileSetViewModel,
};
use thumbnails::{ThumbnailPathMap, get_image_size};

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
        widgets.thumbnail.set_from_file(Some(&self.thumbnail_path));
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
    HandleDownloadResult(Result<DownloadResult, ServiceError>),
}

pub struct ImageFileSetViewerInit {
    pub download_service: Arc<DownloadService>,
}

#[derive(Debug)]
pub struct ImageFilesetViewer {
    file_set: Option<FileSetViewModel>,
    thumbnails_mapping: ThumbnailPathMap,
    grid_view_wrapper: TypedGridView<MyGridItem, gtk::SingleSelection>,
    selected_image: PathBuf,
    image_dimensions: (u32, u32),
    file_download_service: Arc<DownloadService>,
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
                glib::Propagation::Stop
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
            thumbnails_mapping: ThumbnailPathMap::new(),
            grid_view_wrapper,
            selected_image: PathBuf::new(),
            image_dimensions: (0, 0),
            file_download_service: init.download_service,
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
                let file_set_id = file_set.id;
                self.file_set = Some(file_set);

                let download_service = self.file_download_service.clone();

                sender.oneshot_command(async move {
                    let download_result = download_service
                        .download_file_set(file_set_id, true, None)
                        .await;
                    ImageFileSetViewerCommandMsg::HandleDownloadResult(download_result)
                });

                root.show();
            }
            ImageFilesetViewerMsg::Hide => {
                root.hide();
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
            ImageFileSetViewerCommandMsg::HandleDownloadResult(Ok(download_result)) => {
                let thumbnails_mapping = download_result.thumbnail_path_map;
                let grid_items = thumbnails_mapping
                    .iter()
                    .map(|(file_name, thumbnail_path)| {
                        MyGridItem::new(thumbnail_path.clone(), file_name.clone().into())
                    });

                self.grid_view_wrapper.clear();
                self.grid_view_wrapper.extend_from_iter(grid_items);

                self.thumbnails_mapping = thumbnails_mapping;
            }
            ImageFileSetViewerCommandMsg::HandleDownloadResult(Err(e)) => {
                eprintln!("Failed to download fileset: {}", e);
            }
        }
    }
}
