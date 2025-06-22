use service::view_models::SystemListModel;

pub enum SystemsWidgetMessage {
    FetchSystems,
    SetSystems(Vec<SystemListModel>),
}
