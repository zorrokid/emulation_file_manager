pub struct EmulatorViewModel {
    pub id: i64,
    pub name: String,
    pub executable: String,
    pub extract_files: bool,
    pub systems: Vec<EmulatorSystemViewModel>,
}

pub struct EmulatorSystemViewModel {
    pub system_id: i64,
    pub system_name: String,
    pub arguments: String,
}
