use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, ModifierKeyCode};
use crate::app::{App, AppScreen, CatMode, ColMode, ColFormField, ColPos, FKeyMod, MenuState, Mode};

pub fn handle_event(app: &mut App, event: Event) {
    let Event::Key(KeyEvent { code, modifiers, kind, .. }) = event else { return };

    // ── Modifier-only key events (requires enhanced keyboard protocol) ────────
    if let KeyCode::Modifier(mk) = code {
        match kind {
            KeyEventKind::Press | KeyEventKind::Repeat => {
                app.fkey_mod = match mk {
                    ModifierKeyCode::LeftShift  | ModifierKeyCode::RightShift   => FKeyMod::Shift,
                    ModifierKeyCode::LeftControl | ModifierKeyCode::RightControl => FKeyMod::Ctrl,
                    ModifierKeyCode::LeftAlt    | ModifierKeyCode::RightAlt     => FKeyMod::Alt,
                    _ => app.fkey_mod,
                };
            }
            KeyEventKind::Release => {
                let released = match mk {
                    ModifierKeyCode::LeftShift  | ModifierKeyCode::RightShift   => Some(FKeyMod::Shift),
                    ModifierKeyCode::LeftControl | ModifierKeyCode::RightControl => Some(FKeyMod::Ctrl),
                    ModifierKeyCode::LeftAlt    | ModifierKeyCode::RightAlt     => Some(FKeyMod::Alt),
                    _ => None,
                };
                if released == Some(app.fkey_mod) {
                    app.fkey_mod = FKeyMod::Normal;
                }
            }
        }
        return;
    }

    // Ignore key releases for regular keys
    if !matches!(kind, KeyEventKind::Press | KeyEventKind::Repeat) {
        return;
    }

    // Alt-Q always quits regardless of screen/mode
    if modifiers.contains(KeyModifiers::ALT) && code == KeyCode::Char('q') {
        app.quit = true;
        return;
    }

    // Menu takes priority over all other input
    if !matches!(app.menu, MenuState::Closed) {
        handle_menu(app, code);
        return;
    }

    // Calendar picker takes priority
    if matches!(app.col_mode, ColMode::Calendar { .. }) {
        handle_col_calendar(app, code, modifiers);
        return;
    }

    // SetTime modal takes priority
    if matches!(app.col_mode, ColMode::SetTime { .. }) {
        handle_col_set_time(app, code);
        return;
    }

    // Quick-add category picker takes priority
    if matches!(app.col_mode, ColMode::QuickAdd { .. }) {
        handle_col_quick_add(app, code);
        return;
    }

    // Column Properties modal takes priority
    if matches!(app.col_mode, ColMode::Props { .. }) {
        handle_col_props(app, code);
        return;
    }

    // Choices picker takes priority over form
    if matches!(app.col_mode, ColMode::Choices { .. }) {
        handle_col_choices(app, code);
        return;
    }

    // Column form modal takes priority over view input
    if matches!(app.col_mode, ColMode::Form { .. }) {
        handle_col_form(app, code);
        return;
    }

    // Column move mode
    if matches!(app.col_mode, ColMode::Move) {
        handle_col_move(app, code);
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
    if modifiers.contains(KeyModifiers::ALT) {
        match code {
            KeyCode::Char('r') => app.col_quick_add(ColPos::Right),
            KeyCode::Char('l') => app.col_quick_add(ColPos::Left),
            _ => {}
        }
        return;
    }
    match code {
        KeyCode::Up    => app.cursor_up(),
        KeyCode::Down  => app.cursor_down(),
        KeyCode::Left  => app.cursor_col_left(),
        KeyCode::Right => app.cursor_col_right(),
        KeyCode::Insert => app.begin_create_blank(),
        KeyCode::F(2) | KeyCode::Enter => app.begin_edit(),
        KeyCode::F(3)   => app.col_open_calendar(),
        KeyCode::F(6)   => app.col_open_props(),
        KeyCode::F(9)   => app.toggle_catmgr(),
        KeyCode::F(10)  => app.open_menu(),
        KeyCode::Char(ch) if modifiers.is_empty() => app.begin_char_input(ch),
        _ => {}
    }
}

fn handle_view_input(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Enter     => app.confirm(),
        KeyCode::Esc       => app.cancel(),
        KeyCode::Backspace => app.input_backspace(),
        KeyCode::Delete    => app.input_delete(),
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
        KeyCode::F(10)  => app.open_menu(),
        KeyCode::Delete => app.cat_delete(),
        _ => {}
    }
}

fn handle_menu(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Left       => app.menu_left(),
        KeyCode::Right      => app.menu_right(),
        KeyCode::Enter      => app.menu_enter(),
        KeyCode::Esc        => app.menu_esc(),
        KeyCode::Char(ch)   => app.menu_char(ch),
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

// ── Column form handler ───────────────────────────────────────────────────────

fn handle_col_form(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Enter     => app.col_form_confirm(),
        KeyCode::Esc       => app.col_form_cancel(),
        KeyCode::Up        => app.col_form_field_prev(),
        KeyCode::Down      => app.col_form_field_next(),
        KeyCode::Left      => app.col_form_cursor_left(),
        KeyCode::Right     => app.col_form_cursor_right(),
        KeyCode::Backspace => app.col_form_backspace(),
        KeyCode::Char(ch)  => app.col_form_input_char(ch),
        KeyCode::F(3)      => {
            if matches!(&app.col_mode,
                ColMode::Form { active_field: ColFormField::Head, .. } |
                ColMode::Form { active_field: ColFormField::Position, .. })
            {
                app.col_open_choices();
            }
        }
        _ => {}
    }
}

fn handle_col_choices(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Up    => app.col_choices_up(),
        KeyCode::Down  => app.col_choices_down(),
        KeyCode::Enter => app.col_choices_confirm(),
        KeyCode::Esc   => app.col_choices_cancel(),
        _ => {}
    }
}

// ── Column Properties handler ─────────────────────────────────────────────────

fn handle_col_props(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Enter     => app.col_props_confirm(),
        KeyCode::Esc       => app.col_props_cancel(),
        KeyCode::Up        => app.col_props_field_prev(),
        KeyCode::Down      => app.col_props_field_next(),
        KeyCode::Left      => app.col_props_left(),
        KeyCode::Right     => app.col_props_right(),
        KeyCode::Backspace => app.col_props_backspace(),
        KeyCode::Char(ch)  => app.col_props_input_char(ch),
        _ => {}
    }
}

// ── Column move handler ───────────────────────────────────────────────────────

fn handle_col_move(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Left         => app.col_move_left(),
        KeyCode::Right        => app.col_move_right(),
        KeyCode::Enter | KeyCode::Esc => { app.col_mode = ColMode::Normal; }
        _ => {}
    }
}

// ── Quick-add handler ─────────────────────────────────────────────────────────

fn handle_col_quick_add(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Up    => app.col_quick_add_up(),
        KeyCode::Down  => app.col_quick_add_down(),
        KeyCode::Enter => app.col_quick_add_confirm(),
        KeyCode::Esc   => app.col_quick_add_cancel(),
        _ => {}
    }
}

// ── Calendar handler ──────────────────────────────────────────────────────────

fn handle_col_calendar(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    match code {
        KeyCode::Up        => app.col_calendar_up(),
        KeyCode::Down      => app.col_calendar_down(),
        KeyCode::Left  if modifiers.contains(KeyModifiers::CONTROL) => app.col_calendar_year_prev(),
        KeyCode::Left      => app.col_calendar_left(),
        KeyCode::Right if modifiers.contains(KeyModifiers::CONTROL) => app.col_calendar_year_next(),
        KeyCode::Right     => app.col_calendar_right(),
        KeyCode::PageUp    => app.col_calendar_pgup(),
        KeyCode::PageDown  => app.col_calendar_pgdn(),
        KeyCode::Enter     => app.col_calendar_confirm(),
        KeyCode::Esc       => app.col_calendar_cancel(),
        KeyCode::F(6)      => app.col_open_set_time(),
        _ => {}
    }
}

// ── SetTime handler ───────────────────────────────────────────────────────────

fn handle_col_set_time(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Enter     => app.col_set_time_confirm(),
        KeyCode::Esc       => app.col_set_time_cancel(),
        KeyCode::Left      => app.col_set_time_left(),
        KeyCode::Right     => app.col_set_time_right(),
        KeyCode::Backspace => app.col_set_time_backspace(),
        KeyCode::Char(ch)  => app.col_set_time_input_char(ch),
        _ => {}
    }
}
