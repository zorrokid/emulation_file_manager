mod imp;

use crate::objects::release::ReleaseObject;
use crate::objects::software_title::SoftwareTitleObject;
use glib::Object;
use gtk::gio;
use gtk::glib;
use gtk::glib::subclass::types::ObjectSubclassIsExt;

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
        Object::builder().build()
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
}
