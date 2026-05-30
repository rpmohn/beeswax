pub mod catmgr;
pub mod customize;
pub mod fkeys;
pub mod input;
pub mod menu;
pub mod render;
pub mod view;
pub mod viewmgr;

use ratatui::layout::Rect;

/// Compute a centred Rect of `width` × `height` inside `area`.
pub fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect { x, y, width: w, height: h }
}

/// Return `selected` style when `active`, otherwise `normal`.
/// Used to compute dialog-label styles based on whether a field is active.
pub fn dlabel_style(active: bool, normal: ratatui::style::Style, selected: ratatui::style::Style) -> ratatui::style::Style {
    if active { selected } else { normal }
}

/// Pad `s` to width `w` with spaces, or truncate it to `w` chars.
pub fn pad_or_trunc(s: &str, w: usize) -> String {
    let len = s.chars().count();
    if len >= w {
        s.chars().take(w).collect()
    } else {
        format!("{}{}", s, " ".repeat(w - len))
    }
}

/// Build a centred Yes/No button line for confirmation dialogs.
/// `inner_w` is the available inner width. `yes_active` selects which button is reversed.
pub fn yes_no_line(inner_w: usize, yes_active: bool, rev: ratatui::style::Style) -> ratatui::text::Line<'static> {
    let yes_label = "[ Yes ]";
    let no_label  = "[ No ]";
    let gap = inner_w.saturating_sub(yes_label.len() + no_label.len() + 6);
    let lpad = gap / 2;
    let yes_sty = if yes_active { rev } else { ratatui::style::Style::default() };
    let no_sty  = if yes_active { ratatui::style::Style::default() } else { rev };
    ratatui::text::Line::from(vec![
        ratatui::text::Span::raw(" ".repeat(lpad)),
        ratatui::text::Span::styled(yes_label, yes_sty),
        ratatui::text::Span::raw("      "),
        ratatui::text::Span::styled(no_label, no_sty),
    ])
}

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
