//! What the app is currently showing or asking for, and the state each screen needs.

use crate::editor::{LineInput, TextEditor};
use crate::enums::ordered_enum;
use crate::keys::{self, Binding};
use crate::model::{Status, Task};

/// Which screen or dialog is active. Each variant carries the state that screen needs.
#[derive(Debug)]
pub enum Mode {
    /// The task list.
    Normal,
    /// The create / edit form.
    Input(Form),
    /// A yes/no dialog.
    Confirm(Confirm),
    /// Typing into the `/` search box; the list filters live.
    Search,
    /// The `?` menu: every action of the screen it was opened from, runnable.
    Keys(KeysMenu),
    /// A task shown like an issue: description and comments.
    Issue(IssueView),
    /// Editing a description or comment.
    Edit(Edit),
    /// The tags screen: every tag, to rename, recolor or delete.
    Tags(TagsView),
    /// Typing a new name for a tag.
    RenameTag(RenameTag),
}

/// What the create/edit form is for.
#[derive(Debug, Clone, Copy)]
pub enum InputKind {
    /// Create a new `todo` task.
    New,
    /// Edit the title and tags of the task with this id.
    Edit(u32),
}

/// A field of the create/edit form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    /// The task title.
    Title,
    /// The comma- or space-separated tags.
    Tags,
}

impl Field {
    /// The other field (what `tab` switches to).
    pub fn other(self) -> Self {
        match self {
            Field::Title => Field::Tags,
            Field::Tags => Field::Title,
        }
    }
}

/// The create / edit form.
#[derive(Debug)]
pub struct Form {
    /// What submitting the form does.
    pub kind: InputKind,
    /// Typed title, not trimmed yet.
    pub title: LineInput,
    /// Free text, parsed with `parse_tags` on submit.
    pub tags: LineInput,
    /// The field being typed into.
    pub field: Field,
}

impl Form {
    /// An empty form.
    pub fn new(kind: InputKind) -> Self {
        Self { kind, title: LineInput::default(), tags: LineInput::default(), field: Field::Title }
    }

    /// A form filled with the task's current title and tags.
    pub fn edit(task: &Task) -> Self {
        Self {
            kind: InputKind::Edit(task.id),
            title: LineInput::new(&task.title),
            tags: LineInput::new(&task.tags.join(", ")),
            field: Field::Title,
        }
    }

    /// The field being typed into.
    pub fn active(&self) -> &LineInput {
        match self.field {
            Field::Title => &self.title,
            Field::Tags => &self.tags,
        }
    }

    /// The field being typed into, to edit.
    pub fn active_mut(&mut self) -> &mut LineInput {
        match self.field {
            Field::Title => &mut self.title,
            Field::Tags => &mut self.tags,
        }
    }
}

/// A delete waiting for `y` in a dialog.
#[derive(Debug, Clone, Copy)]
pub enum Confirm {
    /// Delete the task with this id.
    Delete(u32),
    /// Task id and comment index.
    DeleteComment(u32, usize),
    /// Remove a tag from every task; the index is into `App::all_tags`.
    DeleteTag(usize),
}

ordered_enum! {
    /// Which tasks the list shows with respect to done ones; `f` cycles through them in this order.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum DoneView {
        /// Only tasks that aren't done (the default).
        #[default]
        Hide,
        /// All tasks, including done ones.
        Show,
        /// Only done tasks.
        Only,
    }
}

impl DoneView {
    /// Whether a task with `status` is listed in this view.
    pub fn allows(self, status: Status) -> bool {
        match self {
            DoneView::Hide => status != Status::Done,
            DoneView::Show => true,
            DoneView::Only => status == Status::Done,
        }
    }
}

/// The task view: which task, and which comment is selected.
#[derive(Debug, Clone, Copy)]
pub struct IssueView {
    /// The task shown.
    pub id: u32,
    /// Selected comment.
    pub sel: usize,
}

/// The tags screen: which tag is selected.
#[derive(Debug, Clone, Copy)]
pub struct TagsView {
    /// Index into `App::all_tags`.
    pub sel: usize,
}

/// The rename dialog of the tags screen.
#[derive(Debug)]
pub struct RenameTag {
    /// The tag being renamed, as an index into `App::all_tags`.
    pub sel: usize,
    /// Typed new name, parsed with `parse_tags` on submit.
    pub name: LineInput,
}

/// What an open editor writes to when saved.
#[derive(Debug, Clone, Copy)]
pub enum Target {
    /// The description of the task with this id.
    Description(u32),
    /// A new comment on the task with this id.
    NewComment(u32),
    /// Task id and comment index.
    Comment(u32, usize),
}

impl Target {
    /// The task this editor belongs to.
    pub fn task_id(self) -> u32 {
        match self {
            Target::Description(id) | Target::NewComment(id) | Target::Comment(id, _) => id,
        }
    }
}

/// An open text editor: the text, where it will be saved, and where to go afterwards.
#[derive(Debug)]
pub struct Edit {
    /// The text being edited.
    pub editor: TextEditor,
    /// Where the text is saved.
    pub target: Target,
    /// Task view to return to; `None` returns to the list.
    pub back: Option<IssueView>,
}

impl Edit {
    /// The screen to go back to once the editor closes.
    pub fn return_mode(&self) -> Mode {
        self.back.map_or(Mode::Normal, Mode::Issue)
    }
}

/// The `?` menu: every action of the screen it was opened from.
#[derive(Debug)]
pub struct KeysMenu {
    /// The highlighted entry.
    pub sel: usize,
    /// The screen it was opened from, and returns to.
    pub back: Back,
}

/// Screen a menu returns to.
#[derive(Debug)]
pub enum Back {
    /// The task list.
    List,
    /// The task view, as it was.
    Issue(IssueView),
    /// The tags screen, as it was.
    Tags(TagsView),
}

impl KeysMenu {
    /// A menu for the screen `back`, with the first entry highlighted.
    pub fn new(back: Back) -> Self {
        Self { sel: 0, back }
    }

    /// The menu entries: the screen's bindings that have a description.
    pub fn bindings(&self) -> Vec<&'static Binding> {
        keys::menu(match self.back {
            Back::List => keys::LIST,
            Back::Issue(_) => keys::ISSUE,
            Back::Tags(_) => keys::TAGS,
        })
    }

    /// The screen to return to when the menu closes.
    pub fn into_back(self) -> Mode {
        match self.back {
            Back::List => Mode::Normal,
            Back::Issue(view) => Mode::Issue(view),
            Back::Tags(view) => Mode::Tags(view),
        }
    }
}
