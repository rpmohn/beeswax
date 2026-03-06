use std::collections::HashMap;

#[derive(Clone)]
pub struct Item {
    pub id:     usize,
    pub text:   String,
    pub values: HashMap<usize, String>,  // cat_id → value string
}
