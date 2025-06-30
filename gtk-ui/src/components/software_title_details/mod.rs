mod imp;

use crate::objects::software_title::SoftwareTitleObject;
use glib::Object;
use gtk::glib;
use gtk::subclass::prelude::*;

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
            imp.description_label.set_label("");
            // TODO: clear releases list, etc.
        }
    }
}
