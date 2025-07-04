mod imp;

use crate::objects::repository_manager::RepositoryManagerObject;
use glib::Object;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

glib::wrapper! {
    pub struct AddReleaseDialog(ObjectSubclass<imp::AddReleaseDialog>)
        @extends gtk::Dialog, gtk::Window, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl AddReleaseDialog {
    pub fn new(repository_manager: &RepositoryManagerObject) -> Self {
        Object::builder()
            .property("repository-manager", repository_manager)
            .build()
    }

    pub fn repository_manager(&self) -> RepositoryManagerObject {
        self.property::<RepositoryManagerObject>("repository-manager")
    }
}
