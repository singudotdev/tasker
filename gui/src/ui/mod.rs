//! The window: a toolbar, the current screen, dialogs over it and a status bar.
//! Buttons run the same actions as the keyboard shortcuts (`App::press`), other clicks select or focus
//! something (`App::click`), and key presses reach `App::on_key` just like the terminal UI's.
//!
//! - [`list`]: the task list with its toolbar, and the details panel.
//! - [`issue`]: the task view (description, history and comments).
//! - [`tags`]: the tags screen.
//! - [`dialogs`]: dialogs drawn over the current screen.
//! - [`widgets`]: buttons, fields, pills and chips.
//! - [`mod@pointer`]: the mouse in text fields and the text editor.
//! - [`menu`]: right-click menus.
//! - [`theme`]: colors, fonts and styled text.

mod dialogs;
mod issue;
mod list;
mod menu;
mod pointer;
mod tags;
mod theme;
mod widgets;

use anyhow::{Context as _, Result, bail};
use chrono::{DateTime, Local};
use gpui::{
    App as GpuiApp, Bounds, ClickEvent, ClipboardItem, Context, Div, ElementId, FocusHandle, KeyDownEvent, Keystroke,
    MouseButton, MouseDownEvent, NavigationDirection, Pixels, Point, ScrollHandle, SharedString, Stateful, Window,
    WindowBounds, WindowOptions, div, prelude::*, px, rgb, size,
};
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use tasker_core::app::{App, Click, Mode};
use tasker_core::event::{KeyCode, KeyEvent, KeyModifiers};
use tasker_core::model::StatusChange;
use theme::{DIM_STYLE, Line, Span, Style};
use widgets::Kind;

/// How often the window redraws without input, so "since" ages stay current and messages expire.
const TICK: Duration = Duration::from_millis(500);

/// Opens the window and runs until it's closed or the app quits.
pub fn run(app: App) {
    gpui_platform::application().run(move |cx: &mut GpuiApp| {
        cx.on_window_closed(|cx, _window| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        let bounds = Bounds::centered(None, size(px(1200.), px(760.)), cx);
        let options = WindowOptions { window_bounds: Some(WindowBounds::Windowed(bounds)), ..Default::default() };
        let opened = cx.open_window(options, |window, cx| {
            window.set_window_title("tasker");
            cx.new(|cx| TaskerView::new(app, window, cx))
        });
        if let Err(e) = opened {
            eprintln!("tasker-gui: could not open the window: {e:#}");
            cx.quit();
        }
        cx.activate(true);
    });
}

/// The root view: the app, and what the window needs to draw it.
struct TaskerView {
    /// Tasks, the current screen, and everything that changes them.
    app: App,
    /// Keyboard focus of the window, so key presses reach the app.
    focus: FocusHandle,
    /// Scroll positions of the scrollable areas.
    scroll: Scrolls,
    /// Whether the next frame scrolls the selection into view: after the keyboard or a button moved it,
    /// but not on every frame, so the wheel can scroll freely.
    follow: bool,
    /// The field or editor a mouse drag that selects text started in, while the button is down.
    drag: Option<pointer::TextBox>,
    /// The open right-click menu, if any.
    menu: Option<menu::ContextMenu>,
    /// The text last put in the primary selection, so it's only replaced when the selection changes.
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    shared: Option<String>,
    /// Redraws every `TICK`; dropped with the view.
    _tick: gpui::Task<()>,
}

/// Scroll positions of the scrollable areas.
#[derive(Default)]
struct Scrolls {
    /// The task list.
    list: ScrollHandle,
    /// The details panel.
    details: ScrollHandle,
    /// The task view.
    issue: ScrollHandle,
    /// The tags screen.
    tags: ScrollHandle,
    /// The keyboard shortcuts dialog.
    keys: ScrollHandle,
    /// The text editor.
    editor: ScrollHandle,
}

impl TaskerView {
    /// The view over `app`, focused so keys work right away, redrawing every `TICK`.
    fn new(app: App, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        let tick = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(TICK).await;
                if this.update(cx, |_, cx| cx.notify()).is_err() {
                    break;
                }
            }
        });
        Self {
            app,
            focus,
            scroll: Scrolls::default(),
            follow: true,
            drag: None,
            menu: None,
            #[cfg(any(target_os = "linux", target_os = "freebsd"))]
            shared: None,
            _tick: tick,
        }
    }

    /// A key press: a clipboard shortcut, or handed to the app as the terminal version would receive it.
    fn on_key_down(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        // An open menu takes the key only to close, as desktop menus do.
        if self.menu.take().is_some() {
            cx.stop_propagation();
            cx.notify();
            return;
        }
        if self.clipboard(&event.keystroke, cx) {
            cx.stop_propagation();
            self.follow = true;
            self.after_input(cx);
            return;
        }
        let keys = key_events(&event.keystroke);
        if keys.is_empty() {
            return;
        }
        cx.stop_propagation();
        for key in keys {
            self.app.on_key(key);
        }
        self.follow = true;
        self.after_input(cx);
    }

    /// Ctrl+A / C / X / V (Cmd on macOS): select all, copy, cut and paste in the field or editor being
    /// typed into, with the system clipboard. Returns whether `keystroke` was one of them. They do nothing
    /// elsewhere; in the text editor, ctrl+c copies instead of discarding (there's a button for that).
    fn clipboard(&mut self, keystroke: &Keystroke, cx: &mut Context<Self>) -> bool {
        let modifiers = &keystroke.modifiers;
        let shortcut = if cfg!(target_os = "macos") { modifiers.platform } else { modifiers.control };
        if !shortcut || modifiers.alt {
            return false;
        }
        let op = match keystroke.key.as_str() {
            "a" => ClipboardOp::SelectAll,
            "c" => ClipboardOp::Copy,
            "x" => ClipboardOp::Cut,
            "v" => ClipboardOp::Paste,
            _ => return false,
        };
        self.clipboard_op(op, cx);
        true
    }

    /// Selects all, copies, cuts or pastes in the field or editor being typed into, with the system clipboard.
    fn clipboard_op(&mut self, op: ClipboardOp, cx: &mut Context<Self>) {
        match op {
            ClipboardOp::SelectAll => self.app.select_all(),
            ClipboardOp::Copy => {
                if let Some(text) = self.app.selected_text() {
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                }
            }
            ClipboardOp::Cut => {
                if let Some(text) = self.app.cut() {
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                }
            }
            ClipboardOp::Paste => {
                if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                    self.app.paste(&text);
                }
            }
        }
    }

    /// Opens a right-click menu of `entries` at `position`, in place of any open one.
    fn open_menu(&mut self, position: Point<Pixels>, entries: Vec<menu::Entry>, cx: &mut Context<Self>) {
        self.menu = Some(menu::ContextMenu::new(position, entries));
        cx.stop_propagation();
        self.after_input(cx);
    }

    /// A picked menu entry: the menu closes, then it runs.
    fn run_menu(&mut self, run: menu::Run, cx: &mut Context<Self>) {
        self.menu = None;
        match run {
            menu::Run::Key(key) => self.press(None, key, cx),
            menu::Run::Clipboard(op) => {
                self.clipboard_op(op, cx);
                self.follow = true;
                self.after_input(cx);
            }
        }
    }

    /// A button: first `click` (e.g. selecting the comment the button belongs to), then its action's key.
    fn press(&mut self, click: Option<Click>, key: KeyEvent, cx: &mut Context<Self>) {
        if let Some(click) = click {
            self.app.click(click);
        }
        self.app.press(key);
        self.follow = true;
        self.after_input(cx);
    }

    /// The mouse's back button leaves the task view and the tags screen, as `esc` does.
    /// Dialogs cover the whole window, so it doesn't reach here while one is open.
    fn on_back_button(&mut self, _: &MouseDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if matches!(self.app.mode, Mode::Issue(_) | Mode::Tags(_)) {
            self.press(None, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), cx);
        }
    }

    /// A click that selects or focuses something.
    fn click(&mut self, click: Click, cx: &mut Context<Self>) {
        self.app.click(click);
        self.after_input(cx);
    }

    /// Quits when the app asked to, runs a requested `$EDITOR`, shares the selected text, and redraws.
    fn after_input(&mut self, cx: &mut Context<Self>) {
        if self.app.quit {
            cx.quit();
            return;
        }
        self.share_selection(cx);
        // "Open file" asked for an external editor: run it, and re-read the files when it exits
        // since the user may have changed anything.
        if let Some(path) = self.app.edit_request.take() {
            let edit = cx.background_executor().spawn(async move { open_editor(&path) });
            cx.spawn(async move |this, cx| {
                let edited = edit.await;
                this.update(cx, |this, cx| {
                    if let Err(e) = edited.and_then(|()| this.app.reload()) {
                        this.app.notify(format!("error: {e:#}"));
                    }
                    cx.notify();
                })
                .ok();
            })
            .detach();
        }
        cx.notify();
    }
}

impl TaskerView {
    /// Puts selected text in the primary selection, as Linux desktops do, for a middle-click to paste elsewhere.
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    fn share_selection(&mut self, cx: &mut Context<Self>) {
        let Some(text) = self.app.selected_text() else { return };
        if self.shared.as_ref() != Some(&text) {
            cx.write_to_primary(ClipboardItem::new_string(text.clone()));
            self.shared = Some(text);
        }
    }

    /// Only Linux desktops have a primary selection.
    #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
    fn share_selection(&mut self, _: &mut Context<Self>) {}
}

/// An edit through the clipboard, from a shortcut or a right-click menu.
#[derive(Debug, Clone, Copy)]
enum ClipboardOp {
    /// Select all.
    SelectAll,
    /// Copy the selection.
    Copy,
    /// Cut the selection.
    Cut,
    /// Paste over the selection.
    Paste,
}

impl Render for TaskerView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let follow = std::mem::take(&mut self.follow);
        let app = &self.app;
        let screen = match (issue::shown_behind(&app.mode), tags::shown_behind(&app.mode)) {
            (Some(view), _) => issue::draw(app, view, &self.scroll.issue, follow, cx),
            (_, Some(view)) => tags::draw(app, view, &self.scroll.tags, follow, cx),
            _ => list::draw(app, window, &self.scroll, follow, cx),
        };
        div()
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key_down))
            .on_mouse_down(MouseButton::Navigate(NavigationDirection::Back), cx.listener(Self::on_back_button))
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(theme::BG))
            .text_color(rgb(theme::TEXT))
            .font_family(theme::UI_FONT)
            .text_size(px(14.))
            .child(div().flex_1().min_h_0().flex().flex_col().child(screen))
            .child(status_bar(app))
            .children(dialogs::draw(app, &self.scroll, follow, cx))
            .children(self.menu.as_ref().map(|m| menu::draw(m, cx)))
    }
}

/// The bottom bar: the latest message, or a tip; and where the tasks are kept.
fn status_bar(app: &App) -> Div {
    let left = match app.message() {
        Some(msg) => div().text_color(rgb(theme::YELLOW)).child(msg.to_string()),
        None if matches!(app.mode, Mode::Normal | Mode::Search) => {
            widgets::dim("Double-click a task to open it · press ? for keyboard shortcuts")
        }
        None => widgets::dim("Press ? for keyboard shortcuts"),
    };
    div()
        .flex()
        .flex_none()
        .items_center()
        .justify_between()
        .gap_4()
        .h(px(26.))
        .px_3()
        .border_t_1()
        .border_color(rgb(theme::BORDER))
        .bg(rgb(theme::SURFACE))
        .text_size(px(12.))
        .whitespace_nowrap()
        .overflow_hidden()
        .child(left)
        .child(widgets::dim(app.store.dir().display().to_string()))
}

/// A button that runs the action of `code`; its tooltip names the shortcut.
fn action(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    kind: Kind,
    code: KeyCode,
    cx: &mut Context<TaskerView>,
) -> Stateful<Div> {
    action_after(id, label, kind, None, KeyEvent::new(code, KeyModifiers::NONE), cx)
}

/// A button that runs `click`, if any, then the action of `key`; its tooltip names the shortcut.
fn action_after(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    kind: Kind,
    click: Option<Click>,
    key: KeyEvent,
    cx: &mut Context<TaskerView>,
) -> Stateful<Div> {
    widgets::button(id, label, kind)
        .tooltip(widgets::tooltip(format!("Shortcut: {}", key_name(key))))
        .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| this.press(click, key, cx)))
}

/// A key as people write it: `s`, `Esc`, `Ctrl+C`.
fn key_name(key: KeyEvent) -> String {
    let name = match key.code {
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "Enter".into(),
        KeyCode::Esc => "Esc".into(),
        KeyCode::Tab => "Tab".into(),
        KeyCode::Backspace => "Backspace".into(),
        KeyCode::Delete => "Delete".into(),
        KeyCode::Left => "←".into(),
        KeyCode::Right => "→".into(),
        KeyCode::Up => "↑".into(),
        KeyCode::Down => "↓".into(),
        KeyCode::Home => "Home".into(),
        KeyCode::End => "End".into(),
    };
    if key.modifiers.contains(KeyModifiers::CONTROL) { format!("Ctrl+{}", name.to_uppercase()) } else { name }
}

/// The app's key events for a GPUI keystroke: what the terminal would have sent for the same key.
/// Usually one; typed text that composes several characters gives one per character.
fn key_events(keystroke: &Keystroke) -> Vec<KeyEvent> {
    let modifiers = &keystroke.modifiers;
    let named = match keystroke.key.as_str() {
        "enter" => Some(KeyCode::Enter),
        "escape" => Some(KeyCode::Esc),
        // shift+tab is a different key in the terminal (BackTab), which tasker doesn't use.
        "tab" if modifiers.shift => return Vec::new(),
        "tab" => Some(KeyCode::Tab),
        "backspace" => Some(KeyCode::Backspace),
        "delete" => Some(KeyCode::Delete),
        "left" => Some(KeyCode::Left),
        "right" => Some(KeyCode::Right),
        "up" => Some(KeyCode::Up),
        "down" => Some(KeyCode::Down),
        "home" => Some(KeyCode::Home),
        "end" => Some(KeyCode::End),
        _ => None,
    };
    let ctrl = if modifiers.control { KeyModifiers::CONTROL } else { KeyModifiers::NONE };
    if let Some(code) = named {
        // Shift with a movement key selects text.
        let shift = if modifiers.shift { KeyModifiers::SHIFT } else { KeyModifiers::NONE };
        return vec![KeyEvent::new(code, ctrl | shift)];
    }
    // Shortcuts with the super/command key are the desktop's, not the app's.
    if modifiers.platform {
        return Vec::new();
    }
    // ctrl+letter arrives as the letter, as in the terminal.
    if modifiers.control {
        let mut chars = keystroke.key.chars();
        return match (chars.next(), chars.next()) {
            (Some(c), None) => vec![KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)],
            _ => Vec::new(),
        };
    }
    // Typed text, after shift, dead keys and compose. Simulated keystrokes (tests) have none: use the key name.
    let text = match &keystroke.key_char {
        Some(text) => text.clone(),
        None if keystroke.key == "space" => " ".to_string(),
        None if keystroke.key.chars().count() == 1 && modifiers.shift => keystroke.key.to_uppercase(),
        None if keystroke.key.chars().count() == 1 => keystroke.key.clone(),
        None => return Vec::new(),
    };
    text.chars().filter(|c| !c.is_control()).map(|c| KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)).collect()
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

/// `Mon 28 Sep 2026 10:12  todo → doing`, the new status in its color.
fn history_line(change: &StatusChange) -> Line {
    Line::from(vec![
        Span::styled(format!("{}  ", when(change.at)), DIM_STYLE),
        Span::raw(format!("{} → ", theme::status_label(change.from))),
        Span::styled(theme::status_label(change.to), theme::status(change.to)),
    ])
}

/// Cuts `s` to `width` characters, ending with `…` when shortened.
fn truncate(s: &str, width: usize) -> String {
    if s.chars().count() <= width {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(width.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

/// "Thu 24 Sep 2026 10:15" in local time.
fn when(dt: DateTime<Local>) -> String {
    dt.format("%a %d %b %Y %H:%M").to_string()
}

/// Text being typed into: the cursor, if any, before character `cursor`, and the characters from `start` to
/// `end` of `selection` highlighted. The cursor is always at one end of the selection.
fn edited(text: &str, cursor: Option<usize>, selection: Option<(usize, usize)>) -> Vec<Span> {
    let byte = |at: usize| text.char_indices().nth(at).map_or(text.len(), |(i, _)| i);
    let (start, end) = selection.or(cursor.map(|c| (c, c))).unwrap_or((0, 0));
    let (a, b) = (byte(start), byte(end));
    let mut spans = vec![
        Span::raw(&text[..a]),
        Span::styled(&text[a..b], Style::new().bg(theme::SELECTION)),
        Span::raw(&text[b..]),
    ];
    if let Some(cursor) = cursor {
        let caret = Span::styled("▏", Style::new().fg(theme::ACCENT).bold());
        spans.insert(if cursor == start { 1 } else { 2 }, caret);
    }
    spans
}

#[cfg(test)]
mod tests;
