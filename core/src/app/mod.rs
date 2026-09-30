//! Application state: the tasks, what's on screen, and everything that changes them.
//!
//! - [`mode`]: the screens and dialogs and the state each one needs.
//! - [`actions`]: operations on tasks (create, change status, comment, …).
//! - [`input`]: keyboard, mouse and button handling, which calls the actions.

mod actions;
mod input;
pub mod mode;
#[cfg(test)]
mod tests;

pub use input::Click;
pub use mode::{
    Back, Confirm, DoneView, Edit, Field, Form, InputKind, IssueView, KeysMenu, Mode, RenameTag, TagsView, Target,
};

use crate::editor::LineInput;
use crate::model::Task;
use crate::store::Store;
use crate::tags::TagColors;
use anyhow::Result;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// How long a status message shows (in the TUI's footer, the desktop app's status bar).
const MESSAGE_TTL: Duration = Duration::from_secs(5);

/// Everything the running app knows: the tasks, the screen being shown, and UI state.
#[derive(Debug)]
pub struct App {
    /// The data folder the tasks are read from and written to.
    pub store: Store,
    /// Every task, in no particular order (see `visible` for display order).
    pub tasks: Vec<Task>,
    /// Tag → color assignments, kept in `tags.md`.
    pub tag_colors: TagColors,
    /// Index into `visible()`.
    pub selected: usize,
    /// Whether done tasks are hidden, shown, or the only ones shown.
    pub done_view: DoneView,
    /// Task list filter; see [`Task::matches`].
    pub search: LineInput,
    /// The current screen or dialog.
    pub mode: Mode,
    /// Set to end the event loop.
    pub quit: bool,
    /// Set when the user asks to open a file in `$EDITOR`; handled by the event loop.
    pub edit_request: Option<PathBuf>,
    /// Footer message and when it was set; it expires after `MESSAGE_TTL`.
    message: Option<(String, Instant)>,
}

impl App {
    /// Loads every task.
    pub fn new(store: Store) -> Result<Self> {
        let mut app = Self {
            store,
            tasks: Vec::new(),
            tag_colors: TagColors::default(),
            selected: 0,
            done_view: DoneView::default(),
            search: LineInput::default(),
            mode: Mode::Normal,
            quit: false,
            edit_request: None,
            message: None,
        };
        app.reload()?;
        Ok(app)
    }

    /// Shows `msg` in the footer instead of the key hints for a few seconds.
    pub fn notify(&mut self, msg: impl Into<String>) {
        self.message = Some((msg.into(), Instant::now()));
    }

    /// The footer message, while it hasn't expired.
    pub fn message(&self) -> Option<&str> {
        self.message.as_ref().filter(|(_, at)| at.elapsed() < MESSAGE_TTL).map(|(msg, _)| msg.as_str())
    }

    /// Re-reads every file from disk.
    pub fn reload(&mut self) -> Result<()> {
        let (tasks, warnings) = self.store.load()?;
        self.tasks = tasks;
        self.tag_colors = self.store.load_tag_colors()?;
        self.sync_tag_colors()?;
        if !warnings.is_empty() {
            self.notify(format!("skipped {} file(s): {}", warnings.len(), warnings.join("; ")));
        }
        self.clamp_selection();
        Ok(())
    }

    /// Indices into `tasks` in display order: doing, todo, done; most recently changed first.
    pub fn visible(&self) -> Vec<usize> {
        let mut indices: Vec<usize> = (0..self.tasks.len())
            .filter(|&i| self.done_view.allows(self.tasks[i].status) && self.tasks[i].matches(self.search.text()))
            .collect();
        indices.sort_by(|&a, &b| {
            let (ta, tb) = (&self.tasks[a], &self.tasks[b]);
            ta.status
                .position()
                .cmp(&tb.status.position())
                .then(tb.status_since().cmp(&ta.status_since()))
                .then(tb.id.cmp(&ta.id))
        });
        indices
    }

    /// Index into `tasks` of the task with this id.
    pub fn find(&self, id: u32) -> Option<usize> {
        self.tasks.iter().position(|t| t.id == id)
    }

    /// The task with this id.
    pub fn task(&self, id: u32) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }

    /// Index into `tasks` of the highlighted row.
    fn selected_index(&self) -> Option<usize> {
        self.visible().get(self.selected).copied()
    }

    /// The highlighted task in the list, if the list isn't empty.
    pub fn selected_task(&self) -> Option<&Task> {
        self.selected_index().map(|i| &self.tasks[i])
    }

    /// Highlights the task with this id, if it's visible.
    fn select_id(&mut self, id: u32) {
        if let Some(pos) = self.visible().iter().position(|&i| self.tasks[i].id == id) {
            self.selected = pos;
        }
    }

    /// Keeps the selection inside the list after it shrank.
    fn clamp_selection(&mut self) {
        self.selected = self.selected.min(self.visible().len().saturating_sub(1));
    }

    /// The id for a new task: one more than the highest so far.
    fn next_id(&self) -> u32 {
        self.tasks.iter().map(|t| t.id).max().unwrap_or(0) + 1
    }

    /// All tags in use, sorted.
    pub fn all_tags(&self) -> Vec<String> {
        let mut tags: Vec<String> = self.tasks.iter().flat_map(|t| t.tags.iter().cloned()).collect();
        tags.sort();
        tags.dedup();
        tags
    }

    /// Colors any new tags, oldest tasks first so earlier tags claim colors first.
    fn sync_tag_colors(&mut self) -> Result<()> {
        let mut by_age: Vec<&Task> = self.tasks.iter().collect();
        by_age.sort_by_key(|t| t.id);
        if self.tag_colors.assign(by_age.iter().flat_map(|t| &t.tags)) {
            self.store.save_tag_colors(&self.tag_colors)?;
        }
        Ok(())
    }
}
