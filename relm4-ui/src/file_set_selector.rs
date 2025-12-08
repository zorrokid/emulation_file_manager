use std::sync::Arc;

use core_types::FileType;
use database::repository_manager::RepositoryManager;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
    gtk::{
        self,
        glib::{self, clone},
        prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt},
    },
    typed_view::list::TypedListView,
};
use service::{
    error::Error as ServiceError,
    file_set_deletion::{model::FileDeletionResult, service::FileSetDeletionService},
    view_model_service::ViewModelService,
    view_models::{FileSetListModel, Settings},
};
use ui_components::{DropDownOutputMsg, FileTypeDropDown, FileTypeSelectedMsg};

use crate::{
    file_set_details_view::{FileSetDetailsInit, FileSetDetailsMsg, FileSetDetailsView},
    file_set_form::{FileSetFormInit, FileSetFormModel, FileSetFormMsg, FileSetFormOutputMsg},
    list_item::FileSetListItem,
    utils::dialog_utils::{show_error_dialog, show_info_dialog},
};

#[derive(Debug)]
pub enum FileSetSelectorMsg {
    FetchFiles,
    SelectClicked,
    DeleteClicked,
    OpenFileSetForm,
    FileSetCreated(FileSetListModel),
    FileSetSelected,
    FileTypeChanged(FileType),
    Show {
        selected_system_ids: Vec<i64>,
        selected_file_set_ids: Vec<i64>,
    },
    Hide,
    Ignore,
}

#[derive(Debug)]
pub enum FileSetSelectorOutputMsg {
    FileSetSelected(FileSetListModel),
}

#[derive(Debug)]
pub enum CommandMsg {
    FilesFetched(Result<Vec<FileSetListModel>, ServiceError>),
    FilesSetDeletionFinished {
        result: Result<Vec<FileDeletionResult>, ServiceError>,
        id: i64,
    },
}

pub struct FileSetSelectorInit {
    pub view_model_service: Arc<ViewModelService>,
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
}

#[derive(Debug)]
pub struct FileSetSelector {
    view_model_service: Arc<ViewModelService>,
    file_sets: Vec<FileSetListModel>,
    list_view_wrapper: TypedListView<FileSetListItem, gtk::SingleSelection>,
    file_set_form: Controller<FileSetFormModel>,
    selected_system_ids: Vec<i64>,
    selected_file_type: Option<FileType>,
    selected_file_set: Option<FileSetListModel>,
    selected_file_set_ids: Vec<i64>,
    dropdown: Controller<FileTypeDropDown>,
    file_set_details_view: Controller<FileSetDetailsView>,
    file_set_deletion_service: Arc<FileSetDeletionService>,
}

#[relm4::component(pub)]
impl Component for FileSetSelector {
    type Input = FileSetSelectorMsg;
    type Output = FileSetSelectorOutputMsg;
    type CommandOutput = CommandMsg;
    type Init = FileSetSelectorInit;

    view! {
        #[root]
        gtk::Window {
            set_default_width: 800,
            set_default_height: 800,
            set_title: Some("Select File Set"),

            connect_close_request[sender] => move |_| {
                sender.input(FileSetSelectorMsg::Hide);
                glib::Propagation::Stop
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 10,
                set_margin_all: 10,

                gtk::Paned {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_start_child: Some(&main_box),
                    set_end_child: Some(&file_set_details),
                },

                #[name = "main_box"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    gtk::Label {
                        set_label: "File Selector",
                    },
                    #[local_ref]
                    file_types_dropdown -> gtk::Box {},
                    gtk::Button {
                        set_label: "Add File Set",
                        connect_clicked => FileSetSelectorMsg::OpenFileSetForm,
                    },

                    gtk::ScrolledWindow {
                        set_vexpand: true,
                        #[local_ref]
                        file_set_list_view -> gtk::ListView {}
                    },

                    gtk::Button {
                        set_label: "Select File Set",
                        connect_clicked => FileSetSelectorMsg::SelectClicked,
                        #[watch]
                        set_sensitive: model.selected_file_set.is_some() && model.selected_file_type.is_some(),
                    },
                    gtk::Button {
                        set_label: "Delete File Set",
                        connect_clicked => FileSetSelectorMsg::DeleteClicked,
                        #[watch]
                        set_sensitive: model.selected_file_set.is_some() && model.selected_file_set.as_ref().is_some_and(|fs| fs.can_delete),
                    },
                    gtk::Label {
                        set_label: "When deleting a file set, also that actual files will be deleted\nunless they are linked to other file sets\n(in that case only those files that are linked won't be deleted).",
                    },
                },

                #[name = "file_set_details"]
                gtk::Box {
                    #[local_ref]
                    file_set_details_view -> gtk::Box {},
                },
            },
        },
    }

    fn init(
        init_model: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let list_view_wrapper: TypedListView<FileSetListItem, gtk::SingleSelection> =
            TypedListView::new();

        let dropdown = FileTypeDropDown::builder().launch(None).forward(
            sender.input_sender(),
            |msg| match msg {
                DropDownOutputMsg::ItemSelected(FileTypeSelectedMsg::FileTypeSelected(
                    file_type,
                )) => FileSetSelectorMsg::FileTypeChanged(file_type),
                _ => unreachable!(),
            },
        );

        let file_set_form_init_model = FileSetFormInit {
            repository_manager: Arc::clone(&init_model.repository_manager),
            settings: Arc::clone(&init_model.settings),
        };

        let file_set_form = FileSetFormModel::builder()
            .transient_for(&root)
            .launch(file_set_form_init_model)
            .forward(sender.input_sender(), |msg| match msg {
                FileSetFormOutputMsg::FileSetCreated(file_set_list_model) => {
                    FileSetSelectorMsg::FileSetCreated(file_set_list_model)
                }
            });

        let file_set_details_view_init = FileSetDetailsInit {
            view_model_service: Arc::clone(&init_model.view_model_service),
        };

        // TODO: is this needed to be Controller?
        let file_set_details_view = FileSetDetailsView::builder()
            .launch(file_set_details_view_init)
            .forward(sender.input_sender(), |_| FileSetSelectorMsg::Ignore);

        let file_set_deletion_service = Arc::new(FileSetDeletionService::new(
            Arc::clone(&init_model.repository_manager),
            Arc::clone(&init_model.settings),
        ));

        let model = FileSetSelector {
            view_model_service: init_model.view_model_service,
            file_sets: Vec::new(),
            list_view_wrapper,
            file_set_form,
            selected_system_ids: Vec::new(),
            selected_file_type: None,
            selected_file_set: None,
            selected_file_set_ids: Vec::new(),
            dropdown,
            file_set_details_view,
            file_set_deletion_service,
        };
        let file_types_dropdown = model.dropdown.widget();
        let file_set_list_view = &model.list_view_wrapper.view;
        let file_set_details_view = model.file_set_details_view.widget();
        model
            .list_view_wrapper
            .selection_model
            .connect_selected_notify(clone!(
                #[strong]
                sender,
                move |_| {
                    sender.input(FileSetSelectorMsg::FileSetSelected);
                }
            ));
        let widgets = view_output!();
        sender.input(FileSetSelectorMsg::FetchFiles);
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            FileSetSelectorMsg::OpenFileSetForm => {
                if let Some(selected_file_type) = self.selected_file_type {
                    self.file_set_form.emit(FileSetFormMsg::Show {
                        selected_system_ids: self.selected_system_ids.clone(),
                        selected_file_type,
                    });
                }
            }
            FileSetSelectorMsg::FileSetCreated(file_set_list_model) => {
                tracing::info!("File set created with id: {}", file_set_list_model.id);
                self.add_to_list(&file_set_list_model);
            }
            FileSetSelectorMsg::SelectClicked => {
                if let Some(file_set_list_model) = self.get_selected_list_model() {
                    sender
                        .output(FileSetSelectorOutputMsg::FileSetSelected(
                            file_set_list_model,
                        ))
                        .unwrap_or_else(|e| {
                            tracing::error!(
                                "Failed to send FileSetSelected output message: {:?}",
                                e
                            );
                        });
                    root.close();
                }
            }
            FileSetSelectorMsg::FileSetSelected => {
                if let Some(file_set) = self.get_selected_list_model() {
                    tracing::info!("File set selected with id: {}", file_set.id);
                    self.file_set_details_view
                        .emit(FileSetDetailsMsg::LoadFileSet(file_set.id));
                    self.selected_file_set = Some(file_set);
                }
            }
            FileSetSelectorMsg::FileTypeChanged(file_type) => {
                tracing::info!("File type changed to: {:?}", file_type);
                self.selected_file_type = Some(file_type);
                sender.input(FileSetSelectorMsg::FetchFiles);
            }
            FileSetSelectorMsg::FetchFiles => {
                tracing::info!("Fetching file sets for selected systems and file type");
                if let Some(file_type) = self.selected_file_type {
                    let view_model_service = Arc::clone(&self.view_model_service);
                    let system_ids = self.selected_system_ids.clone();
                    sender.oneshot_command(clone!(
                        #[strong]
                        view_model_service,
                        async move {
                            let file_sets = view_model_service
                                .get_file_set_list_models(file_type, &system_ids)
                                .await;
                            CommandMsg::FilesFetched(file_sets)
                        }
                    ));
                }
            }
            FileSetSelectorMsg::Show {
                selected_system_ids,
                selected_file_set_ids,
            } => {
                self.selected_system_ids = selected_system_ids;
                self.selected_file_set_ids = selected_file_set_ids;
                sender.input(FileSetSelectorMsg::FetchFiles);
                root.show();
            }
            FileSetSelectorMsg::Hide => {
                root.hide();
            }
            FileSetSelectorMsg::DeleteClicked => {
                if let Some(selected_file_set) = &self.selected_file_set {
                    let file_set_deletion_service = self.file_set_deletion_service.clone();
                    let file_set_id = selected_file_set.id;

                    sender.oneshot_command(clone!(
                        #[strong]
                        file_set_deletion_service,
                        async move {
                            let res = file_set_deletion_service.delete_file_set(file_set_id).await;
                            CommandMsg::FilesSetDeletionFinished {
                                result: res,
                                id: file_set_id,
                            }
                        }
                    ));
                }
            }
            FileSetSelectorMsg::Ignore => {}
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            CommandMsg::FilesFetched(Ok(file_sets)) => {
                tracing::info!("{} file sets fetched", file_sets.len());
                self.file_sets = file_sets;
                self.list_view_wrapper.clear();
                let list_items = self
                    .file_sets
                    .iter()
                    .filter(|f| !self.selected_file_set_ids.contains(&f.id))
                    .map(|file_set| FileSetListItem {
                        id: file_set.id,
                        name: format!("{} [{}]", file_set.file_set_name, file_set.file_name),
                        file_type: file_set.file_type,
                        can_delete: file_set.can_delete,
                    });
                self.list_view_wrapper.extend_from_iter(list_items);
            }
            CommandMsg::FilesFetched(Err(e)) => {
                show_error_dialog(format!("Error fetching file sets: {}", e), root);
            }
            CommandMsg::FilesSetDeletionFinished { result, id } => match result {
                Err(e) => show_error_dialog(format!("Error deleting file set: {}", e), root),
                Ok(deletion_results) => self.handle_deletion_result(deletion_results, id, root),
            },
        }
    }
}

impl FileSetSelector {
    fn remove_from_list(&mut self, file_set_id: i64) {
        for i in 0..self.list_view_wrapper.len() {
            if let Some(item) = self.list_view_wrapper.get_visible(i)
                && item.borrow().id == file_set_id
            {
                self.list_view_wrapper.remove(i);
                tracing::info!("Removed file set with ID: {} from the list", file_set_id);
                break;
            }
        }
    }

    fn add_to_list(&mut self, file_set: &FileSetListModel) {
        self.list_view_wrapper.append(FileSetListItem {
            id: file_set.id,
            name: file_set.file_set_name.clone(),
            file_type: file_set.file_type,
            can_delete: file_set.can_delete,
        });

        for i in 0..self.list_view_wrapper.len() {
            if let Some(item) = self.list_view_wrapper.get_visible(i)
                && item.borrow().id == file_set.id
            {
                tracing::info!("Selecting newly added file set with ID: {}", file_set.id);
                self.list_view_wrapper.selection_model.set_selected(i);
                break;
            }
        }
    }

    fn get_selected_list_item(&self) -> Option<FileSetListItem> {
        let selected_index = self.list_view_wrapper.selection_model.selected();
        if let Some(item) = self.list_view_wrapper.get_visible(selected_index) {
            let item = item.borrow();
            Some(item.clone())
        } else {
            None
        }
    }

    fn get_selected_list_model(&self) -> Option<FileSetListModel> {
        if let Some(list_item) = self.get_selected_list_item() {
            Some(FileSetListModel {
                id: list_item.id,
                file_set_name: list_item.name.clone(),
                file_type: list_item.file_type,
                file_name: list_item.name.clone(),
                can_delete: list_item.can_delete,
            })
        } else {
            None
        }
    }
    fn handle_deletion_result(
        &mut self,
        deletion_results: Vec<FileDeletionResult>,
        id: i64,
        root: &gtk::Window,
    ) {
        let successful_deletions = deletion_results
            .iter()
            .filter(|r| r.file_deletion_success && r.was_deleted_from_db)
            .collect::<Vec<_>>();

        let failed_deletions = deletion_results
            .iter()
            .filter(|r| !r.file_deletion_success || !r.was_deleted_from_db)
            .collect::<Vec<_>>();

        tracing::info!(
            "File set deletion complete for id {}: {} successful, {} failed",
            id,
            successful_deletions.len(),
            failed_deletions.len()
        );

        // TODO: create better summary dialog
        if !failed_deletions.is_empty() {
            let mut message = String::from("Some files failed to delete:\n");
            for result in &failed_deletions {
                message.push_str(&format!(
                    "- File ID {}: (Deleted from FS: {}, Deleted from DB: {})\n",
                    result.file_info.id, result.file_deletion_success, result.was_deleted_from_db
                ));
                for error in result.error_messages.iter() {
                    message.push_str(&format!("  Error: {}\n", error));
                }
            }
            show_error_dialog(message, root);
        } else if !successful_deletions.is_empty() {
            // TODO: list which files were deleted?
            // TODO: show total amount of files in file set?
            show_info_dialog(format!("File set deleted successfully.",), root);
            self.remove_from_list(id);
        } else {
            show_info_dialog(
                "File set was deleted but no files included in file set were deleted.\nFiles may be linked to other file sets.".to_string(),
                root,
            );
        }
    }
}
