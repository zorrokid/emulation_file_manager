use std::sync::Arc;

use database::{
    database_error::Error, models::SoftwareTitle, repository_manager::RepositoryManager,
};
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
use service::view_models::SoftwareTitleListModel;

#[derive(Debug)]
pub struct SoftwareTitleFormModel {
    pub name: String,
    pub edit_software_title_id: Option<i64>,
    pub repository_manager: Arc<RepositoryManager>,
}

#[derive(Debug)]
pub enum SoftwareTitleFormMsg {
    NameChanged(String),
    Submit,
    Show {
        edit_software_title: Option<SoftwareTitleListModel>,
    },
    Hide,
}

#[derive(Debug)]
pub enum SoftwareTitleFormOutputMsg {
    SoftwareTitleAdded(SoftwareTitleListModel),
    SoftwareTitleUpdated(SoftwareTitleListModel),
}

#[derive(Debug)]
pub enum SoftwareTitleFormCommandMsg {
    SoftwareTitleSubmitted(Result<i64, Error>),
}

#[derive(Debug)]
pub struct SoftwareTitleFormInit {
    pub repository_manager: Arc<RepositoryManager>,
    //pub edit_software_title: Option<SoftwareTitleListModel>,
}

#[relm4::component(pub)]
impl Component for SoftwareTitleFormModel {
    type Input = SoftwareTitleFormMsg;
    type Output = SoftwareTitleFormOutputMsg;
    type CommandOutput = SoftwareTitleFormCommandMsg;
    type Init = SoftwareTitleFormInit;

    view! {
        gtk::Window {
            set_title: Some("Software Title Form"),
            set_default_size: (300, 100),
            connect_close_request[sender] => move |_| {
                sender.input(SoftwareTitleFormMsg::Hide);
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

                gtk::Entry {
                    set_text: &model.name,
                    set_placeholder_text: Some("Software title name"),
                    connect_changed[sender] => move |entry| {
                        let buffer = entry.buffer();
                        sender.input(
                            SoftwareTitleFormMsg::NameChanged(buffer.text().into()),
                        );
                    }
                },
                gtk::Button {
                    set_label: "Submit",
                    #[watch]
                    set_sensitive: !model.name.is_empty(),
                    connect_clicked => SoftwareTitleFormMsg::Submit,
                },
            },
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            SoftwareTitleFormMsg::NameChanged(name) => {
                println!("Name changed to: {}", name);
                self.name = name;
            }
            SoftwareTitleFormMsg::Submit => {
                let name = self.name.clone();
                let repository_manager = Arc::clone(&self.repository_manager);
                let edit_id = self.edit_software_title_id;
                sender.oneshot_command(async move {
                    if let Some(edit_id) = edit_id {
                        println!("Updating software title with ID {}: {}", edit_id, name);
                        let update_software_title = SoftwareTitle {
                            id: edit_id,
                            name: name.clone(),
                            franchise_id: None,
                        };
                        let result = repository_manager
                            .get_software_title_repository()
                            .update_software_title(&update_software_title)
                            .await;
                        SoftwareTitleFormCommandMsg::SoftwareTitleSubmitted(result)
                    } else {
                        println!("Adding new software title: {}", name);
                        let result = repository_manager
                            .get_software_title_repository()
                            .add_software_title(&name, None)
                            .await;
                        SoftwareTitleFormCommandMsg::SoftwareTitleSubmitted(result)
                    }
                });
            }
            SoftwareTitleFormMsg::Show {
                edit_software_title,
            } => {
                if let Some(edit_software_title) = edit_software_title {
                    self.name = edit_software_title.name.clone();
                    self.edit_software_title_id = Some(edit_software_title.id);
                } else {
                    self.name.clear();
                    self.edit_software_title_id = None;
                }
                root.show();
            }
            SoftwareTitleFormMsg::Hide => {
                root.hide();
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
            SoftwareTitleFormCommandMsg::SoftwareTitleSubmitted(Ok(id)) => {
                let res = sender.output(if let Some(edit_id) = self.edit_software_title_id {
                    SoftwareTitleFormOutputMsg::SoftwareTitleUpdated(SoftwareTitleListModel {
                        id: edit_id,
                        name: self.name.clone(),
                        can_delete: false,
                    })
                } else {
                    SoftwareTitleFormOutputMsg::SoftwareTitleAdded(SoftwareTitleListModel {
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
            SoftwareTitleFormCommandMsg::SoftwareTitleSubmitted(Err(e)) => {
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
        let model = SoftwareTitleFormModel {
            name: "".to_string(),         /*init
                                          .edit_software_title
                                          .as_ref()
                                          .map_or("".into(), |st| st.name.clone()),*/
            edit_software_title_id: None, // init.edit_software_title.as_ref().map(|st| st.id),
            repository_manager: init.repository_manager,
        };
        println!("Initialized SoftwareTitleFormModel: {:?}", model);
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }
}
