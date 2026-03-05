use crate::menu::{MenuAction, CATMGR_MENU, VIEW_MENU};
use crate::model::{Category, CategoryKind, Column, DateFmt, DateDisplay, Clock, DateFmtCode, Item, Section, View};
use std::collections::HashMap;

// ── F-key modifier state ──────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum FKeyMod { Normal, Shift, Ctrl, Alt }

// ── Screen ────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum AppScreen {
    View,
    CatMgr,
}

// ── View-mode state ───────────────────────────────────────────────────────────

pub enum CursorPos {
    SectionHead(usize),
    Item { section: usize, item: usize },
}

pub enum Mode {
    Normal,
    Edit   { original: String, buffer: String, cursor: usize, col: usize },
    Create { buffer: String, cursor: usize },
}

// ── CatMgr state ──────────────────────────────────────────────────────────────

pub enum CatMode {
    Normal,
    Edit   { buffer: String, cursor: usize },
    /// `as_child`: insert below as child (Alt-R) vs sibling (INS)
    Create { buffer: String, cursor: usize, as_child: bool },
}

pub struct CatMgrState {
    pub cursor: usize,
    pub mode:   CatMode,
}

// ── Column state ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum ColFormField { Head, Width, Position }

#[derive(Clone, Copy, PartialEq)]
pub enum PropsField {
    Head, Width,
    DateDisplay, ShowDow, Clock, DateFmtCode, ShowAmPm, DateSep, TimeSep,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ColPos { Right, Left }

#[derive(Clone, Copy, PartialEq)]
pub enum ChoicesKind { Category, Position }

#[derive(Clone, Copy, PartialEq)]
pub enum TimeField { Hour, Min, Sec }

pub enum ColMode {
    Normal,
    Form {
        is_add:       bool,
        head_cat_idx: Option<usize>,  // None = blank; Some = index into flatten_cats
        width_buf:    String,
        width_cur:    usize,
        position:     ColPos,
        active_field: ColFormField,
    },
    /// F3 Choices picker overlaid on the form.
    Choices {
        is_add:        bool,
        head_cat_idx:  Option<usize>,
        width_buf:     String,
        width_cur:     usize,
        position:      ColPos,
        active_field:  ColFormField,
        picker_cursor: usize,
        kind:          ChoicesKind,
    },
    Move,
    /// Alt-R / Alt-L quick-add: category picker that inserts a column immediately.
    QuickAdd {
        position:      ColPos,
        picker_cursor: usize,
    },
    Props {
        head_buf:     String,
        head_cur:     usize,
        width_buf:    String,
        width_cur:    usize,
        date_fmt:     Option<DateFmt>,
        active_field: PropsField,
        is_date:      bool,
    },
    /// F3 Calendar date picker for date-type columns.
    Calendar {
        year:  i32,
        month: u32,
        day:   u32,
        hour:  u32,
        min:   u32,
        sec:   u32,
    },
    /// F6 SetTime sub-modal opened from Calendar.
    SetTime {
        year:      i32,
        month:     u32,
        day:       u32,
        hour_buf:  String,
        min_buf:   String,
        sec_buf:   String,
        active:    TimeField,
        orig_hour: u32,
        orig_min:  u32,
        orig_sec:  u32,
    },
}

// ── Menu state ────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum MenuState {
    Closed,
    Top    { cursor: usize },
    Sub    { top: usize, cursor: usize },
    SubSub { top: usize, sub: usize, cursor: usize },
}

/// One entry in the depth-first flattened category list used for display/navigation.
pub struct FlatCat {
    pub depth: usize,
    pub path:  Vec<usize>,   // indices through nested children vecs
    pub id:    usize,
    pub name:  String,
    pub kind:  CategoryKind,
}

// ── App ───────────────────────────────────────────────────────────────────────

pub struct App {
    pub screen:      AppScreen,
    // View
    pub view:        View,
    pub cursor:      CursorPos,
    pub mode:        Mode,
    // CatMgr
    pub categories:  Vec<Category>,
    pub cat_state:   CatMgrState,
    // Column
    pub col_cursor:  usize,
    pub col_mode:    ColMode,
    // Menu
    pub menu:        MenuState,
    // F-key bar
    pub fkey_mod:    FKeyMod,
    // Misc
    pub quit:        bool,
    next_id:         usize,
}

// ── Byte-offset helper ────────────────────────────────────────────────────────

fn char_to_byte(s: &str, n: usize) -> usize {
    s.char_indices().nth(n).map(|(b, _)| b).unwrap_or(s.len())
}

// ── Tree helpers (free functions) ─────────────────────────────────────────────

pub fn flatten_cats(cats: &[Category]) -> Vec<FlatCat> {
    let mut out = Vec::new();
    flatten_inner(cats, 0, &[], &mut out);
    out
}

fn flatten_inner(cats: &[Category], depth: usize, prefix: &[usize], out: &mut Vec<FlatCat>) {
    for (i, cat) in cats.iter().enumerate() {
        let mut path = prefix.to_vec();
        path.push(i);
        out.push(FlatCat { depth, path: path.clone(), id: cat.id, name: cat.name.clone(), kind: cat.kind });
        flatten_inner(&cat.children, depth + 1, &path, out);
    }
}

/// Remove and return the category at `path`.
fn take_cat(cats: &mut Vec<Category>, path: &[usize]) -> Category {
    let (&head, tail) = path.split_first().expect("empty path");
    if tail.is_empty() {
        cats.remove(head)
    } else {
        take_cat(&mut cats[head].children, tail)
    }
}

/// Insert `cat` as a child of the node at `parent_path`, at position `idx`.
/// An empty `parent_path` inserts into the top-level vec.
fn insert_at(cats: &mut Vec<Category>, parent_path: &[usize], idx: usize, cat: Category) {
    if parent_path.is_empty() {
        cats.insert(idx, cat);
        return;
    }
    let (&head, tail) = parent_path.split_first().unwrap();
    if tail.is_empty() {
        cats[head].children.insert(idx, cat);
    } else {
        insert_at(&mut cats[head].children, tail, idx, cat);
    }
}

/// Number of direct children of the node at `path`.
fn children_count(cats: &[Category], path: &[usize]) -> usize {
    let (&head, tail) = path.split_first().expect("empty path");
    if tail.is_empty() {
        cats[head].children.len()
    } else {
        children_count(&cats[head].children, tail)
    }
}

fn rename_cat(cats: &mut Vec<Category>, path: &[usize], name: String) {
    let (&head, tail) = path.split_first().expect("empty path");
    if tail.is_empty() {
        cats[head].name = name;
    } else {
        rename_cat(&mut cats[head].children, tail, name);
    }
}

// ── Datetime helpers ──────────────────────────────────────────────────────────

fn now_datetime_string() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days   = (secs / 86400) as i64;
    let rem    = secs % 86400;
    let hour   = (rem / 3600) as u32;
    let minute = ((rem % 3600) / 60) as u32;
    let second = (rem % 60) as u32;
    let (year, month, day) = civil_from_days(days);
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", year, month, day, hour, minute, second)
}

/// Convert days-since-Unix-epoch to (year, month, day).
/// Algorithm: https://howardhinnant.github.io/date_algorithms.html
fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z   = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y   = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp  = (5 * doy + 2) / 153;
    let d   = doy - (153 * mp + 2) / 5 + 1;
    let m   = if mp < 10 { mp + 3 } else { mp - 9 };
    let y   = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

/// Parse `YYYY-MM-DD HH:MM:SS`, `YYYY-MM-DD HH:MM`, or `YYYY-MM-DD`.
/// Returns `(year, month, day, hour, min, sec)` or `None` if invalid.
pub fn parse_datetime(s: &str) -> Option<(i32, u32, u32, u32, u32, u32)> {
    let s = s.trim();
    let parts: Vec<&str> = s.splitn(2, ' ').collect();
    let date_nums: Vec<&str> = parts[0].split('-').collect();
    if date_nums.len() != 3 { return None; }
    let year  = date_nums[0].parse::<i32>().ok()?;
    let month = date_nums[1].parse::<u32>().ok()?;
    let day   = date_nums[2].parse::<u32>().ok()?;
    if !(1..=12).contains(&month) { return None; }
    if !(1..=31).contains(&day)   { return None; }
    let (hour, min, sec) = if parts.len() == 2 {
        let tnums: Vec<&str> = parts[1].split(':').collect();
        let h = tnums.first().and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
        let m = tnums.get(1).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
        let s = tnums.get(2).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
        if h > 23 || m > 59 || s > 59 { return None; }
        (h, m, s)
    } else {
        (0, 0, 0)
    };
    Some((year, month, day, hour, min, sec))
}

/// Parse a time string such as "2300", "11pm", "11:30pm", "23:00", "9:30".
/// Returns (hour, min, sec) or None if unparseable/out-of-range.
fn parse_time_str(s: &str) -> Option<(u32, u32, u32)> {
    let s = s.trim();
    if s.is_empty() { return None; }
    let sl = s.to_ascii_lowercase();
    // Strip am/pm suffix
    let (core, is_pm, is_am) =
        if sl.ends_with("pm") { (&s[..s.len()-2], true,  false) }
        else if sl.ends_with("am") { (&s[..s.len()-2], false, true)  }
        else if sl.ends_with('p')  { (&s[..s.len()-1], true,  false) }
        else if sl.ends_with('a')  { (&s[..s.len()-1], false, true)  }
        else                       { (s,               false, false) };
    let core = core.trim();
    if core.is_empty() { return None; }
    let (hour, min, sec) = if core.contains(':') {
        let parts: Vec<&str> = core.splitn(3, ':').collect();
        let h = parts[0].trim().parse::<u32>().ok()?;
        let m = parts.get(1).and_then(|x| x.trim().parse::<u32>().ok()).unwrap_or(0);
        let s = parts.get(2).and_then(|x| x.trim().parse::<u32>().ok()).unwrap_or(0);
        (h, m, s)
    } else {
        // Pure digits: 1–2 = HH, 3 = H:MM, 4 = HH:MM
        let digits: String = core.chars().filter(|c| c.is_ascii_digit()).collect();
        match digits.len() {
            1 | 2 => (digits.parse::<u32>().ok()?, 0, 0),
            3     => (digits[..1].parse::<u32>().ok()?, digits[1..].parse::<u32>().ok()?, 0),
            4     => (digits[..2].parse::<u32>().ok()?, digits[2..].parse::<u32>().ok()?, 0),
            _     => return None,
        }
    };
    let hour = if is_pm {
        if hour == 12 { 12 } else if hour < 12 { hour + 12 } else { return None }
    } else if is_am {
        if hour == 12 { 0  } else if hour <= 12 { hour }     else { return None }
    } else { hour };
    if hour > 23 || min > 59 || sec > 59 { return None; }
    Some((hour, min, sec))
}

/// Expand a 1–2 digit year to 4 digits (< 70 → 2000+, ≥ 70 → 1900+).
fn parse_year_str(s: &str) -> Option<i32> {
    let s = s.trim();
    let y = s.parse::<i32>().ok()?;
    Some(if s.len() <= 2 { if y < 70 { 2000 + y } else { 1900 + y } } else { y })
}

/// Parse a date string that uses `/`, `.`, or `-` as a separator.
/// Uses `fmt_code` to determine MM/DD vs DD/MM field order.
fn parse_date_fields(s: &str, fmt_code: DateFmtCode, default_year: i32) -> Option<(i32, u32, u32)> {
    let sep = if s.contains('/') { '/' }
              else if s.contains('.') { '.' }
              else if s.contains('-') { '-' }
              else { return None; };
    let parts: Vec<&str> = s.splitn(3, sep).collect();
    let (a, b, year) = match parts.len() {
        2 => (parts[0].trim().parse::<u32>().ok()?,
              parts[1].trim().parse::<u32>().ok()?,
              default_year),
        3 => (parts[0].trim().parse::<u32>().ok()?,
              parts[1].trim().parse::<u32>().ok()?,
              parse_year_str(parts[2])?),
        _ => return None,
    };
    let (month, day) = match fmt_code {
        DateFmtCode::DDMMYY => (b, a),
        _                   => (a, b),
    };
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) { return None; }
    Some((year, month, day))
}

/// Smart date/time input parser.
///
/// Accepts full ISO (`YYYY-MM-DD HH:MM:SS`), partial dates (`M/D`, `M/D/YY`),
/// pure times (`HHMM`, `NNpm`, `HH:MM`), and combinations (`4/2 11pm`).
/// `fmt_code` controls MM/DD vs DD/MM ordering for two-field date inputs.
pub fn parse_date_input(s: &str, fmt_code: DateFmtCode) -> Option<(i32, u32, u32, u32, u32, u32)> {
    let s = s.trim();
    if s.is_empty() { return None; }
    // Full ISO datetime/date takes priority
    if let Some(r) = parse_datetime(s) { return Some(r); }
    let (today_y, today_m, today_d) = today();
    // Split on first space → potential "date time" pair
    let (date_part, time_part) = match s.find(' ') {
        Some(pos) => { let (d, t) = s.split_at(pos); (d.trim(), Some(t.trim())) }
        None      => (s, None),
    };
    let has_slash_dot = date_part.contains('/') || date_part.contains('.');
    let has_dash      = date_part.contains('-');
    if !has_slash_dot && !has_dash && time_part.is_none() {
        // No date separators → treat entirely as a time, use today's date
        let (h, m, sec) = parse_time_str(date_part)?;
        return Some((today_y, today_m, today_d, h, m, sec));
    }
    if !has_slash_dot && has_dash && time_part.is_none() {
        // Dash-only: try as M-D date
        if let Some((y, mo, d)) = parse_date_fields(date_part, fmt_code, today_y) {
            return Some((y, mo, d, 0, 0, 0));
        }
        // Fall back to treating it as a time
        let (h, m, sec) = parse_time_str(date_part)?;
        return Some((today_y, today_m, today_d, h, m, sec));
    }
    // Slash or dot → definite date part
    let (year, month, day) = parse_date_fields(date_part, fmt_code, today_y)?;
    let (h, m, sec) = time_part.and_then(parse_time_str).unwrap_or((0, 0, 0));
    Some((year, month, day, h, m, sec))
}

/// Format a stored `YYYY-MM-DD HH:MM:SS` string according to a `DateFmt`.
/// Returns `stored.to_string()` if the value cannot be parsed.
pub fn format_date_value(stored: &str, fmt: &DateFmt) -> String {
    let Some((year, month, day, hour, min, _sec)) = parse_datetime(stored) else {
        return stored.to_string();
    };
    let ds = fmt.date_sep;
    let ts = fmt.time_sep;
    let date_str = match fmt.code {
        DateFmtCode::MMDDYY   => format!("{:02}{}{:02}{}{:02}", month, ds, day,   ds, year % 100),
        DateFmtCode::DDMMYY   => format!("{:02}{}{:02}{}{:02}", day,   ds, month, ds, year % 100),
        DateFmtCode::YYYYMMDD => format!("{:04}{}{:02}{}{:02}", year,  ds, month, ds, day),
    };
    let time_str = match fmt.clock {
        Clock::Hr24 => format!("{:02}{}{:02}", hour, ts, min),
        Clock::Hr12 => {
            let h12  = if hour == 0 { 12 } else if hour > 12 { hour - 12 } else { hour };
            let ampm = if hour < 12 { "am" } else { "pm" };
            if fmt.show_ampm {
                format!("{}{}{:02}{}", h12, ts, min, ampm)
            } else {
                format!("{}{}{:02}", h12, ts, min)
            }
        }
    };
    match fmt.display {
        DateDisplay::Date     => date_str,
        DateDisplay::Time     => time_str,
        DateDisplay::DateTime => format!("{} {}", date_str, time_str),
    }
}

fn cycle_date_sep_next(c: char) -> char {
    match c { '/' => '-', '-' => '.', '.' => ' ', _ => '/' }
}

fn cycle_date_sep_prev(c: char) -> char {
    match c { '/' => ' ', '-' => '/', '.' => '-', ' ' => '.', _ => '/' }
}

fn today() -> (i32, u32, u32) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    civil_from_days((secs / 86400) as i64)
}

pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11               => 30,
        2 => if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) { 29 } else { 28 },
        _ => 30,
    }
}

// ── App impl ──────────────────────────────────────────────────────────────────

impl App {
    pub fn new() -> Self {
        let section = Section {
            id:    1,
            name:  "Initial Section".to_string(),
            items: Vec::new(),
        };
        let view = View {
            id:         1,
            name:       "Initial View".to_string(),
            sections:   vec![section],
            columns:    Vec::new(),
            left_count: 0,
        };

        fn date(id: usize, name: &str) -> Category {
            Category { id, name: name.to_string(), kind: CategoryKind::Date, children: vec![] }
        }
        fn std(id: usize, name: &str) -> Category {
            Category { id, name: name.to_string(), kind: CategoryKind::Standard, children: vec![] }
        }

        let main_cat = Category {
            id:       2,
            name:     "MAIN".to_string(),
            kind:     CategoryKind::Standard,
            children: vec![
                date(3, "Entry"),
                date(4, "When"),
                date(5, "Done"),
                std(6, "Initial Section"),
            ],
        };

        App {
            screen:     AppScreen::View,
            view,
            cursor:     CursorPos::SectionHead(0),
            mode:       Mode::Normal,
            categories: vec![main_cat],
            cat_state:  CatMgrState { cursor: 0, mode: CatMode::Normal },
            col_cursor: 0,
            col_mode:   ColMode::Normal,
            menu:       MenuState::Closed,
            fkey_mod:   FKeyMod::Normal,
            quit:       false,
            next_id:    7,
        }
    }

    fn alloc_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    // ── Screen toggle ─────────────────────────────────────────────────────────

    pub fn toggle_catmgr(&mut self) {
        self.cat_state.mode = CatMode::Normal;
        self.mode            = Mode::Normal;
        self.screen = match self.screen {
            AppScreen::View   => AppScreen::CatMgr,
            AppScreen::CatMgr => AppScreen::View,
        };
    }

    // ── Menu ──────────────────────────────────────────────────────────────────

    pub fn open_menu(&mut self) {
        // Cancel any in-progress edit before opening the menu
        self.mode          = Mode::Normal;
        self.cat_state.mode = CatMode::Normal;
        self.menu = MenuState::Top { cursor: 0 };
    }

    fn current_menu_items(&self) -> &'static [crate::menu::TopItem] {
        match self.screen {
            AppScreen::View   => VIEW_MENU,
            AppScreen::CatMgr => CATMGR_MENU,
        }
    }

    fn apply_menu_action(&mut self, action: MenuAction) {
        match action {
            MenuAction::Quit         => { self.quit = true; }
            MenuAction::ReturnToView => {
                if matches!(self.screen, AppScreen::CatMgr) { self.toggle_catmgr(); }
            }
            MenuAction::ColumnAdd        => self.col_open_form(true,  ColFormField::Head),
            MenuAction::ColumnProperties => self.col_open_form(false, ColFormField::Head),
            MenuAction::ColumnWidth      => self.col_open_form(false, ColFormField::Width),
            MenuAction::ColumnRemove     => self.col_delete(),
            MenuAction::ColumnMove       => self.col_begin_move(),
            MenuAction::Noop => {}
        }
        self.menu = MenuState::Closed;
    }

    pub fn menu_left(&mut self) {
        let items = self.current_menu_items();
        let new_menu = match self.menu {
            MenuState::Top { cursor } => {
                let max = items.len().saturating_sub(1);
                MenuState::Top { cursor: if cursor == 0 { max } else { cursor - 1 } }
            }
            MenuState::Sub { top, cursor } => {
                let max = items[top].sub.len().saturating_sub(1);
                MenuState::Sub { top, cursor: if cursor == 0 { max } else { cursor - 1 } }
            }
            MenuState::SubSub { top, sub, cursor } => {
                let children = items[top].sub[sub].children.unwrap_or(&[]);
                let max = children.len().saturating_sub(1);
                MenuState::SubSub { top, sub, cursor: if cursor == 0 { max } else { cursor - 1 } }
            }
            MenuState::Closed => MenuState::Closed,
        };
        self.menu = new_menu;
    }

    pub fn menu_right(&mut self) {
        let items = self.current_menu_items();
        let new_menu = match self.menu {
            MenuState::Top { cursor } => {
                let max = items.len().saturating_sub(1);
                MenuState::Top { cursor: if cursor >= max { 0 } else { cursor + 1 } }
            }
            MenuState::Sub { top, cursor } => {
                let max = items[top].sub.len().saturating_sub(1);
                MenuState::Sub { top, cursor: if cursor >= max { 0 } else { cursor + 1 } }
            }
            MenuState::SubSub { top, sub, cursor } => {
                let children = items[top].sub[sub].children.unwrap_or(&[]);
                let max = children.len().saturating_sub(1);
                MenuState::SubSub { top, sub, cursor: if cursor >= max { 0 } else { cursor + 1 } }
            }
            MenuState::Closed => MenuState::Closed,
        };
        self.menu = new_menu;
    }

    pub fn menu_enter(&mut self) {
        match self.menu {
            MenuState::Top { cursor } => {
                self.menu = MenuState::Sub { top: cursor, cursor: 0 };
            }
            MenuState::Sub { top, cursor } => {
                let item = &self.current_menu_items()[top].sub[cursor];
                if item.children.is_some() {
                    self.menu = MenuState::SubSub { top, sub: cursor, cursor: 0 };
                } else {
                    let action = item.action;
                    self.apply_menu_action(action);
                }
            }
            MenuState::SubSub { top, sub, cursor } => {
                let items = self.current_menu_items();
                let children = items[top].sub[sub].children.unwrap_or(&[]);
                if cursor < children.len() {
                    let action = children[cursor].action;
                    self.apply_menu_action(action);
                }
            }
            MenuState::Closed => {}
        }
    }

    pub fn menu_esc(&mut self) {
        self.menu = match self.menu {
            MenuState::SubSub { top, sub, .. } => MenuState::Sub { top, cursor: sub },
            MenuState::Sub { top, .. }         => MenuState::Top { cursor: top },
            MenuState::Top { .. }              => MenuState::Closed,
            MenuState::Closed                  => MenuState::Closed,
        };
    }

    pub fn menu_char(&mut self, ch: char) {
        let ch_up = ch.to_ascii_uppercase();
        match self.menu {
            MenuState::Top { .. } => {
                let pos = self.current_menu_items().iter().position(|t| {
                    t.label.chars().next().map(|c| c.to_ascii_uppercase()) == Some(ch_up)
                });
                if let Some(i) = pos {
                    self.menu = MenuState::Sub { top: i, cursor: 0 };
                }
            }
            MenuState::Sub { top, .. } => {
                let items = self.current_menu_items();
                let pos = items[top].sub.iter().position(|s| {
                    s.label.chars().next().map(|c| c.to_ascii_uppercase()) == Some(ch_up)
                });
                if let Some(i) = pos {
                    let item = &items[top].sub[i];
                    if item.children.is_some() {
                        self.menu = MenuState::SubSub { top, sub: i, cursor: 0 };
                    } else {
                        let action = item.action;
                        self.apply_menu_action(action);
                    }
                }
            }
            MenuState::SubSub { top, sub, .. } => {
                let items = self.current_menu_items();
                let children = items[top].sub[sub].children.unwrap_or(&[]);
                let pos = children.iter().position(|s| {
                    s.label.chars().next().map(|c| c.to_ascii_uppercase()) == Some(ch_up)
                });
                if let Some(i) = pos {
                    let action = children[i].action;
                    self.apply_menu_action(action);
                }
            }
            MenuState::Closed => {}
        }
    }

    // ── View navigation ───────────────────────────────────────────────────────

    pub fn cursor_up(&mut self) {
        if !matches!(self.mode, Mode::Normal) { return; }
        self.cursor = match &self.cursor {
            CursorPos::SectionHead(0) => CursorPos::SectionHead(0),
            CursorPos::SectionHead(s) => {
                let prev = s - 1;
                if self.view.sections[prev].items.is_empty() {
                    CursorPos::SectionHead(prev)
                } else {
                    let last = self.view.sections[prev].items.len() - 1;
                    CursorPos::Item { section: prev, item: last }
                }
            }
            CursorPos::Item { section, item: 0 } => CursorPos::SectionHead(*section),
            CursorPos::Item { section, item }    => {
                CursorPos::Item { section: *section, item: item - 1 }
            }
        };
    }

    pub fn cursor_down(&mut self) {
        if !matches!(self.mode, Mode::Normal) { return; }
        let num_sections = self.view.sections.len();
        self.cursor = match &self.cursor {
            CursorPos::SectionHead(s) => {
                let s = *s;
                if self.view.sections[s].items.is_empty() {
                    if s + 1 < num_sections { CursorPos::SectionHead(s + 1) }
                    else                    { CursorPos::SectionHead(s) }
                } else {
                    CursorPos::Item { section: s, item: 0 }
                }
            }
            CursorPos::Item { section, item } => {
                let s = *section;
                let i = *item;
                let num_items = self.view.sections[s].items.len();
                if i + 1 < num_items {
                    CursorPos::Item { section: s, item: i + 1 }
                } else if s + 1 < num_sections {
                    CursorPos::SectionHead(s + 1)
                } else {
                    CursorPos::Item { section: s, item: i }
                }
            }
        };
    }

    // ── View buffer cursor ────────────────────────────────────────────────────

    pub fn edit_cursor_left(&mut self) {
        match &mut self.mode {
            Mode::Edit { cursor, .. } | Mode::Create { cursor, .. } => {
                if *cursor > 0 { *cursor -= 1; }
            }
            Mode::Normal => {}
        }
    }

    pub fn edit_cursor_right(&mut self) {
        match &mut self.mode {
            Mode::Edit { buffer, cursor, .. } | Mode::Create { buffer, cursor } => {
                let len = buffer.chars().count();
                if *cursor < len { *cursor += 1; }
            }
            Mode::Normal => {}
        }
    }

    // ── View mode transitions ─────────────────────────────────────────────────

    pub fn begin_create(&mut self, first_char: char) {
        if !matches!(self.mode, Mode::Normal) { return; }
        self.mode = Mode::Create { buffer: first_char.to_string(), cursor: 1 };
    }

    /// Typing a printable character in Normal mode:
    /// - On a non-main column item cell → begin editing that cell with `ch` as the first character.
    /// - Otherwise → begin creating a new item (existing behaviour).
    pub fn begin_char_input(&mut self, ch: char) {
        if !matches!(self.mode, Mode::Normal) { return; }
        if self.col_cursor > 0 {
            if let CursorPos::Item { section, item } = &self.cursor {
                let col     = self.col_cursor;
                let col_idx = col - 1;
                if col_idx < self.view.columns.len() {
                    let cat_id   = self.view.columns[col_idx].cat_id;
                    let original = self.view.sections[*section].items[*item]
                        .values.get(&cat_id).cloned().unwrap_or_default();
                    self.mode = Mode::Edit {
                        original,
                        buffer: ch.to_string(),
                        cursor: 1,
                        col,
                    };
                    return;
                }
            }
        }
        // Default: start creating a new main-column item
        self.mode = Mode::Create { buffer: ch.to_string(), cursor: 1 };
    }

    pub fn begin_create_blank(&mut self) {
        if !matches!(self.mode, Mode::Normal) { return; }
        self.mode = Mode::Create { buffer: String::new(), cursor: 0 };
    }

    pub fn begin_edit(&mut self) {
        if !matches!(self.mode, Mode::Normal) { return; }
        let col = self.col_cursor;
        let (original, can_edit) = if col == 0 {
            let orig = match &self.cursor {
                CursorPos::SectionHead(s)         => self.view.sections[*s].name.clone(),
                CursorPos::Item { section, item } => self.view.sections[*section].items[*item].text.clone(),
            };
            (orig, true)
        } else {
            if col - 1 >= self.view.columns.len() {
                return;
            }
            match &self.cursor {
                CursorPos::SectionHead(_) => {
                    // Edit the column display name
                    (self.view.columns[col - 1].name.clone(), true)
                }
                CursorPos::Item { section, item } => {
                    let cat_id = self.view.columns[col - 1].cat_id;
                    let val = self.view.sections[*section].items[*item]
                        .values.get(&cat_id).cloned().unwrap_or_default();
                    (val, true)
                }
            }
        };
        if !can_edit { return; }
        self.mode = Mode::Edit { original: original.clone(), buffer: original, cursor: 0, col };
    }

    // ── View input ────────────────────────────────────────────────────────────

    pub fn input_char(&mut self, ch: char) {
        let (buffer, cursor) = match &mut self.mode {
            Mode::Edit   { buffer, cursor, .. } => (buffer, cursor),
            Mode::Create { buffer, cursor }     => (buffer, cursor),
            Mode::Normal => return,
        };
        let byte_pos = char_to_byte(buffer, *cursor);
        buffer.insert(byte_pos, ch);
        *cursor += 1;
    }

    pub fn input_backspace(&mut self) {
        let (buffer, cursor) = match &mut self.mode {
            Mode::Edit   { buffer, cursor, .. } => (buffer, cursor),
            Mode::Create { buffer, cursor }     => (buffer, cursor),
            Mode::Normal => return,
        };
        if *cursor > 0 {
            *cursor -= 1;
            let byte_pos = char_to_byte(buffer, *cursor);
            buffer.remove(byte_pos);
        }
    }

    pub fn input_delete(&mut self) {
        let (buffer, cursor) = match &mut self.mode {
            Mode::Edit   { buffer, cursor, .. } => (buffer, cursor),
            Mode::Create { buffer, cursor }     => (buffer, cursor),
            Mode::Normal => return,
        };
        let len = buffer.chars().count();
        if *cursor < len {
            let byte_pos = char_to_byte(buffer, *cursor);
            buffer.remove(byte_pos);
        }
    }

    pub fn confirm(&mut self) {
        // Date validation: peek at mode before consuming it
        if let Mode::Edit { ref buffer, col, .. } = self.mode {
            if col > 0 {
                let col_idx = col - 1;
                if col_idx < self.view.columns.len()
                    && self.view.columns[col_idx].date_fmt.is_some()
                    && matches!(self.cursor, CursorPos::Item { .. })
                {
                    let trimmed  = buffer.trim();
                    let fmt_code = self.view.columns[col_idx].date_fmt.as_ref()
                        .map(|f| f.code).unwrap_or(DateFmtCode::MMDDYY);
                    if !trimmed.is_empty() && parse_date_input(trimmed, fmt_code).is_none() {
                        return; // Stay in edit mode — invalid date
                    }
                }
            }
        }
        match std::mem::replace(&mut self.mode, Mode::Normal) {
            Mode::Create { buffer, .. } => {
                let text = buffer.trim().to_string();
                if text.is_empty() { return; }
                let id = self.alloc_id();
                let (sec_idx, insert_after) = match &self.cursor {
                    CursorPos::SectionHead(s)         => (*s, None),
                    CursorPos::Item { section, item } => (*section, Some(*item)),
                };
                let pos = insert_after.map(|i| i + 1).unwrap_or(0);
                // Auto-stamp the Entry category with the creation datetime.
                let mut values = HashMap::new();
                let flat = flatten_cats(&self.categories);
                if let Some(entry) = flat.iter().find(|c| c.name == "Entry") {
                    values.insert(entry.id, now_datetime_string());
                }
                self.view.sections[sec_idx].items.insert(pos, Item { id, text, values });
                self.cursor = CursorPos::Item { section: sec_idx, item: pos };
            }
            Mode::Edit { buffer, col, .. } => {
                let text = buffer.trim().to_string();
                if col == 0 {
                    if text.is_empty() { return; }
                    match &self.cursor {
                        CursorPos::SectionHead(s) => { self.view.sections[*s].name = text; }
                        CursorPos::Item { section, item } => {
                            self.view.sections[*section].items[*item].text = text;
                        }
                    }
                } else if col - 1 < self.view.columns.len() {
                    match &self.cursor {
                        CursorPos::SectionHead(_) => {
                            if !text.is_empty() {
                                let cat_id = self.view.columns[col - 1].cat_id;
                                self.view.columns[col - 1].name = text.clone();
                                let flat = flatten_cats(&self.categories);
                                if let Some(e) = flat.iter().find(|c| c.id == cat_id) {
                                    let path = e.path.clone();
                                    rename_cat(&mut self.categories, &path, text);
                                }
                            }
                        }
                        CursorPos::Item { section, item } => {
                            let col_idx  = col - 1;
                            let cat_id   = self.view.columns[col_idx].cat_id;
                            let is_date  = self.view.columns[col_idx].date_fmt.is_some();
                            let fmt_code = self.view.columns[col_idx].date_fmt.as_ref()
                                .map(|f| f.code).unwrap_or(DateFmtCode::MMDDYY);
                            let (s, i)   = (*section, *item);
                            // Normalize date values to YYYY-MM-DD HH:MM:SS
                            let final_text = if is_date && !text.is_empty() {
                                if let Some((y, mo, d, h, mi, sec)) = parse_date_input(&text, fmt_code) {
                                    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", y, mo, d, h, mi, sec)
                                } else {
                                    text.clone()
                                }
                            } else {
                                text.clone()
                            };
                            if final_text.is_empty() {
                                self.view.sections[s].items[i].values.remove(&cat_id);
                            } else {
                                self.view.sections[s].items[i].values.insert(cat_id, final_text);
                            }
                        }
                    }
                }
            }
            Mode::Normal => {}
        }
    }

    pub fn cancel(&mut self) {
        self.mode = Mode::Normal;
    }

    // ── Column management ─────────────────────────────────────────────────────

    pub fn col_open_form(&mut self, is_add: bool, focus: ColFormField) {
        let flat = flatten_cats(&self.categories);
        // col_cursor: 0 = items column, 1..n = added columns 0..n-1
        let cur_idx = if self.col_cursor > 0 && !self.view.columns.is_empty() {
            Some((self.col_cursor - 1).min(self.view.columns.len() - 1))
        } else {
            None
        };
        // Add: head starts blank. Properties/Width: pre-fill from current column.
        let head_cat_idx = if is_add || cur_idx.is_none() {
            None
        } else {
            let col_name = &self.view.columns[cur_idx.unwrap()].name;
            flat.iter().position(|c| &c.name == col_name)
        };
        let width_buf = if is_add || cur_idx.is_none() {
            "12".to_string()
        } else {
            self.view.columns[cur_idx.unwrap()].width.to_string()
        };
        let width_cur = width_buf.chars().count();
        self.col_mode = ColMode::Form {
            is_add,
            head_cat_idx,
            width_cur,
            width_buf,
            position:     ColPos::Right,
            active_field: focus,
        };
    }

    pub fn col_delete(&mut self) {
        if self.view.columns.is_empty() || self.col_cursor == 0 { return; }
        let idx = (self.col_cursor - 1).min(self.view.columns.len() - 1);
        self.view.columns.remove(idx);
        if idx < self.view.left_count {
            self.view.left_count -= 1;
        }
        let new_len = self.view.columns.len();
        self.col_cursor = if new_len == 0 { 0 } else { self.col_cursor.min(new_len) };
    }

    pub fn col_begin_move(&mut self) {
        if self.col_cursor > 0 && !self.view.columns.is_empty() {
            self.col_mode = ColMode::Move;
        }
    }

    // ── Quick-add column (Alt-R / Alt-L) ─────────────────────────────────────

    pub fn col_quick_add(&mut self, position: ColPos) {
        if !matches!(self.mode, Mode::Normal) { return; }
        let flat = flatten_cats(&self.categories);
        if flat.is_empty() { return; }
        self.col_mode = ColMode::QuickAdd { position, picker_cursor: 0 };
    }

    pub fn col_quick_add_up(&mut self) {
        if let ColMode::QuickAdd { picker_cursor, .. } = &mut self.col_mode {
            if *picker_cursor > 0 { *picker_cursor -= 1; }
        }
    }

    pub fn col_quick_add_down(&mut self) {
        let len = flatten_cats(&self.categories).len();
        if let ColMode::QuickAdd { picker_cursor, .. } = &mut self.col_mode {
            if *picker_cursor + 1 < len { *picker_cursor += 1; }
        }
    }

    pub fn col_quick_add_confirm(&mut self) {
        let (position, picker_cursor) = match &self.col_mode {
            ColMode::QuickAdd { position, picker_cursor } => (*position, *picker_cursor),
            _ => return,
        };
        self.col_mode = ColMode::Normal;
        let flat = flatten_cats(&self.categories);
        let Some(entry) = flat.get(picker_cursor) else { return };
        let name   = entry.name.clone();
        let cat_id = entry.id;
        let kind   = entry.kind;
        let width  = 12usize;
        let id     = self.alloc_id();
        let lc     = self.view.left_count;
        let (pos, new_lc) = if self.view.columns.is_empty() {
            (0, if position == ColPos::Left { 1 } else { 0 })
        } else if self.col_cursor == 0 {
            match position {
                ColPos::Left  => (lc, lc + 1),
                ColPos::Right => (lc, lc),
            }
        } else {
            let cur    = (self.col_cursor - 1).min(self.view.columns.len() - 1);
            let in_left = cur < lc;
            let p = match position { ColPos::Right => cur + 1, ColPos::Left => cur };
            (p, if in_left { lc + 1 } else { lc })
        };
        self.view.left_count = new_lc;
        let date_fmt = if kind == CategoryKind::Date { Some(DateFmt::default()) } else { None };
        self.view.columns.insert(pos, Column { id, name, cat_id, width, date_fmt });
        self.col_cursor = pos + 1;
    }

    pub fn col_quick_add_cancel(&mut self) {
        self.col_mode = ColMode::Normal;
    }

    pub fn col_form_confirm(&mut self) {
        // Column head may not be blank.
        if matches!(&self.col_mode, ColMode::Form { head_cat_idx: None, .. }) { return; }

        let old_mode = std::mem::replace(&mut self.col_mode, ColMode::Normal);
        if let ColMode::Form { is_add, head_cat_idx, width_buf, position, .. } = old_mode {
            let flat   = flatten_cats(&self.categories);
            let flat_e = head_cat_idx.and_then(|i| flat.get(i));
            let name   = flat_e.map(|c| c.name.clone()).unwrap_or_default();
            let cat_id = flat_e.map(|c| c.id).unwrap_or(0);
            let width  = width_buf.trim().parse::<usize>().unwrap_or(12).max(1);
            let kind = flat_e.map(|c| c.kind).unwrap_or(CategoryKind::Standard);
            if is_add {
                let id = self.alloc_id();
                let lc = self.view.left_count;
                let (pos, new_lc) = if self.view.columns.is_empty() {
                    let is_left = self.col_cursor == 0 && position == ColPos::Left;
                    (0, if is_left { 1 } else { 0 })
                } else if self.col_cursor == 0 {
                    match position {
                        ColPos::Left  => (lc, lc + 1),
                        ColPos::Right => (lc, lc),
                    }
                } else {
                    let cur = (self.col_cursor - 1).min(self.view.columns.len() - 1);
                    let in_left = cur < lc;
                    let p = match position { ColPos::Right => cur + 1, ColPos::Left => cur };
                    (p, if in_left { lc + 1 } else { lc })
                };
                self.view.left_count = new_lc;
                let date_fmt = if kind == CategoryKind::Date { Some(DateFmt::default()) } else { None };
                self.view.columns.insert(pos, Column { id, name, cat_id, width, date_fmt });
                self.col_cursor = pos + 1;  // +1: 0=items column, 1..n=added columns
            } else if !self.view.columns.is_empty() && self.col_cursor > 0 {
                let idx = (self.col_cursor - 1).min(self.view.columns.len() - 1);
                self.view.columns[idx].name   = name;
                self.view.columns[idx].cat_id = cat_id;
                self.view.columns[idx].width  = width;
                // Preserve or update date_fmt based on new kind
                if kind == CategoryKind::Date && self.view.columns[idx].date_fmt.is_none() {
                    self.view.columns[idx].date_fmt = Some(DateFmt::default());
                } else if kind != CategoryKind::Date {
                    self.view.columns[idx].date_fmt = None;
                }
            }
        }
    }

    pub fn col_form_cancel(&mut self) {
        self.col_mode = ColMode::Normal;
    }

    pub fn col_form_field_next(&mut self) {
        if let ColMode::Form { active_field, is_add, .. } = &mut self.col_mode {
            *active_field = match active_field {
                ColFormField::Head     => ColFormField::Width,
                ColFormField::Width    => if *is_add { ColFormField::Position } else { ColFormField::Head },
                ColFormField::Position => ColFormField::Head,
            };
        }
    }

    pub fn col_form_field_prev(&mut self) {
        if let ColMode::Form { active_field, is_add, .. } = &mut self.col_mode {
            *active_field = match active_field {
                ColFormField::Head     => if *is_add { ColFormField::Position } else { ColFormField::Width },
                ColFormField::Width    => ColFormField::Head,
                ColFormField::Position => ColFormField::Width,
            };
        }
    }

    pub fn col_form_input_char(&mut self, ch: char) {
        // Head field is a category picker (read-only); only Width accepts typed input.
        if let ColMode::Form { active_field: ColFormField::Width, width_buf, width_cur, .. } = &mut self.col_mode {
            if ch.is_ascii_digit() {
                let byte = char_to_byte(width_buf, *width_cur);
                width_buf.insert(byte, ch);
                *width_cur += 1;
            }
        }
    }

    pub fn col_form_backspace(&mut self) {
        // Backspace only applies to the Width field.
        if let ColMode::Form { active_field: ColFormField::Width, width_buf, width_cur, .. } = &mut self.col_mode {
            if *width_cur > 0 {
                *width_cur -= 1;
                let byte = char_to_byte(width_buf, *width_cur);
                width_buf.remove(byte);
            }
        }
    }

    pub fn col_form_cursor_left(&mut self) {
        if let ColMode::Form { active_field: ColFormField::Width, width_cur, .. } = &mut self.col_mode {
            if *width_cur > 0 { *width_cur -= 1; }
        }
    }

    pub fn col_form_cursor_right(&mut self) {
        if let ColMode::Form { active_field: ColFormField::Width, width_buf, width_cur, .. } = &mut self.col_mode {
            let len = width_buf.chars().count();
            if *width_cur < len { *width_cur += 1; }
        }
    }

    pub fn col_open_choices(&mut self) {
        let active = match &self.col_mode {
            ColMode::Form { active_field, .. } => *active_field,
            _ => return,
        };
        if !matches!(active, ColFormField::Head | ColFormField::Position) { return; }

        let old = std::mem::replace(&mut self.col_mode, ColMode::Normal);
        if let ColMode::Form { is_add, head_cat_idx, width_buf, width_cur, position, active_field } = old {
            let (kind, picker_cursor) = match active_field {
                ColFormField::Head => {
                    let flat = flatten_cats(&self.categories);
                    let cur  = head_cat_idx.unwrap_or(0).min(flat.len().saturating_sub(1));
                    (ChoicesKind::Category, cur)
                }
                ColFormField::Position => {
                    let cur = match position { ColPos::Right => 0, ColPos::Left => 1 };
                    (ChoicesKind::Position, cur)
                }
                ColFormField::Width => unreachable!(),
            };
            self.col_mode = ColMode::Choices {
                is_add, head_cat_idx, width_buf, width_cur, position, active_field,
                picker_cursor, kind,
            };
        }
    }

    pub fn col_choices_up(&mut self) {
        if let ColMode::Choices { picker_cursor, .. } = &mut self.col_mode {
            if *picker_cursor > 0 { *picker_cursor -= 1; }
        }
    }

    pub fn col_choices_down(&mut self) {
        let list_len = match &self.col_mode {
            ColMode::Choices { kind: ChoicesKind::Category, .. } => flatten_cats(&self.categories).len(),
            ColMode::Choices { kind: ChoicesKind::Position, .. } => 2,
            _ => return,
        };
        if let ColMode::Choices { picker_cursor, .. } = &mut self.col_mode {
            if *picker_cursor + 1 < list_len { *picker_cursor += 1; }
        }
    }

    pub fn col_choices_confirm(&mut self) {
        let old = std::mem::replace(&mut self.col_mode, ColMode::Normal);
        if let ColMode::Choices { is_add, head_cat_idx, picker_cursor, width_buf, width_cur,
                                  position, active_field, kind } = old {
            let (new_head, new_pos) = match kind {
                ChoicesKind::Category => (Some(picker_cursor), position),
                ChoicesKind::Position => {
                    let p = if picker_cursor == 0 { ColPos::Right } else { ColPos::Left };
                    (head_cat_idx, p)
                }
            };
            self.col_mode = ColMode::Form {
                is_add, head_cat_idx: new_head, width_buf, width_cur,
                position: new_pos, active_field,
            };
        }
    }

    pub fn col_choices_cancel(&mut self) {
        let old = std::mem::replace(&mut self.col_mode, ColMode::Normal);
        if let ColMode::Choices { is_add, head_cat_idx, width_buf, width_cur, position, active_field, .. } = old {
            self.col_mode = ColMode::Form { is_add, head_cat_idx, width_buf, width_cur, position, active_field };
        }
    }

    pub fn col_move_left(&mut self) {
        if self.col_cursor <= 1 { return; }  // already leftmost
        let lc = self.view.left_count;
        // Crossing main boundary: first right col → last left col
        if self.col_cursor == lc + 1 {
            self.view.left_count += 1;
            return; // col_cursor unchanged; column is now classified as left
        }
        self.view.columns.swap(self.col_cursor - 1, self.col_cursor - 2);
        self.col_cursor -= 1;
    }

    pub fn col_move_right(&mut self) {
        if self.col_cursor == 0 || self.col_cursor >= self.view.columns.len() { return; }
        let lc = self.view.left_count;
        // Crossing main boundary: last left col → first right col
        if self.col_cursor == lc && lc > 0 {
            self.view.left_count -= 1;
            return; // col_cursor unchanged; column is now classified as right
        }
        self.view.columns.swap(self.col_cursor - 1, self.col_cursor);
        self.col_cursor += 1;
    }

    // ── Column Properties modal ───────────────────────────────────────────────

    pub fn col_open_props(&mut self) {
        if self.col_cursor == 0 || self.view.columns.is_empty() { return; }
        let idx = (self.col_cursor - 1).min(self.view.columns.len() - 1);
        let col = &self.view.columns[idx];
        let head_buf  = col.name.clone();
        let head_cur  = head_buf.chars().count();
        let width_buf = col.width.to_string();
        let width_cur = width_buf.chars().count();
        let is_date   = col.date_fmt.is_some();
        let date_fmt  = col.date_fmt.clone();
        self.col_mode = ColMode::Props {
            head_buf, head_cur, width_buf, width_cur,
            date_fmt, active_field: PropsField::Head, is_date,
        };
    }

    pub fn col_props_confirm(&mut self) {
        let old = std::mem::replace(&mut self.col_mode, ColMode::Normal);
        if let ColMode::Props { head_buf, width_buf, date_fmt, .. } = old {
            if self.view.columns.is_empty() || self.col_cursor == 0 { return; }
            let idx  = (self.col_cursor - 1).min(self.view.columns.len() - 1);
            let name = head_buf.trim().to_string();
            if !name.is_empty() {
                let cat_id = self.view.columns[idx].cat_id;
                self.view.columns[idx].name = name.clone();
                let flat = flatten_cats(&self.categories);
                if let Some(e) = flat.iter().find(|c| c.id == cat_id) {
                    let path = e.path.clone();
                    rename_cat(&mut self.categories, &path, name);
                }
            }
            let width = width_buf.trim().parse::<usize>().unwrap_or(12).max(1);
            self.view.columns[idx].width    = width;
            self.view.columns[idx].date_fmt = date_fmt;
        }
    }

    pub fn col_props_cancel(&mut self) {
        self.col_mode = ColMode::Normal;
    }

    pub fn col_props_field_next(&mut self) {
        if let ColMode::Props { ref mut active_field, is_date, .. } = self.col_mode {
            *active_field = match active_field {
                PropsField::Head        => PropsField::Width,
                PropsField::Width       => if is_date { PropsField::DateDisplay } else { PropsField::Head },
                PropsField::DateDisplay => PropsField::ShowDow,
                PropsField::ShowDow     => PropsField::Clock,
                PropsField::Clock       => PropsField::DateFmtCode,
                PropsField::DateFmtCode => PropsField::ShowAmPm,
                PropsField::ShowAmPm    => PropsField::DateSep,
                PropsField::DateSep     => PropsField::TimeSep,
                PropsField::TimeSep     => PropsField::Head,
            };
        }
    }

    pub fn col_props_field_prev(&mut self) {
        if let ColMode::Props { ref mut active_field, is_date, .. } = self.col_mode {
            *active_field = match active_field {
                PropsField::Head        => if is_date { PropsField::TimeSep } else { PropsField::Width },
                PropsField::Width       => PropsField::Head,
                PropsField::DateDisplay => PropsField::Width,
                PropsField::ShowDow     => PropsField::DateDisplay,
                PropsField::Clock       => PropsField::ShowDow,
                PropsField::DateFmtCode => PropsField::Clock,
                PropsField::ShowAmPm    => PropsField::DateFmtCode,
                PropsField::DateSep     => PropsField::ShowAmPm,
                PropsField::TimeSep     => PropsField::DateSep,
            };
        }
    }

    pub fn col_props_input_char(&mut self, ch: char) {
        if let ColMode::Props { ref active_field, ref mut head_buf, ref mut head_cur,
                                ref mut width_buf, ref mut width_cur, .. } = self.col_mode {
            match active_field {
                PropsField::Head => {
                    let byte = char_to_byte(head_buf, *head_cur);
                    head_buf.insert(byte, ch);
                    *head_cur += 1;
                }
                PropsField::Width => {
                    if ch.is_ascii_digit() {
                        let byte = char_to_byte(width_buf, *width_cur);
                        width_buf.insert(byte, ch);
                        *width_cur += 1;
                    }
                }
                _ => {}
            }
        }
    }

    pub fn col_props_backspace(&mut self) {
        if let ColMode::Props { ref active_field, ref mut head_buf, ref mut head_cur,
                                ref mut width_buf, ref mut width_cur, .. } = self.col_mode {
            match active_field {
                PropsField::Head => {
                    if *head_cur > 0 {
                        *head_cur -= 1;
                        let byte = char_to_byte(head_buf, *head_cur);
                        head_buf.remove(byte);
                    }
                }
                PropsField::Width => {
                    if *width_cur > 0 {
                        *width_cur -= 1;
                        let byte = char_to_byte(width_buf, *width_cur);
                        width_buf.remove(byte);
                    }
                }
                _ => {}
            }
        }
    }

    pub fn col_props_left(&mut self) {
        if let ColMode::Props { ref active_field, ref mut head_cur, ref mut width_cur,
                                ref mut date_fmt, .. } = self.col_mode {
            match active_field {
                PropsField::Head  => { if *head_cur > 0 { *head_cur -= 1; } }
                PropsField::Width => { if *width_cur > 0 { *width_cur -= 1; } }
                PropsField::DateDisplay => {
                    if let Some(fmt) = date_fmt {
                        fmt.display = match fmt.display {
                            DateDisplay::Date     => DateDisplay::DateTime,
                            DateDisplay::Time     => DateDisplay::Date,
                            DateDisplay::DateTime => DateDisplay::Time,
                        };
                    }
                }
                PropsField::ShowDow => {
                    if let Some(fmt) = date_fmt { fmt.show_dow = !fmt.show_dow; }
                }
                PropsField::Clock => {
                    if let Some(fmt) = date_fmt {
                        fmt.clock = match fmt.clock {
                            Clock::Hr12 => Clock::Hr24,
                            Clock::Hr24 => Clock::Hr12,
                        };
                    }
                }
                PropsField::DateFmtCode => {
                    if let Some(fmt) = date_fmt {
                        fmt.code = match fmt.code {
                            DateFmtCode::MMDDYY   => DateFmtCode::YYYYMMDD,
                            DateFmtCode::DDMMYY   => DateFmtCode::MMDDYY,
                            DateFmtCode::YYYYMMDD => DateFmtCode::DDMMYY,
                        };
                    }
                }
                PropsField::ShowAmPm => {
                    if let Some(fmt) = date_fmt { fmt.show_ampm = !fmt.show_ampm; }
                }
                PropsField::DateSep => {
                    if let Some(fmt) = date_fmt {
                        fmt.date_sep = cycle_date_sep_prev(fmt.date_sep);
                    }
                }
                PropsField::TimeSep => {
                    if let Some(fmt) = date_fmt {
                        fmt.time_sep = cycle_date_sep_prev(fmt.time_sep);
                    }
                }
            }
        }
    }

    pub fn col_props_right(&mut self) {
        if let ColMode::Props { ref active_field, ref mut head_cur, ref head_buf,
                                ref mut width_cur, ref width_buf, ref mut date_fmt, .. } = self.col_mode {
            match active_field {
                PropsField::Head  => {
                    let len = head_buf.chars().count();
                    if *head_cur < len { *head_cur += 1; }
                }
                PropsField::Width => {
                    let len = width_buf.chars().count();
                    if *width_cur < len { *width_cur += 1; }
                }
                PropsField::DateDisplay => {
                    if let Some(fmt) = date_fmt {
                        fmt.display = match fmt.display {
                            DateDisplay::Date     => DateDisplay::Time,
                            DateDisplay::Time     => DateDisplay::DateTime,
                            DateDisplay::DateTime => DateDisplay::Date,
                        };
                    }
                }
                PropsField::ShowDow => {
                    if let Some(fmt) = date_fmt { fmt.show_dow = !fmt.show_dow; }
                }
                PropsField::Clock => {
                    if let Some(fmt) = date_fmt {
                        fmt.clock = match fmt.clock {
                            Clock::Hr12 => Clock::Hr24,
                            Clock::Hr24 => Clock::Hr12,
                        };
                    }
                }
                PropsField::DateFmtCode => {
                    if let Some(fmt) = date_fmt {
                        fmt.code = match fmt.code {
                            DateFmtCode::MMDDYY   => DateFmtCode::DDMMYY,
                            DateFmtCode::DDMMYY   => DateFmtCode::YYYYMMDD,
                            DateFmtCode::YYYYMMDD => DateFmtCode::MMDDYY,
                        };
                    }
                }
                PropsField::ShowAmPm => {
                    if let Some(fmt) = date_fmt { fmt.show_ampm = !fmt.show_ampm; }
                }
                PropsField::DateSep => {
                    if let Some(fmt) = date_fmt {
                        fmt.date_sep = cycle_date_sep_next(fmt.date_sep);
                    }
                }
                PropsField::TimeSep => {
                    if let Some(fmt) = date_fmt {
                        fmt.time_sep = cycle_date_sep_next(fmt.time_sep);
                    }
                }
            }
        }
    }

    pub fn cursor_col_left(&mut self) {
        if !matches!(self.mode, Mode::Normal) { return; }
        let lc = self.view.left_count;
        self.col_cursor = match self.col_cursor {
            0 if lc > 0 => lc,      // main → last left col
            0           => 0,       // no left cols, stay
            1 if lc > 0 => 1,       // first left col → stay (already leftmost)
            1           => 0,       // first right col (lc==0) → main
            c if c == lc + 1 => 0,  // first right col → main
            c           => c - 1,   // within left or within right
        };
    }

    pub fn cursor_col_right(&mut self) {
        if !matches!(self.mode, Mode::Normal) { return; }
        let n  = self.view.columns.len();
        let lc = self.view.left_count;
        self.col_cursor = match self.col_cursor {
            0           => if lc + 1 <= n { lc + 1 } else { 0 },  // main → first right
            c if c == lc && lc > 0 => 0,  // last left col → main
            c if c < n  => c + 1,          // within left or within right
            _           => self.col_cursor, // already rightmost
        };
    }

    // ── CatMgr navigation ────────────────────────────────────────────────────

    pub fn cat_cursor_up(&mut self) {
        if !matches!(self.cat_state.mode, CatMode::Normal) { return; }
        if self.cat_state.cursor > 0 {
            self.cat_state.cursor -= 1;
        }
    }

    pub fn cat_cursor_down(&mut self) {
        if !matches!(self.cat_state.mode, CatMode::Normal) { return; }
        let flat = flatten_cats(&self.categories);
        if self.cat_state.cursor + 1 < flat.len() {
            self.cat_state.cursor += 1;
        }
    }

    // ── CatMgr buffer cursor ──────────────────────────────────────────────────

    pub fn cat_edit_cursor_left(&mut self) {
        match &mut self.cat_state.mode {
            CatMode::Edit { cursor, .. } | CatMode::Create { cursor, .. } => {
                if *cursor > 0 { *cursor -= 1; }
            }
            CatMode::Normal => {}
        }
    }

    pub fn cat_edit_cursor_right(&mut self) {
        match &mut self.cat_state.mode {
            CatMode::Edit { buffer, cursor } | CatMode::Create { buffer, cursor, .. } => {
                let len = buffer.chars().count();
                if *cursor < len { *cursor += 1; }
            }
            CatMode::Normal => {}
        }
    }

    // ── CatMgr mode transitions ───────────────────────────────────────────────

    pub fn cat_begin_edit(&mut self) {
        if !matches!(self.cat_state.mode, CatMode::Normal) { return; }
        let flat = flatten_cats(&self.categories);
        if flat.is_empty() { return; }
        let idx = self.cat_state.cursor.min(flat.len() - 1);
        let name = flat[idx].name.clone();
        self.cat_state.mode = CatMode::Edit { buffer: name, cursor: 0 };
    }

    /// `as_child = false` → sibling below (INS), `true` → child (Alt-R)
    pub fn cat_begin_create(&mut self, as_child: bool) {
        if !matches!(self.cat_state.mode, CatMode::Normal) { return; }
        self.cat_state.mode = CatMode::Create { buffer: String::new(), cursor: 0, as_child };
    }

    // ── CatMgr input ─────────────────────────────────────────────────────────

    pub fn cat_input_char(&mut self, ch: char) {
        let (buffer, cursor) = match &mut self.cat_state.mode {
            CatMode::Edit   { buffer, cursor }     => (buffer, cursor),
            CatMode::Create { buffer, cursor, .. } => (buffer, cursor),
            CatMode::Normal => return,
        };
        let byte_pos = char_to_byte(buffer, *cursor);
        buffer.insert(byte_pos, ch);
        *cursor += 1;
    }

    pub fn cat_input_backspace(&mut self) {
        let (buffer, cursor) = match &mut self.cat_state.mode {
            CatMode::Edit   { buffer, cursor }     => (buffer, cursor),
            CatMode::Create { buffer, cursor, .. } => (buffer, cursor),
            CatMode::Normal => return,
        };
        if *cursor > 0 {
            *cursor -= 1;
            let byte_pos = char_to_byte(buffer, *cursor);
            buffer.remove(byte_pos);
        }
    }

    pub fn cat_confirm(&mut self) {
        match std::mem::replace(&mut self.cat_state.mode, CatMode::Normal) {
            CatMode::Edit { buffer, .. } => {
                let text = buffer.trim().to_string();
                if text.is_empty() { return; }
                let flat = flatten_cats(&self.categories);
                if flat.is_empty() { return; }
                let idx = self.cat_state.cursor.min(flat.len() - 1);
                rename_cat(&mut self.categories, &flat[idx].path.clone(), text);
            }
            CatMode::Create { buffer, as_child, .. } => {
                let text = buffer.trim().to_string();
                if text.is_empty() { return; }
                let id  = self.alloc_id();
                let cat = Category { id, name: text, kind: CategoryKind::Standard, children: vec![] };
                let flat = flatten_cats(&self.categories);
                if flat.is_empty() {
                    self.categories.push(cat);
                    self.cat_state.cursor = 0;
                } else {
                    let idx  = self.cat_state.cursor.min(flat.len() - 1);
                    let path = flat[idx].path.clone();
                    if as_child {
                        let n = children_count(&self.categories, &path);
                        insert_at(&mut self.categories, &path, n, cat);
                    } else {
                        let my_idx     = *path.last().unwrap();
                        let parent     = &path[..path.len() - 1];
                        insert_at(&mut self.categories, parent, my_idx + 1, cat);
                    }
                    let new_flat = flatten_cats(&self.categories);
                    if let Some(pos) = new_flat.iter().position(|e| e.id == id) {
                        self.cat_state.cursor = pos;
                    }
                }
            }
            CatMode::Normal => {}
        }
    }

    pub fn cat_cancel(&mut self) {
        self.cat_state.mode = CatMode::Normal;
    }

    pub fn cat_delete(&mut self) {
        if !matches!(self.cat_state.mode, CatMode::Normal) { return; }
        let flat = flatten_cats(&self.categories);
        if flat.is_empty() { return; }
        let idx  = self.cat_state.cursor.min(flat.len() - 1);
        let path = flat[idx].path.clone();
        take_cat(&mut self.categories, &path);
        let new_flat = flatten_cats(&self.categories);
        self.cat_state.cursor =
            self.cat_state.cursor.min(new_flat.len().saturating_sub(1));
    }

    // ── CatMgr tree restructuring ─────────────────────────────────────────────

    /// Promote: move current category up one level (become sibling of its parent).
    pub fn cat_promote(&mut self) {
        if !matches!(self.cat_state.mode, CatMode::Normal) { return; }
        let flat = flatten_cats(&self.categories);
        if flat.is_empty() { return; }
        let idx  = self.cat_state.cursor.min(flat.len() - 1);
        let path = flat[idx].path.clone();
        if path.len() <= 1 { return; }  // already top-level

        let cat_id     = flat[idx].id;
        let parent_idx = path[path.len() - 2];
        let gp_path    = &path[..path.len() - 2];

        let cat = take_cat(&mut self.categories, &path);
        // Insert after parent in grandparent's children
        insert_at(&mut self.categories, gp_path, parent_idx + 1, cat);

        let new_flat = flatten_cats(&self.categories);
        if let Some(pos) = new_flat.iter().position(|e| e.id == cat_id) {
            self.cat_state.cursor = pos;
        }
    }

    /// Demote: make current category the last child of its previous sibling.
    pub fn cat_demote(&mut self) {
        if !matches!(self.cat_state.mode, CatMode::Normal) { return; }
        let flat = flatten_cats(&self.categories);
        if flat.is_empty() { return; }
        let idx  = self.cat_state.cursor.min(flat.len() - 1);
        let path = flat[idx].path.clone();
        let my_idx = *path.last().unwrap();
        if my_idx == 0 { return; }  // no previous sibling

        let cat_id    = flat[idx].id;
        let prev_idx  = my_idx - 1;
        let parent    = &path[..path.len() - 1];

        let cat = take_cat(&mut self.categories, &path);

        // Previous sibling is still at prev_idx (we only removed current, which came after)
        let mut prev_sib_path = parent.to_vec();
        prev_sib_path.push(prev_idx);
        let n = children_count(&self.categories, &prev_sib_path);
        insert_at(&mut self.categories, &prev_sib_path, n, cat);

        let new_flat = flatten_cats(&self.categories);
        if let Some(pos) = new_flat.iter().position(|e| e.id == cat_id) {
            self.cat_state.cursor = pos;
        }
    }

    // ── Calendar picker ───────────────────────────────────────────────────────

    pub fn col_open_calendar(&mut self) {
        if !matches!(self.mode, Mode::Normal) { return; }
        if self.col_cursor == 0 { return; }
        let col_idx = self.col_cursor - 1;
        if col_idx >= self.view.columns.len() { return; }
        if self.view.columns[col_idx].date_fmt.is_none() { return; }
        if !matches!(self.cursor, CursorPos::Item { .. }) { return; }
        let cat_id = self.view.columns[col_idx].cat_id;
        let val = match &self.cursor {
            CursorPos::Item { section, item } =>
                self.view.sections[*section].items[*item]
                    .values.get(&cat_id).cloned().unwrap_or_default(),
            _ => String::new(),
        };
        let (year, month, day, hour, min, sec) =
            parse_datetime(&val).unwrap_or_else(|| {
                let (y, mo, d) = today();
                (y, mo, d, 0, 0, 0)
            });
        self.col_mode = ColMode::Calendar { year, month, day, hour, min, sec };
    }

    pub fn col_calendar_left(&mut self) {
        if let ColMode::Calendar { year, month, day, .. } = &mut self.col_mode {
            if *day > 1 { *day -= 1; }
            else if *month > 1 { *month -= 1; *day = days_in_month(*year, *month); }
            else { *year -= 1; *month = 12; *day = 31; }
        }
    }

    pub fn col_calendar_right(&mut self) {
        if let ColMode::Calendar { year, month, day, .. } = &mut self.col_mode {
            let dim = days_in_month(*year, *month);
            if *day < dim { *day += 1; }
            else if *month < 12 { *month += 1; *day = 1; }
            else { *year += 1; *month = 1; *day = 1; }
        }
    }

    pub fn col_calendar_up(&mut self) {
        if let ColMode::Calendar { year, month, day, .. } = &mut self.col_mode {
            for _ in 0..7 {
                if *day > 1 { *day -= 1; }
                else {
                    if *month > 1 { *month -= 1; } else { *year -= 1; *month = 12; }
                    *day = days_in_month(*year, *month);
                }
            }
        }
    }

    pub fn col_calendar_down(&mut self) {
        if let ColMode::Calendar { year, month, day, .. } = &mut self.col_mode {
            for _ in 0..7 {
                let dim = days_in_month(*year, *month);
                if *day < dim { *day += 1; }
                else {
                    if *month < 12 { *month += 1; } else { *year += 1; *month = 1; }
                    *day = 1;
                }
            }
        }
    }

    pub fn col_calendar_pgup(&mut self) {
        if let ColMode::Calendar { year, month, day, .. } = &mut self.col_mode {
            if *month > 1 { *month -= 1; } else { *year -= 1; *month = 12; }
            *day = (*day).min(days_in_month(*year, *month));
        }
    }

    pub fn col_calendar_pgdn(&mut self) {
        if let ColMode::Calendar { year, month, day, .. } = &mut self.col_mode {
            if *month < 12 { *month += 1; } else { *year += 1; *month = 1; }
            *day = (*day).min(days_in_month(*year, *month));
        }
    }

    pub fn col_calendar_year_prev(&mut self) {
        if let ColMode::Calendar { year, month, day, .. } = &mut self.col_mode {
            *year -= 1;
            *day = (*day).min(days_in_month(*year, *month));
        }
    }

    pub fn col_calendar_year_next(&mut self) {
        if let ColMode::Calendar { year, month, day, .. } = &mut self.col_mode {
            *year += 1;
            *day = (*day).min(days_in_month(*year, *month));
        }
    }

    pub fn col_calendar_confirm(&mut self) {
        let (year, month, day, hour, min, sec) = match &self.col_mode {
            ColMode::Calendar { year, month, day, hour, min, sec } =>
                (*year, *month, *day, *hour, *min, *sec),
            _ => return,
        };
        self.col_mode = ColMode::Normal;
        if self.col_cursor == 0 { return; }
        let col_idx = self.col_cursor - 1;
        if col_idx >= self.view.columns.len() { return; }
        let cat_id = self.view.columns[col_idx].cat_id;
        let val = format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", year, month, day, hour, min, sec);
        if let CursorPos::Item { section, item } = &self.cursor {
            let (s, i) = (*section, *item);
            self.view.sections[s].items[i].values.insert(cat_id, val);
        }
    }

    pub fn col_calendar_cancel(&mut self) {
        self.col_mode = ColMode::Normal;
    }

    // ── SetTime modal ─────────────────────────────────────────────────────────

    pub fn col_open_set_time(&mut self) {
        let (year, month, day, hour, min, sec) = match &self.col_mode {
            ColMode::Calendar { year, month, day, hour, min, sec } =>
                (*year, *month, *day, *hour, *min, *sec),
            _ => return,
        };
        self.col_mode = ColMode::SetTime {
            year, month, day,
            hour_buf:  format!("{:02}", hour),
            min_buf:   format!("{:02}", min),
            sec_buf:   format!("{:02}", sec),
            active:    TimeField::Hour,
            orig_hour: hour,
            orig_min:  min,
            orig_sec:  sec,
        };
    }

    pub fn col_set_time_confirm(&mut self) {
        let (year, month, day, h, m, s) = match &self.col_mode {
            ColMode::SetTime { year, month, day, hour_buf, min_buf, sec_buf, .. } => {
                let h = hour_buf.trim().parse::<u32>().unwrap_or(0).min(23);
                let m = min_buf.trim().parse::<u32>().unwrap_or(0).min(59);
                let s = sec_buf.trim().parse::<u32>().unwrap_or(0).min(59);
                (*year, *month, *day, h, m, s)
            }
            _ => return,
        };
        self.col_mode = ColMode::Normal;
        if self.col_cursor == 0 { return; }
        let col_idx = self.col_cursor - 1;
        if col_idx >= self.view.columns.len() { return; }
        let cat_id = self.view.columns[col_idx].cat_id;
        let val = format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", year, month, day, h, m, s);
        if let CursorPos::Item { section, item } = &self.cursor {
            let (si, ii) = (*section, *item);
            self.view.sections[si].items[ii].values.insert(cat_id, val);
        }
    }

    pub fn col_set_time_cancel(&mut self) {
        let (year, month, day, orig_hour, orig_min, orig_sec) = match &self.col_mode {
            ColMode::SetTime { year, month, day, orig_hour, orig_min, orig_sec, .. } =>
                (*year, *month, *day, *orig_hour, *orig_min, *orig_sec),
            _ => return,
        };
        self.col_mode = ColMode::Calendar {
            year, month, day,
            hour: orig_hour, min: orig_min, sec: orig_sec,
        };
    }

    pub fn col_set_time_left(&mut self) {
        if let ColMode::SetTime { active, .. } = &mut self.col_mode {
            *active = match active {
                TimeField::Hour => TimeField::Sec,
                TimeField::Min  => TimeField::Hour,
                TimeField::Sec  => TimeField::Min,
            };
        }
    }

    pub fn col_set_time_right(&mut self) {
        if let ColMode::SetTime { active, .. } = &mut self.col_mode {
            *active = match active {
                TimeField::Hour => TimeField::Min,
                TimeField::Min  => TimeField::Sec,
                TimeField::Sec  => TimeField::Hour,
            };
        }
    }

    pub fn col_set_time_backspace(&mut self) {
        if let ColMode::SetTime { active, hour_buf, min_buf, sec_buf, .. } = &mut self.col_mode {
            let buf = match active {
                TimeField::Hour => hour_buf,
                TimeField::Min  => min_buf,
                TimeField::Sec  => sec_buf,
            };
            buf.pop();
        }
    }

    pub fn col_set_time_input_char(&mut self, ch: char) {
        if !ch.is_ascii_digit() { return; }
        if let ColMode::SetTime { active, hour_buf, min_buf, sec_buf, .. } = &mut self.col_mode {
            let buf = match active {
                TimeField::Hour => hour_buf,
                TimeField::Min  => min_buf,
                TimeField::Sec  => sec_buf,
            };
            if buf.len() >= 2 { buf.clear(); }
            buf.push(ch);
        }
    }
}
