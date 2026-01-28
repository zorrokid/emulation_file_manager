use core_types::{FileType, ImportedFile};

pub struct AddFileSetParams<'a> {
    pub file_set_name: &'a str,
    pub file_set_file_name: &'a str,
    pub file_type: &'a FileType,
    pub source: &'a str,
    pub files_in_fileset: &'a [ImportedFile],
    pub system_ids: &'a [i64],
}
