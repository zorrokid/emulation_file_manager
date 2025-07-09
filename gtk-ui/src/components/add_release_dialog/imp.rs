use crate::components::system_dialog::SystemDialog;
use crate::objects::repository_manager::RepositoryManagerObject;
use crate::objects::system::SystemObject;
use gtk::gio;
use gtk::glib;
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/org/zorrokid/emufiles/release_dialog.ui")]
pub struct AddReleaseDialog {
    #[template_child(id = "release_name_entry")]
    pub release_name_entry: TemplateChild<gtk::Entry>,
    #[template_child(id = "system_dropdown")]
    pub system_dropdown: TemplateChild<gtk::DropDown>,
    #[template_child(id = "add_system_button")]
    pub add_system_button: TemplateChild<gtk::Button>,
    #[template_child(id = "select_system_button")]
    pub select_system_button: TemplateChild<gtk::Button>,
    #[template_child(id = "selected_systems_list")]
    pub selected_systems_list: TemplateChild<gtk::ListBox>,
    repository_manager: std::cell::OnceCell<RepositoryManagerObject>,
}

#[glib::object_subclass]
impl ObjectSubclass for AddReleaseDialog {
    const NAME: &'static str = "AddReleaseDialog";
    type Type = super::AddReleaseDialog;
    type ParentType = gtk::Dialog;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for AddReleaseDialog {
    fn constructed(&self) {
        self.parent_constructed();

        let obj = self.obj();
        obj.connect_close_request(|dialog| {
            dialog.hide();
            gtk::glib::Propagation::Stop
        });

        let system_store = gio::ListStore::new::<SystemObject>();
        self.system_dropdown.get().set_model(Some(&system_store));

        // Add system button logic
        self.add_system_button.get().connect_clicked(clone!(
            #[weak(rename_to = imp)]
            self,
            move |_| {
                let dialog = SystemDialog::new();

                dialog.set_transient_for(Some(&imp.obj().toplevel_window().unwrap()));
                dialog.show();

                dialog.connect_response(clone!(
                    #[weak]
                    system_store,
                    move |dialog, response| {
                        println!("Response received: {:?}", response);
                        if response == gtk::ResponseType::Accept {
                            if let Some(new_system) = dialog.get_system_name() {
                                println!("New system added: {}", new_system);
                                //let gstr = glib::String::from(new_system);
                                //system_store.append(&gstr);
                            }
                        }
                        dialog.hide();
                    }
                ));
            }
        ));

        // Select system button logic
        self.select_system_button.get().connect_clicked(clone!(
            #[weak(rename_to = imp)]
            self,
            move |_| {
                let dropdown = imp.system_dropdown.get();
                if let Some(selected) = dropdown.selected_item() {
                    let label = gtk::Label::new(Some(
                        "TODO", //selected.downcast_ref::<glib::String>().unwrap().as_str(),
                    ));
                    imp.selected_systems_list.get().append(&label);
                }
            }
        ));
    }

    /*fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        match pspec.name() {
            "repository-manager" => {
                let repo_manager = value
                    .get::<RepositoryManagerObject>()
                    .expect("type checked upstream");
                self.repository_manager.set(repo_manager).ok();
            }
            _ => unimplemented!(),
        }
    }*/

    /*fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.name() {
            "repository-manager" => self.repository_manager.get().cloned().to_value(),
            _ => unimplemented!(),
        }
    }*/
}

impl WidgetImpl for AddReleaseDialog {}
impl WindowImpl for AddReleaseDialog {}
impl DialogImpl for AddReleaseDialog {}
