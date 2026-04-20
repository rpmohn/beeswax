pub mod catmgr;
pub mod customize;
pub mod fkeys;
pub mod input;
pub mod menu;
pub mod render;
pub mod view;
pub mod viewmgr;

/// Build the first line of the two-line title bar:
/// "File: <path>" left, "beeswax <version> " right-aligned.
pub fn title_bar_top(width: u16, file_path: Option<&std::path::Path>) -> String {
    let left = match file_path {
        Some(p) => format!(" File: {}", p.display()),
        None    => " File: (none)".to_string(),
    };
    let right = format!("beeswax v{} ", env!("CARGO_PKG_VERSION"));
    let w = width as usize;
    let pad = w.saturating_sub(left.chars().count() + right.chars().count());
    format!("{}{}{}", left, " ".repeat(pad), right)
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
