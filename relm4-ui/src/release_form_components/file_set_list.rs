use std::sync::Arc;

use database::repository_manager::RepositoryManager;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller,
    gtk::{
        self,
        prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt},
    },
    once_cell::sync::OnceCell,
    typed_view::list::{RelmListItem, TypedListView},
};
use service::{
    view_model_service::ViewModelService,
    view_models::{FileSetListModel, Settings},
};

use crate::{
    file_set_form::{FileSetFormInit, FileSetFormModel, FileSetFormMsg, FileSetFormOutputMsg},
    file_set_selector::{
        FileSetSelector, FileSetSelectorInit, FileSetSelectorMsg, FileSetSelectorOutputMsg,
    },
    list_item::HasId,
    release_form::{get_item_ids, get_selected_item_id, remove_selected},
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileSetListItem {
    pub name: String,
    pub id: i64,
    pub file_type: String,
}

impl HasId for FileSetListItem {
    fn id(&self) -> i64 {
        self.id
    }
}

pub struct FileSetListItemWidgets {
    name_label: gtk::Label,
    file_type_label: gtk::Label,
}

impl RelmListItem for FileSetListItem {
    type Root = gtk::Box;
    type Widgets = FileSetListItemWidgets;

    fn setup(_item: &gtk::ListItem) -> (gtk::Box, FileSetListItemWidgets) {
        relm4::view! {
            my_box = gtk::Box {
                set_spacing: 10,
                #[name = "name_label"]
                gtk::Label,
                #[name = "file_type_label"]
                gtk::Label,
            }
        }

        let widgets = FileSetListItemWidgets {
            name_label,
            file_type_label,
        };

        (my_box, widgets)
    }

    fn bind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        let FileSetListItemWidgets {
            name_label,
            file_type_label,
        } = widgets;
        name_label.set_label(&self.name);
        file_type_label.set_label(&self.file_type);
    }
}

#[derive(Debug)]
pub enum FileSetListMsg {
    EditFileSet,
    FileSetSelected(FileSetListModel),
    UnlinkFileSet,
    OpenFileSelector,
    SetSelectedSystemIds(Vec<i64>),
    FileSetUpdated(FileSetListModel),
    ResetItems {
        items: Vec<FileSetListModel>,
        system_ids: Vec<i64>,
    },
}

#[derive(Debug)]
pub enum FileSetListOutputMsg {
    FileSetSelected(i64),
    FileSetUnlinked(i64),
}

#[derive(Debug)]
pub enum CommandMsg {
    //
}

#[derive(Debug)]
pub struct FileSetList {
    view_model_service: Arc<ViewModelService>,
    repository_manager: Arc<RepositoryManager>,
    settings: Arc<Settings>,

    selected_file_sets_list_view_wrapper: TypedListView<FileSetListItem, gtk::SingleSelection>,

    file_set_form: OnceCell<Controller<FileSetFormModel>>,
    file_selector: Controller<FileSetSelector>,
    selected_system_ids: Vec<i64>,
}

pub struct FileSetListInit {
    pub view_model_service: Arc<ViewModelService>,
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
    pub selected_system_ids: Vec<i64>,
}

impl FileSetList {
    fn ensure_file_set_form(&mut self, root: &gtk::Box, sender: &ComponentSender<Self>) {
        if self.file_set_form.get().is_none() {
            tracing::info!("Initializing file set form");
            let file_set_form_init = FileSetFormInit {
                view_model_service: Arc::clone(&self.view_model_service),
                repository_manager: Arc::clone(&self.repository_manager),
                settings: Arc::clone(&self.settings),
            };
            let file_set_form = FileSetFormModel::builder()
                .transient_for(root)
                .launch(file_set_form_init)
                .forward(sender.input_sender(), |msg| match msg {
                    FileSetFormOutputMsg::FileSetUpdated(file_set) => {
                        FileSetListMsg::FileSetUpdated(file_set)
                    }
                    FileSetFormOutputMsg::FileSetCreated(file_set) => {
                        FileSetListMsg::FileSetSelected(file_set)
                    }
                });
            if let Err(e) = self.file_set_form.set(file_set_form) {
                tracing::error!(error = ?e, "Failed to set file set editor");
            }
        }
    }
}

#[relm4::component(pub)]
impl Component for FileSetList {
    type Input = FileSetListMsg;
    type Output = FileSetListOutputMsg;
    type CommandOutput = CommandMsg;
    type Init = FileSetListInit;

    view! {
        #[root]
            gtk::Box {
               set_orientation: gtk::Orientation::Horizontal,
               gtk::ScrolledWindow {
                    set_min_content_height: 360,
                    set_hexpand: true,

                    #[local_ref]
                    selected_file_sets_list_view -> gtk::ListView {}

                },
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_width_request: 250,
                    add_css_class: "button-group",
                     gtk::Button {
                        set_label: "Select File Set",
                        connect_clicked => FileSetListMsg::OpenFileSelector,
                    },
                    gtk::Button {
                        set_label: "Edit File Set",
                        connect_clicked => FileSetListMsg::EditFileSet,
                    },
                    gtk::Button {
                        set_label: "Unlink File Set",
                        connect_clicked => FileSetListMsg::UnlinkFileSet,
                    },
                },
            },
    }

    fn init(
        init_model: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let selected_file_sets_list_view_wrapper: TypedListView<
            FileSetListItem,
            gtk::SingleSelection,
        > = TypedListView::new();
        let file_selector_init_model = FileSetSelectorInit {
            view_model_service: Arc::clone(&init_model.view_model_service),
            repository_manager: Arc::clone(&init_model.repository_manager),
            settings: Arc::clone(&init_model.settings),
        };

        let file_selector = FileSetSelector::builder()
            .transient_for(&root)
            .launch(file_selector_init_model)
            .forward(sender.input_sender(), |msg| match msg {
                FileSetSelectorOutputMsg::FileSetSelected(file_set_liset_model) => {
                    FileSetListMsg::FileSetSelected(file_set_liset_model)
                }
            });

        let model = FileSetList {
            view_model_service: init_model.view_model_service,
            repository_manager: init_model.repository_manager,
            settings: init_model.settings,
            selected_file_sets_list_view_wrapper,
            file_set_form: OnceCell::new(),
            file_selector,
            selected_system_ids: init_model.selected_system_ids,
        };
        let selected_file_sets_list_view = &model.selected_file_sets_list_view_wrapper.view;
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            FileSetListMsg::EditFileSet => {
                let selected = self
                    .selected_file_sets_list_view_wrapper
                    .selection_model
                    .selected();
                if let Some(file_set) = self
                    .selected_file_sets_list_view_wrapper
                    .get_visible(selected)
                {
                    let file_set_id = file_set.borrow().id;
                    tracing::info!(file_set_id = file_set_id, "Editing file set");

                    self.ensure_file_set_form(root, &sender);
                    self.file_set_form
                        .get()
                        .expect("File set form should be initialized")
                        .emit(FileSetFormMsg::ShowEdit { file_set_id });
                }
            }
            FileSetListMsg::FileSetSelected(file_set) => {
                self.selected_file_sets_list_view_wrapper
                    .append(FileSetListItem {
                        name: file_set.file_set_name.clone(),
                        id: file_set.id,
                        file_type: file_set.file_type.to_string(),
                    });
                sender.output(FileSetListOutputMsg::FileSetSelected(file_set.id)).unwrap_or_else(|e| {
                    tracing::error!(error = ?e, "Failed to send FileSetSelected output message");
                });
            }
            FileSetListMsg::UnlinkFileSet => {
                let selected_id = get_selected_item_id(&self.selected_file_sets_list_view_wrapper);
                if let Some(selected_id) = selected_id {
                    remove_selected(&mut self.selected_file_sets_list_view_wrapper);
                    sender.output(FileSetListOutputMsg::FileSetUnlinked(selected_id)).unwrap_or_else(|e| {
                    tracing::error!(error = ?e, "Failed to send FileSetUnlinked output message");
                });
                }
            }
            FileSetListMsg::FileSetUpdated(file_set) => {
                for i in 0..self.selected_file_sets_list_view_wrapper.len() {
                    if let Some(item) = self.selected_file_sets_list_view_wrapper.get(i)
                        && item.borrow().id == file_set.id
                    {
                        item.borrow_mut().name = file_set.file_set_name.clone();
                        break;
                    }
                }
            }
            FileSetListMsg::OpenFileSelector => {
                self.file_selector.emit(FileSetSelectorMsg::Show {
                    selected_system_ids: self.selected_system_ids.clone(),
                    selected_file_set_ids: get_item_ids(&self.selected_file_sets_list_view_wrapper),
                });
            }
            FileSetListMsg::SetSelectedSystemIds(system_ids) => {
                self.selected_system_ids = system_ids;
            }
            FileSetListMsg::ResetItems { items, system_ids } => {
                self.selected_file_sets_list_view_wrapper.clear();
                self.selected_file_sets_list_view_wrapper
                    .extend_from_iter(items.iter().map(|fs| FileSetListItem {
                        id: fs.id,
                        name: fs.file_set_name.clone(),
                        file_type: fs.file_type.to_string(),
                    }));

                self.selected_system_ids = system_ids;
            }
        }
    }
}
