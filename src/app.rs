use crate::menu::{MenuAction, CATMGR_MENU, VIEW_MENU};
use crate::model::{Category, CategoryKind, ColFormat, Column, DateFmt, DateDisplay, Clock, DateFmtCode, Item, Section, View};
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
    /// "Remove this item from the section?" confirmation dialog.
    ConfirmDeleteItem { yes: bool },
}

// ── CatMgr state ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum CatPropsField {
    Name, ShortName, AlsoMatch, Note, NoteFile,
    ExclChildren, MatchCatName, MatchShortName,
}

pub enum CatMode {
    Normal,
    Edit   { buffer: String, cursor: usize },
    /// `as_child`: insert below as child (Alt-R) vs sibling (INS)
    Create { buffer: String, cursor: usize, as_child: bool },
    /// Category Properties modal (F6).
    Props {
        name_buf:         String,
        name_cur:         usize,
        short_name_buf:   String,
        short_name_cur:   usize,
        also_match_buf:   String,
        also_match_cur:   usize,
        note_file_buf:    String,
        note_file_cur:    usize,
        excl_children:    bool,
        match_cat_name:   bool,
        match_short_name: bool,
        active_field:     CatPropsField,
        /// Snapshotted at open time for read-only display.
        parent_name:      String,
        kind:             CategoryKind,
        has_note:         bool,
        cat_id:           usize,
    },
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
    Head, Width, Format,
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
        format:       ColFormat,
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
    /// "Remove this column from the view?" confirmation dialog.
    ConfirmRemove { yes: bool },
}

// ── Assignment Profile state ──────────────────────────────────────────────────

pub enum AssignMode {
    Normal,
    /// Assignment Profile open; `gi` is the global index into `view.items`.
    Profile {
        gi:     usize,
        cursor: usize,   // index into flatten_cats
        on_sub: bool,    // true when cursor rests on a Date value sub-row
    },
}

// ── Section Add state ─────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum SectionInsert { Below, Above }

#[derive(Clone, Copy, PartialEq)]
pub enum SectionFormField { Category, Insert }

pub enum SectionMode {
    Normal,
    Add {
        cat_idx:      Option<usize>,
        insert:       SectionInsert,
        active_field: SectionFormField,
    },
    Choices {
        cat_idx:       Option<usize>,
        insert:        SectionInsert,
        active_field:  SectionFormField,
        picker_cursor: usize,
    },
    ConfirmRemove { yes: bool },
}

// ── Note state ────────────────────────────────────────────────────────────────

/// Identifies what the pending note operation is attached to.
#[derive(Clone)]
pub enum NoteTarget {
    Item(usize),   // global item index
    Cat(usize),    // category id
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
    /// Which sub-row within the current multi-assignment item is highlighted (col_cursor > 0 only).
    pub sub_row:     usize,
    // Assignment Profile
    pub assign_mode: AssignMode,
    // Category search (shared across CatMgr and Assignment Profile)
    pub cat_search:  Option<String>,
    // Section
    pub sec_mode:    SectionMode,
    // Menu
    pub menu:        MenuState,
    // F-key bar
    pub fkey_mod:    FKeyMod,
    // Note
    pub pending_note: Option<NoteTarget>,
    // Misc
    pub quit:        bool,
    next_id:         usize,
}

// ── Byte-offset helper ────────────────────────────────────────────────────────

fn char_to_byte(s: &str, n: usize) -> usize {
    s.char_indices().nth(n).map(|(b, _)| b).unwrap_or(s.len())
}

// ── Section item filtering ────────────────────────────────────────────────────

/// Returns global item-pool indices for the items that belong to section `sec_idx`.
///
/// A section shows items whose `values` map contains a key that is `cat_id`
/// or any descendant of `cat_id` in the category tree.
pub fn section_item_indices(view: &View, sec_idx: usize, cats: &[Category]) -> Vec<usize> {
    if sec_idx >= view.sections.len() { return vec![]; }
    let mut parent_map = HashMap::new();
    let mut name_map   = HashMap::new();
    build_cat_maps(cats, None, &mut parent_map, &mut name_map);

    let is_under = |mut id: usize, target: usize| -> bool {
        loop {
            if id == target { return true; }
            match parent_map.get(&id) {
                Some(Some(p)) => id = *p,
                _             => return false,
            }
        }
    };

    let cat_id = view.sections[sec_idx].cat_id;
    view.items.iter().enumerate()
        .filter(|(_, item)| item.values.keys().any(|&k| is_under(k, cat_id)))
        .map(|(i, _)| i)
        .collect()
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

// ── CatProps helpers (free functions) ─────────────────────────────────────────

/// Returns the parent's name, or `"(top level)"` if found at the top level.
pub fn find_cat_parent_name(cats: &[Category], target_id: usize, parent: Option<&str>) -> Option<String> {
    for cat in cats {
        if cat.id == target_id {
            return Some(parent.unwrap_or("(top level)").to_string());
        }
        if let Some(n) = find_cat_parent_name(&cat.children, target_id, Some(&cat.name)) {
            return Some(n);
        }
    }
    None
}

fn find_cat_by_id(cats: &[Category], id: usize) -> Option<&Category> {
    for cat in cats {
        if cat.id == id { return Some(cat); }
        if let Some(c) = find_cat_by_id(&cat.children, id) { return Some(c); }
    }
    None
}

fn find_cat_by_id_mut(cats: &mut Vec<Category>, id: usize) -> Option<&mut Category> {
    for cat in cats.iter_mut() {
        if cat.id == id { return Some(cat); }
        if let Some(c) = find_cat_by_id_mut(&mut cat.children, id) { return Some(c); }
    }
    None
}

// ── Note helpers (free functions) ─────────────────────────────────────────────

fn find_cat_note(cats: &[Category], id: usize) -> Option<String> {
    for cat in cats {
        if cat.id == id { return Some(cat.note.clone()); }
        if let Some(n) = find_cat_note(&cat.children, id) { return Some(n); }
    }
    None
}

fn find_cat_note_name(cats: &[Category], id: usize) -> Option<String> {
    for cat in cats {
        if cat.id == id { return Some(cat.name.clone()); }
        if let Some(n) = find_cat_note_name(&cat.children, id) { return Some(n); }
    }
    None
}

fn set_cat_note(cats: &mut Vec<Category>, id: usize, note: String) {
    for cat in cats.iter_mut() {
        if cat.id == id { cat.note = note; return; }
        set_cat_note(&mut cat.children, id, note.clone());
    }
}

/// Return the note string for the category backing `cat_id`, or empty string.
pub fn cat_note_for_id(cats: &[Category], id: usize) -> String {
    find_cat_note(cats, id).unwrap_or_default()
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
    let body = match fmt.display {
        DateDisplay::Date     => date_str,
        DateDisplay::Time     => time_str,
        DateDisplay::DateTime => format!("{} {}", date_str, time_str),
    };
    if fmt.show_dow && !matches!(fmt.display, DateDisplay::Time) {
        // Tomohiko Sakamoto: 0=Sun,1=Mon,...,6=Sat
        static DOW: [&str; 7] = ["Sun","Mon","Tue","Wed","Thu","Fri","Sat"];
        let t = [0i32, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
        let y = if month < 3 { year - 1 } else { year } as i32;
        let dow = ((y + y/4 - y/100 + y/400 + t[(month as usize)-1] + day as i32) % 7) as usize;
        format!("{} {}", DOW[dow], body)
    } else {
        body
    }
}

/// Build id→parent and id→name lookup maps from the category tree.
fn build_cat_maps(
    cats:       &[Category],
    parent:     Option<usize>,
    parent_map: &mut std::collections::HashMap<usize, Option<usize>>,
    name_map:   &mut std::collections::HashMap<usize, String>,
) {
    for cat in cats {
        parent_map.insert(cat.id, parent);
        name_map.insert(cat.id, cat.name.clone());
        build_cat_maps(&cat.children, Some(cat.id), parent_map, name_map);
    }
}

/// Compute the display string for a standard-format column cell.
/// For Date columns use `format_date_value` instead.
pub fn col_display_value(
    item_values: &std::collections::HashMap<usize, String>,
    col_cat_id:  usize,
    col_format:  ColFormat,
    cats:        &[Category],
) -> String {
    let mut parent_map = std::collections::HashMap::new();
    let mut name_map   = std::collections::HashMap::new();
    build_cat_maps(cats, None, &mut parent_map, &mut name_map);

    // Check whether a cat_id is at or under the column head
    let is_at_or_under = |mut id: usize| -> bool {
        loop {
            if id == col_cat_id { return true; }
            match parent_map.get(&id) {
                Some(Some(p)) => id = *p,
                _             => return false,
            }
        }
    };

    // Collect assigned cat_ids that are under the column head
    let matches: Vec<usize> = item_values.keys().copied()
        .filter(|&id| is_at_or_under(id))
        .collect();

    if matches.is_empty() {
        return match col_format {
            ColFormat::YesNo => "N".to_string(),
            _                => String::new(),
        };
    }

    match col_format {
        ColFormat::NameOnly => matches.iter()
            .filter_map(|&id| name_map.get(&id).cloned())
            .collect::<Vec<_>>().join(" "),

        ColFormat::ParentCategory => matches.iter().map(|&id| {
            let name   = name_map.get(&id).cloned().unwrap_or_default();
            let parent = parent_map.get(&id).and_then(|p| *p);
            match parent.filter(|&p| p != col_cat_id) {
                Some(pid) => {
                    let pname = name_map.get(&pid).cloned().unwrap_or_default();
                    format!("{}:{}", pname, name)
                }
                _ => name,
            }
        }).collect::<Vec<_>>().join(" "),

        ColFormat::Ancestor => {
            let mut seen   = std::collections::HashSet::new();
            let mut result = Vec::new();
            for &id in &matches {
                if id == col_cat_id { continue; }
                // Walk up to find the immediate child of col_cat_id
                let mut cur = id;
                loop {
                    match parent_map.get(&cur).and_then(|p| *p) {
                        Some(p) if p == col_cat_id => {
                            let n = name_map.get(&cur).cloned().unwrap_or_default();
                            if seen.insert(cur) { result.push(n); }
                            break;
                        }
                        Some(p) => cur = p,
                        None    => break,
                    }
                }
            }
            result.join(" ")
        }

        ColFormat::Star         => "*".to_string(),
        ColFormat::YesNo        => "Y".to_string(),
        ColFormat::CategoryNote => String::new(),  // notes not yet implemented
    }
}

/// Cycle ColFormat forward.
pub fn col_format_next(f: ColFormat) -> ColFormat {
    match f {
        ColFormat::NameOnly       => ColFormat::ParentCategory,
        ColFormat::ParentCategory => ColFormat::Ancestor,
        ColFormat::Ancestor       => ColFormat::Star,
        ColFormat::Star           => ColFormat::YesNo,
        ColFormat::YesNo          => ColFormat::CategoryNote,
        ColFormat::CategoryNote   => ColFormat::NameOnly,
    }
}

/// Cycle ColFormat backward.
pub fn col_format_prev(f: ColFormat) -> ColFormat {
    match f {
        ColFormat::NameOnly       => ColFormat::CategoryNote,
        ColFormat::ParentCategory => ColFormat::NameOnly,
        ColFormat::Ancestor       => ColFormat::ParentCategory,
        ColFormat::Star           => ColFormat::Ancestor,
        ColFormat::YesNo          => ColFormat::Star,
        ColFormat::CategoryNote   => ColFormat::YesNo,
    }
}

/// Return individual display strings for a standard column cell (one per assignment).
/// For NameOnly format returns one entry per assigned subcategory; other formats return one entry.
pub fn col_display_values(
    item_values: &HashMap<usize, String>,
    col_cat_id:  usize,
    col_format:  ColFormat,
    cats:        &[Category],
) -> Vec<String> {
    if col_format != ColFormat::NameOnly {
        let s = col_display_value(item_values, col_cat_id, col_format, cats);
        return if s.is_empty() { vec![] } else { vec![s] };
    }
    let mut parent_map = HashMap::new();
    let mut name_map   = HashMap::new();
    build_cat_maps(cats, None, &mut parent_map, &mut name_map);
    let is_at_or_under = |mut id: usize| -> bool {
        loop {
            if id == col_cat_id { return true; }
            match parent_map.get(&id) {
                Some(Some(p)) => id = *p,
                _             => return false,
            }
        }
    };
    item_values.keys().copied()
        .filter(|&id| is_at_or_under(id))
        .filter_map(|id| name_map.get(&id).cloned())
        .collect()
}

/// Return the category IDs of sub-categories under `col_cat_id` that the item is assigned to.
/// Used to resolve which sub-category a note belongs to (non-date columns).
pub fn item_col_assigned_cat_ids(
    item_values: &HashMap<usize, String>,
    col_cat_id:  usize,
    cats:        &[Category],
) -> Vec<usize> {
    let mut parent_map = HashMap::new();
    let mut name_map   = HashMap::new();
    build_cat_maps(cats, None, &mut parent_map, &mut name_map);
    let is_at_or_under = |mut id: usize| -> bool {
        loop {
            if id == col_cat_id { return true; }
            match parent_map.get(&id) {
                Some(Some(p)) => id = *p,
                _             => return false,
            }
        }
    };
    let mut ids: Vec<usize> = item_values.keys().copied()
        .filter(|&id| is_at_or_under(id))
        .collect();
    // Sort by name for stable ordering (matches col_display_values NameOnly order).
    ids.sort_by_key(|id| name_map.get(id).cloned().unwrap_or_default());
    ids
}

/// Find a descendant of the category with `head_id` whose name starts with `prefix`
/// (case-insensitive). Returns `(id, full_name)` of the first match.
pub fn col_autocomplete_match(cats: &[Category], head_id: usize, prefix: &str) -> Option<(usize, String)> {
    if prefix.is_empty() { return None; }
    let lower = prefix.to_lowercase();
    find_autocomplete_in(cats, head_id, &lower, false)
}

fn find_autocomplete_in(cats: &[Category], head_id: usize, lower: &str, in_subtree: bool) -> Option<(usize, String)> {
    for cat in cats {
        if in_subtree {
            if cat.name.to_lowercase().starts_with(lower) {
                return Some((cat.id, cat.name.clone()));
            }
            if let Some(m) = find_autocomplete_in(&cat.children, head_id, lower, true) {
                return Some(m);
            }
        } else if cat.id == head_id {
            return find_autocomplete_in(&cat.children, head_id, lower, true);
        } else if let Some(m) = find_autocomplete_in(&cat.children, head_id, lower, false) {
            return Some(m);
        }
    }
    None
}

/// Add a new direct child category under the category with `head_id`.
/// Returns true if the parent was found and the child was inserted.
pub fn add_child_to_cat(cats: &mut Vec<Category>, head_id: usize, new_id: usize, name: &str) -> bool {
    for cat in cats.iter_mut() {
        if cat.id == head_id {
            cat.children.push(Category {
                id: new_id, name: name.to_string(),
                kind: CategoryKind::Standard, children: vec![], note: String::new(),
                short_name: String::new(), also_match: String::new(), note_file: String::new(),
                excl_children: false, match_cat_name: true, match_short_name: true,
            });
            return true;
        }
        if add_child_to_cat(&mut cat.children, head_id, new_id, name) {
            return true;
        }
    }
    false
}

/// Compute the number of display rows an item occupies given the current columns.
/// For standard columns with multiple subcategory assignments, each assignment takes one row.
fn item_n_rows(item: &Item, columns: &[Column], cats: &[Category]) -> usize {
    columns.iter()
        .map(|c| {
            if c.date_fmt.is_some() {
                1
            } else {
                col_display_values(&item.values, c.cat_id, c.format, cats).len().max(1)
            }
        })
        .max()
        .unwrap_or(1)
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
            id:     1,
            name:   "Initial Section".to_string(),
            cat_id: 6,   // backed by the "Initial Section" standard category
        };
        let view = View {
            id:         1,
            name:       "Initial View".to_string(),
            sections:   vec![section],
            items:      Vec::new(),
            columns:    Vec::new(),
            left_count: 0,
        };

        fn date(id: usize, name: &str) -> Category {
            Category { id, name: name.to_string(), kind: CategoryKind::Date, children: vec![], note: String::new(),
                short_name: String::new(), also_match: String::new(), note_file: String::new(),
                excl_children: false, match_cat_name: true, match_short_name: true }
        }
        fn std(id: usize, name: &str) -> Category {
            Category { id, name: name.to_string(), kind: CategoryKind::Standard, children: vec![], note: String::new(),
                short_name: String::new(), also_match: String::new(), note_file: String::new(),
                excl_children: false, match_cat_name: true, match_short_name: true }
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
            note:             String::new(),
            short_name:       String::new(),
            also_match:       String::new(),
            note_file:        String::new(),
            excl_children:    false,
            match_cat_name:   true,
            match_short_name: true,
        };

        App {
            screen:     AppScreen::View,
            view,
            cursor:     CursorPos::SectionHead(0),
            mode:       Mode::Normal,
            categories: vec![main_cat],
            cat_state:  CatMgrState { cursor: 0, mode: CatMode::Normal },
            col_cursor:  0,
            col_mode:    ColMode::Normal,
            sub_row:     0,
            assign_mode: AssignMode::Normal,
            cat_search:  None,
            sec_mode:    SectionMode::Normal,
            menu:         MenuState::Closed,
            fkey_mod:     FKeyMod::Normal,
            pending_note: None,
            quit:         false,
            next_id:      7,
        }
    }

    /// Resolve a (section, local_item_pos) cursor to a global index into `view.items`.
    fn global_item_idx(&self, sec: usize, local: usize) -> Option<usize> {
        section_item_indices(&self.view, sec, &self.categories).get(local).copied()
    }

    fn alloc_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    // ── Note ──────────────────────────────────────────────────────────────────

    /// Set `pending_note` for the currently highlighted item or section-backing category.
    /// The main loop will suspend the TUI, open $EDITOR, and call `apply_note`.
    pub fn open_note(&mut self) {
        if matches!(self.mode, Mode::Edit { .. } | Mode::Create { .. }) { return; }

        // CatMgr: note for the currently selected category.
        if self.screen == AppScreen::CatMgr {
            let flat = flatten_cats(&self.categories);
            if let Some(entry) = flat.get(self.cat_state.cursor) {
                self.pending_note = Some(NoteTarget::Cat(entry.id));
            }
            return;
        }

        let target = if self.col_cursor > 0 {
            if let Some(col) = self.view.columns.get(self.col_cursor - 1) {
                if col.date_fmt.is_some() {
                    // Date column: note belongs to the category itself.
                    Some(NoteTarget::Cat(col.cat_id))
                } else if let CursorPos::Item { section, item } = self.cursor {
                    // Non-date column on an item: note belongs to the specific sub-category
                    // the item is assigned to at sub_row.
                    let gi = self.global_item_idx(section, item);
                    let sub_cat_id = gi.and_then(|gi| {
                        let ids = item_col_assigned_cat_ids(
                            &self.view.items[gi].values, col.cat_id, &self.categories,
                        );
                        ids.get(self.sub_row).copied()
                    });
                    sub_cat_id.map(NoteTarget::Cat)
                } else {
                    // Non-date column on section head: note for the column's category.
                    Some(NoteTarget::Cat(col.cat_id))
                }
            } else {
                None
            }
        } else {
            match self.cursor {
                CursorPos::Item { section, item } => {
                    self.global_item_idx(section, item).map(NoteTarget::Item)
                }
                CursorPos::SectionHead(s) => {
                    self.view.sections.get(s).map(|sec| NoteTarget::Cat(sec.cat_id))
                }
            }
        };
        self.pending_note = target;
    }

    /// Return a short label (item text or category name) for use in a temp filename.
    pub fn get_note_label(&self, target: &NoteTarget) -> String {
        let raw = match target {
            NoteTarget::Item(gi) => {
                self.view.items.get(*gi).map(|i| i.text.as_str()).unwrap_or("item").to_string()
            }
            NoteTarget::Cat(cat_id) => {
                find_cat_note_name(&self.categories, *cat_id)
                    .unwrap_or_else(|| "category".to_string())
            }
        };
        // Sanitize: keep alphanumeric/hyphen/underscore, replace the rest with '_', cap at 32 chars.
        raw.chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .take(32)
            .collect::<String>()
            .trim_matches('_')
            .to_string()
    }

    /// Return the current note text for `target`.
    pub fn get_note_content(&self, target: &NoteTarget) -> String {
        match target {
            NoteTarget::Item(gi) => {
                self.view.items.get(*gi).map(|i| i.note.clone()).unwrap_or_default()
            }
            NoteTarget::Cat(cat_id) => {
                find_cat_note(&self.categories, *cat_id).unwrap_or_default()
            }
        }
    }

    /// Write the edited note back to the data model.
    pub fn apply_note(&mut self, target: NoteTarget, content: String) {
        match target {
            NoteTarget::Item(gi) => {
                if let Some(item) = self.view.items.get_mut(gi) {
                    item.note = content;
                }
            }
            NoteTarget::Cat(cat_id) => {
                set_cat_note(&mut self.categories, cat_id, content);
            }
        }
    }

    // ── Screen toggle ─────────────────────────────────────────────────────────

    pub fn toggle_catmgr(&mut self) {
        self.cat_state.mode = CatMode::Normal;
        self.mode            = Mode::Normal;
        if self.screen == AppScreen::View {
            // Position the CatMgr cursor on the contextually relevant category.
            let cat_id = if self.col_cursor > 0 {
                self.view.columns.get(self.col_cursor - 1).map(|c| c.cat_id)
            } else {
                match self.cursor {
                    CursorPos::SectionHead(s) =>
                        self.view.sections.get(s).map(|s| s.cat_id),
                    CursorPos::Item { section, .. } =>
                        self.view.sections.get(section).map(|s| s.cat_id),
                }
            };
            if let Some(id) = cat_id {
                let flat = flatten_cats(&self.categories);
                if let Some(pos) = flat.iter().position(|e| e.id == id) {
                    self.cat_state.cursor = pos;
                }
            }
        }
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
            MenuAction::SectionAdd       => self.sec_open_add(SectionInsert::Below),
            MenuAction::SectionRemove    => self.sec_open_confirm_remove(),
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
        if self.col_cursor > 0 && self.sub_row > 0 {
            self.sub_row -= 1;
            return;
        }
        let new_cursor = match &self.cursor {
            CursorPos::SectionHead(0) => CursorPos::SectionHead(0),
            CursorPos::SectionHead(s) => {
                let prev = s - 1;
                let n = section_item_indices(&self.view, prev, &self.categories).len();
                if n == 0 { CursorPos::SectionHead(prev) }
                else      { CursorPos::Item { section: prev, item: n - 1 } }
            }
            CursorPos::Item { section, item: 0 } => CursorPos::SectionHead(*section),
            CursorPos::Item { section, item }    => CursorPos::Item { section: *section, item: item - 1 },
        };
        if self.col_cursor > 0 {
            if let CursorPos::Item { section: s, item: i } = new_cursor {
                if let Some(gi) = section_item_indices(&self.view, s, &self.categories).get(i).copied() {
                    let n = item_n_rows(&self.view.items[gi], &self.view.columns, &self.categories);
                    self.sub_row = n.saturating_sub(1);
                } else {
                    self.sub_row = 0;
                }
            } else {
                self.sub_row = 0;
            }
        }
        self.cursor = new_cursor;
    }

    pub fn cursor_down(&mut self) {
        if !matches!(self.mode, Mode::Normal) { return; }
        if self.col_cursor > 0 {
            if let CursorPos::Item { section: s, item: i } = &self.cursor {
                let (s, i) = (*s, *i);
                if let Some(gi) = section_item_indices(&self.view, s, &self.categories).get(i).copied() {
                    let n = item_n_rows(&self.view.items[gi], &self.view.columns, &self.categories);
                    if self.sub_row + 1 < n {
                        self.sub_row += 1;
                        return;
                    }
                }
            }
        }
        self.sub_row = 0;
        let num_sections = self.view.sections.len();
        self.cursor = match &self.cursor {
            CursorPos::SectionHead(s) => {
                let s = *s;
                let n = section_item_indices(&self.view, s, &self.categories).len();
                if n == 0 {
                    if s + 1 < num_sections { CursorPos::SectionHead(s + 1) }
                    else                    { CursorPos::SectionHead(s) }
                } else {
                    CursorPos::Item { section: s, item: 0 }
                }
            }
            CursorPos::Item { section, item } => {
                let s = *section;
                let i = *item;
                let num_items = section_item_indices(&self.view, s, &self.categories).len();
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
            _ => {}
        }
    }

    pub fn edit_cursor_right(&mut self) {
        match &mut self.mode {
            Mode::Edit { buffer, cursor, .. } | Mode::Create { buffer, cursor } => {
                let len = buffer.chars().count();
                if *cursor < len { *cursor += 1; }
            }
            _ => {}
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
                    let gi       = self.global_item_idx(*section, *item).unwrap_or(usize::MAX);
                    let original = self.view.items.get(gi)
                        .and_then(|it| it.values.get(&cat_id)).cloned().unwrap_or_default();
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
                CursorPos::Item { section, item } => {
                    let gi = self.global_item_idx(*section, *item).unwrap_or(usize::MAX);
                    self.view.items.get(gi).map(|it| it.text.clone()).unwrap_or_default()
                }
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
                    let gi  = self.global_item_idx(*section, *item).unwrap_or(usize::MAX);
                    let val = self.view.items.get(gi)
                        .and_then(|it| it.values.get(&cat_id)).cloned().unwrap_or_default();
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
            _ => return,
        };
        let byte_pos = char_to_byte(buffer, *cursor);
        buffer.insert(byte_pos, ch);
        *cursor += 1;
    }

    pub fn input_backspace(&mut self) {
        let (buffer, cursor) = match &mut self.mode {
            Mode::Edit   { buffer, cursor, .. } => (buffer, cursor),
            Mode::Create { buffer, cursor }     => (buffer, cursor),
            _ => return,
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
            _ => return,
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
                // Auto-stamp the Entry category with the creation datetime (conditional).
                let mut values    = HashMap::new();
                let mut cond_cats = std::collections::HashSet::new();
                let flat = flatten_cats(&self.categories);
                if let Some(entry) = flat.iter().find(|c| c.name == "Entry") {
                    values.insert(entry.id, now_datetime_string());
                    cond_cats.insert(entry.id);
                }
                // Auto-assign item to the section's backing category.
                values.entry(self.view.sections[sec_idx].cat_id).or_insert_with(String::new);
                // Determine global insert position: after local item `insert_after`, or at end.
                let indices = section_item_indices(&self.view, sec_idx, &self.categories);
                let global_pos = match insert_after {
                    Some(local) => indices.get(local).map(|&g| g + 1).unwrap_or(self.view.items.len()),
                    None        => indices.first().copied().unwrap_or(self.view.items.len()),
                };
                self.view.items.insert(global_pos, Item { id, text, values, cond_cats, note: String::new() });
                // Local index is position within section after insertion.
                let new_local = section_item_indices(&self.view, sec_idx, &self.categories)
                    .iter().position(|&g| g == global_pos).unwrap_or(0);
                self.cursor = CursorPos::Item { section: sec_idx, item: new_local };
            }
            Mode::Edit { buffer, col, .. } => {
                let text = buffer.trim().to_string();
                if col == 0 {
                    if text.is_empty() { return; }
                    match &self.cursor {
                        CursorPos::SectionHead(s) => { self.view.sections[*s].name = text; }
                        CursorPos::Item { section, item } => {
                            if let Some(gi) = self.global_item_idx(*section, *item) {
                                self.view.items[gi].text = text;
                            }
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
                            let gi = self.global_item_idx(s, i);
                            if is_date {
                                // Normalize date values to YYYY-MM-DD HH:MM:SS
                                let final_text = if !text.is_empty() {
                                    if let Some((y, mo, d, h, mi, sec)) = parse_date_input(&text, fmt_code) {
                                        format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", y, mo, d, h, mi, sec)
                                    } else {
                                        text.clone()
                                    }
                                } else {
                                    text.clone()
                                };
                                if let Some(gi) = gi {
                                    if final_text.is_empty() {
                                        self.view.items[gi].values.remove(&cat_id);
                                    } else {
                                        self.view.items[gi].values.insert(cat_id, final_text);
                                    }
                                }
                            } else if !text.is_empty() {
                                // Standard column: find existing subcategory or create new one
                                let sub_id = match col_autocomplete_match(&self.categories, cat_id, &text) {
                                    Some((mid, _)) => mid,
                                    None => {
                                        let new_id = self.alloc_id();
                                        add_child_to_cat(&mut self.categories, cat_id, new_id, &text);
                                        new_id
                                    }
                                };
                                if let Some(gi) = gi {
                                    self.view.items[gi].values.insert(sub_id, String::new());
                                }
                            }
                        }
                    }
                }
            }
            Mode::Normal | Mode::ConfirmDeleteItem { .. } => {}
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

    pub fn col_open_confirm_remove(&mut self) {
        if self.col_cursor == 0 || self.view.columns.is_empty() { return; }
        if !matches!(self.cursor, CursorPos::SectionHead(_)) { return; }
        self.col_mode = ColMode::ConfirmRemove { yes: true };
    }

    pub fn col_confirm_remove_toggle(&mut self) {
        if let ColMode::ConfirmRemove { yes } = &mut self.col_mode {
            *yes = !*yes;
        }
    }

    pub fn col_confirm_remove_confirm(&mut self) {
        if let ColMode::ConfirmRemove { yes } = self.col_mode {
            self.col_mode = ColMode::Normal;
            if yes { self.col_delete(); }
        }
    }

    pub fn col_confirm_remove_cancel(&mut self) {
        self.col_mode = ColMode::Normal;
    }

    // ── Item delete confirmation ──────────────────────────────────────────────

    /// Open "Remove this item from the section?" dialog.
    /// Only valid when the cursor is on an Item in the main column.
    pub fn item_open_confirm_delete(&mut self) {
        if self.col_cursor != 0 { return; }
        if !matches!(self.cursor, CursorPos::Item { .. }) { return; }
        if !matches!(self.mode, Mode::Normal) { return; }
        self.mode = Mode::ConfirmDeleteItem { yes: true };
    }

    pub fn item_confirm_delete_toggle(&mut self) {
        if let Mode::ConfirmDeleteItem { yes } = &mut self.mode {
            *yes = !*yes;
        }
    }

    pub fn item_confirm_delete_confirm(&mut self) {
        if let Mode::ConfirmDeleteItem { yes } = self.mode {
            self.mode = Mode::Normal;
            if yes { self.item_remove(); }
        }
    }

    pub fn item_confirm_delete_cancel(&mut self) {
        if matches!(self.mode, Mode::ConfirmDeleteItem { .. }) {
            self.mode = Mode::Normal;
        }
    }

    /// Remove the currently focused item from its section and adjust the cursor.
    pub fn item_remove(&mut self) {
        let (s, i) = match self.cursor {
            CursorPos::Item { section, item } => (section, item),
            _ => return,
        };
        if s >= self.view.sections.len() { return; }
        let Some(gi) = self.global_item_idx(s, i) else { return; };
        self.view.items.remove(gi);
        // Move cursor to the item above, or the section head if none remain.
        self.cursor = if i > 0 {
            CursorPos::Item { section: s, item: i - 1 }
        } else {
            CursorPos::SectionHead(s)
        };
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
        self.view.columns.insert(pos, Column { id, name, cat_id, width, format: ColFormat::NameOnly, date_fmt });
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
                self.view.columns.insert(pos, Column { id, name, cat_id, width, format: ColFormat::NameOnly, date_fmt });
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
        let format    = col.format;
        self.col_mode = ColMode::Props {
            head_buf, head_cur, width_buf, width_cur,
            format, date_fmt, active_field: PropsField::Head, is_date,
        };
    }

    pub fn col_props_confirm(&mut self) {
        let old = std::mem::replace(&mut self.col_mode, ColMode::Normal);
        if let ColMode::Props { head_buf, width_buf, format, date_fmt, .. } = old {
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
            self.view.columns[idx].format   = format;
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
                PropsField::Width       => if is_date { PropsField::DateDisplay } else { PropsField::Format },
                PropsField::Format      => PropsField::Head,
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
                PropsField::Head        => if is_date { PropsField::TimeSep } else { PropsField::Format },
                PropsField::Width       => PropsField::Head,
                PropsField::Format      => PropsField::Width,
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
                                ref mut format, ref mut date_fmt, .. } = self.col_mode {
            match active_field {
                PropsField::Head   => { if *head_cur > 0 { *head_cur -= 1; } }
                PropsField::Width  => { if *width_cur > 0 { *width_cur -= 1; } }
                PropsField::Format => { *format = col_format_prev(*format); }
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
                                ref mut width_cur, ref width_buf,
                                ref mut format, ref mut date_fmt, .. } = self.col_mode {
            match active_field {
                PropsField::Head  => {
                    let len = head_buf.chars().count();
                    if *head_cur < len { *head_cur += 1; }
                }
                PropsField::Width => {
                    let len = width_buf.chars().count();
                    if *width_cur < len { *width_cur += 1; }
                }
                PropsField::Format => { *format = col_format_next(*format); }
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
        self.sub_row = 0;
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
        self.sub_row = 0;
        let n  = self.view.columns.len();
        let lc = self.view.left_count;
        self.col_cursor = match self.col_cursor {
            0           => if lc + 1 <= n { lc + 1 } else { 0 },  // main → first right
            c if c == lc && lc > 0 => 0,  // last left col → main
            c if c < n  => c + 1,          // within left or within right
            _           => self.col_cursor, // already rightmost
        };
    }

    // ── Assignment Profile ────────────────────────────────────────────────────

    /// Open the Assignment Profile for the currently highlighted item.
    /// No-op if cursor is not on an Item in the main column.
    pub fn assign_open(&mut self) {
        if self.col_cursor != 0 { return; }
        if let CursorPos::Item { section, item } = self.cursor {
            if let Some(gi) = self.global_item_idx(section, item) {
                self.assign_mode = AssignMode::Profile { gi, cursor: 0, on_sub: false };
            }
        }
    }

    pub fn assign_close(&mut self) {
        self.assign_mode = AssignMode::Normal;
        self.cat_search = None;
    }

    pub fn assign_cursor_up(&mut self) {
        let cats = flatten_cats(&self.categories);
        let (gi, cur, sub) = match &self.assign_mode {
            AssignMode::Profile { gi, cursor, on_sub } => (*gi, *cursor, *on_sub),
            AssignMode::Normal => return,
        };
        let empty = std::collections::HashMap::new();
        let item_vals = self.view.items.get(gi).map(|it| &it.values).unwrap_or(&empty);
        let (new_cur, new_sub) = if sub {
            (cur, false)   // sub-row → its parent cat
        } else if cur > 0 {
            let prev = cur - 1;
            let prev_has_sub = cats.get(prev).map(|e|
                e.kind == CategoryKind::Date
                && item_vals.get(&e.id).map_or(false, |v| !v.is_empty())
            ).unwrap_or(false);
            (prev, prev_has_sub)
        } else {
            (cur, false)
        };
        if let AssignMode::Profile { cursor, on_sub, .. } = &mut self.assign_mode {
            *cursor = new_cur;
            *on_sub = new_sub;
        }
    }

    pub fn assign_cursor_down(&mut self) {
        let cats = flatten_cats(&self.categories);
        let len = cats.len();
        let (gi, cur, sub) = match &self.assign_mode {
            AssignMode::Profile { gi, cursor, on_sub } => (*gi, *cursor, *on_sub),
            AssignMode::Normal => return,
        };
        let empty = std::collections::HashMap::new();
        let item_vals = self.view.items.get(gi).map(|it| &it.values).unwrap_or(&empty);
        let (new_cur, new_sub) = if sub {
            ((cur + 1).min(len.saturating_sub(1)), false)
        } else {
            let cur_has_sub = cats.get(cur).map(|e|
                e.kind == CategoryKind::Date
                && item_vals.get(&e.id).map_or(false, |v| !v.is_empty())
            ).unwrap_or(false);
            if cur_has_sub { (cur, true) } else { ((cur + 1).min(len.saturating_sub(1)), false) }
        };
        if let AssignMode::Profile { cursor, on_sub, .. } = &mut self.assign_mode {
            *cursor = new_cur;
            *on_sub = new_sub;
        }
    }

    pub fn assign_cursor_pgup(&mut self, page: usize) {
        if let AssignMode::Profile { cursor, on_sub, .. } = &mut self.assign_mode {
            *cursor = cursor.saturating_sub(page);
            *on_sub = false;
        }
    }

    pub fn assign_cursor_pgdn(&mut self, page: usize) {
        let len = flatten_cats(&self.categories).len();
        if let AssignMode::Profile { cursor, on_sub, .. } = &mut self.assign_mode {
            if len > 0 { *cursor = (*cursor + page).min(len - 1); }
            *on_sub = false;
        }
    }

    pub fn assign_cursor_home(&mut self) {
        if let AssignMode::Profile { cursor, on_sub, .. } = &mut self.assign_mode {
            *cursor = 0;
            *on_sub = false;
        }
    }

    pub fn assign_cursor_end(&mut self) {
        let len = flatten_cats(&self.categories).len();
        if let AssignMode::Profile { cursor, on_sub, .. } = &mut self.assign_mode {
            if len > 0 { *cursor = len - 1; }
            *on_sub = false;
        }
    }

    /// Toggle the assignment of the highlighted category for the current item.
    /// Standard categories: presence in item.values (empty string) = assigned.
    /// Date categories: assigned when a value string is present.
    pub fn assign_toggle(&mut self) {
        let (gi, cur) = match &self.assign_mode {
            AssignMode::Profile { gi, cursor, on_sub } => {
                if *on_sub { return; }  // sub-rows are not directly toggleable
                (*gi, *cursor)
            }
            AssignMode::Normal => return,
        };
        let cats = flatten_cats(&self.categories);
        let Some(entry) = cats.get(cur) else { return };
        let cat_id = entry.id;
        if gi >= self.view.items.len() { return; }
        let item = &mut self.view.items[gi];
        if item.values.contains_key(&cat_id) {
            item.values.remove(&cat_id);
        } else {
            item.values.insert(cat_id, String::new());
        }
    }

    // ── Category search ───────────────────────────────────────────────────────

    /// Returns the indices of all cats whose names contain `buf` (case-insensitive).
    fn cat_search_matches(cats: &[FlatCat], buf: &str) -> Vec<usize> {
        if buf.is_empty() { return vec![]; }
        let lower = buf.to_lowercase();
        cats.iter().enumerate()
            .filter(|(_, e)| e.name.to_lowercase().contains(&lower))
            .map(|(i, _)| i)
            .collect()
    }

    /// Active cat-list cursor regardless of which window is visible.
    fn active_cat_cursor(&self) -> usize {
        match &self.assign_mode {
            AssignMode::Profile { cursor, .. } => *cursor,
            AssignMode::Normal => self.cat_state.cursor,
        }
    }

    /// Set the active cat-list cursor.
    fn set_active_cat_cursor(&mut self, idx: usize) {
        match &mut self.assign_mode {
            AssignMode::Profile { cursor, on_sub, .. } => { *cursor = idx; *on_sub = false; }
            AssignMode::Normal => { self.cat_state.cursor = idx; }
        }
    }

    /// Move to the first match after appending `ch` to the search buffer.
    pub fn cat_search_char(&mut self, ch: char) {
        let buf = self.cat_search.get_or_insert_with(String::new);
        buf.push(ch);
        let buf = buf.clone();
        let cats = flatten_cats(&self.categories);
        let matches = Self::cat_search_matches(&cats, &buf);
        if let Some(&first) = matches.first() {
            self.set_active_cat_cursor(first);
        }
    }

    /// Remove last char from search buffer; clear search when empty.
    pub fn cat_search_backspace(&mut self) {
        if let Some(buf) = &mut self.cat_search {
            buf.pop();
            if buf.is_empty() {
                self.cat_search = None;
                return;
            }
            let buf = buf.clone();
            let cats = flatten_cats(&self.categories);
            let matches = Self::cat_search_matches(&cats, &buf);
            if let Some(&first) = matches.first() {
                self.set_active_cat_cursor(first);
            }
        }
    }

    /// Clear search without moving the cursor.
    pub fn cat_search_clear(&mut self) {
        self.cat_search = None;
    }

    /// Move to the next match (F8).
    pub fn cat_search_next(&mut self) {
        let buf = match &self.cat_search { Some(b) => b.clone(), None => return };
        let cats = flatten_cats(&self.categories);
        let matches = Self::cat_search_matches(&cats, &buf);
        if matches.is_empty() { return; }
        let cur = self.active_cat_cursor();
        let next = matches.iter().find(|&&i| i > cur)
            .copied()
            .unwrap_or(matches[0]);
        self.set_active_cat_cursor(next);
    }

    /// Move to the previous match (F7).
    pub fn cat_search_prev(&mut self) {
        let buf = match &self.cat_search { Some(b) => b.clone(), None => return };
        let cats = flatten_cats(&self.categories);
        let matches = Self::cat_search_matches(&cats, &buf);
        if matches.is_empty() { return; }
        let cur = self.active_cat_cursor();
        let prev = matches.iter().rev().find(|&&i| i < cur)
            .copied()
            .unwrap_or(*matches.last().unwrap());
        self.set_active_cat_cursor(prev);
    }

    // ── Section Add ──────────────────────────────────────────────────────────

    pub fn sec_open_add(&mut self, insert: SectionInsert) {
        self.sec_mode = SectionMode::Add {
            cat_idx:      None,
            insert,
            active_field: SectionFormField::Category,
        };
    }

    pub fn sec_form_confirm(&mut self) {
        let (cat_idx, insert) = match &self.sec_mode {
            SectionMode::Add { cat_idx, insert, .. } => (*cat_idx, *insert),
            _ => return,
        };
        self.sec_mode = SectionMode::Normal;
        let flat_cats = flatten_cats(&self.categories);
        let Some(entry) = cat_idx.and_then(|i| flat_cats.get(i)) else { return; };
        let name    = entry.name.clone();
        let sec_cat = entry.id;
        let id      = self.alloc_id();
        let cur_sec = match self.cursor {
            CursorPos::SectionHead(s)      => s,
            CursorPos::Item { section, .. } => section,
        };
        let insert_idx = match insert {
            SectionInsert::Below => cur_sec + 1,
            SectionInsert::Above => cur_sec,
        };

        self.view.sections.insert(insert_idx, Section {
            id,
            name,
            cat_id: sec_cat,
        });
        self.cursor = CursorPos::SectionHead(insert_idx);
    }

    pub fn sec_form_cancel(&mut self) {
        self.sec_mode = SectionMode::Normal;
    }

    // ── Section remove ────────────────────────────────────────────────────────

    /// Open "Remove this section?" confirmation. Only valid on a SectionHead.
    pub fn sec_open_confirm_remove(&mut self) {
        if !matches!(self.mode, Mode::Normal) { return; }
        if !matches!(self.cursor, CursorPos::SectionHead(_)) { return; }
        if self.view.sections.len() <= 1 { return; }   // must keep at least one section
        self.sec_mode = SectionMode::ConfirmRemove { yes: true };
    }

    pub fn sec_confirm_remove_toggle(&mut self) {
        if let SectionMode::ConfirmRemove { yes } = &mut self.sec_mode {
            *yes = !*yes;
        }
    }

    pub fn sec_confirm_remove_confirm(&mut self) {
        if let SectionMode::ConfirmRemove { yes } = self.sec_mode {
            self.sec_mode = SectionMode::Normal;
            if yes { self.sec_remove(); }
        }
    }

    pub fn sec_confirm_remove_cancel(&mut self) {
        if matches!(self.sec_mode, SectionMode::ConfirmRemove { .. }) {
            self.sec_mode = SectionMode::Normal;
        }
    }

    /// Remove the currently focused section; items in it remain in the global pool.
    fn sec_remove(&mut self) {
        let s = match self.cursor {
            CursorPos::SectionHead(s) => s,
            _ => return,
        };
        if s >= self.view.sections.len() || self.view.sections.len() <= 1 { return; }
        self.view.sections.remove(s);
        // Move cursor to an adjacent section head.
        let new_s = if s > 0 { s - 1 } else { 0 };
        self.cursor = CursorPos::SectionHead(new_s);
    }

    pub fn sec_form_field_next(&mut self) {
        if let SectionMode::Add { active_field, .. } = &mut self.sec_mode {
            *active_field = match active_field {
                SectionFormField::Category => SectionFormField::Insert,
                SectionFormField::Insert   => SectionFormField::Category,
            };
        }
    }

    pub fn sec_form_field_prev(&mut self) {
        self.sec_form_field_next(); // only 2 fields, same as next
    }

    pub fn sec_form_toggle_insert(&mut self) {
        if let SectionMode::Add { insert, .. } = &mut self.sec_mode {
            *insert = match insert {
                SectionInsert::Below => SectionInsert::Above,
                SectionInsert::Above => SectionInsert::Below,
            };
        }
    }

    pub fn sec_form_left(&mut self) {
        if let SectionMode::Add { active_field: SectionFormField::Insert, .. } = &self.sec_mode {
            self.sec_form_toggle_insert();
        }
    }

    pub fn sec_form_right(&mut self) {
        if let SectionMode::Add { active_field: SectionFormField::Insert, .. } = &self.sec_mode {
            self.sec_form_toggle_insert();
        }
    }

    pub fn sec_open_choices(&mut self) {
        if let SectionMode::Add { cat_idx, insert, active_field } = &self.sec_mode {
            if *active_field == SectionFormField::Category {
                let cursor = cat_idx.unwrap_or(0);
                self.sec_mode = SectionMode::Choices {
                    cat_idx:       *cat_idx,
                    insert:        *insert,
                    active_field:  SectionFormField::Category,
                    picker_cursor: cursor,
                };
            }
        }
    }

    pub fn sec_choices_up(&mut self) {
        if let SectionMode::Choices { picker_cursor, .. } = &mut self.sec_mode {
            if *picker_cursor > 0 { *picker_cursor -= 1; }
        }
    }

    pub fn sec_choices_down(&mut self) {
        let len = flatten_cats(&self.categories).len();
        if let SectionMode::Choices { picker_cursor, .. } = &mut self.sec_mode {
            if *picker_cursor + 1 < len { *picker_cursor += 1; }
        }
    }

    pub fn sec_choices_confirm(&mut self) {
        if let SectionMode::Choices { cat_idx: _, insert, active_field, picker_cursor } = &self.sec_mode {
            let insert       = *insert;
            let active_field = *active_field;
            let picked       = *picker_cursor;
            self.sec_mode = SectionMode::Add {
                cat_idx:      Some(picked),
                insert,
                active_field,
            };
        }
    }

    pub fn sec_choices_cancel(&mut self) {
        if let SectionMode::Choices { cat_idx, insert, active_field, .. } = &self.sec_mode {
            let cat_idx      = *cat_idx;
            let insert       = *insert;
            let active_field = *active_field;
            self.sec_mode = SectionMode::Add { cat_idx, insert, active_field };
        }
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

    pub fn cat_cursor_pgup(&mut self, page: usize) {
        if !matches!(self.cat_state.mode, CatMode::Normal) { return; }
        self.cat_state.cursor = self.cat_state.cursor.saturating_sub(page);
    }

    pub fn cat_cursor_pgdn(&mut self, page: usize) {
        if !matches!(self.cat_state.mode, CatMode::Normal) { return; }
        let flat = flatten_cats(&self.categories);
        if flat.is_empty() { return; }
        self.cat_state.cursor = (self.cat_state.cursor + page).min(flat.len() - 1);
    }

    pub fn cat_cursor_home(&mut self) {
        if !matches!(self.cat_state.mode, CatMode::Normal) { return; }
        self.cat_state.cursor = 0;
    }

    pub fn cat_cursor_end(&mut self) {
        if !matches!(self.cat_state.mode, CatMode::Normal) { return; }
        let flat = flatten_cats(&self.categories);
        if !flat.is_empty() { self.cat_state.cursor = flat.len() - 1; }
    }

    // ── CatMgr buffer cursor ──────────────────────────────────────────────────

    pub fn cat_edit_cursor_left(&mut self) {
        match &mut self.cat_state.mode {
            CatMode::Edit { cursor, .. } | CatMode::Create { cursor, .. } => {
                if *cursor > 0 { *cursor -= 1; }
            }
            CatMode::Normal | CatMode::Props { .. } => {}
        }
    }

    pub fn cat_edit_cursor_right(&mut self) {
        match &mut self.cat_state.mode {
            CatMode::Edit { buffer, cursor } | CatMode::Create { buffer, cursor, .. } => {
                let len = buffer.chars().count();
                if *cursor < len { *cursor += 1; }
            }
            CatMode::Normal | CatMode::Props { .. } => {}
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
            CatMode::Normal | CatMode::Props { .. } => return,
        };
        let byte_pos = char_to_byte(buffer, *cursor);
        buffer.insert(byte_pos, ch);
        *cursor += 1;
    }

    pub fn cat_input_backspace(&mut self) {
        let (buffer, cursor) = match &mut self.cat_state.mode {
            CatMode::Edit   { buffer, cursor }     => (buffer, cursor),
            CatMode::Create { buffer, cursor, .. } => (buffer, cursor),
            CatMode::Normal | CatMode::Props { .. } => return,
        };
        if *cursor > 0 {
            *cursor -= 1;
            let byte_pos = char_to_byte(buffer, *cursor);
            buffer.remove(byte_pos);
        }
    }

    pub fn cat_input_delete(&mut self) {
        let (buffer, cursor) = match &mut self.cat_state.mode {
            CatMode::Edit   { buffer, cursor }     => (buffer, cursor),
            CatMode::Create { buffer, cursor, .. } => (buffer, cursor),
            CatMode::Normal | CatMode::Props { .. } => return,
        };
        let len = buffer.chars().count();
        if *cursor < len {
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
                let cat = Category { id, name: text, kind: CategoryKind::Standard, children: vec![], note: String::new(),
                    short_name: String::new(), also_match: String::new(), note_file: String::new(),
                    excl_children: false, match_cat_name: true, match_short_name: true };
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
            CatMode::Normal | CatMode::Props { .. } => {}
        }
    }

    pub fn cat_cancel(&mut self) {
        self.cat_state.mode = CatMode::Normal;
    }

    // ── Category Properties modal ─────────────────────────────────────────────

    pub fn cat_open_props(&mut self) {
        let flat = flatten_cats(&self.categories);
        if flat.is_empty() { return; }
        let idx    = self.cat_state.cursor.min(flat.len() - 1);
        let cat_id = flat[idx].id;
        let kind   = flat[idx].kind;

        let (name, short_name, also_match, note_file, excl_children, match_cat_name, match_short_name) = {
            match find_cat_by_id(&self.categories, cat_id) {
                Some(cat) => (
                    cat.name.clone(),
                    cat.short_name.clone(),
                    cat.also_match.clone(),
                    cat.note_file.clone(),
                    cat.excl_children,
                    cat.match_cat_name,
                    cat.match_short_name,
                ),
                None => return,
            }
        };
        let has_note    = !cat_note_for_id(&self.categories, cat_id).is_empty();
        let parent_name = find_cat_parent_name(&self.categories, cat_id, None)
            .unwrap_or_else(|| "(top level)".to_string());
        let name_cur         = name.chars().count();
        let short_name_cur   = short_name.chars().count();
        let also_match_cur   = also_match.chars().count();
        let note_file_cur    = note_file.chars().count();
        self.cat_state.mode = CatMode::Props {
            name_buf: name, name_cur,
            short_name_buf: short_name, short_name_cur,
            also_match_buf: also_match, also_match_cur,
            note_file_buf: note_file, note_file_cur,
            excl_children, match_cat_name, match_short_name,
            active_field: CatPropsField::Name,
            parent_name, kind, has_note, cat_id,
        };
    }

    pub fn cat_props_confirm(&mut self) {
        let old = std::mem::replace(&mut self.cat_state.mode, CatMode::Normal);
        let CatMode::Props {
            name_buf, short_name_buf, also_match_buf, note_file_buf,
            excl_children, match_cat_name, match_short_name, cat_id, ..
        } = old else { return };
        let name = name_buf.trim().to_string();
        if let Some(cat) = find_cat_by_id_mut(&mut self.categories, cat_id) {
            if !name.is_empty() { cat.name = name; }
            cat.short_name       = short_name_buf;
            cat.also_match       = also_match_buf;
            cat.note_file        = note_file_buf;
            cat.excl_children    = excl_children;
            cat.match_cat_name   = match_cat_name;
            cat.match_short_name = match_short_name;
        }
    }

    pub fn cat_props_cancel(&mut self) {
        self.cat_state.mode = CatMode::Normal;
    }

    /// Open the note editor for the category shown in the Props modal.
    /// Only acts when the active field is Note.
    pub fn cat_props_open_editor(&mut self) {
        if matches!(&self.cat_state.mode, CatMode::Props { active_field: CatPropsField::Note, .. }) {
            self.open_note();
        }
    }

    pub fn cat_props_field_next(&mut self) {
        if let CatMode::Props { active_field, .. } = &mut self.cat_state.mode {
            *active_field = match *active_field {
                CatPropsField::Name           => CatPropsField::ShortName,
                CatPropsField::ShortName      => CatPropsField::AlsoMatch,
                CatPropsField::AlsoMatch      => CatPropsField::Note,
                CatPropsField::Note           => CatPropsField::NoteFile,
                CatPropsField::NoteFile       => CatPropsField::ExclChildren,
                CatPropsField::ExclChildren   => CatPropsField::MatchCatName,
                CatPropsField::MatchCatName   => CatPropsField::MatchShortName,
                CatPropsField::MatchShortName => CatPropsField::Name,
            };
        }
    }

    pub fn cat_props_field_prev(&mut self) {
        if let CatMode::Props { active_field, .. } = &mut self.cat_state.mode {
            *active_field = match *active_field {
                CatPropsField::Name           => CatPropsField::MatchShortName,
                CatPropsField::ShortName      => CatPropsField::Name,
                CatPropsField::AlsoMatch      => CatPropsField::ShortName,
                CatPropsField::Note           => CatPropsField::AlsoMatch,
                CatPropsField::NoteFile       => CatPropsField::Note,
                CatPropsField::ExclChildren   => CatPropsField::NoteFile,
                CatPropsField::MatchCatName   => CatPropsField::ExclChildren,
                CatPropsField::MatchShortName => CatPropsField::MatchCatName,
            };
        }
    }

    pub fn cat_props_input_char(&mut self, ch: char) {
        let CatMode::Props {
            active_field, name_buf, name_cur, short_name_buf, short_name_cur,
            also_match_buf, also_match_cur, note_file_buf, note_file_cur, ..
        } = &mut self.cat_state.mode else { return };
        let (buf, cur) = match active_field {
            CatPropsField::Name      => (name_buf,       name_cur),
            CatPropsField::ShortName => (short_name_buf, short_name_cur),
            CatPropsField::AlsoMatch => (also_match_buf, also_match_cur),
            CatPropsField::NoteFile  => (note_file_buf,  note_file_cur),
            _ => return,
        };
        let byte = char_to_byte(buf, *cur);
        buf.insert(byte, ch);
        *cur += 1;
    }

    pub fn cat_props_backspace(&mut self) {
        let CatMode::Props {
            active_field, name_buf, name_cur, short_name_buf, short_name_cur,
            also_match_buf, also_match_cur, note_file_buf, note_file_cur, ..
        } = &mut self.cat_state.mode else { return };
        let (buf, cur) = match active_field {
            CatPropsField::Name      => (name_buf,       name_cur),
            CatPropsField::ShortName => (short_name_buf, short_name_cur),
            CatPropsField::AlsoMatch => (also_match_buf, also_match_cur),
            CatPropsField::NoteFile  => (note_file_buf,  note_file_cur),
            _ => return,
        };
        if *cur == 0 { return; }
        *cur -= 1;
        let byte = char_to_byte(buf, *cur);
        buf.remove(byte);
    }

    pub fn cat_props_delete(&mut self) {
        let CatMode::Props {
            active_field, name_buf, name_cur, short_name_buf, short_name_cur,
            also_match_buf, also_match_cur, note_file_buf, note_file_cur, ..
        } = &mut self.cat_state.mode else { return };
        let (buf, cur) = match active_field {
            CatPropsField::Name      => (name_buf,       name_cur),
            CatPropsField::ShortName => (short_name_buf, short_name_cur),
            CatPropsField::AlsoMatch => (also_match_buf, also_match_cur),
            CatPropsField::NoteFile  => (note_file_buf,  note_file_cur),
            _ => return,
        };
        let len = buf.chars().count();
        if *cur >= len { return; }
        let byte = char_to_byte(buf, *cur);
        buf.remove(byte);
    }

    pub fn cat_props_cursor_left(&mut self) {
        let CatMode::Props {
            active_field, name_cur, short_name_cur, also_match_cur, note_file_cur,
            excl_children, match_cat_name, match_short_name, ..
        } = &mut self.cat_state.mode else { return };
        match active_field {
            CatPropsField::Name      => { if *name_cur > 0       { *name_cur -= 1; } }
            CatPropsField::ShortName => { if *short_name_cur > 0 { *short_name_cur -= 1; } }
            CatPropsField::AlsoMatch => { if *also_match_cur > 0 { *also_match_cur -= 1; } }
            CatPropsField::Note      => {}
            CatPropsField::NoteFile  => { if *note_file_cur > 0  { *note_file_cur -= 1; } }
            CatPropsField::ExclChildren   => *excl_children    = !*excl_children,
            CatPropsField::MatchCatName   => *match_cat_name   = !*match_cat_name,
            CatPropsField::MatchShortName => *match_short_name = !*match_short_name,
        }
    }

    pub fn cat_props_cursor_right(&mut self) {
        let CatMode::Props {
            active_field,
            name_buf, name_cur, short_name_buf, short_name_cur,
            also_match_buf, also_match_cur, note_file_buf, note_file_cur,
            excl_children, match_cat_name, match_short_name, ..
        } = &mut self.cat_state.mode else { return };
        match active_field {
            CatPropsField::Name      => { let l = name_buf.chars().count();       if *name_cur < l       { *name_cur += 1; } }
            CatPropsField::ShortName => { let l = short_name_buf.chars().count(); if *short_name_cur < l { *short_name_cur += 1; } }
            CatPropsField::AlsoMatch => { let l = also_match_buf.chars().count(); if *also_match_cur < l { *also_match_cur += 1; } }
            CatPropsField::Note      => {}
            CatPropsField::NoteFile  => { let l = note_file_buf.chars().count();  if *note_file_cur < l  { *note_file_cur += 1; } }
            CatPropsField::ExclChildren   => *excl_children    = !*excl_children,
            CatPropsField::MatchCatName   => *match_cat_name   = !*match_cat_name,
            CatPropsField::MatchShortName => *match_short_name = !*match_short_name,
        }
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
        if path.len() <= 2 { return; }  // already top-level, or would become a second top-level

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
            CursorPos::Item { section, item } => {
                let gi = self.global_item_idx(*section, *item).unwrap_or(usize::MAX);
                self.view.items.get(gi)
                    .and_then(|it| it.values.get(&cat_id)).cloned().unwrap_or_default()
            }
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
            if let Some(gi) = self.global_item_idx(*section, *item) {
                self.view.items[gi].values.insert(cat_id, val);
            }
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
            if let Some(gi) = self.global_item_idx(*section, *item) {
                self.view.items[gi].values.insert(cat_id, val);
            }
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
