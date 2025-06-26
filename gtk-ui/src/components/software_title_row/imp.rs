use std::cell::RefCell;

use glib::Binding;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate, Label};

// Object holding the state
#[derive(Default, CompositeTemplate)]
#[template(resource = "/org/zorrokid/emufiles/software_title_row.ui")]
pub struct SoftwareTitleRow {
    #[template_child]
    pub name_label: TemplateChild<Label>,
    // Vector holding the bindings to properties of `SoftwareTitleObject`
    pub bindings: RefCell<Vec<Binding>>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for SoftwareTitleRow {
    // `NAME` needs to match `class` attribute of template
    const NAME: &'static str = "EmuFilesSoftwareTitleRow";
    type Type = super::SoftwareTitleRow;
    type ParentType = gtk::Box;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

// Trait shared by all GObjects
impl ObjectImpl for SoftwareTitleRow {}

// Trait shared by all widgets
impl WidgetImpl for SoftwareTitleRow {}

// Trait shared by all boxes
impl BoxImpl for SoftwareTitleRow {}
