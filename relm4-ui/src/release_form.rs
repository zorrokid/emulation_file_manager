use std::sync::Arc;

use database::repository_manager::RepositoryManager;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller,
    gtk::{
        self, gio,
        glib::clone,
        prelude::{BoxExt, ButtonExt, GtkWindowExt},
    },
    typed_view::list::TypedListView,
};
use service::{
    view_model_service::ViewModelService,
    view_models::{ReleaseListModel, SystemListModel},
};

use crate::{
    list_item::ListItem,
    system_selector::{SystemSelectInit, SystemSelectModel, SystemSelectOutputMsg},
};

#[derive(Debug)]
pub enum ReleaseFormMsg {
    OpenSystemSelector,
    SystemSelected(SystemListModel),
}

#[derive(Debug)]
pub enum ReleaseFormOutputMsg {
    ReleaseCreated(ReleaseListModel),
}

#[derive(Debug)]
pub enum CommandMsg {}

#[derive(Debug)]
pub struct ReleaseFormModel {
    view_model_service: Arc<ViewModelService>,
    repository_manager: Arc<RepositoryManager>,
    selected_systems: Vec<SystemListModel>,
    system_selector: Option<Controller<SystemSelectModel>>,
    selected_systems_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
}

#[derive(Debug)]
pub struct Widgets {}

pub struct ReleaseFormInit {
    pub view_model_service: Arc<ViewModelService>,
    pub repository_manager: Arc<RepositoryManager>,
}

impl Component for ReleaseFormModel {
    type Input = ReleaseFormMsg;
    type Output = ReleaseFormOutputMsg;
    type CommandOutput = CommandMsg;
    type Init = ReleaseFormInit;
    type Widgets = Widgets;
    type Root = gtk::Window;

    fn init_root() -> Self::Root {
        gtk::Window::builder()
            .title("Release Form")
            .default_width(800)
            .default_height(800)
            .build()
    }

    fn init(
        init_model: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let selected_systems_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection> =
            TypedListView::new();

        let v_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        let label = gtk::Label::new(Some("Release Form Component"));

        v_box.append(&label);

        let select_system_button = gtk::Button::with_label("Select System");
        select_system_button.connect_clicked(clone!(
            #[strong]
            sender,
            move |_| {
                sender.input(ReleaseFormMsg::OpenSystemSelector);
                println!("Select System button clicked");
            }
        ));

        v_box.append(&selected_systems_list_view_wrapper.view);
        v_box.append(&select_system_button);

        root.set_child(Some(&v_box));

        let widgets = Widgets {};

        let model = ReleaseFormModel {
            view_model_service: init_model.view_model_service,
            repository_manager: init_model.repository_manager,
            selected_systems: Vec::new(),
            system_selector: None,
            selected_systems_list_view_wrapper,
        };
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _: &Self::Root) {
        match msg {
            ReleaseFormMsg::OpenSystemSelector => {
                let init_model = SystemSelectInit {
                    view_model_service: Arc::clone(&self.view_model_service),
                    repository_manager: Arc::clone(&self.repository_manager),
                };
                let system_selector = SystemSelectModel::builder().launch(init_model).forward(
                    sender.input_sender(),
                    |msg| match msg {
                        SystemSelectOutputMsg::SystemSelected(system_list_model) => {
                            ReleaseFormMsg::SystemSelected(system_list_model)
                        }
                    },
                );
                self.system_selector = Some(system_selector);

                self.system_selector
                    .as_ref()
                    .expect("System selector should be set")
                    .widget()
                    .present();
            }
            ReleaseFormMsg::SystemSelected(system) => {
                println!("System selected: {:?}", &system);
                self.selected_systems_list_view_wrapper.append(ListItem {
                    name: system.name.clone(),
                    id: system.id,
                });
                self.selected_systems.push(system);
            }
        }
    }

    fn update_cmd(
        &mut self,
        _message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _: &Self::Root,
    ) {
    }
}
