use gtk::glib;
use gtk::subclass::prelude::*;

#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/org/zorrokid/emufiles/simple_dialog.ui")]
pub struct SimpleDialog;

#[glib::object_subclass]
impl ObjectSubclass for SimpleDialog {
    const NAME: &'static str = "SimpleDialog";
    type Type = super::SimpleDialog;
    type ParentType = gtk::Dialog;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for SimpleDialog {}
impl WidgetImpl for SimpleDialog {}
impl WindowImpl for SimpleDialog {}
impl DialogImpl for SimpleDialog {}
