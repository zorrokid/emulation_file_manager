use strum_macros::{Display, EnumIter};

use crate::CoreTypeError;

#[derive(Debug, Clone, PartialEq, Copy, EnumIter, Display, Eq, Ord, PartialOrd, Hash)]
#[repr(u8)]
pub enum ItemType {
    #[strum(serialize = "Disk or Set of Disks")]
    DiskOrSetOfDisks,
    #[strum(serialize = "Tape or Set of Tapes")]
    TapeOrSetOfTapes,
    Manual,
    Box,
    Cartridge,
    #[strum(serialize = "Reference Card")]
    ReferenceCard,
    #[strum(serialize = "Registration Card")]
    RegistrationCard,
    #[strum(serialize = "Inlay Card")]
    InlayCard,
    Poster,
    Map,
    #[strum(serialize = "Keyboard Overlay")]
    KeyboardOverlay,
    #[strum(serialize = "Code Wheel")]
    CodeWheel,
    Advertisement,
    Sticker,
    Book,
    Brochure,
    Other,
}

impl ItemType {
    pub fn to_db_int(&self) -> u8 {
        *self as u8
    }

    pub fn from_db_int(value: u8) -> Result<Self, CoreTypeError> {
        match value {
            0 => Ok(ItemType::DiskOrSetOfDisks),
            1 => Ok(ItemType::TapeOrSetOfTapes),
            2 => Ok(ItemType::Manual),
            3 => Ok(ItemType::Box),
            4 => Ok(ItemType::Cartridge),
            5 => Ok(ItemType::ReferenceCard),
            6 => Ok(ItemType::RegistrationCard),
            7 => Ok(ItemType::InlayCard),
            8 => Ok(ItemType::Poster),
            9 => Ok(ItemType::Map),
            10 => Ok(ItemType::KeyboardOverlay),
            11 => Ok(ItemType::CodeWheel),
            12 => Ok(ItemType::Advertisement),
            13 => Ok(ItemType::Sticker),
            14 => Ok(ItemType::Book),
            15 => Ok(ItemType::Brochure),
            16 => Ok(ItemType::Other),
            _ => Err(CoreTypeError::ConversionError(
                "Failed convert to ItemType".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ItemType;
    use strum::IntoEnumIterator;
    #[test]
    fn test_item_type_db_conversion() {
        for item_type in ItemType::iter() {
            let db_int = item_type.to_db_int();
            let converted_item_type = ItemType::from_db_int(db_int).unwrap();
            assert_eq!(item_type, converted_item_type);
        }
    }
}
