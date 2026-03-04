use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use crate::app::{App, CatMode, FlatCat, flatten_cats};
use crate::model::CategoryKind;
use super::cursor_split;

const FKEY_BAR: &str =
    "F2=Edit  F5=Note  F6=Props  F7=Prm  F8=Dem  F9=ToView  F10=Menu";

/// Spaces before the indicator column at a given depth.
fn base_indent(depth: usize) -> String {
    " ".repeat(depth * 2)
}

/// The single-character type indicator (space for Standard).
fn kind_indicator(kind: CategoryKind) -> &'static str {
    match kind {
        CategoryKind::Standard  => " ",
        CategoryKind::Date      => "*",
        CategoryKind::Numeric   => "#",
        CategoryKind::Unindexed => "D",
    }
}

/// Build the three leading spans common to every row:
///   base_indent | indicator | " "
/// Returns them as raw (unstyled) strings ready to pass to Span::raw.
fn leading(entry: &FlatCat) -> (String, &'static str) {
    (base_indent(entry.depth), kind_indicator(entry.kind))
}

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // title bar
            Constraint::Min(0),     // body
            Constraint::Length(1),  // fkey bar
        ])
        .split(area);

    // ── Title bar ────────────────────────────────────────────────────────────
    let title = Line::from(Span::raw(format!(
        " BEESWAX 0.1          Category Manager{:>22}",
        "2026-03-03"
    )));
    frame.render_widget(
        Paragraph::new(title).style(Style::default().add_modifier(Modifier::REVERSED)),
        chunks[0],
    );

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
            let (ind, kchar) = leading(entry);

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

    // ── F-key bar ─────────────────────────────────────────────────────────────
    frame.render_widget(
        Paragraph::new(FKEY_BAR).style(Style::default().add_modifier(Modifier::REVERSED)),
        chunks[2],
    );
}
