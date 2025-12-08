use std::sync::Arc;

use core_types::FileType;
use database::{database_error::DatabaseError, repository_manager::RepositoryManager};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
    gtk::{
        self,
        glib::{SignalHandlerId, clone, object::ObjectExt},
        prelude::{
            BoxExt, ButtonExt, EditableExt, EntryBufferExtManual, EntryExt, GtkWindowExt, WidgetExt,
        },
    },
};

use service::{
    view_model_service::ViewModelService,
    view_models::{FileSetListModel, FileSetViewModel},
};
use ui_components::{DropDownMsg, DropDownOutputMsg, FileTypeDropDown, FileTypeSelectedMsg};

#[derive(Debug)]
pub struct FileSetEditor {
    repository_manager: Arc<RepositoryManager>,
    view_model_service: Arc<ViewModelService>,
    file_set_id: Option<i64>,
    selected_file_type: Option<FileType>,
    file_set_name: String,
    file_set_file_name: String,
    source: String,
    dropdown: Controller<FileTypeDropDown>,
    file_name_changed_signal_id: SignalHandlerId,
    file_set_name_connect_changed_signal_id: SignalHandlerId,
    source_changed_signal_id: SignalHandlerId,
}

#[derive(Debug)]
pub struct AppWidgets {
    entry_file_name: gtk::Entry,
    entry_set_name: gtk::Entry,
    entry_source: gtk::Entry,
}

#[derive(Debug)]
pub enum FileSetEditorMsg {
    Show { file_set_id: i64 },
    Hide,
    SaveChanges,
    FileSetFileNameChanged(String),
    FileSetNameChanged(String),
    SourceChanged(String),
    FileTypeChanged(FileType),
    FileSetUpdated(Result<i64, DatabaseError>),
}

#[derive(Debug)]
pub enum FileSetEditorOutputMsg {
    FileSetUpdated(FileSetListModel),
}

#[derive(Debug)]
pub enum CommandMsg {
    FileSetFetched(Result<FileSetViewModel, service::error::Error>),
    FileSetUpdated(Result<i64, DatabaseError>, FileSetListModel),
}

pub struct FileSetEditorInit {
    pub view_model_service: Arc<service::view_model_service::ViewModelService>,
    pub repository_manager: Arc<database::repository_manager::RepositoryManager>,
}

impl Component for FileSetEditor {
    type Input = FileSetEditorMsg;
    type Output = FileSetEditorOutputMsg;
    type Init = FileSetEditorInit;
    type CommandOutput = CommandMsg;
    type Root = gtk::Window;
    type Widgets = AppWidgets;

    fn init_root() -> Self::Root {
        gtk::Window::builder()
            .title("Edit File Set")
            .default_width(400)
            .default_height(300)
            .build()
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 5);
        vbox.set_margin_all(5);

        let drop_down = FileTypeDropDown::builder().launch(None).forward(
            sender.input_sender(),
            |msg| match msg {
                DropDownOutputMsg::ItemSelected(FileTypeSelectedMsg::FileTypeSelected(
                    file_type,
                )) => FileSetEditorMsg::FileTypeChanged(file_type),
                _ => unreachable!(),
            },
        );

        let file_types_box = drop_down.widget();

        let entry_file_name = gtk::Entry::new();
        entry_file_name.set_placeholder_text(Some("File Set File Name"));
        let file_name_changed_signal_id = entry_file_name.connect_changed(clone!(
            #[strong]
            sender,
            #[strong]
            entry_file_name,
            move |_| {
                let buffer = entry_file_name.buffer();
                sender.input(FileSetEditorMsg::FileSetFileNameChanged(
                    buffer.text().into(),
                ));
            }
        ));

        let entry_set_name = gtk::Entry::new();
        entry_set_name.set_placeholder_text(Some("File Set Description"));
        let file_set_name_connect_changed_signal_id = entry_set_name.connect_changed(clone!(
            #[strong]
            sender,
            #[strong]
            entry_set_name,
            move |_| {
                let buffer = entry_set_name.buffer();
                sender.input(FileSetEditorMsg::FileSetNameChanged(buffer.text().into()));
            }
        ));

        let entry_source = gtk::Entry::new();
        entry_source.set_placeholder_text(Some("Source (e.g. website URL)"));
        let source_changed_signal_id = entry_source.connect_changed(clone!(
            #[strong]
            sender,
            #[strong]
            entry_source,
            move |_| {
                let buffer = entry_source.buffer();
                sender.input(FileSetEditorMsg::SourceChanged(buffer.text().into()));
            }
        ));

        let button_save = gtk::Button::with_label("Save Changes");
        button_save.connect_clicked(clone!(
            #[strong]
            sender,
            move |_| {
                sender.input(FileSetEditorMsg::SaveChanges);
            }
        ));

        vbox.append(file_types_box);
        vbox.append(&entry_file_name);
        vbox.append(&entry_set_name);
        vbox.append(&entry_source);
        vbox.append(&button_save);

        root.set_child(Some(&vbox));

        let model = FileSetEditor {
            file_set_id: None,
            selected_file_type: None,
            file_set_name: String::new(),
            file_set_file_name: String::new(),
            source: String::new(),
            repository_manager: init.repository_manager,
            view_model_service: init.view_model_service,
            dropdown: drop_down,
            file_name_changed_signal_id,
            file_set_name_connect_changed_signal_id,
            source_changed_signal_id,
        };

        let widgets = AppWidgets {
            entry_file_name,
            entry_set_name,
            entry_source,
        };

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            FileSetEditorMsg::Show { file_set_id } => {
                root.show();
                self.file_set_id = Some(file_set_id);
                let view_model_service = Arc::clone(&self.view_model_service);
                sender.oneshot_command(async move {
                    let result = view_model_service
                        .get_file_set_view_model(file_set_id)
                        .await;
                    CommandMsg::FileSetFetched(result)
                });
            }
            FileSetEditorMsg::Hide => {
                root.hide();
            }
            FileSetEditorMsg::FileSetFileNameChanged(file_name) => {
                if file_name != self.file_set_file_name {
                    self.file_set_file_name = file_name;
                }
            }
            FileSetEditorMsg::FileSetNameChanged(name) => {
                if name != self.file_set_name {
                    self.file_set_name = name;
                }
            }
            FileSetEditorMsg::SourceChanged(source) => {
                if source != self.source {
                    self.source = source;
                }
            }
            FileSetEditorMsg::FileTypeChanged(file_type) => {
                self.selected_file_type = Some(file_type);
            }
            FileSetEditorMsg::SaveChanges => {
                if let (Some(file_set_id), Some(file_type)) =
                    (self.file_set_id, self.selected_file_type)
                {
                    let repository_manager = Arc::clone(&self.repository_manager);
                    let file_set_name = self.file_set_name.clone();
                    let file_set_file_name = self.file_set_file_name.clone();
                    let source = self.source.clone();
                    let file_set_list_model = FileSetListModel {
                        id: file_set_id,
                        file_set_name: file_set_name.clone(),
                        file_name: file_set_file_name.clone(),
                        file_type,
                        can_delete: true,
                    };
                    sender.oneshot_command(async move {
                        let res = repository_manager
                            .get_file_set_repository()
                            .update_file_set(
                                file_set_id,
                                &file_set_file_name,
                                &file_set_name,
                                &source,
                                &file_type,
                            )
                            .await;
                        CommandMsg::FileSetUpdated(res, file_set_list_model)
                    });
                } else {
                    eprintln!("File type or file set ID not selected");
                }
            }
            _ => (),
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            CommandMsg::FileSetFetched(Ok(file_set)) => {
                self.file_set_name = file_set.file_set_name.clone();
                self.file_set_file_name = file_set.file_name.clone();
                self.source = file_set.source.clone();
                self.selected_file_type = Some(file_set.file_type);
                self.dropdown
                    .emit(DropDownMsg::SetSelected(file_set.file_type));
            }
            CommandMsg::FileSetFetched(Err(e)) => {
                eprintln!("Error fetching file set: {:?}", e);
            }
            CommandMsg::FileSetUpdated(Ok(file_set_id), file_set_list_model) => {
                let res =
                    sender.output(FileSetEditorOutputMsg::FileSetUpdated(file_set_list_model));
                if res.is_ok() {
                    println!("File set updated with ID: {}", file_set_id);

                    root.close();
                } else {
                    eprintln!("Failed to send FileSetUpdated message");
                }
            }
            CommandMsg::FileSetUpdated(Err(e), _) => {
                eprintln!("Error updating file set: {:?}", e);
            }
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if widgets.entry_file_name.text() != self.file_set_file_name {
            widgets
                .entry_file_name
                .block_signal(&self.file_name_changed_signal_id);
            widgets.entry_file_name.set_text(&self.file_set_file_name);
            widgets
                .entry_file_name
                .unblock_signal(&self.file_name_changed_signal_id);
        }
        if widgets.entry_set_name.text() != self.file_set_name {
            widgets
                .entry_set_name
                .block_signal(&self.file_set_name_connect_changed_signal_id);
            widgets.entry_set_name.set_text(&self.file_set_name);
            widgets
                .entry_set_name
                .unblock_signal(&self.file_set_name_connect_changed_signal_id);
        }
        if widgets.entry_source.text() != self.source {
            widgets
                .entry_source
                .block_signal(&self.source_changed_signal_id);
            widgets.entry_source.set_text(&self.source);
            widgets
                .entry_source
                .unblock_signal(&self.source_changed_signal_id);
        }
    }
}
