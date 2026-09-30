//! Keyboard, mouse and button handling. Every key handled here is listed in `crate::keys`,
//! so the TUI's footer and the `?` menu show everything that works. The desktop app's buttons run
//! their action's key (`App::press`); other clicks select or focus something (`App::click`).

use super::App;
use super::mode::{
    Back, Confirm, DoneView, Field, Form, InputKind, IssueView, KeysMenu, Mode, RenameTag, TagsView, Target,
};
use crate::editor::{LineInput, Outcome, TextEditor};
use crate::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::model::{Status, parse_tags};
use anyhow::Result;

/// A click on something that isn't a button (buttons use `App::press`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Click {
    /// A task list row, as an index into `App::visible`: selects it.
    Row(usize),
    /// A double-clicked task list row: opens the task view.
    OpenRow(usize),
    /// A comment in the task view: selects it.
    Comment(usize),
    /// A row of the tags screen, as an index into `App::all_tags`: selects it.
    Tag(usize),
    /// A field of the create/edit form: types into it.
    Field(Field),
    /// The search box: types into it.
    Search,
    /// A list filter: shows those tasks.
    DoneView(DoneView),
}

impl App {
    /// Handles one key press for the current screen.
    pub fn on_key(&mut self, key: KeyEvent) {
        // ctrl+c only means something in the text editor: discard the changes.
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            if let Mode::Edit(edit) = &self.mode {
                self.mode = edit.return_mode();
                self.notify("changes discarded");
            }
            return;
        }
        // Each handler puts back the mode it wants to continue in; the default is the list.
        let result = match std::mem::replace(&mut self.mode, Mode::Normal) {
            Mode::Normal => self.key_normal(key),
            Mode::Input(form) => self.key_input(form, key),
            Mode::Confirm(action) => self.key_confirm(action, key),
            Mode::Search => {
                self.key_search(key);
                Ok(())
            }
            Mode::Keys(menu) => {
                self.key_keys(menu, key);
                Ok(())
            }
            Mode::Issue(view) => self.key_issue(view, key),
            Mode::Tags(view) => self.key_tags(view, key),
            Mode::RenameTag(rename) => self.key_rename_tag(rename, key),
            Mode::Edit(mut edit) => match edit.editor.on_key(key) {
                Outcome::Continue => {
                    self.mode = Mode::Edit(edit);
                    Ok(())
                }
                Outcome::Save => self.save_edit(&edit),
            },
        };
        if let Err(e) = result {
            self.notify(format!("error: {e:#}"));
        }
    }

    /// The mouse wheel does what `↑`/`↓` do on the current screen.
    pub fn wheel(&mut self, down: bool) {
        // In dialogs and the form any key answers or edits, so the wheel stays out of them.
        if !matches!(self.mode, Mode::Confirm(_) | Mode::Input(_) | Mode::RenameTag(_)) {
            self.on_key(KeyEvent::new(if down { KeyCode::Down } else { KeyCode::Up }, KeyModifiers::NONE));
        }
    }

    /// A button: runs its action's key on the current screen, as pressing the key would.
    /// While typing into the search box, the box is left first (keeping the filter),
    /// so the key runs its action instead of being typed.
    pub fn press(&mut self, key: KeyEvent) {
        if matches!(self.mode, Mode::Search) {
            self.mode = Mode::Normal;
        }
        self.on_key(key);
    }

    /// A click on something that isn't a button. Clicks that don't belong to the current screen are ignored.
    pub fn click(&mut self, click: Click) {
        let tasks = self.visible().len();
        let tags = self.all_tags().len();
        match (click, &mut self.mode) {
            (Click::Row(index), Mode::Normal | Mode::Search) if index < tasks => self.selected = index,
            (Click::OpenRow(index), Mode::Normal | Mode::Search) if index < tasks => {
                self.selected = index;
                self.press(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE));
            }
            (Click::Comment(index), Mode::Issue(view)) => view.sel = index,
            (Click::Tag(index), Mode::Tags(view)) if index < tags => view.sel = index,
            (Click::Field(field), Mode::Input(form)) => form.field = field,
            (Click::Search, Mode::Normal) => self.mode = Mode::Search,
            (Click::DoneView(view), Mode::Normal | Mode::Search) => {
                self.done_view = view;
                self.selected = 0;
            }
            _ => {}
        }
    }

    /// Selects all the text of the field or editor being typed into, if any.
    pub fn select_all(&mut self) {
        match self.typing() {
            Some(Typing::Line(input)) => input.select_all(),
            Some(Typing::Text(editor)) => editor.select_all(),
            None => {}
        }
    }

    /// The text selected in the field or editor being typed into, to copy.
    pub fn selected_text(&self) -> Option<String> {
        match &self.mode {
            Mode::Search => self.search.selected_text().map(str::to_string),
            Mode::Input(form) => form.active().selected_text().map(str::to_string),
            Mode::RenameTag(rename) => rename.name.selected_text().map(str::to_string),
            Mode::Edit(edit) => edit.editor.selected_text(),
            _ => None,
        }
    }

    /// Removes the selected text from the field or editor being typed into and returns it, to cut.
    pub fn cut(&mut self) -> Option<String> {
        let text = self.selected_text()?;
        match self.typing() {
            Some(Typing::Line(input)) => _ = input.delete_selection(),
            Some(Typing::Text(editor)) => _ = editor.delete_selection(),
            None => {}
        }
        self.search_changed();
        Some(text)
    }

    /// Types `text` into the field or editor being typed into, in place of the selection, to paste.
    /// One-line fields get it on one line.
    pub fn paste(&mut self, text: &str) {
        match self.typing() {
            Some(Typing::Line(input)) => _ = input.insert(text),
            Some(Typing::Text(editor)) => editor.insert(text),
            None => {}
        }
        self.search_changed();
    }

    /// Where typed text goes on the current screen, if anywhere.
    fn typing(&mut self) -> Option<Typing<'_>> {
        match &mut self.mode {
            Mode::Search => Some(Typing::Line(&mut self.search)),
            Mode::Input(form) => Some(Typing::Line(form.active_mut())),
            Mode::RenameTag(rename) => Some(Typing::Line(&mut rename.name)),
            Mode::Edit(edit) => Some(Typing::Text(&mut edit.editor)),
            _ => None,
        }
    }

    /// After the search query may have changed outside `key_search`: the results start over at the top.
    fn search_changed(&mut self) {
        if matches!(self.mode, Mode::Search) {
            self.selected = 0;
        }
    }

    /// The task list: global keys first, then keys that act on the highlighted task.
    fn key_normal(&mut self, key: KeyEvent) -> Result<()> {
        if let Some(sel) = step(self.selected, self.visible().len(), key.code) {
            self.selected = sel;
            return Ok(());
        }
        match key.code {
            KeyCode::Char('n') => self.mode = Mode::Input(Form::new(InputKind::New)),
            KeyCode::Char('/') => self.mode = Mode::Search,
            KeyCode::Esc if !self.search.is_empty() => {
                self.search.clear();
                self.clamp_selection();
            }
            KeyCode::Char('f') => {
                self.done_view = self.done_view.next();
                self.selected = 0;
            }
            KeyCode::Char('t') => self.mode = Mode::Tags(TagsView { sel: 0 }),
            KeyCode::Char('R') => self.reload_from_disk()?,
            KeyCode::Char('?') => self.mode = Mode::Keys(KeysMenu::new(Back::List)),
            KeyCode::Char('q') => self.quit = true,
            _ => {
                if let Some(i) = self.selected_index() {
                    self.key_on_task(i, key)?;
                }
            }
        }
        Ok(())
    }

    /// Keys that act on the highlighted task (`i` indexes `tasks`).
    fn key_on_task(&mut self, i: usize, key: KeyEvent) -> Result<()> {
        let id = self.tasks[i].id;
        match key.code {
            KeyCode::Char('s') => self.set_status(i, Status::Doing)?,
            KeyCode::Char('d') => self.set_status(i, Status::Done)?,
            KeyCode::Char('p') => self.set_status(i, Status::Parked)?,
            KeyCode::Char('b') => self.set_status(i, Status::Todo)?,
            KeyCode::Char('e') => self.mode = Mode::Input(Form::edit(&self.tasks[i])),
            KeyCode::Char('v') => {
                let sel = self.tasks[i].comments.len().saturating_sub(1);
                self.mode = Mode::Issue(IssueView { id, sel });
            }
            KeyCode::Char('m') => self.open_editor(Target::Description(id), None),
            KeyCode::Char('c') => self.open_editor(Target::NewComment(id), None),
            KeyCode::Char('o') => self.edit_request.clone_from(&self.tasks[i].path),
            KeyCode::Char('x') => self.mode = Mode::Confirm(Confirm::Delete(id)),
            _ => {}
        }
        Ok(())
    }

    /// The create/edit form: typing, `tab` between fields, `enter` to save, `esc` to cancel.
    fn key_input(&mut self, mut form: Form, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {}
            KeyCode::Enter => return self.submit_form(form),
            // Two fields: tab and the up/down arrows all switch to the other one.
            KeyCode::Tab | KeyCode::Up | KeyCode::Down => {
                form.field = form.field.other();
                self.mode = Mode::Input(form);
            }
            _ => {
                form.active_mut().on_key(key);
                self.mode = Mode::Input(form);
            }
        }
        Ok(())
    }

    /// Creates or updates the task from the form. An empty title keeps the form open.
    fn submit_form(&mut self, mut form: Form) -> Result<()> {
        let title = form.title.text().trim().to_string();
        if title.is_empty() {
            form.field = Field::Title;
            self.mode = Mode::Input(form);
            return Ok(());
        }
        let tags = parse_tags(form.tags.text());
        match form.kind {
            InputKind::New => self.create_task(title, tags),
            InputKind::Edit(id) => self.update_task(id, title, tags),
        }
    }

    /// A delete dialog: `y` confirms, anything else cancels.
    fn key_confirm(&mut self, action: Confirm, key: KeyEvent) -> Result<()> {
        let confirmed = key.code == KeyCode::Char('y');
        match action {
            // Back to the task view whether confirmed or not.
            Confirm::DeleteComment(id, index) => {
                self.mode = Mode::Issue(IssueView { id, sel: index });
                if confirmed {
                    self.delete_comment(id, index)?;
                }
                Ok(())
            }
            // Back to the tags screen whether confirmed or not.
            Confirm::DeleteTag(sel) => {
                self.mode = Mode::Tags(TagsView { sel });
                match self.all_tags().into_iter().nth(sel) {
                    Some(tag) if confirmed => self.delete_tag(&tag),
                    _ => Ok(()),
                }
            }
            Confirm::Delete(id) if confirmed => self.delete_task(id),
            Confirm::Delete(_) => Ok(()),
        }
    }

    /// The `/` search box: the list filters as the query changes.
    fn key_search(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => return,
            KeyCode::Esc => {
                self.search.clear();
                self.clamp_selection();
                return;
            }
            // Only ↑/↓ move the list here: letters are part of the query, ←/→ move the cursor.
            KeyCode::Down | KeyCode::Up => {
                self.selected = step(self.selected, self.visible().len(), key.code).unwrap_or(self.selected);
            }
            _ => {
                if self.search.on_key(key) {
                    self.selected = 0;
                }
            }
        }
        self.mode = Mode::Search;
    }

    /// The task view: select, add, edit and delete comments; edit the description.
    fn key_issue(&mut self, mut view: IssueView, key: KeyEvent) -> Result<()> {
        // The task may be gone (e.g. deleted and reloaded); then fall back to the list.
        let Some(i) = self.find(view.id) else { return Ok(()) };
        let count = self.tasks[i].comments.len();
        view.sel = view.sel.min(count.saturating_sub(1));
        if let Some(sel) = step(view.sel, count, key.code) {
            view.sel = sel;
            self.mode = Mode::Issue(view);
            return Ok(());
        }
        let status = match key.code {
            KeyCode::Char('s') => Some(Status::Doing),
            KeyCode::Char('d') => Some(Status::Done),
            KeyCode::Char('p') => Some(Status::Parked),
            KeyCode::Char('b') => Some(Status::Todo),
            _ => None,
        };
        if let Some(status) = status {
            self.mode = Mode::Issue(view);
            return self.set_status(i, status);
        }
        match key.code {
            KeyCode::Esc => {}
            KeyCode::Char('?') => self.mode = Mode::Keys(KeysMenu::new(Back::Issue(view))),
            KeyCode::Char('m') => self.open_editor(Target::Description(view.id), Some(view)),
            KeyCode::Char('c') => self.open_editor(Target::NewComment(view.id), Some(view)),
            KeyCode::Char('e') if count > 0 => self.open_editor(Target::Comment(view.id, view.sel), Some(view)),
            KeyCode::Char('x') if count > 0 => self.mode = Mode::Confirm(Confirm::DeleteComment(view.id, view.sel)),
            _ => self.mode = Mode::Issue(view),
        }
        Ok(())
    }

    /// The tags screen: select a tag, then rename, recolor or delete it everywhere.
    fn key_tags(&mut self, mut view: TagsView, key: KeyEvent) -> Result<()> {
        let tags = self.all_tags();
        view.sel = view.sel.min(tags.len().saturating_sub(1));
        if let Some(sel) = step(view.sel, tags.len(), key.code) {
            view.sel = sel;
            self.mode = Mode::Tags(view);
            return Ok(());
        }
        self.mode = Mode::Tags(view);
        match key.code {
            KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Char('?') => self.mode = Mode::Keys(KeysMenu::new(Back::Tags(view))),
            code => {
                // The remaining keys act on the selected tag, if there is one.
                let Some(tag) = tags.get(view.sel) else { return Ok(()) };
                match code {
                    KeyCode::Char('r') => {
                        self.mode = Mode::RenameTag(RenameTag { sel: view.sel, name: LineInput::new(tag) });
                    }
                    KeyCode::Char('c') => self.cycle_tag_color(tag, false)?,
                    KeyCode::Char('C') => self.cycle_tag_color(tag, true)?,
                    KeyCode::Char('x') => self.mode = Mode::Confirm(Confirm::DeleteTag(view.sel)),
                    _ => {}
                }
            }
        }
        Ok(())
    }

    /// The rename dialog: `enter` renames (an empty name keeps it open), `esc` cancels.
    fn key_rename_tag(&mut self, mut rename: RenameTag, key: KeyEvent) -> Result<()> {
        let back = Mode::Tags(TagsView { sel: rename.sel });
        match key.code {
            KeyCode::Esc => self.mode = back,
            KeyCode::Enter => {
                let Some(from) = self.all_tags().into_iter().nth(rename.sel) else {
                    self.mode = back;
                    return Ok(());
                };
                match parse_tags(rename.name.text()).as_slice() {
                    [] => self.mode = Mode::RenameTag(rename),
                    [to] if *to == from => self.mode = back,
                    [to] => return self.rename_tag(&from, to),
                    _ => {
                        self.notify("a tag is a single word: no spaces or commas");
                        self.mode = Mode::RenameTag(rename);
                    }
                }
            }
            _ => {
                rename.name.on_key(key);
                self.mode = Mode::RenameTag(rename);
            }
        }
        Ok(())
    }

    /// Runs the highlighted action (or the one whose key was pressed) by replaying its key.
    fn key_keys(&mut self, mut menu: KeysMenu, key: KeyEvent) {
        let bindings = menu.bindings();
        if let Some(sel) = step(menu.sel, bindings.len(), key.code) {
            menu.sel = sel;
            self.mode = Mode::Keys(menu);
            return;
        }
        let run = match key.code {
            KeyCode::Esc => {
                self.mode = menu.into_back();
                return;
            }
            KeyCode::Enter => bindings.get(menu.sel).map(|b| b.code),
            code => bindings.iter().find(|b| b.code == code).map(|b| b.code),
        };
        match run {
            Some(code) => {
                self.mode = menu.into_back();
                self.on_key(KeyEvent::new(code, KeyModifiers::NONE));
            }
            None => self.mode = Mode::Keys(menu),
        }
    }
}

/// A text input being typed into: one line, or the multi-line editor.
enum Typing<'a> {
    /// A form field, the search box or a tag's new name.
    Line(&'a mut LineInput),
    /// The description or comment editor.
    Text(&'a mut TextEditor),
}

/// `↑`/`k` and `↓`/`j` move a selection within `len` items; other keys return `None`.
fn step(sel: usize, len: usize, key: KeyCode) -> Option<usize> {
    match key {
        KeyCode::Char('j') | KeyCode::Down => Some((sel + 1).min(len.saturating_sub(1))),
        KeyCode::Char('k') | KeyCode::Up => Some(sel.saturating_sub(1)),
        _ => None,
    }
}
