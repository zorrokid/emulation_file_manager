use database::repository_manager::RepositoryManager;
use gtk::glib;
use gtk::glib::subclass::prelude::*;
use gtk::glib::Object;
use std::cell::OnceCell;
use std::sync::Arc;

pub struct RepositoryManagerObject {
    pub inner: OnceCell<Arc<RepositoryManager>>,
}

#[glib::object_subclass]
impl ObjectSubclass for RepositoryManagerObject {
    const NAME: &'static str = "RepositoryManagerObject";
    type Type = super::RepositoryManagerObject;
    type ParentType = Object;

    fn new() -> Self {
        Self {
            inner: OnceCell::new(),
        }
    }
}

impl ObjectImpl for RepositoryManagerObject {}
