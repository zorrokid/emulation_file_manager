use core_types::events::SyncEvent;
use gtk::prelude::*;
use gtk::{Box as GtkBox, Label, Orientation, ProgressBar};
use relm4::prelude::*;
use relm4::typed_view::list::{RelmListItem, TypedListView};

#[derive(Debug)]
pub enum StatusBarMsg {
    SetStatus(String),
    StartProgress { total: i64 },
    UpdateProgress { done: i64, total: i64 },
    SyncEventReceived(SyncEvent),
    Finish,
    Fail(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum MessageStatus {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MessageListItem {
    message: String,
    status: MessageStatus,
}

pub struct ListItemWidgets {
    label: gtk::Label,
}

impl RelmListItem for MessageListItem {
    type Root = gtk::Box;
    type Widgets = ListItemWidgets;

    fn setup(_item: &gtk::ListItem) -> (gtk::Box, ListItemWidgets) {
        relm4::view! {
            my_box = gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                #[name = "label"]
                gtk::Label,
                // TODO: add an icon for the status
            }
        }

        let widgets = ListItemWidgets { label };

        (my_box, widgets)
    }

    fn bind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        let ListItemWidgets { label } = widgets;
        label.set_label(self.message.as_str());
        // TODO: set the icon based on the status
    }
}

#[tracker::track]
pub struct StatusBarModel {
    status_text: String,
    total: i64,
    done: i64,
    syncing: bool,
    #[tracker::do_not_track]
    message_list_view_wrapper: TypedListView<MessageListItem, gtk::NoSelection>,
}

#[relm4::component(pub)]
impl SimpleComponent for StatusBarModel {
    type Init = ();
    type Input = StatusBarMsg;
    type Output = ();

    view! {
        #[root]
        GtkBox {
            set_orientation: Orientation::Vertical,
            set_spacing: 8,
            set_margin_all: 6,

            GtkBox {
                set_orientation: Orientation::Horizontal,
                set_spacing: 8,
                set_margin_all: 6,

                Label {
                    #[watch]
                    set_label: &model.status_text,
                },

                ProgressBar {
                    #[watch]
                    set_visible: model.syncing,
                    #[watch]
                    set_fraction: if model.total > 0 {
                        model.done as f64 / model.total as f64
                    } else {
                        0.0
                    },
                    set_hexpand: true,
                }
            },
            gtk::ScrolledWindow {
                set_vexpand: true,
                #[local_ref]
                message_list -> gtk::ListView {},
            },

        }
    }

    fn init(_init: (), root: Self::Root, _sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let message_list_view_wrapper = TypedListView::<MessageListItem, gtk::NoSelection>::new();
        let model = StatusBarModel {
            status_text: "Ready.".into(),
            total: 0,
            done: 0,
            syncing: false,
            tracker: 0,
            message_list_view_wrapper,
        };

        let message_list = &model.message_list_view_wrapper.view;
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: StatusBarMsg, sender: ComponentSender<Self>) {
        self.reset();

        match msg {
            StatusBarMsg::SyncEventReceived(event) => {
                self.process_sync_event(event, &sender);
            }
            StatusBarMsg::SetStatus(text) => {
                self.status_text = text;
            }
            StatusBarMsg::StartProgress { total } => {
                self.status_text = "Syncing files...".into();
                self.total = total;
                self.done = 0;
                self.syncing = true;
            }
            StatusBarMsg::UpdateProgress { done, total } => {
                self.done = done;
                self.total = total;
                self.status_text = format!("Synced {done}/{total} files");
            }
            StatusBarMsg::Finish => {
                self.status_text = "Sync complete.".into();
                self.syncing = false;
            }
            StatusBarMsg::Fail(error) => {
                self.status_text = format!("Sync failed: {error}");
                self.syncing = false;
            }
        }
    }
}

impl StatusBarModel {
    fn process_sync_event(&mut self, event: SyncEvent, sender: &ComponentSender<Self>) {
        match event {
            SyncEvent::SyncStarted { total_files_count } => {
                sender.input(StatusBarMsg::StartProgress {
                    total: total_files_count,
                });
                self.message_list_view_wrapper.clear();
                self.message_list_view_wrapper.append(MessageListItem {
                    message: "Sync started.".into(),
                    status: MessageStatus::Info,
                });
            }
            SyncEvent::FileUploadStarted {
                key,
                file_number,
                total_files,
            } => {
                self.message_list_view_wrapper.append(MessageListItem {
                    message: format!("Uploading file {file_number}/{total_files}: {key}"),
                    status: MessageStatus::Info,
                });
            }
            SyncEvent::PartUploaded { key, part } => {
                self.message_list_view_wrapper.append(MessageListItem {
                    message: format!("Uploaded part {part} of file: {key}"),
                    status: MessageStatus::Info,
                });
            }
            SyncEvent::FileUploadCompleted {
                key,
                file_number,
                total_files,
            } => {
                sender.input(StatusBarMsg::UpdateProgress {
                    done: file_number,
                    total: total_files,
                });
                self.message_list_view_wrapper.append(MessageListItem {
                    message: format!("Completed upload of file {file_number}/{total_files}: {key}"),
                    status: MessageStatus::Info,
                });
            }
            SyncEvent::FileUploadFailed {
                key,
                error,
                file_number,
                total_files,
            } => {
                self.message_list_view_wrapper.append(MessageListItem {
                    message: format!(
                        "Failed to upload file {file_number}/{total_files}: {key}. Error: {error}"
                    ),
                    status: MessageStatus::Error,
                });
            }
            SyncEvent::SyncCompleted => {
                sender.input(StatusBarMsg::Finish);
                self.message_list_view_wrapper.append(MessageListItem {
                    message: "Sync completed successfully.".into(),
                    status: MessageStatus::Info,
                });
            }
            SyncEvent::PartUploadFailed { key, error } => {
                self.message_list_view_wrapper.append(MessageListItem {
                    message: format!("Failed to upload part of file: {key}. Error: {error}"),
                    status: MessageStatus::Error,
                });
            }
            SyncEvent::SyncCancelled => {
                sender.input(StatusBarMsg::Finish);
                self.message_list_view_wrapper.append(MessageListItem {
                    message: "Sync cancelled.".into(),
                    status: MessageStatus::Warning,
                });
            }
            _ => { /* Handle other events as needed */ }
        }
    }
}
