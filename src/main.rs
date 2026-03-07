mod app;
mod menu;
mod model;
mod ui;

use std::io;
use std::io::Write;
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

        // Handle pending note: suspend TUI, open editor, resume TUI.
        if let Some(target) = app.pending_note.take() {
            let existing = app.get_note_content(&target);
            let label    = app.get_note_label(&target);

            // Write existing note to a temp file named after the item/category.
            let filename = format!("beeswax_note_{}.txt", label);
            let tmp_path = std::env::temp_dir().join(&filename);
            if let Ok(mut f) = std::fs::File::create(&tmp_path) {
                let _ = f.write_all(existing.as_bytes());
            }

            // Suspend TUI.
            if enhanced {
                let _ = execute!(terminal.backend_mut(), PopKeyboardEnhancementFlags);
            }
            disable_raw_mode()?;
            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

            // Spawn editor.
            let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
            let _ = std::process::Command::new(&editor)
                .arg(&tmp_path)
                .status();

            // Read result back.
            let content = std::fs::read_to_string(&tmp_path).unwrap_or_default();

            // Resume TUI.
            enable_raw_mode()?;
            execute!(terminal.backend_mut(), EnterAlternateScreen)?;
            if enhanced {
                let _ = execute!(
                    terminal.backend_mut(),
                    PushKeyboardEnhancementFlags(
                        KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                            | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES,
                    )
                );
            }
            terminal.clear()?;

            app.apply_note(target, content);
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
