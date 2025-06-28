use gtk::glib;
use gtk::glib::subclass::prelude::*;
use gtk::glib::Object;
use service::view_model_service::ViewModelService;
use std::cell::OnceCell;
use std::sync::Arc;

pub struct ViewModelServiceObject {
    pub inner: OnceCell<Arc<ViewModelService>>,
}

#[glib::object_subclass]
impl ObjectSubclass for ViewModelServiceObject {
    const NAME: &'static str = "ViewModelServiceObject";
    type Type = super::ViewModelServiceObject;
    type ParentType = Object;

    fn new() -> Self {
        Self {
            inner: OnceCell::new(),
        }
    }
}

impl ObjectImpl for ViewModelServiceObject {}
