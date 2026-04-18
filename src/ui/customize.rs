use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use crate::app::{App, CustomizeSubMode, CUSTOMIZE_COLOR_COUNT, CUSTOMIZE_COLOR_LABELS, get_custom_field};
use crate::theme::{color_to_hex, theme_color_for_field, ColorScheme, parse_hex};
use super::cursor_split;

/// Label column width for color fields.
const LABEL_W: usize = 16;
/// Hex value width: "#rrggbb" or "-------".
const HEX_W: usize = 7;
/// Gap between the two color columns.
const COL_GAP: usize = 3;
/// Inner dialog width (excluding border chars).
const INNER_W: usize = 2 + LABEL_W + 2 + HEX_W + COL_GAP + LABEL_W + 2 + HEX_W + 2;
/// Dialog total width (inner + 2 border chars).
const DIALOG_W: usize = INNER_W + 2;

/// Total color field rows: pairs + solo section_fg row.
const COLOR_ROWS: usize = 12;

/// Dialog height: border(1) + blank(1) + nav(1) + scheme(1) + blank(1) +
///                header(1) + color_rows(12) + hint(1) + blank(1) + border(1) = 21
const DIALOG_H: usize = 21;

pub fn render_overlay(frame: &mut Frame, app: &App, area: Rect) {
    let Some(ref st) = app.customize_state else { return };

    let dlg = centered_rect(DIALOG_W as u16, DIALOG_H as u16, area);
    frame.render_widget(Clear, dlg);

    let is_custom = ColorScheme::ALL[st.scheme_idx] == ColorScheme::Custom;
    let in_picker = matches!(st.sub_mode, CustomizeSubMode::NavPicker { .. } | CustomizeSubMode::SchemePicker { .. });
    let in_hex_edit = matches!(st.sub_mode, CustomizeSubMode::EditHex { .. });

    let border_style = app.theme.dialog_border;
    let content_style = app.theme.dialog;
    let sel_style    = app.theme.item_selected_field;
    let label_style  = app.theme.dialog_label;
    let label_sel    = app.theme.dialog_label_sel;
    let dim_style    = app.theme.dim;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(content_style)
        .title(" Utilities Customize ")
        .title_bottom(" Press ENTER when done, ESC to cancel ");
    let inner = block.inner(dlg);
    frame.render_widget(block, dlg);

    // Build content lines
    let mut lines: Vec<Line> = Vec::new();

    // Blank line
    lines.push(Line::from(""));

    // Nav Mode field (cursor 0)
    {
        let nav_labels = ["Agenda", "Vi"];
        let val = nav_labels[st.nav_idx];
        let on_field = st.cursor == 0 && !in_picker && !in_hex_edit;
        let label_s = if on_field { label_sel } else { label_style };
        let val_s   = if on_field { sel_style  } else { content_style };
        lines.push(Line::from(vec![
            Span::styled("  Nav Mode:       ".to_string(), label_s),
            Span::styled(format!("{:<20}", val), val_s),
        ]));
    }

    // Color Theme field (cursor 1)
    {
        let val = ColorScheme::ALL[st.scheme_idx].label();
        let on_field = st.cursor == 1 && !in_picker && !in_hex_edit;
        let label_s = if on_field { label_sel } else { label_style };
        let val_s   = if on_field { sel_style  } else { content_style };
        lines.push(Line::from(vec![
            Span::styled("  Color Theme:    ".to_string(), label_s),
            Span::styled(format!("{:<20}", val), val_s),
        ]));
    }

    // Blank + header
    lines.push(Line::from(""));
    let col_header = if is_custom {
        "  Color Settings (F2=edit hex, Space=clear):              "
    } else {
        "  Color Settings (select Custom to edit):                 "
    };
    lines.push(Line::from(Span::styled(col_header, label_style)));

    // Color field rows (12 rows, 2 columns, field_idx 0..22).
    // Row 5 is special: field 10 (section_fg) has no right partner.
    // Rows 6..11 continue with fields 11..22 in pairs.
    for row in 0..COLOR_ROWS {
        let (li, ri_opt) = if row <= 4 {
            (row * 2, Some(row * 2 + 1))
        } else if row == 5 {
            (10, None) // section_fg is solo
        } else {
            let base = 11 + (row - 6) * 2;
            (base, if base + 1 < CUSTOMIZE_COLOR_COUNT { Some(base + 1) } else { None })
        };

        let left_cursor = 2 + li;

        let build_cell = |field_idx: usize, cursor_pos: usize| -> Vec<Span<'static>> {
            // cursor_pos is only meaningful when is_custom (only Custom allows cursor in color rows)
            let on_field = is_custom && st.cursor == cursor_pos && !in_picker;
            let label    = CUSTOMIZE_COLOR_LABELS[field_idx];
            let padded   = format!("{:<width$}", label, width = LABEL_W);

            // Effective RGB color for rendering the hex value in its own color.
            // For Custom: parse the stored hex string.
            // For built-in: extract from the live theme.
            let effective_color: Option<Color> = if is_custom {
                get_custom_field(&st.custom, field_idx)
                    .and_then(|opt| opt.as_deref())
                    .and_then(|s| parse_hex(s))
            } else {
                theme_color_for_field(&app.theme, field_idx)
            };

            // Hex string to display:
            // Custom: stored value or "-------" if None.
            // Built-in: hex of the RGB if available, "-----" if named/default color.
            let hex_str: String = if is_custom {
                if let Some(Some(h)) = get_custom_field(&st.custom, field_idx) {
                    format!("{:>width$}", h, width = HEX_W)
                } else {
                    "-------".to_string()
                }
            } else {
                effective_color
                    .and_then(color_to_hex)
                    .map(|s| format!("{:>width$}", s, width = HEX_W))
                    .unwrap_or_else(|| "-------".to_string())
            };

            let label_s = if on_field { label_sel } else { label_style };
            let colon_s = if on_field { label_sel } else { label_style };

            // Inline hex edit: show text cursor on the active field.
            if in_hex_edit {
                if let CustomizeSubMode::EditHex { field_idx: fi, ref buf, char_cur } = st.sub_mode {
                    if fi == field_idx && on_field {
                        let (left, hi, right) = cursor_split(buf, char_cur);
                        return vec![
                            Span::styled(format!("  {}", padded), label_s),
                            Span::styled(": ", colon_s),
                            Span::styled(left, content_style),
                            Span::styled(hi, sel_style),
                            Span::styled(right, content_style),
                        ];
                    }
                }
            }

            // Normal display: render hex value in its own color (if RGB), dimmed otherwise.
            let hex_s = if on_field {
                if let Some(c) = effective_color {
                    Style::default().fg(c).add_modifier(Modifier::REVERSED)
                } else {
                    sel_style
                }
            } else if let Some(c) = effective_color {
                Style::default().fg(c)
            } else {
                dim_style
            };

            vec![
                Span::styled(format!("  {}", padded), label_s),
                Span::styled(": ", colon_s),
                Span::styled(hex_str, hex_s),
            ]
        };

        let mut row_spans = build_cell(li, left_cursor);
        if let Some(ri) = ri_opt {
            let right_cursor = 2 + ri;
            row_spans.push(Span::raw("   "));
            row_spans.extend(build_cell(ri, right_cursor));
        }
        lines.push(Line::from(row_spans));
    }

    // Hint
    let hint = if is_custom {
        "  (F2 or type to edit hex, Space=clear, -------=terminal default)"
    } else {
        "  (select Custom theme to edit individual colors)"
    };
    lines.push(Line::from(Span::styled(hint, dim_style)));
    lines.push(Line::from(""));

    frame.render_widget(Paragraph::new(lines).style(content_style), inner);

    // ── Nav picker overlay ────────────────────────────────────────────────────
    if let CustomizeSubMode::NavPicker { cursor } = &st.sub_mode {
        let items = ["Agenda", "Vi"];
        render_picker(frame, dlg, "Navigation Mode", &items, *cursor, app);
    }

    // ── Scheme picker overlay ─────────────────────────────────────────────────
    if let CustomizeSubMode::SchemePicker { cursor } = &st.sub_mode {
        let items: Vec<&str> = ColorScheme::ALL.iter().map(|s| s.label()).collect();
        render_picker(frame, dlg, "Color Theme", &items, *cursor, app);
    }
}

/// Render a small list-picker overlay centered within `base`.
fn render_picker(frame: &mut Frame, base: Rect, title: &str, items: &[&str], cursor: usize, app: &App) {
    let max_w = items.iter().map(|s| s.len()).max().unwrap_or(0) + 4;
    let w = (max_w.max(title.len() + 4) as u16).min(base.width.saturating_sub(4));
    let h = (items.len() as u16 + 2).min(base.height.saturating_sub(4));
    let picker = centered_rect(w, h, base);
    frame.render_widget(Clear, picker);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(app.theme.dialog_border)
        .style(app.theme.dialog)
        .title(format!(" {} ", title));
    let inner = block.inner(picker);
    frame.render_widget(block, picker);

    let sel  = app.theme.item_selected_field;
    let norm = app.theme.dialog;
    let visible = inner.height as usize;
    let start   = if cursor >= visible { cursor + 1 - visible } else { 0 };

    let lines: Vec<Line> = items.iter().enumerate()
        .skip(start)
        .take(visible)
        .map(|(i, label)| {
            let s = if i == cursor { sel } else { norm };
            Line::from(Span::styled(format!(" {}", label), s))
        })
        .collect();
    frame.render_widget(Paragraph::new(lines).style(app.theme.dialog), inner);
}

fn centered_rect(w: u16, h: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(w) / 2;
    let y = area.y + area.height.saturating_sub(h) / 2;
    Rect { x, y, width: w.min(area.width), height: h.min(area.height) }
}
