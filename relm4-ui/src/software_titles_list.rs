use std::sync::Arc;

use relm4::{
    Component, ComponentParts, ComponentSender, RelmWidgetExt,
    gtk::{
        self,
        glib::clone,
        prelude::{BoxExt, OrientableExt, WidgetExt},
    },
    typed_view::list::TypedListView,
};
use service::{
    error::Error, view_model_service::ViewModelService, view_models::SoftwareTitleListModel,
};

use crate::list_item::ListItem;

#[derive(Debug)]
pub struct SoftwareTitlesList {
    view_model_service: Arc<ViewModelService>,
    list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
}

#[derive(Debug)]
pub enum SoftwareTitleListMsg {
    Selected { index: u32 },
    FetchSoftwareTitles,
    AddSoftwareTitle(SoftwareTitleListModel),
}

#[derive(Debug)]
pub enum SoftwareTitleListCmdMsg {
    SoftwareTitlesFetched(Result<Vec<SoftwareTitleListModel>, Error>),
}

#[derive(Debug)]
pub enum SoftwareTitleListOutMsg {
    SoftwareTitleSelected { id: i64 },
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
        }
    }

    fn init(
        init_model: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection> =
            TypedListView::with_sorting();

        let model = SoftwareTitlesList {
            view_model_service: init_model.view_model_service,
            list_view_wrapper,
        };
        let list_view = &model.list_view_wrapper.view;
        let selection_model = &model.list_view_wrapper.selection_model;
        selection_model.connect_selected_notify(clone!(
            #[strong]
            sender,
            move |selection| {
                sender.input(SoftwareTitleListMsg::Selected {
                    index: selection.selected(),
                });
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
                    let res = sender.output(SoftwareTitleListOutMsg::SoftwareTitleSelected { id });
                    if res.is_err() {
                        eprintln!("Failed to send SoftwareTitleSelected message");
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
        }
    }
    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
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
                eprintln!("Error fetching software titles: {:?}", err);
            }
        }
    }
}
