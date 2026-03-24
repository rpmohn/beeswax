mod app;
mod menu;
mod model;
mod persist;
mod ui;

use std::io;
use std::io::Write;
use std::path::PathBuf;
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
    // ── CLI arg parsing ───────────────────────────────────────────────────────
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut encrypt_flag = false;
    let mut file_arg: Option<PathBuf> = None;

    for arg in &args {
        if arg == "--encrypt" {
            encrypt_flag = true;
        } else if !arg.starts_with('-') {
            file_arg = Some(PathBuf::from(arg));
        }
    }

    // ── Pre-TUI startup: load or create ──────────────────────────────────────
    let mut app = if let Some(ref path) = file_arg {
        if path.exists() {
            // File exists: probe it
            match persist::probe(path)? {
                persist::LoadResult::Plain(data) => {
                    App::from_save(data, Some(path.clone()), None)
                }
                persist::LoadResult::NeedsPassword => {
                    // Encrypted: ask for password before entering TUI
                    let password = read_password_pre_tui("Password: ")?;
                    match persist::load_encrypted(path, &password) {
                        Ok(data) => App::from_save(data, Some(path.clone()), Some(password)),
                        Err(e) => {
                            eprintln!("Error loading file: {e}");
                            std::process::exit(1);
                        }
                    }
                }
            }
        } else if encrypt_flag {
            // New file, encrypted: ask for password before TUI
            let password = read_password_with_confirm_pre_tui()?;
            let mut a = App::new();
            a.file_path        = Some(path.clone());
            a.session_password = Some(password);
            a
        } else {
            // New plain file
            let mut a = App::new();
            a.file_path = Some(path.clone());
            a
        }
    } else {
        // No file arg → ephemeral (in-memory only)
        App::new()
    };

    // ── Setup terminal ────────────────────────────────────────────────────────
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    // Enable enhanced keyboard protocol if the terminal supports it.
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

// ── Pre-TUI password prompts ──────────────────────────────────────────────────

/// Read a password character-by-character with no echo.
fn read_password_pre_tui(prompt: &str) -> io::Result<String> {
    enable_raw_mode()?;
    print!("{}", prompt);
    io::stdout().flush()?;

    let mut buf = String::new();
    loop {
        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(k) = event::read()? {
                use crossterm::event::{KeyCode, KeyEventKind};
                if !matches!(k.kind, KeyEventKind::Press) { continue; }
                match k.code {
                    KeyCode::Enter => break,
                    KeyCode::Char(c) => { buf.push(c); }
                    KeyCode::Backspace => { buf.pop(); }
                    KeyCode::Esc => { buf.clear(); break; }
                    _ => {}
                }
            }
        }
    }
    disable_raw_mode()?;
    println!();
    Ok(buf)
}

/// Read a password with confirm prompt.
fn read_password_with_confirm_pre_tui() -> io::Result<String> {
    loop {
        let pw1 = read_password_pre_tui("New password:     ")?;
        let pw2 = read_password_pre_tui("Confirm password: ")?;
        if pw1 == pw2 {
            return Ok(pw1);
        }
        println!("Passwords do not match, try again.");
    }
}
