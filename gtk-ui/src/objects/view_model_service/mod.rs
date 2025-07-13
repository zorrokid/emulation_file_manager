mod imp;
use std::sync::Arc;

use gtk::glib::{self, object::Cast, subclass::types::ObjectSubclassIsExt, Object};
use service::{
    error::Error,
    view_model_service::{ReleaseFilter, ViewModelService},
};

use super::{release::ReleaseObject, software_title::SoftwareTitleObject, system::SystemObject};

glib::wrapper! {
    pub struct ViewModelServiceObject(ObjectSubclass<imp::ViewModelServiceObject>);
}

impl ViewModelServiceObject {
    pub fn new(service: Arc<ViewModelService>) -> Self {
        let obj: Self = Object::new::<Self>()
            .downcast()
            .expect("Failed to create ViewModelServiceObject");
        obj.imp().service.set(service).expect("Already initialized");
        obj
    }

    pub fn service(&self) -> &Arc<ViewModelService> {
        self.imp().service.get().expect("Not initialized")
    }

    pub async fn get_software_title_list_models(&self) -> Result<Vec<SoftwareTitleObject>, Error> {
        let res = self.service().get_software_title_list_models().await;
        match res {
            Ok(software_titles) => Ok(software_titles
                .iter()
                .map(|st| SoftwareTitleObject::new(st.id, st.name.clone()))
                .collect()),
            Err(error) => Err(error),
        }
    }

    pub async fn get_software_title_releases(&self, id: i64) -> Result<Vec<ReleaseObject>, Error> {
        let release_filter = ReleaseFilter {
            system_id: None,
            software_title_id: Some(id),
        };
        let res = self.service().get_release_list_models(release_filter).await;
        match res {
            Ok(release_list_models) => Ok(release_list_models
                .into_iter()
                .map(|rlm| ReleaseObject::new(rlm.id, rlm.name, rlm.system_names, rlm.file_types))
                .collect()),
            Err(error) => Err(error),
        }
    }

    pub async fn get_system_list_models(&self) -> Result<Vec<SystemObject>, Error> {
        let res = self.service().get_system_list_models().await;
        match res {
            Ok(system_list_models) => Ok(system_list_models
                .into_iter()
                .map(|slm| SystemObject::new(slm.id, slm.name))
                .collect()),
            Err(error) => Err(error),
        }
    }
}
