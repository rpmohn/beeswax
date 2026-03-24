#[derive(serde::Serialize, serde::Deserialize)]
pub struct Section {
    pub id:     usize,
    pub name:   String,
    pub cat_id: usize,   // backing category
}
