use std::sync::Arc;

use relm4::{
    ComponentParts, ComponentSender, RelmWidgetExt,
    gtk::{
        self,
        prelude::{BoxExt, OrientableExt},
    },
};
use service::{error::Error, view_model_service::ViewModelService, view_models::FileSetViewModel};

#[derive(Debug)]
pub struct FileSetDetailsView {
    view_model_service: Arc<ViewModelService>,
}

#[derive(Debug)]
pub enum FileSetDetailsMsg {
    LoadFileSet(i64),
}

#[derive(Debug)]
pub enum FileSetDetailsCmdMsg {
    FileSetLoaded(Result<FileSetViewModel, Error>),
}

#[derive(Debug)]
pub struct FileSetDetailsInit {
    pub view_model_service: Arc<ViewModelService>,
}

#[relm4::component(pub)]
impl relm4::Component for FileSetDetailsView {
    type Init = FileSetDetailsInit;
    type Input = FileSetDetailsMsg;
    type Output = ();
    type CommandOutput = FileSetDetailsCmdMsg;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 10,
            set_margin_all: 10,

            #[name = "file_set_name_label"]
            gtk::Label {
                set_label: "File Set Details",
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = FileSetDetailsView {
            view_model_service: init.view_model_service,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            FileSetDetailsMsg::LoadFileSet(file_set_id) => {
                let view_model_service = self.view_model_service.clone();
                sender.oneshot_command(async move {
                    let result = view_model_service
                        .get_file_set_view_model(file_set_id)
                        .await;
                    FileSetDetailsCmdMsg::FileSetLoaded(result)
                });
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            FileSetDetailsCmdMsg::FileSetLoaded(Ok(file_set_view_model)) => {
                println!("Loaded File Set: {:?}", file_set_view_model);
            }
            FileSetDetailsCmdMsg::FileSetLoaded(Err(err)) => {
                eprintln!("Error loading File Set: {:?}", err);
            }
        }
    }
}
