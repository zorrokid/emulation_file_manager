use std::sync::Arc;

use database::{database_error::Error, repository_manager::RepositoryManager};
use relm4::{
    Component, ComponentParts, ComponentSender,
    gtk::{
        self, glib,
        prelude::{
            ButtonExt, EditableExt, EntryBufferExtManual, EntryExt, GtkWindowExt, OrientableExt,
            WidgetExt,
        },
    },
};
use service::view_models::SystemListModel;

#[derive(Debug)]
pub struct SystemFormModel {
    pub name: String,
    pub edit_system_id: Option<i64>,
    pub repository_manager: Arc<RepositoryManager>,
}

#[derive(Debug)]
pub enum SystemFormMsg {
    NameChanged(String),
    Submit,
    Show {
        edit_system: Option<SystemListModel>,
    },
    Hide,
}

#[derive(Debug)]
pub enum SystemFormOutputMsg {
    SystemAdded(SystemListModel),
    SystemUpdated(SystemListModel),
}

#[derive(Debug)]
pub enum SystemFormCommandMsg {
    SystemSubmitted(Result<i64, Error>),
}

#[derive(Debug)]
pub struct SystemFormInit {
    pub repository_manager: Arc<RepositoryManager>,
}

#[relm4::component(pub)]
impl Component for SystemFormModel {
    type Input = SystemFormMsg;
    type Output = SystemFormOutputMsg;
    type CommandOutput = SystemFormCommandMsg;
    type Init = SystemFormInit;

    view! {
        gtk::Window {
            set_title: Some("System Form"),
            set_default_size: (300, 100),
            connect_close_request[sender] => move |_| {
                sender.input(SystemFormMsg::Hide);
                glib::Propagation::Proceed
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_top: 10,
                set_margin_bottom: 10,
                set_margin_start: 10,
                set_margin_end: 10,

                gtk::Label {
                    set_label: "Name",
                },

                #[name = "name_entry"]
                gtk::Entry {
                    set_text: &model.name,
                    set_placeholder_text: Some("System name"),
                    connect_changed[sender] => move |entry| {
                        let buffer = entry.buffer();
                        sender.input(
                            SystemFormMsg::NameChanged(buffer.text().into()),
                        );
                    },
                },

                gtk::Button {
                    set_label: "Submit",
                    #[watch]
                    set_sensitive: !model.name.is_empty(),
                    connect_clicked => SystemFormMsg::Submit,
                },
            },
        }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::Input,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match msg {
            SystemFormMsg::NameChanged(name) => {
                self.name = name;
            }
            SystemFormMsg::Submit => {
                let name = self.name.clone();
                let repository_manager = Arc::clone(&self.repository_manager);
                let edit_id = self.edit_system_id;
                sender.oneshot_command(async move {
                    if let Some(edit_id) = edit_id {
                        println!("Updating system with ID {}: {}", edit_id, name);
                        let result = repository_manager
                            .get_system_repository()
                            .update_system(edit_id, &name)
                            .await;
                        SystemFormCommandMsg::SystemSubmitted(result)
                    } else {
                        println!("Adding new software title: {}", name);
                        let result = repository_manager
                            .get_system_repository()
                            .add_system(&name)
                            .await;
                        SystemFormCommandMsg::SystemSubmitted(result)
                    }
                });
            }
            SystemFormMsg::Show { edit_system } => {
                if let Some(edit_system) = edit_system {
                    self.name = edit_system.name.clone();
                    widgets.name_entry.set_text(&self.name);
                    self.edit_system_id = Some(edit_system.id);
                } else {
                    self.name.clear();
                    widgets.name_entry.set_text("");
                    self.edit_system_id = None;
                }
                root.show();
            }
            SystemFormMsg::Hide => {
                root.hide();
            }
            _ => (),
        }
        // This is essential:
        self.update_view(widgets, sender);
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            SystemFormCommandMsg::SystemSubmitted(Ok(id)) => {
                let res = sender.output(if let Some(edit_id) = self.edit_system_id {
                    SystemFormOutputMsg::SystemUpdated(SystemListModel {
                        id: edit_id,
                        name: self.name.clone(),
                        can_delete: false,
                    })
                } else {
                    SystemFormOutputMsg::SystemAdded(SystemListModel {
                        id,
                        name: self.name.clone(),
                        can_delete: false,
                    })
                });
                if let Err(e) = res {
                    eprintln!("Failed to send output message: {:?}", e);
                }
                root.close();
            }
            SystemFormCommandMsg::SystemSubmitted(Err(e)) => {
                eprintln!("Error submitting software title: {}", e);
            }
            _ => (),
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = SystemFormModel {
            name: "".to_string(),
            edit_system_id: None,
            repository_manager: init.repository_manager,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }
}
