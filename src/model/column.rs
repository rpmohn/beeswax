#[derive(Clone, Copy, PartialEq)]
pub enum DateDisplay { Date, Time, DateTime }

#[derive(Clone, Copy, PartialEq)]
pub enum Clock { Hr12, Hr24 }

#[derive(Clone, Copy, PartialEq)]
pub enum DateFmtCode { MMDDYY, DDMMYY, YYYYMMDD }

#[derive(Clone)]
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

pub struct Column {
    pub id:       usize,
    pub name:     String,   // "Column head" (category name)
    pub cat_id:   usize,    // ID of the backing category
    pub width:    usize,    // default 12
    pub date_fmt: Option<DateFmt>,  // Some only for Date-type backing category
}
