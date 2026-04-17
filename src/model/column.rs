#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DateDisplay { Date, Time, DateTime }

#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Clock { Hr12, Hr24 }

#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DateFmtCode { MMDDYY, DDMMYY, YYYYMMDD }

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct DateFmt {
    pub display:   DateDisplay,
    pub show_dow:  bool,
    pub clock:     Clock,
    pub code:      DateFmtCode,
    pub show_ampm: bool,
    pub date_sep:  char,
    pub time_sep:  char,
}

impl Default for DateFmt {
    fn default() -> Self {
        DateFmt {
            display:   DateDisplay::DateTime,
            show_dow:  false,
            clock:     Clock::Hr12,
            code:      DateFmtCode::MMDDYY,
            show_ampm: true,
            date_sep:  '/',
            time_sep:  ':',
        }
    }
}

/// Display format for a Standard-category column.
#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ColFormat {
    NameOnly,       // name of the assigned subcategory (default)
    ParentCategory, // "Parent:Name"
    Ancestor,       // immediate child of column head that is ancestor of assigned cat
    Star,           // "*" if assigned, blank if not
    YesNo,          // "Y" if assigned, "N" if not
    CategoryNote,   // one line from the category's note (notes not yet implemented)
}

impl Default for ColFormat {
    fn default() -> Self { ColFormat::NameOnly }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Column {
    pub id:       usize,
    pub name:     String,   // "Column head" (category name)
    pub cat_id:   usize,    // ID of the backing category
    pub width:    usize,    // default 12
    pub format:   ColFormat,         // display format for Standard columns
    pub date_fmt: Option<DateFmt>,   // Some only for Date-type backing category
}
