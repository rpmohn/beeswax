use std::collections::{HashMap, HashSet};

#[derive(Clone)]
pub struct Item {
    pub id:        usize,
    pub text:      String,
    pub values:    HashMap<usize, String>,  // cat_id → value string
    pub cond_cats: HashSet<usize>,          // cat_ids assigned conditionally (auto by system)
    pub note:      String,
    pub note_file: String,
}
