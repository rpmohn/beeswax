use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use crate::app::{App, ChoicesKind, ColFormField, ColMode, ColPos, CursorPos, MenuState, Mode,
                 PropsField, TimeField, flatten_cats, format_date_value};
use crate::model::{CategoryKind, Column, DateDisplay, Clock, DateFmtCode};
use super::{cursor_split, fkeys, menu};

const SECTION_PREFIX: &str = " ";
const ITEM_PREFIX:    &str = "    \u{2022} ";

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
        let title = Paragraph::new(vec![
            Line::from(Span::raw(format!(" BEESWAX 0.1{:>68}", "2026-03-04"))),
            Line::from(Span::raw(format!(" View: {}", app.view.name))),
        ])
        .style(Style::default().add_modifier(Modifier::REVERSED));
        frame.render_widget(title, chunks[0]);
    } else {
        menu::render_bar(frame, chunks[0], app);
    }

    // ── Body ─────────────────────────────────────────────────────────────
    let body_block = Block::default().borders(Borders::NONE);
    let body_inner = body_block.inner(chunks[1]);
    frame.render_widget(body_block, chunks[1]);

    // Column layout: left columns | main items column | right columns.
    // Each added column occupies col.width + 1 chars (the +1 is the '|' separator).
    let total_body_w = body_inner.width as usize;
    let added_w: usize = app.view.columns.iter().map(|c| c.width + 1).sum();
    let main_col_w = total_body_w.saturating_sub(added_w);
    let lc = app.view.left_count;
    let left_cols  = &app.view.columns[..lc];
    let right_cols = &app.view.columns[lc..];

    // Determine which column index (0-based into view.columns) is active.
    // active_col is Some(i) when col_cursor > 0 and col_cursor-1 == i.
    let active_col: Option<usize> = if app.col_cursor > 0 {
        Some(app.col_cursor - 1)
    } else {
        None
    };

    let mut lines: Vec<Line> = Vec::new();

    for (s_idx, section) in app.view.sections.iter().enumerate() {
        let cursor_on_head = matches!(&app.cursor, CursorPos::SectionHead(i) if *i == s_idx);

        // ── Section head row ─────────────────────────────────────────────
        // Left column header cells
        let left_head_vals: Vec<String> = left_cols.iter().map(|c| c.name.clone()).collect();
        let left_active  = if cursor_on_head { active_col.filter(|&i| i < lc) } else { None };
        let left_head_edit = head_cell_edit(left_active, &app.mode);
        let left_head_spans = col_cells(left_cols, &left_head_vals,
                                        left_active, left_head_edit);

        // Main column content
        let pfx_w      = SECTION_PREFIX.chars().count();
        let max_name_w = main_col_w.saturating_sub(pfx_w);
        let (mut head_spans, head_used): (Vec<Span<'static>>, usize) = if cursor_on_head {
            match &app.mode {
                Mode::Normal => {
                    let name: String = section.name.chars().take(max_name_w).collect();
                    let w = pfx_w + name.chars().count();
                    let style = if app.col_cursor == 0 {
                        Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
                    } else {
                        Style::default().add_modifier(Modifier::BOLD)
                    };
                    (vec![Span::raw(SECTION_PREFIX), Span::styled(name, style)], w)
                }
                Mode::Edit { buffer, cursor, col, .. } if *col == 0 => {
                    let (left, hi, right) = cursor_split(buffer, *cursor);
                    let w = pfx_w + buffer.chars().count();
                    (vec![
                        Span::raw(SECTION_PREFIX),
                        Span::styled(left,  Style::default().add_modifier(Modifier::BOLD)),
                        Span::styled(hi,    Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED)),
                        Span::styled(right, Style::default().add_modifier(Modifier::BOLD)),
                    ], w)
                }
                _ => {
                    let name: String = section.name.chars().take(max_name_w).collect();
                    let w = pfx_w + name.chars().count();
                    (vec![Span::raw(SECTION_PREFIX),
                          Span::styled(name, Style::default().add_modifier(Modifier::BOLD))], w)
                }
            }
        } else {
            let name: String = section.name.chars().take(max_name_w).collect();
            let w = pfx_w + name.chars().count();
            (vec![Span::raw(SECTION_PREFIX),
                  Span::styled(name, Style::default().add_modifier(Modifier::BOLD))], w)
        };
        if head_used < main_col_w {
            head_spans.push(Span::raw(" ".repeat(main_col_w - head_used)));
        }

        // Right column header cells
        let right_head_vals: Vec<String> = right_cols.iter().map(|c| c.name.clone()).collect();
        let right_active = if cursor_on_head { active_col.filter(|&i| i >= lc).map(|i| i - lc) } else { None };
        let right_head_edit = head_cell_edit(right_active.map(|i| i + lc), &app.mode);
        let right_head_spans = col_cells(right_cols, &right_head_vals,
                                         right_active, right_head_edit);

        let mut row = left_head_spans;
        row.extend(head_spans);
        row.extend(right_head_spans);
        lines.push(Line::from(row));

        if cursor_on_head {
            if let Mode::Create { buffer, cursor } = &app.mode {
                let used  = ITEM_PREFIX.chars().count() + buffer.chars().count();
                let empty: Vec<String> = app.view.columns.iter().map(|_| String::new()).collect();
                let left_empty  = &empty[..lc];
                let right_empty = &empty[lc..];
                let mut spans = col_cells(left_cols, left_empty, None, None);
                spans.extend(input_row_spans(buffer, *cursor));
                if used < main_col_w { spans.push(Span::raw(" ".repeat(main_col_w - used))); }
                spans.extend(col_cells(right_cols, right_empty, None, None));
                lines.push(Line::from(spans));
            }
        }

        // ── Item rows ────────────────────────────────────────────────────
        for (i_idx, item) in section.items.iter().enumerate() {
            let cursor_on_item = matches!(
                &app.cursor,
                CursorPos::Item { section: si, item: ii } if *si == s_idx && *ii == i_idx
            );

            let pfx_w      = ITEM_PREFIX.chars().count();
            let max_text_w = main_col_w.saturating_sub(pfx_w);
            let item_text: String = item.text.chars().take(max_text_w).collect();
            let item_w    = pfx_w + item_text.chars().count();

            // All column values for this item
            let all_vals: Vec<String> = app.view.columns.iter()
                .map(|c| item.values.get(&c.cat_id).cloned().unwrap_or_default())
                .collect();
            let left_vals  = &all_vals[..lc];
            let right_vals = &all_vals[lc..];

            // Which cell (if any) is in edit mode
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

            let left_item_active  = item_active_col.filter(|&i| i < lc);
            let right_item_active = item_active_col.filter(|&i| i >= lc).map(|i| i - lc);
            let left_item_edit    = item_cell_edit.filter(|_| left_item_active.is_some());
            let right_item_edit   = item_cell_edit.filter(|_| right_item_active.is_some());

            // Left column value cells
            let left_item_spans = col_cells(left_cols, left_vals,
                                            left_item_active, left_item_edit);

            // Main column content
            let mut item_spans: Vec<Span<'static>> = if cursor_on_item {
                match &app.mode {
                    Mode::Normal => {
                        let style = if app.col_cursor == 0 {
                            Style::default().add_modifier(Modifier::REVERSED)
                        } else {
                            Style::default().add_modifier(Modifier::BOLD)
                        };
                        vec![Span::raw(ITEM_PREFIX), Span::styled(item_text, style)]
                    }
                    Mode::Edit { buffer, cursor, col, .. } if *col == 0 => {
                        let (left, hi, right) = cursor_split(buffer, *cursor);
                        vec![
                            Span::raw(ITEM_PREFIX),
                            Span::raw(left),
                            Span::styled(hi, Style::default().add_modifier(Modifier::REVERSED)),
                            Span::raw(right),
                        ]
                    }
                    Mode::Edit { .. } =>
                        vec![Span::raw(ITEM_PREFIX),
                             Span::styled(item_text, Style::default().add_modifier(Modifier::BOLD))],
                    Mode::Create { .. } =>
                        vec![Span::raw(ITEM_PREFIX), Span::raw(item_text)],
                }
            } else {
                vec![Span::raw(ITEM_PREFIX), Span::raw(item_text)]
            };
            if item_w < main_col_w {
                item_spans.push(Span::raw(" ".repeat(main_col_w - item_w)));
            }

            // Right column value cells
            let right_item_spans = col_cells(right_cols, right_vals,
                                             right_item_active, right_item_edit);

            let mut row = left_item_spans;
            row.extend(item_spans);
            row.extend(right_item_spans);
            lines.push(Line::from(row));

            if cursor_on_item {
                if let Mode::Create { buffer, cursor } = &app.mode {
                    let used  = ITEM_PREFIX.chars().count() + buffer.chars().count();
                    let empty: Vec<String> = app.view.columns.iter().map(|_| String::new()).collect();
                    let left_empty  = &empty[..lc];
                    let right_empty = &empty[lc..];
                    let mut spans = col_cells(left_cols, left_empty, None, None);
                    spans.extend(input_row_spans(buffer, *cursor));
                    if used < main_col_w { spans.push(Span::raw(" ".repeat(main_col_w - used))); }
                    spans.extend(col_cells(right_cols, right_empty, None, None));
                    lines.push(Line::from(spans));
                }
            }
        }
    }

    frame.render_widget(Paragraph::new(lines), body_inner);

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
        let block = Block::default().borders(Borders::ALL).title(title);
        frame.render_widget(block.clone(), modal_rect);
        let inner = block.inner(modal_rect);

        let flat = flatten_cats(&app.categories);
        let cat_name = head_cat_idx
            .and_then(|i| flat.get(i))
            .map(|c| c.name.as_str())
            .unwrap_or("");

        let rev = Style::default().add_modifier(Modifier::REVERSED);

        // Head — fully highlighted when active; show at least one space when blank
        let head_line = {
            let label = Span::raw(" Column head:  ");
            if active_field == ColFormField::Head {
                let display = if cat_name.is_empty() { " " } else { cat_name };
                Line::from(vec![label, Span::styled(display.to_string(), rev)])
            } else {
                Line::from(vec![label, Span::raw(cat_name.to_string())])
            }
        };

        // Width — fully highlighted when active
        let width_line = {
            let label = Span::raw(" Width:        ");
            if active_field == ColFormField::Width {
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
            let label = Span::raw(" Position:     ");
            if active_field == ColFormField::Position {
                Line::from(vec![label, Span::styled(pos_label, rev)])
            } else {
                Line::from(vec![label, Span::raw(pos_label)])
            }
        } else {
            Line::from(vec![Span::raw(" Position:     "), Span::raw(pos_label)])
        };

        let form_lines = vec![
            Line::from(""),
            head_line,
            width_line,
            position_line,
            Line::from(" Format:       Name only"),
            Line::from(""),
            Line::from(" Category type: Standard    Insert in: All sections"),
            Line::from(""),
            Line::from(" \u{2500}\u{2500}\u{2500} Press ENTER when done, ESC to cancel \u{2500}\u{2500}\u{2500}"),
        ];

        frame.render_widget(Paragraph::new(form_lines), inner);
    }

    // ── Choices picker overlay ────────────────────────────────────────────
    if let ColMode::Choices { picker_cursor, kind, .. } = &app.col_mode {
        let rev = Style::default().add_modifier(Modifier::REVERSED);

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

        let block = Block::default().borders(Borders::ALL).title(" Choices ");
        frame.render_widget(block.clone(), picker_rect);
        let inner = block.inner(picker_rect);

        // Scroll so picker_cursor is visible
        let visible = inner.height as usize;
        let offset  = if *picker_cursor >= visible { picker_cursor - visible + 1 } else { 0 };

        let visible_lines: Vec<Line> = lines.into_iter()
            .skip(offset)
            .take(visible)
            .collect();

        frame.render_widget(Paragraph::new(visible_lines), inner);
    }

    // ── Quick-add category picker (Alt-R / Alt-L) ────────────────────────────
    if let ColMode::QuickAdd { position, picker_cursor } = &app.col_mode {
        let rev   = Style::default().add_modifier(Modifier::REVERSED);
        let flat  = flatten_cats(&app.categories);
        let title = match position {
            ColPos::Right => " Add Column Right ",
            ColPos::Left  => " Add Column Left ",
        };
        let lines: Vec<Line> = flat.iter().enumerate().map(|(i, cat)| {
            let indent    = " ".repeat(cat.depth * 2 + 1);
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
        }).collect();

        let picker_h = (lines.len() as u16 + 2).min(area.height.saturating_sub(4)).max(4);
        let picker_rect = centered_rect(40, picker_h, area);
        frame.render_widget(Clear, picker_rect);

        let block = Block::default().borders(Borders::ALL).title(title);
        frame.render_widget(block.clone(), picker_rect);
        let inner = block.inner(picker_rect);

        let visible = inner.height as usize;
        let offset  = if *picker_cursor >= visible { picker_cursor - visible + 1 } else { 0 };
        let visible_lines: Vec<Line> = lines.into_iter().skip(offset).take(visible).collect();
        frame.render_widget(Paragraph::new(visible_lines), inner);
    }

    // ── Column Properties modal ───────────────────────────────────────────────
    if let ColMode::Props { head_buf, head_cur, width_buf, width_cur,
                            date_fmt, active_field, is_date } = &app.col_mode {
        let modal_h = if *is_date { 18u16 } else { 8u16 };
        let modal_rect = centered_rect(66, modal_h, area);
        frame.render_widget(Clear, modal_rect);

        let block = Block::default().borders(Borders::ALL).title(" Column Properties ");
        frame.render_widget(block.clone(), modal_rect);
        let inner = block.inner(modal_rect);

        let rev = Style::default().add_modifier(Modifier::REVERSED);

        // Helper: field value span (REVERSED when active)
        let field_span = |label: &'static str, val: String, af: PropsField, target: PropsField| -> Vec<Span<'static>> {
            if af == target {
                vec![Span::raw(label), Span::styled(val, rev)]
            } else {
                vec![Span::raw(label), Span::raw(val)]
            }
        };

        // Column head — editable text with cursor
        let head_line = {
            let label = Span::raw(" Column head:  ");
            let cat_type = if *is_date { "Date" } else { "Standard" };
            let type_span = Span::raw(format!("    Category type: {}", cat_type));
            if *active_field == PropsField::Head {
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
            let label = Span::raw(" Width:        ");
            if *active_field == PropsField::Width {
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

        let mut form_lines: Vec<Line<'static>> = vec![
            Line::from(""),
            head_line,
            width_line,
        ];

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

        frame.render_widget(Paragraph::new(form_lines), inner);
    }

    // ── Calendar modal ────────────────────────────────────────────────────────
    if let ColMode::Calendar { year, month, day, hour, min, sec } = &app.col_mode {
        let (cal_year, cal_month, cal_day) = (*year, *month, *day);
        let (cal_hour, cal_min, cal_sec)   = (*hour, *min, *sec);
        let (today_y, today_m, today_d)    = cal_today();
        let dim   = cal_days_in_month(cal_year, cal_month);
        let start = cal_first_dow(cal_year, cal_month) as usize;
        let mname = CAL_MONTH_NAMES[(cal_month as usize).saturating_sub(1)];

        // Box: 24 wide (22 inner), 11 tall (9 inner)
        let cal_rect = centered_rect(24, 11, area);
        frame.render_widget(Clear, cal_rect);
        let block = Block::default().borders(Borders::ALL).title(" Calendar ");
        frame.render_widget(block.clone(), cal_rect);
        let inner = block.inner(cal_rect);
        let iw = inner.width as usize;

        let rev  = Style::default().add_modifier(Modifier::REVERSED);
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

        frame.render_widget(Paragraph::new(cal_lines), inner);
    }

    // ── SetTime modal ─────────────────────────────────────────────────────────
    if let ColMode::SetTime { year, month, day, hour_buf, min_buf, sec_buf, active, .. } = &app.col_mode {
        let rev = Style::default().add_modifier(Modifier::REVERSED);

        let st_rect = centered_rect(28, 7, area);
        frame.render_widget(Clear, st_rect);
        let block = Block::default().borders(Borders::ALL).title(" Set Time ");
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
        ]), inner);
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

/// Pad `s` to exactly `w` chars (or truncate if longer).
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

/// Append `| cell` for each column in `columns`.
/// `values` must have the same length as `columns`.
/// `active_col` (0-indexed within `columns`) highlights that cell.
/// `cell_edit` supplies (buffer, cursor) to show an edit cursor in the active cell.
fn col_cells(
    columns: &[Column],
    values: &[String],
    active_col: Option<usize>,
    cell_edit: Option<(&str, usize)>,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for (i, (col, val)) in columns.iter().zip(values.iter()).enumerate() {
        spans.push(Span::raw("|"));
        let display_val = if let Some(ref fmt) = col.date_fmt {
            format_date_value(val, fmt)
        } else {
            val.clone()
        };
        if active_col == Some(i) {
            if let Some((buf, cur)) = cell_edit {
                spans.extend(cell_edit_spans(buf, cur, col.width));
            } else {
                let cell = pad_or_trunc(&display_val, col.width);
                spans.push(Span::styled(cell, Style::default().add_modifier(Modifier::REVERSED)));
            }
        } else {
            let cell = pad_or_trunc(&display_val, col.width);
            spans.push(Span::raw(cell));
        }
    }
    spans
}

/// Spans for a column cell in edit mode: scrolling window + cursor-highlighted char.
fn cell_edit_spans(buffer: &str, cursor: usize, width: usize) -> Vec<Span<'static>> {
    let chars: Vec<char> = buffer.chars().collect();
    let cur   = cursor.min(chars.len());
    // Scroll the window left so the cursor is always visible.
    let start = if cur + 1 > width { cur + 1 - width } else { 0 };
    let visible: String = chars[start..].iter().take(width).collect();
    let cur_in_win = cur - start;
    let (left, hi, right) = cursor_split(&visible, cur_in_win);
    let used = left.chars().count() + 1 + right.chars().count();
    let pad  = width.saturating_sub(used);
    let mut spans = vec![
        Span::raw(left),
        Span::styled(hi, Style::default().add_modifier(Modifier::REVERSED)),
        Span::raw(right),
    ];
    if pad > 0 {
        spans.push(Span::raw(" ".repeat(pad)));
    }
    spans
}

/// Spans for a new-item input row: prefix + buffer text with cursor-highlighted char.
fn input_row_spans(buffer: &str, cursor: usize) -> Vec<Span<'static>> {
    let (left, hi, right) = cursor_split(buffer, cursor);
    vec![
        Span::raw(ITEM_PREFIX),
        Span::raw(left),
        Span::styled(hi, Style::default().add_modifier(Modifier::REVERSED)),
        Span::raw(right),
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
