use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crate::app::App;

pub fn handle_event(app: &mut App, event: Event) {
    let Event::Key(KeyEvent { code, modifiers, .. }) = event else { return };

    // Alt-Q always quits
    if modifiers.contains(KeyModifiers::ALT) && code == KeyCode::Char('q') {
        app.quit = true;
        return;
    }

    match &app.mode {
        crate::app::Mode::Normal    => handle_normal(app, code),
        crate::app::Mode::Create {..} => handle_create(app, code),
        crate::app::Mode::Edit   {..} => handle_edit(app, code),
    }
}

fn handle_normal(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Up     => app.cursor_up(),
        KeyCode::Down   => app.cursor_down(),
        KeyCode::Insert => app.begin_create_blank(),
        KeyCode::F(2) | KeyCode::Enter => app.begin_edit(),
        KeyCode::Char(ch) => app.begin_create(ch),
        _ => {}
    }
}

fn handle_create(app: &mut App, code: KeyCode) {
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

fn handle_edit(app: &mut App, code: KeyCode) {
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
