use crate::model::{Item, Section, View};

pub enum CursorPos {
    SectionHead(usize),
    Item { section: usize, item: usize },
}

pub enum Mode {
    Normal,
    Edit   { original: String, buffer: String, cursor: usize },
    Create { buffer: String, cursor: usize },
}

pub struct App {
    pub view:   View,
    pub cursor: CursorPos,
    pub mode:   Mode,
    pub quit:   bool,
    next_id:    usize,
}

// Returns the byte offset of char index `n` in `s`.
fn char_to_byte(s: &str, n: usize) -> usize {
    s.char_indices().nth(n).map(|(b, _)| b).unwrap_or(s.len())
}

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
        App {
            view,
            cursor:  CursorPos::SectionHead(0),
            mode:    Mode::Normal,
            quit:    false,
            next_id: 2,
        }
    }

    fn alloc_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    // ── Navigation (Normal mode only) ─────────────────────────────────────

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
            CursorPos::Item { section, item } => {
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

    // ── Buffer cursor movement (Edit / Create) ────────────────────────────

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

    // ── Mode transitions ──────────────────────────────────────────────────

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
            CursorPos::SectionHead(s) => self.view.sections[*s].name.clone(),
            CursorPos::Item { section, item } => {
                self.view.sections[*section].items[*item].text.clone()
            }
        };
        // Cursor starts at the beginning of the text.
        self.mode = Mode::Edit { original: original.clone(), buffer: original, cursor: 0 };
    }

    // ── Input ─────────────────────────────────────────────────────────────

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
                    CursorPos::SectionHead(s)          => (*s, None),
                    CursorPos::Item { section, item }  => (*section, Some(*item)),
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
}
