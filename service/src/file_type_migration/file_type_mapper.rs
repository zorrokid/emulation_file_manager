use core_types::{FileType, item_type::ItemType};

pub fn map_old_file_type_to_new(file_type: FileType) -> FileType {
    match file_type {
        FileType::Rom => FileType::Rom,
        FileType::DiskImage => FileType::DiskImage,
        FileType::TapeImage => FileType::TapeImage,
        FileType::Screenshot => FileType::Screenshot,
        FileType::Manual => FileType::Document,
        FileType::CoverScan => FileType::Scan,
        FileType::MemorySnapshot => FileType::MemorySnapshot,
        FileType::LoadingScreen => FileType::Screenshot,
        FileType::TitleScreen => FileType::Screenshot,
        FileType::ManualScan => FileType::Scan,
        FileType::MediaScan => FileType::Scan,
        FileType::InlayScan => FileType::Scan,
        FileType::BoxScan => FileType::Scan,
        FileType::Box => FileType::Document,
        FileType::Document => FileType::Document,
        FileType::Scan => FileType::Scan,
    }
}

pub fn map_old_file_type_to_item_type(file_type: FileType) -> Option<ItemType> {
    match file_type {
        FileType::Manual | FileType::ManualScan => Some(core_types::item_type::ItemType::Manual),
        FileType::Box | FileType::BoxScan => Some(core_types::item_type::ItemType::Box),
        FileType::InlayScan => Some(core_types::item_type::ItemType::InlayCard),
        _ => None,
    }
}
