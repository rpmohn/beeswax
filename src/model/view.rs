use super::section::{Section, SortOrder};
use super::column::Column;

#[derive(Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub enum SectionSortMethod {
    #[default]
    None,
    CategoryOrder,
    Alphabetic,
    Numeric,
}

impl SectionSortMethod {
    pub fn label(self) -> &'static str {
        match self {
            Self::None          => "None",
            Self::CategoryOrder => "Category order",
            Self::Alphabetic    => "Alphabetic",
            Self::Numeric       => "Numeric",
        }
    }
    pub const ALL: [SectionSortMethod; 4] = [
        SectionSortMethod::None,
        SectionSortMethod::CategoryOrder,
        SectionSortMethod::Alphabetic,
        SectionSortMethod::Numeric,
    ];
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct View {
    pub id:         usize,
    pub name:       String,
    pub sections:   Vec<Section>,
    pub columns:    Vec<Column>,
    /// Number of columns in `columns[0..left_count]` rendered to the LEFT of the main column.
    pub left_count: usize,
    // Display flags (all default false = "No")
    #[serde(default)] pub hide_empty_sections:  bool,
    #[serde(default)] pub hide_done_items:       bool,
    #[serde(default)] pub hide_dependent_items:  bool,
    #[serde(default)] pub hide_inherited_items:  bool,
    #[serde(default)] pub hide_column_heads:     bool,
    #[serde(default)] pub section_separators:    bool,
    #[serde(default)] pub number_items:          bool,
    // Section ordering
    #[serde(default)] pub section_sort_method: SectionSortMethod,
    #[serde(default)] pub section_sort_order:  SortOrder,
}
