use super::item::Item;

pub struct Section {
    pub id:    usize,
    pub name:  String,
    pub items: Vec<Item>,
}
