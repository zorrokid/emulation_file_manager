mod imp;
use std::sync::Arc;

use database::repository_manager::RepositoryManager;
use gtk::glib::{self, object::Cast, subclass::types::ObjectSubclassIsExt, Object};

glib::wrapper! {
    pub struct RepositoryManagerObject(ObjectSubclass<imp::RepositoryManagerObject>);
}

impl RepositoryManagerObject {
    pub fn new(inner: Arc<RepositoryManager>) -> Self {
        let obj: Self = Object::new::<Self>()
            .downcast()
            .expect("Failed to create RepositoryManagerObject");
        obj.imp().inner.set(inner).expect("Already initialized");
        obj
    }

    pub fn inner(&self) -> &Arc<RepositoryManager> {
        self.imp().inner.get().expect("Not initialized")
    }
}
