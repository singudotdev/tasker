//! Tasks, their status history and comments.
//!
//! - [`markdown`]: reading and writing the task file format.
//! - [`text`]: tags, file-name slugs and heading rules for user text.
//! - [`time`]: timestamps and ages.

mod markdown;
mod text;
mod time;

pub use text::{parse_tags, sanitize_comment, sanitize_description, slugify};
#[cfg(test)]
pub use time::parse_ts;
pub use time::{fmt_age, now};

use crate::enums::named_enum;
use chrono::{DateTime, Local};
use std::path::PathBuf;

named_enum! {
    /// Where a task stands. Declared in the order the task list shows them, so
    /// `position()` is the sort key; the names are what task files contain.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Status {
        /// Being worked on.
        Doing => "doing",
        /// Not started yet.
        Todo => "todo",
        /// Started, then set aside for now.
        Parked => "parked",
        /// Finished; hidden from the list by default.
        Done => "done",
    }
}

/// One status change, as kept in the task's history.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StatusChange {
    /// When the status changed.
    pub at: DateTime<Local>,
    /// The status before.
    pub from: Status,
    /// The status after.
    pub to: Status,
}

/// A timestamped comment on a task, like on a GitHub/GitLab issue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Comment {
    /// When the comment was written.
    pub created: DateTime<Local>,
    /// Set when the body was changed after posting.
    pub edited: Option<DateTime<Local>>,
    /// The comment text (markdown).
    pub body: String,
}

/// A task: what it is, how it's tagged, where it stands and how it got there.
#[derive(Clone, Debug)]
pub struct Task {
    /// Unique number, also the start of the file name.
    pub id: u32,
    /// One-line title.
    pub title: String,
    /// Lowercase, without the leading `#`.
    pub tags: Vec<String>,
    /// Where the task stands.
    pub status: Status,
    /// When the task was created.
    pub created: DateTime<Local>,
    /// Free text describing the task.
    pub description: String,
    /// Oldest first.
    pub comments: Vec<Comment>,
    /// Every status change, oldest first.
    pub history: Vec<StatusChange>,
    /// File this task was loaded from / saved to.
    pub path: Option<PathBuf>,
}

impl Task {
    /// A new `todo` task.
    pub fn new(id: u32, title: String, now: DateTime<Local>) -> Self {
        Self {
            id,
            title,
            tags: Vec::new(),
            status: Status::Todo,
            created: now,
            description: String::new(),
            comments: Vec::new(),
            history: Vec::new(),
            path: None,
        }
    }

    /// Moves the task to `status` and records the change. Returns false if it already was there.
    pub fn set_status(&mut self, status: Status, now: DateTime<Local>) -> bool {
        if self.status == status {
            return false;
        }
        self.history.push(StatusChange { at: now, from: self.status, to: status });
        self.status = status;
        true
    }

    /// When the status last changed (or the task was created, if never); used for sorting.
    pub fn status_since(&self) -> DateTime<Local> {
        self.history.last().map_or(self.created, |c| c.at)
    }

    /// Search: every whitespace-separated term must match. `#term` matches a tag prefix;
    /// anything else matches the title or a tag. Case-insensitive.
    pub fn matches(&self, query: &str) -> bool {
        query.split_whitespace().all(|term| {
            let term = term.to_lowercase();
            match term.strip_prefix('#') {
                Some("") => true,
                Some(tag) => self.tags.iter().any(|t| t.starts_with(tag)),
                None => self.title.to_lowercase().contains(&term) || self.tags.iter().any(|t| t.contains(&term)),
            }
        })
    }
}

#[cfg(test)]
#[path = "../../tests/model.rs"]
mod tests;
