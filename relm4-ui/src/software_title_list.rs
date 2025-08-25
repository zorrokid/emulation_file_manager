use std::sync::Arc;

use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
    gtk::{self, glib::clone, prelude::*},
    typed_view::list::TypedListView,
};
use service::{
    error::Error, view_model_service::ViewModelService, view_models::SoftwareTitleListModel,
};

use crate::list_item::ListItem;

#[derive(Debug)]
pub enum SoftwareTitleListMsg {
    FetchSoftwareTitles,
    SoftwareTitleSelected { index: u32 },
}

#[derive(Debug)]
pub struct SoftwareTitleList {
    software_title_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    view_model_service: Arc<ViewModelService>,
}

#[derive(Debug)]
pub enum SoftwareTitleListOutputMsg {
    SoftwareTitleSelected { id: i64 },
}

#[derive(Debug)]
pub enum SoftwareTitleListCommandMsg {
    SoftwareTitlesFetched(Result<Vec<SoftwareTitleListModel>, Error>),
}

pub struct SoftwareTitleListInit {
    pub view_model_service: Arc<ViewModelService>,
}

#[relm4::component(pub)]
impl Component for SoftwareTitleList {
    type Input = SoftwareTitleListMsg;
    type Output = SoftwareTitleListOutputMsg;
    type CommandOutput = SoftwareTitleListCommandMsg;
    type Init = SoftwareTitleListInit;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 10,
            set_margin_all: 10,
            gtk::ScrolledWindow {
                set_vexpand: true,
                #[local_ref]
                software_title_list_view -> gtk::ListView {}
            },

        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = SoftwareTitleList {
            software_title_list_view_wrapper: TypedListView::new(),
            view_model_service: init.view_model_service,
        };

        let software_title_list_view = &model.software_title_list_view_wrapper.view;
        let selection_model = &model.software_title_list_view_wrapper.selection_model;
        selection_model.connect_selected_notify(clone!(
            #[strong]
            sender,
            move |selection| {
                sender.input(SoftwareTitleListMsg::SoftwareTitleSelected {
                    index: selection.selected(),
                })
            }
        ));

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            SoftwareTitleListMsg::FetchSoftwareTitles => {
                let view_model_service = Arc::clone(&self.view_model_service);
                sender.oneshot_command(async move {
                    let res = view_model_service.get_software_title_list_models().await;

                    SoftwareTitleListCommandMsg::SoftwareTitlesFetched(res)
                });
            }
            SoftwareTitleListMsg::SoftwareTitleSelected { index } => {
                println!("Software title selected at index: {}", index);
                let selected = self.software_title_list_view_wrapper.get_visible(index);
                if let Some(item) = selected {
                    println!("Selected item: {:?}", item);
                    let id = item.borrow().id;
                    let res =
                        sender.output(SoftwareTitleListOutputMsg::SoftwareTitleSelected { id });
                    if res.is_err() {
                        eprintln!("Failed to send output message");
                    }
                }
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _: &Self::Root,
    ) {
        match message {
            SoftwareTitleListCommandMsg::SoftwareTitlesFetched(Ok(software_titles)) => {
                let list_items: Vec<ListItem> = software_titles
                    .into_iter()
                    .map(|st| ListItem {
                        id: st.id,
                        name: st.name,
                    })
                    .collect();
                self.software_title_list_view_wrapper.clear();
                self.software_title_list_view_wrapper
                    .extend_from_iter(list_items);
            }
            SoftwareTitleListCommandMsg::SoftwareTitlesFetched(Err(e)) => {
                eprintln!("Error fetching software titles: {:?}", e);
            }
        }
    }
}
