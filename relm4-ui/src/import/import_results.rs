use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller,
    gtk::{
        self, glib,
        prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt},
    },
};
use service::mass_import::models::{FileImportResult, FileSetImportStatus};
use ui_components::message_list_view::{
    MessageListItem, MessageListView, MessageListViewMsg, MessageStatus,
};

#[derive(Debug)]
pub struct ImportResults {
    message_list_view: Controller<MessageListView>,
}

#[derive(Debug)]
pub enum ImportResultsMsg {
    Show(FileImportResult),
    Hide,
}

#[relm4::component(pub)]
impl Component for ImportResults {
    type Init = ();
    type Input = ImportResultsMsg;
    type Output = ();
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Window {
            set_default_width: 400,
            set_default_height: 600,
            set_title: Some("Import Results"),
            connect_close_request[sender] => move |_| {
                sender.input(ImportResultsMsg::Hide);
                glib::Propagation::Proceed
            },
             gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                gtk::Box {
                    append = model.message_list_view.widget(),
                },
                gtk::Button {
                    set_label: "Close",
                    connect_clicked[sender] => move |_| {
                        sender.input(ImportResultsMsg::Hide);
                    }
                }
             }
        }
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let message_list_view = MessageListView::builder().launch(()).detach();
        let model = ImportResults { message_list_view };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>, root: &Self::Root) {
        let mut messages: Vec<MessageListItem> = Vec::new();
        match msg {
            ImportResultsMsg::Show(result) => {
                messages.extend(
                    result
                        .read_failed_files
                        .into_iter()
                        .map(|path| {
                            let message = format!("Failed to read file: {}", path.display());
                            ui_components::message_list_view::MessageListItem {
                                message,
                                status: MessageStatus::Error,
                            }
                        })
                        .collect::<Vec<_>>(),
                );
                messages.extend(
                    result
                        .dir_scan_errors
                        .into_iter()
                        .map(|error| {
                            let message = format!("Directory scan error: {}", error);
                            ui_components::message_list_view::MessageListItem {
                                message,
                                status: MessageStatus::Error,
                            }
                        })
                        .collect::<Vec<_>>(),
                );
                messages.extend(
                    result
                        .import_results
                        .into_iter()
                        .map(|import_result| {
                            let status = import_result.status.clone();
                            let status_message = match status {
                                FileSetImportStatus::Success => "Import successful".to_string(),
                                FileSetImportStatus::SuccessWithWarnings(warnings) => {
                                    format!(
                                        "Import successful with warnings: {}",
                                        warnings.join(", ")
                                    )
                                }
                                FileSetImportStatus::Failed(error) => {
                                    format!("Import failed: {}", error)
                                }
                                FileSetImportStatus::AlreadyExists => {
                                    "File set already exists".to_string()
                                }
                            };
                            MessageListItem {
                                message: format!(
                                    "File Set '{}': {}",
                                    import_result.file_set_name, status_message
                                ),
                                status: match import_result.status {
                                    FileSetImportStatus::Success => MessageStatus::Info,
                                    FileSetImportStatus::SuccessWithWarnings(_) => {
                                        MessageStatus::Warning
                                    }
                                    FileSetImportStatus::Failed(_) => MessageStatus::Error,
                                    FileSetImportStatus::AlreadyExists => MessageStatus::Info,
                                },
                            }
                        })
                        .collect::<Vec<_>>(),
                );
                self.message_list_view
                    .emit(MessageListViewMsg::SetItems(messages));
                root.show();
            }
            ImportResultsMsg::Hide => {
                root.hide();
            }
        }
    }
}
