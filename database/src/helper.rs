use core_types::{FileType, ImportedFile};

pub struct AddFileSetParams<'a> {
    pub file_set_name: &'a str,
    pub file_set_file_name: &'a str,
    pub file_type: &'a FileType,
    pub source: &'a str,
    pub files_in_fileset: &'a [ImportedFile],
    pub system_ids: &'a [i64],
}

#[derive(Default)]
pub struct AddDatFileParams<'a> {
    pub dat_id: i32,
    pub name: &'a str,
    pub description: &'a str,
    pub version: &'a str,
    pub date: Option<&'a str>,
    pub author: &'a str,
    pub homepage: Option<&'a str>,
    pub url: Option<&'a str>,
    pub subset: Option<&'a str>,
    pub system_id: i64,
}

pub struct AddDatGameParams<'a> {
    pub dat_file_id: i64,
    pub name: &'a str,
    pub game_id: Option<&'a str>,
    pub description: &'a str,
    pub cloneof: Option<&'a str>,
    pub cloneofid: Option<&'a str>,
}

pub struct AddDatRomParams<'a> {
    pub dat_game_id: i64,
    pub name: &'a str,
    pub size: i64,
    pub crc: &'a str,
    pub md5: &'a str,
    pub sha1: &'a str,
    pub sha256: Option<&'a str>,
    pub status: Option<&'a str>,
    pub serial: Option<&'a str>,
    pub header: Option<&'a str>,
}
