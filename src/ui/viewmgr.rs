use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};
use crate::app::{App, FilterState, SortPicker, ViewMgrMode, ViewPropsField, flatten_cats, section_item_indices, cat_note_indicator};
use crate::model::{CategoryKind, FilterOp};
use crate::app::SortState;
use super::cursor_split;

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Draw the View screen as background (its title bar shows Ctrl hints).
    super::view::render(frame, app);

    // ── ViewMgr popup ─────────────────────────────────────────────────────────
    let view_count = 1 + app.inactive_views.len();
    let popup_h = ((view_count as u16) + 4).clamp(6, area.height.saturating_sub(4));
    let popup_w = 52u16.min(area.width.saturating_sub(4));
    let popup_rect = centered_rect(popup_w, popup_h, area);

    frame.render_widget(Clear, popup_rect);

    let block = Block::default()
        .borders(Borders::ALL)
        .style(app.theme.dialog_border)
        .title_top(Line::from(" View Manager ").alignment(Alignment::Center))
        .title_bottom(
            Line::from(" Press ENTER when done, ESC to cancel ")
                .alignment(Alignment::Center),
        );
    frame.render_widget(block.clone(), popup_rect);
    let inner = block.inner(popup_rect);

    // ── View list ─────────────────────────────────────────────────────────────
    let cursor = app.vmgr_state.cursor;
    let voi    = app.view_order_idx;
    let count  = 1 + app.inactive_views.len();
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(""));  // top padding

    for i in 0..count {
        let view = if i == voi { &app.view }
                   else {
                       let ii = if i < voi { i } else { i - 1 };
                       &app.inactive_views[ii]
                   };
        let marker = if i == voi { ">" } else { " " };
        let cursor_here = i == cursor;

        let line = if cursor_here {
            match &app.vmgr_state.mode {
                ViewMgrMode::Rename { buffer, cursor: buf_cur } => {
                    let (left, hi, right) = cursor_split(buffer, *buf_cur);
                    Line::from(vec![
                        Span::raw(format!("{} ", marker)),
                        Span::raw(left),
                        Span::styled(hi, app.theme.item_selected_field),
                        Span::raw(right),
                    ])
                }
                _ => Line::from(vec![
                    Span::raw(format!("{} ", marker)),
                    Span::styled(view.name.clone(), app.theme.item_selected_field),
                ]),
            }
        } else {
            Line::from(vec![
                Span::raw(format!("{} ", marker)),
                Span::raw(view.name.clone()),
            ])
        };
        lines.push(line);
    }

    frame.render_widget(Paragraph::new(lines).style(app.theme.dialog), inner);

    // ── ConfirmDelete overlay ─────────────────────────────────────────────────
    if let ViewMgrMode::ConfirmDelete { yes } = &app.vmgr_state.mode {
        let view_name = if cursor == voi {
            app.view.name.as_str()
        } else {
            let ii = if cursor < voi { cursor } else { cursor - 1 };
            app.inactive_views.get(ii).map(|v| v.name.as_str()).unwrap_or("")
        };

        let modal_rect = centered_rect(48, 7, area);
        frame.render_widget(Clear, modal_rect);
        let del_block = Block::default().borders(Borders::ALL)
            .title(" Delete View? ").style(app.theme.dialog_border);
        frame.render_widget(del_block.clone(), modal_rect);
        let del_inner = del_block.inner(modal_rect);

        let rev = app.theme.item_selected_field;
        let yes_span = if *yes { Span::styled("[ Yes ]", rev) } else { Span::raw("[ Yes ]") };
        let no_span  = if !*yes { Span::styled("[ No ]", rev)  } else { Span::raw("[ No ]")  };

        let iw = del_inner.width as usize;
        let btn_text = "[ Yes ]      [ No ]";
        let btn_pad = iw.saturating_sub(btn_text.chars().count()) / 2;

        let del_rows = vec![
            Line::from(""),
            Line::from(Span::raw(format!("  \"{}\"", view_name))),
            Line::from(""),
            Line::from(vec![
                Span::raw(" ".repeat(btn_pad)),
                yes_span,
                Span::raw("      "),
                no_span,
            ]),
            Line::from(""),
        ];
        frame.render_widget(Paragraph::new(del_rows).style(app.theme.dialog), del_inner);
    }
}

/// Renders the View Properties / Add View dialog as a floating overlay over any screen.
pub fn render_view_props_overlay(frame: &mut Frame, app: &App, area: Rect) {
    let props = match &app.vmgr_state.mode {
        ViewMgrMode::Props { .. } => match &app.vmgr_state.mode { ViewMgrMode::Props {
            is_new,
            name_buf, name_cur, name_editing,
            sec_cursor,
            sort_state,
            sec_sort_method, sec_sort_order, sec_sort_picker,
            sec_add_picker,
            filter_state, filter_scroll, filter_cursor,
            hide_empty_sections, hide_done_items, hide_dependent_items,
            hide_inherited_items, hide_column_heads, section_separators, number_items,
            active_field, sec_scroll, name_scroll, ..
        } => (*is_new, name_buf, *name_cur, *name_scroll, *name_editing, *sec_cursor, sort_state,
              *sec_sort_method, *sec_sort_order, sec_sort_picker, sec_add_picker,
              filter_state, *filter_scroll, *filter_cursor,
              *hide_empty_sections, *hide_done_items,
              *hide_dependent_items, *hide_inherited_items, *hide_column_heads,
              *section_separators, *number_items, *active_field, *sec_scroll),
        _ => return },
        _ => return,
    };
    let (is_new, name_buf, name_cur, name_scroll, name_editing, sec_cursor, sort_state,
         sec_sort_method, sec_sort_order, sec_sort_picker, sec_add_picker,
         filter_state, filter_scroll, filter_cursor,
         hes, hdi, hdep, hii, hch, ss, ni, active_field, sec_scroll) = props;

    // The view being edited (for is_new this is the draft in inactive_views)
    let v_cursor = app.vmgr_state.cursor;
    let voi      = app.view_order_idx;
    let view_ref = if v_cursor == voi { &app.view }
                   else {
                       let ii = if v_cursor < voi { v_cursor } else { v_cursor - 1 };
                       app.inactive_views.get(ii).unwrap_or(&app.view)
                   };

    let dlg = centered_rect(66, 19, area);
    frame.render_widget(Clear, dlg);
    let title = if is_new { " Add View " } else { " View Properties " };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .style(app.theme.dialog_border)
        .title_top(Line::from(title).alignment(Alignment::Center))
        .title_bottom(Line::from(" Press ENTER when done, ESC to cancel ").alignment(Alignment::Center));
    frame.render_widget(block.clone(), dlg);
    let inner = block.inner(dlg);
    let content = inner;

    let rev       = app.theme.item_selected_field;
    let dlabel    = app.theme.dialog_label;
    let dlabel_sel = app.theme.dialog_label_sel;
    let iw  = content.width as usize;

    // Layout constants
    let left_w   = 36usize;   // left column width (labels + values)
    let right_x  = left_w;   // right column starts here

    let yn = |v: bool| if v { "Yes" } else { "No" };

    // Helper: build a Yes/No field line with separate label and value spans.
    let bool_line = |label: &str, val: bool, field: ViewPropsField, right: &str| -> Line<'static> {
        let val_str = yn(val);
        let val_w   = left_w.saturating_sub(label.chars().count());
        let is_active = active_field == field;
        let lbl_style = if is_active { dlabel_sel } else { dlabel };
        let mut spans: Vec<Span<'static>> = if is_active {
            let pad = val_w.saturating_sub(val_str.chars().count());
            vec![
                Span::styled(label.to_string(), lbl_style),
                Span::styled(val_str.to_string(), rev),
                Span::raw(" ".repeat(pad)),
            ]
        } else {
            vec![
                Span::styled(label.to_string(), lbl_style),
                Span::raw(format!("{:<w$}", val_str, w = val_w)),
            ]
        };
        if !right.is_empty() {
            spans.push(Span::raw(right.to_string()));
        }
        Line::from(spans)
    };

    // Sections list on right side (visible starting from sec_scroll).
    let sec_names: Vec<&str> = view_ref.sections.iter().map(|s| s.name.as_str()).collect();
    let right_avail = iw.saturating_sub(right_x);
    let is_secs_active   = active_field == ViewPropsField::Sections;
    let is_filter_active = active_field == ViewPropsField::Filter;
    let can_scroll_up   = sec_scroll > 0;
    let can_scroll_down = sec_scroll + 6 < sec_names.len();

    // Precompute filter entries for right-column display.
    let view_filter_entries: Vec<String> = {
        let all_cats = flatten_cats(&app.categories);
        view_ref.filter.iter().map(|f| {
            let name = all_cats.iter().find(|c| c.id == f.cat_id)
                .map(|c| c.name.as_str()).unwrap_or("?");
            if f.op == FilterOp::Exclude { format!("-{}", name) } else { name.to_string() }
        }).collect()
    };
    let filter_entry_w = right_avail.saturating_sub(2);
    let filter_total   = view_filter_entries.len();
    let filter_start   = filter_scroll.min(if filter_total > 2 { filter_total - 2 } else { 0 });

    // Returns (prefix, text, highlight, text_style) for each right-column slot.
    // prefix is unstyled; text_style applies when highlight=false (labels use dlabel/dlabel_sel).
    // slot 0 → blank, slot 1 → "Sections:" header, slots 2-7 → up to 6 section names,
    // slot 8 → "Filter:" label, slots 9-10 → filter entry lines.
    let right_slot = |slot: usize| -> (String, String, bool, Style) {
        match slot {
            0 => (String::new(), String::new(), false, Style::default()),
            1 => {
                let lbl_style = if is_secs_active { dlabel_sel } else { dlabel };
                (String::new(), "Sections:".chars().take(right_avail).collect(), false, lbl_style)
            }
            n if n >= 2 && n <= 7 => {
                let offset = n - 2;
                let idx = sec_scroll + offset;
                if let Some(name) = sec_names.get(idx) {
                    let arrow = if offset == 0 && can_scroll_up { "↑" }
                                else if offset == 5 && can_scroll_down { "↓" }
                                else { " " };
                    let prefix = format!("{} ", arrow);
                    let name_text: String = name.chars().take(25).collect();
                    let hi = is_secs_active && idx == sec_cursor;
                    (prefix, name_text, hi, Style::default())
                } else if n == 2 && is_secs_active && sec_names.is_empty() {
                    // No sections yet: show a highlighted blank so the cursor is visible.
                    (String::new(), " ".to_string(), true, Style::default())
                } else {
                    (String::new(), String::new(), false, Style::default())
                }
            }
            10 => {
                // Filter: label line (aligns with Number items row)
                let lbl_style = if is_filter_active { dlabel_sel } else { dlabel };
                ("".to_string(), "Filter:".chars().take(right_avail).collect(), false, lbl_style)
            }
            11 => {
                // Filter entry line 1 (aligns with blank row)
                let entry_idx = filter_start;
                let row_raw = view_filter_entries.get(entry_idx).map(|s| s.as_str()).unwrap_or("");
                let row_str: String = row_raw.chars().take(filter_entry_w).collect();
                let hi = is_filter_active && filter_cursor == entry_idx;
                let padded = if hi {
                    format!("{:<w$}", row_str, w = filter_entry_w)
                } else { row_str };
                let arrow = if filter_start > 0 { "\u{25B2}" } else { " " };
                (format!("{} ", arrow), padded, hi, Style::default())
            }
            12 => {
                // Filter entry line 2 (aligns with View statistics row)
                let entry_idx = filter_start + 1;
                let row_raw = view_filter_entries.get(entry_idx).map(|s| s.as_str()).unwrap_or("");
                let row_str: String = row_raw.chars().take(filter_entry_w).collect();
                let hi = is_filter_active && filter_cursor == entry_idx;
                let padded = if hi {
                    format!("{:<w$}", row_str, w = filter_entry_w)
                } else { row_str };
                let arrow = if filter_start + 2 < filter_total { "\u{25BC}" } else { " " };
                (format!("{} ", arrow), padded, hi, Style::default())
            }
            _ => (String::new(), String::new(), false, Style::default()),
        }
    };

    // Build a single Line from left text (already padded to left_w) and right slot.
    let row_with_right = |left_padded: String, slot: usize| -> Line<'static> {
        let (prefix, right_text, highlight, text_style) = right_slot(slot);
        if highlight {
            Line::from(vec![
                Span::raw(left_padded),
                Span::raw(prefix),
                Span::styled(right_text, rev),
            ])
        } else if right_text.is_empty() {
            Line::from(format!("{}{}", left_padded, prefix))
        } else {
            Line::from(vec![
                Span::raw(format!("{}{}", left_padded, prefix)),
                Span::styled(right_text, text_style),
            ])
        }
    };

    // View statistics: item count.
    let item_count = {
        let mut seen = std::collections::HashSet::new();
        for si in 0..view_ref.sections.len() {
            for gi in section_item_indices(&app.items, view_ref, si, &app.categories) {
                seen.insert(gi);
            }
        }
        seen.len()
    };

    // ── Build the dialog lines ─────────────────────────────────────────────────
    let name_field_w = left_w.saturating_sub("View name:   ".len()).min(20);

    // Row 0: blank
    let mut rows: Vec<Line<'static>> = vec![Line::from("")];

    // Row 1: View name | Sections:
    {
        let label = " View name:   ";
        let (_, right_text, _, _) = right_slot(0);
        let right_padded = format!("{:>width$}", right_text, width = right_avail);
        if active_field == ViewPropsField::Name && name_editing {
            let mut spans = vec![Span::styled(label.to_string(), dlabel_sel)];
            spans.extend(super::text_field_spans(name_buf, name_cur, name_scroll, name_field_w, Style::default(), rev));
            spans.push(Span::raw(right_padded));
            rows.push(Line::from(spans));
        } else if active_field == ViewPropsField::Name {
            // Selected but not editing — highlight only the text characters.
            let displayed: String = name_buf.chars().take(name_field_w).collect();
            let val_w = left_w.saturating_sub(label.chars().count());
            let pad = val_w.saturating_sub(displayed.chars().count());
            rows.push(Line::from(vec![
                Span::styled(label.to_string(), dlabel_sel),
                Span::styled(displayed, rev),
                Span::raw(" ".repeat(pad)),
                Span::raw(right_padded),
            ]));
        } else {
            let displayed: String = name_buf.chars().take(name_field_w).collect();
            let val_w = left_w.saturating_sub(label.chars().count());
            rows.push(Line::from(vec![
                Span::styled(label.to_string(), dlabel),
                Span::raw(format!("{:<w$}", displayed, w = val_w)),
                Span::raw(right_padded),
            ]));
        }
    }

    // Helper: build a single sorting-field line with separate label/value spans.
    let sort_line = |label: &'static str, val: &str, field: ViewPropsField, slot: usize| -> Line<'static> {
        let val_w = left_w.saturating_sub(label.chars().count());
        let (right_prefix, right_text, right_hi, text_style) = right_slot(slot);
        let right_spans: Vec<Span<'static>> = if right_hi {
            vec![Span::raw(right_prefix), Span::styled(right_text, rev)]
        } else if right_text.is_empty() {
            vec![Span::raw(right_prefix)]
        } else {
            vec![Span::raw(right_prefix), Span::styled(right_text, text_style)]
        };
        if active_field == field {
            let pad = val_w.saturating_sub(val.chars().count());
            let mut spans = vec![
                Span::styled(label, dlabel_sel),
                Span::styled(val.to_string(), rev),
                Span::raw(" ".repeat(pad)),
            ];
            spans.extend(right_spans);
            Line::from(spans)
        } else {
            let mut spans = vec![
                Span::styled(label, dlabel),
                Span::raw(format!("{:<w$}", val, w = val_w)),
            ];
            spans.extend(right_spans);
            Line::from(spans)
        }
    };

    // Row 2: Item sorting | sec[0]
    rows.push(sort_line(" Item sorting:      ", "...", ViewPropsField::ItemSorting, 1));

    // Row 3: Section sorting | sec[1]
    rows.push(sort_line(" Section sorting:   ", sec_sort_method.label(), ViewPropsField::SectionSorting, 2));

    // Row 4: "  Order:" if section sort != None, else blank | sec[2]
    if sec_sort_method != crate::model::SectionSortMethod::None {
        rows.push(sort_line("   Order:           ", sec_sort_order.label(), ViewPropsField::SectionSortOrder, 3));
    } else {
        rows.push(row_with_right(format!("{:<w$}", "", w = left_w), 3));
    }

    // Row 5-11: Yes/No fields, right column continues
    let bool_fields: &[(_, _, ViewPropsField)] = &[
        (" Hide empty sections:   ", hes,  ViewPropsField::HideEmptySections),
        (" Hide done items:       ", hdi,  ViewPropsField::HideDoneItems),
        (" Hide dependent items:  ", hdep, ViewPropsField::HideDependentItems),
        (" Hide inherited items:  ", hii,  ViewPropsField::HideInheritedItems),
        (" Hide column heads:     ", hch,  ViewPropsField::HideColumnHeads),
        (" Section separators:    ", ss,   ViewPropsField::SectionSeparators),
        (" Number items:          ", ni,   ViewPropsField::NumberItems),
    ];
    for (row_offset, (label, val, field)) in bool_fields.iter().enumerate() {
        let slot = 4 + row_offset;
        let (right_prefix, right_text, right_hi, text_style) = right_slot(slot);
        if active_field == *field {
            let val_str = yn(*val);
            let val_w   = left_w.saturating_sub(label.chars().count());
            let pad     = val_w.saturating_sub(val_str.chars().count());
            let right_spans: Vec<Span<'static>> = if right_hi {
                vec![Span::raw(right_prefix), Span::styled(right_text, rev)]
            } else if right_text.is_empty() {
                vec![Span::raw(right_prefix)]
            } else {
                vec![Span::raw(right_prefix), Span::styled(right_text, text_style)]
            };
            let mut spans = vec![
                Span::styled(label.to_string(), dlabel_sel),
                Span::styled(val_str.to_string(), rev),
                Span::raw(" ".repeat(pad)),
            ];
            spans.extend(right_spans);
            rows.push(Line::from(spans));
        } else if right_hi {
            let val_str = yn(*val);
            let val_w   = left_w.saturating_sub(label.chars().count());
            rows.push(Line::from(vec![
                Span::styled(label.to_string(), dlabel),
                Span::raw(format!("{:<w$}", val_str, w = val_w)),
                Span::raw(right_prefix),
                Span::styled(right_text, rev),
            ]));
        } else if right_text.is_empty() {
            rows.push(bool_line(label, *val, *field, ""));
        } else {
            // Non-highlighted right text with a style (label slot 10).
            let val_str = yn(*val);
            let val_w   = left_w.saturating_sub(label.chars().count());
            rows.push(Line::from(vec![
                Span::styled(label.to_string(), dlabel),
                Span::raw(format!("{:<w$}", val_str, w = val_w)),
                Span::raw(right_prefix),
                Span::styled(right_text, text_style),
            ]));
        }
    }

    // Row 12: blank | filter entry 1
    rows.push(row_with_right(format!("{:<w$}", "", w = left_w), 11));

    // Row 13: View statistics | filter entry 2
    {
        let label = " View statistics:   ";
        let val   = format!("{} items", item_count);
        let val_w = left_w.saturating_sub(label.chars().count());
        let (right_prefix, right_text, right_hi, text_style) = right_slot(12);
        let right_spans: Vec<Span<'static>> = if right_hi {
            vec![Span::raw(right_prefix), Span::styled(right_text, rev)]
        } else if right_text.is_empty() {
            vec![Span::raw(right_prefix)]
        } else {
            vec![Span::raw(right_prefix), Span::styled(right_text, text_style)]
        };
        if active_field == ViewPropsField::ViewStatistics {
            let mut spans = vec![Span::styled(label, dlabel_sel), Span::styled(format!("{:<w$}", val, w = val_w), rev)];
            spans.extend(right_spans);
            rows.push(Line::from(spans));
        } else {
            let mut spans = vec![
                Span::styled(label, dlabel),
                Span::raw(format!("{:<w$}", val, w = val_w)),
            ];
            spans.extend(right_spans);
            rows.push(Line::from(spans));
        }
    }

    // Row 14: blank
    rows.push(Line::from(""));

    // Row 15: View protection
    {
        let label = " View protection:   ";
        let val   = "Global (No protection)";
        let val_w = left_w.saturating_sub(label.chars().count());
        if active_field == ViewPropsField::ViewProtection {
            rows.push(Line::from(vec![
                Span::styled(label, dlabel_sel),
                Span::styled(val, rev),
            ]));
        } else {
            rows.push(Line::from(vec![
                Span::styled(label, dlabel),
                Span::raw(format!("{:<w$}", val, w = val_w)),
            ]));
        }
    }

    // Row 16: blank
    rows.push(Line::from(""));

    frame.render_widget(Paragraph::new(rows).style(app.theme.dialog), content);

    // ── Section sort picker popup ─────────────────────────────────────────────
    if let Some((target, cursor)) = sec_sort_picker {
        let (title, choices): (&str, Vec<&str>) = match target {
            crate::app::SecSortTarget::Method => (
                " Choices ",
                crate::model::SectionSortMethod::ALL.iter().map(|m| m.label()).collect(),
            ),
            crate::app::SecSortTarget::Order => (
                " Choices ",
                crate::model::SortOrder::ALL.iter().map(|o| o.label()).collect(),
            ),
        };
        let h = (choices.len() as u16 + 4).min(area.height.saturating_sub(4));
        let w = 30u16.min(area.width.saturating_sub(4));
        let pick_rect = centered_rect(w, h, area);
        frame.render_widget(Clear, pick_rect);
        let pick_block = Block::default()
            .borders(Borders::ALL)
            .title_top(Line::from(title).alignment(Alignment::Center))
            .style(app.theme.dialog_border);
        frame.render_widget(pick_block.clone(), pick_rect);
        let pick_inner = pick_block.inner(pick_rect);
        let mut pick_lines: Vec<Line<'static>> = vec![Line::from("")];
        for (i, label) in choices.iter().enumerate() {
            if i == *cursor {
                pick_lines.push(Line::from(Span::styled(
                    format!(" {}", label),
                    app.theme.item_selected_field,
                )));
            } else {
                pick_lines.push(Line::from(format!(" {}", label)));
            }
        }
        frame.render_widget(Paragraph::new(pick_lines).style(app.theme.dialog), pick_inner);
    }

    // ── Filter picker overlay (F3 on Filter field) ───────────────────────────
    if let FilterState::Open { cursor, entries } = filter_state {
        let cursor   = *cursor;
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

    // ── Section Select picker (F3 on Sections field) ──────────────────────────
    if let Some(picker_cur) = sec_add_picker {
        let cats = flatten_cats(&app.categories);
        // Collect cat_ids already used as sections in this view
        let used_ids: std::collections::HashSet<usize> =
            view_ref.sections.iter().map(|s| s.cat_id).collect();

        let visible_rows = cats.len().min(16);
        let picker_h = (visible_rows as u16 + 3).max(6).min(area.height.saturating_sub(4));
        let picker_w = 50u16.min(area.width.saturating_sub(4));
        let pick_rect = centered_rect(picker_w, picker_h, area);
        frame.render_widget(Clear, pick_rect);
        let pick_block = Block::default()
            .borders(Borders::ALL)
            .style(app.theme.dialog_border)
            .title_top(Line::from(" Section Select ").alignment(Alignment::Center))
            .title_bottom(Line::from(" Press ENTER to accept ").alignment(Alignment::Center));
        frame.render_widget(pick_block.clone(), pick_rect);
        let pick_inner = pick_block.inner(pick_rect);

        let view_name = view_ref.name.as_str();
        let view_label = format!(" Current View: {}", view_name);
        let view_line = Line::from(Span::styled(view_label, app.theme.dialog_label));

        let visible = (pick_inner.height as usize).saturating_sub(1); // -1 for header line
        let start = if *picker_cur >= visible { picker_cur - visible + 1 } else { 0 };
        let rev = app.theme.item_selected_field;

        let mut pick_lines: Vec<Line<'static>> = vec![view_line];
        for (i, entry) in cats.iter().enumerate().skip(start).take(visible) {
            let in_all   = view_ref.sec_all.contains(&entry.id);
            let in_subs  = view_ref.sec_subs.contains(&entry.id);
            let is_sec   = used_ids.contains(&entry.id);
            let marker = if in_all { "A" } else if in_subs { "S" } else if is_sec { "*" } else { " " };
            let note_ind = cat_note_indicator(&app.categories, entry.id);
            let type_ind = match entry.kind {
                CategoryKind::Standard  => if !note_ind.is_empty() { note_ind } else { " " },
                CategoryKind::Date      => "*",
                CategoryKind::Numeric   => "#",
                CategoryKind::Unindexed => "\u{25A1}",
            };
            let indent = "  ".repeat(entry.depth);
            let highlighted = i == *picker_cur;
            pick_lines.push(Line::from(vec![
                Span::raw(format!(" {}\u{2502} {}{} ", marker, indent, type_ind)),
                if highlighted {
                    Span::styled(entry.name.clone(), rev)
                } else {
                    Span::raw(entry.name.clone())
                },
            ]));
        }
        frame.render_widget(Paragraph::new(pick_lines).style(app.theme.dialog), pick_inner);
    }

    // ── Sort dialog overlay (when F3 opens Item Sorting sub-dialog) ───────────
    if let SortState::Dialog {
        sort_new, primary_on, primary_order, primary_na, primary_cat_id, primary_seq,
        secondary_on, secondary_order, secondary_na, secondary_cat_id, secondary_seq,
        active_field: sf, picker,
    } = sort_state {
        let picker_ref: Option<&SortPicker> = picker.as_ref();
        super::view::render_sort_dialog(
            frame, app, area,
            " Item Sorting in All Sections ",
            *sort_new,
            *primary_on, *primary_order, *primary_na, *primary_cat_id, *primary_seq,
            *secondary_on, *secondary_order, *secondary_na, *secondary_cat_id, *secondary_seq,
            *sf, picker_ref,
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
