use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crate::app::{App, AppScreen, CatMode, Mode};

pub fn handle_event(app: &mut App, event: Event) {
    let Event::Key(KeyEvent { code, modifiers, .. }) = event else { return };

    // Alt-Q always quits regardless of screen
    if modifiers.contains(KeyModifiers::ALT) && code == KeyCode::Char('q') {
        app.quit = true;
        return;
    }

    // Determine which handler to call without holding a borrow on app
    match app.screen {
        AppScreen::View => {
            let in_normal  = matches!(app.mode, Mode::Normal);
            let in_create  = matches!(app.mode, Mode::Create { .. });
            let in_edit    = matches!(app.mode, Mode::Edit   { .. });
            if      in_normal { handle_view_normal(app, code, modifiers) }
            else if in_create { handle_view_input(app, code) }
            else if in_edit   { handle_view_input(app, code) }
        }
        AppScreen::CatMgr => {
            let in_normal = matches!(app.cat_state.mode, CatMode::Normal);
            let in_edit   = matches!(app.cat_state.mode, CatMode::Edit   { .. });
            let in_create = matches!(app.cat_state.mode, CatMode::Create { .. });
            if      in_normal { handle_catmgr_normal(app, code, modifiers) }
            else if in_edit   { handle_catmgr_input(app, code) }
            else if in_create { handle_catmgr_input(app, code) }
        }
    }
}

// ── View handlers ─────────────────────────────────────────────────────────────

fn handle_view_normal(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    match code {
        KeyCode::Up     => app.cursor_up(),
        KeyCode::Down   => app.cursor_down(),
        KeyCode::Insert => app.begin_create_blank(),
        KeyCode::F(2) | KeyCode::Enter => app.begin_edit(),
        KeyCode::F(9)   => app.toggle_catmgr(),
        KeyCode::Char(ch) if modifiers.is_empty() => app.begin_create(ch),
        _ => {}
    }
}

fn handle_view_input(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Enter     => app.confirm(),
        KeyCode::Esc       => app.cancel(),
        KeyCode::Backspace => app.input_backspace(),
        KeyCode::Left      => app.edit_cursor_left(),
        KeyCode::Right     => app.edit_cursor_right(),
        KeyCode::Char(ch)  => app.input_char(ch),
        _ => {}
    }
}

// ── CatMgr handlers ───────────────────────────────────────────────────────────

fn handle_catmgr_normal(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    if modifiers.contains(KeyModifiers::ALT) {
        match code {
            KeyCode::Char('r') => app.cat_begin_create(true),   // child
            _ => {}
        }
        return;
    }
    match code {
        KeyCode::Up     => app.cat_cursor_up(),
        KeyCode::Down   => app.cat_cursor_down(),
        KeyCode::Insert => app.cat_begin_create(false),          // sibling
        KeyCode::F(2) | KeyCode::Enter => app.cat_begin_edit(),
        KeyCode::F(7)   => app.cat_promote(),
        KeyCode::F(8)   => app.cat_demote(),
        KeyCode::F(9)   => app.toggle_catmgr(),
        KeyCode::Delete => app.cat_delete(),
        _ => {}
    }
}

fn handle_catmgr_input(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Enter     => app.cat_confirm(),
        KeyCode::Esc       => app.cat_cancel(),
        KeyCode::Backspace => app.cat_input_backspace(),
        KeyCode::Left      => app.cat_edit_cursor_left(),
        KeyCode::Right     => app.cat_edit_cursor_right(),
        KeyCode::Char(ch)  => app.cat_input_char(ch),
        _ => {}
    }
}
