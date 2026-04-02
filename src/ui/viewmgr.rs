use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use crate::app::{App, ViewMgrMode};
use super::cursor_split;

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Draw the View screen as background.
    super::view::render(frame, app);

    // ── ViewMgr popup ─────────────────────────────────────────────────────────
    let view_count = 1 + app.inactive_views.len();
    let popup_h = ((view_count as u16) + 4).clamp(6, area.height.saturating_sub(4));
    let popup_w = 52u16.min(area.width.saturating_sub(4));
    let popup_rect = centered_rect(popup_w, popup_h, area);

    frame.render_widget(Clear, popup_rect);

    let block = Block::default()
        .borders(Borders::ALL)
        .title_top(Line::from(" View Manager ").alignment(Alignment::Center))
        .title_bottom(
            Line::from(" Press ENTER when done, ESC to cancel ")
                .alignment(Alignment::Center),
        );
    frame.render_widget(block.clone(), popup_rect);
    let inner = block.inner(popup_rect);

    // ── View list ─────────────────────────────────────────────────────────────
    let cursor = app.vmgr_state.cursor;
    let all_views = std::iter::once(&app.view).chain(app.inactive_views.iter());
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(""));  // top padding

    for (i, view) in all_views.enumerate() {
        let marker = if i == 0 { ">" } else { " " };
        let cursor_here = i == cursor;

        let line = if cursor_here {
            match &app.vmgr_state.mode {
                ViewMgrMode::Rename { buffer, cursor: buf_cur } => {
                    let (left, hi, right) = cursor_split(buffer, *buf_cur);
                    Line::from(vec![
                        Span::raw(format!("{} ", marker)),
                        Span::raw(left),
                        Span::styled(hi, Style::default().add_modifier(Modifier::REVERSED)),
                        Span::raw(right),
                    ])
                }
                _ => Line::from(vec![
                    Span::raw(format!("{} ", marker)),
                    Span::styled(
                        view.name.clone(),
                        Style::default().add_modifier(Modifier::REVERSED),
                    ),
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

    frame.render_widget(Paragraph::new(lines).style(Style::default()), inner);

    // ── ConfirmDelete overlay ─────────────────────────────────────────────────
    if let ViewMgrMode::ConfirmDelete { yes } = &app.vmgr_state.mode {
        let view_name = if cursor == 0 {
            app.view.name.as_str()
        } else {
            app.inactive_views
                .get(cursor - 1)
                .map(|v| v.name.as_str())
                .unwrap_or("")
        };

        let modal_rect = centered_rect(48, 7, area);
        frame.render_widget(Clear, modal_rect);
        let del_block = Block::default().borders(Borders::ALL).title(" Delete View? ");
        frame.render_widget(del_block.clone(), modal_rect);
        let del_inner = del_block.inner(modal_rect);

        let rev = Style::default().add_modifier(Modifier::REVERSED);
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

    // ── Props overlay ─────────────────────────────────────────────────────────
    if let ViewMgrMode::Props { buffer, cursor: buf_cur } = &app.vmgr_state.mode {
        let view_ref = if cursor == 0 {
            &app.view
        } else {
            match app.inactive_views.get(cursor - 1) {
                Some(v) => v,
                None    => &app.view,
            }
        };

        let sec_count  = view_ref.sections.len();
        // Count distinct items that appear in any section of this view.
        let item_count = {
            let mut seen = std::collections::HashSet::new();
            for si in 0..sec_count {
                for gi in crate::app::section_item_indices(&app.items, view_ref, si, &app.categories) {
                    seen.insert(gi);
                }
            }
            seen.len()
        };

        let modal_rect = centered_rect(54, 9, area);
        frame.render_widget(Clear, modal_rect);
        let props_block = Block::default()
            .borders(Borders::ALL)
            .title_top(Line::from(" View Properties ").alignment(Alignment::Center))
            .title_bottom(
                Line::from(" Press ENTER when done, ESC to cancel ")
                    .alignment(Alignment::Center),
            );
        frame.render_widget(props_block.clone(), modal_rect);
        let props_inner = props_block.inner(modal_rect);

        let field_w = 22usize;
        let (left, hi, right) = cursor_split(buffer, *buf_cur);
        let left_part: String  = left.chars().take(field_w).collect();
        let right_part: String = {
            let used = left_part.chars().count() + 1;  // +1 for hi char
            right.chars().take(field_w.saturating_sub(used)).collect()
        };

        let iw = props_inner.width as usize;

        let rows = vec![
            Line::from(""),
            Line::from(vec![
                Span::raw("  Name:      "),
                Span::raw(left_part),
                Span::styled(hi, Style::default().add_modifier(Modifier::REVERSED)),
                Span::raw(right_part),
                Span::raw(" ".repeat(iw)),  // fill rest of line
            ]),
            Line::from(format!("  Sections:  {}", sec_count)),
            Line::from(format!("  Items:     {}", item_count)),
            Line::from(""),
        ];
        frame.render_widget(Paragraph::new(rows), props_inner);
    }
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect { x, y, width: w, height: h }
}
