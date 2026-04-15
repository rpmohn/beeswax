use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, ModifierKeyCode};
use crate::app::{App, AppScreen, AskChoice, AssignMode, CatMode, ColMode, ColFormField, ColPos, CursorPos, FilterState, FKeyMod, MenuState, Mode, NavMode, PasswordPurpose, SaveState, SecPropsField, SectionInsert, SectionMode, SortState, ViewMgrMode, ViewMode, ViewPropsField};

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

    // Alt-Q — trigger quit (may show ask-save dialog)
    if modifiers.contains(KeyModifiers::ALT) && code == KeyCode::Char('q') {
        app.trigger_quit();
        return;
    }

    // Ctrl-S — save
    if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('s') {
        let _ = app.save();
        return;
    }

    // Ask-save dialog takes priority
    if matches!(app.save_state, SaveState::AskOnQuit { .. }) {
        handle_ask_save(app, code);
        return;
    }

    // Password-entry dialog takes priority
    if matches!(app.save_state, SaveState::PasswordEntry { .. }) {
        handle_password_entry(app, code);
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

    // Section Properties dialog (and sort sub-dialogs) take priority
    if matches!(app.sec_mode, SectionMode::Props { .. }) {
        handle_sec_props(app, code);
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

    // Sub-category picker (F3 on standard column) takes priority
    if matches!(app.col_mode, ColMode::SubPick { .. }) {
        handle_col_sub_pick(app, code, modifiers);
        return;
    }

    // Item search bar takes priority when active
    if app.item_search.is_some() {
        match code {
            KeyCode::Enter     => app.search_confirm(),
            KeyCode::Esc       => app.search_cancel(),
            KeyCode::Backspace => app.search_backspace(),
            KeyCode::Left      => app.search_cursor_left(),
            KeyCode::Right     => app.search_cursor_right(),
            KeyCode::Char(ch)  => app.search_char(ch),
            _ => {}
        }
        return;
    }

    // Item Properties modal takes priority
    if matches!(app.mode, Mode::ItemProps { .. }) {
        handle_item_props(app, code);
        return;
    }

    // Remove-item confirmation takes priority
    if matches!(app.mode, Mode::ConfirmDeleteItem { .. }) {
        handle_item_confirm_delete(app, code);
        return;
    }

    // Discard-item confirmation takes priority
    if matches!(app.mode, Mode::ConfirmDiscardItem { .. }) {
        handle_item_confirm_discard(app, code);
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

    // View Add dialog takes priority
    if matches!(app.view_mode, ViewMode::Add { .. } | ViewMode::AddPick { .. }) {
        handle_view_add(app, code);
        return;
    }

    // View Properties dialog can appear over any screen
    if matches!(app.vmgr_state.mode, ViewMgrMode::Props { .. }) {
        handle_vmgr_props(app, code);
        return;
    }

    // View Manager screen — handled here to avoid catch-all dirty marking for navigation
    if matches!(app.screen, AppScreen::ViewMgr) {
        handle_vmgr(app, code, modifiers);
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
            let in_move   = matches!(app.cat_state.mode, CatMode::Move);
            let in_normal = matches!(app.cat_state.mode, CatMode::Normal);
            let in_edit   = matches!(app.cat_state.mode, CatMode::Edit   { .. });
            let in_create = matches!(app.cat_state.mode, CatMode::Create { .. });
            if      in_props  { handle_catmgr_props(app, code, modifiers) }
            else if in_move   { handle_catmgr_move(app, code) }
            else if in_normal { handle_catmgr_normal(app, code, modifiers) }
            else if in_edit   { handle_catmgr_input(app, code) }
            else if in_create { handle_catmgr_input(app, code) }
        }
        AppScreen::ViewMgr => {} // handled above as priority block
    }
    // Mark dirty after any key event that reached the main handlers.
    // (Over-marks navigation keys, which is acceptable per spec.)
    if app.file_path.is_some() {
        app.dirty = true;
    }
}

// ── Ask-save dialog handler ───────────────────────────────────────────────────

fn handle_ask_save(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Left  => app.ask_save_move_left(),
        KeyCode::Right => app.ask_save_move_right(),
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            app.ask_save_set_choice(AskChoice::Yes);
            app.ask_save_confirm();
        }
        KeyCode::Char('n') | KeyCode::Char('N') => {
            app.ask_save_set_choice(AskChoice::No);
            app.ask_save_no();
        }
        KeyCode::Char('c') | KeyCode::Char('C') | KeyCode::Esc => {
            app.ask_save_cancel();
        }
        KeyCode::Enter => {
            let choice = if let SaveState::AskOnQuit { choice } = &app.save_state {
                *choice
            } else {
                return;
            };
            match choice {
                AskChoice::Yes    => app.ask_save_confirm(),
                AskChoice::No     => app.ask_save_no(),
                AskChoice::Cancel => app.ask_save_cancel(),
            }
        }
        _ => {}
    }
}

// ── Password-entry dialog handler ─────────────────────────────────────────────

fn handle_password_entry(app: &mut App, code: KeyCode) {
    // For Disable purpose, skip confirm — treat Enter as confirm directly.
    let is_disable = matches!(
        app.save_state,
        SaveState::PasswordEntry { purpose: PasswordPurpose::Disable, .. }
    );
    match code {
        KeyCode::Esc   => app.password_entry_cancel(),
        KeyCode::Enter => { app.password_entry_confirm(); }
        KeyCode::Tab   => {
            if !is_disable { app.password_entry_tab(); }
        }
        KeyCode::Backspace => app.password_entry_backspace(),
        KeyCode::Char(c)   => app.password_entry_char(c),
        _ => {}
    }
}

// ── View handlers ─────────────────────────────────────────────────────────────

fn handle_view_normal(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    // In vi mode, non-char keys can complete a pending sequence.
    if app.nav_mode == NavMode::Vi {
        if let Some(pending) = app.vi_pending {
            match (pending, code) {
                ('z', KeyCode::Enter) => {
                    app.vi_pending = None;
                    app.scroll_to_top();
                    return;
                }
                (_, KeyCode::Char(_)) => {
                    // Char keys reach handle_view_normal_vi via the Char arm below — do nothing here.
                }
                _ => {
                    // Unknown sequence: discard pending, fall through to process the key normally.
                    app.vi_pending = None;
                }
            }
        }
    }

    if modifiers.contains(KeyModifiers::CONTROL) {
        match code {
            KeyCode::Char('f') | KeyCode::Char('F') => {
                app.cursor_pgdn(app.body_height.get().max(1));
                return;
            }
            KeyCode::Char('b') | KeyCode::Char('B') => {
                app.cursor_pgup(app.body_height.get().max(1));
                return;
            }
            _ => {}
        }
    }
    if modifiers.contains(KeyModifiers::ALT) {
        match code {
            KeyCode::Char('r') => app.col_quick_add(ColPos::Right),
            KeyCode::Char('l') => app.col_quick_add(ColPos::Left),
            KeyCode::Char('d') => app.sec_open_add(SectionInsert::Below),
            KeyCode::Char('u') => app.sec_open_add(SectionInsert::Above),
            KeyCode::Char('s') => app.sec_sort_now(),
            KeyCode::F(4)      => app.item_remove(),   // discard without confirmation
            _ => {}
        }
        return;
    }
    match code {
        KeyCode::Up       => app.cursor_up(),
        KeyCode::Down     => app.cursor_down(),
        KeyCode::PageUp   => app.cursor_pgup(10),
        KeyCode::PageDown => app.cursor_pgdn(10),
        KeyCode::Home     => app.cursor_home(),
        KeyCode::End      => app.cursor_end(),
        KeyCode::Left  | KeyCode::BackTab => app.cursor_col_left(),
        KeyCode::Right | KeyCode::Tab    => app.cursor_col_right(),
        KeyCode::Enter  => app.cursor_down(),
        KeyCode::Char('/') => app.search_open(),
        KeyCode::Insert => app.begin_create_blank(),
        KeyCode::F(2)   => app.begin_edit(),
        KeyCode::F(4)   => app.item_mark_done(),
        KeyCode::F(3)   => {
            if app.col_cursor == 0 {
                app.assign_open();
            } else {
                let is_date = app.view.columns.get(app.col_cursor - 1)
                    .map(|c| c.date_fmt.is_some())
                    .unwrap_or(false);
                if is_date { app.col_open_calendar(); }
                else       { app.col_open_sub_pick(); }
            }
        }
        KeyCode::F(5)   => app.open_note(),
        KeyCode::F(6)   => {
            if app.col_cursor == 0 {
                match app.cursor {
                    CursorPos::SectionHead(_) => app.sec_open_props(),
                    CursorPos::Item { .. }    => app.item_open_props(),
                }
            } else {
                app.col_open_props();
            }
        }
        KeyCode::Delete => {
            if modifiers.contains(KeyModifiers::SHIFT) {
                if app.col_cursor == 0 {
                    if matches!(app.cursor, CursorPos::Item { .. }) {
                        app.item_open_confirm_discard();
                    }
                }
            } else if app.col_cursor == 0 {
                match app.cursor {
                    CursorPos::SectionHead(_) => app.sec_open_confirm_remove(),
                    CursorPos::Item { .. }    => app.item_open_confirm_delete(),
                }
            } else {
                app.col_open_confirm_remove();
            }
        }
        KeyCode::F(8)   => app.open_view_mgr(),
        KeyCode::F(9)   => app.toggle_catmgr(),
        KeyCode::F(10)  => app.open_menu(),
        KeyCode::Char(ch) if !modifiers.contains(KeyModifiers::CONTROL)
                          && !modifiers.contains(KeyModifiers::ALT) => {
            match app.nav_mode {
                NavMode::Vi => handle_view_normal_vi(app, ch),
                NavMode::Agenda => app.begin_char_input(ch),
            }
        }
        _ => {}
    }
}

/// Vi-mode character handler for Normal mode.
/// Maps hjkl, i, o, O, zz; ignores all other printable keys.
fn handle_view_normal_vi(app: &mut App, ch: char) {
    // Resolve pending two-key sequences first.
    if let Some(pending) = app.vi_pending.take() {
        match (pending, ch) {
            ('z', 'z') | ('z', '.') => app.scroll_center(),
            ('z', '-')              => app.scroll_to_bottom(),
            ('g', 'g')              => app.cursor_first(),
            _ => handle_view_normal_vi(app, ch),  // discard first key, process second
        }
        return;
    }

    match ch {
        'j' => app.cursor_down(),
        'k' => app.cursor_up(),
        'h' => app.cursor_col_left(),
        'l' => app.cursor_col_right(),
        'H' => app.cursor_screen_top(),
        'M' => app.cursor_screen_middle(),
        'L' => app.cursor_screen_bottom(),
        '{' => app.cursor_home(),
        '}' => app.cursor_end(),
        'G' => app.cursor_last(),
        'i' => app.begin_edit(),
        'o' => app.begin_create_blank(),
        'O' => app.begin_create_above(),
        'g' => app.vi_pending = Some('g'),
        'z' => app.vi_pending = Some('z'),
        _   => {}   // all other printable keys are no-ops in vi normal mode
    }
}

fn handle_view_input(app: &mut App, code: KeyCode) {
    // For item text col=0 (Edit or Create), Up/Down navigate wrapped lines.
    let is_item_text = matches!(&app.mode,
        Mode::Edit   { col, .. } if *col == 0) ||
        matches!(&app.mode, Mode::Create { .. });
    match code {
        KeyCode::Enter     => app.confirm(),
        KeyCode::Esc       => app.cancel(),
        KeyCode::Backspace => app.input_backspace(),
        KeyCode::Delete    => app.input_delete(),
        KeyCode::Left      => app.edit_cursor_left(),
        KeyCode::Right     => app.edit_cursor_right(),
        KeyCode::Home      => app.edit_cursor_home(),
        KeyCode::End       => app.edit_cursor_end(),
        KeyCode::Up   if is_item_text => app.edit_cursor_line_up(),
        KeyCode::Down if is_item_text => app.edit_cursor_line_down(),
        KeyCode::Char(ch)  => app.input_char(ch),
        _ => {}
    }
}

// ── Vi list-navigation helper ─────────────────────────────────────────────────

/// Handles vi navigation keys (j/k/G/gg/H/M/L) for list-style pickers.
/// Returns `true` if `code` was consumed so the caller can return early.
/// No-ops and returns `false` when nav_mode != Vi.
/// Always clears `vi_pending` so stale state doesn't leak across handlers.
fn handle_vi_list(
    app:  &mut App,
    code: KeyCode,
    down: &dyn Fn(&mut App),
    up:   &dyn Fn(&mut App),
    end:  &dyn Fn(&mut App),
    home: &dyn Fn(&mut App),
    mid:  &dyn Fn(&mut App),
) -> bool {
    if app.nav_mode != NavMode::Vi { return false; }
    let pending = app.vi_pending.take();   // always clear stale state
    if let Some('g') = pending {
        if code == KeyCode::Char('g') { home(app); return true; }
        // Unknown two-key sequence — fall through with pending cleared.
    }
    match code {
        KeyCode::Char('j') => { down(app); true }
        KeyCode::Char('k') => { up(app);   true }
        KeyCode::Char('G') => { end(app);  true }
        KeyCode::Char('g') => { app.vi_pending = Some('g'); true }
        KeyCode::Char('H') => { home(app); true }
        KeyCode::Char('M') => { mid(app);  true }
        KeyCode::Char('L') => { end(app);  true }
        _ => false
    }
}

// ── CatMgr handlers ───────────────────────────────────────────────────────────

fn handle_catmgr_normal(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    if modifiers.contains(KeyModifiers::ALT) {
        match code {
            KeyCode::Char('r') => app.cat_begin_create(true),   // child
            KeyCode::F(10)     => app.cat_begin_move(),
            _ => {}
        }
        return;
    }
    if modifiers.contains(KeyModifiers::CONTROL) {
        match code {
            // Standard terminals: Ctrl+Arrow with CONTROL modifier
            KeyCode::Up    => { app.cat_move_up();   return; }
            KeyCode::Down  => { app.cat_move_down(); return; }
            KeyCode::Left  => { app.cat_promote();   return; }
            KeyCode::Right => { app.cat_demote();    return; }
            // CSI-u terminals: Ctrl+Arrow encoded as Ctrl+U/D/L/R (codepoints 85/68/76/82)
            KeyCode::Char('u') | KeyCode::Char('U') => { app.cat_move_up();   return; }
            KeyCode::Char('d') | KeyCode::Char('D') => { app.cat_move_down(); return; }
            KeyCode::Char('l') | KeyCode::Char('L') => { app.cat_promote();   return; }
            KeyCode::Char('r') | KeyCode::Char('R') => { app.cat_demote();    return; }
            _ => {}
        }
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
    if handle_vi_list(app, code,
        &|a| a.cat_cursor_down(), &|a| a.cat_cursor_up(),
        &|a| a.cat_cursor_end(), &|a| a.cat_cursor_home(),
        &|a| a.cat_cursor_middle()) { return; }
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
        KeyCode::Esc | KeyCode::F(9) => app.toggle_catmgr(),
        KeyCode::F(10)  => app.open_menu(),
        KeyCode::Delete => app.cat_delete(),
        KeyCode::Char(ch) if !modifiers.contains(KeyModifiers::CONTROL)
                          && !modifiers.contains(KeyModifiers::ALT) => {
            match app.nav_mode {
                NavMode::Vi => match ch {
                    'i' => app.cat_begin_edit(),
                    'o' => app.cat_begin_create(false),
                    '/' => app.cat_search_open(),
                    _   => {}
                },
                NavMode::Agenda => app.cat_search_char(ch),
            }
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

fn handle_catmgr_move(app: &mut App, code: KeyCode) {
    if app.nav_mode == NavMode::Vi {
        app.vi_pending = None;   // clear any stale pending
        match code {
            KeyCode::Char('j') => { app.cat_move_down(); return; }
            KeyCode::Char('k') => { app.cat_move_up();   return; }
            _ => {}
        }
    }
    match code {
        KeyCode::Up           => app.cat_move_up(),
        KeyCode::Down         => app.cat_move_down(),
        KeyCode::Enter | KeyCode::Esc | KeyCode::F(10) => app.cat_move_confirm(),
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
    if handle_vi_list(app, code,
        &|a| a.col_choices_down(), &|a| a.col_choices_up(),
        &|a| a.col_choices_end(), &|a| a.col_choices_home(),
        &|a| a.col_choices_middle()) { return; }
    match code {
        KeyCode::Up       => app.col_choices_up(),
        KeyCode::Down     => app.col_choices_down(),
        KeyCode::PageUp   => app.col_choices_pgup(10),
        KeyCode::PageDown => app.col_choices_pgdn(10),
        KeyCode::Home     => app.col_choices_home(),
        KeyCode::End      => app.col_choices_end(),
        KeyCode::Enter    => app.col_choices_confirm(),
        KeyCode::Esc      => app.col_choices_cancel(),
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
    // Delete confirmation takes priority.
    if let ColMode::QuickAdd { confirm_delete, .. } = &app.col_mode {
        if *confirm_delete {
            match code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => app.col_quick_add_delete_confirm(),
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc   => app.col_quick_add_delete_cancel(),
                _ => {}
            }
            return;
        }
    }
    // Props modal active: route to its handler.
    if matches!(app.cat_state.mode, CatMode::Props { .. }) {
        handle_catmgr_props(app, code, KeyModifiers::NONE);
        return;
    }
    // While a create/edit buffer is active, route typing to the catmgr input handler.
    if !matches!(app.cat_state.mode, CatMode::Normal) {
        handle_catmgr_input(app, code);
        return;
    }
    // Search mode: search keys here; any other key clears search and falls through.
    if app.cat_search.is_some() {
        match code {
            KeyCode::Esc | KeyCode::Enter => { app.cat_search_clear(); return; }
            KeyCode::Backspace => { app.cat_search_backspace(); return; }
            KeyCode::F(7)      => { app.cat_search_prev(); return; }
            KeyCode::F(8)      => { app.cat_search_next(); return; }
            KeyCode::Char(ch) if ch != ' ' => { app.cat_search_char(ch); return; }
            _ => { app.cat_search_clear(); }
        }
    }
    if handle_vi_list(app, code,
        &|a| a.col_quick_add_down(), &|a| a.col_quick_add_up(),
        &|a| a.col_quick_add_end(), &|a| a.col_quick_add_home(),
        &|a| a.col_quick_add_middle()) { return; }
    match code {
        KeyCode::Up       => app.col_quick_add_up(),
        KeyCode::Down     => app.col_quick_add_down(),
        KeyCode::PageUp   => app.col_quick_add_pgup(10),
        KeyCode::PageDown => app.col_quick_add_pgdn(10),
        KeyCode::Home     => app.col_quick_add_home(),
        KeyCode::End      => app.col_quick_add_end(),
        KeyCode::Enter    => app.col_quick_add_confirm(),
        KeyCode::Esc      => app.col_quick_add_cancel(),
        KeyCode::Insert   => {
            // Sync cat_state.cursor so cat_confirm inserts at picker position.
            if let ColMode::QuickAdd { picker_cursor, .. } = &app.col_mode {
                app.cat_state.cursor = *picker_cursor;
            }
            app.cat_begin_create(false);
        }
        KeyCode::Delete   => app.col_quick_add_begin_delete(),
        KeyCode::F(2)     => {
            if let ColMode::QuickAdd { picker_cursor, .. } = &app.col_mode {
                app.cat_state.cursor = *picker_cursor;
            }
            app.cat_begin_edit();
        }
        KeyCode::F(6)     => {
            if let ColMode::QuickAdd { picker_cursor, .. } = &app.col_mode {
                app.cat_state.cursor = *picker_cursor;
            }
            app.cat_open_props();
        }
        KeyCode::Char(ch) if ch != ' ' => app.cat_search_char(ch),
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
        // CSI-u terminals encode Ctrl+Arrow as Ctrl+letter (L=Left, R=Right)
        KeyCode::Char('l') | KeyCode::Char('L')
            if modifiers.contains(KeyModifiers::CONTROL) => app.col_calendar_year_prev(),
        KeyCode::Char('r') | KeyCode::Char('R')
            if modifiers.contains(KeyModifiers::CONTROL) => app.col_calendar_year_next(),
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

// ── Item Properties modal handler ────────────────────────────────────────────

fn handle_item_props(app: &mut App, code: KeyCode) {
    // When in-place text editing is active, route to text-edit sub-handler.
    let editing = matches!(&app.mode, Mode::ItemProps { edit_buf: Some(_), .. });
    if editing {
        match code {
            KeyCode::Enter                  => app.item_props_text_confirm(),
            KeyCode::Esc                    => app.item_props_cancel(),
            KeyCode::Left                   => app.item_props_text_cursor_left(),
            KeyCode::Right                  => app.item_props_text_cursor_right(),
            KeyCode::Backspace              => app.item_props_text_backspace(),
            KeyCode::Delete                 => app.item_props_text_delete(),
            KeyCode::Char(ch)               => app.item_props_text_input_char(ch),
            _ => {}
        }
        return;
    }
    if handle_vi_list(app, code,
        &|a| a.item_props_cursor_down(), &|a| a.item_props_cursor_up(),
        &|a| a.item_props_cursor_end(), &|a| a.item_props_cursor_home(),
        &|a| a.item_props_cursor_middle()) { return; }
    match code {
        KeyCode::Esc                        => app.item_props_cancel(),
        KeyCode::Enter | KeyCode::F(2)      => app.item_props_edit(),
        KeyCode::F(3)                       => app.item_props_choices(),
        KeyCode::Up                         => app.item_props_cursor_up(),
        KeyCode::Down                       => app.item_props_cursor_down(),
        KeyCode::Home                       => app.item_props_cursor_home(),
        KeyCode::End                        => app.item_props_cursor_end(),
        KeyCode::PageUp                     => app.item_props_cursor_pgup(10),
        KeyCode::PageDown                   => app.item_props_cursor_pgdn(10),
        KeyCode::Delete                     => app.item_props_remove(),
        KeyCode::Char('i') if app.nav_mode == NavMode::Vi => app.item_props_edit(),
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

fn handle_item_confirm_discard(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Enter                 => app.item_confirm_discard_confirm(),
        KeyCode::Esc                   => app.item_confirm_discard_cancel(),
        KeyCode::Left | KeyCode::Right => app.item_confirm_discard_toggle(),
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            if let Mode::ConfirmDiscardItem { yes } = &mut app.mode { *yes = true; }
            app.item_confirm_discard_confirm();
        }
        KeyCode::Char('n') | KeyCode::Char('N') => app.item_confirm_discard_cancel(),
        _ => {}
    }
}

// ── Sub-category picker handler ───────────────────────────────────────────────

fn handle_col_sub_pick(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    // If a cat inline edit/create is in progress, route text input there.
    if matches!(app.cat_state.mode, CatMode::Edit { .. } | CatMode::Create { .. }) {
        handle_catmgr_input(app, code);
        return;
    }
    // If cat props dialog is open, route to its handler.
    if matches!(app.cat_state.mode, CatMode::Props { .. }) {
        handle_catmgr_props(app, code, modifiers);
        return;
    }
    if handle_vi_list(app, code,
        &|a| a.col_sub_pick_down(), &|a| a.col_sub_pick_up(),
        &|a| a.col_sub_pick_end(), &|a| a.col_sub_pick_home(),
        &|a| a.col_sub_pick_middle()) { return; }
    match code {
        KeyCode::Up              => app.col_sub_pick_up(),
        KeyCode::Down            => app.col_sub_pick_down(),
        KeyCode::PageUp          => app.col_sub_pick_pgup(10),
        KeyCode::PageDown        => app.col_sub_pick_pgdn(10),
        KeyCode::Home            => app.col_sub_pick_home(),
        KeyCode::End             => app.col_sub_pick_end(),
        KeyCode::Char(' ')       => app.col_sub_pick_toggle(),
        KeyCode::F(2)            => app.col_sub_pick_begin_edit(),
        KeyCode::F(6)            => app.col_sub_pick_open_props(),
        KeyCode::Insert          => app.col_sub_pick_begin_create(),
        KeyCode::Enter | KeyCode::Esc | KeyCode::F(3) => app.col_sub_pick_close(),
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
    if handle_vi_list(app, code,
        &|a| a.sec_choices_down(), &|a| a.sec_choices_up(),
        &|a| a.sec_choices_end(), &|a| a.sec_choices_home(),
        &|a| a.sec_choices_middle()) { return; }
    match code {
        KeyCode::Up       => app.sec_choices_up(),
        KeyCode::Down     => app.sec_choices_down(),
        KeyCode::PageUp   => app.sec_choices_pgup(10),
        KeyCode::PageDown => app.sec_choices_pgdn(10),
        KeyCode::Home     => app.sec_choices_home(),
        KeyCode::End      => app.sec_choices_end(),
        KeyCode::Enter    => app.sec_choices_confirm(),
        KeyCode::Esc      => app.sec_choices_cancel(),
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
    if handle_vi_list(app, code,
        &|a| a.assign_cursor_down(), &|a| a.assign_cursor_up(),
        &|a| a.assign_cursor_end(), &|a| a.assign_cursor_home(),
        &|a| a.assign_cursor_middle()) { return; }
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

// ── View Add dialog ───────────────────────────────────────────────────────────

fn handle_view_add(app: &mut App, code: KeyCode) {
    // Picker sub-mode
    if matches!(app.view_mode, ViewMode::AddPick { .. }) {
        if handle_vi_list(app, code,
            &|a| a.view_add_pick_down(), &|a| a.view_add_pick_up(),
            &|a| a.view_add_pick_end(), &|a| a.view_add_pick_home(),
            &|a| a.view_add_pick_middle()) { return; }
        match code {
            KeyCode::Up       => app.view_add_pick_up(),
            KeyCode::Down     => app.view_add_pick_down(),
            KeyCode::PageUp   => app.view_add_pick_pgup(10),
            KeyCode::PageDown => app.view_add_pick_pgdn(10),
            KeyCode::Home     => app.view_add_pick_home(),
            KeyCode::End      => app.view_add_pick_end(),
            KeyCode::Enter    => app.view_add_pick_confirm(),
            KeyCode::Esc      => app.view_add_pick_cancel(),
            _ => {}
        }
        return;
    }
    // Main dialog
    match code {
        KeyCode::Enter                        => app.view_add_confirm(),
        KeyCode::Esc                          => app.view_add_cancel(),
        KeyCode::Tab | KeyCode::Down          => app.view_add_tab(),
        KeyCode::BackTab | KeyCode::Up        => app.view_add_tab(),
        KeyCode::Left                         => app.view_add_cursor_left(),
        KeyCode::Right                        => app.view_add_cursor_right(),
        KeyCode::Backspace                    => app.view_add_backspace(),
        KeyCode::F(3)                         => app.view_add_open_pick(),
        KeyCode::Char(ch)                     => app.view_add_char(ch),
        _ => {}
    }
}

// ── View Manager handlers ─────────────────────────────────────────────────────

fn handle_vmgr(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    match &app.vmgr_state.mode {
        ViewMgrMode::Rename { .. }        => handle_vmgr_rename(app, code),
        ViewMgrMode::ConfirmDelete { .. } => handle_vmgr_delete(app, code),
        ViewMgrMode::Props { .. }         => handle_vmgr_props(app, code),
        ViewMgrMode::Normal               => handle_vmgr_normal(app, code, modifiers),
    }
}

fn handle_vmgr_normal(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    if modifiers.contains(KeyModifiers::CONTROL) {
        match code {
            KeyCode::Up    => { app.vmgr_move_up();   return; }
            KeyCode::Down  => { app.vmgr_move_down(); return; }
            // CSI-u: Ctrl+Up/Down encoded as Ctrl+U/D
            KeyCode::Char('u') | KeyCode::Char('U') => { app.vmgr_move_up();   return; }
            KeyCode::Char('d') | KeyCode::Char('D') => { app.vmgr_move_down(); return; }
            _ => {}
        }
    }
    if handle_vi_list(app, code,
        &|a| a.vmgr_cursor_down(), &|a| a.vmgr_cursor_up(),
        &|a| a.vmgr_cursor_end(), &|a| a.vmgr_cursor_home(),
        &|a| a.vmgr_cursor_middle()) { return; }
    match code {
        KeyCode::Up       => app.vmgr_cursor_up(),
        KeyCode::Down     => app.vmgr_cursor_down(),
        KeyCode::PageUp   => app.vmgr_cursor_pgup(10),
        KeyCode::PageDown => app.vmgr_cursor_pgdn(10),
        KeyCode::Home     => app.vmgr_cursor_home(),
        KeyCode::End      => app.vmgr_cursor_end(),
        KeyCode::Enter    => app.vmgr_select(),
        KeyCode::Insert   => app.view_add_open(),
        KeyCode::Delete   => app.vmgr_open_confirm_delete(),
        KeyCode::F(2)  => app.vmgr_begin_rename(),
        KeyCode::F(4)  => app.vmgr_open_confirm_delete(),
        KeyCode::F(6)  => app.vmgr_begin_props(),
        KeyCode::F(8) | KeyCode::Esc => app.close_view_mgr(),
        KeyCode::F(9)  => { app.close_view_mgr(); app.toggle_catmgr(); }
        KeyCode::F(10) => app.open_menu(),
        KeyCode::Char('i') if app.nav_mode == NavMode::Vi => app.vmgr_begin_rename(),
        KeyCode::Char('o') | KeyCode::Char('O') if app.nav_mode == NavMode::Vi => app.view_add_open(),
        _ => {}
    }
}

fn handle_vmgr_rename(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Enter     => app.vmgr_rename_confirm(),
        KeyCode::Esc       => app.vmgr_rename_cancel(),
        KeyCode::Left      => app.vmgr_rename_left(),
        KeyCode::Right     => app.vmgr_rename_right(),
        KeyCode::Backspace => app.vmgr_rename_backspace(),
        KeyCode::Char(ch)  => app.vmgr_rename_char(ch),
        _ => {}
    }
}

fn handle_vmgr_delete(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => app.vmgr_delete_confirm(),
        KeyCode::Esc   | KeyCode::Char('n') | KeyCode::Char('N') => app.vmgr_delete_cancel(),
        _ => {}
    }
}

fn handle_vmgr_props(app: &mut App, code: KeyCode) {
    // Section sort picker (choices popup for method or order).
    let has_sec_sort_picker = matches!(
        app.vmgr_state.mode,
        ViewMgrMode::Props { sec_sort_picker: Some(_), .. }
    );
    if has_sec_sort_picker { handle_vmgr_sec_sort_picker(app, code); return; }

    // Item sort picker is the innermost layer of the sort dialog.
    let has_item_sort_picker = matches!(
        app.vmgr_state.mode,
        ViewMgrMode::Props { sort_state: SortState::Dialog { ref picker, .. }, .. }
        if picker.is_some()
    );
    let has_sort = matches!(
        app.vmgr_state.mode,
        ViewMgrMode::Props { sort_state: SortState::Dialog { .. }, .. }
    );
    if has_item_sort_picker { handle_vmgr_sort_picker(app, code); return; }
    if has_sort             { handle_vmgr_sort_dialog(app, code); return; }

    let is_name = matches!(
        app.vmgr_state.mode,
        ViewMgrMode::Props { active_field: ViewPropsField::Name, .. }
    );
    let is_sections = matches!(
        app.vmgr_state.mode,
        ViewMgrMode::Props { active_field: ViewPropsField::Sections, .. }
    );
    let is_item_sorting = matches!(
        app.vmgr_state.mode,
        ViewMgrMode::Props { active_field: ViewPropsField::ItemSorting, .. }
    );
    let is_sec_sorting = matches!(
        app.vmgr_state.mode,
        ViewMgrMode::Props { active_field: ViewPropsField::SectionSorting, .. }
    );
    let is_sec_order = matches!(
        app.vmgr_state.mode,
        ViewMgrMode::Props { active_field: ViewPropsField::SectionSortOrder, .. }
    );
    let is_bool = matches!(
        app.vmgr_state.mode,
        ViewMgrMode::Props { active_field, .. }
        if active_field.is_bool()
    );
    match code {
        KeyCode::Enter                                               => app.vmgr_props_confirm(),
        KeyCode::Esc                                                 => app.vmgr_props_cancel(),
        KeyCode::Up   if is_sections                                 => app.vmgr_props_sec_up(),
        KeyCode::Down if is_sections                                 => app.vmgr_props_sec_down(),
        KeyCode::Tab | KeyCode::Down                                 => app.vmgr_props_field_next(),
        KeyCode::BackTab | KeyCode::Up                               => app.vmgr_props_field_prev(),
        KeyCode::F(3) if is_item_sorting                             => app.vmgr_open_item_sort(),
        KeyCode::F(3) if is_sec_sorting || is_sec_order              => app.vmgr_sec_sort_open_picker(),
        KeyCode::Char(' ') if is_sec_sorting                         => app.vmgr_sec_sort_cycle(),
        KeyCode::Char(' ') if is_sec_order                           => app.vmgr_sec_order_cycle(),
        KeyCode::Char(' ') | KeyCode::Left | KeyCode::Right if is_bool => app.vmgr_props_toggle(),
        KeyCode::Left      if is_name                                => app.vmgr_props_name_left(),
        KeyCode::Right     if is_name                                => app.vmgr_props_name_right(),
        KeyCode::Backspace if is_name                                => app.vmgr_props_name_backspace(),
        KeyCode::Char(ch)  if is_name                                => app.vmgr_props_name_char(ch),
        _ => {}
    }
}

fn handle_vmgr_sec_sort_picker(app: &mut App, code: KeyCode) {
    if app.nav_mode == NavMode::Vi {
        app.vi_pending = None;
        match code {
            KeyCode::Char('j') => { app.vmgr_sec_sort_picker_down(); return; }
            KeyCode::Char('k') => { app.vmgr_sec_sort_picker_up();   return; }
            _ => {}
        }
    }
    match code {
        KeyCode::Up    => app.vmgr_sec_sort_picker_up(),
        KeyCode::Down  => app.vmgr_sec_sort_picker_down(),
        KeyCode::Enter => app.vmgr_sec_sort_picker_confirm(),
        KeyCode::Esc   => app.vmgr_sec_sort_picker_cancel(),
        _ => {}
    }
}

fn handle_vmgr_sort_dialog(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Enter  => app.vmgr_sort_confirm(),
        KeyCode::Esc    => app.vmgr_sort_cancel(),
        KeyCode::Tab | KeyCode::Down   => app.vmgr_sort_tab(),
        KeyCode::BackTab | KeyCode::Up => app.vmgr_sort_tab_back(),
        KeyCode::F(3) => app.vmgr_sort_open_picker(),
        _ => {}
    }
}

fn handle_vmgr_sort_picker(app: &mut App, code: KeyCode) {
    if handle_vi_list(app, code,
        &|a| a.vmgr_sort_picker_down(), &|a| a.vmgr_sort_picker_up(),
        &|a| a.vmgr_sort_picker_end(), &|a| a.vmgr_sort_picker_home(),
        &|a| a.vmgr_sort_picker_middle()) { return; }
    match code {
        KeyCode::Up       => app.vmgr_sort_picker_up(),
        KeyCode::Down     => app.vmgr_sort_picker_down(),
        KeyCode::PageUp   => app.vmgr_sort_picker_pgup(10),
        KeyCode::PageDown => app.vmgr_sort_picker_pgdn(10),
        KeyCode::Home     => app.vmgr_sort_picker_home(),
        KeyCode::End      => app.vmgr_sort_picker_end(),
        KeyCode::Enter    => app.vmgr_sort_picker_confirm(),
        KeyCode::Esc      => app.vmgr_sort_picker_cancel(),
        _ => {}
    }
}

// ── Section Properties handlers ───────────────────────────────────────────────

fn handle_sec_props(app: &mut App, code: KeyCode) {
    // Filter picker is outermost (it has no sub-layer).
    let has_filter = matches!(
        app.sec_mode,
        SectionMode::Props { filter_state: FilterState::Open { .. }, .. }
    );
    if has_filter { handle_sec_filter_picker(app, code); return; }
    // Sort picker is the innermost layer of the sort dialog.
    let has_picker = matches!(
        app.sec_mode,
        SectionMode::Props { sort_state: SortState::Dialog { ref picker, .. }, .. }
        if picker.is_some()
    );
    let has_sort = matches!(
        app.sec_mode,
        SectionMode::Props { sort_state: SortState::Dialog { .. }, .. }
    );
    if has_picker { handle_sec_sort_picker(app, code); return; }
    if has_sort   { handle_sec_sort_dialog(app, code); return; }
    handle_sec_props_normal(app, code);
}

fn handle_sec_props_normal(app: &mut App, code: KeyCode) {
    let is_head = matches!(
        app.sec_mode,
        SectionMode::Props { active_field: SecPropsField::Head, .. }
    );
    let is_sorting = matches!(
        app.sec_mode,
        SectionMode::Props { active_field: SecPropsField::ItemSorting, .. }
    );
    let is_filter = matches!(
        app.sec_mode,
        SectionMode::Props { active_field: SecPropsField::Filter, .. }
    );
    match code {
        KeyCode::Enter => app.sec_props_confirm(),
        KeyCode::Esc   => app.sec_props_cancel(),
        KeyCode::Up   if is_filter => app.sec_filter_list_up(),
        KeyCode::Down if is_filter => app.sec_filter_list_down(),
        KeyCode::Tab | KeyCode::Down  => app.sec_props_tab(),
        KeyCode::BackTab | KeyCode::Up => app.sec_props_tab(),
        KeyCode::F(3) if is_sorting   => app.sec_open_sort_dialog(),
        KeyCode::F(3) if is_filter    => app.sec_open_filter_picker(),
        KeyCode::Left  if is_head     => app.sec_props_head_left(),
        KeyCode::Right if is_head     => app.sec_props_head_right(),
        KeyCode::Backspace if is_head => app.sec_props_head_backspace(),
        KeyCode::Char(ch) if is_head  => app.sec_props_head_char(ch),
        _ => {}
    }
}

fn handle_sec_filter_picker(app: &mut App, code: KeyCode) {
    if handle_vi_list(app, code,
        &|a| a.sec_filter_picker_down(), &|a| a.sec_filter_picker_up(),
        &|a| a.sec_filter_picker_end(), &|a| a.sec_filter_picker_home(),
        &|a| a.sec_filter_picker_middle()) { return; }
    match code {
        KeyCode::Up       => app.sec_filter_picker_up(),
        KeyCode::Down     => app.sec_filter_picker_down(),
        KeyCode::PageUp   => app.sec_filter_picker_pgup(10),
        KeyCode::PageDown => app.sec_filter_picker_pgdn(10),
        KeyCode::Home     => app.sec_filter_picker_home(),
        KeyCode::End      => app.sec_filter_picker_end(),
        KeyCode::Char(' ') => app.sec_filter_picker_toggle(),
        KeyCode::Enter    => app.sec_filter_picker_confirm(),
        KeyCode::Esc      => app.sec_filter_picker_cancel(),
        _ => {}
    }
}

fn handle_sec_sort_dialog(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Enter  => app.sec_sort_confirm(),
        KeyCode::Esc    => app.sec_sort_cancel(),
        KeyCode::Tab | KeyCode::Down   => app.sec_sort_tab(),
        KeyCode::BackTab | KeyCode::Up => app.sec_sort_tab_back(),
        KeyCode::F(3) => app.sec_sort_open_picker(),
        _ => {}
    }
}

fn handle_sec_sort_picker(app: &mut App, code: KeyCode) {
    if handle_vi_list(app, code,
        &|a| a.sec_sort_picker_down(), &|a| a.sec_sort_picker_up(),
        &|a| a.sec_sort_picker_end(), &|a| a.sec_sort_picker_home(),
        &|a| a.sec_sort_picker_middle()) { return; }
    match code {
        KeyCode::Up       => app.sec_sort_picker_up(),
        KeyCode::Down     => app.sec_sort_picker_down(),
        KeyCode::PageUp   => app.sec_sort_picker_pgup(10),
        KeyCode::PageDown => app.sec_sort_picker_pgdn(10),
        KeyCode::Home     => app.sec_sort_picker_home(),
        KeyCode::End      => app.sec_sort_picker_end(),
        KeyCode::Enter    => app.sec_sort_picker_confirm(),
        KeyCode::Esc      => app.sec_sort_picker_cancel(),
        _ => {}
    }
}
