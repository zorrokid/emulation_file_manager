mod imp;
use std::sync::Arc;

use gtk::glib::{self, object::Cast, subclass::types::ObjectSubclassIsExt, Object};
use service::view_model_service::ViewModelService;

glib::wrapper! {
    pub struct ViewModelServiceObject(ObjectSubclass<imp::ViewModelServiceObject>);
}

impl ViewModelServiceObject {
    pub fn new(inner: Arc<ViewModelService>) -> Self {
        let obj: Self = Object::new::<Self>()
            .downcast()
            .expect("Failed to create ViewModelServiceObject");
        obj.imp().inner.set(inner).expect("Already initialized");
        obj
    }

    pub fn inner(&self) -> &Arc<ViewModelService> {
        self.imp().inner.get().expect("Not initialized")
    }
}
