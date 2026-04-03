#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize, Default)]
pub enum SortOn {
    #[default]
    None,
    ItemText,
    Category,
    CategoryNote,
}

impl SortOn {
    pub fn label(self) -> &'static str {
        match self {
            SortOn::None         => "None",
            SortOn::ItemText     => "Item text",
            SortOn::Category     => "Category",
            SortOn::CategoryNote => "Category note",
        }
    }
    pub const ALL: [SortOn; 4] = [SortOn::None, SortOn::ItemText, SortOn::Category, SortOn::CategoryNote];
}

#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize, Default)]
pub enum SortNewItems {
    #[serde(rename = "NoAutoSort")]
    OnDemand,
    #[serde(rename = "OnAddingItem")]
    WhenEntered,
    #[default]
    OnLeavingSection,
}

impl SortNewItems {
    pub fn label(self) -> &'static str {
        match self {
            SortNewItems::OnDemand         => "On demand",
            SortNewItems::WhenEntered      => "When item is entered",
            SortNewItems::OnLeavingSection => "On leaving a section",
        }
    }
    pub const ALL: [SortNewItems; 3] = [
        SortNewItems::OnDemand, SortNewItems::WhenEntered, SortNewItems::OnLeavingSection,
    ];
}

#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize, Default)]
pub enum SortOrder {
    #[default]
    Ascending,
    Descending,
}

impl SortOrder {
    pub fn label(self) -> &'static str {
        match self {
            SortOrder::Ascending  => "Ascending",
            SortOrder::Descending => "Descending",
        }
    }
    pub const ALL: [SortOrder; 2] = [SortOrder::Ascending, SortOrder::Descending];
}

#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize, Default)]
pub enum SortSeq {
    #[default]
    CategoryHierarchy,
    Alphabetic,
    Numeric,
    Date,
}

impl SortSeq {
    pub fn label(self) -> &'static str {
        match self {
            SortSeq::CategoryHierarchy => "Category hierarchy",
            SortSeq::Alphabetic        => "Alphabetic",
            SortSeq::Numeric           => "Numeric",
            SortSeq::Date              => "Date",
        }
    }
    pub const ALL: [SortSeq; 4] = [SortSeq::CategoryHierarchy, SortSeq::Alphabetic, SortSeq::Numeric, SortSeq::Date];
}

#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize, Default)]
pub enum SortNa {
    #[default]
    Bottom,
    Top,
}

impl SortNa {
    pub fn label(self) -> &'static str {
        match self {
            SortNa::Bottom => "Bottom of section",
            SortNa::Top    => "Top of section",
        }
    }
    pub const ALL: [SortNa; 2] = [SortNa::Bottom, SortNa::Top];
}

#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum FilterOp { Include, Exclude }

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct FilterEntry {
    pub cat_id: usize,
    pub op:     FilterOp,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Section {
    pub id:     usize,
    pub name:   String,
    pub cat_id: usize,   // backing category
    #[serde(default)] pub sort_new:          SortNewItems,
    #[serde(default)] pub primary_on:        SortOn,
    #[serde(default)] pub primary_order:     SortOrder,
    #[serde(default)] pub primary_na:        SortNa,
    #[serde(default)] pub primary_cat_id:    Option<usize>,
    #[serde(default)] pub primary_seq:       SortSeq,
    #[serde(default)] pub secondary_on:      SortOn,
    #[serde(default)] pub secondary_order:   SortOrder,
    #[serde(default)] pub secondary_na:      SortNa,
    #[serde(default)] pub secondary_cat_id:  Option<usize>,
    #[serde(default)] pub secondary_seq:     SortSeq,
    #[serde(default)] pub filter:            Vec<FilterEntry>,
}
