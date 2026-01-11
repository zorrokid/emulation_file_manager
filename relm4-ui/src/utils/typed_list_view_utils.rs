use relm4::{
    gtk,
    typed_view::list::{RelmListItem, TypedListView},
};

use crate::list_item::HasId;

pub fn get_item_ids<T>(list_view_wrapper: &TypedListView<T, gtk::SingleSelection>) -> Vec<i64>
where
    T: RelmListItem + HasId,
{
    (0..list_view_wrapper.len())
        .filter_map(|i| list_view_wrapper.get(i).map(|st| st.borrow().id()))
        .collect()
}

pub fn remove_selected<T>(list_view_wrapper: &mut TypedListView<T, gtk::SingleSelection>)
where
    T: RelmListItem + HasId,
{
    list_view_wrapper.remove(list_view_wrapper.selection_model.selected());
}

pub fn remove_by_id<T>(list_view_wrapper: &mut TypedListView<T, gtk::SingleSelection>, id: i64)
where
    T: RelmListItem + HasId,
{
    for i in 0..list_view_wrapper.len() {
        if let Some(list_item) = list_view_wrapper.get(i)
            && list_item.borrow().id() == id
        {
            list_view_wrapper.remove(i);
            break;
        }
    }
}

pub fn get_selected_item_id<T>(
    list_view_wrapper: &TypedListView<T, gtk::SingleSelection>,
) -> Option<i64>
where
    T: RelmListItem + HasId,
{
    list_view_wrapper
        .get(list_view_wrapper.selection_model.selected())
        .map(|st| st.borrow().id())
}
