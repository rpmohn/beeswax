use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};
use crate::app::{App, SortPicker, ViewMgrMode, ViewPropsField, section_item_indices};
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
        let del_block = Block::default().borders(Borders::ALL).title(" Delete View? ");
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
        frame.render_widget(Paragraph::new(del_rows), del_inner);
    }
}

/// Renders the View Properties dialog as a floating overlay over any screen.
pub fn render_view_props_overlay(frame: &mut Frame, app: &App, area: Rect) {
    let props = match &app.vmgr_state.mode {
        ViewMgrMode::Props { .. } => match &app.vmgr_state.mode { ViewMgrMode::Props {
            name_buf, name_cur,
            sec_cursor,
            sort_state,
            sec_sort_method, sec_sort_order, sec_sort_picker,
            hide_empty_sections, hide_done_items, hide_dependent_items,
            hide_inherited_items, hide_column_heads, section_separators, number_items,
            active_field, sec_scroll,
        } => (name_buf, *name_cur, *sec_cursor, sort_state,
              *sec_sort_method, *sec_sort_order, sec_sort_picker,
              *hide_empty_sections, *hide_done_items,
              *hide_dependent_items, *hide_inherited_items, *hide_column_heads,
              *section_separators, *number_items, *active_field, *sec_scroll),
        _ => return },
        _ => return,
    };
    let (name_buf, name_cur, sec_cursor, sort_state,
         sec_sort_method, sec_sort_order, sec_sort_picker,
         hes, hdi, hdep, hii, hch, ss, ni, active_field, sec_scroll) = props;

    // The view being edited
    let v_cursor = app.vmgr_state.cursor;
    let voi      = app.view_order_idx;
    let view_ref = if v_cursor == voi { &app.view }
                   else {
                       let ii = if v_cursor < voi { v_cursor } else { v_cursor - 1 };
                       app.inactive_views.get(ii).unwrap_or(&app.view)
                   };

    let dlg = centered_rect(64, 19, area);
    frame.render_widget(Clear, dlg);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .style(app.theme.dialog_border)
        .title_top(Line::from(" View Properties ").alignment(Alignment::Center))
        .title_bottom(Line::from(" Press ENTER when done, ESC to cancel ").alignment(Alignment::Center));
    frame.render_widget(block.clone(), dlg);
    let inner = block.inner(dlg);
    let content = Rect {
        x:      inner.x + 1,
        y:      inner.y,
        width:  inner.width.saturating_sub(2),
        height: inner.height,
    };

    let rev       = app.theme.item_selected_field;
    let dlabel    = app.theme.dialog_label;
    let dlabel_sel = app.theme.dialog_label_sel;
    let iw  = content.width as usize;

    // Layout constants
    let left_w   = 36usize;   // left column width (labels + values)
    let right_x  = left_w;   // right column starts here

    let yn = |v: bool| if v { "Yes" } else { "No" };
    let pad_to = |s: &str, w: usize| -> String {
        let n = s.chars().count();
        if n >= w { s.chars().take(w).collect() } else { format!("{}{}", s, " ".repeat(w - n)) }
    };

    // Helper: build a Yes/No field line with separate label and value spans.
    let bool_line = |label: &str, val: bool, field: ViewPropsField, right: &str| -> Line<'static> {
        let val_str = yn(val);
        let val_w   = left_w.saturating_sub(label.chars().count());
        let is_active = active_field == field;
        let lbl_style = if is_active { dlabel_sel } else { dlabel };
        let mut spans: Vec<Span<'static>> = if is_active {
            vec![
                Span::styled(label.to_string(), lbl_style),
                Span::styled(format!("{:<w$}", val_str, w = val_w), rev),
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
    let is_secs_active = active_field == ViewPropsField::Sections;

    // Returns (text, highlight) for each right-column slot.
    // slot 0 → blank, slot 1 → "Sections:" header, slot 2+ → section names.
    let right_slot = |slot: usize| -> (String, bool) {
        match slot {
            0 => (String::new(), false),
            1 => ("Sections:".chars().take(right_avail).collect(), false),
            n => {
                let idx = sec_scroll + n - 2;
                let text = if let Some(name) = sec_names.get(idx) {
                    let indented = format!("  {}", name);
                    indented.chars().take(right_avail).collect()
                } else {
                    String::new()
                };
                let hi = is_secs_active && idx == sec_cursor;
                (text, hi)
            }
        }
    };

    // Build a single Line from left text (already padded to left_w) and right slot.
    let row_with_right = |left_padded: String, slot: usize| -> Line<'static> {
        let (right_text, highlight) = right_slot(slot);
        if highlight {
            let padded = format!("{:<w$}", right_text, w = right_avail);
            Line::from(vec![
                Span::raw(left_padded),
                Span::styled(padded, rev),
            ])
        } else {
            Line::from(format!("{}{}", left_padded, right_text))
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
    let name_field_w = left_w.saturating_sub("View name:   ".len());

    // Row 0: blank
    let mut rows: Vec<Line<'static>> = vec![Line::from("")];

    // Row 1: View name | Sections:
    {
        let label = "View name:   ";
        let (right_text, _) = right_slot(0);
        let right_padded = format!("{:>width$}", right_text, width = right_avail);
        if active_field == ViewPropsField::Name {
            let (left, hi, rt) = cursor_split(name_buf, name_cur);
            let lp: String = left.chars().take(name_field_w).collect();
            let rp: String = {
                let used = lp.chars().count() + 1;
                rt.chars().take(name_field_w.saturating_sub(used)).collect()
            };
            let pad = name_field_w.saturating_sub(lp.chars().count() + 1 + rp.chars().count());
            rows.push(Line::from(vec![
                Span::styled(label.to_string(), dlabel_sel),
                Span::styled(lp, rev),
                Span::styled(hi, rev),
                Span::styled(rp, rev),
                Span::styled(" ".repeat(pad), rev),
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
        let (right_text, right_hi) = right_slot(slot);
        let right_span = if right_hi {
            Span::styled(format!("{:<w$}", right_text, w = right_avail), rev)
        } else {
            Span::raw(right_text)
        };
        if active_field == field {
            Line::from(vec![
                Span::styled(label, dlabel_sel),
                Span::styled(format!("{:<w$}", val, w = val_w), rev),
                right_span,
            ])
        } else {
            Line::from(vec![
                Span::styled(label, dlabel),
                Span::raw(format!("{:<w$}", val, w = val_w)),
                right_span,
            ])
        }
    };

    // Row 2: Item sorting | sec[0]
    rows.push(sort_line("Item sorting:      ", "...", ViewPropsField::ItemSorting, 1));

    // Row 3: Section sorting | sec[1]
    rows.push(sort_line("Section sorting:   ", sec_sort_method.label(), ViewPropsField::SectionSorting, 2));

    // Row 4: "  Order:" if section sort != None, else blank | sec[2]
    if sec_sort_method != crate::model::SectionSortMethod::None {
        rows.push(sort_line("  Order:           ", sec_sort_order.label(), ViewPropsField::SectionSortOrder, 3));
    } else {
        rows.push(row_with_right(format!("{:<w$}", "", w = left_w), 3));
    }

    // Row 5-11: Yes/No fields, right column continues
    let bool_fields: &[(_, _, ViewPropsField)] = &[
        ("Hide empty sections:   ", hes,  ViewPropsField::HideEmptySections),
        ("Hide done items:       ", hdi,  ViewPropsField::HideDoneItems),
        ("Hide dependent items:  ", hdep, ViewPropsField::HideDependentItems),
        ("Hide inherited items:  ", hii,  ViewPropsField::HideInheritedItems),
        ("Hide column heads:     ", hch,  ViewPropsField::HideColumnHeads),
        ("Section separators:    ", ss,   ViewPropsField::SectionSeparators),
        ("Number items:          ", ni,   ViewPropsField::NumberItems),
    ];
    for (row_offset, (label, val, field)) in bool_fields.iter().enumerate() {
        let slot = 4 + row_offset;
        let (right_text, right_hi) = right_slot(slot);
        let right_str = if *field == ViewPropsField::NumberItems && right_text.is_empty() {
            "Filter:".to_string()
        } else {
            right_text
        };
        if active_field == *field {
            let val_str = yn(*val);
            let val_w   = left_w.saturating_sub(label.chars().count());
            let right_span = if right_hi {
                Span::styled(format!("{:<w$}", right_str, w = right_avail), rev)
            } else {
                Span::raw(right_str)
            };
            rows.push(Line::from(vec![
                Span::styled(label.to_string(), dlabel_sel),
                Span::styled(format!("{:<w$}", val_str, w = val_w), rev),
                right_span,
            ]));
        } else if right_hi {
            let val_str = yn(*val);
            let val_w   = left_w.saturating_sub(label.chars().count());
            rows.push(Line::from(vec![
                Span::styled(label.to_string(), dlabel),
                Span::raw(format!("{:<w$}", val_str, w = val_w)),
                Span::styled(format!("{:<w$}", right_str, w = right_avail), rev),
            ]));
        } else {
            rows.push(bool_line(label, *val, *field, &right_str));
        }
    }

    // Row 12: blank
    rows.push(Line::from(""));

    // Row 13: View statistics
    {
        let label = "View statistics:   ";
        let val   = format!("{} items", item_count);
        let val_w = left_w.saturating_sub(label.chars().count());
        if active_field == ViewPropsField::ViewStatistics {
            rows.push(Line::from(vec![
                Span::styled(label, dlabel_sel),
                Span::styled(format!("{:<w$}", val, w = val_w), rev),
            ]));
        } else {
            rows.push(Line::from(vec![
                Span::styled(label, dlabel),
                Span::raw(format!("{:<w$}", val, w = val_w)),
            ]));
        }
    }

    // Row 14: blank
    rows.push(Line::from(""));

    // Row 15: View protection
    {
        let label = "View protection:   ";
        let val   = "Global (No protection)";
        let val_w = left_w.saturating_sub(label.chars().count());
        if active_field == ViewPropsField::ViewProtection {
            rows.push(Line::from(vec![
                Span::styled(label, dlabel_sel),
                Span::styled(format!("{:<w$}", val, w = val_w), rev),
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
            .title_top(Line::from(title).alignment(Alignment::Center));
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
        frame.render_widget(Paragraph::new(pick_lines), pick_inner);
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
