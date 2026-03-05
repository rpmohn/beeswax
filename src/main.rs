mod app;
mod menu;
mod model;
mod ui;

use std::io;
use crossterm::{
    event::{
        self, Event,
        KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
        PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, supports_keyboard_enhancement,
        EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use ratatui::{backend::CrosstermBackend, Terminal};

use app::App;
use ui::{input::handle_event, render::render};

fn main() -> io::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    // Enable enhanced keyboard protocol if the terminal supports it.
    // This allows modifier-only key events (Shift/Ctrl/Alt press and release)
    // so the F-key bar can update live while a modifier is held.
    let enhanced = supports_keyboard_enhancement().unwrap_or(false);
    if enhanced {
        let _ = execute!(
            stdout,
            PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                    | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES,
            )
        );
    }

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    loop {
        terminal.draw(|f| render(f, &app))?;

        if event::poll(std::time::Duration::from_millis(50))? {
            let ev = event::read()?;
            if matches!(ev, Event::Key(_)) {
                handle_event(&mut app, ev);
            }
        }

        if app.quit {
            break;
        }
    }

    // Restore terminal
    if enhanced {
        let _ = execute!(terminal.backend_mut(), PopKeyboardEnhancementFlags);
    }
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
