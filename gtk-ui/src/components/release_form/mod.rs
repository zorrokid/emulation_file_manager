mod imp;

use gtk::glib::{self, subclass::types::ObjectSubclassIsExt};

use crate::objects::view_model_service::ViewModelServiceObject;

glib::wrapper! {
    pub struct ReleaseFormWindow(ObjectSubclass<imp::ReleaseFormWindow>)
        @extends gtk::Widget, gtk::Window,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ReleaseFormWindow {
    pub fn new(view_model_service: ViewModelServiceObject) -> Self {
        let obj: Self = glib::Object::new();
        obj.imp()
            .view_model_service
            .set(view_model_service.clone())
            .expect("Already initialized");
        obj
    }
}
