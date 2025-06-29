mod imp;

use glib::Object;
use gtk::glib;

glib::wrapper! {
    pub struct SoftwareTitleObject(ObjectSubclass<imp::SoftwareTitleObject>);
}

impl SoftwareTitleObject {
    pub fn new(id: i64, name: String) -> Self {
        Object::builder()
            .property("id", id)
            .property("name", name)
            .build()
    }
}

#[derive(Default)]
pub struct SoftwareTitleData {
    pub id: i64,
    pub name: String,
}
