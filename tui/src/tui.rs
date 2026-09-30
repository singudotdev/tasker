//! The terminal session: setup and teardown, the event loop (crossterm input to the core's), and handing
//! over to `$EDITOR`.

use crate::ui::{self, ListArea};
use anyhow::{Context, Result, bail};
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode as TermKey, KeyEvent as TermKeyEvent, KeyEventKind,
    KeyModifiers as TermModifiers, MouseButton, MouseEventKind,
};
use ratatui::crossterm::execute;
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use tasker_core::app::{App, Click};
use tasker_core::event::{KeyCode, KeyEvent, KeyModifiers};

/// How often the screen refreshes without input, so "since" ages stay current.
const TICK: Duration = Duration::from_millis(500);

/// Runs the interactive app until the user quits, restoring the terminal afterwards (even on a panic).
pub fn run(mut app: App) -> Result<()> {
    let mut terminal = init_terminal();
    // ratatui's panic hook restores the screen; also give the mouse back to the terminal.
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        release_mouse();
        hook(info);
    }));
    let result = event_loop(&mut terminal, &mut app);
    restore_terminal();
    result
}

/// Draws, waits for input (or the next tick), handles it; repeats until `app.quit`.
fn event_loop(terminal: &mut DefaultTerminal, app: &mut App) -> Result<()> {
    let list_area = ListArea::default();
    while !app.quit {
        terminal.draw(|f| ui::draw(f, app, &list_area))?;
        if event::poll(TICK)? {
            match event::read()? {
                // Windows also reports key releases.
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if let Some(key) = key_event(key) {
                        app.on_key(key);
                    }
                }
                // The wheel does what ↑/↓ do; a left click selects a task.
                Event::Mouse(mouse) => match mouse.kind {
                    MouseEventKind::ScrollDown => app.wheel(true),
                    MouseEventKind::ScrollUp => app.wheel(false),
                    MouseEventKind::Down(MouseButton::Left) => {
                        if let Some(index) = list_area.row_at(mouse.column, mouse.row) {
                            app.click(Click::Row(index));
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        // `o` asked for an external editor: hand the terminal over, wait, take it back, and
        // re-read the files since the user may have changed anything.
        if let Some(path) = app.edit_request.take() {
            restore_terminal();
            let edited = open_editor(&path);
            *terminal = init_terminal();
            terminal.clear()?;
            if let Err(e) = edited.and_then(|()| app.reload()) {
                app.notify(format!("error: {e:#}"));
            }
        }
    }
    Ok(())
}

/// A crossterm key press as the core's key event; `None` for keys tasker doesn't use.
fn key_event(key: TermKeyEvent) -> Option<KeyEvent> {
    let code = match key.code {
        TermKey::Char(c) => KeyCode::Char(c),
        TermKey::Enter => KeyCode::Enter,
        TermKey::Esc => KeyCode::Esc,
        TermKey::Tab => KeyCode::Tab,
        TermKey::Backspace => KeyCode::Backspace,
        TermKey::Delete => KeyCode::Delete,
        TermKey::Left => KeyCode::Left,
        TermKey::Right => KeyCode::Right,
        TermKey::Up => KeyCode::Up,
        TermKey::Down => KeyCode::Down,
        TermKey::Home => KeyCode::Home,
        TermKey::End => KeyCode::End,
        _ => return None,
    };
    let control = key.modifiers.contains(TermModifiers::CONTROL);
    Some(KeyEvent::new(code, if control { KeyModifiers::CONTROL } else { KeyModifiers::NONE }))
}

/// Full-screen terminal with mouse support (wheel scrolls, click selects).
fn init_terminal() -> DefaultTerminal {
    let terminal = ratatui::init();
    // Without mouse capture the app still works fully from the keyboard.
    let _ = execute!(std::io::stdout(), EnableMouseCapture);
    terminal
}

/// Leaves the full-screen mode and gives the mouse back.
fn restore_terminal() {
    release_mouse();
    ratatui::restore();
}

/// Turns mouse capture off.
fn release_mouse() {
    // Best effort: there's nothing useful to do if the terminal refuses.
    let _ = execute!(std::io::stdout(), DisableMouseCapture);
}

/// Opens `path` in `$VISUAL` / `$EDITOR` (which may include arguments, e.g. `code -w`) and waits.
fn open_editor(path: &Path) -> Result<()> {
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| if cfg!(windows) { "notepad" } else { "vi" }.to_string());
    let mut parts = editor.split_whitespace();
    let program = parts.next().context("$EDITOR is empty")?;
    let status = Command::new(program).args(parts).arg(path).status().with_context(|| format!("running {program}"))?;
    if !status.success() {
        bail!("{program} exited with {status}");
    }
    Ok(())
}
