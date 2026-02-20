use std::sync::Arc;

use relm4::{
    ComponentParts, ComponentSender, RelmWidgetExt,
    gtk::{
        self,
        prelude::{BoxExt, OrientableExt, WidgetExt},
    },
    typed_view::list::TypedListView,
};
use service::view_models::FileInfoViewModel;

use crate::list_item::ListItem;

#[derive(Debug)]
pub struct FileInfoDetails {
    app_services: Arc<service::app_services::AppServices>,
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
    pub app_services: Arc<service::app_services::AppServices>,
}

#[derive(Debug)]
pub enum FileInfoDetailsOutputMsg {
    ShowError(String),
}

#[relm4::component(pub)]
impl relm4::Component for FileInfoDetails {
    type Init = FileInfoDetailsInit;
    type Input = FileInfoDetailsMsg;
    type Output = FileInfoDetailsOutputMsg;
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

            // show preview of the file if thumbnail is available
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let file_sets_list_view_wrapper = TypedListView::<ListItem, gtk::NoSelection>::new();
        let model = FileInfoDetails {
            app_services: init.app_services,
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
                let app_services = Arc::clone(&self.app_services);
                sender.oneshot_command(async move {
                    let result = app_services
                        .view_model
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
        sender: ComponentSender<Self>,
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
                tracing::error!(
                    error = ?err,
                    "Error loading file info"
                );
                sender
                    .output(FileInfoDetailsOutputMsg::ShowError(format!(
                        "Error loading file info: {}",
                        err
                    )))
                    .unwrap_or_else(|err| {
                        tracing::error!(
                            error = ?err,
                            "Error sending FileInfoDetailsOutputMsg::ShowError"
                        )
                    });
            }
        }
    }
}
