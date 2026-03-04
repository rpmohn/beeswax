#[derive(Clone, Copy, PartialEq)]
pub enum CategoryKind {
    Standard,
    Date,
    Numeric,
    Unindexed,
}

pub struct Category {
    pub id:       usize,
    pub name:     String,
    pub kind:     CategoryKind,
    pub children: Vec<Category>,
}
