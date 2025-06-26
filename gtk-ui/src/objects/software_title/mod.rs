mod imp;

use glib::Object;
use gtk::glib;

glib::wrapper! {
    pub struct SoftwareTitleObject(ObjectSubclass<imp::SoftwareTitleObject>);
}

impl SoftwareTitleObject {
    pub fn new(name: String) -> Self {
        Object::builder().property("name", name).build()
    }
}

#[derive(Default)]
pub struct SoftwareTitleData {
    pub name: String,
}
