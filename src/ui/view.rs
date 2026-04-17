use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};
use crate::app::{App, AppScreen, AskChoice, AssignMode, CatMode, ChoicesKind, ColFormField, ColMode, ColPos,
                 CursorPos, FilterState, MenuState, Mode, PasswordPurpose, PropsField, SaveState,
                 SecPropsField, SectionFormField, SectionInsert, SectionMode, SortField, SortState,
                 TimeField, ViewAddField, ViewMode, cat_note_indicator, col_autocomplete_match,
                 col_display_values, flatten_cats, format_date_value,
                 visible_item_indices};
use crate::model::{FilterOp, SortNewItems, SortOn, SortOrder, SortSeq};
use crate::model::ColFormat;
use crate::model::{CategoryKind, Column, DateDisplay, Clock, DateFmtCode};
use super::{cursor_split, fkeys, menu, title_bar_top};

const SECTION_PREFIX:    &str = " ";
const ITEM_PREFIX:       &str = "    \u{2022} ";   // bullet  •
const ITEM_NOTE_PREFIX:  &str = "    \u{266A} ";   // musical eighth note ♪ (single note indicator)
const ITEM_DONE_PREFIX:  &str = "    \u{203C} ";   // double exclamation mark ‼

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

    // ── Title bar / Menu bar (2 lines) ───────────────────────────────────
    if matches!(app.menu, MenuState::Closed) {
        let second_line = if let Some((buf, cur)) = &app.item_search {
            let chars: Vec<char> = buf.chars().collect();
            let left: String  = chars[..*cur].iter().collect();
            let right: String = chars[*cur..].iter().collect();
            format!(" Search: {}|{}", left, right)
        } else if let Mode::ItemProps { cursor, edit_buf, .. } = &app.mode {
            if edit_buf.is_some() {
                " Type to edit. Press ENTER to save, ESC to cancel.".to_string()
            } else {
                match cursor {
                    0 => " Press F2 to edit the item text.",
                    1 => " Press F2 to edit the note.",
                    2 => " Press F2 to edit the note file.",
                    3 => " Item statistics are read-only.",
                    _ => " Press F2 to edit. Del to remove assignment.",
                }.to_string()
            }
        } else if matches!(app.screen, AppScreen::ViewMgr) {
            let left = format!(" View: {}", app.view.name);
            let hint = "^\u{2191}Up ^\u{2193}Dwn ";
            let w = area.width as usize;
            let pad = w.saturating_sub(left.chars().count() + hint.chars().count());
            format!("{}{}{}", left, " ".repeat(pad), hint)
        } else {
            format!(" View: {}", app.view.name)
        };
        let title = Paragraph::new(vec![
            Line::from(Span::raw(title_bar_top(area.width))),
            Line::from(Span::raw(second_line)),
        ])
        .style(app.theme.bar);
        frame.render_widget(title, chunks[0]);
    } else {
        menu::render_bar(frame, chunks[0], app);
    }

    // ── Body ─────────────────────────────────────────────────────────────
    let body_block = Block::default().borders(Borders::NONE);
    let body_inner = body_block.inner(chunks[1]);
    frame.render_widget(body_block, chunks[1]);

    // Column layout: left columns | main items column | right columns.
    // Each added column occupies col.width + 1 chars (the +1 is the '·' prefix).
    let total_body_w = body_inner.width as usize;
    let added_w: usize = app.view.columns.iter().map(|c| c.width + 1).sum();
    let lc = app.view.left_count;
    let left_cols  = &app.view.columns[..lc];
    let right_cols = &app.view.columns[lc..];
    // Reserve 1 extra char for the gap between the main column and right columns.
    let right_gap = if right_cols.is_empty() { 0 } else { 1 };
    let main_col_w = total_body_w.saturating_sub(added_w + right_gap);

    // Determine which column index (0-based into view.columns) is active.
    // active_col is Some(i) when col_cursor > 0 and col_cursor-1 == i.
    let active_col: Option<usize> = if app.col_cursor > 0 {
        Some(app.col_cursor - 1)
    } else {
        None
    };

    let done_cat_id: Option<usize> = flatten_cats(&app.categories)
        .iter().find(|c| c.name == "Done").map(|c| c.id);

    let body_h = body_inner.height as usize;
    let mut lines: Vec<Line> = Vec::new();
    let mut cursor_first_line: usize = 0;
    let mut cursor_last_line:  usize = 0;
    let mut cursor_line_found  = false;
    let mut lmap: Vec<(CursorPos, usize, usize)> = Vec::new();

    // Precompute which sections are visible (non-empty when hide_empty_sections is on).
    let visible_sections: Vec<usize> = if app.view.hide_empty_sections {
        (0..app.view.sections.len())
            .filter(|&s| !visible_item_indices(&app.items, &app.view, s, &app.categories).is_empty())
            .collect()
    } else {
        (0..app.view.sections.len()).collect()
    };

    for (display_pos, &s_idx) in visible_sections.iter().enumerate() {
        let section = &app.view.sections[s_idx];
        let cursor_on_head = matches!(&app.cursor, CursorPos::SectionHead(i) if *i == s_idx);

        // ── Section head row ─────────────────────────────────────────────
        // Note indicator: leading space replaced with ♪/♬ when backing category has a note/file.
        let sec_note_ind = cat_note_indicator(&app.categories, section.cat_id);
        let sec_prefix = if !sec_note_ind.is_empty() { sec_note_ind } else { SECTION_PREFIX };
        let sec_display_name = section.name.clone();

        // Left column header cells
        let head_col_text_style = if cursor_on_head && matches!(app.mode, Mode::Normal) {
            app.theme.item_selected_line
        } else {
            app.theme.view_col_head
        };
        let left_head_vals: Vec<String> = left_cols.iter().map(|c| c.name.clone()).collect();
        let left_active  = if cursor_on_head { active_col.filter(|&i| i < lc) } else { None };
        let left_head_edit = head_cell_edit(left_active, &app.mode);
        let left_head_spans = col_cells(left_cols, &left_head_vals,
                                        left_active, left_head_edit, None, "", head_col_text_style, app.theme.item_selected_field);

        // Main column content
        // When left columns exist, indent section header to align with item text.
        let sec_indent = if lc > 0 {
            ITEM_PREFIX.chars().count().saturating_sub(SECTION_PREFIX.chars().count())
        } else { 0 };
        let pfx_w      = SECTION_PREFIX.chars().count() + sec_indent;
        let max_name_w = main_col_w.saturating_sub(pfx_w);
        let ind = " ".repeat(sec_indent);
        let (mut head_spans, head_used): (Vec<Span<'static>>, usize) = if cursor_on_head {
            match &app.mode {
                Mode::Normal => {
                    let name: String = sec_display_name.chars().take(max_name_w).collect();
                    let w = pfx_w + name.chars().count();
                    let name_style = if app.col_cursor == 0 {
                        app.theme.item_selected_field.add_modifier(Modifier::BOLD)
                    } else {
                        app.theme.item_selected_line.add_modifier(Modifier::BOLD)
                    };
                    (vec![
                        Span::styled(ind, app.theme.item_selected_line),
                        Span::styled(sec_prefix, app.theme.item_selected_line),
                        Span::styled(name, name_style),
                    ], w)
                }
                Mode::Edit { buffer, cursor, col, .. } if *col == 0 => {
                    let (left, hi, right) = cursor_split(buffer, *cursor);
                    let w = pfx_w + buffer.chars().count();
                    let sec_style = app.theme.view_sec_head.add_modifier(Modifier::BOLD);
                    (vec![
                        Span::raw(ind),
                        Span::raw(sec_prefix),
                        Span::styled(left,  sec_style),
                        Span::styled(hi,    app.theme.item_selected_field),
                        Span::styled(right, sec_style),
                    ], w)
                }
                _ => {
                    let name: String = sec_display_name.chars().take(max_name_w).collect();
                    let w = pfx_w + name.chars().count();
                    (vec![Span::raw(ind), Span::raw(sec_prefix),
                          Span::styled(name, app.theme.view_sec_head.add_modifier(Modifier::BOLD))], w)
                }
            }
        } else {
            let name: String = sec_display_name.chars().take(max_name_w).collect();
            let w = pfx_w + name.chars().count();
            (vec![Span::raw(ind), Span::raw(sec_prefix),
                  Span::styled(name, app.theme.view_sec_head.add_modifier(Modifier::BOLD))], w)
        };
        if head_used < main_col_w {
            let pad = " ".repeat(main_col_w - head_used);
            if cursor_on_head && matches!(app.mode, Mode::Normal) {
                head_spans.push(Span::styled(pad, app.theme.item_selected_line));
            } else {
                head_spans.push(Span::raw(pad));
            }
        }

        // Right column header cells
        let right_head_vals: Vec<String> = right_cols.iter().map(|c| c.name.clone()).collect();
        let right_active = if cursor_on_head { active_col.filter(|&i| i >= lc).map(|i| i - lc) } else { None };
        let right_head_edit = head_cell_edit(right_active.map(|i| i + lc), &app.mode);
        let right_head_spans = col_cells(right_cols, &right_head_vals,
                                         right_active, right_head_edit, None, "", head_col_text_style, app.theme.item_selected_field);

        let mut row = left_head_spans;
        row.extend(head_spans);
        if !right_cols.is_empty() {
            if cursor_on_head && matches!(app.mode, Mode::Normal) {
                row.push(Span::styled(" ", app.theme.item_selected_line));
            } else {
                row.push(Span::raw(" "));
            }
        }
        row.extend(right_head_spans);
        if cursor_on_head && matches!(app.mode, Mode::Normal) {
            row.push(Span::styled(" ".repeat(total_body_w), app.theme.item_selected_line));
        }
        if cursor_on_head { cursor_first_line = lines.len(); cursor_line_found = true; }
        let head_first = lines.len();
        let head_line_style = if cursor_on_head && matches!(app.mode, Mode::Normal) {
            app.theme.item_selected_line
        } else {
            app.theme.view_head_bg
        };
        lines.push(Line::from(row).style(head_line_style));

        if cursor_on_head {
            if let Mode::Create { buffer, cursor } = &app.mode {
                let used  = ITEM_PREFIX.chars().count() + buffer.chars().count();
                let empty: Vec<String> = app.view.columns.iter().map(|_| String::new()).collect();
                let left_empty  = &empty[..lc];
                let right_empty = &empty[lc..];
                let mut spans = col_cells(left_cols, left_empty, None, None, None, "\u{00B7}", app.theme.view_col, app.theme.item_selected_field);
                spans.extend(input_row_spans(buffer, *cursor, app.theme.view_item, app.theme.item_selected_field));
                if used < main_col_w { spans.push(Span::raw(" ".repeat(main_col_w - used))); }
                if !right_cols.is_empty() { spans.push(Span::raw(" ")); }
                spans.extend(col_cells(right_cols, right_empty, None, None, None, "\u{00B7}", app.theme.view_col, app.theme.item_selected_field));
                lines.push(Line::from(spans));
            }
            cursor_last_line = lines.len() - 1;
        }
        lmap.push((CursorPos::SectionHead(s_idx), head_first, lines.len() - 1));

        // ── Item rows ────────────────────────────────────────────────────
        let sec_item_indices = visible_item_indices(&app.items, &app.view, s_idx, &app.categories);
        for (i_idx, &gi) in sec_item_indices.iter().enumerate() {
            let item = &app.items[gi];
            let cursor_on_item = matches!(
                &app.cursor,
                CursorPos::Item { section: si, item: ii } if *si == s_idx && *ii == i_idx
            );

            let is_done    = done_cat_id.map(|id| item.values.contains_key(&id)).unwrap_or(false);
            let item_pfx   = if is_done { ITEM_DONE_PREFIX } else if item.note.is_empty() { ITEM_PREFIX } else { ITEM_NOTE_PREFIX };
            let pfx_w      = item_pfx.chars().count();
            let max_text_w = main_col_w.saturating_sub(pfx_w);

            // Cache the item text column width so app.rs navigation methods can use it.
            if cursor_on_item { app.item_wrap_width.set(max_text_w); }

            // Determine the text source for wrapping: buffer when editing col=0, else item.text.
            let editing_text = cursor_on_item && matches!(&app.mode, Mode::Edit { col, .. } if *col == 0);
            let wrap_src = if editing_text {
                if let Mode::Edit { buffer, .. } = &app.mode { buffer.as_str() } else { item.text.as_str() }
            } else {
                item.text.as_str()
            };
            let (wrapped_lines, wrap_starts) = word_wrap_lines(wrap_src, max_text_w);
            let n_text_rows = wrapped_lines.len();

            // Multi-line column values: Vec<Vec<String>> (outer=columns, inner=assignments).
            // Date columns always produce one entry; standard columns may produce multiple.
            let all_vals_lines: Vec<Vec<String>> = app.view.columns.iter()
                .map(|c| {
                    if c.date_fmt.is_some() {
                        let v = item.values.get(&c.cat_id).cloned().unwrap_or_default();
                        vec![v]   // col_cells formats via date_fmt
                    } else {
                        col_display_values(&item.values, c.cat_id, c.format, &app.categories)
                    }
                })
                .collect();

            // How many display rows does this item need?
            let n_col_rows = all_vals_lines.iter().map(|v| v.len().max(1)).max().unwrap_or(1);
            let n_rows     = n_text_rows.max(n_col_rows);

            // Which cell (if any) is in edit mode — only applies to row 0.
            let (item_active_col, item_cell_edit): (Option<usize>, Option<(&str, usize)>) =
                if cursor_on_item {
                    match &app.mode {
                        Mode::Edit { col, buffer, cursor, .. } if *col > 0 =>
                            (Some(col - 1), Some((buffer.as_str(), *cursor))),
                        Mode::Normal if app.col_cursor > 0 =>
                            (Some(app.col_cursor - 1), None),
                        _ => (None, None),
                    }
                } else {
                    (None, None)
                };

            // Autocomplete hint for standard-column edit cells (row 0 only).
            let autocomplete_hint: Option<String> =
                if cursor_on_item {
                    if let Mode::Edit { col, buffer, .. } = &app.mode {
                        if *col > 0 {
                            let col_idx = col - 1;
                            if col_idx < app.view.columns.len()
                                && app.view.columns[col_idx].date_fmt.is_none()
                            {
                                let col_cat_id = app.view.columns[col_idx].cat_id;
                                let trimmed = buffer.trim();
                                col_autocomplete_match(&app.categories, col_cat_id, trimmed)
                                    .map(|(_, name)| {
                                        let pfx = trimmed.chars().count();
                                        name.chars().skip(pfx).collect::<String>()
                                    })
                            } else { None }
                        } else { None }
                    } else { None }
                } else { None };

            let left_item_active  = item_active_col.filter(|&i| i < lc);
            let right_item_active = item_active_col.filter(|&i| i >= lc).map(|i| i - lc);

            // Compute the edit cursor position once (doesn't vary by row).
            let edit_cursor_line_col: Option<(usize, usize)> = if editing_text {
                if let Mode::Edit { cursor, .. } = &app.mode {
                    Some(find_cursor_in_wrapped(&wrap_starts, &wrapped_lines, *cursor))
                } else { None }
            } else { None };

            // When the cursor sits at the end of a line that fills max_text_w exactly, a
            // trailing cursor-block space would overflow into the right columns.  Remap:
            //   • non-last line → show cursor at start of the next wrapped line
            //   • last line     → overlay the cursor on the last visible character
            let edit_cursor_display: Option<(usize, usize)> = edit_cursor_line_col.map(|(cl, cc)| {
                let lw = wrapped_lines[cl].chars().count();
                if cc >= lw && pfx_w + lw >= main_col_w {
                    if cl + 1 < n_text_rows       { (cl + 1, 0) }
                    else if lw > 0                { (cl, lw - 1) }
                    else                          { (cl, cc) }
                } else {
                    (cl, cc)
                }
            });

            if cursor_on_item { cursor_first_line = lines.len(); cursor_line_found = true; }
            let item_first = lines.len();
            for row_i in 0..n_rows {
                // Values for this sub-row (empty string if this column has fewer assignments).
                let left_vals_row: Vec<String> = all_vals_lines[..lc].iter()
                    .map(|v| v.get(row_i).cloned().unwrap_or_default())
                    .collect();
                let right_vals_row: Vec<String> = all_vals_lines[lc..].iter()
                    .map(|v| v.get(row_i).cloned().unwrap_or_default())
                    .collect();

                // Edit / active highlight applies to the focused sub-row.
                let (left_active, right_active, left_edit, right_edit, hint_ref) = if row_i == app.sub_row {
                    let le = item_cell_edit.filter(|_| left_item_active.is_some());
                    let re = item_cell_edit.filter(|_| right_item_active.is_some());
                    let h  = autocomplete_hint.as_deref();
                    (left_item_active, right_item_active, le, re, h)
                } else {
                    (None, None, None, None, None)
                };

                let col_text_style = if cursor_on_item {
                    app.theme.item_selected_line
                } else {
                    app.theme.view_col
                };
                let left_item_spans = col_cells(left_cols, &left_vals_row,
                                                left_active, left_edit,
                                                hint_ref.filter(|_| left_active.is_some()), "\u{00B7}", col_text_style, app.theme.item_selected_field);

                // Main column content: item text (word-wrapped) across rows.
                let is_text_row = row_i < n_text_rows;
                let line_text = if is_text_row { wrapped_lines[row_i].clone() } else { String::new() };
                let indent = if row_i == 0 { item_pfx.to_string() } else { " ".repeat(pfx_w) };

                let mut item_spans: Vec<Span<'static>> = if cursor_on_item && is_text_row {
                    match &app.mode {
                        Mode::Normal => {
                            // Prefix (indent + icon) always gets line style; only the text gets field style.
                            let text_style = if app.col_cursor == 0 {
                                app.theme.item_selected_field
                            } else {
                                app.theme.item_selected_line
                            };
                            vec![
                                Span::styled(indent, app.theme.item_selected_line),
                                Span::styled(line_text, text_style),
                            ]
                        }
                        Mode::Edit { col, .. } if *col == 0 => {
                            let body_style = app.theme.view_bg.patch(app.theme.view_item);
                            if let Some((dcl, dcc)) = edit_cursor_display {
                                if row_i == dcl {
                                    let (left, hi, right) = cursor_split(&line_text, dcc);
                                    vec![
                                        Span::styled(indent, app.theme.item_selected_line),
                                        Span::styled(left,   body_style),
                                        Span::styled(hi,     app.theme.item_selected_field),
                                        Span::styled(right,  body_style),
                                    ]
                                } else {
                                    vec![Span::styled(indent, app.theme.item_selected_line),
                                         Span::styled(line_text, body_style)]
                                }
                            } else {
                                vec![Span::styled(indent, app.theme.item_selected_line),
                                     Span::styled(line_text, body_style)]
                            }
                        }
                        Mode::Edit { .. } =>
                            vec![Span::styled(indent, app.theme.item_selected_line),
                                 Span::styled(line_text, app.theme.item_selected_line)],
                        Mode::Create { .. } | Mode::ConfirmDeleteItem { .. } | Mode::ConfirmDiscardItem { .. } | Mode::ItemProps { .. } =>
                            vec![Span::styled(indent, app.theme.item_selected_line),
                                 Span::styled(line_text, app.theme.item_selected_line)],
                    }
                } else if is_text_row {
                    vec![Span::styled(indent, app.theme.view_item), Span::styled(line_text, app.theme.view_item)]
                } else {
                    vec![Span::raw(" ".repeat(main_col_w))]
                };
                // Pad to main_col_w.  cursor_extra accounts for the trailing cursor-block
                // space when the cursor is past the last char of a short line.
                let text_chars = if is_text_row {
                    let line_w = wrapped_lines[row_i].chars().count();
                    let cursor_extra = if let Some((dcl, dcc)) = edit_cursor_display {
                        if row_i == dcl && dcc >= line_w { 1 } else { 0 }
                    } else { 0 };
                    pfx_w + line_w + cursor_extra
                } else {
                    main_col_w
                };
                if text_chars < main_col_w {
                    let pad = " ".repeat(main_col_w - text_chars);
                    if cursor_on_item {
                        item_spans.push(Span::styled(pad, app.theme.item_selected_line));
                    } else {
                        item_spans.push(Span::raw(pad));
                    }
                }

                let right_item_spans = col_cells(right_cols, &right_vals_row,
                                                 right_active, right_edit,
                                                 hint_ref.filter(|_| right_active.is_some()), "\u{00B7}", col_text_style, app.theme.item_selected_field);

                let mut row = left_item_spans;
                row.extend(item_spans);
                if !right_cols.is_empty() {
                    if cursor_on_item {
                        row.push(Span::styled(" ", app.theme.item_selected_line));
                    } else {
                        row.push(Span::raw(" "));
                    }
                }
                row.extend(right_item_spans);
                if cursor_on_item {
                    row.push(Span::styled(" ".repeat(total_body_w), app.theme.item_selected_line));
                }
                lines.push(Line::from(row));
            }

            if cursor_on_item {
                if let Mode::Create { buffer, cursor } = &app.mode {
                    let create_pfx_w = ITEM_PREFIX.chars().count();
                    let create_text_w = main_col_w.saturating_sub(create_pfx_w);
                    let (create_lines, create_starts) = word_wrap_lines(buffer, create_text_w);
                    let (cur_line, cur_col) = find_cursor_in_wrapped(&create_starts, &create_lines, *cursor);
                    // Remap cursor when it would overflow (same logic as edit mode above).
                    let (dcl, dcc) = {
                        let lw = create_lines[cur_line].chars().count();
                        if cur_col >= lw && create_pfx_w + lw >= main_col_w {
                            if cur_line + 1 < create_lines.len() { (cur_line + 1, 0) }
                            else if lw > 0                        { (cur_line, lw - 1) }
                            else                                  { (cur_line, cur_col) }
                        } else {
                            (cur_line, cur_col)
                        }
                    };
                    let empty: Vec<String> = app.view.columns.iter().map(|_| String::new()).collect();
                    let left_empty  = &empty[..lc];
                    let right_empty = &empty[lc..];
                    for (row_i, line) in create_lines.iter().enumerate() {
                        let indent = if row_i == 0 { ITEM_PREFIX.to_string() } else { " ".repeat(create_pfx_w) };
                        let (left, hi, right) = if row_i == dcl {
                            cursor_split(line, dcc)
                        } else {
                            (line.clone(), String::new(), String::new())
                        };
                        let mut spans = if row_i == 0 {
                            col_cells(left_cols, left_empty, None, None, None, "\u{00B7}", app.theme.view_col, app.theme.item_selected_field)
                        } else {
                            let blanks: Vec<String> = left_empty.iter().map(|_| String::new()).collect();
                            col_cells(left_cols, &blanks, None, None, None, "\u{00B7}", app.theme.view_col, app.theme.item_selected_field)
                        };
                        spans.push(Span::styled(indent, app.theme.view_item));
                        spans.push(Span::styled(left,   app.theme.view_item));
                        if !hi.is_empty() || row_i == dcl {
                            spans.push(Span::styled(
                                if hi.is_empty() { " ".to_string() } else { hi },
                                app.theme.item_selected_field,
                            ));
                        }
                        spans.push(Span::styled(right, app.theme.view_item));
                        let cursor_extra = if row_i == dcl && dcc >= line.chars().count() { 1 } else { 0 };
                        let used = create_pfx_w + line.chars().count() + cursor_extra;
                        if used < main_col_w { spans.push(Span::raw(" ".repeat(main_col_w - used))); }
                        if !right_cols.is_empty() { spans.push(Span::raw(" ")); }
                        let right_blanks: Vec<String> = right_empty.iter().map(|_| String::new()).collect();
                        spans.extend(col_cells(right_cols, &right_blanks, None, None, None, "\u{00B7}", app.theme.view_col, app.theme.item_selected_field));
                        lines.push(Line::from(spans));
                    }
                }
            }
            if cursor_on_item { cursor_last_line = lines.len() - 1; }
            lmap.push((CursorPos::Item { section: s_idx, item: i_idx }, item_first, lines.len() - 1));
        }

        // Blank line between sections
        if display_pos + 1 < visible_sections.len() {
            lines.push(Line::from(""));
        }
    }

    // Compute scroll offset to keep the cursor row visible.
    let mut off = app.scroll_offset.get();
    if cursor_line_found && body_h > 0 {
        if cursor_first_line < off {
            off = cursor_first_line;
        }
        if cursor_last_line >= off + body_h {
            off = cursor_last_line + 1 - body_h;
            // Don't scroll so far that cursor_first_line goes off the top.
            if cursor_first_line < off { off = cursor_first_line; }
        }
    }
    app.scroll_offset.set(off);
    app.cursor_line.set(cursor_first_line);
    app.body_height.set(body_h);
    *app.line_map.borrow_mut() = lmap;
    let visible: Vec<Line> = lines.into_iter().skip(off).take(body_h).collect();
    frame.render_widget(Paragraph::new(visible).style(app.theme.view_bg), body_inner);

    // ── F-key bar ─────────────────────────────────────────────────────────
    fkeys::render_fkey_bar(frame, chunks[2], app);

    // ── Column form modal ─────────────────────────────────────────────────
    let show_form = matches!(app.col_mode, ColMode::Form { .. } | ColMode::Choices { .. });
    if show_form {
        let (is_add, head_cat_idx, width_buf, _width_cur, position, active_field) = match &app.col_mode {
            ColMode::Form { is_add, head_cat_idx, width_buf, width_cur, position, active_field } |
            ColMode::Choices { is_add, head_cat_idx, width_buf, width_cur, position, active_field, .. } =>
                (*is_add, *head_cat_idx, width_buf.as_str(), *width_cur, *position, *active_field),
            _ => unreachable!(),
        };

        let modal_rect = centered_rect(64, 10, area);
        frame.render_widget(Clear, modal_rect);

        let title = if is_add { " Column Add " } else { " Column Properties " };
        let block = Block::default().borders(Borders::ALL)
            .title(title).style(app.theme.dialog_border);
        frame.render_widget(block.clone(), modal_rect);
        let inner = block.inner(modal_rect);

        let flat = flatten_cats(&app.categories);
        let cat_name = head_cat_idx
            .and_then(|i| flat.get(i))
            .map(|c| c.name.as_str())
            .unwrap_or("");

        let rev        = app.theme.item_selected_field;
        let dlabel     = app.theme.dialog_label;
        let dlabel_sel = app.theme.dialog_label_sel;

        // Head — fully highlighted when active; show at least one space when blank
        let head_line = {
            let head_active = active_field == ColFormField::Head;
            let label = Span::styled(" Column head:  ", if head_active { dlabel_sel } else { dlabel });
            if head_active {
                let display = if cat_name.is_empty() { " " } else { cat_name };
                Line::from(vec![label, Span::styled(display.to_string(), rev)])
            } else {
                Line::from(vec![label, Span::raw(cat_name.to_string())])
            }
        };

        // Width — fully highlighted when active
        let width_line = {
            let width_active = active_field == ColFormField::Width;
            let label = Span::styled(" Width:        ", if width_active { dlabel_sel } else { dlabel });
            if width_active {
                Line::from(vec![label, Span::styled(width_buf.to_string(), rev)])
            } else {
                Line::from(vec![label, Span::raw(width_buf.to_string())])
            }
        };

        // Position — only shown (and focusable) when adding
        let pos_label = match position {
            ColPos::Right => "Right of current column",
            ColPos::Left  => "Left of current column",
        };
        let position_line = if is_add {
            let pos_active = active_field == ColFormField::Position;
            let label = Span::styled(" Position:     ", if pos_active { dlabel_sel } else { dlabel });
            if pos_active {
                Line::from(vec![label, Span::styled(pos_label, rev)])
            } else {
                Line::from(vec![label, Span::raw(pos_label)])
            }
        } else {
            Line::from(vec![Span::styled(" Position:     ", dlabel), Span::raw(pos_label)])
        };

        let form_lines = vec![
            Line::from(""),
            head_line,
            width_line,
            position_line,
            Line::from(vec![Span::styled(" Format:       ", dlabel), Span::raw("Name only")]),
            Line::from(""),
            Line::from(" Category type: Standard    Insert in: All sections"),
            Line::from(""),
            Line::from(" \u{2500}\u{2500}\u{2500} Press ENTER when done, ESC to cancel \u{2500}\u{2500}\u{2500}"),
        ];

        frame.render_widget(Paragraph::new(form_lines).style(app.theme.dialog), inner);
    }

    // ── Choices picker overlay ────────────────────────────────────────────
    if let ColMode::Choices { picker_cursor, kind, .. } = &app.col_mode {
        let rev = app.theme.item_selected_field;

        let lines: Vec<Line> = match kind {
            ChoicesKind::Category => {
                let flat = flatten_cats(&app.categories);
                flat.iter().enumerate().map(|(i, cat)| {
                    let indent = " ".repeat(cat.depth * 2 + 1);
                    let indicator = match cat.kind {
                        CategoryKind::Standard  => " ",
                        CategoryKind::Date      => "*",
                        CategoryKind::Numeric   => "#",
                        CategoryKind::Unindexed => "D",
                    };
                    let text = format!("{}{} {}", indent, indicator, cat.name);
                    if i == *picker_cursor {
                        Line::from(Span::styled(text, rev))
                    } else {
                        Line::from(Span::raw(text))
                    }
                }).collect()
            }
            ChoicesKind::Position => {
                let opts = ["Right of current column", "Left of current column"];
                opts.iter().enumerate().map(|(i, &label)| {
                    let text = format!(" {}", label);
                    if i == *picker_cursor {
                        Line::from(Span::styled(text, rev))
                    } else {
                        Line::from(Span::raw(text))
                    }
                }).collect()
            }
        };

        let picker_h = (lines.len() as u16 + 2).min(area.height.saturating_sub(4)).max(4);
        let picker_rect = centered_rect(40, picker_h, area);
        frame.render_widget(Clear, picker_rect);

        let block = Block::default().borders(Borders::ALL)
            .title(" Choices ").style(app.theme.dialog_border);
        frame.render_widget(block.clone(), picker_rect);
        let inner = block.inner(picker_rect);

        // Scroll so picker_cursor is visible
        let visible = inner.height as usize;
        let offset  = if *picker_cursor >= visible { picker_cursor - visible + 1 } else { 0 };

        let visible_lines: Vec<Line> = lines.into_iter()
            .skip(offset)
            .take(visible)
            .collect();

        frame.render_widget(Paragraph::new(visible_lines).style(app.theme.dialog), inner);
    }

    // ── Quick-add category picker (Alt-R / Alt-L) ────────────────────────────
    if let ColMode::QuickAdd { position, picker_cursor, confirm_delete } = &app.col_mode {
        let rev  = app.theme.item_selected_field;
        let dim  = Style::default().add_modifier(Modifier::DIM);
        let flat = flatten_cats(&app.categories);
        let pc   = *picker_cursor;

        // Header line: search or instruction
        let header_text = if let Some(buf) = &app.cat_search {
            format!(" Search for: {}", buf)
        } else {
            " Select category for column head".to_string()
        };

        // Build scrollable category rows (may include an inline create row)
        let in_create = matches!(app.cat_state.mode, CatMode::Create { .. });
        let mut cat_lines: Vec<Line<'static>> = Vec::new();

        if flat.is_empty() {
            if let CatMode::Create { buffer, cursor: buf_cur, .. } = &app.cat_state.mode {
                let (left, hi, right) = cursor_split(buffer, *buf_cur);
                cat_lines.push(Line::from(vec![
                    Span::raw("   "),
                    Span::raw(left), Span::styled(hi, rev), Span::raw(right),
                ]));
            } else {
                cat_lines.push(Line::from(Span::styled(
                    " (no categories \u{2014} press INS to add)", dim,
                )));
            }
        } else {
            for (i, cat) in flat.iter().enumerate() {
                let indent   = " ".repeat(cat.depth * 2 + 1);
                let note_ind = cat_note_indicator(&app.categories, cat.id);
                let type_ind = match cat.kind {
                    CategoryKind::Standard  => if !note_ind.is_empty() { note_ind } else { " " },
                    CategoryKind::Date      => "*",
                    CategoryKind::Numeric   => "#",
                    CategoryKind::Unindexed => "\u{25A1}",
                };
                let is_cur = i == pc;
                cat_lines.push(Line::from(vec![
                    Span::raw(format!("{}{} ", indent, type_ind)),
                    if is_cur { Span::styled(cat.name.clone(), rev) }
                    else      { Span::raw(cat.name.clone()) },
                ]));

                // Inline create row appears after the cursor row
                if is_cur {
                    if let CatMode::Create { buffer, cursor: buf_cur, as_child, .. } = &app.cat_state.mode {
                        let d = if *as_child { cat.depth + 1 } else { cat.depth };
                        let cr_indent = " ".repeat(d * 2 + 1);
                        let (left, hi, right) = cursor_split(buffer, *buf_cur);
                        cat_lines.push(Line::from(vec![
                            Span::raw(format!("{}  ", cr_indent)),
                            Span::raw(left), Span::styled(hi, rev), Span::raw(right),
                        ]));
                    }
                }
            }
        }

        // Box sizing: +3 = 2 borders + 1 header line
        let n = cat_lines.len();
        let picker_h = (n as u16 + 3).min(area.height.saturating_sub(4)).max(5);
        let picker_rect = centered_rect(44, picker_h, area);
        frame.render_widget(Clear, picker_rect);

        let box_title = match position {
            ColPos::Right => " Add Column Right ",
            ColPos::Left  => " Add Column Left ",
        };
        let block = Block::default().borders(Borders::ALL)
            .title(box_title).style(app.theme.dialog_border);
        frame.render_widget(block.clone(), picker_rect);
        let inner = block.inner(picker_rect);

        // Scrollable list area is inner minus the header row
        let list_h  = inner.height.saturating_sub(1) as usize;
        // If a create row follows cursor, keep it visible too
        let bottom  = if in_create { pc + 1 } else { pc };
        let offset  = if bottom >= list_h { bottom - list_h + 1 } else { 0 };

        let mut all_lines = vec![Line::from(Span::raw(header_text))];
        all_lines.extend(cat_lines.into_iter().skip(offset).take(list_h));
        frame.render_widget(Paragraph::new(all_lines).style(app.theme.dialog), inner);

        // ── Delete confirmation overlay ───────────────────────────────────
        if *confirm_delete {
            let cat_name = flat.get(pc).map(|e| e.name.as_str()).unwrap_or("?");
            let rev = app.theme.item_selected_field;
            // Make the dialog wide enough to show the category name.
            let msg = format!("Discard \"{}\"?", cat_name);
            let dlg_w = (msg.chars().count() + 4).max(30).min(area.width as usize) as u16;
            let dlg_rect = centered_rect(dlg_w, 5, area);
            frame.render_widget(Clear, dlg_rect);
            let dlg_block = Block::default().borders(Borders::ALL)
                .title(" Discard Category? ").style(app.theme.dialog_border);
            frame.render_widget(dlg_block.clone(), dlg_rect);
            let dlg_inner = dlg_block.inner(dlg_rect);
            let iw = dlg_inner.width as usize;
            let mpad = (iw.saturating_sub(msg.chars().count())) / 2;
            let yes_label = " Yes ";
            let no_label  = " No  ";
            let gap  = iw.saturating_sub(yes_label.chars().count() + no_label.chars().count() + 2);
            let lpad = gap / 2;
            frame.render_widget(Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::raw(format!("{}{}", " ".repeat(mpad), msg))),
                Line::from(vec![
                    Span::raw(" ".repeat(lpad)),
                    Span::styled(yes_label, rev),
                    Span::raw("  "),
                    Span::raw(no_label),
                ]),
            ]).style(app.theme.dialog), dlg_inner);
        }
    }

    // ── Column Properties modal ───────────────────────────────────────────────
    if let ColMode::Props { head_buf, head_cur, width_buf, width_cur,
                            format, date_fmt, active_field, is_date } = &app.col_mode {
        let modal_h = if *is_date { 18u16 } else { 10u16 };
        let modal_rect = centered_rect(66, modal_h, area);
        frame.render_widget(Clear, modal_rect);

        let block = Block::default().borders(Borders::ALL)
            .title(" Column Properties ").style(app.theme.dialog_border);
        frame.render_widget(block.clone(), modal_rect);
        let inner = block.inner(modal_rect);

        let rev        = app.theme.item_selected_field;
        let dlabel     = app.theme.dialog_label;
        let dlabel_sel = app.theme.dialog_label_sel;

        // Helper: field value span (REVERSED when active; label styled dlabel/dlabel_sel)
        let field_span = |label: &'static str, val: String, af: PropsField, target: PropsField| -> Vec<Span<'static>> {
            let active = af == target;
            if active {
                vec![Span::styled(label, dlabel_sel), Span::styled(val, rev)]
            } else {
                vec![Span::styled(label, dlabel), Span::raw(val)]
            }
        };

        // Column head — editable text with cursor
        let head_line = {
            let head_active = *active_field == PropsField::Head;
            let label = Span::styled(" Column head:  ", if head_active { dlabel_sel } else { dlabel });
            let cat_type = if *is_date { "Date" } else { "Standard" };
            let type_span = Span::raw(format!("    Category type: {}", cat_type));
            if head_active {
                let (left, hi, right) = cursor_split(head_buf, *head_cur);
                Line::from(vec![
                    label,
                    Span::raw(left),
                    Span::styled(hi, rev),
                    Span::raw(right),
                    type_span,
                ])
            } else {
                Line::from(vec![label, Span::raw(head_buf.clone()), type_span])
            }
        };

        // Width — editable text with cursor
        let width_line = {
            let width_active = *active_field == PropsField::Width;
            let label = Span::styled(" Width:        ", if width_active { dlabel_sel } else { dlabel });
            if width_active {
                let (left, hi, right) = cursor_split(width_buf, *width_cur);
                Line::from(vec![
                    label,
                    Span::raw(left),
                    Span::styled(hi, rev),
                    Span::raw(right),
                ])
            } else {
                Line::from(vec![label, Span::raw(width_buf.clone())])
            }
        };

        // Format field (Standard columns only)
        let format_line = if !*is_date {
            let fmt_label = match format {
                ColFormat::NameOnly       => "Name only",
                ColFormat::ParentCategory => "Parent:Category",
                ColFormat::Ancestor       => "Ancestor",
                ColFormat::Star           => "*  (Star)",
                ColFormat::YesNo          => "Yes/No",
                ColFormat::CategoryNote   => "Category note",
            };
            let fmt_spans = field_span(" Format:       ", fmt_label.to_string(),
                                       *active_field, PropsField::Format);
            Some(Line::from(fmt_spans))
        } else {
            None
        };

        let mut form_lines: Vec<Line<'static>> = vec![
            Line::from(""),
            head_line,
            width_line,
        ];
        if let Some(fl) = format_line {
            form_lines.push(fl);
        }

        if *is_date {
            if let Some(fmt) = date_fmt {
                let disp_label = match fmt.display {
                    DateDisplay::Date     => "Date",
                    DateDisplay::Time     => "Time",
                    DateDisplay::DateTime => "Date and time",
                };
                let clock_label = match fmt.clock {
                    Clock::Hr12 => "12 hr",
                    Clock::Hr24 => "24 hr",
                };
                let code_label = match fmt.code {
                    DateFmtCode::MMDDYY   => "MM/DD/YY",
                    DateFmtCode::DDMMYY   => "DD/MM/YY",
                    DateFmtCode::YYYYMMDD => "YYYY/MM/DD",
                };
                let dow_label   = if fmt.show_dow  { "Yes" } else { "No" };
                let ampm_label  = if fmt.show_ampm { "Yes" } else { "No" };
                let dsep_label  = fmt.date_sep.to_string();
                let tsep_label  = fmt.time_sep.to_string();

                let sample = format_date_value("2026-03-04 15:16:00", fmt);

                form_lines.push(Line::from(""));
                form_lines.push(Line::from("          Date Column Properties"));
                form_lines.push(Line::from("           (Global defaults)"));
                form_lines.push(Line::from(""));

                let disp_line = field_span(" Display date and/or time:  ",
                    disp_label.to_string(), *active_field, PropsField::DateDisplay);
                form_lines.push(Line::from(disp_line));

                {
                    let mut spans = field_span(" Show day of week:   ",
                        dow_label.to_string(), *active_field, PropsField::ShowDow);
                    spans.push(Span::raw("          "));
                    let clock_spans = field_span("Clock:       ",
                        clock_label.to_string(), *active_field, PropsField::Clock);
                    spans.extend(clock_spans);
                    form_lines.push(Line::from(spans));
                }

                {
                    let mut spans = field_span(" Date format:        ",
                        code_label.to_string(), *active_field, PropsField::DateFmtCode);
                    spans.push(Span::raw("      "));
                    let ampm_spans = field_span("Show am/pm:  ",
                        ampm_label.to_string(), *active_field, PropsField::ShowAmPm);
                    spans.extend(ampm_spans);
                    form_lines.push(Line::from(spans));
                }

                {
                    let mut spans = field_span(" Date separator:     ",
                        dsep_label, *active_field, PropsField::DateSep);
                    spans.push(Span::raw("           "));
                    let tsep_spans = field_span("Time separator: ",
                        tsep_label, *active_field, PropsField::TimeSep);
                    spans.extend(tsep_spans);
                    form_lines.push(Line::from(spans));
                }

                form_lines.push(Line::from(""));
                form_lines.push(Line::from("               Formatted Sample"));
                form_lines.push(Line::from(format!("               {}", sample)));
                form_lines.push(Line::from(""));
            }
        }

        form_lines.push(Line::from(
            " \u{2500}\u{2500}\u{2500} Press ENTER when done, ESC to cancel \u{2500}\u{2500}\u{2500}"
        ));

        frame.render_widget(Paragraph::new(form_lines).style(app.theme.dialog), inner);
    }

    // ── Calendar modal ────────────────────────────────────────────────────────
    if let ColMode::Calendar { year, month, day, hour, min, sec } = &app.col_mode {
        let (cal_year, cal_month, cal_day) = (*year, *month, *day);
        let (cal_hour, cal_min, cal_sec)   = (*hour, *min, *sec);
        let (today_y, today_m, today_d)    = cal_today();
        let dim   = cal_days_in_month(cal_year, cal_month);
        let start = cal_first_dow(cal_year, cal_month) as usize;
        let mname = CAL_MONTH_NAMES[(cal_month as usize).saturating_sub(1)];

        // Box: 24 wide (22 inner), 12 tall (10 inner)
        let cal_rect = centered_rect(24, 12, area);
        frame.render_widget(Clear, cal_rect);
        let block = Block::default().borders(Borders::ALL)
            .title(" Calendar ").style(app.theme.dialog_border);
        frame.render_widget(block.clone(), cal_rect);
        let inner = block.inner(cal_rect);
        let iw = inner.width as usize;

        let rev  = app.theme.item_selected_field;
        let bold = Style::default().add_modifier(Modifier::BOLD);

        // Title: centre "Month YYYY" in iw chars
        let title_str = format!("{} {}", mname, cal_year);
        let tpad = (iw.saturating_sub(title_str.chars().count())) / 2;
        let title_line = Line::from(Span::raw(format!("{}{}", " ".repeat(tpad), title_str)));

        // Day-of-week header
        let header_line = Line::from(Span::raw(" Su Mo Tu We Th Fr Sa"));

        let mut cal_lines: Vec<Line<'static>> = vec![title_line, header_line];

        // 6 week rows: each cell is 2 chars, separated by 1 space, with 1 leading space
        for row in 0..6usize {
            let mut spans: Vec<Span<'static>> = vec![Span::raw(" ")];
            for col in 0..7usize {
                if col > 0 { spans.push(Span::raw(" ")); }
                let cell = row * 7 + col;
                if cell < start || cell >= start + dim as usize {
                    spans.push(Span::raw("  "));
                } else {
                    let d = (cell - start + 1) as u32;
                    let s = format!("{:2}", d);
                    let style = if d == cal_day {
                        rev
                    } else if cal_year == today_y && cal_month == today_m && d == today_d {
                        bold
                    } else {
                        Style::default()
                    };
                    spans.push(Span::styled(s, style));
                }
            }
            cal_lines.push(Line::from(spans));
        }

        // Time display
        cal_lines.push(Line::from(Span::raw(
            format!(" Time: {:02}:{:02}:{:02}", cal_hour, cal_min, cal_sec)
        )));

        // Year hint (< / > keys; Ctrl+PgUp/Dn may be intercepted by terminal)
        let left_hint  = "< Prev Yr";
        let right_hint = "Next Yr >";
        let gap = iw.saturating_sub(left_hint.chars().count() + right_hint.chars().count());
        cal_lines.push(Line::from(Span::raw(format!(
            "{}{}{}", left_hint, " ".repeat(gap), right_hint
        ))));

        frame.render_widget(Paragraph::new(cal_lines).style(app.theme.dialog), inner);
    }

    // ── SetTime modal ─────────────────────────────────────────────────────────
    if let ColMode::SetTime { year, month, day, hour_buf, min_buf, sec_buf, active, .. } = &app.col_mode {
        let rev = app.theme.item_selected_field;

        let st_rect = centered_rect(28, 7, area);
        frame.render_widget(Clear, st_rect);
        let block = Block::default().borders(Borders::ALL)
            .title(" Set Time ").style(app.theme.dialog_border);
        frame.render_widget(block.clone(), st_rect);
        let inner = block.inner(st_rect);

        let date_line = Line::from(Span::raw(
            format!(" Date: {:04}-{:02}-{:02}", year, month, day)
        ));

        let make_field = |buf: &str, field: TimeField| -> Span<'static> {
            let s = format!("{:>2}", buf);
            if *active == field { Span::styled(s, rev) } else { Span::raw(s) }
        };

        let time_line = Line::from(vec![
            Span::raw(" Time: "),
            make_field(hour_buf, TimeField::Hour),
            Span::raw(":"),
            make_field(min_buf, TimeField::Min),
            Span::raw(":"),
            make_field(sec_buf, TimeField::Sec),
        ]);

        let help_line = Line::from(Span::raw(
            " \u{2190}\u{2192} field   ENTER saves   ESC cancels"
        ));

        frame.render_widget(Paragraph::new(vec![
            Line::from(""),
            date_line,
            Line::from(""),
            time_line,
            Line::from(""),
            help_line,
        ]).style(app.theme.dialog), inner);
    }

    // ── Sub-category picker (F3 on standard column) ───────────────────────────
    if let ColMode::SubPick { col_idx, picker_cursor } = &app.col_mode {
        let col_idx       = *col_idx;
        let picker_cursor = *picker_cursor;

        // Build the list: column head first, then all descendants.
        let all_cats  = flatten_cats(&app.categories);
        let head_id   = app.view.columns.get(col_idx).map(|c| c.cat_id).unwrap_or(0);
        let head_entry = all_cats.iter().find(|e| e.id == head_id);
        let (head_path, head_depth) = match head_entry {
            Some(h) => (h.path.clone(), h.depth),
            None    => return,
        };
        let flat_subs: Vec<&crate::app::FlatCat> = all_cats.iter()
            .filter(|e| e.id == head_id || e.path.starts_with(&head_path))
            .collect();

        // Current item: global index and its assigned values.
        let (item_text, item_vals, cond_cats) = match &app.cursor {
            CursorPos::Item { section, item } => {
                let gi = visible_item_indices(&app.items, &app.view, *section, &app.categories)
                    .get(*item).copied();
                let it = gi.and_then(|gi| app.items.get(gi));
                (
                    it.map(|i| i.text.as_str()).unwrap_or(""),
                    it.map(|i| &i.values),
                    it.map(|i| &i.cond_cats),
                )
            }
            _ => ("", None, None),
        };
        let empty_vals  = std::collections::HashMap::new();
        let empty_conds = std::collections::HashSet::new();
        let item_vals   = item_vals.unwrap_or(&empty_vals);
        let cond_cats   = cond_cats.unwrap_or(&empty_conds);

        let rev  = app.theme.item_selected_field;
        let bold = Style::default().add_modifier(Modifier::BOLD);

        // Scroll window: same 16-row limit as Assignment Profile.
        let visible = flat_subs.len().min(16);
        let start   = if picker_cursor >= visible { picker_cursor - visible + 1 } else { 0 };

        let in_create = matches!(app.cat_state.mode, CatMode::Create { .. });
        let box_h    = (visible + 3 + if in_create { 1 } else { 0 }) as u16;
        let box_w    = 50u16;
        let dlg_rect = centered_rect(box_w, box_h, area);
        frame.render_widget(Clear, dlg_rect);
        let block = Block::default()
            .borders(Borders::ALL)
            .title_top(Line::from(" Select Category ").alignment(Alignment::Center))
            .title_bottom(Line::from(" Press ENTER to accept ").alignment(Alignment::Center))
            .style(app.theme.dialog_border);
        frame.render_widget(block.clone(), dlg_rect);
        let inner = block.inner(dlg_rect);

        // Header line: bold description.
        let header = format!(" Select categories for \"{}\"", item_text);
        let mut cat_lines: Vec<Line<'static>> = vec![
            Line::from(Span::styled(header, bold)),
        ];

        for (i, e) in flat_subs.iter().enumerate().skip(start).take(visible) {
            let assigned = item_vals.contains_key(&e.id);
            let is_cond  = cond_cats.contains(&e.id);
            let marker   = match (assigned, is_cond) {
                (true,  true)  => "*c",
                (true,  false) => "* ",
                (false, _)     => "  ",
            };
            let note_ind = cat_note_indicator(&app.categories, e.id);
            let type_ind = match e.kind {
                CategoryKind::Standard  => if !note_ind.is_empty() { note_ind } else { " " },
                CategoryKind::Date      => "*",
                CategoryKind::Numeric   => "#",
                CategoryKind::Unindexed => "\u{25A1}",
            };
            // Indent relative to the column head (head = 0, children = 1, etc.).
            let rel_depth = e.depth.saturating_sub(head_depth);
            let indent    = "  ".repeat(rel_depth);
            let highlighted = i == picker_cursor;
            let pfx = Span::raw(format!(" {}\u{2502}{} {}", marker, type_ind, indent));
            if highlighted {
                match &app.cat_state.mode {
                    CatMode::Edit { buffer, cursor: buf_cur } => {
                        let (left, hi, right) = cursor_split(buffer, *buf_cur);
                        cat_lines.push(Line::from(vec![
                            pfx,
                            Span::raw(left), Span::styled(hi, rev), Span::raw(right),
                        ]));
                    }
                    CatMode::Create { buffer, cursor: buf_cur, .. } => {
                        // Cursor row shown plain; new input row injected after it.
                        cat_lines.push(Line::from(vec![pfx, Span::raw(e.name.clone())]));
                        let (left, hi, right) = cursor_split(buffer, *buf_cur);
                        cat_lines.push(Line::from(vec![
                            Span::raw(format!("   \u{2502}  {}", indent)),
                            Span::raw(left), Span::styled(hi, rev), Span::raw(right),
                        ]));
                    }
                    _ => {
                        cat_lines.push(Line::from(vec![pfx, Span::styled(e.name.clone(), rev)]));
                    }
                }
            } else {
                cat_lines.push(Line::from(vec![pfx, Span::raw(e.name.clone())]));
            }
        }

        frame.render_widget(Paragraph::new(cat_lines).style(app.theme.dialog), inner);
    }

    // ── Item Properties modal ─────────────────────────────────────────────────
    if let Mode::ItemProps { gi, cursor, edit_buf } = &app.mode {
        let gi       = *gi;
        let cursor   = *cursor;
        let edit_buf = edit_buf.clone();
        let rev = app.theme.item_selected_field;
        let dim = app.theme.dim;

        let item = match app.items.get(gi) { Some(it) => it, None => return };

        // Build sorted assigned list.
        let assigned = app.item_props_assigned(gi);

        // Note field value: first line of note text (or placeholder if empty).
        let note_text = item.note.clone();
        let note_display = {
            let first_line = note_text.lines().next().unwrap_or("");
            if first_line.is_empty() { "...".to_string() } else { first_line.to_string() }
        };

        // Modal size: fixed rows + assigned list rows (min 1).
        let n = assigned.len();
        let max_inner = area.height.saturating_sub(4) as usize;
        let list_h    = n.max(1).min(max_inner.saturating_sub(9));
        let modal_h   = (2 + 8 + list_h) as u16;
        let modal_w   = 58u16;
        let modal_rect = centered_rect(modal_w, modal_h, area);
        frame.render_widget(Clear, modal_rect);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .title(" Item Properties ")
            .style(app.theme.dialog_border);
        frame.render_widget(block.clone(), modal_rect);
        let inner = block.inner(modal_rect);
        let iw = inner.width as usize;

        // Field value width: iw minus label width (longest label = "  Item statistics:   " = 21).
        // We right-pad each value to fill the remaining width when active.
        let fval = |label: &str, val: &str, active: bool| -> Line<'static> {
            let label_len = label.chars().count();
            let val_w = iw.saturating_sub(label_len);
            let displayed: String = val.chars().take(val_w).collect();
            let padded = format!("{:<width$}", displayed, width = val_w);
            Line::from(vec![
                Span::raw(label.to_string()),
                if active { Span::styled(padded, rev) } else { Span::raw(displayed) },
            ])
        };

        // Scroll offset for the assigned list (only when cursor >= 4).
        let list_cursor = cursor.saturating_sub(4);
        let list_offset = if cursor >= 4 && list_cursor >= list_h {
            list_cursor - list_h + 1
        } else { 0 };

        // Helper: compact date format MM/DD/YY H:MMam/pm from stored YYYY-MM-DD HH:MM:SS.
        let fmt_date = |stored: &str| -> String {
            if let Some((y, mo, d, h, mi, _)) = crate::app::parse_datetime(stored) {
                let yy  = y % 100;
                let (h12, ampm) = if h == 0 { (12u32, "am") }
                                  else if h < 12 { (h, "am") }
                                  else if h == 12 { (12u32, "pm") }
                                  else { (h - 12, "pm") };
                format!("{:02}/{:02}/{:02} {}:{:02}{}", mo, d, yy, h12, mi, ampm)
            } else {
                stored.to_string()
            }
        };

        // Build Item text row — shows editing cursor when edit_buf is active.
        let item_text_line = if let Some((buf, cur)) = &edit_buf {
            let label = "  Item text:   ";
            let label_len = label.chars().count();
            let val_w = iw.saturating_sub(label_len);
            let (left, hi, right) = crate::ui::cursor_split(buf, *cur);
            let left: String  = left.chars().take(val_w).collect();
            let hi: String    = hi.chars().next().map(|c| c.to_string()).unwrap_or_else(|| " ".to_string());
            let take_right = val_w.saturating_sub(left.chars().count() + 1);
            let right: String = right.chars().take(take_right).collect();
            Line::from(vec![
                Span::raw(label.to_string()),
                Span::raw(left),
                Span::styled(hi, rev),
                Span::raw(right),
            ])
        } else {
            fval("  Item text:   ", &item.text, cursor == 0)
        };

        let mut lines: Vec<Line> = vec![
            Line::from(""),
            item_text_line,
            fval("  Note:        ", &note_display,     cursor == 1),
            fval("  Note file:   ", &item.note_file,  cursor == 2),
            fval("  Item statistics:   ", "...",      cursor == 3),
            Line::from(Span::raw("  Assigned to:")),
        ];

        if n == 0 {
            lines.push(Line::from(Span::styled("    (none)", dim)));
        } else {
            for (i, (_, name, kind, val)) in assigned.iter().enumerate().skip(list_offset).take(list_h) {
                let entry = if *kind == CategoryKind::Date && !val.is_empty() {
                    format!("  {}({})", name, fmt_date(val))
                } else {
                    format!("  {}", name)
                };
                let highlighted = cursor >= 4 && i == list_cursor;
                let entry: String = entry.chars().take(iw.saturating_sub(2)).collect();
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    if highlighted { Span::styled(entry, rev) } else { Span::raw(entry) },
                ]));
            }
        }

        lines.push(Line::from(""));
        let footer = "\u{2550}\u{2550}\u{2550} Press ENTER when done, ESC to cancel \u{2550}\u{2550}\u{2550}";
        let fpad = iw.saturating_sub(footer.chars().count()) / 2;
        lines.push(Line::from(Span::raw(format!("{}{}", " ".repeat(fpad), footer))));

        frame.render_widget(Paragraph::new(lines).style(app.theme.dialog), inner);
    }

    // ── Remove-item confirmation modal ────────────────────────────────────────
    if let Mode::ConfirmDeleteItem { yes } = &app.mode {
        let rev   = app.theme.item_selected_field;
        let unsel = app.theme.dialog;
        let dim   = app.theme.dim;
        let dlg_rect = centered_rect(46, 9, area);
        frame.render_widget(Clear, dlg_rect);
        let block = Block::default().borders(Borders::ALL)
            .title(" Remove Item ").style(app.theme.dialog_border);
        frame.render_widget(block.clone(), dlg_rect);
        let inner = block.inner(dlg_rect);
        let iw = inner.width as usize;

        let msg = "Remove this item from the section?";
        let mpad = (iw.saturating_sub(msg.chars().count())) / 2;
        let hint = "(Use Alt-F4 to discard the item completely.)";
        let hpad = (iw.saturating_sub(hint.chars().count())) / 2;

        let yes_label = " Yes ";
        let no_label  = " No  ";
        let yes_style = if *yes { rev } else { unsel };
        let no_style  = if !yes { rev } else { unsel };
        let gap  = iw.saturating_sub(yes_label.chars().count() + no_label.chars().count() + 2);
        let lpad = gap / 2;

        frame.render_widget(Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::raw(format!("{}{}", " ".repeat(mpad), msg))),
            Line::from(""),
            Line::from(vec![
                Span::raw(" ".repeat(lpad)),
                Span::styled(yes_label, yes_style),
                Span::raw("  "),
                Span::styled(no_label, no_style),
            ]),
            Line::from(""),
            Line::from(Span::styled(format!("{}{}", " ".repeat(hpad), hint), dim)),
        ]).style(app.theme.dialog), inner);
    }

    // ── Discard-item confirmation modal ───────────────────────────────────────
    if let Mode::ConfirmDiscardItem { yes } = &app.mode {
        let rev   = app.theme.item_selected_field;
        let unsel = app.theme.dialog;
        let dlg_rect = centered_rect(46, 9, area);
        frame.render_widget(Clear, dlg_rect);
        let block = Block::default().borders(Borders::ALL)
            .title(" Discard Item ").style(app.theme.dialog_border);
        frame.render_widget(block.clone(), dlg_rect);
        let inner = block.inner(dlg_rect);
        let iw = inner.width as usize;

        let msg = "Discard this item?";
        let mpad = (iw.saturating_sub(msg.chars().count())) / 2;
        let hint = "Press ENTER to accept, ESC to cancel";
        let hpad = (iw.saturating_sub(hint.chars().count())) / 2;

        let yes_label = " Yes ";
        let no_label  = " No  ";
        let yes_style = if *yes { rev } else { unsel };
        let no_style  = if !yes { rev } else { unsel };
        let gap  = iw.saturating_sub(yes_label.chars().count() + no_label.chars().count() + 2);
        let lpad = gap / 2;

        frame.render_widget(Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::raw(format!("{}{}", " ".repeat(mpad), msg))),
            Line::from(""),
            Line::from(vec![
                Span::raw(" ".repeat(lpad)),
                Span::styled(yes_label, yes_style),
                Span::raw("  "),
                Span::styled(no_label, no_style),
            ]),
            Line::from(""),
            Line::from(Span::raw(format!("{}{}", " ".repeat(hpad), hint))),
        ]).style(app.theme.dialog), inner);
    }

    // ── Remove-column confirmation modal ──────────────────────────────────────
    if let ColMode::ConfirmRemove { yes } = &app.col_mode {
        let rev = app.theme.item_selected_field;
        let dlg_rect = centered_rect(38, 7, area);
        frame.render_widget(Clear, dlg_rect);
        let block = Block::default().borders(Borders::ALL)
            .title(" Remove Column ").style(app.theme.dialog_border);
        frame.render_widget(block.clone(), dlg_rect);
        let inner = block.inner(dlg_rect);
        let iw = inner.width as usize;

        let msg = "Remove this column from the view?";
        let mpad = (iw.saturating_sub(msg.chars().count())) / 2;
        let msg_line = Line::from(Span::raw(format!("{}{}", " ".repeat(mpad), msg)));

        let yes_label = " Yes ";
        let no_label  = " No  ";
        let yes_style = if *yes { rev } else { app.theme.dialog };
        let no_style  = if !yes { rev } else { app.theme.dialog };
        let gap = iw.saturating_sub(yes_label.chars().count() + no_label.chars().count() + 2);
        let lpad = gap / 2;
        let btn_line = Line::from(vec![
            Span::raw(" ".repeat(lpad)),
            Span::styled(yes_label, yes_style),
            Span::raw("  "),
            Span::styled(no_label, no_style),
        ]);

        frame.render_widget(Paragraph::new(vec![
            Line::from(""),
            msg_line,
            Line::from(""),
            btn_line,
            Line::from(""),
        ]).style(app.theme.dialog), inner);
    }

    // ── Remove-section confirmation modal ─────────────────────────────────────
    if let SectionMode::ConfirmRemove { yes } = &app.sec_mode {
        let rev = app.theme.item_selected_field;
        let dlg_rect = centered_rect(44, 7, area);
        frame.render_widget(Clear, dlg_rect);
        let block = Block::default().borders(Borders::ALL)
            .title(" Remove Section ").style(app.theme.dialog_border);
        frame.render_widget(block.clone(), dlg_rect);
        let inner = block.inner(dlg_rect);
        let iw = inner.width as usize;

        let msg = "Remove this section from the view?";
        let mpad = (iw.saturating_sub(msg.chars().count())) / 2;
        let msg_line = Line::from(Span::raw(format!("{}{}", " ".repeat(mpad), msg)));

        let yes_label = " Yes ";
        let no_label  = " No  ";
        let yes_style = if *yes { rev } else { app.theme.dialog };
        let no_style  = if !yes { rev } else { app.theme.dialog };
        let gap  = iw.saturating_sub(yes_label.chars().count() + no_label.chars().count() + 2);
        let lpad = gap / 2;
        let btn_line = Line::from(vec![
            Span::raw(" ".repeat(lpad)),
            Span::styled(yes_label, yes_style),
            Span::raw("  "),
            Span::styled(no_label, no_style),
        ]);

        frame.render_widget(Paragraph::new(vec![
            Line::from(""),
            msg_line,
            Line::from(""),
            btn_line,
            Line::from(""),
        ]).style(app.theme.dialog), inner);
    }

    // ── Assignment Profile modal ──────────────────────────────────────────────
    if let AssignMode::Profile { gi, cursor: prof_cursor, on_sub } = &app.assign_mode {
        let (gi, prof_cur, prof_on_sub) = (*gi, *prof_cursor, *on_sub);
        let cats    = flatten_cats(&app.categories);
        let rev     = app.theme.item_selected_field;
        let bold    = Style::default().add_modifier(Modifier::BOLD);
        let dim     = Style::default().add_modifier(Modifier::DIM);

        let empty_vals  = std::collections::HashMap::new();
        let empty_conds = std::collections::HashSet::new();
        let item        = app.items.get(gi);
        let item_vals   = item.map(|it| &it.values).unwrap_or(&empty_vals);
        let cond_cats   = item.map(|it| &it.cond_cats).unwrap_or(&empty_conds);

        // Scroll window: show up to 16 categories at a time.
        let visible = cats.len().min(16);
        let start   = if prof_cur >= visible { prof_cur - visible + 1 } else { 0 };

        // Count Date sub-rows in the visible window to size the box correctly.
        let date_sub_count = cats.iter().skip(start).take(visible)
            .filter(|e| e.kind == CategoryKind::Date
                && item_vals.get(&e.id).map_or(false, |v| !v.is_empty()))
            .count();

        let box_h  = (visible + date_sub_count + 4) as u16;  // 2 border + 1 header + 1 help
        let box_w  = 50u16;
        let dlg_rect = centered_rect(box_w, box_h, area);
        frame.render_widget(Clear, dlg_rect);
        let block = Block::default().borders(Borders::ALL)
            .title(" Assignment Profile ").style(app.theme.dialog_border);
        frame.render_widget(block.clone(), dlg_rect);
        let inner = block.inner(dlg_rect);

        // Title line: "Search for: ..." during search, else "Profile for ..."
        let item_text = item.map(|it| it.text.as_str()).unwrap_or("");
        let title_text = if let Some(buf) = &app.cat_search {
            format!(" Search for: {}", buf)
        } else {
            format!(" Profile for \"{}\"", item_text)
        };
        let title_line = Line::from(Span::styled(title_text, bold));

        let mut cat_lines: Vec<Line<'static>> = vec![title_line];
        for (i, e) in cats.iter().enumerate().skip(start).take(visible) {
            let assigned = item_vals.contains_key(&e.id);
            let is_cond  = cond_cats.contains(&e.id);
            let marker   = match (assigned, is_cond) {
                (true,  true)  => "*c",
                (true,  false) => "* ",
                (false, _)     => "  ",
            };
            let note_ind  = cat_note_indicator(&app.categories, e.id);
            let type_ind  = match e.kind {
                CategoryKind::Standard  => if !note_ind.is_empty() { note_ind } else { " " },
                CategoryKind::Date      => "*",
                CategoryKind::Numeric   => "#",
                CategoryKind::Unindexed => "\u{25A1}",  // □
            };
            let indent = "  ".repeat(e.depth);
            let highlighted = i == prof_cur && !prof_on_sub;
            let cat_line = Line::from(vec![
                Span::raw(format!(" {}\u{2502} {}{} ", marker, indent, type_ind)),
                if highlighted {
                    Span::styled(e.name.clone(), rev)
                } else {
                    Span::raw(e.name.clone())
                },
            ]);
            cat_lines.push(cat_line);

            // For Date categories with a value, show the datetime as a sub-row.
            if e.kind == CategoryKind::Date {
                if let Some(val) = item_vals.get(&e.id) {
                    if !val.is_empty() {
                        let sub_hi = i == prof_cur && prof_on_sub;
                        let val_span = if sub_hi {
                            Span::styled(val.clone(), rev)
                        } else {
                            Span::styled(val.clone(), dim)
                        };
                        cat_lines.push(Line::from(vec![
                            Span::raw(format!("   \u{2502}    {}", indent)),
                            val_span,
                        ]));
                    }
                }
            }
        }

        // Help line at bottom
        let help = Line::from(Span::raw(
            " Space=assign/unassign  Enter/Esc=close"
        ));

        // Pad with empty lines to fill box before help
        while cat_lines.len() < inner.height.saturating_sub(1) as usize {
            cat_lines.push(Line::from(""));
        }
        cat_lines.push(help);

        frame.render_widget(Paragraph::new(cat_lines).style(app.theme.dialog), inner);
    }

    // ── Section Add modal ─────────────────────────────────────────────────────
    let sec_add_state = match &app.sec_mode {
        SectionMode::Add { cat_idx, insert, active_field } =>
            Some((*cat_idx, *insert, *active_field, None::<usize>)),
        SectionMode::Choices { cat_idx, insert, active_field, picker_cursor } =>
            Some((*cat_idx, *insert, *active_field, Some(*picker_cursor))),
        SectionMode::Normal | SectionMode::ConfirmRemove { .. } | SectionMode::Props { .. } => None,
    };
    if let Some((cat_idx, insert, active_field, picker_cursor)) = sec_add_state {
        let cats       = flatten_cats(&app.categories);
        let rev        = app.theme.item_selected_field;
        let dlabel     = app.theme.dialog_label;
        let dlabel_sel = app.theme.dialog_label_sel;
        let dlg_rect   = centered_rect(52, 11, area);
        frame.render_widget(Clear, dlg_rect);
        let block = Block::default().borders(Borders::ALL)
            .title(" Section Add ").style(app.theme.dialog_border);
        frame.render_widget(block.clone(), dlg_rect);
        let inner = block.inner(dlg_rect);
        let iw = inner.width as usize;

        // Category field
        let cat_name: String = cat_idx
            .and_then(|i| cats.get(i))
            .map(|e| e.name.clone())
            .unwrap_or_default();
        let label_w = "  Category:  ".chars().count();
        let field_w = iw.saturating_sub(label_w + 2);
        let cat_disp: String = cat_name.chars().take(field_w).collect();
        let cat_padded = format!("{:<width$}", cat_disp, width = field_w);
        let cat_active = active_field == SectionFormField::Category;
        let cat_style = if cat_active { rev } else { app.theme.dialog };
        let cat_line = Line::from(vec![
            Span::styled("  Category:  ", if cat_active { dlabel_sel } else { dlabel }),
            Span::styled(cat_padded, cat_style),
        ]);

        // Insert field
        let ins_str = match insert {
            SectionInsert::Below => "Below",
            SectionInsert::Above => "Above",
        };
        let ins_active = active_field == SectionFormField::Insert;
        let ins_style = if ins_active { rev } else { app.theme.dialog };
        let ins_line = Line::from(vec![
            Span::styled("  Insert:    ", if ins_active { dlabel_sel } else { dlabel }),
            Span::styled(format!("{:<8}", ins_str), ins_style),
            Span::raw("  (Left/Right to toggle)"),
        ]);

        // Columns: informational
        let col_names: Vec<&str> = app.view.columns.iter().map(|c| c.name.as_str()).collect();
        let cols_str = if col_names.is_empty() {
            "(none)".to_string()
        } else {
            col_names.join("  ")
        };
        let cols_line = Line::from(Span::raw(format!("  Columns:   {}", cols_str)));

        // Help line
        let help = "  F3 Choose category   Enter confirm   Esc cancel  ";
        let help_line = Line::from(Span::raw(help));

        let lines = vec![
            Line::from(""),
            cat_line,
            Line::from(""),
            ins_line,
            Line::from(""),
            cols_line,
            Line::from(""),
            help_line,
            Line::from(""),
        ];

        frame.render_widget(Paragraph::new(lines.clone()).style(app.theme.dialog), inner);

        // Choices picker overlay
        if let Some(picker_cur) = picker_cursor {
            let picker_h = (cats.len().min(10) + 2) as u16;
            let picker_rect = centered_rect(40, picker_h, area);
            frame.render_widget(Clear, picker_rect);
            let pb = Block::default().borders(Borders::ALL)
                .title(" Choose Category ").style(app.theme.dialog_border);
            frame.render_widget(pb.clone(), picker_rect);
            let pi = pb.inner(picker_rect);
            let visible = pi.height as usize;
            let start = if picker_cur >= visible { picker_cur - visible + 1 } else { 0 };
            let pick_lines: Vec<Line<'static>> = cats.iter().enumerate()
                .skip(start).take(visible)
                .map(|(i, e)| {
                    let indent = "  ".repeat(e.depth);
                    let label  = format!("{}{}", indent, e.name);
                    let style  = if i == picker_cur { rev } else { app.theme.dialog };
                    Line::from(Span::styled(label, style))
                })
                .collect();
            frame.render_widget(Paragraph::new(pick_lines).style(app.theme.dialog), pi);
        }

    }

}

/// Compute a centred Rect of `width` × `height` inside `area`.
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect { x, y, width: w, height: h }
}

/// Word-wrap `text` to lines of at most `width` chars, tracking char offsets.
/// Breaks at the last space within each line; hard-breaks words longer than `width`.
/// Returns `(lines, starts)` where `starts[i]` is the char index in `text` where line i begins.
/// Always returns at least one element.
fn word_wrap_lines(text: &str, width: usize) -> (Vec<String>, Vec<usize>) {
    if width == 0 { return (vec![String::new()], vec![0]); }
    let chars: Vec<char> = text.chars().collect();
    let total = chars.len();
    let mut lines:  Vec<String> = Vec::new();
    let mut starts: Vec<usize>  = Vec::new();
    let mut pos = 0usize; // char index into chars
    while pos < total {
        // Skip leading spaces (only on lines after the first)
        if !lines.is_empty() {
            while pos < total && chars[pos] == ' ' { pos += 1; }
            if pos >= total { break; }
        }
        let line_start = pos;
        // Find the furthest break point within [pos, pos+width)
        let end = (pos + width).min(total);
        // Look for last space in [pos, end)
        let last_space = chars[pos..end].iter().rposition(|&c| c == ' ');
        let break_at = if end == total {
            // Reached end of text — take everything
            end
        } else if let Some(sp) = last_space {
            pos + sp
        } else {
            // No space found → hard break
            end
        };
        let line: String = chars[pos..break_at].iter().collect();
        // Trim trailing spaces except on the last segment, where trailing spaces
        // are intentionally typed by the user and must be preserved for cursor rendering.
        let is_last = break_at == total;
        let line = if is_last { line } else { line.trim_end_matches(' ').to_string() };
        lines.push(line);
        starts.push(line_start);
        pos = break_at;
    }
    if lines.is_empty() {
        lines.push(String::new());
        starts.push(0);
    }
    (lines, starts)
}

/// Given wrap starts and cursor char position, return `(line_idx, col_within_line)`.
fn find_cursor_in_wrapped(starts: &[usize], lines: &[String], cursor: usize) -> (usize, usize) {
    let line_idx = starts.partition_point(|&s| s <= cursor).saturating_sub(1).min(lines.len().saturating_sub(1));
    let col = cursor.saturating_sub(starts[line_idx]).min(lines[line_idx].chars().count());
    (line_idx, col)
}

fn pad_or_trunc(s: &str, w: usize) -> String {
    let len = s.chars().count();
    if len >= w {
        s.chars().take(w).collect()
    } else {
        format!("{}{}", s, " ".repeat(w - len))
    }
}

/// Returns the cell edit state (buffer, cursor) if the active column header is being edited.
/// `active_col` is 0-based into view.columns.
fn head_cell_edit<'a>(active_col: Option<usize>, mode: &'a Mode) -> Option<(&'a str, usize)> {
    match mode {
        Mode::Edit { col, buffer, cursor, .. } if *col > 0 => {
            if active_col == Some(col - 1) {
                Some((buffer.as_str(), *cursor))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Append `· cell` for each column in `columns`.
/// `values` must have the same length as `columns`.
/// `active_col` (0-indexed within `columns`) highlights that cell.
/// `cell_edit` supplies (buffer, cursor) to show an edit cursor in the active cell.
/// `autocomplete_hint` is a dim suffix shown after the cursor in the active edit cell.
fn col_cells(
    columns: &[Column],
    values: &[String],
    active_col: Option<usize>,
    cell_edit: Option<(&str, usize)>,
    autocomplete_hint: Option<&str>,
    prefix: &'static str,
    text_style: Style,
    selected_style: Style,
) -> Vec<Span<'static>> {
    let prefix_w = prefix.chars().count();
    let mut spans = Vec::new();
    for (i, (col, val)) in columns.iter().zip(values.iter()).enumerate() {
        let display_val = if let Some(ref fmt) = col.date_fmt {
            format_date_value(val, fmt)
        } else {
            val.clone()
        };
        let editing = active_col == Some(i) && cell_edit.is_some();
        // Show the prefix dot only when there is actual content or the cell is being edited.
        let show_prefix = !prefix.is_empty() && (!display_val.is_empty() || editing);
        let text_w = if show_prefix {
            col.width.saturating_sub(prefix_w)
        } else {
            col.width
        };
        if show_prefix {
            spans.push(Span::styled(prefix, text_style));
        }
        if active_col == Some(i) {
            if let Some((buf, cur)) = cell_edit {
                let hint = if cell_edit.is_some() { autocomplete_hint } else { None };
                spans.extend(cell_edit_spans(buf, cur, text_w, hint, text_style, selected_style));
            } else {
                let cell = pad_or_trunc(&display_val, text_w);
                spans.push(Span::styled(cell, selected_style));
            }
        } else {
            let cell = pad_or_trunc(&display_val, text_w);
            spans.push(Span::styled(cell, text_style));
        }
        spans.push(Span::styled(" ", text_style));
    }
    spans
}

/// Spans for a column cell in edit mode: scrolling window + cursor-highlighted char.
/// `hint` is an optional dim autocomplete suffix shown after typed text.
fn cell_edit_spans(buffer: &str, cursor: usize, width: usize, hint: Option<&str>, text_style: Style, cursor_style: Style) -> Vec<Span<'static>> {
    let chars: Vec<char> = buffer.chars().collect();
    let cur   = cursor.min(chars.len());
    // Scroll the window left so the cursor is always visible.
    let start = if cur + 1 > width { cur + 1 - width } else { 0 };
    let visible: String = chars[start..].iter().take(width).collect();
    let cur_in_win = cur - start;
    let (left, hi, right) = cursor_split(&visible, cur_in_win);
    let buf_used = left.chars().count() + 1 + right.chars().count();
    let mut spans = vec![
        Span::styled(left,  text_style),
        Span::styled(hi,    cursor_style),
        Span::styled(right, text_style),
    ];
    if let Some(hint_str) = hint.filter(|s| !s.is_empty()) {
        let remaining = width.saturating_sub(buf_used);
        if remaining > 0 {
            let shown: String = hint_str.chars().take(remaining).collect();
            let hint_len = shown.chars().count();
            spans.push(Span::styled(shown, Style::default().add_modifier(Modifier::DIM)));
            let pad = remaining.saturating_sub(hint_len);
            if pad > 0 { spans.push(Span::raw(" ".repeat(pad))); }
        }
    } else {
        let pad = width.saturating_sub(buf_used);
        if pad > 0 { spans.push(Span::raw(" ".repeat(pad))); }
    }
    spans
}

/// Spans for a new-item input row: prefix + buffer text with cursor-highlighted char.
fn input_row_spans(buffer: &str, cursor: usize, text_style: Style, cursor_style: Style) -> Vec<Span<'static>> {
    let (left, hi, right) = cursor_split(buffer, cursor);
    vec![
        Span::styled(ITEM_PREFIX, text_style),
        Span::styled(left,        text_style),
        Span::styled(hi,          cursor_style),
        Span::styled(right,       text_style),
    ]
}

// ── Calendar helpers ──────────────────────────────────────────────────────────

static CAL_MONTH_NAMES: [&str; 12] = [
    "January", "February", "March", "April", "May", "June",
    "July", "August", "September", "October", "November", "December",
];

fn cal_today() -> (i32, u32, u32) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let z   = (secs / 86400) as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y   = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp  = (5 * doy + 2) / 153;
    let d   = doy - (153 * mp + 2) / 5 + 1;
    let m   = if mp < 10 { mp + 3 } else { mp - 9 };
    let y   = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

fn cal_days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11               => 30,
        2 => if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) { 29 } else { 28 },
        _ => 30,
    }
}

/// Day of week for the 1st of `month`/`year`. 0 = Sunday … 6 = Saturday.
fn cal_first_dow(year: i32, month: u32) -> u32 {
    static T: [i64; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if month < 3 { year as i64 - 1 } else { year as i64 };
    ((y + y / 4 - y / 100 + y / 400 + T[month as usize - 1] + 1).rem_euclid(7)) as u32
}

// ── Ask-save dialog ───────────────────────────────────────────────────────────

pub fn render_ask_save_dialog(frame: &mut Frame, app: &App, area: Rect) {
    let SaveState::AskOnQuit { choice } = &app.save_state else { return };

    let dlg = centered_rect(48, 5, area);
    frame.render_widget(Clear, dlg);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Save Changes? ")
        .title_bottom(
            ratatui::text::Line::from(" Press ENTER to accept, ESC to cancel ")
                .alignment(ratatui::layout::Alignment::Center),
        )
        .style(app.theme.dialog_border);
    frame.render_widget(block.clone(), dlg);
    let inner = block.inner(dlg);

    let rev = app.theme.item_selected_field;
    let yes = *choice == AskChoice::Yes;
    let val_span = if yes { Span::styled("Yes", rev) } else { Span::styled("No", rev) };

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  Save changes before quitting?  "),
            val_span,
        ]),
        Line::from(""),
    ];

    frame.render_widget(Paragraph::new(lines).style(app.theme.dialog), inner);
}

// ── Password-entry dialog ─────────────────────────────────────────────────────

pub fn render_password_entry_dialog(frame: &mut Frame, app: &App, area: Rect) {
    let SaveState::PasswordEntry { purpose, buf, confirm_buf, confirm_active, error, .. } = &app.save_state
    else { return };

    let title = match purpose {
        PasswordPurpose::Enable  => " Enable Encryption ",
        PasswordPurpose::Change  => " Change Password ",
        PasswordPurpose::Disable => " Disable Encryption ",
    };

    let need_confirm = *purpose != PasswordPurpose::Disable;
    let dlg_h: u16 = if need_confirm { 9 } else { 7 };

    let dlg = centered_rect(50, dlg_h, area);
    frame.render_widget(Clear, dlg);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .style(app.theme.dialog_border)
        .title(title);
    frame.render_widget(block.clone(), dlg);
    let inner = block.inner(dlg);

    let dlabel     = app.theme.dialog_label;
    let dlabel_sel = app.theme.dialog_label_sel;
    let rev        = app.theme.item_selected_field;

    let fw = inner.width.saturating_sub(14) as usize;  // field width

    let stars: String = "*".repeat(buf.chars().count());
    let pw_field = format!("{:<width$}", stars, width = fw);
    let pw_active = !*confirm_active;
    let pw_line = Line::from(vec![
        Span::styled("  Password:  ", if pw_active { dlabel_sel } else { dlabel }),
        Span::raw("["),
        Span::styled(pw_field, if pw_active { rev } else { Style::default() }),
        Span::raw("]"),
    ]);

    let mut lines = vec![Line::from(""), pw_line];

    if need_confirm {
        let cf_stars: String = "*".repeat(confirm_buf.chars().count());
        let cf_field = format!("{:<width$}", cf_stars, width = fw);
        let cf_active = *confirm_active;
        let cf_line = Line::from(vec![
            Span::styled("  Confirm:   ", if cf_active { dlabel_sel } else { dlabel }),
            Span::raw("["),
            Span::styled(cf_field, if cf_active { rev } else { Style::default() }),
            Span::raw("]"),
        ]);
        lines.push(cf_line);
    }

    lines.push(Line::from(""));

    if let Some(err) = error {
        lines.push(Line::from(Span::styled(
            format!("  {}", err),
            Style::default().add_modifier(Modifier::BOLD),
        )));
    } else {
        lines.push(Line::from(""));
    }

    lines.push(Line::from(""));
    lines.push(Line::from("  \u{2500}\u{2500}\u{2500} ENTER to confirm, ESC to cancel \u{2500}\u{2500}\u{2500}"));

    frame.render_widget(Paragraph::new(lines).style(app.theme.dialog), inner);
}

// ── View Add dialog ───────────────────────────────────────────────────────────

pub fn render_view_add_dialog(frame: &mut Frame, app: &App, area: Rect) {
    let (name_buf, name_cur, sec_buf, sec_cur, active_field, pick_cur) = match &app.view_mode {
        ViewMode::Add { name_buf, name_cursor, sec_buf, sec_cursor, active_field, .. } =>
            (name_buf.as_str(), *name_cursor, sec_buf.as_str(), *sec_cursor,
             *active_field, None::<usize>),
        ViewMode::AddPick { name_buf, name_cursor, sec_buf, sec_cursor, picker_cursor } =>
            (name_buf.as_str(), *name_cursor, sec_buf.as_str(), *sec_cursor,
             ViewAddField::Section, Some(*picker_cursor)),
        _ => return,
    };

    let dlg = centered_rect(62, 19, area);
    frame.render_widget(Clear, dlg);
    let block = Block::default()
        .borders(Borders::ALL)
        .title_top(Line::from(" View Add ").alignment(Alignment::Center))
        .title_bottom(Line::from(" Press ENTER when done, ESC to cancel ").alignment(Alignment::Center))
        .style(app.theme.dialog_border);
    frame.render_widget(block.clone(), dlg);
    let inner = block.inner(dlg);

    let name_label = "  View name: ";
    let sec_label  = "  Sections:  ";
    let field_w    = 22usize;
    let rev        = app.theme.item_selected_field;
    let dlabel     = app.theme.dialog_label;
    let dlabel_sel = app.theme.dialog_label_sel;

    let name_active = active_field == ViewAddField::Name;
    let name_line: Line = if name_active {
        let (left, hi, right) = super::cursor_split(name_buf, name_cur);
        let pad = " ".repeat(field_w.saturating_sub(name_buf.chars().count()));
        Line::from(vec![
            Span::styled(name_label, dlabel_sel),
            Span::raw(left),
            Span::styled(hi, rev),
            Span::raw(right),
            Span::raw(pad),
            Span::raw("  Type:          Standard"),
        ])
    } else {
        Line::from(vec![
            Span::styled(name_label, dlabel),
            Span::raw(format!("{}  Type:          Standard", pad_or_trunc(name_buf, field_w))),
        ])
    };

    let sec_active = active_field == ViewAddField::Section;
    let sec_line: Line = if sec_active {
        let (left, hi, right) = super::cursor_split(sec_buf, sec_cur);
        let pad = " ".repeat(field_w.saturating_sub(sec_buf.chars().count()));
        Line::from(vec![
            Span::styled(sec_label, dlabel_sel),
            Span::raw(left),
            Span::styled(hi, rev),
            Span::raw(right),
            Span::raw(pad),
            Span::raw("  F3 to pick"),
        ])
    } else {
        Line::from(vec![
            Span::styled(sec_label, dlabel),
            Span::raw(format!("{}  F3 to pick", pad_or_trunc(sec_buf, field_w))),
        ])
    };

    let mut lines: Vec<Line> = vec![Line::from(""), name_line, sec_line];
    lines.extend([
        Line::from(vec![Span::styled("  Item sorting:  ", dlabel),    Span::raw("...")]),
        Line::from(vec![Span::styled("  Section sorting:  ", dlabel), Span::raw("None")]),
        Line::from(""),
        Line::from(vec![Span::styled("  Hide empty sections:  ", dlabel),  Span::raw("No")]),
        Line::from(vec![Span::styled("  Hide done items:      ", dlabel),  Span::raw("No")]),
        Line::from(vec![Span::styled("  Hide dependent items: ", dlabel),  Span::raw("No")]),
        Line::from(vec![Span::styled("  Hide inherited items: ", dlabel),  Span::raw("No")]),
        Line::from(vec![Span::styled("  Hide column heads:    ", dlabel),  Span::raw("No")]),
        Line::from(vec![Span::styled("  Section separators:   ", dlabel),  Span::raw("No")]),
        Line::from(vec![Span::styled("  Number items:         ", dlabel),  Span::raw("No          Filter:")]),
        Line::from(""),
        Line::from(vec![Span::styled("  View statistics:  ", dlabel),  Span::raw("...")]),
        Line::from(""),
        Line::from(vec![Span::styled("  View protection:  ", dlabel),  Span::raw("Global (No protection)")]),
    ]);
    frame.render_widget(Paragraph::new(lines).style(app.theme.dialog), inner);

    // Picker overlay
    if let Some(pc) = pick_cur {
        let cats = flatten_cats(&app.categories);
        let picker_h = (cats.len().min(10) + 2) as u16;
        let picker_rect = centered_rect(40, picker_h, area);
        frame.render_widget(Clear, picker_rect);
        let pb = Block::default().borders(Borders::ALL)
            .title(" Choose Category ").style(app.theme.dialog_border);
        frame.render_widget(pb.clone(), picker_rect);
        let pi = pb.inner(picker_rect);
        let visible = pi.height as usize;
        let start = if pc >= visible { pc - visible + 1 } else { 0 };
        let pick_lines: Vec<Line<'static>> = cats.iter().enumerate()
            .skip(start).take(visible)
            .map(|(i, e)| {
                let indent = "  ".repeat(e.depth);
                let label  = format!("{}{}", indent, e.name);
                let style  = if i == pc { rev } else { app.theme.dialog };
                Line::from(Span::styled(label, style))
            })
            .collect();
        frame.render_widget(Paragraph::new(pick_lines).style(app.theme.dialog), pi);
    }
}

// ── Section Properties dialog ─────────────────────────────────────────────────

pub fn render_sec_props_dialog(frame: &mut Frame, app: &App, area: Rect) {
    let (sec_idx, head_buf, head_cur, active_field, sort_state, filter_state, filter_scroll) = match &app.sec_mode {
        SectionMode::Props { sec_idx, head_buf, head_cur, active_field, sort_state, filter_state, filter_scroll } =>
            (*sec_idx, head_buf.as_str(), *head_cur, *active_field, sort_state, filter_state, *filter_scroll),
        _ => return,
    };

    let dlg = centered_rect(64, 11, area);
    frame.render_widget(Clear, dlg);
    let block = Block::default()
        .borders(Borders::ALL)
        .title_top(Line::from(" Section Properties ").alignment(Alignment::Center))
        .title_bottom(Line::from(" Press ENTER when done, ESC to cancel ").alignment(Alignment::Center))
        .style(app.theme.dialog_border);
    frame.render_widget(block.clone(), dlg);
    let inner = block.inner(dlg);

    let rev        = app.theme.item_selected_field;
    let dim        = app.theme.dim;
    let dlabel     = app.theme.dialog_label;
    let dlabel_sel = app.theme.dialog_label_sel;
    let iw  = inner.width as usize;

    // Left column width (labels + field): ~36 chars. Right column: Columns list.
    let left_w  = 38usize;
    let right_x = left_w;

    // ── Section head field ──
    let head_label = "Section head:  ";
    let field_w    = left_w.saturating_sub(head_label.len()).min(22);
    let head_field: Line = if active_field == SecPropsField::Head {
        let (left, hi, right) = super::cursor_split(head_buf, head_cur);
        let pad = field_w.saturating_sub(head_buf.chars().count());
        Line::from(vec![
            Span::raw(head_label),
            Span::styled(left, rev),
            Span::styled(hi, rev),
            Span::styled(right, rev),
            Span::styled(" ".repeat(pad), rev),
        ])
    } else {
        let displayed: String = head_buf.chars().take(field_w).collect();
        let pad = field_w.saturating_sub(displayed.chars().count());
        Line::from(format!("{}{}{}", head_label, displayed, " ".repeat(pad)))
    };

    // ── Item sorting field ──
    let sort_label = "Item sorting:       ";
    let sort_val   = if sec_idx < app.view.sections.len() {
        let sec = &app.view.sections[sec_idx];
        if sec.primary_on == SortOn::None && sec.secondary_on == SortOn::None {
            "..."
        } else {
            sec.primary_on.label()
        }
    } else { "..." };

    // ── Columns list (right side) ──
    let col_header = "Columns:";
    let mut col_names: Vec<String> = vec!["<Items>".to_string()];
    col_names.extend(app.view.columns.iter().map(|c| c.name.clone()));

    // Build lines: merge left and right.
    // Row 0: blank
    // Row 1: head field | "Columns:"
    // Row 2..col_end: blank (left) | col names
    // Row after cols: sort field
    // Row: stats (dim)
    // blank
    // Row: filter label (dim)
    // blank

    let pad_to = |s: &str, w: usize| -> String {
        let n = s.chars().count();
        if n < w { format!("{}{}", s, " ".repeat(w - n)) } else { s.chars().take(w).collect() }
    };

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from("")); // row 0 blank

    // Row 1: head + "Columns:"
    {
        let left_str = head_field;
        // We'll overlay; use a combined line approach
        // Actually build merged spans
        let right_str = if 0 < col_names.len() { col_header } else { "" };
        // Pad left spans to left_w, then append right
        // Simplification: render left and right as separate paragraphs in sub-rects
        // For now build a single line with padding
        lines.push(Line::from(vec![
            Span::raw(""), // placeholder — we'll render with sub-rects below
        ]));
        let _ = right_str;
        let _ = left_str;
    }

    // We'll use a sub-rect approach instead. Let's just build a clean Paragraph.
    // Reset and use Layout splitting.
    let use_layout = true;
    let _ = use_layout;

    // Simpler: build each row as a padded String with both columns concatenated.
    let mut final_lines: Vec<Line> = Vec::new();
    final_lines.push(Line::from("")); // blank

    // Row 1: head | Columns:
    {
        let head_active = active_field == SecPropsField::Head;
        let left_part: Vec<Span> = if head_active {
            let (left, hi, right) = super::cursor_split(head_buf, head_cur);
            let used = head_label.len() + head_buf.chars().count();
            let pad = left_w.saturating_sub(used);
            vec![
                Span::styled(head_label, dlabel_sel),
                Span::styled(left, rev),
                Span::styled(hi, rev),
                Span::styled(right, rev),
                Span::styled(" ".repeat(pad), rev),
            ]
        } else {
            let displayed: String = head_buf.chars().take(field_w).collect();
            let val_pad = left_w.saturating_sub(head_label.len() + displayed.chars().count());
            vec![
                Span::styled(head_label, dlabel),
                Span::raw(format!("{}{}", displayed, " ".repeat(val_pad))),
            ]
        };
        let mut spans = left_part;
        if !col_names.is_empty() {
            spans.push(Span::styled(col_header, dlabel));
        }
        final_lines.push(Line::from(spans));
    }

    // Rows for column names (starting from index 0 = "<Items>")
    for (ci, cname) in col_names.iter().enumerate() {
        let left_part = match ci {
            0 => pad_to("", left_w),
            1 => {
                // "Item sorting:" row — row 3
                let sort_str = format!("{}{}", sort_label, sort_val);
                if active_field == SecPropsField::ItemSorting {
                    // handled below as styled span
                    pad_to(&sort_str, left_w)
                } else {
                    pad_to(&sort_str, left_w)
                }
            }
            2 => pad_to("Section statistics: ...", left_w),
            _ => pad_to("", left_w),
        };
        let name_display: String = cname.chars().take(iw.saturating_sub(right_x + 2)).collect();

        if ci == 1 && active_field == SecPropsField::ItemSorting {
            // Sort field row with reverse styling
            let pad_needed = left_w.saturating_sub(sort_label.len() + sort_val.len());
            final_lines.push(Line::from(vec![
                Span::styled(sort_label, dlabel_sel),
                Span::styled(sort_val, rev),
                Span::raw(" ".repeat(pad_needed)),
                Span::raw(format!("  {}", name_display)),
            ]));
        } else if ci == 1 {
            // Sort field row — inactive
            let pad_needed = left_w.saturating_sub(sort_label.len() + sort_val.len());
            final_lines.push(Line::from(vec![
                Span::styled(sort_label, dlabel),
                Span::raw(sort_val),
                Span::raw(" ".repeat(pad_needed)),
                Span::raw(format!("  {}", name_display)),
            ]));
        } else if ci == 2 {
            final_lines.push(Line::from(vec![
                Span::styled(format!("{:<width$}", "Section statistics: ...", width = left_w), dim),
                Span::raw(format!("  {}", name_display)),
            ]));
        } else {
            let right_part = format!("  {}", name_display);
            final_lines.push(Line::from(format!("{}{}", left_part, right_part)));
        }
    }

    // If cols < 3, fill remaining left-side rows
    let col_rows_filled = col_names.len();
    if col_rows_filled <= 1 {
        // Item sorting row
        if active_field == SecPropsField::ItemSorting {
            final_lines.push(Line::from(vec![
                Span::styled(sort_label, dlabel_sel),
                Span::styled(sort_val, rev),
            ]));
        } else {
            final_lines.push(Line::from(vec![
                Span::styled(sort_label, dlabel),
                Span::raw(sort_val),
            ]));
        }
    }
    if col_rows_filled <= 2 {
        final_lines.push(Line::from(Span::styled("Section statistics: ...", dim)));
    }

    // ── Filter field — label (never highlighted), then up to 2 entry lines ──
    let filter_entry_w = left_w.saturating_sub(2); // 2 chars for "▲ " / "▼ " / "  "

    let filter_entries: Vec<String> = if sec_idx < app.view.sections.len() {
        let all_cats = flatten_cats(&app.categories);
        app.view.sections[sec_idx].filter.iter().map(|f| {
            let name = all_cats.iter().find(|c| c.id == f.cat_id)
                .map(|c| c.name.as_str()).unwrap_or("?");
            if f.op == FilterOp::Exclude { format!("-{}", name) } else { name.to_string() }
        }).collect()
    } else { vec![] };

    let total = filter_entries.len();
    let start = filter_scroll.min(if total > 2 { total - 2 } else { 0 });
    let row1_raw = filter_entries.get(start    ).map(|s| s.as_str()).unwrap_or("");
    let row2_raw = filter_entries.get(start + 1).map(|s| s.as_str()).unwrap_or("");
    let row1_str: String = row1_raw.chars().take(filter_entry_w).collect();
    let row2_str: String = row2_raw.chars().take(filter_entry_w).collect();
    let pad1 = filter_entry_w.saturating_sub(row1_str.chars().count());
    let pad2 = filter_entry_w.saturating_sub(row2_str.chars().count());
    // Scroll arrows: ▲ on line 1 if entries exist above, ▼ on line 2 if entries exist below.
    let arrow1 = if start > 0              { "\u{25B2}" } else { " " }; // ▲
    let arrow2 = if start + 2 < total      { "\u{25BC}" } else { " " }; // ▼

    let is_active = active_field == SecPropsField::Filter;

    // Label line — never highlighted.
    final_lines.push(Line::from(Span::styled("Filter:", dlabel)));
    // Entry line 1.
    final_lines.push(Line::from(if is_active {
        vec![
            Span::raw(format!("{} ", arrow1)),
            Span::styled(format!("{}{}", row1_str, " ".repeat(pad1)), rev),
        ]
    } else {
        vec![Span::raw(format!("{} {}", arrow1, row1_str))]
    }));
    // Entry line 2.
    final_lines.push(Line::from(if is_active {
        vec![
            Span::raw(format!("{} ", arrow2)),
            Span::styled(format!("{}{}", row2_str, " ".repeat(pad2)), rev),
        ]
    } else {
        vec![Span::raw(format!("{} {}", arrow2, row2_str))]
    }));

    frame.render_widget(Paragraph::new(final_lines).style(app.theme.dialog), inner);
    let _ = lines; // unused above

    // ── Filter picker overlay ─────────────────────────────────────────────────
    if let FilterState::Open { cursor, entries } = filter_state {
        let cursor = *cursor;
        let all_cats = flatten_cats(&app.categories);
        let max_vis  = 20usize;
        let visible  = all_cats.len().min(max_vis);
        let start    = if cursor >= visible { cursor - visible + 1 } else { 0 };

        let box_h = (visible + 2) as u16;
        let box_w = 62u16;
        let dlg_rect = centered_rect(box_w, box_h, area);
        frame.render_widget(Clear, dlg_rect);
        let block = Block::default()
            .borders(Borders::ALL)
            .title_top(Line::from(" Filter ").alignment(Alignment::Center))
            .title_bottom(Line::from(" Press ENTER to accept ").alignment(Alignment::Center))
            .style(app.theme.dialog_border);
        frame.render_widget(block.clone(), dlg_rect);
        let inner = block.inner(dlg_rect);

        let inner_w = inner.width as usize;
        let mut cat_lines: Vec<Line> = Vec::new();
        for (i, cat) in all_cats.iter().enumerate().skip(start).take(visible) {
            let marker = match entries.get(&cat.id).copied() {
                None                    => ' ',
                Some(FilterOp::Include) => '+',
                Some(FilterOp::Exclude) => '-',
            };
            let note_ind = cat_note_indicator(&app.categories, cat.id);
            let kind_ind = match cat.kind {
                CategoryKind::Standard  => if !note_ind.is_empty() { note_ind } else { " " },
                CategoryKind::Date      => "*",
                CategoryKind::Numeric   => "#",
                CategoryKind::Unindexed => "\u{25A1}",
            };
            let indent  = "  ".repeat(cat.depth);
            let name_w  = inner_w.saturating_sub(6 + indent.len());
            let name: String = cat.name.chars().take(name_w).collect();
            let row_text = format!(" {} \u{2502}{}{} {}", marker, indent, kind_ind, name);
            if i == cursor {
                cat_lines.push(Line::from(Span::styled(row_text, rev)));
            } else {
                cat_lines.push(Line::from(row_text));
            }
        }
        frame.render_widget(Paragraph::new(cat_lines).style(app.theme.dialog), inner);
    }

    // ── Sort dialog overlay ───────────────────────────────────────────────────
    if let SortState::Dialog {
        sort_new, primary_on, primary_order, primary_na, primary_cat_id, primary_seq,
        secondary_on, secondary_order, secondary_na, secondary_cat_id, secondary_seq,
        active_field: sf, picker,
    } = sort_state {
        render_sort_dialog(
            frame, app, area,
            " Item Sorting ",
            *sort_new,
            *primary_on, *primary_order, *primary_na, *primary_cat_id, *primary_seq,
            *secondary_on, *secondary_order, *secondary_na, *secondary_cat_id, *secondary_seq,
            *sf, picker.as_ref(),
        );
    }
}

pub fn render_sort_dialog(
    frame:            &mut Frame,
    app:              &App,
    area:             Rect,
    title:            &str,
    sort_new:         SortNewItems,
    primary_on:       SortOn,
    primary_order:    SortOrder,
    primary_na:       crate::model::SortNa,
    primary_cat_id:   Option<usize>,
    primary_seq:      SortSeq,
    secondary_on:     SortOn,
    secondary_order:  SortOrder,
    secondary_na:     crate::model::SortNa,
    secondary_cat_id: Option<usize>,
    secondary_seq:    SortSeq,
    active_field:     SortField,
    picker:           Option<&crate::app::SortPicker>,
) {
    let flat_cats = flatten_cats(&app.categories);
    let cat_name = |id: usize| -> &str {
        flat_cats.iter().find(|e| e.id == id).map(|e| e.name.as_str()).unwrap_or("?")
    };

    let dlg = centered_rect(64, 20, area);
    frame.render_widget(Clear, dlg);
    let block = Block::default()
        .borders(Borders::ALL)
        .title_top(Line::from(title.to_string()).alignment(Alignment::Center))
        .title_bottom(Line::from(" Press ENTER when done, ESC to cancel ").alignment(Alignment::Center))
        .style(app.theme.dialog_border);
    frame.render_widget(block.clone(), dlg);
    let inner = block.inner(dlg);

    let rev        = app.theme.item_selected_field;
    let dlabel     = app.theme.dialog_label;
    let dlabel_sel = app.theme.dialog_label_sel;
    // Value style: rev when active, plain (inherit dialog) when inactive.
    let fval = |active: bool| if active { rev } else { Style::default() };
    // Label style: dlabel_sel when active, dlabel when inactive.
    let flbl = |active: bool| if active { dlabel_sel } else { dlabel };

    let sort_new_label = "Sort new items:  ";
    let sort_on_label  = "  Sort on:       ";
    let order_label    = "  Order:         ";
    let na_label       = "  Sort n/a's:    ";
    let cat_label      = "  Category:      ";
    let seq_label      = "  Sequence:      ";
    let vd             = " (View default)";

    let mut rows: Vec<Line> = Vec::new();
    rows.push(Line::from(""));
    {
        let a = active_field == SortField::SortNewItems;
        rows.push(Line::from(vec![
            Span::styled(sort_new_label, flbl(a)),
            Span::styled(sort_new.label(), fval(a)),
        ]));
    }
    rows.push(Line::from(""));
    rows.push(Line::from(Span::styled(format!("Primary sort key{}", vd), dlabel)));
    {
        let a = active_field == SortField::PrimaryOn;
        rows.push(Line::from(vec![
            Span::styled(sort_on_label, flbl(a)),
            Span::styled(primary_on.label(), fval(a)),
        ]));
    }
    if primary_on != SortOn::None {
        let a = active_field == SortField::PrimaryOrder;
        rows.push(Line::from(vec![
            Span::styled(order_label, flbl(a)),
            Span::styled(primary_order.label(), fval(a)),
        ]));
        let a = active_field == SortField::PrimaryNa;
        rows.push(Line::from(vec![
            Span::styled(na_label, flbl(a)),
            Span::styled(primary_na.label(), fval(a)),
        ]));
    }
    if primary_on == SortOn::Category {
        let display = primary_cat_id.map(cat_name).unwrap_or("(choose)");
        let a = active_field == SortField::PrimaryCategory;
        rows.push(Line::from(vec![
            Span::styled(cat_label, flbl(a)),
            Span::styled(display, fval(a)),
        ]));
        if let Some(_) = primary_cat_id {
            let a = active_field == SortField::PrimarySequence;
            rows.push(Line::from(vec![
                Span::styled(seq_label, flbl(a)),
                Span::styled(primary_seq.label(), fval(a)),
            ]));
        }
    }
    rows.push(Line::from(""));
    rows.push(Line::from(Span::styled(format!("Secondary sort key{}", vd), dlabel)));
    {
        let a = active_field == SortField::SecondaryOn;
        rows.push(Line::from(vec![
            Span::styled(sort_on_label, flbl(a)),
            Span::styled(secondary_on.label(), fval(a)),
        ]));
    }
    if secondary_on != SortOn::None {
        let a = active_field == SortField::SecondaryOrder;
        rows.push(Line::from(vec![
            Span::styled(order_label, flbl(a)),
            Span::styled(secondary_order.label(), fval(a)),
        ]));
        let a = active_field == SortField::SecondaryNa;
        rows.push(Line::from(vec![
            Span::styled(na_label, flbl(a)),
            Span::styled(secondary_na.label(), fval(a)),
        ]));
    }
    if secondary_on == SortOn::Category {
        let display = secondary_cat_id.map(cat_name).unwrap_or("(choose)");
        let a = active_field == SortField::SecondaryCategory;
        rows.push(Line::from(vec![
            Span::styled(cat_label, flbl(a)),
            Span::styled(display, fval(a)),
        ]));
        if let Some(_) = secondary_cat_id {
            let a = active_field == SortField::SecondarySequence;
            rows.push(Line::from(vec![
                Span::styled(seq_label, flbl(a)),
                Span::styled(secondary_seq.label(), fval(a)),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(rows).style(app.theme.dialog), inner);

    // ── Sort picker overlay ───────────────────────────────────────────────────
    if let Some(p) = picker {
        match p.target {
            SortField::PrimaryCategory | SortField::SecondaryCategory => {
                // Category picker — scrollable flat list
                let visible = 10usize;
                let h = (visible.min(flat_cats.len()) + 2) as u16;
                let w = 36u16;
                let pick_rect = centered_rect(w, h, area);
                frame.render_widget(Clear, pick_rect);
                let pick_block = Block::default().borders(Borders::ALL)
                    .title(" Choose Category ").style(app.theme.dialog_border);
                frame.render_widget(pick_block.clone(), pick_rect);
                let pick_inner = pick_block.inner(pick_rect);
                let vis = pick_inner.height as usize;
                let start = if p.cursor >= vis { p.cursor - vis + 1 } else { 0 };
                let pick_lines: Vec<Line> = flat_cats.iter().enumerate()
                    .skip(start).take(vis)
                    .map(|(i, e)| {
                        let indent = "  ".repeat(e.depth);
                        let label  = format!("{}{}", indent, e.name);
                        let style  = if i == p.cursor { rev } else { Style::default() };
                        Line::from(Span::styled(label, style))
                    })
                    .collect();
                frame.render_widget(Paragraph::new(pick_lines).style(app.theme.dialog), pick_inner);
            }
            _ => {
                // Simple choices list
                let choices: &[&str] = match p.target {
                    SortField::SortNewItems => &[SortNewItems::OnDemand.label(), SortNewItems::WhenEntered.label(), SortNewItems::OnLeavingSection.label()],
                    SortField::PrimaryOn | SortField::SecondaryOn => &["None", "Item text", "Category", "Category note"],
                    SortField::PrimaryOrder | SortField::SecondaryOrder => &["Ascending", "Descending"],
                    SortField::PrimaryNa | SortField::SecondaryNa => &["Bottom of section", "Top of section"],
                    SortField::PrimarySequence | SortField::SecondarySequence =>
                        &["Category hierarchy", "Alphabetic", "Numeric", "Date"],
                    _ => &[],
                };
                let h = (choices.len() as u16) + 2;
                let w = choices.iter().map(|s| s.len()).max().unwrap_or(10) as u16 + 4;
                let pick_rect = centered_rect(w, h, area);
                frame.render_widget(Clear, pick_rect);
                let pick_block = Block::default().borders(Borders::ALL)
                    .title(" Choices ").style(app.theme.dialog_border);
                frame.render_widget(pick_block.clone(), pick_rect);
                let pick_inner = pick_block.inner(pick_rect);
                let pick_lines: Vec<Line> = choices.iter().enumerate()
                    .map(|(i, label)| {
                        let style = if i == p.cursor { rev } else { Style::default() };
                        Line::from(Span::styled(*label, style))
                    })
                    .collect();
                frame.render_widget(Paragraph::new(pick_lines).style(app.theme.dialog), pick_inner);
            }
        }
    }
}
