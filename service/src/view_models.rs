pub struct EmulatorViewModel {
    pub id: i64,
    pub name: String,
    pub executable: String,
    pub extract_files: bool,
    pub systems: Vec<EmulatorSystemViewModel>,
}

pub struct EmulatorSystemViewModel {
    pub system: SystemViewModel,
    pub arguments: String,
}

pub struct SystemViewModel {
    pub id: i64,
    pub name: String,
}
