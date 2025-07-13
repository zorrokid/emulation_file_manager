use std::cell::OnceCell;

use gtk::glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, Button, CompositeTemplate, DropDown, Entry};

use crate::objects::system::SystemObject;
use crate::objects::view_model_service::ViewModelServiceObject;

#[derive(CompositeTemplate, Default)]
#[template(resource = "/org/zorrokid/emufiles/release_form.ui")]
pub struct ReleaseFormWindow {
    #[template_child(id = "name_entry")]
    pub name_entry: TemplateChild<Entry>,
    #[template_child(id = "system_dropdown")]
    pub system_dropdown: TemplateChild<DropDown>,
    #[template_child(id = "save_button")]
    pub save_button: TemplateChild<Button>,
    pub view_model_service: OnceCell<ViewModelServiceObject>,
}

#[glib::object_subclass]
impl ObjectSubclass for ReleaseFormWindow {
    const NAME: &'static str = "ReleaseFormWindow";
    type Type = super::ReleaseFormWindow;
    type ParentType = gtk::Window;

    fn class_init(klass: &mut Self::Class) {
        Self::bind_template(klass);
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for ReleaseFormWindow {
    fn constructed(&self) {
        self.parent_constructed();

        let systems = vec![
            SystemObject::new(1, "Commodore 64".to_string()),
            SystemObject::new(2, "Nintendo Entertainment System".to_string()),
            SystemObject::new(3, "Sinclair ZX Spectrum".to_string()),
        ];

        let list_store = gtk::gio::ListStore::new::<SystemObject>();
        for sys in systems {
            list_store.append(&sys);
        }
        let expr = gtk::PropertyExpression::new(
            SystemObject::static_type(),
            None::<gtk::Expression>,
            "name",
        );
        self.system_dropdown.set_expression(Some(&expr));
        self.system_dropdown.set_model(Some(&list_store));

        self.save_button.connect_clicked(|_| {
            println!("Saving release...");
        });

        // TODO: do i need to clone the dropdown and service?
        let dropdown = self.system_dropdown.get();
        let service = self
            .view_model_service
            .get()
            .expect("ViewModelService not set");
        // TODO: add view model service to form

        glib::MainContext::default().spawn_local(clone!(
            // TODO; check correct usage for referencing dropdown and service
            #[weak]
            service,
            #[weak]
            list_store,
            async move {
                match service.get_system_list_models().await {
                    Ok(systems) => {
                        for sys in systems {
                            list_store.append(&sys);
                        }
                    }
                    Err(e) => {
                        eprintln!("Error loading systems: {}", e);
                    }
                }
                println!("ReleaseFormWindow initialized with systems.");
            }
        ));
    }
}
impl WidgetImpl for ReleaseFormWindow {}
impl WindowImpl for ReleaseFormWindow {}
