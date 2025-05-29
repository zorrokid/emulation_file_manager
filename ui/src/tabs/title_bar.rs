use iced::{
    widget::{button, row},
    Element,
};

use crate::tabs::tabs_controller::Tab;

#[derive(Debug, Clone)]
pub enum TitleBarMessage {
    TabSelected(Tab),
}

pub struct TitleBar {
    active_tab: Tab,
}

impl TitleBar {
    pub fn new() -> Self {
        Self {
            active_tab: Tab::Home,
        }
    }

    pub fn update(&mut self, message: TitleBarMessage) {
        println!("TitleBar update: {:?}", message);
        match message {
            TitleBarMessage::TabSelected(index) => {
                self.active_tab = index;
            }
        }
    }

    pub fn view(&self) -> Element<TitleBarMessage> {
        let home_button = button("Home").on_press(TitleBarMessage::TabSelected(Tab::Home));
        let settings_button =
            button("Settings").on_press(TitleBarMessage::TabSelected(Tab::Settings));
        let add_release_button =
            button("Add release").on_press(TitleBarMessage::TabSelected(Tab::AddRelease));
        let emulators_button =
            button("Emulators").on_press(TitleBarMessage::TabSelected(Tab::Emulators));
        row![
            home_button,
            settings_button,
            add_release_button,
            emulators_button
        ]
        .into()
    }
}
