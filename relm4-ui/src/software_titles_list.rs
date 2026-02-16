use std::sync::Arc;

use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
    gtk::{
        self,
        glib::{clone, object::ObjectExt},
        prelude::{BoxExt, ButtonExt, OrientableExt, SelectionModelExt, WidgetExt},
    },
    typed_view::list::TypedListView,
};
use service::{
    error::Error, view_model_service::ViewModelService, view_models::SoftwareTitleListModel,
};

use crate::{
    list_item::ListItem,
    software_title_merge_dialog::{
        SoftwareTitleMergeDialog, SoftwareTitleMergeDialogMsg, SoftwareTitleMergeDialogOutputMsg,
    },
};

#[derive(Debug)]
pub struct SoftwareTitlesList {
    view_model_service: Arc<ViewModelService>,
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
}

#[derive(Debug)]
pub enum SoftwareTitleListOutMsg {
    SoftwareTitleSelected { id: i64 },
    SoftwareTitleDeselected { id: i64 },
    ShowError(String),
}

#[derive(Debug)]
pub struct SoftwareTitleListInit {
    pub view_model_service: Arc<ViewModelService>,
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
            view_model_service: init_model.view_model_service,
            list_view_wrapper,
            selected_items: Vec::new(),
            merge_dialog_controller,
        };
        let list_view = &model.list_view_wrapper.view;
        let selection_model = &model.list_view_wrapper.selection_model;
        for prop in selection_model.list_properties() {
            println!("property: {}", prop.name());
        }

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
                let view_model_service = Arc::clone(&self.view_model_service);
                sender.oneshot_command(async move {
                    let res = view_model_service.get_software_title_list_models().await;
                    SoftwareTitleListCmdMsg::SoftwareTitlesFetched(res)
                });
            }
            SoftwareTitleListMsg::Selected { index } => {
                let item = self.list_view_wrapper.get_visible(index);
                if let Some(item) = item {
                    let id = item.borrow().id;
                    sender
                        .output(SoftwareTitleListOutMsg::SoftwareTitleSelected { id })
                        .unwrap_or_else(|res| {
                            eprintln!("Failed to send SoftwareTitleSelected message: {:?}", res);
                        });
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
                            sender.output(SoftwareTitleListOutMsg::SoftwareTitleSelected { id });
                        }
                    } else if let Some(software_title) = self.list_view_wrapper.get_visible(i) {
                        let software_title = software_title.borrow().clone();
                        let id = software_title.id;
                        println!("Deselected: {:?}", software_title);
                        self.selected_items
                            .retain(|item| item.id != software_title.id);
                        sender.output(SoftwareTitleListOutMsg::SoftwareTitleDeselected { id });
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
                // call service to merge selected software titles with the given id
                println!("Start merge with id: {}", id);
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
                tracing::error!(error = ?err, "Failed to fetch software titles");
                sender
                    .output(SoftwareTitleListOutMsg::ShowError(format!(
                        "Failed to fetch software titles: {:?}",
                        err
                    )))
                    .unwrap_or_else(|e| {
                        tracing::error!(
                        error = ?e, "Failed to send ShowError message"
                        )
                    });
            }
        }
    }
}
