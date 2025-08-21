use relm4::prelude::*;
use relm4::gtk::prelude::*;
use core_types::FileType;
use ui_components::{FileTypeDropDown, FileTypeSelectedMsg, DropDownOutputMsg};

#[derive(Debug)]
enum AppMsg {
    FileTypeChanged(FileType),
}

struct AppModel {
    selected_file_type: Option<FileType>,
    dropdown: Controller<FileTypeDropDown>,
}

#[relm4::component]
impl SimpleComponent for AppModel {
    type Init = ();
    type Input = AppMsg;
    type Output = ();

    view! {
        gtk::ApplicationWindow {
            set_title: Some("FileType Dropdown Test"),
            set_default_size: (300, 200),

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 12,
                set_margin_all: 12,

                gtk::Label {
                    set_text: "Select a FileType:",
                },

                #[local_ref]
                dropdown_widget -> gtk::Box {},

                gtk::Label {
                    #[watch]
                    set_text: &match &model.selected_file_type {
                        Some(ft) => format!("Selected: {}", ft),
                        None => "No selection".to_string(),
                    },
                },
            }
        }
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let dropdown = FileTypeDropDown::builder()
            .launch(Some(FileType::Rom))
            .forward(sender.input_sender(), |msg| match msg {
                DropDownOutputMsg::ItemSelected(FileTypeSelectedMsg::FileTypeSelected(file_type)) => {
                    AppMsg::FileTypeChanged(file_type)
                }
                _ => unreachable!(),
            });

        let model = AppModel {
            selected_file_type: None,
            dropdown,
        };

        let dropdown_widget = model.dropdown.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            AppMsg::FileTypeChanged(file_type) => {
                self.selected_file_type = Some(file_type);
            }
        }
    }
}

fn main() {
    let app = RelmApp::new("relm4.test.dropdown");
    app.run::<AppModel>(());
}