use std::{path::PathBuf, sync::Arc};

use database::{database_error::Error as DatabaseError, repository_manager::RepositoryManager};
use relm4::{
    Component, ComponentParts, ComponentSender,
    gtk::{
        self, FileChooserDialog,
        ffi::GtkFileDialog,
        gio::prelude::FileExt,
        glib::clone,
        prelude::{BoxExt, ButtonExt, DialogExt, FileChooserExt, GtkWindowExt},
    },
    typed_view::list::TypedListView,
};
use service::{
    error::Error as ServiceError, view_model_service::ViewModelService,
    view_models::FileSetListModel,
};

use crate::file_importer::FileImporter;

#[derive(Debug)]
pub enum FileSetFormMsg {
    OpenFileSelector,
    FileSelected(PathBuf),
}

#[derive(Debug)]
pub enum FileSetFormOutputMsg {
    FileSetCreated(FileSetListModel),
}

#[derive(Debug)]
pub enum CommandMsg {
    FileSelected,
}

pub struct FileSetFormInit {
    pub view_model_service: Arc<ViewModelService>,
    pub repository_manager: Arc<RepositoryManager>,
}

#[derive(Debug)]
pub struct FileSetFormModel {
    view_model_service: Arc<ViewModelService>,
    repository_manager: Arc<RepositoryManager>,
    file_importer: FileImporter,
}

#[derive(Debug)]
pub struct Widgets {
    selected_file_label: gtk::Label,
}

impl Component for FileSetFormModel {
    type Input = FileSetFormMsg;
    type Output = FileSetFormOutputMsg;
    type CommandOutput = CommandMsg;
    type Init = FileSetFormInit;
    type Widgets = Widgets;
    type Root = gtk::Window;

    fn init_root() -> Self::Root {
        gtk::Window::builder()
            .title("Create file set")
            .default_width(800)
            .default_height(800)
            .build()
    }
    fn init(
        init_model: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let open_file_selector_button = gtk::Button::with_label("Open file selector");
        open_file_selector_button.connect_clicked(clone!(
            #[strong]
            sender,
            move |_| {
                sender.input(FileSetFormMsg::OpenFileSelector);
            }
        ));
        let v_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        v_box.append(&open_file_selector_button);
        root.set_child(Some(&v_box));

        let model = FileSetFormModel {
            view_model_service: init_model.view_model_service,
            repository_manager: init_model.repository_manager,
            file_importer: FileImporter::new(),
        };
        let selected_file_label = gtk::Label::new(Some("No file selected"));
        v_box.append(&selected_file_label);
        let widgets = Widgets {
            selected_file_label,
        };
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            FileSetFormMsg::OpenFileSelector => {
                println!("Open file selector button clicked");
                let dialog = FileChooserDialog::builder()
                    .title("Select Files")
                    .action(gtk::FileChooserAction::Open)
                    .modal(true)
                    .transient_for(root)
                    .build();

                dialog.add_button("Cancel", gtk::ResponseType::Cancel);
                dialog.add_button("Open", gtk::ResponseType::Accept);

                dialog.connect_response(clone!(
                    #[strong]
                    sender,
                    move |dialog, response| {
                        if response == gtk::ResponseType::Accept {
                            if let Some(path) = dialog.file().and_then(|f| f.path()) {
                                sender.input(FileSetFormMsg::FileSelected(path));
                            }
                        }
                        dialog.close();
                    }
                ));

                dialog.present();
            }
            FileSetFormMsg::FileSelected(path) => {
                println!("File selected: {:?}", path);
                self.file_importer.set_current_picked_file(path);
                // TODO: handle selected file
            }
            _ => {

                // Handle input messages here
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
            _ => {
                // Handle command outputs here
            }
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        let selected_file_text = self
            .file_importer
            .get_current_picked_file()
            .map_or("No file selected".to_string(), |path| {
                path.to_string_lossy().to_string()
            });
        widgets
            .selected_file_label
            .set_text(selected_file_text.as_str());
    }
}
