mod integer_object;
use gtk::{
    Application, ApplicationWindow, Label, ListBox, ListItem, ListView, PolicyType, ScrolledWindow,
    SignalListItemFactory, SingleSelection, glib,
};
use gtk::{gio, prelude::*};

use integer_object::IntegerObject;

const APP_ID: &str = "org.gtk_rs.ListWidgets1";

fn main() -> glib::ExitCode {
    // Create a new application
    let app = Application::builder().application_id(APP_ID).build();

    // Connect to "activate" signal of `app`
    app.connect_activate(build_ui);

    // Run the application
    app.run()
}

fn build_ui(app: &Application) {
    let vector: Vec<IntegerObject> = (1..=100_000).map(IntegerObject::new).collect();

    let model = gio::ListStore::new::<IntegerObject>();
    model.extend_from_slice(&vector);
    let list_box = ListBox::new();
    for number in 1..=100 {
        let label = Label::new(Some(&number.to_string()));
        list_box.append(&label);
    }

    let factory = SignalListItemFactory::new();
    factory.connect_setup(move |_, item| {
        let label = Label::new(None);
        item.downcast_ref::<ListItem>()
            .expect("Failed to downcast to ListItem")
            .set_child(Some(&label));
    });

    factory.connect_bind(move |_, item| {
        let integer_object = item
            .downcast_ref::<ListItem>()
            .expect("Failed to downcast to ListItem")
            .item()
            .and_downcast::<IntegerObject>()
            .expect("Failed to downcast to IntegerObject");

        let label = item
            .downcast_ref::<ListItem>()
            .expect("Failed to downcast to ListItem")
            .child()
            .and_downcast::<Label>()
            .expect("Failed to downcast to Label");

        label.set_label(&integer_object.number().to_string());
    });

    let selection_model = SingleSelection::new(Some(model));
    let list_view = ListView::new(Some(selection_model), Some(factory));

    let scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .min_content_width(360)
        .child(&list_view)
        .build();

    let window = ApplicationWindow::builder()
        .application(app)
        .title("List Widgets Example")
        .default_height(600)
        .default_width(360)
        .child(&scrolled_window)
        .build();

    window.present();
}
