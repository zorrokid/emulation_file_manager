use service::view_models::SystemListModel;

use super::effect::{HandleSystemEffect, SystemEffect};

pub struct SystemsWidget {
    systems: Vec<SystemListModel>,
}

pub enum SystemWidgetMessage {
    SetSystems(Vec<SystemListModel>),
    FetchSystems,
}

impl SystemsWidget {
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
        }
    }

    pub fn update(&mut self, message: SystemWidgetMessage) {
        match message {
            SystemWidgetMessage::SetSystems(systems) => {
                self.systems = systems;
            }
        }
    }

    pub fn view(&self) -> String {
        // Placeholder for actual view rendering logic
        format!("Systems: {:?}", self.systems)
    }
}

impl HandleSystemEffect for SystemsWidget {
    fn handle_system_effect(&mut self, effect: SystemEffect) {
        match effect {
            SystemEffect::SystemsFetched(systems) => {
                self.update(SystemWidgetMessage::SetSystems(systems));
            }
        }
    }
}
