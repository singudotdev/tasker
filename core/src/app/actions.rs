//! Operations on tasks. Every change is written to disk immediately.

use super::App;
use super::mode::{Edit, IssueView, Mode, TagsView, Target};
use crate::editor::TextEditor;
use crate::model::{Comment, Status, Task, now, sanitize_comment, sanitize_description};
use anyhow::Result;

impl App {
    /// Moves the task at `idx` to `status`, recording when.
    pub(super) fn set_status(&mut self, idx: usize, status: Status) -> Result<()> {
        let id = self.tasks[idx].id;
        if !self.tasks[idx].set_status(status, now()) {
            self.notify(format!("#{id} is already {}", status.as_str()));
            return Ok(());
        }
        self.store.save(&mut self.tasks[idx])?;
        self.notify(format!("#{id} {} → {}", self.tasks[idx].title, status.as_str()));
        // The task moves in the list (and may leave it, e.g. done while done tasks are hidden).
        self.select_id(id);
        self.clamp_selection();
        Ok(())
    }

    /// Creates a `todo` task with the next free id and selects it.
    pub(super) fn create_task(&mut self, title: String, tags: Vec<String>) -> Result<()> {
        let mut task = Task::new(self.next_id(), title, now());
        task.tags = tags;
        self.store.save(&mut task)?;
        let id = task.id;
        self.tasks.push(task);
        self.sync_tag_colors()?;
        self.select_id(id);
        Ok(())
    }

    /// Saves a new title and tags from the edit form (a new title also renames the file).
    pub(super) fn update_task(&mut self, id: u32, title: String, tags: Vec<String>) -> Result<()> {
        let Some(i) = self.find(id) else { return Ok(()) };
        self.tasks[i].title = title;
        self.tasks[i].tags = tags;
        self.store.save(&mut self.tasks[i])?;
        self.sync_tag_colors()
    }

    /// Deletes the task's file.
    pub(super) fn delete_task(&mut self, id: u32) -> Result<()> {
        let Some(idx) = self.find(id) else { return Ok(()) };
        self.store.delete(&self.tasks[idx])?;
        self.tasks.remove(idx);
        self.clamp_selection();
        Ok(())
    }

    /// Renames a tag on every task that has it. If `to` is already in use the two tags merge,
    /// keeping `to`'s color. Selects the renamed tag in the tags screen.
    pub(super) fn rename_tag(&mut self, from: &str, to: &str) -> Result<()> {
        let mut count = 0;
        for task in &mut self.tasks {
            let Some(pos) = task.tags.iter().position(|t| t == from) else { continue };
            if task.tags.iter().any(|t| t == to) {
                task.tags.remove(pos);
            } else {
                task.tags[pos] = to.to_string();
            }
            self.store.save(task)?;
            count += 1;
        }
        self.tag_colors.rename(from, to);
        self.store.save_tag_colors(&self.tag_colors)?;
        // A search filter may match the task list differently now.
        self.clamp_selection();
        let sel = self.all_tags().iter().position(|t| t == to).unwrap_or(0);
        self.mode = Mode::Tags(TagsView { sel });
        self.notify(format!("renamed {from} → {to} on {count} task(s)"));
        Ok(())
    }

    /// Removes a tag from every task and forgets its color.
    pub(super) fn delete_tag(&mut self, tag: &str) -> Result<()> {
        let mut count = 0;
        for task in &mut self.tasks {
            let before = task.tags.len();
            task.tags.retain(|t| t != tag);
            if task.tags.len() != before {
                self.store.save(task)?;
                count += 1;
            }
        }
        self.tag_colors.remove(tag);
        self.store.save_tag_colors(&self.tag_colors)?;
        self.clamp_selection();
        self.notify(format!("removed {tag} from {count} task(s)"));
        Ok(())
    }

    /// Steps a tag to the next (or previous) palette color.
    pub(super) fn cycle_tag_color(&mut self, tag: &str, back: bool) -> Result<()> {
        self.tag_colors.cycle(tag, back);
        self.store.save_tag_colors(&self.tag_colors)
    }

    /// `R`: re-reads everything and says so, unless reloading had something else to report.
    pub(super) fn reload_from_disk(&mut self) -> Result<()> {
        self.message = None;
        self.reload()?;
        if self.message.is_none() {
            self.notify("reloaded from disk");
        }
        Ok(())
    }

    /// Opens the text editor on a description or comment.
    pub(super) fn open_editor(&mut self, target: Target, back: Option<IssueView>) {
        let Some(task) = self.task(target.task_id()) else { return };
        let text = match target {
            Target::Description(_) => task.description.as_str(),
            Target::NewComment(_) => "",
            Target::Comment(_, c) => task.comments.get(c).map_or("", |c| c.body.as_str()),
        };
        self.mode = Mode::Edit(Edit { editor: TextEditor::new(text), target, back });
    }

    /// Writes the editor's text to its target and returns to the screen it was opened from.
    pub(super) fn save_edit(&mut self, edit: &Edit) -> Result<()> {
        self.mode = edit.return_mode();
        if !edit.editor.modified {
            return Ok(());
        }
        let text = edit.editor.text();
        let message = match edit.target {
            Target::Description(id) => self.set_description(id, &text)?,
            Target::NewComment(id) => self.add_comment(id, &text)?,
            Target::Comment(id, index) => self.update_comment(id, index, &text)?,
        };
        self.notify(message);
        Ok(())
    }

    /// Replaces the description. Returns the footer message.
    fn set_description(&mut self, id: u32, text: &str) -> Result<&'static str> {
        let Some(i) = self.find(id) else { return Ok("the task no longer exists") };
        let (text, demoted) = sanitize_description(text);
        self.tasks[i].description = text;
        self.store.save(&mut self.tasks[i])?;
        Ok(if demoted { "description saved (## headings became ###)" } else { "description saved" })
    }

    /// Adds a comment and selects it in the task view.
    fn add_comment(&mut self, id: u32, text: &str) -> Result<&'static str> {
        let Some(i) = self.find(id) else { return Ok("the task no longer exists") };
        let (body, demoted) = sanitize_comment(text);
        if body.is_empty() {
            return Ok("empty comment, nothing added");
        }
        self.tasks[i].comments.push(Comment { created: now(), edited: None, body });
        self.store.save(&mut self.tasks[i])?;
        if let Mode::Issue(view) = &mut self.mode {
            view.sel = self.tasks[i].comments.len() - 1;
        }
        Ok(if demoted { "comment added (headings became ####)" } else { "comment added" })
    }

    /// Changes a comment's text and marks it edited, unless the text is unchanged. Returns the footer message.
    fn update_comment(&mut self, id: u32, index: usize, text: &str) -> Result<&'static str> {
        let Some(i) = self.find(id) else { return Ok("the task no longer exists") };
        let (body, demoted) = sanitize_comment(text);
        if body.is_empty() {
            return Ok("comment left unchanged: to remove it, press x");
        }
        let Some(comment) = self.tasks[i].comments.get_mut(index) else { return Ok("the comment no longer exists") };
        if comment.body != body {
            comment.body = body;
            comment.edited = Some(now());
            self.store.save(&mut self.tasks[i])?;
        }
        Ok(if demoted { "comment updated (headings became ####)" } else { "comment updated" })
    }

    /// Deletes a comment and stays in the task view with a neighbour selected.
    pub(super) fn delete_comment(&mut self, id: u32, index: usize) -> Result<()> {
        let Some(i) = self.find(id) else { return Ok(()) };
        if index >= self.tasks[i].comments.len() {
            return Ok(());
        }
        self.tasks[i].comments.remove(index);
        let sel = index.min(self.tasks[i].comments.len().saturating_sub(1));
        self.mode = Mode::Issue(IssueView { id, sel });
        self.notify("comment deleted");
        self.store.save(&mut self.tasks[i])
    }
}
