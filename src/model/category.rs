#[derive(Clone, Copy, PartialEq)]
pub enum CategoryKind {
    Standard,
    Date,
    Numeric,
    Unindexed,
}

#[derive(Clone)]
pub struct Category {
    pub id:               usize,
    pub name:             String,
    pub kind:             CategoryKind,
    pub children:         Vec<Category>,
    pub note:             String,
    pub short_name:       String,
    pub also_match:       String,
    pub note_file:        String,
    pub excl_children:    bool,
    pub match_cat_name:   bool,
    pub match_short_name: bool,
}
