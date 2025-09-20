use std::path::{Path, PathBuf};

use core_types::FileType;

pub fn resolve_file_type_path(root_path: &Path, file_type: &FileType) -> PathBuf {
    let mut path = PathBuf::from(root_path);
    path.push(file_type.dir_name());
    path
}
