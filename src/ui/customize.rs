use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use crate::app::{App, CustomizeSubMode, CUSTOMIZE_COLOR_COUNT, CUSTOMIZE_COLOR_LABELS, CURSOR_TO_FIELD, get_custom_field};
use crate::theme::{color_to_hex, theme_color_for_field, ColorScheme, parse_hex};
use super::cursor_split;

/// Label column width for color fields.
const LABEL_W: usize = 16;
/// Hex value width: "#rrggbb" or "-------".
const HEX_W: usize = 7;
/// Gap between the two color columns.
const COL_GAP: usize = 3;
/// Width of one color cell: indent(2) + label(16) + colon+space(2) + hex(7) = 27.
const CELL_W: usize = 2 + LABEL_W + 2 + HEX_W;
/// Inner dialog width (excluding border chars).
const INNER_W: usize = CELL_W + COL_GAP + CELL_W + 2;
/// Dialog total width (inner + 2 border chars).
const DIALOG_W: usize = INNER_W + 2;

/// Total display rows in the color section (includes blank-cell rows).
const COLOR_ROWS: usize = 11;

/// Dialog height: border(1) + blank(1) + nav(1) + scheme(1) + blank(1) +
///                header(1) + color_rows(11) + blank(1) + hint(1) + blank(1) + border(1) = 21
const DIALOG_H: usize = 21;

/// Row layout: (left_field_idx, right_field_idx), None = blank cell.
/// Indices refer to CUSTOMIZE_COLOR_LABELS / get_custom_field positions.
const LAYOUT: [(Option<usize>, Option<usize>); COLOR_ROWS] = [
    (Some(12), Some( 2)),  // view_bg          | selected_item_fg
    (Some(13), Some( 3)),  // view_item         | selected_item_bg
    (Some(17), None    ),  // view_head_bg      | —
    (Some(16), Some( 6)),  // view_sec_head     | dialog_bg
    (Some(15), Some( 7)),  // view_col_head     | dialog_item
    (Some(14), Some( 8)),  // view_col_entry    | dialog_label
    (Some( 4), Some( 9)),  // view_selected_fg  | dialog_label_sel_fg
    (Some( 5), Some(10)),  // view_selected_bg  | dialog_border_fg
    (None,     Some(11)),  // —                 | dialog_border_bg
    (Some( 0), None    ),  // bar_fg            | —
    (Some( 1), None    ),  // bar_bg            | —
];

/// Maps field index → cursor position in the Customize dialog.
/// Inverse of CURSOR_TO_FIELD.
const FIELD_TO_CURSOR: [usize; CUSTOMIZE_COLOR_COUNT] = [
    18, // 0:  bar_fg
    19, // 1:  bar_bg
     3, // 2:  selected_item_fg
     5, // 3:  selected_item_bg
    13, // 4:  view_selected_fg
    15, // 5:  view_selected_bg
     8, // 6:  dialog_bg
    10, // 7:  dialog_item
    12, // 8:  dialog_label
    14, // 9:  dialog_label_sel_fg
    16, // 10: dialog_border_fg
    17, // 11: dialog_border_bg
     2, // 12: view_bg
     4, // 13: view_item
    11, // 14: view_col_entry
     9, // 15: view_col_head
     7, // 16: view_sec_head
     6, // 17: view_head_bg
];

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
        .title(Line::from(" Utilities Customize ").alignment(Alignment::Center))
        .title_bottom(Line::from(" Press ENTER when done, ESC to cancel ").alignment(Alignment::Center));
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
            Span::styled("  Navigation Mode:".to_string(), label_s),
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
    lines.push(Line::from(Span::styled(
        "  Color Settings (F2=edit hex, Space=clear):              ",
        label_style,
    )));

    // Build a single color cell's spans given a field index.
    // cursor_pos is the dialog cursor value that selects this cell.
    let build_cell = |field_idx: usize, cursor_pos: usize| -> Vec<Span<'static>> {
        let on_field = st.cursor == cursor_pos && !in_picker;
        let label    = CUSTOMIZE_COLOR_LABELS[field_idx];
        let padded   = format!("{:<width$}", label, width = LABEL_W);

        // Effective RGB color for rendering the hex swatch.
        let effective_color: Option<Color> = if is_custom {
            get_custom_field(&st.custom, field_idx)
                .and_then(|opt| opt.as_deref())
                .and_then(|s| parse_hex(s))
        } else {
            theme_color_for_field(&app.theme, field_idx)
        };

        // Hex string to display.
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

        // Paired swatch: some fields are rendered with a partner's color to
        // preview how two colors look together.
        //   view_bg(12) + view_item(13): view_item fg on view_bg bg
        //   view_head_bg(17) + view_sec_head(16): view_sec_head fg on view_head_bg bg
        let paired_style: Option<Style> = {
            let pair = match field_idx {
                 0 |  1 => Some(( 0,  1)), // bar_fg fg on bar_bg bg
                 2 |  3 => Some(( 2,  3)), // selected_item_fg fg on selected_item_bg bg
                 4 |  5 => Some(( 4,  5)), // view_selected_fg fg on view_selected_bg bg
                 6 |  7 => Some(( 7,  6)), // dialog_item fg on dialog_bg bg
                 8      => Some(( 8,  6)), // dialog_label fg on dialog_bg bg
                 9      => Some(( 9,  6)), // dialog_label_sel_fg fg on dialog_bg bg
                10 | 11 => Some((10, 11)), // dialog_border_fg fg on dialog_border_bg bg
                12 | 13 => Some((13, 12)), // view_item fg on view_bg bg
                14      => Some((14, 12)), // view_col_entry fg on view_bg bg
                15      => Some((15, 17)), // view_col_head fg on view_head_bg bg
                16 | 17 => Some((16, 17)), // view_sec_head fg on view_head_bg bg
                _       => None,
            };
            pair.map(|(fg_fi, bg_fi)| {
                let (fg, bg) = if is_custom {
                    let fg = get_custom_field(&st.custom, fg_fi)
                        .and_then(|o| o.as_deref()).and_then(|s| parse_hex(s));
                    let bg = get_custom_field(&st.custom, bg_fi)
                        .and_then(|o| o.as_deref()).and_then(|s| parse_hex(s));
                    (fg, bg)
                } else {
                    (theme_color_for_field(&app.theme, fg_fi),
                     theme_color_for_field(&app.theme, bg_fi))
                };
                let mut s = Style::default();
                if let Some(fg) = fg { s = s.fg(fg); }
                if let Some(bg) = bg { s = s.bg(bg); }
                s
            })
        };

        // Normal display: render hex value in its own color (if RGB), dimmed otherwise.
        let hex_s = if on_field {
            match paired_style {
                Some(ps) => ps.add_modifier(Modifier::REVERSED),
                None if effective_color.is_some() =>
                    Style::default().fg(effective_color.unwrap()).add_modifier(Modifier::REVERSED),
                _ => sel_style,
            }
        } else {
            match paired_style {
                Some(ps) => ps,
                None => match effective_color {
                    Some(c) => Style::default().fg(c),
                    None    => dim_style,
                },
            }
        };

        vec![
            Span::styled(format!("  {}", padded), label_s),
            Span::styled(": ", colon_s),
            Span::styled(hex_str, hex_s),
        ]
    };

    // Color field rows — driven by LAYOUT.
    for &(li_opt, ri_opt) in &LAYOUT {
        let mut row_spans: Vec<Span<'static>> = Vec::new();

        match li_opt {
            Some(li) => row_spans.extend(build_cell(li, FIELD_TO_CURSOR[li])),
            None => {
                // Blank left cell — pad to cell width + gap so the right column aligns.
                row_spans.push(Span::raw(" ".repeat(CELL_W + COL_GAP)));
            }
        }

        if let Some(ri) = ri_opt {
            if li_opt.is_some() {
                row_spans.push(Span::raw("   ")); // COL_GAP
            }
            row_spans.extend(build_cell(ri, FIELD_TO_CURSOR[ri]));
        }

        lines.push(Line::from(row_spans));
    }

    // Hint
    lines.push(Line::from(""));
    let hint = if is_custom {
        "------- = terminal default"
    } else {
        "select Custom theme to edit individual colors"
    };
    lines.push(Line::from(Span::styled(hint, label_style)).alignment(Alignment::Center));
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

// Verify CURSOR_TO_FIELD and FIELD_TO_CURSOR are consistent at compile time.
const _: () = {
    let mut i = 0;
    while i < CUSTOMIZE_COLOR_COUNT {
        let cursor = FIELD_TO_CURSOR[i];
        assert!(cursor >= 2);
        let field = CURSOR_TO_FIELD[cursor - 2];
        assert!(field == i, "FIELD_TO_CURSOR / CURSOR_TO_FIELD mismatch");
        i += 1;
    }
};
