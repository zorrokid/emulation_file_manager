use std::sync::Arc;

use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
    gtk::{
        self,
        glib::clone,
        prelude::{BoxExt, ButtonExt, OrientableExt, SelectionModelExt, WidgetExt},
    },
    typed_view::list::TypedListView,
};
use service::{
    app_services::AppServices, error::Error, software_title_service::SoftwareTitleServiceError,
    view_models::SoftwareTitleListModel,
};

use crate::{
    list_item::ListItem,
    software_title_merge_dialog::{
        SoftwareTitleMergeDialog, SoftwareTitleMergeDialogMsg, SoftwareTitleMergeDialogOutputMsg,
    },
};

#[derive(Debug)]
pub struct SoftwareTitlesList {
    app_services: Arc<AppServices>,
    list_view_wrapper: TypedListView<ListItem, gtk::MultiSelection>,
    selected_items: Vec<ListItem>,
    merge_dialog_controller: Controller<SoftwareTitleMergeDialog>,
}

#[derive(Debug)]
pub enum SoftwareTitleListMsg {
    Selected { index: u32 },
    FetchSoftwareTitles,
    AddSoftwareTitle(SoftwareTitleListModel),
    SelectionChanged { position: u32, n_items: u32 },
    StartMerge,
    StartMergeWith(i64),
}

#[derive(Debug)]
pub enum SoftwareTitleListCmdMsg {
    SoftwareTitlesFetched(Result<Vec<SoftwareTitleListModel>, Error>),
    ProcessMergeResult(Result<(), SoftwareTitleServiceError>),
}

#[derive(Debug)]
pub enum SoftwareTitleListOutMsg {
    SoftwareTitleSelected { id: i64 },
    SoftwareTitleDeselected { id: i64 },
    ClearSelected,
    ShowError(String),
    ShowMessage(String),
}

#[derive(Debug)]
pub struct SoftwareTitleListInit {
    pub app_services: Arc<AppServices>,
}

#[relm4::component(pub)]
impl Component for SoftwareTitlesList {
    type Init = SoftwareTitleListInit;
    type Input = SoftwareTitleListMsg;
    type Output = SoftwareTitleListOutMsg;
    type CommandOutput = SoftwareTitleListCmdMsg;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 10,
            set_margin_all: 10,

            gtk::ScrolledWindow {
                set_vexpand: true,
                #[local_ref]
                list_view -> gtk::ListView {}
            },

            gtk::Button {
                set_label: "Merge",
                connect_clicked => SoftwareTitleListMsg::StartMerge,
                #[watch]
                set_sensitive: model.selected_items.len() > 1,
            }
        }
    }

    fn init(
        init_model: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let list_view_wrapper: TypedListView<ListItem, gtk::MultiSelection> =
            TypedListView::with_sorting();

        let merge_dialog_controller = SoftwareTitleMergeDialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                SoftwareTitleMergeDialogOutputMsg::Selected(item) => {
                    SoftwareTitleListMsg::StartMergeWith(item.id)
                }
            });

        let model = SoftwareTitlesList {
            app_services: init_model.app_services,
            list_view_wrapper,
            selected_items: Vec::new(),
            merge_dialog_controller,
        };
        let list_view = &model.list_view_wrapper.view;
        let selection_model = &model.list_view_wrapper.selection_model;

        selection_model.connect_selection_changed(clone!(
            #[strong]
            sender,
            move |_, position, n_items| {
                sender.input(SoftwareTitleListMsg::SelectionChanged { position, n_items });
            }
        ));

        let widgets = view_output!();
        sender.input(SoftwareTitleListMsg::FetchSoftwareTitles);
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            SoftwareTitleListMsg::FetchSoftwareTitles => {
                let app_services = Arc::clone(&self.app_services);
                sender.oneshot_command(async move {
                    let res = app_services
                        .view_model
                        .get_software_title_list_models()
                        .await;
                    SoftwareTitleListCmdMsg::SoftwareTitlesFetched(res)
                });
            }
            SoftwareTitleListMsg::Selected { index } => {
                let item = self.list_view_wrapper.get_visible(index);
                if let Some(item) = item {
                    let id = item.borrow().id;
                    let res = sender.output(SoftwareTitleListOutMsg::SoftwareTitleSelected { id });
                    if let Err(e) = res {
                        tracing::error!(
                            error = ?e,
                            "Failed to send SoftwareTitleSelected message"
                        );
                    }
                }
            }
            SoftwareTitleListMsg::AddSoftwareTitle(software_title) => {
                let item = ListItem {
                    id: software_title.id,
                    name: software_title.name,
                };
                self.list_view_wrapper.append(item);
            }
            SoftwareTitleListMsg::SelectionChanged { position, n_items } => {
                println!("Selection changed");
                let selection = &self.list_view_wrapper.selection_model;
                for i in position..position + n_items {
                    if selection.is_selected(i) {
                        let software_title = self.list_view_wrapper.get_visible(i);
                        if let Some(software_title) = software_title {
                            let software_title = software_title.borrow().clone();
                            let id = software_title.id;
                            println!("Selected: {:?}", software_title);
                            self.selected_items.push(software_title);
                            let res = sender
                                .output(SoftwareTitleListOutMsg::SoftwareTitleSelected { id });
                            if let Err(e) = res {
                                tracing::error!(
                                    error = ?e,
                                    "Failed to send SoftwareTitleSelected message"
                                );
                            }
                        }
                    } else if let Some(software_title) = self.list_view_wrapper.get_visible(i) {
                        let software_title = software_title.borrow().clone();
                        let id = software_title.id;
                        self.selected_items
                            .retain(|item| item.id != software_title.id);
                        let res =
                            sender.output(SoftwareTitleListOutMsg::SoftwareTitleDeselected { id });
                        if let Err(e) = res {
                            tracing::error!(
                                error = ?e,
                                "Failed to send SoftwareTitleDeselected message"
                            );
                        }
                    }
                }
            }
            SoftwareTitleListMsg::StartMerge => {
                let items_to_merge: Vec<ListItem> = self.selected_items.clone();
                self.merge_dialog_controller
                    .emit(SoftwareTitleMergeDialogMsg::Show {
                        software_titles_to_merge: items_to_merge,
                    });
                println!("Start merge");
            }
            SoftwareTitleListMsg::StartMergeWith(id) => {
                let service = Arc::clone(&self.app_services);
                let ids_to_be_merged: Vec<i64> = self
                    .selected_items
                    .iter()
                    .filter(|item| item.id != id)
                    .map(|item| item.id)
                    .collect();
                sender.oneshot_command(async move {
                    let res = service.software_title.merge(id, &ids_to_be_merged).await;
                    SoftwareTitleListCmdMsg::ProcessMergeResult(res)
                });
            }
        }
    }
    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _: &Self::Root,
    ) {
        match message {
            SoftwareTitleListCmdMsg::SoftwareTitlesFetched(Ok(software_titles)) => {
                let items: Vec<ListItem> = software_titles
                    .into_iter()
                    .map(|title| ListItem {
                        id: title.id,
                        name: title.name,
                    })
                    .collect();
                self.list_view_wrapper.clear();
                self.list_view_wrapper.extend_from_iter(items);
            }
            SoftwareTitleListCmdMsg::SoftwareTitlesFetched(Err(err)) => {
                self.show_error(&sender, "Failed to fetch software titles", &err);
            }
            SoftwareTitleListCmdMsg::ProcessMergeResult(Ok(())) => {
                self.show_message(&sender, "Software titles merged successfully");
                self.selected_items.clear();
                sender.input(SoftwareTitleListMsg::FetchSoftwareTitles);
                let res = sender.output(SoftwareTitleListOutMsg::ClearSelected);
                if let Err(e) = res {
                    tracing::error!(
                        error = ?e,
                        "Failed to send ClearSelected message"
                    );
                }
            }
            SoftwareTitleListCmdMsg::ProcessMergeResult(Err(err)) => {
                self.show_error(&sender, "Failed to merge software titles", &err);
            }
        }
    }
}

impl SoftwareTitlesList {
    fn show_error(
        &self,
        sender: &ComponentSender<Self>,
        message: &str,
        err: &impl std::fmt::Debug,
    ) {
        tracing::error!(error = ?err, message);
        let res = sender.output(SoftwareTitleListOutMsg::ShowError(format!(
            "{}: {:?}",
            message, err
        )));
        if let Err(e) = res {
            tracing::error!(
            error = ?e, "Failed to send ShowError message"
            );
        }
    }

    fn show_message(&self, sender: &ComponentSender<Self>, message: &str) {
        let res = sender.output(SoftwareTitleListOutMsg::ShowMessage(message.to_string()));
        if let Err(e) = res {
            tracing::error!(
            error = ?e, "Failed to send ShowMessage message"
            );
        }
    }
}
