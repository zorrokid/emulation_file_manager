#[derive(Debug)]
pub struct LibretroFirmwareInfo {
    pub desc: String,
    pub path: String,
    pub opt: bool,
}

#[derive(Debug)]
pub struct LibretroSystemInfo {
    pub display_name: String,
    pub authors: String,
    pub supported_extensions: Vec<String>,
    pub core_name: String,
    pub categories: Vec<String>,
    pub license: String,
    pub permissions: Vec<String>,
    pub display_version: String,
    pub manufacturer: String,
    pub system_name: String,
    pub sustem_id: String,
    pub databasae: String,
    pub supports_no_game: bool,
    pub firmware: Vec<LibretroFirmwareInfo>,
    pub description: String,
}
