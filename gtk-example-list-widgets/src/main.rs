mod integer_object;
use gtk::{
    Application, ApplicationWindow, CustomFilter, CustomSorter, FilterChange, FilterListModel,
    Label, ListBox, ListItem, ListView, PolicyType, ScrolledWindow, SignalListItemFactory,
    SingleSelection, SortListModel, SorterChange, Widget, glib,
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
        let item = item
            .downcast_ref::<ListItem>()
            .expect("Failed to downcast to ListItem");
        item.set_child(Some(&label));

        // Bind `list_item->item->number` to `label->label`
        // An expression provides a way to describe references to values. One interesting part here is that these references can be several steps away.
        //
        // Expressions allow us to describe relationships between objects or properties that might not even exist yet. We just had to tell it to change the label whenever the number that belongs to it changes.
        item.property_expression("item")
            .chain_property::<IntegerObject>("number")
            .bind(&label, "label", Widget::NONE);
    });

    let filter = CustomFilter::new(move |obj| {
        let integer_object = obj
            .downcast_ref::<IntegerObject>()
            .expect("Failed to downcast to IntegerObject");
        // This filter will only allow items with an even number to be displayed
        integer_object.number() % 2 == 0
    });

    let filter_model = FilterListModel::new(Some(model), Some(filter.clone()));

    let sorter = CustomSorter::new(move |obj1, obj2| {
        // Get `IntegerObject` from `glib::Object`
        let integer_object_1 = obj1
            .downcast_ref::<IntegerObject>()
            .expect("The object needs to be of type `IntegerObject`.");
        let integer_object_2 = obj2
            .downcast_ref::<IntegerObject>()
            .expect("The object needs to be of type `IntegerObject`.");

        // Get property "number" from `IntegerObject`
        let number_1 = integer_object_1.number();
        let number_2 = integer_object_2.number();

        // Reverse sorting order -> large numbers come first
        number_2.cmp(&number_1).into()
    });
    let sort_model = SortListModel::new(Some(filter_model), Some(sorter.clone()));

    /*factory.connect_bind(move |_, item| {
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
    });*/

    let selection_model = SingleSelection::new(Some(sort_model));
    let list_view = ListView::new(Some(selection_model), Some(factory));

    list_view.connect_activate(move |list_view, position| {
        let model = list_view.model().expect("The model has to be set");
        let ingteger_object = model
            .item(position)
            .and_downcast::<IntegerObject>()
            .expect("Failed to downcast to IntegerObject");
        ingteger_object.increase_number();

        filter.changed(FilterChange::Different);
        sorter.changed(SorterChange::Different);
    });

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
