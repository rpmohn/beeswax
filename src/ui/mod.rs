pub mod catmgr;
pub mod fkeys;
pub mod input;
pub mod menu;
pub mod render;
pub mod view;
pub mod viewmgr;

/// Build the first line of the two-line title bar: " BEESWAX 0.1" left,
/// current local date/time right-aligned with one trailing space.
pub fn title_bar_top(width: u16) -> String {
    let dt = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let prefix = " BEESWAX 0.1";
    let w = width as usize;
    let pad = w.saturating_sub(prefix.len() + dt.len() + 1);
    format!("{}{}{} ", prefix, " ".repeat(pad), dt)
}

/// Split `buffer` at char index `cursor` into (left, highlighted, right).
/// `highlighted` is the char at cursor, or a space when past the end,
/// so there is always a visible reversed cell.
pub fn cursor_split(buffer: &str, cursor: usize) -> (String, String, String) {
    let mut chars = buffer.chars();
    let left:  String = chars.by_ref().take(cursor).collect();
    let hi_ch          = chars.next();
    let right: String  = chars.collect();
    let hi = match hi_ch {
        Some(c) => c.to_string(),
        None    => " ".to_string(),
    };
    (left, hi, right)
}
