use std::sync::Arc;

use relm4::{
    ComponentParts, ComponentSender, RelmWidgetExt,
    gtk::{
        self,
        prelude::{BoxExt, OrientableExt, WidgetExt},
    },
    typed_view::list::TypedListView,
};
use service::{view_model_service::ViewModelService, view_models::FileInfoViewModel};

use crate::list_item::ListItem;

#[derive(Debug)]
pub struct FileInfoDetails {
    view_model_service: Arc<ViewModelService>,
    file_info_view_model: Option<FileInfoViewModel>,
    file_sets_list_view_wrapper: TypedListView<ListItem, gtk::NoSelection>,
}

#[derive(Debug)]
pub enum FileInfoDetailsMsg {
    LoadFileInfo(i64),
}

#[derive(Debug)]
pub enum FileInfoDetailsCmdMsg {
    FileInfoLoaded(Result<FileInfoViewModel, service::error::Error>),
}

pub struct FileInfoDetailsInit {
    pub view_model_service: Arc<ViewModelService>,
}

#[relm4::component(pub)]
impl relm4::Component for FileInfoDetails {
    type Init = FileInfoDetailsInit;
    type Input = FileInfoDetailsMsg;
    type Output = ();
    type CommandOutput = FileInfoDetailsCmdMsg;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 10,
            set_margin_all: 10,

            gtk::Label {
                set_label: "File Sets containing this file:",
            },

            gtk::ScrolledWindow {
                set_vexpand: true,
                #[local_ref]
                file_sets_list -> gtk::ListView {}
            },
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let file_sets_list_view_wrapper = TypedListView::<ListItem, gtk::NoSelection>::new();
        let model = FileInfoDetails {
            view_model_service: init.view_model_service,
            file_info_view_model: None,
            file_sets_list_view_wrapper,
        };
        let file_sets_list = &model.file_sets_list_view_wrapper.view;
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            FileInfoDetailsMsg::LoadFileInfo(file_info_id) => {
                let view_model_service = Arc::clone(&self.view_model_service);
                sender.oneshot_command(async move {
                    let result = view_model_service
                        .get_file_info_view_model(file_info_id)
                        .await;
                    FileInfoDetailsCmdMsg::FileInfoLoaded(result)
                });
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
            FileInfoDetailsCmdMsg::FileInfoLoaded(Ok(file_info_view_model)) => {
                let file_sets_as_list_items = file_info_view_model
                    .belongs_to_file_sets
                    .iter()
                    .map(|fs| ListItem {
                        name: fs.file_name.clone(),
                        id: fs.id,
                    })
                    .collect::<Vec<ListItem>>();
                self.file_info_view_model = Some(file_info_view_model);
                self.file_sets_list_view_wrapper.clear();
                self.file_sets_list_view_wrapper
                    .extend_from_iter(file_sets_as_list_items);
            }
            FileInfoDetailsCmdMsg::FileInfoLoaded(Err(err)) => {
                eprintln!("Error loading file info: {}", err);
            }
        }
    }
}
