#[derive(Clone, Copy, PartialEq)]
pub enum CategoryKind {
    Standard,
    Date,
    Numeric,
    Unindexed,
}

#[derive(Clone)]
pub struct Category {
    pub id:       usize,
    pub name:     String,
    pub kind:     CategoryKind,
    pub children: Vec<Category>,
}
