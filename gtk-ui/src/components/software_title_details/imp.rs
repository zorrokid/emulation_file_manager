use crate::components::release_form::ReleaseFormWindow;
use glib::Properties;
use gtk::gio;
use gtk::glib;
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use crate::objects::repository_manager::RepositoryManagerObject;

#[derive(Default, gtk::CompositeTemplate, Properties)]
#[properties(wrapper_type = super::SoftwareTitleDetails)]
#[template(resource = "/org/zorrokid/emufiles/software_title_details.ui")]
pub struct SoftwareTitleDetails {
    #[template_child(id = "title_label")]
    pub title_label: TemplateChild<gtk::Label>,
    #[template_child(id = "add_release_button")]
    pub add_release_button: TemplateChild<gtk::Button>,
    #[template_child(id = "releases_grid")]
    pub releases_grid: TemplateChild<gtk::GridView>,
    pub releases_model: std::cell::OnceCell<gtk::NoSelection>,
    pub repo_manager: std::cell::OnceCell<RepositoryManagerObject>,
    #[property(get, set)]
    pub view_model_service:
        std::cell::OnceCell<crate::objects::view_model_service::ViewModelServiceObject>,
}

#[glib::object_subclass]
impl ObjectSubclass for SoftwareTitleDetails {
    const NAME: &'static str = "SoftwareTitleDetails";
    type Type = super::SoftwareTitleDetails;
    type ParentType = gtk::Box;
    type Interfaces = ();

    fn class_init(klass: &mut Self::Class) {
        Self::bind_template(klass);
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

// Trait shared by all GObjects
impl ObjectImpl for SoftwareTitleDetails {
    fn constructed(&self) {
        self.parent_constructed();

        let imp = self;
        let view_model_service = imp
            .view_model_service
            .get()
            .expect("ViewModelService not initialized");

        imp.add_release_button.connect_clicked(clone!(
            #[weak]
            view_model_service,
            move |_| {
                let win = ReleaseFormWindow::new(view_model_service);
                win.present();
            }
        ));
    }
}

// Trait shared by all widgets
impl WidgetImpl for SoftwareTitleDetails {}

// Trait shared by all boxes
impl BoxImpl for SoftwareTitleDetails {}
