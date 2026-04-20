use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use crate::app::{App, CatMode, CatPropsField, FlatCat, MenuState, cat_note_for_id, cat_note_indicator, flatten_cats};
use ratatui::layout::Alignment;
use ratatui::widgets::BorderType;
use crate::model::CategoryKind;
use super::{cursor_split, fkeys, menu, title_bar_top};

/// Spaces before the indicator column at a given depth.
fn base_indent(depth: usize) -> String {
    " ".repeat(depth * 2)
}

/// The single-character type indicator.
/// Standard cats show their note indicator (♪ inline, ♬ file); other types show their symbol.
fn kind_indicator(kind: CategoryKind, note_ind: &'static str) -> &'static str {
    match kind {
        CategoryKind::Standard  => if !note_ind.is_empty() { note_ind } else { " " },
        CategoryKind::Date      => "*",
        CategoryKind::Numeric   => "#",
        CategoryKind::Unindexed => "\u{25A1}",  // □
    }
}

/// Build the leading indent + indicator for a row.
fn leading(entry: &FlatCat, note_ind: &'static str) -> (String, &'static str) {
    (base_indent(entry.depth), kind_indicator(entry.kind, note_ind))
}

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),  // title bar
            Constraint::Min(0),     // body
            Constraint::Length(2),  // fkey bar
        ])
        .split(area);

    // ── Title bar / Menu bar (2 lines) ───────────────────────────────────────
    if matches!(app.menu, MenuState::Closed) {
        let left = if let Some(buf) = &app.cat_search {
            format!(" Search for: {}", buf)
        } else {
            " Category Manager".to_string()
        };
        // Ctrl+Arrow hints right-aligned on the second line, beneath the timestamp
        let hint = "^\u{2190}Prm ^\u{2192}Dem ^\u{2191}Up ^\u{2193}Dwn ";
        let w = area.width as usize;
        let hint_w = hint.chars().count();
        let left_w = left.chars().count();
        let pad = w.saturating_sub(left_w + hint_w);
        let second_line = format!("{}{}{}", left, " ".repeat(pad), hint);
        let title = Paragraph::new(vec![
            Line::from(Span::raw(title_bar_top(area.width, app.file_path.as_deref()))),
            Line::from(Span::raw(second_line)),
        ])
        .style(app.theme.bar);
        frame.render_widget(title, chunks[0]);
    } else {
        menu::render_bar(frame, chunks[0], app);
    }

    // ── Body ─────────────────────────────────────────────────────────────────
    let body_block = Block::default().borders(Borders::NONE);
    let body_inner = body_block.inner(chunks[1]);
    frame.render_widget(body_block, chunks[1]);

    let flat = flatten_cats(&app.categories);
    let cursor = if flat.is_empty() {
        0
    } else {
        app.cat_state.cursor.min(flat.len() - 1)
    };

    let body_h = body_inner.height as usize;
    app.cat_state.body_height.set(body_h);
    let mut lines: Vec<Line> = Vec::new();
    let mut cursor_first_line: usize = 0;
    let mut cursor_last_line:  usize = 0;
    let mut cursor_line_found = false;

    if flat.is_empty() {
        match &app.cat_state.mode {
            CatMode::Create { buffer, cursor: buf_cur, .. } => {
                // Show create input at depth 0, standard (blank indicator)
                let (left, hi, right) = cursor_split(buffer, *buf_cur);
                lines.push(Line::from(vec![
                    Span::raw(""),   // no base indent at depth 0
                    Span::raw(" "),  // blank indicator
                    Span::raw(" "),  // separator
                    Span::raw(left),
                    Span::styled(hi, app.theme.item_selected_field),
                    Span::raw(right),
                ]));
                cursor_first_line = 0;
                cursor_last_line  = 0;
                cursor_line_found = true;
            }
            _ => {
                lines.push(Line::from(Span::styled(
                    " (no categories \u{2014} press INS to add)",
                    Style::default().add_modifier(Modifier::DIM),
                )));
            }
        }
    } else {
        // For sibling-create (Ins): input row appears after all descendants of cursor.
        // For child-create (Alt+R): input row appears immediately after cursor.
        let create_show_after: Option<usize> = match &app.cat_state.mode {
            CatMode::Create { as_child: false, .. } => {
                let cursor_path = &flat[cursor].path;
                Some(flat.iter().enumerate().rev()
                    .find(|(_, e)| e.path.starts_with(cursor_path.as_slice()))
                    .map(|(i, _)| i)
                    .unwrap_or(cursor))
            }
            CatMode::Create { as_child: true, .. } => Some(cursor),
            _ => None,
        };

        for (row, entry) in flat.iter().enumerate() {
            let cursor_here = row == cursor;
            let note_ind     = cat_note_indicator(&app.categories, entry.id);
            let (ind, kchar) = leading(entry, note_ind);

            // ── Category row ─────────────────────────────────────────────
            let line = if cursor_here {
                match &app.cat_state.mode {
                    CatMode::Normal | CatMode::Move | CatMode::ConfirmDelete { .. } | CatMode::ProtectedWarning { .. } => Line::from(vec![
                        Span::raw(ind),
                        Span::raw(kchar),
                        Span::raw(" "),
                        Span::styled(entry.name.clone(), app.theme.item_selected_field),
                    ]),
                    CatMode::Edit { buffer, cursor: buf_cur } => {
                        // Indicator stays; only the name is being edited
                        let (left, hi, right) = cursor_split(buffer, *buf_cur);
                        Line::from(vec![
                            Span::raw(ind),
                            Span::raw(kchar),
                            Span::raw(" "),
                            Span::raw(left),
                            Span::styled(hi, app.theme.item_selected_field),
                            Span::raw(right),
                        ])
                    }
                    // Create mode: cursor row loses highlight; input row appears below subtree
                    CatMode::Create { .. } => Line::from(vec![
                        Span::raw(ind),
                        Span::raw(kchar),
                        Span::raw(" "),
                        Span::raw(entry.name.clone()),
                    ]),
                    // Props modal overlaid — background row stays highlighted
                    CatMode::Props { .. } => Line::from(vec![
                        Span::raw(ind),
                        Span::raw(kchar),
                        Span::raw(" "),
                        Span::styled(entry.name.clone(), app.theme.item_selected_field),
                    ]),
                }
            } else {
                Line::from(vec![
                    Span::raw(ind),
                    Span::raw(kchar),
                    Span::raw(" "),
                    Span::raw(entry.name.clone()),
                ])
            };

            if cursor_here && !cursor_line_found {
                cursor_first_line = lines.len();
                cursor_line_found = true;
            }
            lines.push(line);
            if cursor_here {
                cursor_last_line = lines.len() - 1;
            }

            // ── Create-mode input row ─────────────────────────────────────
            if Some(row) == create_show_after {
                if let CatMode::Create { buffer, cursor: buf_cur, as_child, .. } = &app.cat_state.mode {
                    let create_depth = if *as_child {
                        flat[cursor].depth + 1
                    } else {
                        flat[cursor].depth
                    };
                    let (left, hi, right) = cursor_split(buffer, *buf_cur);
                    let create_line_idx = lines.len();
                    lines.push(Line::from(vec![
                        Span::raw(base_indent(create_depth)),
                        Span::raw(" "),  // blank indicator — new cats are Standard
                        Span::raw(" "),  // separator
                        Span::raw(left),
                        Span::styled(hi, app.theme.item_selected_field),
                        Span::raw(right),
                    ]));
                    // The create input row is the cursor row for scrolling purposes
                    cursor_first_line = create_line_idx;
                    cursor_last_line  = create_line_idx;
                }
            }
        }
    }

    // ── Scroll to keep cursor visible ────────────────────────────────────────
    let mut off = app.cat_state.scroll_offset.get();
    if cursor_line_found && body_h > 0 {
        if cursor_first_line < off {
            off = cursor_first_line;
        }
        if cursor_last_line >= off + body_h {
            off = cursor_last_line + 1 - body_h;
            if cursor_first_line < off { off = cursor_first_line; }
        }
    }
    app.cat_state.scroll_offset.set(off);

    let visible: Vec<Line> = lines.into_iter().skip(off).take(body_h).collect();
    frame.render_widget(Paragraph::new(visible).style(app.theme.dialog), body_inner);

    // ── Protected-category warning modal ─────────────────────────────────────
    render_cat_protected_warning_modal(frame, app, area);

    // ── Delete confirmation modal ─────────────────────────────────────────────
    render_cat_confirm_delete_modal(frame, app, area);

    // ── Category Properties modal ─────────────────────────────────────────────
    render_cat_props_modal(frame, app, area);

    // ── F-key bar ─────────────────────────────────────────────────────────────
    fkeys::render_fkey_bar(frame, chunks[2], app);
}

/// Render the protected-category warning modal. No-op unless `CatMode::ProtectedWarning`.
fn render_cat_protected_warning_modal(frame: &mut Frame, app: &App, area: Rect) {
    let CatMode::ProtectedWarning { ref message } = app.cat_state.mode else { return; };

    let w = (message.chars().count() as u16 + 6).min(area.width);
    let modal_rect = centered_rect(w, 5, area);
    frame.render_widget(Clear, modal_rect);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .title_bottom(
            Line::from(" Press any key to continue ")
                .alignment(Alignment::Center),
        )
        .style(app.theme.dialog_border);
    frame.render_widget(block.clone(), modal_rect);
    let inner = block.inner(modal_rect);

    let rows = vec![
        Line::from(""),
        Line::from(format!("  {}", message)),
        Line::from(""),
    ];
    frame.render_widget(
        Paragraph::new(rows).style(app.theme.dialog),
        inner,
    );
}

/// Render the delete confirmation modal. No-op unless `CatMode::ConfirmDelete`.
fn render_cat_confirm_delete_modal(frame: &mut Frame, app: &App, area: Rect) {
    let CatMode::ConfirmDelete { yes, has_assignments, has_children } = app.cat_state.mode else { return; };

    let modal_rect = centered_rect(58, 5, area);
    frame.render_widget(Clear, modal_rect);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .title_bottom(
            Line::from(" Press ENTER to accept, ESC to cancel ")
                .alignment(Alignment::Center),
        )
        .style(app.theme.dialog_border);
    frame.render_widget(block.clone(), modal_rect);
    let inner = block.inner(modal_rect);

    let rev = app.theme.item_selected_field;
    let question = if has_assignments {
        "Category has assignments. Discard the category?"
    } else if has_children {
        "Category has children. Discard the category?"
    } else {
        "Discard this category?"
    };
    let val_str = if yes { "Yes" } else { "No" };

    let rows = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw(format!("  {}  ", question)),
            Span::styled(val_str.to_string(), rev),
        ]),
        Line::from(""),
    ];
    frame.render_widget(
        Paragraph::new(rows).style(app.theme.dialog),
        inner,
    );
}

/// Render the Category Properties modal centered on `area`.
/// No-op unless `app.cat_state.mode` is `CatMode::Props`.
pub fn render_cat_props_modal(frame: &mut Frame, app: &App, area: Rect) {
    if let CatMode::Props {
        name_buf, short_name_buf, also_match_buf, note_file_buf,
        excl_children, match_cat_name, match_short_name,
        active_field, parent_name, kind, cat_id, ..
    } = &app.cat_state.mode {
        let modal_rect = centered_rect(64, 17, area);
        frame.render_widget(Clear, modal_rect);
        let block = Block::default().borders(Borders::ALL)
            .title(" Category Properties ").style(app.theme.dialog_border);
        frame.render_widget(block.clone(), modal_rect);
        let inner = block.inner(modal_rect);

        let dim = Style::default().add_modifier(Modifier::DIM);

        let kind_str = match kind {
            CategoryKind::Standard  => "Standard",
            CategoryKind::Date      => "Date",
            CategoryKind::Numeric   => "Numeric",
            CategoryKind::Unindexed => "Unindexed",
        };

        let sel        = app.theme.item_selected_field;
        let dlabel     = app.theme.dialog_label;
        let dlabel_sel = app.theme.dialog_label_sel;

        // Active: whole field content in selected style, padded to field_w.
        // Inactive: plain text padded to field_w.
        fn text_field(buf: &str, field_w: usize, active: bool, sel: Style) -> Span<'static> {
            let displayed: String = buf.chars().take(field_w).collect();
            let pad = field_w.saturating_sub(displayed.chars().count());
            let s = format!("{}{}", displayed, " ".repeat(pad));
            if active { Span::styled(s, sel) } else { Span::raw(s) }
        }

        fn bool_field(val: bool, active: bool, sel: Style) -> Span<'static> {
            let s = if val { "Yes" } else { "No " };
            if active { Span::styled(s, sel) } else { Span::raw(s) }
        }

        // Note field: empty → "...", non-empty → content with \n shown as ↵, truncated with "..."
        fn note_field(note: &str, field_w: usize, active: bool, sel: Style) -> Span<'static> {
            let text = if note.is_empty() {
                let pad = field_w.saturating_sub(3);
                format!("...{}", " ".repeat(pad))
            } else {
                let processed: String = note.chars()
                    .map(|c| if c == '\n' { '\u{21B5}' } else { c })  // ↵
                    .collect();
                let n = processed.chars().count();
                if n <= field_w {
                    format!("{}{}", processed, " ".repeat(field_w - n))
                } else {
                    let truncated: String = processed.chars().take(field_w.saturating_sub(3)).collect();
                    format!("{}...", truncated)
                }
            };
            if active { Span::styled(text, sel) } else { Span::raw(text) }
        }

        // All text-field labels are 17 chars; fields are 20 chars wide → 37 total = rc.
        // Right column starts at inner col 37, occupies the remaining 25 chars (inner=62).
        let rc = 37usize;
        let fw = 20usize;
        let iw = inner.width as usize;

        // Read note live so it reflects edits made via F2/F3 without reopening Props.
        let note_text = cat_note_for_id(&app.categories, *cat_id);

        let name_active   = *active_field == CatPropsField::Name;
        let short_active  = *active_field == CatPropsField::ShortName;
        let also_active   = *active_field == CatPropsField::AlsoMatch;
        let note_active   = *active_field == CatPropsField::Note;
        let nfile_active  = *active_field == CatPropsField::NoteFile;
        let excl_active   = *active_field == CatPropsField::ExclChildren;
        let mcat_active   = *active_field == CatPropsField::MatchCatName;
        let mshort_active = *active_field == CatPropsField::MatchShortName;

        // Row 0: blank
        let row0 = Line::from("");

        // Row 1: " Category name:  "(17) + field(20) + "    Type: <kind>"
        let row1 = Line::from(vec![
            Span::styled(" Category name:  ", if name_active { dlabel_sel } else { dlabel }),
            text_field(name_buf, fw, name_active, sel),
            Span::raw(format!("    Type: {}", kind_str)),
        ]);

        // Row 2: " Parent is <name...>"(37) + "Match cat name:  " + bool
        let parent_disp: String = parent_name.chars().take(rc.saturating_sub(11)).collect();
        let parent_pad = rc.saturating_sub(11 + parent_disp.chars().count());
        let row2 = Line::from(vec![
            Span::styled(format!(" Parent is {}{}", parent_disp, " ".repeat(parent_pad)), dlabel),
            Span::styled("Match cat name:  ", if mcat_active { dlabel_sel } else { dlabel }),
            bool_field(*match_cat_name, mcat_active, sel),
        ]);

        // Row 3: " Short name:     "(17) + field(20) + "    Match short name: " + bool
        let row3 = Line::from(vec![
            Span::styled(" Short name:     ", if short_active { dlabel_sel } else { dlabel }),
            text_field(short_name_buf, fw, short_active, sel),
            Span::styled("    Match short name: ", if mshort_active { dlabel_sel } else { dlabel }),
            bool_field(*match_short_name, mshort_active, sel),
        ]);

        // Row 4: " Also match:     "(17) + field(20)
        let row4 = Line::from(vec![
            Span::styled(" Also match:     ", if also_active { dlabel_sel } else { dlabel }),
            text_field(also_match_buf, fw, also_active, sel),
        ]);

        // Row 5: " Note:           "(17) + note_field(20) + "Assignment conditions:"
        let row5 = Line::from(vec![
            Span::styled(" Note:           ", if note_active { dlabel_sel } else { dlabel }),
            note_field(&note_text, fw, note_active, sel),
            Span::styled("Assignment conditions:", dlabel),
        ]);

        // Row 6: " Note file:      "(17) + field(20)
        let row6 = Line::from(vec![
            Span::styled(" Note file:      ", if nfile_active { dlabel_sel } else { dlabel }),
            text_field(note_file_buf, fw, nfile_active, sel),
        ]);

        // Row 7: " Exclusive children: "(21) + bool(3) + padding + "Assignment actions:"
        let excl_pad = rc.saturating_sub(21 + 3);
        let row7 = Line::from(vec![
            Span::styled(" Exclusive children: ", if excl_active { dlabel_sel } else { dlabel }),
            bool_field(*excl_children, excl_active, sel),
            Span::raw(" ".repeat(excl_pad)),
            Span::styled("Assignment actions:", dlabel),
        ]);

        // Row 8: Special actions
        let row8 = Line::from(Span::styled(" Special actions:    No action", dlabel));

        // Row 9: blank
        let row9 = Line::from("");

        // Row 10: Statistics
        let row10 = Line::from(Span::styled(" Statistics:         ...", dim));

        // Row 11: Advanced settings
        let row11 = Line::from(Span::styled(" Advanced settings:  ...", dim));

        // Row 12: blank
        let row12 = Line::from("");

        // Row 13: centered help line
        let help_text = "\u{2500}\u{2500}\u{2500} Press ENTER when done, ESC to cancel \u{2500}\u{2500}\u{2500}";
        let lpad = iw.saturating_sub(help_text.chars().count()) / 2;
        let row13 = Line::from(Span::raw(format!("{}{}", " ".repeat(lpad), help_text)));

        // Row 14: blank
        let row14 = Line::from("");

        frame.render_widget(
            Paragraph::new(vec![
                row0, row1, row2, row3, row4, row5, row6, row7, row8,
                row9, row10, row11, row12, row13, row14,
            ]).style(app.theme.dialog),
            inner,
        );
    }
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect { x, y, width: w, height: h }
}
