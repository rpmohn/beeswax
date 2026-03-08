use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use crate::app::{App, CatMode, CatPropsField, FlatCat, MenuState, cat_note_for_id, cat_note_indicator, flatten_cats};
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
        let second_line = if let Some(buf) = &app.cat_search {
            format!(" Search for: {}", buf)
        } else {
            " Category Manager".to_string()
        };
        let title = Paragraph::new(vec![
            Line::from(Span::raw(title_bar_top(area.width))),
            Line::from(Span::raw(second_line)),
        ])
        .style(Style::default().add_modifier(Modifier::REVERSED));
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

    let mut lines: Vec<Line> = Vec::new();

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
                    Span::styled(hi, Style::default().add_modifier(Modifier::REVERSED)),
                    Span::raw(right),
                ]));
            }
            _ => {
                lines.push(Line::from(Span::styled(
                    " (no categories \u{2014} press INS to add)",
                    Style::default().add_modifier(Modifier::DIM),
                )));
            }
        }
    } else {
        for (row, entry) in flat.iter().enumerate() {
            let cursor_here = row == cursor;
            let note_ind     = cat_note_indicator(&app.categories, entry.id);
            let (ind, kchar) = leading(entry, note_ind);

            // ── Category row ─────────────────────────────────────────────
            let line = if cursor_here {
                match &app.cat_state.mode {
                    CatMode::Normal => Line::from(vec![
                        Span::raw(ind),
                        Span::raw(kchar),
                        Span::raw(" "),
                        Span::styled(
                            entry.name.clone(),
                            Style::default().add_modifier(Modifier::REVERSED),
                        ),
                    ]),
                    CatMode::Edit { buffer, cursor: buf_cur } => {
                        // Indicator stays; only the name is being edited
                        let (left, hi, right) = cursor_split(buffer, *buf_cur);
                        Line::from(vec![
                            Span::raw(ind),
                            Span::raw(kchar),
                            Span::raw(" "),
                            Span::raw(left),
                            Span::styled(hi, Style::default().add_modifier(Modifier::REVERSED)),
                            Span::raw(right),
                        ])
                    }
                    // Create mode: cursor row loses highlight; input row appears below
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
                        Span::styled(
                            entry.name.clone(),
                            Style::default().add_modifier(Modifier::REVERSED),
                        ),
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
            lines.push(line);

            // ── Create-mode input row after cursor ────────────────────────
            if cursor_here {
                if let CatMode::Create { buffer, cursor: buf_cur, as_child } = &app.cat_state.mode {
                    let create_depth =
                        if *as_child { entry.depth + 1 } else { entry.depth };
                    let (left, hi, right) = cursor_split(buffer, *buf_cur);
                    lines.push(Line::from(vec![
                        Span::raw(base_indent(create_depth)),
                        Span::raw(" "),  // blank indicator — new cats are Standard
                        Span::raw(" "),  // separator
                        Span::raw(left),
                        Span::styled(hi, Style::default().add_modifier(Modifier::REVERSED)),
                        Span::raw(right),
                    ]));
                }
            }
        }
    }

    frame.render_widget(Paragraph::new(lines), body_inner);

    // ── Category Properties modal ─────────────────────────────────────────────
    if let CatMode::Props {
        name_buf, short_name_buf, also_match_buf, note_file_buf,
        excl_children, match_cat_name, match_short_name,
        active_field, parent_name, kind, cat_id, ..
    } = &app.cat_state.mode {
        let modal_rect = centered_rect(64, 17, area);
        frame.render_widget(Clear, modal_rect);
        let block = Block::default().borders(Borders::ALL).title(" Category Properties ");
        frame.render_widget(block.clone(), modal_rect);
        let inner = block.inner(modal_rect);

        let dim = Style::default().add_modifier(Modifier::DIM);

        let kind_str = match kind {
            CategoryKind::Standard  => "Standard",
            CategoryKind::Date      => "Date",
            CategoryKind::Numeric   => "Numeric",
            CategoryKind::Unindexed => "Unindexed",
        };

        // Active: whole field content in REVERSED, padded to field_w.
        // Inactive: plain text padded to field_w.
        fn text_field(buf: &str, field_w: usize, active: bool) -> Span<'static> {
            let displayed: String = buf.chars().take(field_w).collect();
            let pad = field_w.saturating_sub(displayed.chars().count());
            let s = format!("{}{}", displayed, " ".repeat(pad));
            if active {
                Span::styled(s, Style::default().add_modifier(Modifier::REVERSED))
            } else {
                Span::raw(s)
            }
        }

        fn bool_field(val: bool, active: bool) -> Span<'static> {
            let s = if val { "Yes" } else { "No " };
            if active {
                Span::styled(s, Style::default().add_modifier(Modifier::REVERSED))
            } else {
                Span::raw(s)
            }
        }

        // Note field: empty → "...", non-empty → content with \n shown as ↵, truncated with "..."
        fn note_field(note: &str, field_w: usize, active: bool) -> Span<'static> {
            let text = if note.is_empty() {
                let pad = field_w.saturating_sub(3);
                format!("...{}", " ".repeat(pad))
            } else {
                let processed: String = note.chars()
                    .map(|c| if c == '\n' { '\u{21B5}' } else { c })  // ↵
                    .collect();
                let n = processed.chars().count();
                if n <= field_w {
                    let pad = field_w - n;
                    format!("{}{}", processed, " ".repeat(pad))
                } else {
                    let truncated: String = processed.chars().take(field_w.saturating_sub(3)).collect();
                    format!("{}...", truncated)
                }
            };
            if active {
                Span::styled(text, Style::default().add_modifier(Modifier::REVERSED))
            } else {
                Span::raw(text)
            }
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
            Span::raw(" Category name:  "),
            text_field(name_buf, fw, name_active),
            Span::raw(format!("    Type: {}", kind_str)),
        ]);

        // Row 2: " Parent is <name...>"(37) + "Match cat name:  " + bool
        let parent_disp: String = parent_name.chars().take(rc.saturating_sub(11)).collect();
        let parent_pad = rc.saturating_sub(11 + parent_disp.chars().count());
        let row2 = Line::from(vec![
            Span::raw(format!(" Parent is {}{}", parent_disp, " ".repeat(parent_pad))),
            Span::raw("Match cat name:  "),
            bool_field(*match_cat_name, mcat_active),
        ]);

        // Row 3: " Short name:     "(17) + field(20) + "    Match short name: " + bool
        let row3 = Line::from(vec![
            Span::raw(" Short name:     "),
            text_field(short_name_buf, fw, short_active),
            Span::raw("    Match short name: "),
            bool_field(*match_short_name, mshort_active),
        ]);

        // Row 4: " Also match:     "(17) + field(20)
        let row4 = Line::from(vec![
            Span::raw(" Also match:     "),
            text_field(also_match_buf, fw, also_active),
        ]);

        // Row 5: " Note:           "(17) + note_field(20) + "Assignment conditions:"
        let row5 = Line::from(vec![
            Span::raw(" Note:           "),
            note_field(&note_text, fw, note_active),
            Span::raw("Assignment conditions:"),
        ]);

        // Row 6: " Note file:      "(17) + field(20)
        let row6 = Line::from(vec![
            Span::raw(" Note file:      "),
            text_field(note_file_buf, fw, nfile_active),
        ]);

        // Row 7: " Exclusive children: "(21) + bool(3) + padding + "Assignment actions:"
        let excl_pad = rc.saturating_sub(21 + 3);
        let row7 = Line::from(vec![
            Span::raw(" Exclusive children: "),
            bool_field(*excl_children, excl_active),
            Span::raw(" ".repeat(excl_pad)),
            Span::raw("Assignment actions:"),
        ]);

        // Row 8: Special actions
        let row8 = Line::from(Span::raw(" Special actions:    No action"));

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
            ]),
            inner,
        );
    }

    // ── F-key bar ─────────────────────────────────────────────────────────────
    fkeys::render_fkey_bar(frame, chunks[2], app);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect { x, y, width: w, height: h }
}
