mod imp;

use glib::Object;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, pango};
use pango::{AttrInt, AttrList};

use crate::objects::software_title::SoftwareTitleObject;

glib::wrapper! {
    pub struct SoftwareTitleRow(ObjectSubclass<imp::SoftwareTitleRow>)
    @extends gtk::Box, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl Default for SoftwareTitleRow {
    fn default() -> Self {
        Self::new()
    }
}

impl SoftwareTitleRow {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn bind(&self, software_title_object: &SoftwareTitleObject) {
        let name_label = self.imp().name_label.get();
        let mut bindings = self.imp().bindings.borrow_mut();

        // Bind `software_title_object.name` to `software_title_row.name_label.label`
        let name_label_binding = software_title_object
            .bind_property("name", &name_label, "label")
            .sync_create()
            .build();
        // Save binding
        bindings.push(name_label_binding);
    }

    pub fn unbind(&self) {
        // Unbind all stored bindings
        for binding in self.imp().bindings.borrow_mut().drain(..) {
            binding.unbind();
        }
    }
}
