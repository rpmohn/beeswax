use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use crate::app::{App, AppScreen, MenuState};
use crate::menu::{SubItem, SubSubItem, TopItem, CATMGR_MENU, VIEW_MENU};

/// Render the two-row Lotus-style menu bar into `area` (which should be 2 rows tall).
/// Call this instead of the normal title bar when `app.menu != Closed`.
pub fn render_bar(frame: &mut Frame, area: Rect, app: &App) {
    let items: &'static [TopItem] = match app.screen {
        AppScreen::View   => VIEW_MENU,
        AppScreen::CatMgr => CATMGR_MENU,
    };

    let (row1, row2) = match app.menu {
        MenuState::Closed => return,

        MenuState::Top { cursor } => {
            let r1 = top_spans(items, cursor);
            let r2 = sub_name_spans(items[cursor].sub);
            (r1, r2)
        }

        MenuState::Sub { top, cursor } => {
            let r1 = sub_item_spans(items[top].sub, cursor);
            let desc = items[top].sub[cursor].description;
            let r2 = vec![Span::raw(format!(" {desc}"))];
            (r1, r2)
        }

        MenuState::SubSub { top, sub, cursor } => {
            let children = items[top].sub[sub].children.unwrap_or(&[]);
            let r1 = subsub_item_spans(children, cursor);
            let desc = if cursor < children.len() { children[cursor].description } else { "" };
            let r2 = vec![Span::raw(format!(" {desc}"))];
            (r1, r2)
        }
    };

    frame.render_widget(
        Paragraph::new(vec![Line::from(row1), Line::from(row2)])
            .style(Style::default().add_modifier(Modifier::REVERSED)),
        area,
    );
}

/// Build spans for the top-level item row.
/// The currently selected item is shown without REVERSED (appears as normal text
/// against the REVERSED bar background).
fn top_spans(items: &'static [TopItem], cursor: usize) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    spans.push(Span::raw(" "));
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        if i == cursor {
            spans.push(Span::styled(
                item.label,
                Style::default().remove_modifier(Modifier::REVERSED),
            ));
        } else {
            spans.push(Span::raw(item.label));
        }
    }
    spans
}

/// Build spans listing all sub-item labels (used on row 2 at Top level).
fn sub_name_spans(sub: &'static [SubItem]) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    spans.push(Span::raw(" "));
    for (i, item) in sub.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::raw(item.label));
    }
    spans
}

/// Build spans for the sub-item row (used on row 1 at Sub level).
/// The currently selected sub-item is shown without REVERSED.
fn sub_item_spans(sub: &'static [SubItem], cursor: usize) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    spans.push(Span::raw(" "));
    for (i, item) in sub.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        if i == cursor {
            spans.push(Span::styled(
                item.label,
                Style::default().remove_modifier(Modifier::REVERSED),
            ));
        } else {
            spans.push(Span::raw(item.label));
        }
    }
    spans
}

/// Build spans for the sub-sub-item row (used on row 1 at SubSub level).
fn subsub_item_spans(children: &'static [SubSubItem], cursor: usize) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    spans.push(Span::raw(" "));
    for (i, item) in children.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        if i == cursor {
            spans.push(Span::styled(
                item.label,
                Style::default().remove_modifier(Modifier::REVERSED),
            ));
        } else {
            spans.push(Span::raw(item.label));
        }
    }
    spans
}
