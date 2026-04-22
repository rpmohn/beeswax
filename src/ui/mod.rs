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
/// The dirty marker uses `dirty_style`; the rest inherits the bar style from the Paragraph.
pub fn title_bar_top(width: u16, file_path: Option<&std::path::Path>, dirty: bool, dirty_style: ratatui::style::Style) -> ratatui::text::Line<'static> {
    let file_str = file_path.map(|p| p.display().to_string()).unwrap_or_default();
    let right = format!("beeswax v{} ", env!("CARGO_PKG_VERSION"));
    let w = width as usize;

    if dirty {
        let prefix = " File: ".to_string();
        let marker = "*".to_string();
        let suffix_and_pad = {
            let total_left = prefix.len() + 1 + file_str.chars().count();
            let pad = w.saturating_sub(total_left + right.chars().count());
            format!("{}{}{}", file_str, " ".repeat(pad), right)
        };
        ratatui::text::Line::from(vec![
            ratatui::text::Span::raw(prefix),
            ratatui::text::Span::styled(marker, dirty_style),
            ratatui::text::Span::raw(suffix_and_pad),
        ])
    } else {
        let left = format!(" File: {}", file_str);
        let pad = w.saturating_sub(left.chars().count() + right.chars().count());
        ratatui::text::Line::from(ratatui::text::Span::raw(format!("{}{}{}", left, " ".repeat(pad), right)))
    }
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
