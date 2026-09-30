//! Right-click menus: on a task, a comment, a tag, and in text fields and the editor.
//! Their entries run what the matching button or shortcut runs.

use super::theme::{self, BORDER, DIM, TEXT};
use super::{ClipboardOp, TaskerView, key_name};
use gpui::{ClickEvent, Context, Div, Pixels, Point, SharedString, anchored, deferred, div, prelude::*, px, rgb};
use tasker_core::event::{KeyCode, KeyEvent, KeyModifiers};
use tasker_core::model::{Status, Task};

/// Space kept between the menu and the window's edges.
const MARGIN: f32 = 8.;
/// Narrowest the menu gets.
const MIN_WIDTH: f32 = 200.;

/// An open right-click menu.
pub struct ContextMenu {
    /// Where the mouse was, in the window: the menu's top left corner, unless that doesn't fit.
    position: Point<Pixels>,
    /// Its entries, top to bottom.
    entries: Vec<Entry>,
}

impl ContextMenu {
    /// A menu of `entries` at `position`.
    pub fn new(position: Point<Pixels>, entries: Vec<Entry>) -> Self {
        Self { position, entries }
    }
}

/// A line of a menu.
pub enum Entry {
    /// Something to do.
    Item(Item),
    /// A thin line between groups.
    Separator,
}

/// A menu entry that does something.
pub struct Item {
    /// What it says.
    label: SharedString,
    /// Its shortcut, shown on the right.
    shortcut: String,
    /// What it does.
    run: Run,
    /// Whether it can be picked now; disabled entries are dimmed.
    enabled: bool,
}

/// What a menu entry does.
#[derive(Debug, Clone, Copy)]
pub enum Run {
    /// Runs the action of a key, as a button does.
    Key(KeyEvent),
    /// Edits the field being typed into through the clipboard.
    Clipboard(ClipboardOp),
}

/// An entry that runs the action of `key`.
fn key(label: &'static str, c: char, enabled: bool) -> Entry {
    let key = KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE);
    Entry::Item(Item { label: label.into(), shortcut: key_name(key), run: Run::Key(key), enabled })
}

/// An entry that edits text through the clipboard, with its `ctrl` shortcut (`cmd` on macOS).
fn clipboard(label: &'static str, op: ClipboardOp, c: char, enabled: bool) -> Entry {
    let modifier = if cfg!(target_os = "macos") { "Cmd" } else { "Ctrl" };
    Entry::Item(Item { label: label.into(), shortcut: format!("{modifier}+{c}"), run: Run::Clipboard(op), enabled })
}

/// For a task: open it, edit it, change its status (the current one disabled), open its file, delete it.
pub fn for_task(task: &Task) -> Vec<Entry> {
    let status = |label, c, status| key(label, c, task.status != status);
    vec![
        key("Open", 'v', true),
        key("Edit title and tags…", 'e', true),
        key("Edit description…", 'm', true),
        key("Add comment…", 'c', true),
        Entry::Separator,
        status("To do", 'b', Status::Todo),
        status("Doing", 's', Status::Doing),
        status("Parked", 'p', Status::Parked),
        status("Done", 'd', Status::Done),
        Entry::Separator,
        key("Open file", 'o', true),
        key("Delete…", 'x', true),
    ]
}

/// For a comment in the task view.
pub fn for_comment() -> Vec<Entry> {
    vec![key("Edit…", 'e', true), key("Delete…", 'x', true)]
}

/// For a row of the tags screen.
pub fn for_tag() -> Vec<Entry> {
    vec![
        key("Rename…", 'r', true),
        key("Next color", 'c', true),
        key("Previous color", 'C', true),
        Entry::Separator,
        key("Delete…", 'x', true),
    ]
}

/// For a text field or the editor: cut and copy need a selection, paste text on the clipboard.
pub fn for_text(selected: bool, can_paste: bool) -> Vec<Entry> {
    vec![
        clipboard("Cut", ClipboardOp::Cut, 'X', selected),
        clipboard("Copy", ClipboardOp::Copy, 'C', selected),
        clipboard("Paste", ClipboardOp::Paste, 'V', can_paste),
        Entry::Separator,
        clipboard("Select all", ClipboardOp::SelectAll, 'A', true),
    ]
}

/// The menu over everything else, kept inside the window. A press outside closes it and does nothing else.
pub fn draw(menu: &ContextMenu, cx: &mut Context<TaskerView>) -> impl IntoElement {
    let entries = menu.entries.iter().enumerate().map(|(index, entry)| match entry {
        Entry::Separator => div().id(("menu-separator", index)).h(px(1.)).my_1().bg(rgb(BORDER)),
        Entry::Item(item) => {
            let run = item.run;
            div()
                .id(("menu-item", index))
                .debug_selector(|| format!("menu {}", item.label))
                .flex()
                .items_center()
                .justify_between()
                .gap_6()
                .mx_1()
                .px_2()
                .py_1()
                .rounded_sm()
                .text_color(rgb(if item.enabled { TEXT } else { DIM }))
                .when(item.enabled, |d| {
                    d.cursor_pointer()
                        .hover(|s| s.bg(rgb(theme::SELECTED)))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.run_menu(run, cx)))
                })
                .child(item.label.clone())
                .child(div().text_size(px(12.)).text_color(rgb(DIM)).child(item.shortcut.clone()))
        }
    });
    let panel: Div = div()
        .occlude()
        .flex()
        .flex_col()
        .min_w(px(MIN_WIDTH))
        .py_1()
        .rounded_md()
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(theme::POPUP))
        .shadow_lg()
        .text_size(px(13.))
        .on_mouse_down_out(cx.listener(|this, _, _, cx| {
            this.menu = None;
            cx.stop_propagation();
            cx.notify();
        }))
        .children(entries);
    deferred(anchored().position(menu.position).snap_to_window_with_margin(px(MARGIN)).child(panel)).with_priority(1)
}
