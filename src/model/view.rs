use super::section::Section;
use super::column::Column;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct View {
    pub id:         usize,
    pub name:       String,
    pub sections:   Vec<Section>,
    pub columns:    Vec<Column>,
    /// Number of columns in `columns[0..left_count]` rendered to the LEFT of the main column.
    pub left_count: usize,
}
