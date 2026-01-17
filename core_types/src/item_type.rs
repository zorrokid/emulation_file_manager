use strum_macros::{Display, EnumIter};

use crate::CoreTypeError;

#[derive(Debug, Clone, PartialEq, Copy, EnumIter, Display, Eq, Ord, PartialOrd, Hash)]
#[repr(u8)]
pub enum ItemType {
    #[strum(serialize = "Disk or Set of Disks")]
    DiskOrSetOfDisks, // FileType: DiskImage, MediaScan
    #[strum(serialize = "Tape or Set of Tapes")]
    TapeOrSetOfTapes, // FileType: TapeImage
    Manual,    // FileType: Manual or ManualScan
    Box,       // FileType: Box or BoxScan
    Cartridge, // FileType: Rom
    #[strum(serialize = "Reference Card")]
    ReferenceCard, // No associated FileType yet
    #[strum(serialize = "Registration Card")]
    RegistrationCard, // No associated FileType yet
    #[strum(serialize = "Inlay Card")]
    InlayCard, // FileType: InlayScan
    Poster,    // No associated FileType yet
    Map,       // No associated FileType yet
    #[strum(serialize = "Keyboard Overlay")]
    KeyboardOverlay, // No associated FileType yet
    #[strum(serialize = "Code Wheel")]
    CodeWheel, // No associated FileType yet
    Advertisement, // No associated FileType yet
    Sticker,   // No associated FileType yet
    Book,      // No associated FileType yet
    Brochure,  // No associated FileType yet
    Other,     // No associated FileType yet
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

    #[test]
    fn test_invalid_db_int() {
        let invalid_value = 255;
        let result = ItemType::from_db_int(invalid_value);
        assert!(result.is_err());
    }

    #[test]
    fn test_item_type_display() {
        let item_type = ItemType::DiskOrSetOfDisks;
        assert_eq!(item_type.to_string(), "Disk or Set of Disks");
    }
}
