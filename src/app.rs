use crate::menu::{MenuAction, CATMGR_MENU, VIEW_MENU};
use crate::model::{Category, CategoryKind, ColFormat, Column, DateFmt, DateDisplay, Clock, DateFmtCode, FilterEntry, FilterOp, Item, Section, SectionSortMethod, SortNewItems, SortNa, SortOn, SortOrder, SortSeq, View};
use crate::persist;
use std::collections::HashMap;
use std::path::PathBuf;

// ── Navigation mode ───────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum NavMode {
    Agenda,  // default: printable keys start a new item
    Vi,      // hjkl navigation; i/o/O to enter insert
}

impl NavMode {
    pub fn from_str(s: &str) -> Self {
        if s.eq_ignore_ascii_case("vi") { NavMode::Vi } else { NavMode::Agenda }
    }
}

// ── F-key modifier state ──────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum FKeyMod { Normal, Shift, Ctrl, Alt }

// ── Screen ────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum AppScreen {
    View,
    CatMgr,
    ViewMgr,
}

// ── View-mode state ───────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum ViewAddField { Name, Section }

pub enum ViewMode {
    Normal,
    Add {
        name_buf:     String,
        name_cursor:  usize,
        sec_buf:      String,
        sec_cursor:   usize,
        sec_cat_idx:  Option<usize>,   // Some = F3-picked index into flatten_cats
        active_field: ViewAddField,
    },
    /// F3 category picker overlaid on the View Add dialog
    AddPick {
        name_buf:      String,
        name_cursor:   usize,
        sec_buf:       String,
        sec_cursor:    usize,
        picker_cursor: usize,
    },
}

#[derive(Clone, Copy)]
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
    /// "Discard this item?" confirmation dialog — removes from all categories.
    ConfirmDiscardItem { yes: bool },
    /// Item Properties modal (F6 on an item in the main column).
    /// cursor: 0=Item text, 1=Note, 2=Note file, 3=Item statistics, 4+=assigned list
    /// edit_buf: Some((buffer, cur)) when in-place text editing is active (cursor==0 only)
    ItemProps { gi: usize, cursor: usize, edit_buf: Option<(String, usize)> },
}

// ── CatMgr state ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum CatPropsField {
    Name, ShortName, AlsoMatch, Note, NoteFile,
    ExclChildren, MatchCatName, MatchShortName,
}

pub enum CatMode {
    Normal,
    /// Category reorder mode (Alt+F10): Up/Down swap with adjacent siblings.
    Move,
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

// ── ViewMgr state ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum ViewPropsField {
    Name,
    Sections,
    ItemSorting,
    SectionSorting,
    SectionSortOrder,   // only in tab order when section_sort_method != None
    HideEmptySections,
    HideDoneItems,
    HideDependentItems,
    HideInheritedItems,
    HideColumnHeads,
    SectionSeparators,
    NumberItems,
    ViewStatistics,
    ViewProtection,
}

impl ViewPropsField {
    pub fn next(self) -> Self {
        match self {
            Self::Name               => Self::Sections,
            Self::Sections           => Self::ItemSorting,
            Self::ItemSorting        => Self::SectionSorting,
            Self::SectionSorting     => Self::SectionSortOrder,
            Self::SectionSortOrder   => Self::HideEmptySections,
            Self::HideEmptySections  => Self::HideDoneItems,
            Self::HideDoneItems      => Self::HideDependentItems,
            Self::HideDependentItems => Self::HideInheritedItems,
            Self::HideInheritedItems => Self::HideColumnHeads,
            Self::HideColumnHeads    => Self::SectionSeparators,
            Self::SectionSeparators  => Self::NumberItems,
            Self::NumberItems        => Self::ViewStatistics,
            Self::ViewStatistics     => Self::ViewProtection,
            Self::ViewProtection     => Self::Name,
        }
    }
    pub fn prev(self) -> Self {
        match self {
            Self::Name               => Self::ViewProtection,
            Self::Sections           => Self::Name,
            Self::ItemSorting        => Self::Sections,
            Self::SectionSorting     => Self::ItemSorting,
            Self::SectionSortOrder   => Self::SectionSorting,
            Self::HideEmptySections  => Self::SectionSortOrder,
            Self::HideDoneItems      => Self::HideEmptySections,
            Self::HideDependentItems => Self::HideDoneItems,
            Self::HideInheritedItems => Self::HideDependentItems,
            Self::HideColumnHeads    => Self::HideInheritedItems,
            Self::SectionSeparators  => Self::HideColumnHeads,
            Self::NumberItems        => Self::SectionSeparators,
            Self::ViewStatistics     => Self::NumberItems,
            Self::ViewProtection     => Self::ViewStatistics,
        }
    }
    pub fn is_bool(self) -> bool {
        matches!(self,
            Self::HideEmptySections | Self::HideDoneItems | Self::HideDependentItems |
            Self::HideInheritedItems | Self::HideColumnHeads | Self::SectionSeparators |
            Self::NumberItems
        )
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum SecSortTarget { Method, Order }

pub enum ViewMgrMode {
    Normal,
    Rename { buffer: String, cursor: usize },
    ConfirmDelete { yes: bool },
    Props {
        name_buf:             String,
        name_cur:             usize,
        sec_cursor:           usize,      // cursor within the sections list
        sort_state:           SortState,  // for the Item Sorting sub-dialog
        sec_sort_method:      SectionSortMethod,
        sec_sort_order:       SortOrder,
        sec_sort_picker:      Option<(SecSortTarget, usize)>, // (target, cursor)
        hide_empty_sections:  bool,
        hide_done_items:      bool,
        hide_dependent_items: bool,
        hide_inherited_items: bool,
        hide_column_heads:    bool,
        section_separators:   bool,
        number_items:         bool,
        active_field:         ViewPropsField,
        sec_scroll:           usize,
    },
}

pub struct ViewMgrState {
    pub cursor: usize,
    pub mode:   ViewMgrMode,
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
        position:       ColPos,
        picker_cursor:  usize,
        confirm_delete: bool,   // true when "Discard this category?" dialog is shown
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
    /// F3 sub-category picker for standard (non-Date) columns.
    SubPick {
        col_idx:       usize,   // 0-based into view.columns
        picker_cursor: usize,   // index into col_sub_cat_list()
    },
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

// ── Section Properties state ──────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum SecPropsField { Head, ItemSorting, Filter }

pub enum FilterState {
    Closed,
    Open {
        cursor:  usize,
        entries: HashMap<usize, FilterOp>,  // cat_id → Include/Exclude
    },
}

#[derive(Clone, Copy, PartialEq)]
pub enum SortField {
    SortNewItems,
    PrimaryOn, PrimaryOrder, PrimaryNa, PrimaryCategory, PrimarySequence,
    SecondaryOn, SecondaryOrder, SecondaryNa, SecondaryCategory, SecondarySequence,
}

pub struct SortPicker {
    pub cursor: usize,
    pub target: SortField,
}

pub enum SortState {
    Closed,
    Dialog {
        sort_new:         SortNewItems,
        primary_on:       SortOn,
        primary_order:    SortOrder,
        primary_na:       SortNa,
        primary_cat_id:   Option<usize>,
        primary_seq:      SortSeq,
        secondary_on:     SortOn,
        secondary_order:  SortOrder,
        secondary_na:     SortNa,
        secondary_cat_id: Option<usize>,
        secondary_seq:    SortSeq,
        active_field:     SortField,
        picker:           Option<SortPicker>,
    },
}

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
    Props {
        sec_idx:       usize,
        head_buf:      String,
        head_cur:      usize,
        active_field:  SecPropsField,
        sort_state:    SortState,
        filter_state:  FilterState,
        filter_scroll: usize,
    },
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

// ── Save/password state ───────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum AskChoice { Yes, No, Cancel }

#[derive(Clone, Copy, PartialEq)]
pub enum PasswordPurpose { Enable, Change, Disable }

pub enum SaveState {
    Idle,
    AskOnQuit { choice: AskChoice },
    PasswordEntry {
        purpose:        PasswordPurpose,
        buf:            String,
        cursor:         usize,
        confirm_buf:    String,
        confirm_active: bool,
        error:          Option<String>,
    },
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
    // Global item pool (shared across all views)
    pub items:       Vec<Item>,
    // View
    pub view:        View,
    pub view_mode:   ViewMode,
    pub inactive_views:   Vec<View>,
    /// Index of `view` (the active view) in the combined ordered list [inactive[..voi], view, inactive[voi..]].
    pub view_order_idx:   usize,
    pub cursor:      CursorPos,
    pub mode:        Mode,
    // CatMgr
    pub categories:  Vec<Category>,
    pub cat_state:   CatMgrState,
    // ViewMgr
    pub vmgr_state:  ViewMgrState,
    // Column
    pub col_cursor:  usize,
    pub col_mode:    ColMode,
    /// Which sub-row within the current multi-assignment item is highlighted (col_cursor > 0 only).
    pub sub_row:     usize,
    // Assignment Profile
    pub assign_mode: AssignMode,
    // Category search (shared across CatMgr and Assignment Profile)
    pub cat_search:  Option<String>,
    // Item search (View screen, '/' key): (buffer, cursor_char_pos)
    pub item_search: Option<(String, usize)>,
    // Last confirmed search query (session-only, for repeat search)
    pub last_search: Option<String>,
    // Section
    pub sec_mode:    SectionMode,
    // Menu
    pub menu:        MenuState,
    // F-key bar
    pub fkey_mod:    FKeyMod,
    // Navigation mode (set from config at startup)
    pub nav_mode:    NavMode,
    // Note
    pub pending_note: Option<NoteTarget>,
    // Persistence
    pub file_path:        Option<PathBuf>,
    pub session_password: Option<String>,
    pub dirty:            bool,
    pub save_state:       SaveState,
    // Misc
    pub quit:        bool,
    next_id:         usize,
    /// Wrap width (chars) of the item text column — set during render, used for line navigation.
    pub item_wrap_width:  std::cell::Cell<usize>,
    /// View body scroll offset (lines). Updated each render frame to keep cursor visible.
    pub scroll_offset:    std::cell::Cell<usize>,
    /// Absolute line index of the first line of the cursor row — set each render frame.
    pub cursor_line:      std::cell::Cell<usize>,
    /// Height of the view body in rows — set each render frame.
    pub body_height:      std::cell::Cell<usize>,
    /// Map of every renderable cursor position to its (first_line, last_line) — set each render frame.
    pub line_map:         std::cell::RefCell<Vec<(CursorPos, usize, usize)>>,
    /// Pending first character of a two-key vi sequence (e.g. 'z' waiting for 'z').
    pub vi_pending:       Option<char>,
    /// Active color scheme / theme. Set from config on startup.
    pub theme:            crate::theme::Theme,
}

// ── Byte-offset helper ────────────────────────────────────────────────────────

fn char_to_byte(s: &str, n: usize) -> usize {
    s.char_indices().nth(n).map(|(b, _)| b).unwrap_or(s.len())
}

// ── Word-wrap helpers (for item text cursor navigation) ───────────────────────

/// Word-wrap `text` to lines of at most `width` chars, returning `(lines, starts)`.
/// `starts[i]` is the char offset in `text` where line i begins.
fn wrap_lines_for_nav(text: &str, width: usize) -> (Vec<String>, Vec<usize>) {
    if width == 0 { return (vec![String::new()], vec![0]); }
    let chars: Vec<char> = text.chars().collect();
    let total = chars.len();
    let mut lines:  Vec<String> = Vec::new();
    let mut starts: Vec<usize>  = Vec::new();
    let mut pos = 0usize;
    while pos < total {
        if !lines.is_empty() {
            while pos < total && chars[pos] == ' ' { pos += 1; }
            if pos >= total { break; }
        }
        let line_start = pos;
        let end = (pos + width).min(total);
        let last_space = chars[pos..end].iter().rposition(|&c| c == ' ');
        let break_at = if end == total {
            end
        } else if let Some(sp) = last_space {
            pos + sp
        } else {
            end
        };
        let line: String = chars[pos..break_at].iter().collect::<String>()
            .trim_end_matches(' ').to_string();
        lines.push(line);
        starts.push(line_start);
        pos = break_at;
    }
    if lines.is_empty() {
        lines.push(String::new());
        starts.push(0);
    }
    (lines, starts)
}

/// Map char cursor position in original text to `(line_idx, col_within_line)`.
fn find_wrap_cursor(starts: &[usize], lines: &[String], cursor: usize) -> (usize, usize) {
    let li = starts.partition_point(|&s| s <= cursor)
        .saturating_sub(1)
        .min(lines.len().saturating_sub(1));
    let col = cursor.saturating_sub(starts[li]).min(lines[li].chars().count());
    (li, col)
}

// ── Section item filtering ────────────────────────────────────────────────────

/// Returns global item-pool indices for the items that belong to section `sec_idx`.
///
/// A section shows items whose `values` map contains a key that is `cat_id`
/// or any descendant of `cat_id` in the category tree.
/// Items are returned in their current physical (insertion) order.
pub fn section_item_indices(items: &[Item], view: &View, sec_idx: usize, cats: &[Category]) -> Vec<usize> {
    if sec_idx >= view.sections.len() { return vec![]; }
    let sec = &view.sections[sec_idx];

    let mut parent_map = HashMap::new();
    build_cat_maps(cats, None, &mut parent_map, &mut HashMap::new());

    let cat_id = sec.cat_id;
    let mut indices: Vec<usize> = items.iter().enumerate()
        .filter(|(_, item)| item.values.keys().any(|&k| is_under_map(k, cat_id, &parent_map)))
        .map(|(i, _)| i)
        .collect();
    apply_section_filter(&mut indices, items, sec, &parent_map);
    indices
}

/// Like `section_item_indices` but respects `view.hide_done_items`.
/// Use this everywhere cursor positions / render need to agree on which items are visible.
pub fn visible_item_indices(items: &[Item], view: &View, sec_idx: usize, cats: &[Category]) -> Vec<usize> {
    let all = section_item_indices(items, view, sec_idx, cats);
    if !view.hide_done_items { return all; }
    let done_id = flatten_cats(cats).iter().find(|c| c.name == "Done").map(|c| c.id);
    let Some(did) = done_id else { return all; };
    all.into_iter().filter(|&gi| !items[gi].values.contains_key(&did)).collect()
}

/// Like `section_item_indices` but applies the section's sort criteria.
/// Used by `apply_section_sort` to determine the desired order before
/// physically reordering items.
fn section_item_indices_sorted(items: &[Item], view: &View, sec_idx: usize, cats: &[Category]) -> Vec<usize> {
    if sec_idx >= view.sections.len() { return vec![]; }
    let sec = &view.sections[sec_idx];

    let mut parent_map = HashMap::new();
    let mut name_map   = HashMap::new();
    build_cat_maps(cats, None, &mut parent_map, &mut name_map);

    let cat_id = sec.cat_id;
    let mut indices: Vec<usize> = items.iter().enumerate()
        .filter(|(_, item)| item.values.keys().any(|&k| is_under_map(k, cat_id, &parent_map)))
        .map(|(i, _)| i)
        .collect();
    apply_section_filter(&mut indices, items, sec, &parent_map);

    if sec.primary_on != SortOn::None {
        let flat = flatten_cats(cats);
        let flat_order: HashMap<usize, usize> = flat.iter().enumerate().map(|(i, e)| (e.id, i)).collect();

        indices.sort_by(|&a, &b| {
            use std::cmp::Ordering;
            let na_a = item_is_na(&items[a], sec.primary_on, sec.primary_cat_id, cat_id, &parent_map);
            let na_b = item_is_na(&items[b], sec.primary_on, sec.primary_cat_id, cat_id, &parent_map);
            match (na_a, na_b) {
                (true, false) => return if sec.primary_na == SortNa::Bottom { Ordering::Greater } else { Ordering::Less },
                (false, true) => return if sec.primary_na == SortNa::Bottom { Ordering::Less } else { Ordering::Greater },
                _ => {}
            }
            let ka = item_sort_key(&items[a], sec.primary_on, sec.primary_cat_id, sec.primary_seq,
                                   cat_id, &parent_map, &name_map, &flat_order, cats);
            let kb = item_sort_key(&items[b], sec.primary_on, sec.primary_cat_id, sec.primary_seq,
                                   cat_id, &parent_map, &name_map, &flat_order, cats);
            let ord = ka.cmp(&kb);
            let ord = if sec.primary_order == SortOrder::Descending { ord.reverse() } else { ord };
            if ord != Ordering::Equal { return ord; }
            if sec.secondary_on != SortOn::None {
                let na2_a = item_is_na(&items[a], sec.secondary_on, sec.secondary_cat_id, cat_id, &parent_map);
                let na2_b = item_is_na(&items[b], sec.secondary_on, sec.secondary_cat_id, cat_id, &parent_map);
                match (na2_a, na2_b) {
                    (true, false) => return if sec.secondary_na == SortNa::Bottom { Ordering::Greater } else { Ordering::Less },
                    (false, true) => return if sec.secondary_na == SortNa::Bottom { Ordering::Less } else { Ordering::Greater },
                    _ => {}
                }
                let sa = item_sort_key(&items[a], sec.secondary_on, sec.secondary_cat_id, sec.secondary_seq,
                                       cat_id, &parent_map, &name_map, &flat_order, cats);
                let sb = item_sort_key(&items[b], sec.secondary_on, sec.secondary_cat_id, sec.secondary_seq,
                                       cat_id, &parent_map, &name_map, &flat_order, cats);
                let ord2 = sa.cmp(&sb);
                if sec.secondary_order == SortOrder::Descending { ord2.reverse() } else { ord2 }
            } else {
                Ordering::Equal
            }
        });
    }

    indices
}

/// Apply a section's include/exclude filter rules to a list of global indices.
fn apply_section_filter(
    indices:    &mut Vec<usize>,
    items:      &[Item],
    sec:        &Section,
    parent_map: &HashMap<usize, Option<usize>>,
) {
    if sec.filter.is_empty() { return; }
    let has_includes = sec.filter.iter().any(|f| f.op == FilterOp::Include);
    if has_includes {
        indices.retain(|&gi| {
            sec.filter.iter()
                .filter(|f| f.op == FilterOp::Include)
                .any(|f| items[gi].values.keys().any(|&k| is_under_map(k, f.cat_id, parent_map)))
        });
    }
    indices.retain(|&gi| {
        !sec.filter.iter()
            .filter(|f| f.op == FilterOp::Exclude)
            .any(|f| items[gi].values.keys().any(|&k| is_under_map(k, f.cat_id, parent_map)))
    });
}

fn is_under_map(mut id: usize, target: usize, parent_map: &HashMap<usize, Option<usize>>) -> bool {
    loop {
        if id == target { return true; }
        match parent_map.get(&id) {
            Some(Some(p)) => id = *p,
            _             => return false,
        }
    }
}

/// Build a string sort key for one item under the given sort configuration.
/// All keys are strings so comparisons are uniform; numeric/date keys are
/// encoded as zero-padded or bit-pattern hex so lexicographic order is correct.
fn item_sort_key(
    item:       &Item,
    sort_on:    SortOn,
    sort_cat:   Option<usize>,
    seq:        SortSeq,
    sec_cat:    usize,
    parent_map: &HashMap<usize, Option<usize>>,
    name_map:   &HashMap<usize, String>,
    flat_order: &HashMap<usize, usize>,
    cats:       &[Category],
) -> String {
    match sort_on {
        SortOn::None => String::new(),
        SortOn::ItemText => item.text.to_lowercase(),
        SortOn::Category => {
            let target = sort_cat.unwrap_or(sec_cat);
            let Some(mid) = item.values.keys().copied()
                .find(|&k| is_under_map(k, target, parent_map))
            else { return String::new(); };
            match seq {
                SortSeq::CategoryHierarchy =>
                    format!("{:016}", flat_order.get(&mid).copied().unwrap_or(usize::MAX)),
                SortSeq::Alphabetic =>
                    name_map.get(&mid).cloned().unwrap_or_default().to_lowercase(),
                SortSeq::Numeric => {
                    let v = item.values.get(&mid).map(|s| s.as_str()).unwrap_or("");
                    let n = v.trim().parse::<f64>().unwrap_or(0.0);
                    // Encode float bits so lexicographic order == numeric order
                    let bits = n.to_bits();
                    let sortable: u64 = if n.is_sign_negative() { !bits } else { bits | (1u64 << 63) };
                    format!("{:016x}", sortable)
                }
                SortSeq::Date =>
                    // ISO-8601 value strings sort lexicographically
                    item.values.get(&mid).cloned().unwrap_or_default(),
            }
        }
        SortOn::CategoryNote => {
            let Some(mid) = item.values.keys().copied()
                .find(|&k| is_under_map(k, sec_cat, parent_map))
            else { return String::new(); };
            cat_note_for_id(cats, mid).to_lowercase()
        }
    }
}

/// Returns true if an item has no value for the given sort key (i.e., is n/a).
fn item_is_na(
    item:       &Item,
    sort_on:    SortOn,
    sort_cat:   Option<usize>,
    sec_cat:    usize,
    parent_map: &HashMap<usize, Option<usize>>,
) -> bool {
    match sort_on {
        SortOn::None | SortOn::ItemText => false,
        SortOn::Category => {
            let target = sort_cat.unwrap_or(sec_cat);
            !item.values.keys().any(|&k| is_under_map(k, target, parent_map))
        }
        SortOn::CategoryNote => {
            !item.values.keys().any(|&k| is_under_map(k, sec_cat, parent_map))
        }
    }
}

// ── Tree helpers (free functions) ─────────────────────────────────────────────

/// Returns true if a category may never be deleted:
/// - top-level categories (depth 0) — only one is allowed
/// - the reserved system categories: Entry, When, Done
pub fn cat_is_protected(entry: &FlatCat) -> bool {
    entry.depth == 0
        || matches!(entry.name.as_str(), "Entry" | "When" | "Done")
}

/// Returns true if `candidate_id` is equal to `descendant_id` or is an ancestor of it.
/// Uses the flat path representation: candidate is an ancestor if descendant's path starts
/// with candidate's path.
pub fn cat_is_ancestor_or_equal(flat: &[FlatCat], candidate_id: usize, descendant_id: usize) -> bool {
    if candidate_id == descendant_id { return true; }
    let cand_path = flat.iter().find(|e| e.id == candidate_id).map(|e| e.path.as_slice());
    let desc_path = flat.iter().find(|e| e.id == descendant_id).map(|e| e.path.as_slice());
    match (cand_path, desc_path) {
        (Some(cp), Some(dp)) => dp.starts_with(cp),
        _ => false,
    }
}

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

/// Compute the ordered list of visible SortField values for the sort dialog.
fn sort_visible_fields(
    primary_on:       SortOn,
    primary_cat_id:   Option<usize>,
    secondary_on:     SortOn,
    secondary_cat_id: Option<usize>,
) -> Vec<SortField> {
    let mut f = vec![SortField::SortNewItems, SortField::PrimaryOn];
    if primary_on != SortOn::None {
        f.push(SortField::PrimaryOrder);
        f.push(SortField::PrimaryNa);
    }
    if primary_on == SortOn::Category {
        f.push(SortField::PrimaryCategory);
        if primary_cat_id.is_some() {
            f.push(SortField::PrimarySequence);
        }
    }
    f.push(SortField::SecondaryOn);
    if secondary_on != SortOn::None {
        f.push(SortField::SecondaryOrder);
        f.push(SortField::SecondaryNa);
    }
    if secondary_on == SortOn::Category {
        f.push(SortField::SecondaryCategory);
        if secondary_cat_id.is_some() {
            f.push(SortField::SecondarySequence);
        }
    }
    f
}

/// Remove and return the category at `path`.
/// Swap the category at `path` with its previous (go_up=true) or next (go_up=false) sibling.
/// The entire subtree moves with the category. Returns true if a swap was performed.
fn swap_cat_in_tree(cats: &mut Vec<Category>, path: &[usize], go_up: bool) -> bool {
    match path {
        [] => false,
        [i] => {
            let i = *i;
            if go_up {
                if i == 0 { return false; }
                cats.swap(i, i - 1); true
            } else {
                if i + 1 >= cats.len() { return false; }
                cats.swap(i, i + 1); true
            }
        }
        [first, rest @ ..] => {
            if let Some(cat) = cats.get_mut(*first) {
                swap_cat_in_tree(&mut cat.children, rest, go_up)
            } else { false }
        }
    }
}

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

/// Return the note indicator character for a category:
/// ♬ (U+266C) if an external note_file is set (takes priority),
/// ♪ (U+266A) if an inline note is set,
/// ""  if neither.
pub fn cat_note_indicator(cats: &[Category], id: usize) -> &'static str {
    for cat in cats {
        if cat.id == id {
            if !cat.note_file.is_empty() { return "\u{266C}"; }
            if !cat.note.is_empty()      { return "\u{266A}"; }
            return "";
        }
        let ind = cat_note_indicator(&cat.children, id);
        if !ind.is_empty() { return ind; }
    }
    ""
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
            id:               1,
            name:             "Initial Section".to_string(),
            cat_id:           6,
            sort_new:         SortNewItems::OnLeavingSection,
            primary_on:       SortOn::None,   primary_order:   SortOrder::Ascending,  primary_na:   SortNa::Bottom,
            primary_cat_id:   None,           primary_seq:     SortSeq::CategoryHierarchy,
            secondary_on:     SortOn::None,   secondary_order: SortOrder::Ascending,  secondary_na: SortNa::Bottom,
            secondary_cat_id: None,           secondary_seq:   SortSeq::CategoryHierarchy,
            filter:           vec![],
        };
        let view = View {
            id:         1,
            name:       "Initial View".to_string(),
            sections:   vec![section],
            columns:    Vec::new(),
            left_count: 0,
            hide_empty_sections: false, hide_done_items: false, hide_dependent_items: false,
            hide_inherited_items: false, hide_column_heads: false, section_separators: false,
            number_items: false,
            section_sort_method: SectionSortMethod::None, section_sort_order: SortOrder::Ascending,
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
            items:      vec![],
            view,
            view_mode:  ViewMode::Normal,
            inactive_views: vec![],
            view_order_idx: 0,
            cursor:     CursorPos::SectionHead(0),
            mode:       Mode::Normal,
            categories: vec![main_cat],
            cat_state:  CatMgrState { cursor: 0, mode: CatMode::Normal },
            vmgr_state:  ViewMgrState { cursor: 0, mode: ViewMgrMode::Normal },
            col_cursor:  0,
            col_mode:    ColMode::Normal,
            sub_row:     0,
            assign_mode: AssignMode::Normal,
            cat_search:  None,
            item_search: None,
            last_search: None,
            sec_mode:    SectionMode::Normal,
            menu:         MenuState::Closed,
            fkey_mod:     FKeyMod::Normal,
            nav_mode:     NavMode::Agenda,
            pending_note: None,
            file_path:        None,
            session_password: None,
            dirty:            false,
            save_state:       SaveState::Idle,
            quit:         false,
            next_id:      7,
            item_wrap_width: std::cell::Cell::new(0),
            scroll_offset:   std::cell::Cell::new(0),
            cursor_line:     std::cell::Cell::new(0),
            body_height:     std::cell::Cell::new(0),
            line_map:        std::cell::RefCell::new(Vec::new()),
            vi_pending:      None,
            theme:           crate::theme::Theme::for_scheme(crate::theme::ColorScheme::Default),
        }
    }

    /// Build an App from loaded save data, with an optional file path and password.
    pub fn from_save(
        data:     persist::SaveData,
        path:     Option<PathBuf>,
        password: Option<String>,
    ) -> Self {
        let mut all_views = data.views;
        let idx = data.current_view.min(all_views.len().saturating_sub(1));
        let view = all_views.remove(idx);
        let inactive_views = all_views;
        App {
            screen:     AppScreen::View,
            items:      data.items,
            view,
            view_mode:  ViewMode::Normal,
            inactive_views,
            view_order_idx: idx,
            cursor:     CursorPos::SectionHead(0),
            mode:       Mode::Normal,
            categories: data.categories,
            cat_state:  CatMgrState { cursor: 0, mode: CatMode::Normal },
            vmgr_state:  ViewMgrState { cursor: 0, mode: ViewMgrMode::Normal },
            col_cursor:  0,
            col_mode:    ColMode::Normal,
            sub_row:     0,
            assign_mode: AssignMode::Normal,
            cat_search:  None,
            item_search: None,
            last_search: None,
            sec_mode:    SectionMode::Normal,
            menu:         MenuState::Closed,
            fkey_mod:     FKeyMod::Normal,
            nav_mode:     NavMode::Agenda,
            pending_note: None,
            file_path:        path,
            session_password: password,
            dirty:            false,
            save_state:       SaveState::Idle,
            quit:         false,
            next_id:      data.next_id,
            item_wrap_width: std::cell::Cell::new(0),
            scroll_offset:   std::cell::Cell::new(0),
            cursor_line:     std::cell::Cell::new(0),
            body_height:     std::cell::Cell::new(0),
            line_map:        std::cell::RefCell::new(Vec::new()),
            vi_pending:      None,
            theme:           crate::theme::Theme::for_scheme(crate::theme::ColorScheme::Default),
        }
    }

    /// Resolve a (section, local_item_pos) cursor to a global index into `view.items`.
    /// Uses the visible item list so cursor indices always match what is rendered.
    fn global_item_idx(&self, sec: usize, local: usize) -> Option<usize> {
        visible_item_indices(&self.items, &self.view, sec, &self.categories).get(local).copied()
    }

    /// First section index >= `from` that is not hidden by `hide_empty_sections`.
    fn next_visible_section_fwd(&self, from: usize) -> Option<usize> {
        (from..self.view.sections.len()).find(|&s|
            !self.view.hide_empty_sections
                || !visible_item_indices(&self.items, &self.view, s, &self.categories).is_empty()
        )
    }

    /// Last section index <= `from` that is not hidden by `hide_empty_sections`.
    fn next_visible_section_bwd(&self, from: usize) -> Option<usize> {
        (0..=from).rev().find(|&s|
            !self.view.hide_empty_sections
                || !visible_item_indices(&self.items, &self.view, s, &self.categories).is_empty()
        )
    }

    fn alloc_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    // ── View Add dialog ───────────────────────────────────────────────────────

    pub fn view_add_open(&mut self) {
        self.view_mode = ViewMode::Add {
            name_buf: String::new(), name_cursor: 0,
            sec_buf:  String::new(), sec_cursor:  0,
            sec_cat_idx: None,
            active_field: ViewAddField::Name,
        };
    }
    pub fn view_add_char(&mut self, ch: char) {
        if let ViewMode::Add { name_buf, name_cursor, sec_buf, sec_cursor, sec_cat_idx, active_field } = &mut self.view_mode {
            if *active_field == ViewAddField::Name {
                let b = char_to_byte(name_buf, *name_cursor);
                name_buf.insert(b, ch); *name_cursor += 1;
            } else {
                *sec_cat_idx = None;
                let b = char_to_byte(sec_buf, *sec_cursor);
                sec_buf.insert(b, ch); *sec_cursor += 1;
            }
        }
    }
    pub fn view_add_backspace(&mut self) {
        if let ViewMode::Add { name_buf, name_cursor, sec_buf, sec_cursor, sec_cat_idx, active_field } = &mut self.view_mode {
            if *active_field == ViewAddField::Name {
                if *name_cursor > 0 {
                    *name_cursor -= 1;
                    let b = char_to_byte(name_buf, *name_cursor);
                    name_buf.remove(b);
                }
            } else {
                if *sec_cursor > 0 {
                    *sec_cat_idx = None;
                    *sec_cursor -= 1;
                    let b = char_to_byte(sec_buf, *sec_cursor);
                    sec_buf.remove(b);
                }
            }
        }
    }
    pub fn view_add_cursor_left(&mut self) {
        if let ViewMode::Add { name_cursor, sec_cursor, active_field, .. } = &mut self.view_mode {
            if *active_field == ViewAddField::Name {
                if *name_cursor > 0 { *name_cursor -= 1; }
            } else {
                if *sec_cursor > 0 { *sec_cursor -= 1; }
            }
        }
    }
    pub fn view_add_cursor_right(&mut self) {
        if let ViewMode::Add { name_buf, name_cursor, sec_buf, sec_cursor, active_field, .. } = &mut self.view_mode {
            if *active_field == ViewAddField::Name {
                if *name_cursor < name_buf.chars().count() { *name_cursor += 1; }
            } else {
                if *sec_cursor < sec_buf.chars().count() { *sec_cursor += 1; }
            }
        }
    }
    pub fn view_add_tab(&mut self) {
        if let ViewMode::Add { active_field, .. } = &mut self.view_mode {
            *active_field = match *active_field {
                ViewAddField::Name    => ViewAddField::Section,
                ViewAddField::Section => ViewAddField::Name,
            };
        }
    }
    pub fn view_add_open_pick(&mut self) {
        if let ViewMode::Add { name_buf, name_cursor, sec_buf, sec_cursor, sec_cat_idx, active_field } = &self.view_mode {
            if *active_field != ViewAddField::Section { return; }
            let flat = flatten_cats(&self.categories);
            let cursor = sec_cat_idx.unwrap_or(0).min(flat.len().saturating_sub(1));
            let (nb, nc, sb, sc) = (name_buf.clone(), *name_cursor, sec_buf.clone(), *sec_cursor);
            self.view_mode = ViewMode::AddPick {
                name_buf: nb, name_cursor: nc,
                sec_buf: sb, sec_cursor: sc,
                picker_cursor: cursor,
            };
        }
    }
    pub fn view_add_pick_up(&mut self) {
        if let ViewMode::AddPick { picker_cursor, .. } = &mut self.view_mode {
            if *picker_cursor > 0 { *picker_cursor -= 1; }
        }
    }
    pub fn view_add_pick_down(&mut self) {
        if let ViewMode::AddPick { picker_cursor, .. } = &mut self.view_mode {
            let max = flatten_cats(&self.categories).len().saturating_sub(1);
            if *picker_cursor < max { *picker_cursor += 1; }
        }
    }
    pub fn view_add_pick_pgup(&mut self, page: usize) {
        if let ViewMode::AddPick { picker_cursor, .. } = &mut self.view_mode {
            *picker_cursor = picker_cursor.saturating_sub(page);
        }
    }
    pub fn view_add_pick_pgdn(&mut self, page: usize) {
        let len = flatten_cats(&self.categories).len();
        if let ViewMode::AddPick { picker_cursor, .. } = &mut self.view_mode {
            if len > 0 { *picker_cursor = (*picker_cursor + page).min(len - 1); }
        }
    }
    pub fn view_add_pick_home(&mut self) {
        if let ViewMode::AddPick { picker_cursor, .. } = &mut self.view_mode {
            *picker_cursor = 0;
        }
    }
    pub fn view_add_pick_end(&mut self) {
        let len = flatten_cats(&self.categories).len();
        if let ViewMode::AddPick { picker_cursor, .. } = &mut self.view_mode {
            if len > 0 { *picker_cursor = len - 1; }
        }
    }
    pub fn view_add_pick_confirm(&mut self) {
        if let ViewMode::AddPick { name_buf, name_cursor, picker_cursor, .. } = &self.view_mode {
            let flat = flatten_cats(&self.categories);
            let idx  = *picker_cursor;
            let (nb, nc) = (name_buf.clone(), *name_cursor);
            let sec_name = flat.get(idx).map(|e| e.name.clone()).unwrap_or_default();
            let sc = sec_name.chars().count();
            self.view_mode = ViewMode::Add {
                name_buf: nb, name_cursor: nc,
                sec_buf: sec_name, sec_cursor: sc,
                sec_cat_idx: Some(idx), active_field: ViewAddField::Section,
            };
        }
    }
    pub fn view_add_pick_cancel(&mut self) {
        if let ViewMode::AddPick { name_buf, name_cursor, sec_buf, sec_cursor, .. } = &self.view_mode {
            let (nb, nc, sb, sc) = (name_buf.clone(), *name_cursor, sec_buf.clone(), *sec_cursor);
            self.view_mode = ViewMode::Add {
                name_buf: nb, name_cursor: nc,
                sec_buf: sb, sec_cursor: sc,
                sec_cat_idx: None, active_field: ViewAddField::Section,
            };
        }
    }
    pub fn view_add_confirm(&mut self) {
        let (name, sec_name, sec_cat_idx) = match &self.view_mode {
            ViewMode::Add { name_buf, sec_buf, sec_cat_idx, .. } =>
                (name_buf.trim().to_string(), sec_buf.trim().to_string(), *sec_cat_idx),
            _ => return,
        };
        if name.is_empty() || sec_name.is_empty() { return; }

        let cat_id: usize = {
            let flat = flatten_cats(&self.categories);
            let found = if let Some(idx) = sec_cat_idx {
                flat.get(idx).map(|e| e.id)
            } else {
                let lower = sec_name.to_lowercase();
                flat.iter().find(|e| e.name.to_lowercase() == lower).map(|e| e.id)
            };
            if let Some(id) = found { id }
            else {
                let new_id = self.alloc_id();
                if let Some(root) = self.categories.first() {
                    let root_id = root.id;
                    add_child_to_cat(&mut self.categories, root_id, new_id, &sec_name);
                }
                new_id
            }
        };

        self.view_mode = ViewMode::Normal;
        let view_id = self.alloc_id();
        let sec_id  = self.alloc_id();
        let section = Section {
            id: sec_id, name: sec_name, cat_id,
            sort_new:         SortNewItems::OnLeavingSection,
            primary_on:       SortOn::None,   primary_order:   SortOrder::Ascending,  primary_na:   SortNa::Bottom,
            primary_cat_id:   None,           primary_seq:     SortSeq::CategoryHierarchy,
            secondary_on:     SortOn::None,   secondary_order: SortOrder::Ascending,  secondary_na: SortNa::Bottom,
            secondary_cat_id: None,           secondary_seq:   SortSeq::CategoryHierarchy,
            filter:           vec![],
        };
        let new_view = View { id: view_id, name, sections: vec![section],
                              columns: vec![], left_count: 0,
                              hide_empty_sections: false, hide_done_items: false,
                              hide_dependent_items: false, hide_inherited_items: false,
                              hide_column_heads: false, section_separators: false,
                              number_items: false,
                              section_sort_method: SectionSortMethod::None,
                              section_sort_order: SortOrder::Ascending };
        let old = std::mem::replace(&mut self.view, new_view);
        self.inactive_views.push(old);
        self.cursor = CursorPos::SectionHead(0);
        self.col_cursor = 0; self.col_mode = ColMode::Normal; self.sec_mode = SectionMode::Normal;
        if self.file_path.is_some() { self.dirty = true; }
    }
    pub fn view_add_cancel(&mut self) {
        self.view_mode = ViewMode::Normal;
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
                            &self.items[gi].values, col.cat_id, &self.categories,
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
                self.items.get(*gi).map(|i| i.text.as_str()).unwrap_or("item").to_string()
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
                self.items.get(*gi).map(|i| i.note.clone()).unwrap_or_default()
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
                if let Some(item) = self.items.get_mut(gi) {
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
            AppScreen::View    => AppScreen::CatMgr,
            AppScreen::CatMgr  => AppScreen::View,
            AppScreen::ViewMgr => AppScreen::CatMgr,
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
            AppScreen::View    => VIEW_MENU,
            AppScreen::CatMgr  => CATMGR_MENU,
            AppScreen::ViewMgr => VIEW_MENU,
        }
    }

    fn apply_menu_action(&mut self, action: MenuAction) {
        match action {
            MenuAction::Quit         => { self.trigger_quit(); }
            MenuAction::ReturnToView => {
                match self.screen {
                    AppScreen::CatMgr  => { self.toggle_catmgr(); }
                    AppScreen::ViewMgr => { self.close_view_mgr(); }
                    AppScreen::View    => {}
                }
            }
            MenuAction::ColumnAdd        => self.col_open_form(true,  ColFormField::Head),
            MenuAction::ColumnProperties => self.col_open_form(false, ColFormField::Head),
            MenuAction::ColumnWidth      => self.col_open_form(false, ColFormField::Width),
            MenuAction::ColumnRemove     => self.col_delete(),
            MenuAction::ColumnMove       => self.col_begin_move(),
            MenuAction::SectionAdd       => self.sec_open_add(SectionInsert::Below),
            MenuAction::SectionRemove    => self.sec_open_confirm_remove(),
            MenuAction::CategoryMove     => self.cat_begin_move(),
            MenuAction::ViewAdd          => self.view_add_open(),
            MenuAction::ViewProperties   => self.open_view_props(),
            MenuAction::FileSave                => { self.handle_file_save(); }
            MenuAction::FileEnableEncryption
            | MenuAction::FileChangePassword
            | MenuAction::FileDisableEncryption => { self.handle_file_encryption(action); }
            MenuAction::Noop => {}
        }
        // Don't close the menu if we opened a password dialog
        if !matches!(self.save_state, SaveState::PasswordEntry { .. }) {
            self.menu = MenuState::Closed;
        } else {
            self.menu = MenuState::Closed;
        }
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
        let old_sec = match &self.cursor {
            CursorPos::SectionHead(s)       => *s,
            CursorPos::Item { section, .. } => *section,
        };
        let new_cursor = match &self.cursor {
            CursorPos::SectionHead(s) => {
                let s = *s;
                if s == 0 { CursorPos::SectionHead(0) }
                else {
                    match self.next_visible_section_bwd(s - 1) {
                        Some(prev) => {
                            let n = visible_item_indices(&self.items, &self.view, prev, &self.categories).len();
                            if n == 0 { CursorPos::SectionHead(prev) }
                            else      { CursorPos::Item { section: prev, item: n - 1 } }
                        }
                        None => CursorPos::SectionHead(s),
                    }
                }
            }
            CursorPos::Item { section, item: 0 } => CursorPos::SectionHead(*section),
            CursorPos::Item { section, item }    => CursorPos::Item { section: *section, item: item - 1 },
        };
        let new_sec = match &new_cursor {
            CursorPos::SectionHead(s)       => *s,
            CursorPos::Item { section, .. } => *section,
        };
        if new_sec != old_sec {
            if let Some(sec) = self.view.sections.get(old_sec) {
                if sec.sort_new == SortNewItems::OnLeavingSection {
                    self.apply_section_sort(old_sec);
                }
            }
        }
        if self.col_cursor > 0 {
            if let CursorPos::Item { section: s, item: i } = new_cursor {
                if let Some(gi) = visible_item_indices(&self.items, &self.view, s, &self.categories).get(i).copied() {
                    let n = item_n_rows(&self.items[gi], &self.view.columns, &self.categories);
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
                if let Some(gi) = visible_item_indices(&self.items, &self.view, s, &self.categories).get(i).copied() {
                    let n = item_n_rows(&self.items[gi], &self.view.columns, &self.categories);
                    if self.sub_row + 1 < n {
                        self.sub_row += 1;
                        return;
                    }
                }
            }
        }
        self.sub_row = 0;
        let old_sec = match &self.cursor {
            CursorPos::SectionHead(s)       => *s,
            CursorPos::Item { section, .. } => *section,
        };
        let new_cursor = match &self.cursor {
            CursorPos::SectionHead(s) => {
                let s = *s;
                let n = visible_item_indices(&self.items, &self.view, s, &self.categories).len();
                if n == 0 {
                    match self.next_visible_section_fwd(s + 1) {
                        Some(next) => CursorPos::SectionHead(next),
                        None       => CursorPos::SectionHead(s),
                    }
                } else {
                    CursorPos::Item { section: s, item: 0 }
                }
            }
            CursorPos::Item { section, item } => {
                let s = *section;
                let i = *item;
                let num_visible = visible_item_indices(&self.items, &self.view, s, &self.categories).len();
                if i + 1 < num_visible {
                    CursorPos::Item { section: s, item: i + 1 }
                } else {
                    match self.next_visible_section_fwd(s + 1) {
                        Some(next) => CursorPos::SectionHead(next),
                        None       => CursorPos::Item { section: s, item: i },
                    }
                }
            }
        };
        let new_sec = match &new_cursor {
            CursorPos::SectionHead(s)       => *s,
            CursorPos::Item { section, .. } => *section,
        };
        if new_sec != old_sec {
            if let Some(sec) = self.view.sections.get(old_sec) {
                if sec.sort_new == SortNewItems::OnLeavingSection {
                    self.apply_section_sort(old_sec);
                }
            }
        }
        self.cursor = new_cursor;
    }

    pub fn cursor_pgup(&mut self, n: usize) {
        for _ in 0..n { self.cursor_up(); }
    }

    pub fn cursor_pgdn(&mut self, n: usize) {
        for _ in 0..n { self.cursor_down(); }
    }

    /// zz — set scroll offset so the cursor row is vertically centred in the body.
    /// Uses the cursor_line and body_height values written by the last render frame.
    pub fn scroll_center(&mut self) {
        let half = self.body_height.get() / 2;
        self.scroll_offset.set(self.cursor_line.get().saturating_sub(half));
    }

    /// H — move cursor to the first visible row on screen.
    pub fn cursor_screen_top(&mut self) {
        let off = self.scroll_offset.get();
        let pos = self.line_map.borrow().iter()
            .find(|(_, _, last)| *last >= off)
            .map(|(p, _, _)| *p);
        if let Some(p) = pos { self.cursor = p; }
    }

    /// L — move cursor to the last visible row on screen.
    pub fn cursor_screen_bottom(&mut self) {
        let last_vis = self.scroll_offset.get() + self.body_height.get().saturating_sub(1);
        let pos = self.line_map.borrow().iter().rev()
            .find(|(_, first, _)| *first <= last_vis)
            .map(|(p, _, _)| *p);
        if let Some(p) = pos { self.cursor = p; }
    }

    /// Home: move to the current section head (preserving col_cursor).
    /// If already on a section head, move to the previous section head.
    pub fn cursor_home(&mut self) {
        if !matches!(self.mode, Mode::Normal) { return; }
        self.sub_row = 0;
        match self.cursor {
            CursorPos::Item { section, .. } => {
                self.cursor = CursorPos::SectionHead(section);
            }
            CursorPos::SectionHead(s) => {
                if s > 0 {
                    if let Some(prev) = self.next_visible_section_bwd(s - 1) {
                        self.cursor = CursorPos::SectionHead(prev);
                    }
                }
            }
        }
    }

    /// End: move to the last item of the current section (preserving col_cursor).
    /// If already on the last item, move to the last item of the next section.
    pub fn cursor_end(&mut self) {
        if !matches!(self.mode, Mode::Normal) { return; }
        self.sub_row = 0;
        match self.cursor {
            CursorPos::SectionHead(s) => {
                let n = visible_item_indices(&self.items, &self.view, s, &self.categories).len();
                if n > 0 {
                    self.cursor = CursorPos::Item { section: s, item: n - 1 };
                } else {
                    self.end_next_section_last(s);
                }
            }
            CursorPos::Item { section, item } => {
                let n = visible_item_indices(&self.items, &self.view, section, &self.categories).len();
                if item + 1 >= n {
                    // Already at last item — jump to last item of next section.
                    self.end_next_section_last(section);
                } else {
                    self.cursor = CursorPos::Item { section, item: n - 1 };
                }
            }
        }
    }

    /// gg — move to the first section head of the first visible section (col unchanged).
    pub fn cursor_first(&mut self) {
        if !matches!(self.mode, Mode::Normal) { return; }
        self.sub_row = 0;
        if let Some(s) = self.next_visible_section_fwd(0) {
            self.cursor = CursorPos::SectionHead(s);
        }
    }

    /// G — move to the last item of the last visible section (col unchanged).
    pub fn cursor_last(&mut self) {
        if !matches!(self.mode, Mode::Normal) { return; }
        self.sub_row = 0;
        let n = self.view.sections.len();
        if n == 0 { return; }
        if let Some(s) = self.next_visible_section_bwd(n - 1) {
            let count = visible_item_indices(&self.items, &self.view, s, &self.categories).len();
            self.cursor = if count > 0 {
                CursorPos::Item { section: s, item: count - 1 }
            } else {
                CursorPos::SectionHead(s)
            };
        }
    }

    fn end_next_section_last(&mut self, from: usize) {
        if let Some(s) = self.next_visible_section_fwd(from + 1) {
            let n = visible_item_indices(&self.items, &self.view, s, &self.categories).len();
            self.cursor = if n > 0 {
                CursorPos::Item { section: s, item: n - 1 }
            } else {
                CursorPos::SectionHead(s)
            };
        }
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

    pub fn edit_cursor_home(&mut self) {
        match &mut self.mode {
            Mode::Edit { cursor, .. } | Mode::Create { cursor, .. } => { *cursor = 0; }
            _ => {}
        }
    }

    pub fn edit_cursor_end(&mut self) {
        let w = self.item_wrap_width.get();
        match &mut self.mode {
            Mode::Edit { buffer, cursor, col, .. } if *col == 0 && w > 0 => {
                let buf = buffer.clone();
                let (lines, starts) = wrap_lines_for_nav(&buf, w);
                let (li, _) = find_wrap_cursor(&starts, &lines, *cursor);
                let line_end = starts[li] + lines[li].chars().count();
                if *cursor == line_end && li + 1 < lines.len() {
                    *cursor = starts[li + 1] + lines[li + 1].chars().count();
                } else {
                    *cursor = line_end;
                }
            }
            Mode::Create { buffer, cursor } if w > 0 => {
                let buf = buffer.clone();
                let (lines, starts) = wrap_lines_for_nav(&buf, w);
                let (li, _) = find_wrap_cursor(&starts, &lines, *cursor);
                let line_end = starts[li] + lines[li].chars().count();
                if *cursor == line_end && li + 1 < lines.len() {
                    *cursor = starts[li + 1] + lines[li + 1].chars().count();
                } else {
                    *cursor = line_end;
                }
            }
            Mode::Edit { buffer, cursor, .. } | Mode::Create { buffer, cursor } => {
                *cursor = buffer.chars().count();
            }
            _ => {}
        }
    }

    pub fn edit_cursor_line_up(&mut self) {
        let w = self.item_wrap_width.get();
        if w == 0 { return; }
        match &mut self.mode {
            Mode::Edit { buffer, cursor, col, .. } if *col == 0 => {
                let buf = buffer.clone();
                let (lines, starts) = wrap_lines_for_nav(&buf, w);
                let (li, col_in_line) = find_wrap_cursor(&starts, &lines, *cursor);
                if li > 0 {
                    *cursor = starts[li - 1] + col_in_line.min(lines[li - 1].chars().count());
                }
            }
            Mode::Create { buffer, cursor } => {
                let buf = buffer.clone();
                let (lines, starts) = wrap_lines_for_nav(&buf, w);
                let (li, col_in_line) = find_wrap_cursor(&starts, &lines, *cursor);
                if li > 0 {
                    *cursor = starts[li - 1] + col_in_line.min(lines[li - 1].chars().count());
                }
            }
            _ => {}
        }
    }

    pub fn edit_cursor_line_down(&mut self) {
        let w = self.item_wrap_width.get();
        if w == 0 { return; }
        match &mut self.mode {
            Mode::Edit { buffer, cursor, col, .. } if *col == 0 => {
                let buf = buffer.clone();
                let (lines, starts) = wrap_lines_for_nav(&buf, w);
                let (li, col_in_line) = find_wrap_cursor(&starts, &lines, *cursor);
                if li + 1 < lines.len() {
                    *cursor = starts[li + 1] + col_in_line.min(lines[li + 1].chars().count());
                }
            }
            Mode::Create { buffer, cursor } => {
                let buf = buffer.clone();
                let (lines, starts) = wrap_lines_for_nav(&buf, w);
                let (li, col_in_line) = find_wrap_cursor(&starts, &lines, *cursor);
                if li + 1 < lines.len() {
                    *cursor = starts[li + 1] + col_in_line.min(lines[li + 1].chars().count());
                }
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
                    let original = self.items.get(gi)
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

    /// Vi-mode `O`: open a new item above the cursor row.
    /// Repositions cursor so `confirm`'s "insert after current" logic inserts before the
    /// original position, then enters Create mode.
    pub fn begin_create_above(&mut self) {
        if !matches!(self.mode, Mode::Normal) { return; }
        match self.cursor {
            CursorPos::SectionHead(_) => {
                // Already at section head — insert at start of section, same as 'o'.
            }
            CursorPos::Item { section, item } => {
                if item == 0 {
                    // First item in section: anchor at the section head so the new item
                    // is inserted at position 0 (i.e., before the existing first item).
                    self.cursor = CursorPos::SectionHead(section);
                } else {
                    // Anchor one item above so the new item lands before the current one.
                    self.cursor = CursorPos::Item { section, item: item - 1 };
                }
            }
        }
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
                    self.items.get(gi).map(|it| it.text.clone()).unwrap_or_default()
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
                    let val = self.items.get(gi)
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
                let indices = section_item_indices(&self.items, &self.view, sec_idx, &self.categories);
                let global_pos = match insert_after {
                    Some(local) => indices.get(local).map(|&g| g + 1).unwrap_or(self.items.len()),
                    None        => indices.first().copied().unwrap_or(self.items.len()),
                };
                self.items.insert(global_pos, Item { id, text, values, cond_cats, note: String::new(), note_file: String::new() });
                // Local index is position within section after insertion.
                let new_local = section_item_indices(&self.items, &self.view, sec_idx, &self.categories)
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
                                self.items[gi].text = text;
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
                                        self.items[gi].values.remove(&cat_id);
                                    } else {
                                        self.items[gi].values.insert(cat_id, final_text);
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
                                    self.items[gi].values.insert(sub_id, String::new());
                                }
                            }
                        }
                    }
                }
            }
            Mode::Normal | Mode::ConfirmDeleteItem { .. } | Mode::ConfirmDiscardItem { .. } | Mode::ItemProps { .. } => {}
        }
        // Apply sort immediately if section is configured for WhenEntered.
        let sec_idx = match &self.cursor {
            CursorPos::SectionHead(s)       => *s,
            CursorPos::Item { section, .. } => *section,
        };
        if sec_idx < self.view.sections.len()
            && self.view.sections[sec_idx].sort_new == SortNewItems::WhenEntered
        {
            self.apply_section_sort(sec_idx);
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

    pub fn item_open_confirm_discard(&mut self) {
        if self.col_cursor != 0 { return; }
        if !matches!(self.cursor, CursorPos::Item { .. }) { return; }
        if !matches!(self.mode, Mode::Normal) { return; }
        self.mode = Mode::ConfirmDiscardItem { yes: true };
    }

    pub fn item_confirm_discard_toggle(&mut self) {
        if let Mode::ConfirmDiscardItem { yes } = &mut self.mode {
            *yes = !*yes;
        }
    }

    pub fn item_confirm_discard_confirm(&mut self) {
        if let Mode::ConfirmDiscardItem { yes } = self.mode {
            self.mode = Mode::Normal;
            if yes { self.item_discard(); }
        }
    }

    pub fn item_confirm_discard_cancel(&mut self) {
        if matches!(self.mode, Mode::ConfirmDiscardItem { .. }) {
            self.mode = Mode::Normal;
        }
    }

    /// Remove the item entirely from the global pool (all category assignments discarded).
    fn item_discard(&mut self) {
        let (s, i) = match self.cursor {
            CursorPos::Item { section, item } => (section, item),
            _ => return,
        };
        let Some(gi) = self.global_item_idx(s, i) else { return; };
        self.items.remove(gi);
        self.cursor = if i > 0 {
            CursorPos::Item { section: s, item: i - 1 }
        } else {
            CursorPos::SectionHead(s)
        };
        if self.file_path.is_some() { self.dirty = true; }
    }

    // ── Item Properties modal ─────────────────────────────────────────────────

    pub fn item_open_props(&mut self) {
        if self.col_cursor != 0 { return; }
        if !matches!(self.mode, Mode::Normal) { return; }
        if let CursorPos::Item { section, item } = self.cursor {
            if let Some(gi) = self.global_item_idx(section, item) {
                self.mode = Mode::ItemProps { gi, cursor: 0, edit_buf: None };
            }
        }
    }

    pub fn item_props_cancel(&mut self) {
        match &mut self.mode {
            // If editing text in-place, Esc cancels only the edit, not the whole modal.
            Mode::ItemProps { edit_buf, .. } if edit_buf.is_some() => {
                *edit_buf = None;
            }
            Mode::ItemProps { .. } => { self.mode = Mode::Normal; }
            _ => {}
        }
    }

    /// Enter in-place text edit for the Item text field (cursor == 0).
    pub fn item_props_begin_text_edit(&mut self) {
        if let Mode::ItemProps { gi, cursor, edit_buf } = &mut self.mode {
            if *cursor != 0 { return; }
            if edit_buf.is_some() { return; }
            let text = self.items.get(*gi)
                .map(|it| it.text.clone()).unwrap_or_default();
            let len = text.chars().count();
            *edit_buf = Some((text, len)); // cursor at end
        }
    }

    pub fn item_props_text_confirm(&mut self) {
        let (gi, buf) = match &self.mode {
            Mode::ItemProps { gi, edit_buf: Some((buf, _)), .. } => (*gi, buf.clone()),
            _ => return,
        };
        let trimmed = buf.trim().to_string();
        if !trimmed.is_empty() {
            if let Some(item) = self.items.get_mut(gi) {
                item.text = trimmed;
            }
        }
        if let Mode::ItemProps { edit_buf, .. } = &mut self.mode {
            *edit_buf = None;
        }
    }

    pub fn item_props_text_input_char(&mut self, ch: char) {
        if let Mode::ItemProps { edit_buf: Some((buf, cur)), .. } = &mut self.mode {
            let byte_pos: usize = buf.char_indices().nth(*cur).map(|(i,_)| i).unwrap_or(buf.len());
            buf.insert(byte_pos, ch);
            *cur += 1;
        }
    }

    pub fn item_props_text_backspace(&mut self) {
        if let Mode::ItemProps { edit_buf: Some((buf, cur)), .. } = &mut self.mode {
            if *cur > 0 {
                *cur -= 1;
                let byte_pos = buf.char_indices().nth(*cur).map(|(i,_)| i).unwrap_or(buf.len());
                buf.remove(byte_pos);
            }
        }
    }

    pub fn item_props_text_delete(&mut self) {
        if let Mode::ItemProps { edit_buf: Some((buf, cur)), .. } = &mut self.mode {
            let len = buf.chars().count();
            if *cur < len {
                let byte_pos = buf.char_indices().nth(*cur).map(|(i,_)| i).unwrap_or(buf.len());
                buf.remove(byte_pos);
            }
        }
    }

    pub fn item_props_text_cursor_left(&mut self) {
        if let Mode::ItemProps { edit_buf: Some((_, cur)), .. } = &mut self.mode {
            if *cur > 0 { *cur -= 1; }
        }
    }

    pub fn item_props_text_cursor_right(&mut self) {
        if let Mode::ItemProps { edit_buf: Some((buf, cur)), .. } = &mut self.mode {
            if *cur < buf.chars().count() { *cur += 1; }
        }
    }

    /// Build sorted assigned-category list: (cat_id, name, kind, stored_value).
    pub fn item_props_assigned(&self, gi: usize) -> Vec<(usize, String, CategoryKind, String)> {
        let flat = flatten_cats(&self.categories);
        let item = match self.items.get(gi) { Some(it) => it, None => return vec![] };
        let mut list: Vec<(usize, String, CategoryKind, String)> = item.values.keys()
            .filter_map(|id| flat.iter().find(|e| e.id == *id).map(|e|
                (*id, e.name.clone(), e.kind, item.values[id].clone())
            ))
            .collect();
        list.sort_by(|a, b| a.1.cmp(&b.1));
        list
    }

    pub fn item_props_max_cursor(&self, gi: usize) -> usize {
        let n = self.item_props_assigned(gi).len();
        // 4 fixed fields + assigned list; min cursor max is 3 (Statistics)
        if n == 0 { 3 } else { 3 + n }
    }

    pub fn item_props_cursor_up(&mut self) {
        if let Mode::ItemProps { cursor, .. } = &mut self.mode {
            if *cursor > 0 { *cursor -= 1; }
        }
    }

    pub fn item_props_cursor_down(&mut self) {
        let (gi, cur) = match &self.mode {
            Mode::ItemProps { gi, cursor, .. } => (*gi, *cursor),
            _ => return,
        };
        let max = self.item_props_max_cursor(gi);
        if let Mode::ItemProps { cursor, .. } = &mut self.mode {
            if cur < max { *cursor = cur + 1; }
        }
    }

    pub fn item_props_cursor_home(&mut self) {
        if let Mode::ItemProps { cursor, .. } = &mut self.mode { *cursor = 0; }
    }

    pub fn item_props_cursor_end(&mut self) {
        let gi = match &self.mode { Mode::ItemProps { gi, .. } => *gi, _ => return };
        let max = self.item_props_max_cursor(gi);
        if let Mode::ItemProps { cursor, .. } = &mut self.mode { *cursor = max; }
    }

    pub fn item_props_cursor_pgup(&mut self, page: usize) {
        if let Mode::ItemProps { cursor, .. } = &mut self.mode {
            *cursor = cursor.saturating_sub(page);
        }
    }

    pub fn item_props_cursor_pgdn(&mut self, page: usize) {
        let (gi, cur) = match &self.mode {
            Mode::ItemProps { gi, cursor, .. } => (*gi, *cursor),
            _ => return,
        };
        let max = self.item_props_max_cursor(gi);
        if let Mode::ItemProps { cursor, .. } = &mut self.mode {
            *cursor = (cur + page).min(max);
        }
    }

    /// F2/Enter: act on the current field.
    /// - Item text: close and begin edit
    /// - Note: close and open note editor
    /// - Assigned Date category (4+): close and open calendar if column exists
    pub fn item_props_edit(&mut self) {
        let (gi, cur) = match &self.mode {
            Mode::ItemProps { gi, cursor, .. } => (*gi, *cursor),
            _ => return,
        };
        match cur {
            0 => { self.item_props_begin_text_edit(); return; }
            1 => { self.pending_note = Some(NoteTarget::Item(gi)); }
            c if c >= 4 => {
                let list = self.item_props_assigned(gi);
                if let Some((cat_id, _, kind, _)) = list.get(c - 4) {
                    if *kind == CategoryKind::Date {
                        if let Some(col_idx) = self.view.columns.iter().position(|c| c.cat_id == *cat_id) {
                            self.mode = Mode::Normal;
                            self.col_cursor = col_idx + 1;
                            self.col_open_calendar();
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// F3: Note field → open note editor (modal stays open); other fields → open Assignment Profile.
    pub fn item_props_choices(&mut self) {
        let (gi, cur) = match &self.mode {
            Mode::ItemProps { gi, cursor, .. } => (*gi, *cursor),
            _ => return,
        };
        if cur == 1 {
            self.pending_note = Some(NoteTarget::Item(gi));
        } else {
            self.mode = Mode::Normal;
            self.assign_open();
        }
    }

    /// Del: remove the selected assignment (only valid when cursor >= 4).
    pub fn item_props_remove(&mut self) {
        let (gi, cur) = match &self.mode {
            Mode::ItemProps { gi, cursor, .. } => (*gi, *cursor),
            _ => return,
        };
        if cur < 4 { return; }
        let list = self.item_props_assigned(gi);
        if let Some((cat_id, _, _, _)) = list.get(cur - 4) {
            let cat_id = *cat_id;
            if let Some(item) = self.items.get_mut(gi) {
                item.values.remove(&cat_id);
                item.cond_cats.remove(&cat_id);
            }
        }
        let new_max = self.item_props_max_cursor(gi);
        if let Mode::ItemProps { cursor, .. } = &mut self.mode {
            *cursor = (*cursor).min(new_max);
        }
    }

    /// Remove the currently focused item from its section and adjust the cursor.
    /// Toggle the "Done" timestamp on the current item.
    pub fn item_mark_done(&mut self) {
        let CursorPos::Item { section, item } = self.cursor else { return; };
        let Some(gi) = self.global_item_idx(section, item) else { return; };
        let flat = flatten_cats(&self.categories);
        let Some(done) = flat.iter().find(|c| c.name == "Done") else { return; };
        let done_id = done.id;
        if self.items[gi].values.remove(&done_id).is_none() {
            self.items[gi].values.insert(done_id, now_datetime_string());
        }
        if self.file_path.is_some() { self.dirty = true; }
    }

    // ── Section sort ──────────────────────────────────────────────────────────

    /// Physically reorder the items belonging to `sec_idx` according to the
    /// section's sort criteria.  Items at other sections are unaffected.
    /// The cursor is updated to follow the item it was on (by item ID).
    pub fn apply_section_sort(&mut self, sec_idx: usize) {
        if sec_idx >= self.view.sections.len() { return; }
        if self.view.sections[sec_idx].primary_on == SortOn::None { return; }

        // Natural (physical) order of global indices for this section.
        let natural = section_item_indices(&self.items, &self.view, sec_idx, &self.categories);
        if natural.len() < 2 { return; }

        // Desired order according to sort criteria.
        let sorted = section_item_indices_sorted(&self.items, &self.view, sec_idx, &self.categories);

        // Save the ID of the item under the cursor so we can restore it.
        let cursor_item_id = match &self.cursor {
            CursorPos::Item { section, item } if *section == sec_idx => {
                natural.get(*item).map(|&gi| self.items[gi].id)
            }
            _ => None,
        };

        // Rearrange: copy items in `sorted` order into the slots `natural` occupies.
        let sorted_items: Vec<Item> = sorted.iter().map(|&gi| self.items[gi].clone()).collect();
        for (slot, item) in natural.iter().zip(sorted_items) {
            self.items[*slot] = item;
        }

        // Restore cursor position.
        if let Some(id) = cursor_item_id {
            let new_natural = section_item_indices(&self.items, &self.view, sec_idx, &self.categories);
            if let Some(new_local) = new_natural.iter().position(|&gi| self.items[gi].id == id) {
                self.cursor = CursorPos::Item { section: sec_idx, item: new_local };
            }
        }
    }

    /// Sort the current section immediately (Alt-S, "On demand").
    pub fn sec_sort_now(&mut self) {
        let sec_idx = match &self.cursor {
            CursorPos::SectionHead(s)        => *s,
            CursorPos::Item { section, .. }  => *section,
        };
        self.apply_section_sort(sec_idx);
        if self.file_path.is_some() { self.dirty = true; }
    }

    pub fn item_remove(&mut self) {
        let (s, i) = match self.cursor {
            CursorPos::Item { section, item } => (section, item),
            _ => return,
        };
        if s >= self.view.sections.len() { return; }
        let Some(gi) = self.global_item_idx(s, i) else { return; };

        // Remove all category assignments that place this item in section s.
        let sec_cat_id = self.view.sections[s].cat_id;
        let mut parent_map = HashMap::new();
        build_cat_maps(&self.categories, None, &mut parent_map, &mut HashMap::new());
        let to_remove: Vec<usize> = self.items[gi].values.keys()
            .copied()
            .filter(|&k| is_under_map(k, sec_cat_id, &parent_map))
            .collect();
        for k in &to_remove {
            self.items[gi].values.remove(k);
            self.items[gi].cond_cats.remove(k);
        }

        // Only discard the item entirely when it has no remaining assignments.
        if self.items[gi].values.is_empty() {
            self.items.remove(gi);
        }

        // Move cursor to the item above, or the section head if none remain.
        self.cursor = if i > 0 {
            CursorPos::Item { section: s, item: i - 1 }
        } else {
            CursorPos::SectionHead(s)
        };
        if self.file_path.is_some() { self.dirty = true; }
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
        self.col_mode = ColMode::QuickAdd { position, picker_cursor: 0, confirm_delete: false };
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

    pub fn col_quick_add_pgup(&mut self, page: usize) {
        if let ColMode::QuickAdd { picker_cursor, .. } = &mut self.col_mode {
            *picker_cursor = picker_cursor.saturating_sub(page);
        }
    }

    pub fn col_quick_add_pgdn(&mut self, page: usize) {
        let len = flatten_cats(&self.categories).len();
        if let ColMode::QuickAdd { picker_cursor, .. } = &mut self.col_mode {
            if len > 0 { *picker_cursor = (*picker_cursor + page).min(len - 1); }
        }
    }

    pub fn col_quick_add_home(&mut self) {
        if let ColMode::QuickAdd { picker_cursor, .. } = &mut self.col_mode {
            *picker_cursor = 0;
        }
    }

    pub fn col_quick_add_end(&mut self) {
        let len = flatten_cats(&self.categories).len();
        if let ColMode::QuickAdd { picker_cursor, .. } = &mut self.col_mode {
            if len > 0 { *picker_cursor = len - 1; }
        }
    }

    pub fn col_quick_add_confirm(&mut self) {
        if !matches!(self.cat_state.mode, CatMode::Normal) { return; } // don't confirm mid-create
        let (position, picker_cursor) = match &self.col_mode {
            ColMode::QuickAdd { position, picker_cursor, .. } => (*position, *picker_cursor),
            _ => return,
        };
        self.col_mode = ColMode::Normal;
        self.cat_search = None;
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
        self.cat_state.mode = CatMode::Normal;
        self.cat_search = None;
    }

    pub fn col_quick_add_begin_delete(&mut self) {
        let flat = flatten_cats(&self.categories);
        if flat.is_empty() { return; }
        let pc = match &self.col_mode {
            ColMode::QuickAdd { picker_cursor, .. } => *picker_cursor,
            _ => return,
        };
        let entry = match flat.get(pc) {
            Some(e) => e,
            None => return,
        };
        // Guard: protected system categories and top-level category.
        if cat_is_protected(entry) { return; }
        let candidate_id = entry.id;
        // Guard: cannot delete the current section head or any of its ancestors.
        let section_head_cat_id = match &self.cursor {
            CursorPos::SectionHead(si) => self.view.sections.get(*si).map(|s| s.cat_id),
            CursorPos::Item { section, .. } => self.view.sections.get(*section).map(|s| s.cat_id),
        };
        if let Some(head_id) = section_head_cat_id {
            if cat_is_ancestor_or_equal(&flat, candidate_id, head_id) {
                return;
            }
        }
        if let ColMode::QuickAdd { confirm_delete, .. } = &mut self.col_mode {
            *confirm_delete = true;
        }
    }

    pub fn col_quick_add_delete_confirm(&mut self) {
        let pc = match &mut self.col_mode {
            ColMode::QuickAdd { confirm_delete, picker_cursor, .. } => {
                if !*confirm_delete { return; }
                *confirm_delete = false;
                *picker_cursor
            }
            _ => return,
        };
        // Sync so cat_delete operates on the picker's position.
        self.cat_state.cursor = pc;
        self.cat_delete();
        // Sync picker_cursor back (cat_delete may have clamped it).
        let new_cur = self.cat_state.cursor;
        if let ColMode::QuickAdd { picker_cursor, .. } = &mut self.col_mode {
            *picker_cursor = new_cur;
        }
    }

    pub fn col_quick_add_delete_cancel(&mut self) {
        if let ColMode::QuickAdd { confirm_delete, .. } = &mut self.col_mode {
            *confirm_delete = false;
        }
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
    pub fn col_choices_pgup(&mut self, page: usize) {
        if let ColMode::Choices { picker_cursor, .. } = &mut self.col_mode {
            *picker_cursor = picker_cursor.saturating_sub(page);
        }
    }
    pub fn col_choices_pgdn(&mut self, page: usize) {
        let list_len = match &self.col_mode {
            ColMode::Choices { kind: ChoicesKind::Category, .. } => flatten_cats(&self.categories).len(),
            ColMode::Choices { kind: ChoicesKind::Position, .. } => 2,
            _ => return,
        };
        if let ColMode::Choices { picker_cursor, .. } = &mut self.col_mode {
            if list_len > 0 { *picker_cursor = (*picker_cursor + page).min(list_len - 1); }
        }
    }
    pub fn col_choices_home(&mut self) {
        if let ColMode::Choices { picker_cursor, .. } = &mut self.col_mode {
            *picker_cursor = 0;
        }
    }
    pub fn col_choices_end(&mut self) {
        let list_len = match &self.col_mode {
            ColMode::Choices { kind: ChoicesKind::Category, .. } => flatten_cats(&self.categories).len(),
            ColMode::Choices { kind: ChoicesKind::Position, .. } => 2,
            _ => return,
        };
        if let ColMode::Choices { picker_cursor, .. } = &mut self.col_mode {
            if list_len > 0 { *picker_cursor = list_len - 1; }
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
        let item_vals = self.items.get(gi).map(|it| &it.values).unwrap_or(&empty);
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
        let item_vals = self.items.get(gi).map(|it| &it.values).unwrap_or(&empty);
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
        if gi >= self.items.len() { return; }
        let item = &mut self.items[gi];
        if item.values.contains_key(&cat_id) {
            item.values.remove(&cat_id);
        } else {
            item.values.insert(cat_id, String::new());
        }
    }

    // ── Item search ───────────────────────────────────────────────────────────

    /// Open the item search bar (triggered by '/').
    pub fn search_open(&mut self) {
        if !matches!(self.mode, Mode::Normal) { return; }
        self.item_search = Some((String::new(), 0));
    }

    /// Insert a character at the cursor position and advance the cursor.
    pub fn search_char(&mut self, ch: char) {
        if let Some((buf, cur)) = &mut self.item_search {
            let byte = char_to_byte(buf, *cur);
            buf.insert(byte, ch);
            *cur += 1;
        }
    }

    /// Remove the character before the cursor.
    pub fn search_backspace(&mut self) {
        if let Some((buf, cur)) = &mut self.item_search {
            if *cur > 0 {
                *cur -= 1;
                let byte = char_to_byte(buf, *cur);
                buf.remove(byte);
            }
        }
    }

    pub fn search_cursor_left(&mut self) {
        if let Some((_, cur)) = &mut self.item_search {
            if *cur > 0 { *cur -= 1; }
        }
    }

    pub fn search_cursor_right(&mut self) {
        if let Some((buf, cur)) = &mut self.item_search {
            if *cur < buf.chars().count() { *cur += 1; }
        }
    }

    /// Cancel the search without moving the cursor.
    pub fn search_cancel(&mut self) {
        self.item_search = None;
    }

    /// Confirm the search: move cursor to the next visible item (after the
    /// current position, wrapping) whose text contains the query as a
    /// case-insensitive substring, then close.
    /// If the buffer is empty, repeats the last successful query.
    pub fn search_confirm(&mut self) {
        let query = match &self.item_search {
            Some((q, _)) if !q.trim().is_empty() => q.trim().to_lowercase(),
            _ => {
                // Empty buffer — try the last query.
                match self.last_search.clone() {
                    Some(q) => {
                        self.item_search = None;
                        q
                    }
                    None => { self.item_search = None; return; }
                }
            }
        };
        if !query.is_empty() {
            self.last_search = Some(query.clone());
        }
        self.item_search = None;

        // Determine (start_sec, start_item) = position *after* the current cursor.
        let (start_sec, start_item) = match self.cursor {
            CursorPos::SectionHead(s) => (s, 0),
            CursorPos::Item { section, item } => {
                let vis = visible_item_indices(&self.items, &self.view, section, &self.categories);
                if item + 1 < vis.len() {
                    (section, item + 1)
                } else {
                    (section + 1, 0)
                }
            }
        };

        let n_secs = self.view.sections.len();
        // Build a list of (s_idx, i_idx) pairs for all visible items, starting
        // from start_sec/start_item and wrapping around.
        let mut candidates: Vec<(usize, usize)> = Vec::new();
        for s_idx in 0..n_secs {
            let vis = visible_item_indices(&self.items, &self.view, s_idx, &self.categories);
            if self.view.hide_empty_sections && vis.is_empty() { continue; }
            for i_idx in 0..vis.len() {
                candidates.push((s_idx, i_idx));
            }
        }
        if candidates.is_empty() { return; }

        // Find the index in candidates that corresponds to (start_sec, start_item).
        let start_pos = candidates.iter().position(|&(s, i)| s == start_sec && i == start_item)
            .unwrap_or(0);

        // Search from start_pos, wrapping around.
        let len = candidates.len();
        for offset in 0..len {
            let (s_idx, i_idx) = candidates[(start_pos + offset) % len];
            let vis = visible_item_indices(&self.items, &self.view, s_idx, &self.categories);
            let gi = vis[i_idx];
            if self.items[gi].text.to_lowercase().contains(&query) {
                self.cursor = CursorPos::Item { section: s_idx, item: i_idx };
                self.col_cursor = 0;
                self.sub_row = 0;
                return;
            }
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
        if let AssignMode::Profile { cursor, .. } = &self.assign_mode { return *cursor; }
        if let ColMode::QuickAdd { picker_cursor, .. } = &self.col_mode { return *picker_cursor; }
        self.cat_state.cursor
    }

    /// Set the active cat-list cursor.
    fn set_active_cat_cursor(&mut self, idx: usize) {
        if let AssignMode::Profile { cursor, on_sub, .. } = &mut self.assign_mode {
            *cursor = idx; *on_sub = false; return;
        }
        if let ColMode::QuickAdd { picker_cursor, .. } = &mut self.col_mode {
            *picker_cursor = idx; return;
        }
        self.cat_state.cursor = idx;
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
            cat_id:           sec_cat,
            sort_new:         SortNewItems::OnLeavingSection,
            primary_on:       SortOn::None,   primary_order:   SortOrder::Ascending,  primary_na:   SortNa::Bottom,
            primary_cat_id:   None,           primary_seq:     SortSeq::CategoryHierarchy,
            secondary_on:     SortOn::None,   secondary_order: SortOrder::Ascending,  secondary_na: SortNa::Bottom,
            secondary_cat_id: None,           secondary_seq:   SortSeq::CategoryHierarchy,
            filter:           vec![],
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
    pub fn sec_choices_pgup(&mut self, page: usize) {
        if let SectionMode::Choices { picker_cursor, .. } = &mut self.sec_mode {
            *picker_cursor = picker_cursor.saturating_sub(page);
        }
    }
    pub fn sec_choices_pgdn(&mut self, page: usize) {
        let len = flatten_cats(&self.categories).len();
        if let SectionMode::Choices { picker_cursor, .. } = &mut self.sec_mode {
            if len > 0 { *picker_cursor = (*picker_cursor + page).min(len - 1); }
        }
    }
    pub fn sec_choices_home(&mut self) {
        if let SectionMode::Choices { picker_cursor, .. } = &mut self.sec_mode {
            *picker_cursor = 0;
        }
    }
    pub fn sec_choices_end(&mut self) {
        let len = flatten_cats(&self.categories).len();
        if let SectionMode::Choices { picker_cursor, .. } = &mut self.sec_mode {
            if len > 0 { *picker_cursor = len - 1; }
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

    // ── Section Properties ────────────────────────────────────────────────────

    pub fn sec_open_props(&mut self) {
        let sec_idx = match self.cursor {
            CursorPos::SectionHead(s) => s,
            _ => return,
        };
        if sec_idx >= self.view.sections.len() { return; }
        let name = self.view.sections[sec_idx].name.clone();
        let head_cur = name.chars().count();
        self.sec_mode = SectionMode::Props {
            sec_idx,
            head_buf:     name,
            head_cur,
            active_field:  SecPropsField::Head,
            sort_state:    SortState::Closed,
            filter_state:  FilterState::Closed,
            filter_scroll: 0,
        };
    }

    pub fn sec_props_tab(&mut self) {
        if let SectionMode::Props { active_field, .. } = &mut self.sec_mode {
            *active_field = match active_field {
                SecPropsField::Head        => SecPropsField::ItemSorting,
                SecPropsField::ItemSorting => SecPropsField::Filter,
                SecPropsField::Filter      => SecPropsField::Head,
            };
        }
    }

    pub fn sec_filter_list_up(&mut self) {
        if let SectionMode::Props { filter_scroll, .. } = &mut self.sec_mode {
            if *filter_scroll > 0 { *filter_scroll -= 1; }
        }
    }

    pub fn sec_filter_list_down(&mut self) {
        let (sec_idx, filter_scroll) = match &self.sec_mode {
            SectionMode::Props { sec_idx, filter_scroll, .. } => (*sec_idx, *filter_scroll),
            _ => return,
        };
        let count = self.view.sections.get(sec_idx).map(|s| s.filter.len()).unwrap_or(0);
        if count > 2 && filter_scroll + 2 < count {
            if let SectionMode::Props { filter_scroll: fs, .. } = &mut self.sec_mode {
                *fs += 1;
            }
        }
    }

    pub fn sec_props_head_char(&mut self, ch: char) {
        if let SectionMode::Props { head_buf, head_cur, active_field: SecPropsField::Head, .. }
            = &mut self.sec_mode
        {
            let byte = char_to_byte(head_buf, *head_cur);
            head_buf.insert(byte, ch);
            *head_cur += 1;
        }
    }

    pub fn sec_props_head_backspace(&mut self) {
        if let SectionMode::Props { head_buf, head_cur, active_field: SecPropsField::Head, .. }
            = &mut self.sec_mode
        {
            if *head_cur > 0 {
                *head_cur -= 1;
                let byte = char_to_byte(head_buf, *head_cur);
                head_buf.remove(byte);
            }
        }
    }

    pub fn sec_props_head_left(&mut self) {
        if let SectionMode::Props { head_cur, active_field: SecPropsField::Head, .. }
            = &mut self.sec_mode
        {
            if *head_cur > 0 { *head_cur -= 1; }
        }
    }

    pub fn sec_props_head_right(&mut self) {
        if let SectionMode::Props { head_buf, head_cur, active_field: SecPropsField::Head, .. }
            = &mut self.sec_mode
        {
            if *head_cur < head_buf.chars().count() { *head_cur += 1; }
        }
    }

    pub fn sec_props_confirm(&mut self) {
        if let SectionMode::Props { sec_idx, ref head_buf, .. } = self.sec_mode {
            let trimmed = head_buf.trim().to_string();
            if !trimmed.is_empty() && sec_idx < self.view.sections.len() {
                self.view.sections[sec_idx].name = trimmed;
                if self.file_path.is_some() { self.dirty = true; }
            }
        }
        self.sec_mode = SectionMode::Normal;
    }

    pub fn sec_props_cancel(&mut self) {
        self.sec_mode = SectionMode::Normal;
    }

    // ── Filter picker ─────────────────────────────────────────────────────────

    pub fn sec_open_filter_picker(&mut self) {
        let sec_idx = match &self.sec_mode {
            SectionMode::Props { sec_idx, active_field: SecPropsField::Filter, .. } => *sec_idx,
            _ => return,
        };
        let entries: HashMap<usize, FilterOp> = if sec_idx < self.view.sections.len() {
            self.view.sections[sec_idx].filter.iter().map(|e| (e.cat_id, e.op)).collect()
        } else {
            HashMap::new()
        };
        if let SectionMode::Props { ref mut filter_state, .. } = self.sec_mode {
            *filter_state = FilterState::Open { cursor: 0, entries };
        }
    }

    pub fn sec_filter_picker_up(&mut self) {
        if let SectionMode::Props { filter_state: FilterState::Open { cursor, .. }, .. } = &mut self.sec_mode {
            if *cursor > 0 { *cursor -= 1; }
        }
    }

    pub fn sec_filter_picker_down(&mut self) {
        let count = flatten_cats(&self.categories).len();
        if let SectionMode::Props { filter_state: FilterState::Open { cursor, .. }, .. } = &mut self.sec_mode {
            if *cursor + 1 < count { *cursor += 1; }
        }
    }

    pub fn sec_filter_picker_pgup(&mut self, n: usize) {
        if let SectionMode::Props { filter_state: FilterState::Open { cursor, .. }, .. } = &mut self.sec_mode {
            *cursor = cursor.saturating_sub(n);
        }
    }

    pub fn sec_filter_picker_pgdn(&mut self, n: usize) {
        let count = flatten_cats(&self.categories).len();
        if let SectionMode::Props { filter_state: FilterState::Open { cursor, .. }, .. } = &mut self.sec_mode {
            *cursor = (*cursor + n).min(count.saturating_sub(1));
        }
    }

    pub fn sec_filter_picker_home(&mut self) {
        if let SectionMode::Props { filter_state: FilterState::Open { cursor, .. }, .. } = &mut self.sec_mode {
            *cursor = 0;
        }
    }

    pub fn sec_filter_picker_end(&mut self) {
        let count = flatten_cats(&self.categories).len();
        if let SectionMode::Props { filter_state: FilterState::Open { cursor, .. }, .. } = &mut self.sec_mode {
            *cursor = count.saturating_sub(1);
        }
    }

    /// Cycle the filter status of the category at the cursor: (none) → Include → Exclude → (none).
    pub fn sec_filter_picker_toggle(&mut self) {
        let cat_id = {
            let flat = flatten_cats(&self.categories);
            match &self.sec_mode {
                SectionMode::Props { filter_state: FilterState::Open { cursor, .. }, .. } => {
                    flat.get(*cursor).map(|c| c.id)
                }
                _ => None,
            }
        };
        let Some(cat_id) = cat_id else { return; };
        if let SectionMode::Props { filter_state: FilterState::Open { entries, .. }, .. } = &mut self.sec_mode {
            match entries.get(&cat_id).copied() {
                None                     => { entries.insert(cat_id, FilterOp::Include); }
                Some(FilterOp::Include)  => { entries.insert(cat_id, FilterOp::Exclude); }
                Some(FilterOp::Exclude)  => { entries.remove(&cat_id); }
            }
        }
    }

    /// Confirm the filter picker: write working entries to the section.
    pub fn sec_filter_picker_confirm(&mut self) {
        let flat = flatten_cats(&self.categories);
        let (sec_idx, entries) = match &self.sec_mode {
            SectionMode::Props {
                sec_idx,
                filter_state: FilterState::Open { entries, .. }, ..
            } => (*sec_idx, entries.clone()),
            _ => return,
        };
        if sec_idx < self.view.sections.len() {
            // Rebuild in flat-cats order for deterministic serialization.
            let new_filter: Vec<FilterEntry> = flat.iter()
                .filter_map(|c| entries.get(&c.id).map(|&op| FilterEntry { cat_id: c.id, op }))
                .collect();
            self.view.sections[sec_idx].filter = new_filter;
            if self.file_path.is_some() { self.dirty = true; }
        }
        if let SectionMode::Props { ref mut filter_state, .. } = self.sec_mode {
            *filter_state = FilterState::Closed;
        }
    }

    pub fn sec_filter_picker_cancel(&mut self) {
        if let SectionMode::Props { ref mut filter_state, .. } = self.sec_mode {
            *filter_state = FilterState::Closed;
        }
    }

    // ── Sort dialog ───────────────────────────────────────────────────────────

    pub fn sec_open_sort_dialog(&mut self) {
        if let SectionMode::Props { sec_idx, ref mut sort_state, active_field: SecPropsField::ItemSorting, .. }
            = self.sec_mode
        {
            if sec_idx < self.view.sections.len() {
                let sec = &self.view.sections[sec_idx];
                *sort_state = SortState::Dialog {
                    sort_new:         sec.sort_new,
                    primary_on:       sec.primary_on,
                    primary_order:    sec.primary_order,
                    primary_na:       sec.primary_na,
                    primary_cat_id:   sec.primary_cat_id,
                    primary_seq:      sec.primary_seq,
                    secondary_on:     sec.secondary_on,
                    secondary_order:  sec.secondary_order,
                    secondary_na:     sec.secondary_na,
                    secondary_cat_id: sec.secondary_cat_id,
                    secondary_seq:    sec.secondary_seq,
                    active_field:     SortField::SortNewItems,
                    picker:           None,
                };
            }
        }
    }

    pub fn sec_sort_tab(&mut self) {
        if let SectionMode::Props {
            sort_state: SortState::Dialog { active_field, primary_on, primary_cat_id, secondary_on, secondary_cat_id, .. }, ..
        } = &mut self.sec_mode {
            let fields = sort_visible_fields(*primary_on, *primary_cat_id, *secondary_on, *secondary_cat_id);
            let pos = fields.iter().position(|f| f == active_field).unwrap_or(0);
            *active_field = fields[(pos + 1) % fields.len()];
        }
    }

    pub fn sec_sort_tab_back(&mut self) {
        if let SectionMode::Props {
            sort_state: SortState::Dialog { active_field, primary_on, primary_cat_id, secondary_on, secondary_cat_id, .. }, ..
        } = &mut self.sec_mode {
            let fields = sort_visible_fields(*primary_on, *primary_cat_id, *secondary_on, *secondary_cat_id);
            let pos = fields.iter().position(|f| f == active_field).unwrap_or(0);
            *active_field = if pos == 0 { *fields.last().unwrap_or(&SortField::SortNewItems) } else { fields[pos - 1] };
        }
    }

    pub fn sec_sort_confirm(&mut self) {
        if let SectionMode::Props {
            sec_idx,
            sort_state: SortState::Dialog {
                sort_new, primary_on, primary_order, primary_na, primary_cat_id, primary_seq,
                secondary_on, secondary_order, secondary_na, secondary_cat_id, secondary_seq, ..
            }, ..
        } = self.sec_mode {
            if sec_idx < self.view.sections.len() {
                let sec = &mut self.view.sections[sec_idx];
                sec.sort_new         = sort_new;
                sec.primary_on       = primary_on;
                sec.primary_order    = primary_order;
                sec.primary_na       = primary_na;
                sec.primary_cat_id   = primary_cat_id;
                sec.primary_seq      = primary_seq;
                sec.secondary_on     = secondary_on;
                sec.secondary_order  = secondary_order;
                sec.secondary_na     = secondary_na;
                sec.secondary_cat_id = secondary_cat_id;
                sec.secondary_seq    = secondary_seq;
                if self.file_path.is_some() { self.dirty = true; }
            }
        }
        if let SectionMode::Props { ref mut sort_state, .. } = self.sec_mode {
            *sort_state = SortState::Closed;
        }
    }

    pub fn sec_sort_cancel(&mut self) {
        if let SectionMode::Props { ref mut sort_state, .. } = self.sec_mode {
            *sort_state = SortState::Closed;
        }
    }

    // ── Sort field picker ─────────────────────────────────────────────────────

    pub fn sec_sort_open_picker(&mut self) {
        // Precompute flat cats before borrowing sec_mode
        let flat_cats = flatten_cats(&self.categories);
        let (target, current_idx) = {
            let SectionMode::Props {
                sort_state: SortState::Dialog {
                    active_field, primary_on, primary_order, primary_na, primary_cat_id, primary_seq,
                    secondary_on, secondary_order, secondary_na, secondary_cat_id, secondary_seq, ..
                }, ..
            } = &self.sec_mode else { return; };
            match active_field {
                SortField::SortNewItems      => (*active_field, 0),
                SortField::PrimaryOn         => (*active_field, SortOn::ALL.iter().position(|&x| x == *primary_on).unwrap_or(0)),
                SortField::PrimaryOrder      => (*active_field, SortOrder::ALL.iter().position(|&x| x == *primary_order).unwrap_or(0)),
                SortField::PrimaryNa         => (*active_field, SortNa::ALL.iter().position(|&x| x == *primary_na).unwrap_or(0)),
                SortField::PrimaryCategory   => (*active_field, primary_cat_id.and_then(|id| flat_cats.iter().position(|e| e.id == id)).unwrap_or(0)),
                SortField::PrimarySequence   => (*active_field, SortSeq::ALL.iter().position(|&x| x == *primary_seq).unwrap_or(0)),
                SortField::SecondaryOn       => (*active_field, SortOn::ALL.iter().position(|&x| x == *secondary_on).unwrap_or(0)),
                SortField::SecondaryOrder    => (*active_field, SortOrder::ALL.iter().position(|&x| x == *secondary_order).unwrap_or(0)),
                SortField::SecondaryNa       => (*active_field, SortNa::ALL.iter().position(|&x| x == *secondary_na).unwrap_or(0)),
                SortField::SecondaryCategory => (*active_field, secondary_cat_id.and_then(|id| flat_cats.iter().position(|e| e.id == id)).unwrap_or(0)),
                SortField::SecondarySequence => (*active_field, SortSeq::ALL.iter().position(|&x| x == *secondary_seq).unwrap_or(0)),
            }
        };
        if let SectionMode::Props { sort_state: SortState::Dialog { ref mut picker, .. }, .. } = self.sec_mode {
            *picker = Some(SortPicker { cursor: current_idx, target });
        }
    }

    fn sec_sort_picker_len(&self) -> usize {
        let flat_len = flatten_cats(&self.categories).len();
        if let SectionMode::Props { sort_state: SortState::Dialog { picker: Some(p), .. }, .. } = &self.sec_mode {
            match p.target {
                SortField::SortNewItems                                   => SortNewItems::ALL.len(),
                SortField::PrimaryOn | SortField::SecondaryOn             => SortOn::ALL.len(),
                SortField::PrimaryOrder | SortField::SecondaryOrder       => SortOrder::ALL.len(),
                SortField::PrimaryNa | SortField::SecondaryNa             => SortNa::ALL.len(),
                SortField::PrimaryCategory | SortField::SecondaryCategory => flat_len,
                SortField::PrimarySequence | SortField::SecondarySequence => SortSeq::ALL.len(),
            }
        } else { 0 }
    }

    pub fn sec_sort_picker_up(&mut self) {
        if let SectionMode::Props {
            sort_state: SortState::Dialog { ref mut picker, .. }, ..
        } = self.sec_mode {
            if let Some(p) = picker {
                if p.cursor > 0 { p.cursor -= 1; }
            }
        }
    }

    pub fn sec_sort_picker_down(&mut self) {
        let max = self.sec_sort_picker_len();
        if let SectionMode::Props {
            sort_state: SortState::Dialog { ref mut picker, .. }, ..
        } = self.sec_mode {
            if let Some(p) = picker {
                if p.cursor + 1 < max { p.cursor += 1; }
            }
        }
    }
    pub fn sec_sort_picker_pgup(&mut self, page: usize) {
        if let SectionMode::Props {
            sort_state: SortState::Dialog { ref mut picker, .. }, ..
        } = self.sec_mode {
            if let Some(p) = picker { p.cursor = p.cursor.saturating_sub(page); }
        }
    }
    pub fn sec_sort_picker_pgdn(&mut self, page: usize) {
        let max = self.sec_sort_picker_len();
        if let SectionMode::Props {
            sort_state: SortState::Dialog { ref mut picker, .. }, ..
        } = self.sec_mode {
            if let Some(p) = picker {
                if max > 0 { p.cursor = (p.cursor + page).min(max - 1); }
            }
        }
    }
    pub fn sec_sort_picker_home(&mut self) {
        if let SectionMode::Props {
            sort_state: SortState::Dialog { ref mut picker, .. }, ..
        } = self.sec_mode {
            if let Some(p) = picker { p.cursor = 0; }
        }
    }
    pub fn sec_sort_picker_end(&mut self) {
        let max = self.sec_sort_picker_len();
        if let SectionMode::Props {
            sort_state: SortState::Dialog { ref mut picker, .. }, ..
        } = self.sec_mode {
            if let Some(p) = picker {
                if max > 0 { p.cursor = max - 1; }
            }
        }
    }

    pub fn sec_sort_picker_confirm(&mut self) {
        let flat_cats = flatten_cats(&self.categories);
        let (cursor, target) = {
            let SectionMode::Props { sort_state: SortState::Dialog { picker, .. }, .. } = &self.sec_mode
            else { return; };
            match picker { Some(p) => (p.cursor, p.target), None => return }
        };

        if let SectionMode::Props {
            sort_state: SortState::Dialog {
                ref mut sort_new, ref mut primary_on, ref mut primary_order, ref mut primary_na,
                ref mut primary_cat_id, ref mut primary_seq,
                ref mut secondary_on, ref mut secondary_order, ref mut secondary_na,
                ref mut secondary_cat_id, ref mut secondary_seq,
                ref mut picker, ..
            }, ..
        } = self.sec_mode {
            match target {
                SortField::SortNewItems => {
                    if let Some(&v) = SortNewItems::ALL.get(cursor) { *sort_new = v; }
                }
                SortField::PrimaryOn => {
                    if let Some(&v) = SortOn::ALL.get(cursor) {
                        if v != SortOn::Category { *primary_cat_id = None; }
                        *primary_on = v;
                    }
                }
                SortField::PrimaryOrder => {
                    if let Some(&v) = SortOrder::ALL.get(cursor) { *primary_order = v; }
                }
                SortField::PrimaryNa => {
                    if let Some(&v) = SortNa::ALL.get(cursor) { *primary_na = v; }
                }
                SortField::PrimaryCategory => {
                    if let Some(e) = flat_cats.get(cursor) { *primary_cat_id = Some(e.id); }
                }
                SortField::PrimarySequence => {
                    if let Some(&v) = SortSeq::ALL.get(cursor) { *primary_seq = v; }
                }
                SortField::SecondaryOn => {
                    if let Some(&v) = SortOn::ALL.get(cursor) {
                        if v != SortOn::Category { *secondary_cat_id = None; }
                        *secondary_on = v;
                    }
                }
                SortField::SecondaryOrder => {
                    if let Some(&v) = SortOrder::ALL.get(cursor) { *secondary_order = v; }
                }
                SortField::SecondaryNa => {
                    if let Some(&v) = SortNa::ALL.get(cursor) { *secondary_na = v; }
                }
                SortField::SecondaryCategory => {
                    if let Some(e) = flat_cats.get(cursor) { *secondary_cat_id = Some(e.id); }
                }
                SortField::SecondarySequence => {
                    if let Some(&v) = SortSeq::ALL.get(cursor) { *secondary_seq = v; }
                }
            }
            *picker = None;
        }
    }

    pub fn sec_sort_picker_cancel(&mut self) {
        if let SectionMode::Props {
            sort_state: SortState::Dialog { ref mut picker, .. }, ..
        } = self.sec_mode {
            *picker = None;
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
            CatMode::Normal | CatMode::Move | CatMode::Props { .. } => {}
        }
    }

    pub fn cat_edit_cursor_right(&mut self) {
        match &mut self.cat_state.mode {
            CatMode::Edit { buffer, cursor } | CatMode::Create { buffer, cursor, .. } => {
                let len = buffer.chars().count();
                if *cursor < len { *cursor += 1; }
            }
            CatMode::Normal | CatMode::Move | CatMode::Props { .. } => {}
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
            CatMode::Normal | CatMode::Move | CatMode::Props { .. } => return,
        };
        let byte_pos = char_to_byte(buffer, *cursor);
        buffer.insert(byte_pos, ch);
        *cursor += 1;
    }

    pub fn cat_input_backspace(&mut self) {
        let (buffer, cursor) = match &mut self.cat_state.mode {
            CatMode::Edit   { buffer, cursor }     => (buffer, cursor),
            CatMode::Create { buffer, cursor, .. } => (buffer, cursor),
            CatMode::Normal | CatMode::Move | CatMode::Props { .. } => return,
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
            CatMode::Normal | CatMode::Move | CatMode::Props { .. } => return,
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
                        self.set_active_cat_cursor(pos);
                    }
                }
            }
            CatMode::Normal | CatMode::Move | CatMode::Props { .. } => {}
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
        if cat_is_protected(&flat[idx]) { return; }
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

    // ── Category Move (Alt+F10) ───────────────────────────────────────────────

    pub fn cat_begin_move(&mut self) {
        if matches!(self.cat_state.mode, CatMode::Normal) {
            self.cat_state.mode = CatMode::Move;
        }
    }

    pub fn cat_move_confirm(&mut self) {
        self.cat_state.mode = CatMode::Normal;
    }

    pub fn cat_move_up(&mut self) {
        let flat = flatten_cats(&self.categories);
        let Some(entry) = flat.get(self.cat_state.cursor) else { return };
        let id = entry.id;
        let path = entry.path.clone();
        if swap_cat_in_tree(&mut self.categories, &path, true) {
            let new_flat = flatten_cats(&self.categories);
            if let Some(pos) = new_flat.iter().position(|f| f.id == id) {
                self.cat_state.cursor = pos;
            }
            if self.file_path.is_some() { self.dirty = true; }
        }
    }

    pub fn cat_move_down(&mut self) {
        let flat = flatten_cats(&self.categories);
        let Some(entry) = flat.get(self.cat_state.cursor) else { return };
        let id = entry.id;
        let path = entry.path.clone();
        if swap_cat_in_tree(&mut self.categories, &path, false) {
            let new_flat = flatten_cats(&self.categories);
            if let Some(pos) = new_flat.iter().position(|f| f.id == id) {
                self.cat_state.cursor = pos;
            }
            if self.file_path.is_some() { self.dirty = true; }
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
                self.items.get(gi)
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

    // ── Sub-category picker (F3 on standard columns) ──────────────────────────

    /// Returns (cat_id, name, relative_depth) for the column head and all its
    /// descendants, in depth-first tree order.
    /// relative_depth 0 = the column head itself; 1 = direct children; etc.
    pub fn col_sub_cat_list(&self, col_idx: usize) -> Vec<(usize, String, usize)> {
        let Some(col) = self.view.columns.get(col_idx) else { return vec![] };
        let head_id = col.cat_id;
        let flat = flatten_cats(&self.categories);
        let head = flat.iter().find(|e| e.id == head_id);
        let (head_path, head_depth) = match head {
            Some(h) => (h.path.clone(), h.depth),
            None    => return vec![],
        };
        // Include the head itself (depth 0) then all descendants.
        flat.iter()
            .filter(|e| e.id == head_id || e.path.starts_with(&head_path))
            .map(|e| (e.id, e.name.clone(), e.depth.saturating_sub(head_depth)))
            .collect()
    }

    pub fn col_open_sub_pick(&mut self) {
        if !matches!(self.mode, Mode::Normal) { return; }
        if self.col_cursor == 0 { return; }
        let col_idx = self.col_cursor - 1;
        let is_date = self.view.columns.get(col_idx)
            .map(|c| c.date_fmt.is_some())
            .unwrap_or(true);
        if is_date { return; }
        if !matches!(self.cursor, CursorPos::Item { .. }) { return; }
        let subs = self.col_sub_cat_list(col_idx);
        if subs.is_empty() { return; }
        // Start on the first already-assigned entry, or index 0.
        let gi = match self.cursor {
            CursorPos::Item { section, item } => self.global_item_idx(section, item),
            _ => None,
        };
        let start = gi.and_then(|gi| {
            let vals = &self.items[gi].values;
            subs.iter().position(|(id, _, _)| vals.contains_key(id))
        }).unwrap_or(0);
        self.col_mode = ColMode::SubPick { col_idx, picker_cursor: start };
    }

    pub fn col_sub_pick_up(&mut self) {
        if let ColMode::SubPick { picker_cursor, .. } = &mut self.col_mode {
            if *picker_cursor > 0 { *picker_cursor -= 1; }
        }
    }

    pub fn col_sub_pick_down(&mut self) {
        let len = match &self.col_mode {
            ColMode::SubPick { col_idx, .. } => self.col_sub_cat_list(*col_idx).len(),
            _ => return,
        };
        if let ColMode::SubPick { picker_cursor, .. } = &mut self.col_mode {
            if *picker_cursor + 1 < len { *picker_cursor += 1; }
        }
    }
    pub fn col_sub_pick_pgup(&mut self, page: usize) {
        if let ColMode::SubPick { picker_cursor, .. } = &mut self.col_mode {
            *picker_cursor = picker_cursor.saturating_sub(page);
        }
    }
    pub fn col_sub_pick_pgdn(&mut self, page: usize) {
        let len = match &self.col_mode {
            ColMode::SubPick { col_idx, .. } => self.col_sub_cat_list(*col_idx).len(),
            _ => return,
        };
        if let ColMode::SubPick { picker_cursor, .. } = &mut self.col_mode {
            if len > 0 { *picker_cursor = (*picker_cursor + page).min(len - 1); }
        }
    }
    pub fn col_sub_pick_home(&mut self) {
        if let ColMode::SubPick { picker_cursor, .. } = &mut self.col_mode {
            *picker_cursor = 0;
        }
    }
    pub fn col_sub_pick_end(&mut self) {
        let len = match &self.col_mode {
            ColMode::SubPick { col_idx, .. } => self.col_sub_cat_list(*col_idx).len(),
            _ => return,
        };
        if let ColMode::SubPick { picker_cursor, .. } = &mut self.col_mode {
            if len > 0 { *picker_cursor = len - 1; }
        }
    }

    /// Sync `cat_state.cursor` to the flat-cats index of the category at `picker_cursor`.
    fn col_sub_pick_sync_cursor(&mut self) -> bool {
        let (col_idx, picker_cursor) = match &self.col_mode {
            ColMode::SubPick { col_idx, picker_cursor } => (*col_idx, *picker_cursor),
            _ => return false,
        };
        let subs = self.col_sub_cat_list(col_idx);
        let Some(&(cat_id, _, _)) = subs.get(picker_cursor) else { return false; };
        let flat = flatten_cats(&self.categories);
        let Some(flat_idx) = flat.iter().position(|e| e.id == cat_id) else { return false; };
        self.cat_state.cursor = flat_idx;
        true
    }
    pub fn col_sub_pick_begin_edit(&mut self) {
        if self.col_sub_pick_sync_cursor() { self.cat_begin_edit(); }
    }
    pub fn col_sub_pick_open_props(&mut self) {
        if self.col_sub_pick_sync_cursor() { self.cat_open_props(); }
    }
    /// Ins — add new sibling below cursor; on the head row, add as child instead.
    pub fn col_sub_pick_begin_create(&mut self) {
        let picker_cursor = match &self.col_mode {
            ColMode::SubPick { picker_cursor, .. } => *picker_cursor,
            _ => return,
        };
        if self.col_sub_pick_sync_cursor() {
            // picker_cursor == 0 is the column head; creating a sibling would leave the subtree,
            // so create as child instead.
            self.cat_begin_create(picker_cursor == 0);
        }
    }

    /// Toggle assignment of the highlighted entry.
    ///
    /// Rules:
    /// - Selecting the head: clears all descendant assignments, toggles the head.
    /// - Selecting a descendant: clears the head assignment, toggles the descendant.
    pub fn col_sub_pick_toggle(&mut self) {
        let (col_idx, picker_cursor) = match &self.col_mode {
            ColMode::SubPick { col_idx, picker_cursor } => (*col_idx, *picker_cursor),
            _ => return,
        };
        let subs = self.col_sub_cat_list(col_idx);
        let Some(&(cat_id, _, _)) = subs.get(picker_cursor) else { return };
        let head_id = self.view.columns.get(col_idx).map(|c| c.cat_id).unwrap_or(0);

        let gi = match self.cursor {
            CursorPos::Item { section, item } => self.global_item_idx(section, item),
            _ => return,
        };
        let Some(gi) = gi else { return };
        let item = &mut self.items[gi];

        if cat_id == head_id {
            // Toggling the head: clear all descendant assignments, then toggle head.
            for &(desc_id, _, _) in subs.iter().filter(|(id, _, _)| *id != head_id) {
                item.values.remove(&desc_id);
            }
            if item.values.contains_key(&head_id) {
                item.values.remove(&head_id);
            } else {
                item.values.insert(head_id, String::new());
            }
        } else {
            // Toggling a descendant: clear head assignment, toggle this descendant.
            item.values.remove(&head_id);
            if item.values.contains_key(&cat_id) {
                item.values.remove(&cat_id);
            } else {
                item.values.insert(cat_id, String::new());
            }
        }
    }

    pub fn col_sub_pick_close(&mut self) {
        self.col_mode = ColMode::Normal;
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
                self.items[gi].values.insert(cat_id, val);
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
                self.items[gi].values.insert(cat_id, val);
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

    // ── Persistence ───────────────────────────────────────────────────────────

    /// Save to `file_path`. Encrypted if `session_password` is set, plain otherwise.
    /// No-op if no `file_path` is configured.
    pub fn save(&mut self) -> std::io::Result<()> {
        let path = match &self.file_path {
            Some(p) => p.clone(),
            None    => return Ok(()),
        };
        let result = if let Some(pw) = &self.session_password.clone() {
            persist::save_encrypted(&path, pw, &self.categories, &self.items, &self.view, &self.inactive_views, self.view_order_idx, self.next_id)
        } else {
            persist::save_plain(&path, &self.categories, &self.items, &self.view, &self.inactive_views, self.view_order_idx, self.next_id)
        };
        if result.is_ok() { self.dirty = false; }
        result
    }

    /// Called when the user presses Alt-Q.
    /// If there are unsaved changes and a file is open, show the ask-save dialog.
    /// Otherwise quit immediately.
    pub fn trigger_quit(&mut self) {
        if self.dirty && self.file_path.is_some() {
            self.save_state = SaveState::AskOnQuit { choice: AskChoice::Yes };
        } else {
            self.quit = true;
        }
    }

    // ── Ask-save dialog ───────────────────────────────────────────────────────

    pub fn ask_save_confirm(&mut self) {
        let _ = self.save();
        self.save_state = SaveState::Idle;
        self.quit = true;
    }

    pub fn ask_save_no(&mut self) {
        self.save_state = SaveState::Idle;
        self.quit = true;
    }

    pub fn ask_save_cancel(&mut self) {
        self.save_state = SaveState::Idle;
    }

    pub fn ask_save_set_choice(&mut self, choice: AskChoice) {
        if let SaveState::AskOnQuit { choice: c } = &mut self.save_state {
            *c = choice;
        }
    }

    pub fn ask_save_move_left(&mut self) {
        if let SaveState::AskOnQuit { choice } = &mut self.save_state {
            *choice = match choice {
                AskChoice::Yes    => AskChoice::Cancel,
                AskChoice::No     => AskChoice::Yes,
                AskChoice::Cancel => AskChoice::No,
            };
        }
    }

    pub fn ask_save_move_right(&mut self) {
        if let SaveState::AskOnQuit { choice } = &mut self.save_state {
            *choice = match choice {
                AskChoice::Yes    => AskChoice::No,
                AskChoice::No     => AskChoice::Cancel,
                AskChoice::Cancel => AskChoice::Yes,
            };
        }
    }

    // ── Password-entry dialog ─────────────────────────────────────────────────

    pub fn password_entry_open(&mut self, purpose: PasswordPurpose) {
        self.save_state = SaveState::PasswordEntry {
            purpose,
            buf:            String::new(),
            cursor:         0,
            confirm_buf:    String::new(),
            confirm_active: false,
            error:          None,
        };
    }

    pub fn password_entry_char(&mut self, ch: char) {
        if let SaveState::PasswordEntry { buf, cursor, confirm_buf, confirm_active, .. } = &mut self.save_state {
            if *confirm_active {
                confirm_buf.push(ch);
            } else {
                let byte_pos = char_to_byte(buf, *cursor);
                buf.insert(byte_pos, ch);
                *cursor += 1;
            }
        }
    }

    pub fn password_entry_backspace(&mut self) {
        if let SaveState::PasswordEntry { buf, cursor, confirm_buf, confirm_active, .. } = &mut self.save_state {
            if *confirm_active {
                confirm_buf.pop();
            } else if *cursor > 0 {
                *cursor -= 1;
                let byte_pos = char_to_byte(buf, *cursor);
                buf.remove(byte_pos);
            }
        }
    }

    pub fn password_entry_tab(&mut self) {
        if let SaveState::PasswordEntry { confirm_active, purpose, .. } = &mut self.save_state {
            if *purpose != PasswordPurpose::Disable {
                *confirm_active = !*confirm_active;
            }
        }
    }

    /// Confirm the password entry dialog.
    /// Returns false and sets an error message if validation fails.
    pub fn password_entry_confirm(&mut self) -> bool {
        let (purpose, buf, confirm_buf) = match &self.save_state {
            SaveState::PasswordEntry { purpose, buf, confirm_buf, .. } => {
                (*purpose, buf.clone(), confirm_buf.clone())
            }
            _ => return false,
        };

        match purpose {
            PasswordPurpose::Disable => {
                self.session_password = None;
                self.save_state = SaveState::Idle;
                let _ = self.save();
                true
            }
            PasswordPurpose::Enable | PasswordPurpose::Change => {
                if buf.is_empty() {
                    if let SaveState::PasswordEntry { error, .. } = &mut self.save_state {
                        *error = Some("Password cannot be empty".to_string());
                    }
                    return false;
                }
                if buf != confirm_buf {
                    if let SaveState::PasswordEntry { error, confirm_buf: cb, confirm_active, .. } = &mut self.save_state {
                        *error  = Some("Passwords do not match".to_string());
                        cb.clear();
                        *confirm_active = true;
                    }
                    return false;
                }
                self.session_password = Some(buf);
                self.save_state = SaveState::Idle;
                let _ = self.save();
                true
            }
        }
    }

    pub fn password_entry_cancel(&mut self) {
        self.save_state = SaveState::Idle;
    }

    // ── Menu action dispatch ──────────────────────────────────────────────────

    pub fn handle_file_save(&mut self) {
        let _ = self.save();
    }

    pub fn handle_file_encryption(&mut self, action: MenuAction) {
        match action {
            MenuAction::FileEnableEncryption => {
                if self.file_path.is_some() {
                    self.password_entry_open(PasswordPurpose::Enable);
                }
            }
            MenuAction::FileChangePassword => {
                if self.file_path.is_some() && self.session_password.is_some() {
                    self.password_entry_open(PasswordPurpose::Change);
                }
            }
            MenuAction::FileDisableEncryption => {
                if self.session_password.is_some() {
                    self.password_entry_open(PasswordPurpose::Disable);
                }
            }
            _ => {}
        }
    }

    // ── View Manager ──────────────────────────────────────────────────────────

    pub fn open_view_mgr(&mut self) {
        self.mode       = Mode::Normal;
        self.vmgr_state = ViewMgrState { cursor: self.view_order_idx, mode: ViewMgrMode::Normal };
        self.screen     = AppScreen::ViewMgr;
    }

    pub fn close_view_mgr(&mut self) {
        self.vmgr_state.mode = ViewMgrMode::Normal;
        self.screen = AppScreen::View;
    }

    pub fn vmgr_cursor_up(&mut self) {
        if self.vmgr_state.cursor > 0 { self.vmgr_state.cursor -= 1; }
    }

    pub fn vmgr_cursor_down(&mut self) {
        let count = 1 + self.inactive_views.len();
        if self.vmgr_state.cursor + 1 < count { self.vmgr_state.cursor += 1; }
    }
    pub fn vmgr_cursor_pgup(&mut self, page: usize) {
        self.vmgr_state.cursor = self.vmgr_state.cursor.saturating_sub(page);
    }
    pub fn vmgr_cursor_pgdn(&mut self, page: usize) {
        let count = 1 + self.inactive_views.len();
        if count > 0 { self.vmgr_state.cursor = (self.vmgr_state.cursor + page).min(count - 1); }
    }
    pub fn vmgr_cursor_home(&mut self) {
        self.vmgr_state.cursor = 0;
    }
    pub fn vmgr_cursor_end(&mut self) {
        let count = 1 + self.inactive_views.len();
        if count > 0 { self.vmgr_state.cursor = count - 1; }
    }

    pub fn vmgr_move_up(&mut self) {
        let p   = self.vmgr_state.cursor;
        let voi = self.view_order_idx;
        if p == 0 { return; }
        if p == voi {
            // Active view moves up — just shift the boundary
            self.view_order_idx -= 1;
        } else if p == voi + 1 {
            // View right after active swaps with active — active shifts down
            self.view_order_idx += 1;
        } else if p < voi {
            // Both p and p-1 are before active → swap inactive[p-1] and inactive[p]
            self.inactive_views.swap(p - 1, p);
        } else {
            // Both p and p-1 are after active (p > voi+1) → swap inactive[p-2] and inactive[p-1]
            self.inactive_views.swap(p - 2, p - 1);
        }
        self.vmgr_state.cursor -= 1;
        if self.file_path.is_some() { self.dirty = true; }
    }

    pub fn vmgr_move_down(&mut self) {
        let p     = self.vmgr_state.cursor;
        let voi   = self.view_order_idx;
        let count = 1 + self.inactive_views.len();
        if p + 1 >= count { return; }
        if p == voi {
            // Active view moves down — shift boundary
            self.view_order_idx += 1;
        } else if p + 1 == voi {
            // View right before active swaps with active — active shifts up
            self.view_order_idx -= 1;
        } else if p < voi {
            // Both p and p+1 before active → swap inactive[p] and inactive[p+1]
            self.inactive_views.swap(p, p + 1);
        } else {
            // Both p and p+1 after active (p > voi) → swap inactive[p-1] and inactive[p]
            self.inactive_views.swap(p - 1, p);
        }
        self.vmgr_state.cursor += 1;
        if self.file_path.is_some() { self.dirty = true; }
    }

    pub fn vmgr_select(&mut self) {
        let idx = self.vmgr_state.cursor;
        let voi = self.view_order_idx;
        if idx != voi {
            // Which inactive slot holds the selected view?
            let inact_from = Self::vmgr_inact_idx(idx, voi);
            let new_view  = self.inactive_views.remove(inact_from);
            let old_view  = std::mem::replace(&mut self.view, new_view);
            // Re-insert old active view at the position where active previously lived.
            // After the remove, if inact_from < voi then voi shifts left by one.
            let insert_at = if inact_from < voi { voi - 1 } else { voi };
            self.inactive_views.insert(insert_at, old_view);
            self.view_order_idx = idx;
        }
        self.cursor     = CursorPos::SectionHead(0);
        self.col_cursor = 0;
        self.mode       = Mode::Normal;
        self.col_mode   = ColMode::Normal;
        self.sec_mode   = SectionMode::Normal;
        self.close_view_mgr();
        if self.file_path.is_some() { self.dirty = true; }
    }

    /// Map an ordered display index to an `inactive_views` index.
    /// Panics if `ordered_idx == view_order_idx` (that's `self.view`, not inactive).
    fn vmgr_inact_idx(ordered_idx: usize, voi: usize) -> usize {
        if ordered_idx < voi { ordered_idx } else { ordered_idx - 1 }
    }

    fn vmgr_view_name_at_cursor(&self) -> &str {
        let c = self.vmgr_state.cursor;
        let voi = self.view_order_idx;
        if c == voi {
            &self.view.name
        } else {
            self.inactive_views
                .get(Self::vmgr_inact_idx(c, voi))
                .map(|v| v.name.as_str())
                .unwrap_or("")
        }
    }

    fn vmgr_set_view_name_at_cursor(&mut self, name: String) {
        let c   = self.vmgr_state.cursor;
        let voi = self.view_order_idx;
        if c == voi {
            self.view.name = name;
        } else if let Some(v) = self.inactive_views.get_mut(Self::vmgr_inact_idx(c, voi)) {
            v.name = name;
        }
    }

    // ── Rename (F2 — inline) ─────────────────────────────────────────────────

    pub fn vmgr_begin_rename(&mut self) {
        let name = self.vmgr_view_name_at_cursor().to_string();
        let cursor = name.chars().count();
        self.vmgr_state.mode = ViewMgrMode::Rename { buffer: name, cursor };
    }

    pub fn vmgr_rename_char(&mut self, ch: char) {
        if let ViewMgrMode::Rename { buffer, cursor } = &mut self.vmgr_state.mode {
            let byte = char_to_byte(buffer, *cursor);
            buffer.insert(byte, ch);
            *cursor += 1;
        }
    }

    pub fn vmgr_rename_backspace(&mut self) {
        if let ViewMgrMode::Rename { buffer, cursor } = &mut self.vmgr_state.mode {
            if *cursor > 0 {
                *cursor -= 1;
                let byte = char_to_byte(buffer, *cursor);
                buffer.remove(byte);
            }
        }
    }

    pub fn vmgr_rename_left(&mut self) {
        if let ViewMgrMode::Rename { cursor, .. } = &mut self.vmgr_state.mode {
            if *cursor > 0 { *cursor -= 1; }
        }
    }

    pub fn vmgr_rename_right(&mut self) {
        if let ViewMgrMode::Rename { buffer, cursor } = &mut self.vmgr_state.mode {
            if *cursor < buffer.chars().count() { *cursor += 1; }
        }
    }

    pub fn vmgr_rename_confirm(&mut self) {
        if let ViewMgrMode::Rename { buffer, .. } = &self.vmgr_state.mode {
            let name = buffer.trim().to_string();
            if !name.is_empty() {
                self.vmgr_set_view_name_at_cursor(name);
                if self.file_path.is_some() { self.dirty = true; }
            }
        }
        self.vmgr_state.mode = ViewMgrMode::Normal;
    }

    pub fn vmgr_rename_cancel(&mut self) {
        self.vmgr_state.mode = ViewMgrMode::Normal;
    }

    // ── Props (F6 — dialog) ───────────────────────────────────────────────────

    fn vmgr_view_ref(&self) -> &View {
        let c = self.vmgr_state.cursor;
        let voi = self.view_order_idx;
        if c == voi { &self.view }
        else { self.inactive_views.get(Self::vmgr_inact_idx(c, voi)).unwrap_or(&self.view) }
    }

    pub fn vmgr_begin_props(&mut self) {
        let v = self.vmgr_view_ref();
        let name_buf             = v.name.clone();
        let hide_empty_sections  = v.hide_empty_sections;
        let hide_done_items      = v.hide_done_items;
        let hide_dependent_items = v.hide_dependent_items;
        let hide_inherited_items = v.hide_inherited_items;
        let hide_column_heads    = v.hide_column_heads;
        let section_separators   = v.section_separators;
        let number_items         = v.number_items;
        let sec_sort_method      = v.section_sort_method;
        let sec_sort_order       = v.section_sort_order;
        let name_cur = name_buf.chars().count();
        self.vmgr_state.mode = ViewMgrMode::Props {
            name_buf, name_cur,
            sec_cursor:   0,
            sort_state:   SortState::Closed,
            sec_sort_method, sec_sort_order, sec_sort_picker: None,
            hide_empty_sections, hide_done_items, hide_dependent_items,
            hide_inherited_items, hide_column_heads, section_separators, number_items,
            active_field: ViewPropsField::Name,
            sec_scroll:   0,
        };
    }

    /// Open View Properties for the active (current) view — used by menu.
    pub fn open_view_props(&mut self) {
        self.vmgr_state.cursor = self.view_order_idx;
        self.vmgr_begin_props();
    }

    pub fn vmgr_props_field_next(&mut self) {
        if let ViewMgrMode::Props { active_field, sec_sort_method, .. } = &mut self.vmgr_state.mode {
            let mut next = active_field.next();
            if next == ViewPropsField::SectionSortOrder && *sec_sort_method == SectionSortMethod::None {
                next = next.next();
            }
            *active_field = next;
        }
    }

    pub fn vmgr_props_field_prev(&mut self) {
        if let ViewMgrMode::Props { active_field, sec_sort_method, .. } = &mut self.vmgr_state.mode {
            let mut prev = active_field.prev();
            if prev == ViewPropsField::SectionSortOrder && *sec_sort_method == SectionSortMethod::None {
                prev = prev.prev();
            }
            *active_field = prev;
        }
    }

    pub fn vmgr_props_toggle(&mut self) {
        if let ViewMgrMode::Props { active_field, hide_empty_sections, hide_done_items,
            hide_dependent_items, hide_inherited_items, hide_column_heads,
            section_separators, number_items, .. } = &mut self.vmgr_state.mode
        {
            match active_field {
                ViewPropsField::HideEmptySections  => *hide_empty_sections  = !*hide_empty_sections,
                ViewPropsField::HideDoneItems      => *hide_done_items      = !*hide_done_items,
                ViewPropsField::HideDependentItems => *hide_dependent_items = !*hide_dependent_items,
                ViewPropsField::HideInheritedItems => *hide_inherited_items = !*hide_inherited_items,
                ViewPropsField::HideColumnHeads    => *hide_column_heads    = !*hide_column_heads,
                ViewPropsField::SectionSeparators  => *section_separators   = !*section_separators,
                ViewPropsField::NumberItems        => *number_items         = !*number_items,
                _ => {}
            }
        }
    }

    pub fn vmgr_props_name_char(&mut self, ch: char) {
        if let ViewMgrMode::Props { name_buf, name_cur, active_field: ViewPropsField::Name, .. }
            = &mut self.vmgr_state.mode
        {
            let byte = char_to_byte(name_buf, *name_cur);
            name_buf.insert(byte, ch);
            *name_cur += 1;
        }
    }

    pub fn vmgr_props_name_backspace(&mut self) {
        if let ViewMgrMode::Props { name_buf, name_cur, active_field: ViewPropsField::Name, .. }
            = &mut self.vmgr_state.mode
        {
            if *name_cur > 0 {
                *name_cur -= 1;
                let byte = char_to_byte(name_buf, *name_cur);
                name_buf.remove(byte);
            }
        }
    }

    pub fn vmgr_props_name_left(&mut self) {
        if let ViewMgrMode::Props { name_cur, .. } = &mut self.vmgr_state.mode {
            if *name_cur > 0 { *name_cur -= 1; }
        }
    }

    pub fn vmgr_props_name_right(&mut self) {
        if let ViewMgrMode::Props { name_buf, name_cur, .. } = &mut self.vmgr_state.mode {
            if *name_cur < name_buf.chars().count() { *name_cur += 1; }
        }
    }

    pub fn vmgr_props_confirm(&mut self) {
        let idx = self.vmgr_state.cursor;
        let (name, hes, hdi, hdep, hii, hch, ss, ni, ssm, sso) = match &self.vmgr_state.mode {
            ViewMgrMode::Props {
                name_buf, hide_empty_sections, hide_done_items, hide_dependent_items,
                hide_inherited_items, hide_column_heads, section_separators, number_items,
                sec_sort_method, sec_sort_order, ..
            } => (
                name_buf.trim().to_string(),
                *hide_empty_sections, *hide_done_items, *hide_dependent_items,
                *hide_inherited_items, *hide_column_heads, *section_separators, *number_items,
                *sec_sort_method, *sec_sort_order,
            ),
            _ => return,
        };
        self.vmgr_state.mode = ViewMgrMode::Normal;
        if name.is_empty() { return; }
        let voi = self.view_order_idx;
        let view = if idx == voi { &mut self.view }
                   else { match self.inactive_views.get_mut(Self::vmgr_inact_idx(idx, voi)) { Some(v) => v, None => return } };
        view.name                = name;
        view.hide_empty_sections  = hes;
        view.hide_done_items      = hdi;
        view.hide_dependent_items = hdep;
        view.hide_inherited_items = hii;
        view.hide_column_heads    = hch;
        view.section_separators   = ss;
        view.number_items         = ni;
        view.section_sort_method  = ssm;
        view.section_sort_order   = sso;
        // Apply section sort in place.
        match ssm {
            SectionSortMethod::Alphabetic => {
                view.sections.sort_by(|a, b| {
                    let c = a.name.to_lowercase().cmp(&b.name.to_lowercase());
                    if sso == SortOrder::Descending { c.reverse() } else { c }
                });
                if idx == voi { self.cursor = CursorPos::SectionHead(0); self.mode = Mode::Normal; }
            }
            SectionSortMethod::CategoryOrder => {
                let flat = flatten_cats(&self.categories);
                let pos_of = |sec: &crate::model::Section| -> usize {
                    flat.iter().position(|f| f.id == sec.cat_id).unwrap_or(usize::MAX)
                };
                view.sections.sort_by(|a, b| {
                    let c = pos_of(a).cmp(&pos_of(b));
                    if sso == SortOrder::Descending { c.reverse() } else { c }
                });
                if idx == voi { self.cursor = CursorPos::SectionHead(0); self.mode = Mode::Normal; }
            }
            _ => {}
        }
        if self.file_path.is_some() { self.dirty = true; }
    }

    pub fn vmgr_props_cancel(&mut self) {
        self.vmgr_state.mode = ViewMgrMode::Normal;
    }

    pub fn vmgr_props_sec_up(&mut self) {
        if let ViewMgrMode::Props { sec_cursor, .. } = &mut self.vmgr_state.mode {
            if *sec_cursor > 0 { *sec_cursor -= 1; }
        }
    }

    pub fn vmgr_props_sec_down(&mut self) {
        let v_idx = self.vmgr_state.cursor;
        let voi   = self.view_order_idx;
        let sec_count = if v_idx == voi { self.view.sections.len() }
                        else { self.inactive_views.get(Self::vmgr_inact_idx(v_idx, voi)).map(|v| v.sections.len()).unwrap_or(0) };
        if let ViewMgrMode::Props { sec_cursor, .. } = &mut self.vmgr_state.mode {
            if *sec_cursor + 1 < sec_count { *sec_cursor += 1; }
        }
    }

    // ── Section Sort picker ───────────────────────────────────────────────────

    pub fn vmgr_sec_sort_cycle(&mut self) {
        if let ViewMgrMode::Props { active_field: ViewPropsField::SectionSorting, sec_sort_method, .. }
            = &mut self.vmgr_state.mode
        {
            let pos = SectionSortMethod::ALL.iter().position(|m| m == sec_sort_method).unwrap_or(0);
            *sec_sort_method = SectionSortMethod::ALL[(pos + 1) % SectionSortMethod::ALL.len()];
        }
    }

    pub fn vmgr_sec_order_cycle(&mut self) {
        if let ViewMgrMode::Props { active_field: ViewPropsField::SectionSortOrder, sec_sort_order, .. }
            = &mut self.vmgr_state.mode
        {
            *sec_sort_order = match *sec_sort_order {
                SortOrder::Ascending  => SortOrder::Descending,
                SortOrder::Descending => SortOrder::Ascending,
            };
        }
    }

    pub fn vmgr_sec_sort_open_picker(&mut self) {
        if let ViewMgrMode::Props { active_field, sec_sort_method, sec_sort_order, sec_sort_picker, .. }
            = &mut self.vmgr_state.mode
        {
            match *active_field {
                ViewPropsField::SectionSorting => {
                    let pos = SectionSortMethod::ALL.iter().position(|m| m == sec_sort_method).unwrap_or(0);
                    *sec_sort_picker = Some((SecSortTarget::Method, pos));
                }
                ViewPropsField::SectionSortOrder => {
                    let pos = SortOrder::ALL.iter().position(|o| o == sec_sort_order).unwrap_or(0);
                    *sec_sort_picker = Some((SecSortTarget::Order, pos));
                }
                _ => {}
            }
        }
    }

    pub fn vmgr_sec_sort_picker_up(&mut self) {
        if let ViewMgrMode::Props { sec_sort_picker: Some((_, cursor)), .. } = &mut self.vmgr_state.mode {
            if *cursor > 0 { *cursor -= 1; }
        }
    }

    pub fn vmgr_sec_sort_picker_down(&mut self) {
        if let ViewMgrMode::Props { sec_sort_picker: Some((target, cursor)), .. } = &mut self.vmgr_state.mode {
            let len = match target {
                SecSortTarget::Method => SectionSortMethod::ALL.len(),
                SecSortTarget::Order  => SortOrder::ALL.len(),
            };
            if *cursor + 1 < len { *cursor += 1; }
        }
    }

    pub fn vmgr_sec_sort_picker_confirm(&mut self) {
        if let ViewMgrMode::Props { sec_sort_picker, sec_sort_method, sec_sort_order, active_field, .. }
            = &mut self.vmgr_state.mode
        {
            if let Some((target, cursor)) = sec_sort_picker.take() {
                match target {
                    SecSortTarget::Method => {
                        *sec_sort_method = SectionSortMethod::ALL[cursor.min(SectionSortMethod::ALL.len() - 1)];
                        // If method becomes None, move active_field back to SectionSorting
                        if *sec_sort_method == SectionSortMethod::None
                            && *active_field == ViewPropsField::SectionSortOrder
                        {
                            *active_field = ViewPropsField::SectionSorting;
                        }
                    }
                    SecSortTarget::Order => {
                        *sec_sort_order = SortOrder::ALL[cursor.min(SortOrder::ALL.len() - 1)];
                    }
                }
            }
        }
    }

    pub fn vmgr_sec_sort_picker_cancel(&mut self) {
        if let ViewMgrMode::Props { sec_sort_picker, .. } = &mut self.vmgr_state.mode {
            *sec_sort_picker = None;
        }
    }

    // ── Item Sorting sub-dialog (applies to all sections) ─────────────────────

    pub fn vmgr_open_item_sort(&mut self) {
        let v_idx = self.vmgr_state.cursor;
        let voi   = self.view_order_idx;
        let view_ref = if v_idx == voi { &self.view }
                       else { self.inactive_views.get(Self::vmgr_inact_idx(v_idx, voi)).unwrap_or(&self.view) };
        let (sn, po, poor, pna, pcid, pseq, so, soor, sna, scid, sseq) =
            if let Some(sec) = view_ref.sections.first() {
                (sec.sort_new, sec.primary_on, sec.primary_order, sec.primary_na,
                 sec.primary_cat_id, sec.primary_seq, sec.secondary_on, sec.secondary_order,
                 sec.secondary_na, sec.secondary_cat_id, sec.secondary_seq)
            } else {
                (SortNewItems::OnDemand, SortOn::None, SortOrder::Ascending, SortNa::Bottom,
                 None, SortSeq::CategoryHierarchy, SortOn::None, SortOrder::Ascending,
                 SortNa::Bottom, None, SortSeq::CategoryHierarchy)
            };
        if let ViewMgrMode::Props { active_field: ViewPropsField::ItemSorting, ref mut sort_state, .. }
            = self.vmgr_state.mode
        {
            *sort_state = SortState::Dialog {
                sort_new:         sn,
                primary_on:       po,   primary_order:    poor,  primary_na:    pna,
                primary_cat_id:   pcid, primary_seq:      pseq,
                secondary_on:     so,   secondary_order:  soor,  secondary_na:  sna,
                secondary_cat_id: scid, secondary_seq:    sseq,
                active_field:     SortField::SortNewItems,
                picker:           None,
            };
        }
    }

    pub fn vmgr_sort_tab(&mut self) {
        if let ViewMgrMode::Props {
            sort_state: SortState::Dialog { active_field, primary_on, primary_cat_id, secondary_on, secondary_cat_id, .. }, ..
        } = &mut self.vmgr_state.mode {
            let fields = sort_visible_fields(*primary_on, *primary_cat_id, *secondary_on, *secondary_cat_id);
            let pos = fields.iter().position(|f| f == active_field).unwrap_or(0);
            *active_field = fields[(pos + 1) % fields.len()];
        }
    }

    pub fn vmgr_sort_tab_back(&mut self) {
        if let ViewMgrMode::Props {
            sort_state: SortState::Dialog { active_field, primary_on, primary_cat_id, secondary_on, secondary_cat_id, .. }, ..
        } = &mut self.vmgr_state.mode {
            let fields = sort_visible_fields(*primary_on, *primary_cat_id, *secondary_on, *secondary_cat_id);
            let pos = fields.iter().position(|f| f == active_field).unwrap_or(0);
            *active_field = if pos == 0 { *fields.last().unwrap_or(&SortField::SortNewItems) } else { fields[pos - 1] };
        }
    }

    pub fn vmgr_sort_confirm(&mut self) {
        let v_idx = self.vmgr_state.cursor;
        let vals = match &self.vmgr_state.mode {
            ViewMgrMode::Props { sort_state: SortState::Dialog {
                sort_new, primary_on, primary_order, primary_na, primary_cat_id, primary_seq,
                secondary_on, secondary_order, secondary_na, secondary_cat_id, secondary_seq, ..
            }, .. } => (*sort_new, *primary_on, *primary_order, *primary_na, *primary_cat_id,
                        *primary_seq, *secondary_on, *secondary_order, *secondary_na,
                        *secondary_cat_id, *secondary_seq),
            _ => return,
        };
        let (sn, po, poor, pna, pcid, pseq, so, soor, sna, scid, sseq) = vals;
        // Apply to every section in the view at cursor.
        let voi = self.view_order_idx;
        let view = if v_idx == voi { &mut self.view }
                   else { match self.inactive_views.get_mut(Self::vmgr_inact_idx(v_idx, voi)) { Some(v) => v, None => return } };
        for sec in &mut view.sections {
            sec.sort_new         = sn;
            sec.primary_on       = po;   sec.primary_order    = poor;  sec.primary_na    = pna;
            sec.primary_cat_id   = pcid; sec.primary_seq      = pseq;
            sec.secondary_on     = so;   sec.secondary_order  = soor;  sec.secondary_na  = sna;
            sec.secondary_cat_id = scid; sec.secondary_seq    = sseq;
        }
        if self.file_path.is_some() { self.dirty = true; }
        if let ViewMgrMode::Props { ref mut sort_state, .. } = self.vmgr_state.mode {
            *sort_state = SortState::Closed;
        }
    }

    pub fn vmgr_sort_cancel(&mut self) {
        if let ViewMgrMode::Props { ref mut sort_state, .. } = self.vmgr_state.mode {
            *sort_state = SortState::Closed;
        }
    }

    pub fn vmgr_sort_open_picker(&mut self) {
        let flat_cats = flatten_cats(&self.categories);
        let (target, current_idx) = {
            let ViewMgrMode::Props {
                sort_state: SortState::Dialog {
                    active_field, primary_on, primary_order, primary_na, primary_cat_id, primary_seq,
                    secondary_on, secondary_order, secondary_na, secondary_cat_id, secondary_seq, ..
                }, ..
            } = &self.vmgr_state.mode else { return; };
            match active_field {
                SortField::SortNewItems      => (*active_field, 0),
                SortField::PrimaryOn         => (*active_field, SortOn::ALL.iter().position(|&x| x == *primary_on).unwrap_or(0)),
                SortField::PrimaryOrder      => (*active_field, SortOrder::ALL.iter().position(|&x| x == *primary_order).unwrap_or(0)),
                SortField::PrimaryNa         => (*active_field, SortNa::ALL.iter().position(|&x| x == *primary_na).unwrap_or(0)),
                SortField::PrimaryCategory   => (*active_field, primary_cat_id.and_then(|id| flat_cats.iter().position(|e| e.id == id)).unwrap_or(0)),
                SortField::PrimarySequence   => (*active_field, SortSeq::ALL.iter().position(|&x| x == *primary_seq).unwrap_or(0)),
                SortField::SecondaryOn       => (*active_field, SortOn::ALL.iter().position(|&x| x == *secondary_on).unwrap_or(0)),
                SortField::SecondaryOrder    => (*active_field, SortOrder::ALL.iter().position(|&x| x == *secondary_order).unwrap_or(0)),
                SortField::SecondaryNa       => (*active_field, SortNa::ALL.iter().position(|&x| x == *secondary_na).unwrap_or(0)),
                SortField::SecondaryCategory => (*active_field, secondary_cat_id.and_then(|id| flat_cats.iter().position(|e| e.id == id)).unwrap_or(0)),
                SortField::SecondarySequence => (*active_field, SortSeq::ALL.iter().position(|&x| x == *secondary_seq).unwrap_or(0)),
            }
        };
        if let ViewMgrMode::Props { sort_state: SortState::Dialog { ref mut picker, .. }, .. } = self.vmgr_state.mode {
            *picker = Some(SortPicker { cursor: current_idx, target });
        }
    }

    fn vmgr_sort_picker_len(&self) -> usize {
        let flat_len = flatten_cats(&self.categories).len();
        if let ViewMgrMode::Props { sort_state: SortState::Dialog { picker: Some(p), .. }, .. } = &self.vmgr_state.mode {
            match p.target {
                SortField::SortNewItems                                   => SortNewItems::ALL.len(),
                SortField::PrimaryOn | SortField::SecondaryOn             => SortOn::ALL.len(),
                SortField::PrimaryOrder | SortField::SecondaryOrder       => SortOrder::ALL.len(),
                SortField::PrimaryNa | SortField::SecondaryNa             => SortNa::ALL.len(),
                SortField::PrimaryCategory | SortField::SecondaryCategory => flat_len,
                SortField::PrimarySequence | SortField::SecondarySequence => SortSeq::ALL.len(),
            }
        } else { 0 }
    }

    pub fn vmgr_sort_picker_up(&mut self) {
        if let ViewMgrMode::Props { sort_state: SortState::Dialog { ref mut picker, .. }, .. } = self.vmgr_state.mode {
            if let Some(p) = picker { if p.cursor > 0 { p.cursor -= 1; } }
        }
    }
    pub fn vmgr_sort_picker_down(&mut self) {
        let max = self.vmgr_sort_picker_len();
        if let ViewMgrMode::Props { sort_state: SortState::Dialog { ref mut picker, .. }, .. } = self.vmgr_state.mode {
            if let Some(p) = picker { if p.cursor + 1 < max { p.cursor += 1; } }
        }
    }
    pub fn vmgr_sort_picker_pgup(&mut self, n: usize) {
        if let ViewMgrMode::Props { sort_state: SortState::Dialog { ref mut picker, .. }, .. } = self.vmgr_state.mode {
            if let Some(p) = picker { p.cursor = p.cursor.saturating_sub(n); }
        }
    }
    pub fn vmgr_sort_picker_pgdn(&mut self, n: usize) {
        let max = self.vmgr_sort_picker_len();
        if let ViewMgrMode::Props { sort_state: SortState::Dialog { ref mut picker, .. }, .. } = self.vmgr_state.mode {
            if let Some(p) = picker { if max > 0 { p.cursor = (p.cursor + n).min(max - 1); } }
        }
    }
    pub fn vmgr_sort_picker_home(&mut self) {
        if let ViewMgrMode::Props { sort_state: SortState::Dialog { ref mut picker, .. }, .. } = self.vmgr_state.mode {
            if let Some(p) = picker { p.cursor = 0; }
        }
    }
    pub fn vmgr_sort_picker_end(&mut self) {
        let max = self.vmgr_sort_picker_len();
        if let ViewMgrMode::Props { sort_state: SortState::Dialog { ref mut picker, .. }, .. } = self.vmgr_state.mode {
            if let Some(p) = picker { if max > 0 { p.cursor = max - 1; } }
        }
    }

    pub fn vmgr_sort_picker_confirm(&mut self) {
        let flat_cats = flatten_cats(&self.categories);
        let (cursor, target) = {
            let ViewMgrMode::Props { sort_state: SortState::Dialog { picker, .. }, .. } = &self.vmgr_state.mode
            else { return; };
            match picker { Some(p) => (p.cursor, p.target), None => return }
        };
        if let ViewMgrMode::Props {
            sort_state: SortState::Dialog {
                ref mut sort_new, ref mut primary_on, ref mut primary_order, ref mut primary_na,
                ref mut primary_cat_id, ref mut primary_seq,
                ref mut secondary_on, ref mut secondary_order, ref mut secondary_na,
                ref mut secondary_cat_id, ref mut secondary_seq,
                ref mut picker, ..
            }, ..
        } = self.vmgr_state.mode {
            match target {
                SortField::SortNewItems => { if let Some(&v) = SortNewItems::ALL.get(cursor) { *sort_new = v; } }
                SortField::PrimaryOn => { if let Some(&v) = SortOn::ALL.get(cursor) { if v != SortOn::Category { *primary_cat_id = None; } *primary_on = v; } }
                SortField::PrimaryOrder => { if let Some(&v) = SortOrder::ALL.get(cursor) { *primary_order = v; } }
                SortField::PrimaryNa => { if let Some(&v) = SortNa::ALL.get(cursor) { *primary_na = v; } }
                SortField::PrimaryCategory => { if let Some(e) = flat_cats.get(cursor) { *primary_cat_id = Some(e.id); } }
                SortField::PrimarySequence => { if let Some(&v) = SortSeq::ALL.get(cursor) { *primary_seq = v; } }
                SortField::SecondaryOn => { if let Some(&v) = SortOn::ALL.get(cursor) { if v != SortOn::Category { *secondary_cat_id = None; } *secondary_on = v; } }
                SortField::SecondaryOrder => { if let Some(&v) = SortOrder::ALL.get(cursor) { *secondary_order = v; } }
                SortField::SecondaryNa => { if let Some(&v) = SortNa::ALL.get(cursor) { *secondary_na = v; } }
                SortField::SecondaryCategory => { if let Some(e) = flat_cats.get(cursor) { *secondary_cat_id = Some(e.id); } }
                SortField::SecondarySequence => { if let Some(&v) = SortSeq::ALL.get(cursor) { *secondary_seq = v; } }
            }
            *picker = None;
        }
    }

    pub fn vmgr_sort_picker_cancel(&mut self) {
        if let ViewMgrMode::Props { sort_state: SortState::Dialog { ref mut picker, .. }, .. } = self.vmgr_state.mode {
            *picker = None;
        }
    }

    // ── Delete (F4) ───────────────────────────────────────────────────────────

    pub fn vmgr_open_confirm_delete(&mut self) {
        if 1 + self.inactive_views.len() <= 1 { return; }  // guard: can't delete last view
        self.vmgr_state.mode = ViewMgrMode::ConfirmDelete { yes: false };
    }

    pub fn vmgr_delete_confirm(&mut self) {
        let idx = self.vmgr_state.cursor;
        let voi = self.view_order_idx;
        if idx == voi {
            // Deleting the active view — promote the neighbor (prefer next, else prev).
            let promote_inact = if voi < self.inactive_views.len() { voi } else { voi - 1 };
            let new_view = self.inactive_views.remove(promote_inact);
            self.view = new_view;
            self.cursor     = CursorPos::SectionHead(0);
            self.col_cursor = 0;
            self.mode       = Mode::Normal;
            self.col_mode   = ColMode::Normal;
            self.sec_mode   = SectionMode::Normal;
            // After removing promote_inact, the ordered position of the new active view:
            self.view_order_idx = promote_inact.min(self.inactive_views.len());
            self.vmgr_state.cursor = self.view_order_idx;
        } else {
            let inact_idx = Self::vmgr_inact_idx(idx, voi);
            self.inactive_views.remove(inact_idx);
            // If the removed view was before the active view, active shifts left.
            if idx < voi { self.view_order_idx -= 1; }
            let count = 1 + self.inactive_views.len();
            if self.vmgr_state.cursor >= count { self.vmgr_state.cursor = count - 1; }
        }
        self.vmgr_state.mode = ViewMgrMode::Normal;
        if self.file_path.is_some() { self.dirty = true; }
    }

    pub fn vmgr_delete_cancel(&mut self) {
        self.vmgr_state.mode = ViewMgrMode::Normal;
    }
}
