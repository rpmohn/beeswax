use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, ModifierKeyCode};
use crate::app::{App, AppScreen, AssignMode, CatMode, ColMode, ColFormField, ColPos, CursorPos, FKeyMod, MenuState, Mode, SectionInsert, SectionMode};

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

    // Assignment Profile takes priority
    if matches!(app.assign_mode, AssignMode::Profile { .. }) {
        handle_assign_profile(app, code);
        return;
    }

    // Section remove confirmation takes priority
    if matches!(app.sec_mode, SectionMode::ConfirmRemove { .. }) {
        handle_sec_confirm_remove(app, code);
        return;
    }

    // Section Add choices picker takes priority
    if matches!(app.sec_mode, SectionMode::Choices { .. }) {
        handle_sec_choices(app, code);
        return;
    }

    // Section Add form takes priority
    if matches!(app.sec_mode, SectionMode::Add { .. }) {
        handle_sec_form(app, code);
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

    // Remove-item confirmation takes priority
    if matches!(app.mode, Mode::ConfirmDeleteItem { .. }) {
        handle_item_confirm_delete(app, code);
        return;
    }

    // Remove-column confirmation takes priority
    if matches!(app.col_mode, ColMode::ConfirmRemove { .. }) {
        handle_col_confirm_remove(app, code);
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
            let in_props  = matches!(app.cat_state.mode, CatMode::Props { .. });
            let in_normal = matches!(app.cat_state.mode, CatMode::Normal);
            let in_edit   = matches!(app.cat_state.mode, CatMode::Edit   { .. });
            let in_create = matches!(app.cat_state.mode, CatMode::Create { .. });
            if      in_props  { handle_catmgr_props(app, code, modifiers) }
            else if in_normal { handle_catmgr_normal(app, code, modifiers) }
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
            KeyCode::Char('d') => app.sec_open_add(SectionInsert::Below),
            KeyCode::Char('u') => app.sec_open_add(SectionInsert::Above),
            KeyCode::F(4)      => app.item_remove(),   // discard without confirmation
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
        KeyCode::F(3)   => {
            if app.col_cursor == 0 { app.assign_open(); }
            else                   { app.col_open_calendar(); }
        }
        KeyCode::F(5)   => app.open_note(),
        KeyCode::F(6)   => app.col_open_props(),
        KeyCode::Delete => {
            if app.col_cursor == 0 {
                match app.cursor {
                    CursorPos::SectionHead(_) => app.sec_open_confirm_remove(),
                    CursorPos::Item { .. }    => app.item_open_confirm_delete(),
                }
            } else {
                app.col_open_confirm_remove();
            }
        }
        KeyCode::F(9)   => app.toggle_catmgr(),
        KeyCode::F(10)  => app.open_menu(),
        KeyCode::Char(ch) if !modifiers.contains(KeyModifiers::CONTROL)
                          && !modifiers.contains(KeyModifiers::ALT) => app.begin_char_input(ch),
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
    // Search mode: search keys handled here; any other key clears search and falls through.
    if app.cat_search.is_some() {
        match code {
            KeyCode::Esc       => { app.cat_search_clear(); return; }
            KeyCode::Enter     => { app.cat_search_clear(); return; }
            KeyCode::Backspace => { app.cat_search_backspace(); return; }
            KeyCode::F(7)      => { app.cat_search_prev(); return; }
            KeyCode::F(8)      => { app.cat_search_next(); return; }
            KeyCode::Char(ch) if !modifiers.contains(KeyModifiers::CONTROL)
                              && !modifiers.contains(KeyModifiers::ALT) => {
                app.cat_search_char(ch);
                return;
            }
            _ => { app.cat_search_clear(); }  // navigation key: cancel search, then handle
        }
    }
    match code {
        KeyCode::Up       => app.cat_cursor_up(),
        KeyCode::Down     => app.cat_cursor_down(),
        KeyCode::PageUp   => app.cat_cursor_pgup(10),
        KeyCode::PageDown => app.cat_cursor_pgdn(10),
        KeyCode::Home     => app.cat_cursor_home(),
        KeyCode::End      => app.cat_cursor_end(),
        KeyCode::Insert   => app.cat_begin_create(false),
        KeyCode::F(2) | KeyCode::Enter => app.cat_begin_edit(),
        KeyCode::F(5)   => app.open_note(),
        KeyCode::F(6)   => app.cat_open_props(),
        KeyCode::F(7)   => app.cat_promote(),
        KeyCode::F(8)   => app.cat_demote(),
        KeyCode::Esc | KeyCode::F(9) => app.toggle_catmgr(),
        KeyCode::F(10)  => app.open_menu(),
        KeyCode::Delete => app.cat_delete(),
        KeyCode::Char(ch) if !modifiers.contains(KeyModifiers::CONTROL)
                          && !modifiers.contains(KeyModifiers::ALT) => {
            app.cat_search_char(ch);
        }
        _ => {}
    }
}

fn handle_catmgr_props(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    match code {
        KeyCode::Enter     => app.cat_props_confirm(),
        KeyCode::Esc       => app.cat_props_cancel(),
        // F2 Edit / F3 Choices open the note editor when Note field is active
        KeyCode::F(2) | KeyCode::F(3) => app.cat_props_open_editor(),
        // Shift+Tab: BackTab (most terminals) or Tab+SHIFT (some terminals)
        KeyCode::Tab if modifiers.contains(KeyModifiers::SHIFT) => app.cat_props_field_prev(),
        KeyCode::Tab       | KeyCode::Down   => app.cat_props_field_next(),
        KeyCode::BackTab   | KeyCode::Up     => app.cat_props_field_prev(),
        KeyCode::Left      => app.cat_props_cursor_left(),
        KeyCode::Right     => app.cat_props_cursor_right(),
        // Both Backspace and Delete perform backward deletion (cursor starts at end of text)
        KeyCode::Backspace | KeyCode::Delete => app.cat_props_backspace(),
        KeyCode::Char(ch)  => app.cat_props_input_char(ch),
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
        KeyCode::Delete    => app.cat_input_delete(),
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
        KeyCode::PageUp   if modifiers.contains(KeyModifiers::CONTROL) => app.col_calendar_year_prev(),
        KeyCode::PageDown if modifiers.contains(KeyModifiers::CONTROL) => app.col_calendar_year_next(),
        KeyCode::PageUp    => app.col_calendar_pgup(),
        KeyCode::PageDown  => app.col_calendar_pgdn(),
        // < / > as reliable year nav (Ctrl+PgUp/Dn often intercepted by terminal)
        KeyCode::Char('<') => app.col_calendar_year_prev(),
        KeyCode::Char('>') => app.col_calendar_year_next(),
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

// ── Remove-item confirmation handler ─────────────────────────────────────────

fn handle_item_confirm_delete(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Enter                 => app.item_confirm_delete_confirm(),
        KeyCode::Esc                   => app.item_confirm_delete_cancel(),
        KeyCode::Left | KeyCode::Right => app.item_confirm_delete_toggle(),
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            if let Mode::ConfirmDeleteItem { yes } = &mut app.mode { *yes = true; }
            app.item_confirm_delete_confirm();
        }
        KeyCode::Char('n') | KeyCode::Char('N') => app.item_confirm_delete_cancel(),
        _ => {}
    }
}

// ── Remove-column confirmation handler ───────────────────────────────────────

fn handle_col_confirm_remove(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Enter                 => app.col_confirm_remove_confirm(),
        KeyCode::Esc                   => app.col_confirm_remove_cancel(),
        KeyCode::Left | KeyCode::Right => app.col_confirm_remove_toggle(),
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            if let crate::app::ColMode::ConfirmRemove { yes } = &mut app.col_mode { *yes = true; }
            app.col_confirm_remove_confirm();
        }
        KeyCode::Char('n') | KeyCode::Char('N') => app.col_confirm_remove_cancel(),
        _ => {}
    }
}

// ── Section remove confirmation handler ────────────────────────────────────────

fn handle_sec_confirm_remove(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Left | KeyCode::Right => app.sec_confirm_remove_toggle(),
        KeyCode::Enter                 => app.sec_confirm_remove_confirm(),
        KeyCode::Esc                   => app.sec_confirm_remove_cancel(),
        _ => {}
    }
}

// ── Section Add form handler ───────────────────────────────────────────────────

fn handle_sec_form(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Enter => app.sec_form_confirm(),
        KeyCode::Esc   => app.sec_form_cancel(),
        KeyCode::Up    => app.sec_form_field_prev(),
        KeyCode::Down  => app.sec_form_field_next(),
        KeyCode::Left  => app.sec_form_left(),
        KeyCode::Right => app.sec_form_right(),
        KeyCode::F(3)  => app.sec_open_choices(),
        _ => {}
    }
}

fn handle_sec_choices(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Up    => app.sec_choices_up(),
        KeyCode::Down  => app.sec_choices_down(),
        KeyCode::Enter => app.sec_choices_confirm(),
        KeyCode::Esc   => app.sec_choices_cancel(),
        _ => {}
    }
}

// ── Assignment Profile handler ────────────────────────────────────────────────

fn handle_assign_profile(app: &mut App, code: KeyCode) {
    // Search mode: search keys handled here; any other key clears search and falls through.
    if app.cat_search.is_some() {
        match code {
            KeyCode::Esc       => { app.cat_search_clear(); return; }
            KeyCode::Enter     => { app.cat_search_clear(); return; }
            KeyCode::Backspace => { app.cat_search_backspace(); return; }
            KeyCode::F(7)      => { app.cat_search_prev(); return; }
            KeyCode::F(8)      => { app.cat_search_next(); return; }
            KeyCode::Char(ch) if ch != ' ' => { app.cat_search_char(ch); return; }
            _ => { app.cat_search_clear(); }  // navigation key: cancel search, then handle
        }
    }
    match code {
        KeyCode::Up       => app.assign_cursor_up(),
        KeyCode::Down     => app.assign_cursor_down(),
        KeyCode::PageUp   => app.assign_cursor_pgup(10),
        KeyCode::PageDown => app.assign_cursor_pgdn(10),
        KeyCode::Home     => app.assign_cursor_home(),
        KeyCode::End      => app.assign_cursor_end(),
        KeyCode::Char(' ')     => app.assign_toggle(),
        KeyCode::Enter | KeyCode::Esc | KeyCode::F(3) => app.assign_close(),
        KeyCode::F(7)     => app.cat_search_prev(),
        KeyCode::F(8)     => app.cat_search_next(),
        KeyCode::Backspace => app.cat_search_backspace(),
        KeyCode::Char(ch) if ch != ' ' => app.cat_search_char(ch),
        _ => {}
    }
}
