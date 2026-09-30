//! The task file format:
//!
//! ```markdown
//! # Title
//!
//! - id: 2
//! - status: doing
//! - tags: prod, support
//! - created: 2026-09-24T10:12:00+02:00
//!
//! ## Description
//!
//! free text
//!
//! ## Comments
//!
//! ### 2026-09-24T10:40:00+02:00 (edited 2026-09-24T10:52:00+02:00)
//!
//! comment text
//!
//! ## History
//!
//! - 2026-09-24T10:57:00+02:00 todo -> doing
//! ```

use super::text::parse_tags;
use super::time::{fmt_ts, now, parse_ts};
use super::{Comment, Status, StatusChange, Task};
use chrono::{DateTime, Local};
use std::fmt::Write;

impl Task {
    /// The task as a markdown file (see the format at the top of this module).
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        self.write_markdown(&mut out).expect("writing to a String cannot fail");
        out
    }

    /// Writes the file contents into `out`.
    fn write_markdown(&self, out: &mut String) -> std::fmt::Result {
        writeln!(out, "# {}\n", self.title)?;
        writeln!(out, "- id: {}", self.id)?;
        writeln!(out, "- status: {}", self.status.as_str())?;
        writeln!(out, "- tags: {}", self.tags.join(", "))?;
        writeln!(out, "- created: {}\n", fmt_ts(self.created))?;

        writeln!(out, "## Description\n")?;
        if !self.description.is_empty() {
            writeln!(out, "{}\n", self.description)?;
        }

        writeln!(out, "## Comments\n")?;
        for comment in &self.comments {
            write!(out, "### {}", fmt_ts(comment.created))?;
            if let Some(edited) = comment.edited {
                write!(out, " (edited {})", fmt_ts(edited))?;
            }
            writeln!(out, "\n")?;
            if !comment.body.is_empty() {
                writeln!(out, "{}\n", comment.body)?;
            }
        }

        writeln!(out, "## History\n")?;
        for change in &self.history {
            writeln!(out, "- {} {} -> {}", fmt_ts(change.at), change.from.as_str(), change.to.as_str())?;
        }
        Ok(())
    }

    /// Parses a task file. Unknown `- key:` lines and `##` sections are ignored.
    pub fn from_markdown(text: &str) -> Result<Task, String> {
        let mut parsed = Parsed::default();
        let mut section = Section::Meta;
        // Read line by line; each `## ` heading switches the section the next lines belong to.
        for line in text.lines() {
            let trimmed = line.trim_end();
            if let Some(heading) = trimmed.strip_prefix("## ") {
                section = Section::from_heading(heading);
                continue;
            }
            match section {
                Section::Meta => parsed.meta_line(trimmed),
                // Free text keeps its original indentation, so pass the untrimmed line.
                Section::Description => parsed.description.push(line),
                Section::Comments => parsed.comment_line(line),
                Section::History => parsed.history_line(trimmed)?,
                Section::Other => {}
            }
        }
        parsed.into_task()
    }
}

/// The `##` section a line belongs to.
#[derive(Clone, Copy)]
enum Section {
    /// Title and `- key: value` lines, before the first `##`.
    Meta,
    /// `## Description`: free text.
    Description,
    /// `## Comments`: `### <timestamp>` headings, each followed by the comment.
    Comments,
    /// `## History`: one `- <time> <from> -> <to>` line per status change.
    History,
    /// Any other `##` section; its lines are ignored.
    Other,
}

impl Section {
    /// The section a `## <heading>` starts.
    fn from_heading(heading: &str) -> Self {
        match heading.trim() {
            "Description" => Section::Description,
            "Comments" => Section::Comments,
            "History" => Section::History,
            _ => Section::Other,
        }
    }
}

/// Everything read from a file so far, before it becomes a `Task`.
#[derive(Default)]
struct Parsed<'a> {
    /// From the `# Title` line.
    title: Option<String>,
    /// From `- id:`; required.
    id: Option<u32>,
    /// From `- status:`; `todo` if missing.
    status: Option<Status>,
    /// From `- tags:`.
    tags: Vec<String>,
    /// From `- created:`.
    created: Option<DateTime<Local>>,
    /// Lines of the description, joined at the end.
    description: Vec<&'a str>,
    /// Each comment with its body lines, joined at the end.
    comments: Vec<(Comment, Vec<&'a str>)>,
    /// In file order; sorted at the end.
    history: Vec<StatusChange>,
}

impl<'a> Parsed<'a> {
    /// The title line or a `- key: value` line before the first section.
    fn meta_line(&mut self, line: &str) {
        if self.title.is_none()
            && let Some(title) = line.strip_prefix("# ")
        {
            self.title = Some(title.trim().to_string());
            return;
        }
        let Some((key, value)) = line.trim_start().strip_prefix("- ").and_then(|rest| rest.split_once(':')) else {
            return;
        };
        match key.trim() {
            "id" => self.id = value.trim().parse().ok(),
            "status" => self.status = Status::parse(value),
            "tags" => self.tags = parse_tags(value),
            "created" => self.created = parse_ts(value),
            _ => {}
        }
    }

    /// `### <created> [(edited <time>)]` starts a comment; other lines belong to the current one.
    fn comment_line(&mut self, line: &'a str) {
        match comment_header(line.trim_end()) {
            Some((created, edited)) => {
                self.comments.push((Comment { created, edited, body: String::new() }, Vec::new()));
            }
            None => {
                // Text before the first comment header has nowhere to go.
                if let Some((_, body)) = self.comments.last_mut() {
                    body.push(line);
                }
            }
        }
    }

    /// `- <time> <from> -> <to>`. The time may contain a space when typed by hand.
    fn history_line(&mut self, line: &str) -> Result<(), String> {
        let Some(rest) = line.trim_start().strip_prefix("- ") else { return Ok(()) };
        let bad = || format!("bad history line: {rest}");
        let (left, to) = rest.split_once("->").ok_or_else(bad)?;
        let (at, from) = left.trim().rsplit_once(' ').ok_or_else(bad)?;
        let at = parse_ts(at).ok_or_else(|| format!("bad timestamp: {}", at.trim()))?;
        let from = Status::parse(from).ok_or_else(bad)?;
        let to = Status::parse(to).ok_or_else(bad)?;
        self.history.push(StatusChange { at, from, to });
        Ok(())
    }

    /// Builds the task.
    fn into_task(mut self) -> Result<Task, String> {
        let id = self.id.ok_or("missing `- id:` line")?;
        self.history.sort_by_key(|c| c.at);
        Ok(Task {
            id,
            title: self.title.unwrap_or_else(|| "untitled".into()),
            tags: self.tags,
            status: self.status.unwrap_or(Status::Todo),
            created: self.created.or_else(|| self.history.first().map(|c| c.at)).unwrap_or_else(now),
            description: self.description.join("\n").trim().to_string(),
            comments: self
                .comments
                .into_iter()
                .map(|(comment, body)| Comment { body: body.join("\n").trim().to_string(), ..comment })
                .collect(),
            history: self.history,
            path: None,
        })
    }
}

/// Parses `### <created>` or `### <created> (edited <time>)`.
fn comment_header(line: &str) -> Option<(DateTime<Local>, Option<DateTime<Local>>)> {
    let header = line.strip_prefix("### ")?;
    let (created, edited) = match header.split_once("(edited") {
        Some((created, edited)) => (created, parse_ts(edited.trim().trim_end_matches(')'))),
        None => (header, None),
    };
    Some((parse_ts(created)?, edited))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn round_trip() {
        let t0 = parse_ts("2026-09-24 09:00").unwrap();
        let mut task = Task::new(7, "Fix prod login".into(), t0);
        task.description = "customer X reported it\n\n- check logs\n### steps".into();
        task.comments = vec![
            Comment { created: t0, edited: None, body: "first\n\nwith a blank line".into() },
            Comment {
                created: t0 + Duration::minutes(5),
                edited: Some(t0 + Duration::minutes(9)),
                body: "second".into(),
            },
            Comment { created: t0 + Duration::minutes(7), edited: None, body: String::new() },
        ];
        task.tags = vec!["prod".into(), "support".into()];
        task.set_status(Status::Doing, t0 + Duration::minutes(10));
        task.set_status(Status::Done, t0 + Duration::hours(2));

        let back = Task::from_markdown(&task.to_markdown()).unwrap();
        assert_eq!(back.id, 7);
        assert_eq!(back.title, task.title);
        assert_eq!(back.status, Status::Done);
        assert_eq!(back.created, t0);
        assert_eq!(back.description, task.description);
        assert_eq!(back.comments, task.comments);
        assert_eq!(back.tags, task.tags);
        assert_eq!(back.history, task.history);
    }

    #[test]
    fn hand_typed_history_and_missing_status() {
        let md = "# x\n\n- id: 1\n\n## History\n\n- 2026-09-24 10:00 todo -> doing\n";
        let task = Task::from_markdown(md).unwrap();
        assert_eq!(task.status, Status::Todo, "the status line decides, not the history");
        assert_eq!(task.history[0].at, parse_ts("2026-09-24 10:00").unwrap());
        assert_eq!(task.history[0].to, Status::Doing);
        assert!(Task::from_markdown("# x\n\n- id: 1\n\n## History\n\n- 2026-09-24 10:00 todo -> nope\n").is_err());
    }

    #[test]
    fn missing_id_is_an_error() {
        assert!(Task::from_markdown("# no id\n").is_err());
    }
}
