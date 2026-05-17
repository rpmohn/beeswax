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

/// Renders a scrollable text-entry field of fixed width, keeping the cursor always visible.
/// Returns spans filling exactly `field_w` columns: `cursor_sty` for the character under the
/// cursor, `text_sty` for all other characters (including trailing padding).
pub fn text_field_spans(
    buf: &str,
    cursor: usize,
    scroll: usize,
    field_w: usize,
    text_sty: ratatui::style::Style,
    cursor_sty: ratatui::style::Style,
) -> Vec<ratatui::text::Span<'static>> {
    if field_w == 0 { return vec![]; }
    let chars: Vec<char> = buf.chars().collect();
    let cur   = cursor.min(chars.len());
    // Clamp stored scroll so cursor is always visible (lazy: window moves only as needed).
    let start = scroll.min(cur).max(cur.saturating_sub(field_w - 1));
    let visible: String = chars[start..].iter().take(field_w).collect();
    let cur_in_win = cur - start;
    let (left, hi, right) = cursor_split(&visible, cur_in_win);
    let used = left.chars().count() + 1 + right.chars().count();
    let pad  = field_w.saturating_sub(used);
    let mut spans = vec![
        ratatui::text::Span::styled(left,  text_sty),
        ratatui::text::Span::styled(hi,    cursor_sty),
        ratatui::text::Span::styled(right, text_sty),
    ];
    if pad > 0 {
        spans.push(ratatui::text::Span::styled(" ".repeat(pad), text_sty));
    }
    spans
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
