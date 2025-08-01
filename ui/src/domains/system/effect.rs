use service::view_models::SystemListModel;

#[derive(Debug, Clone)]
pub enum SystemEffect {
    SystemsFetched(Vec<SystemListModel>),
}

pub trait HandleSystemEffect {
    fn handle_system_effect(&mut self, effect: SystemEffect);
}
