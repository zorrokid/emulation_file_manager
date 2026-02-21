use core_types::item_type::ItemType;

#[derive(Clone, Debug, PartialEq)]
pub struct ReleaseItem {
    pub id: i64,
    pub release_id: i64,
    pub item_type: ItemType,
    pub notes: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct System {
    pub id: i64,
    pub name: String,
}
