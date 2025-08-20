use core_types::FileType;
use relm4::gtk::{self};
use strum::IntoEnumIterator;

pub fn create_file_types_dropdown() -> (gtk::DropDown, Vec<FileType>) {
    let file_types: Vec<FileType> = FileType::iter().collect();

    let file_types_dropdown = gtk::DropDown::builder().build();
    let file_types_to_drop_down: Vec<String> = file_types.iter().map(|ft| ft.to_string()).collect();
    let file_types_str: Vec<&str> = file_types_to_drop_down.iter().map(|s| s.as_str()).collect();

    let file_types_drop_down_model = gtk::StringList::new(&file_types_str);

    file_types_dropdown.set_model(Some(&file_types_drop_down_model));
    file_types_dropdown.set_selected(0);
    (file_types_dropdown, file_types)
}
