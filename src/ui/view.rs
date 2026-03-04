use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use crate::app::{App, CursorPos, MenuState, Mode};
use super::{cursor_split, menu};

const FKEY_BAR: &str =
    "F1=Help  F2=Edit  F5=Note  F6=Props  F7=Mark  F8=VwMgr  F9=CatMgr  F10=Menu";

const SECTION_PREFIX: &str = " ";
const ITEM_PREFIX:    &str = "    \u{2022} ";

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),  // title bar
            Constraint::Min(0),     // body
            Constraint::Length(1),  // fkey bar
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

    let mut lines: Vec<Line> = Vec::new();

    for (s_idx, section) in app.view.sections.iter().enumerate() {
        let cursor_on_head = matches!(&app.cursor, CursorPos::SectionHead(i) if *i == s_idx);

        // ── Section head row ─────────────────────────────────────────────
        let head_line = if cursor_on_head {
            match &app.mode {
                Mode::Normal => Line::from(vec![
                    Span::raw(SECTION_PREFIX),
                    Span::styled(
                        section.name.clone(),
                        Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD),
                    ),
                ]),
                Mode::Edit { buffer, cursor, .. } => {
                    let (left, hi, right) = cursor_split(buffer, *cursor);
                    Line::from(vec![
                        Span::raw(SECTION_PREFIX),
                        Span::styled(left,  Style::default().add_modifier(Modifier::BOLD)),
                        Span::styled(hi,    Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED)),
                        Span::styled(right, Style::default().add_modifier(Modifier::BOLD)),
                    ])
                }
                // Create mode: section head loses highlight; input row appears below
                Mode::Create { .. } => Line::from(vec![
                    Span::raw(SECTION_PREFIX),
                    Span::styled(section.name.clone(), Style::default().add_modifier(Modifier::BOLD)),
                ]),
            }
        } else {
            Line::from(vec![
                Span::raw(SECTION_PREFIX),
                Span::styled(section.name.clone(), Style::default().add_modifier(Modifier::BOLD)),
            ])
        };
        lines.push(head_line);

        // Create-mode: cursor row loses highlight; input row appears below
        if cursor_on_head {
            if let Mode::Create { buffer, cursor } = &app.mode {
                lines.push(input_row(buffer, *cursor));
            }
        }

        // ── Item rows ────────────────────────────────────────────────────
        for (i_idx, item) in section.items.iter().enumerate() {
            let cursor_on_item = matches!(
                &app.cursor,
                CursorPos::Item { section: si, item: ii } if *si == s_idx && *ii == i_idx
            );

            let item_line = if cursor_on_item {
                match &app.mode {
                    Mode::Normal => Line::from(vec![
                        Span::raw(ITEM_PREFIX),
                        Span::styled(
                            item.text.clone(),
                            Style::default().add_modifier(Modifier::REVERSED),
                        ),
                    ]),
                    Mode::Edit { buffer, cursor, .. } => {
                        let (left, hi, right) = cursor_split(buffer, *cursor);
                        Line::from(vec![
                            Span::raw(ITEM_PREFIX),
                            Span::raw(left),
                            Span::styled(hi, Style::default().add_modifier(Modifier::REVERSED)),
                            Span::raw(right),
                        ])
                    }
                    // Create mode: item loses highlight; input row appears below
                    Mode::Create { .. } => Line::from(vec![
                        Span::raw(ITEM_PREFIX),
                        Span::raw(item.text.clone()),
                    ]),
                }
            } else {
                Line::from(vec![
                    Span::raw(ITEM_PREFIX),
                    Span::raw(item.text.clone()),
                ])
            };
            lines.push(item_line);

            // Create-mode input row after this item
            if cursor_on_item {
                if let Mode::Create { buffer, cursor } = &app.mode {
                    lines.push(input_row(buffer, *cursor));
                }
            }
        }
    }

    frame.render_widget(Paragraph::new(lines), body_inner);

    // ── F-key bar ─────────────────────────────────────────────────────────
    frame.render_widget(
        Paragraph::new(FKEY_BAR).style(Style::default().add_modifier(Modifier::REVERSED)),
        chunks[2],
    );
}

/// A new-item input row: prefix + text with cursor-highlighted character.
fn input_row(buffer: &str, cursor: usize) -> Line<'static> {
    let (left, hi, right) = cursor_split(buffer, cursor);
    Line::from(vec![
        Span::raw(ITEM_PREFIX),
        Span::raw(left),
        Span::styled(hi, Style::default().add_modifier(Modifier::REVERSED)),
        Span::raw(right),
    ])
}
