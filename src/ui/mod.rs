pub mod catmgr;
pub mod input;
pub mod menu;
pub mod render;
pub mod view;

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
