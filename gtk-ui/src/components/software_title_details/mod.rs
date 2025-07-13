mod imp;

use crate::components::add_release_dialog::AddReleaseDialog;
use crate::objects::release::ReleaseObject;
use crate::objects::repository_manager::RepositoryManagerObject;
use crate::objects::software_title::SoftwareTitleObject;
use glib::Object;
use gtk::gio;
use gtk::gio::prelude::ActionMapExt as _;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::object::Cast;
use gtk::glib::subclass::types::ObjectSubclassIsExt;
use gtk::prelude::GtkWindowExt;
use gtk::prelude::WidgetExt;

glib::wrapper! {
    pub struct SoftwareTitleDetails(ObjectSubclass<imp::SoftwareTitleDetails>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl Default for SoftwareTitleDetails {
    fn default() -> Self {
        Self::new()
    }
}

impl SoftwareTitleDetails {
    pub fn new() -> Self {
        let details_pane: Self = Object::builder().build();
        // TODO...?
        details_pane
    }

    pub fn set_software_title(&self, software_title: Option<&SoftwareTitleObject>) {
        let imp = self.imp();
        if let Some(title) = software_title {
            imp.title_label.set_label(&title.name());
            // TODO: update releases list, etc.
        } else {
            imp.title_label.set_label("");
            // TODO: clear releases list, etc.
        }
    }

    pub fn set_releases(&self, releases: Vec<ReleaseObject>) {
        let imp = self.imp();
        let list_store = gio::ListStore::new::<ReleaseObject>();
        for release in releases {
            list_store.append(&release);
        }
        let selection_model = gtk::NoSelection::new(Some(list_store));
        imp.releases_grid.set_model(Some(&selection_model));
        imp.releases_model.set(selection_model).ok();
    }

    pub fn toplevel_window(&self) -> Option<gtk::Window> {
        self.root()
            .and_then(|root| root.downcast::<gtk::Window>().ok())
    }

    /*pub fn register_actions(&self, app: &gtk::Application, repo_manager: &RepositoryManagerObject) {
        println!("Registering actions");
        let add_release_action = gio::SimpleAction::new("add_release", None);
        add_release_action.connect_activate(clone!(
            #[weak(rename_to = details_pane)]
            self,
            #[weak]
            repo_manager,
            move |_, _| {
                println!("Add Release action triggered");
                let dialog = AddReleaseDialog::new(&repo_manager);
                dialog.set_transient_for(details_pane.toplevel_window().as_ref());
                dialog.show();
            }
        ));
        app.add_action(&add_release_action);
    }*/
}
