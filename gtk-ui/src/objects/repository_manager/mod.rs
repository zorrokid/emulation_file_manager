mod imp;
use std::sync::Arc;

use database::{database_error::Error, repository_manager::RepositoryManager};
use gtk::glib::{self, object::Cast, subclass::types::ObjectSubclassIsExt, Object};

use super::software_title::SoftwareTitleObject;

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

    pub async fn add_software_title(&self, name: String) -> Result<SoftwareTitleObject, Error> {
        let res = self
            .inner()
            .get_software_title_repository()
            .add_software_title(&name, None)
            .await;

        match res {
            Ok(id) => Ok(SoftwareTitleObject::new(id, name)),
            Err(error) => Err(error),
        }
    }
}
