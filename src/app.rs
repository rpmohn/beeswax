use crate::model::{Category, CategoryKind, Item, Section, View};

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
    Edit   { original: String, buffer: String, cursor: usize },
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

// ── App impl ──────────────────────────────────────────────────────────────────

impl App {
    pub fn new() -> Self {
        let section = Section {
            id:    1,
            name:  "Initial Section".to_string(),
            items: Vec::new(),
        };
        let view = View {
            id:       1,
            name:     "Initial View".to_string(),
            sections: vec![section],
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

    pub fn begin_create_blank(&mut self) {
        if !matches!(self.mode, Mode::Normal) { return; }
        self.mode = Mode::Create { buffer: String::new(), cursor: 0 };
    }

    pub fn begin_edit(&mut self) {
        if !matches!(self.mode, Mode::Normal) { return; }
        let original = match &self.cursor {
            CursorPos::SectionHead(s)         => self.view.sections[*s].name.clone(),
            CursorPos::Item { section, item } => {
                self.view.sections[*section].items[*item].text.clone()
            }
        };
        self.mode = Mode::Edit { original: original.clone(), buffer: original, cursor: 0 };
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

    pub fn confirm(&mut self) {
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
                self.view.sections[sec_idx].items.insert(pos, Item { id, text });
                self.cursor = CursorPos::Item { section: sec_idx, item: pos };
            }
            Mode::Edit { buffer, .. } => {
                let text = buffer.trim().to_string();
                if text.is_empty() { return; }
                match &self.cursor {
                    CursorPos::SectionHead(s) => {
                        self.view.sections[*s].name = text;
                    }
                    CursorPos::Item { section, item } => {
                        self.view.sections[*section].items[*item].text = text;
                    }
                }
            }
            Mode::Normal => {}
        }
    }

    pub fn cancel(&mut self) {
        self.mode = Mode::Normal;
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
}
