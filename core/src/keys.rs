//! One list of key bindings per screen. The footer and the `?` menu are both built from it,
//! and the menu runs an action by replaying its key, so there is a single source of truth.

use crate::event::KeyCode;

/// A key and what it does on one screen.
#[derive(Debug)]
pub struct Binding {
    /// Key replayed when the action is picked from the `?` menu.
    pub code: KeyCode,
    /// How the key is shown in the `?` menu.
    pub key: &'static str,
    /// Footer hint as (key shown, label), rendered mnemonic-style: `(n)ew`, `(x) delete`.
    /// `None` keeps it out of the footer.
    pub footer: Option<(&'static str, &'static str)>,
    /// Line in the `?` menu; `None` keeps it out of the menu (e.g. plain movement).
    pub menu: Option<&'static str>,
}

/// Short constructor, to keep the tables below readable.
const fn b(
    code: KeyCode,
    key: &'static str,
    footer: Option<(&'static str, &'static str)>,
    menu: Option<&'static str>,
) -> Binding {
    Binding { code, key, footer, menu }
}

use KeyCode::{Char, Down, Esc};

/// The task list.
pub const LIST: &[Binding] = &[
    b(Down, "↑↓/jk", Some(("↑↓/jk", "move")), None),
    b(Char('n'), "n", Some(("n", "new")), Some("new task (title + tags), as todo")),
    b(Char('s'), "s", Some(("s", "start")), Some("move the selected task to doing")),
    b(Char('p'), "p", Some(("p", "park")), Some("move the selected task to parked: started, set aside for now")),
    b(Char('d'), "d", Some(("d", "done")), Some("move the selected task to done")),
    b(Char('b'), "b", Some(("b", "back to todo")), Some("move the selected task back to todo")),
    b(Char('e'), "e", Some(("e", "edit")), Some("edit title and tags")),
    b(Char('v'), "v", Some(("v", "view")), Some("view the task like an issue: description, comments, history")),
    b(Char('m'), "m", Some(("m", "description")), Some("edit the selected task's description")),
    b(Char('c'), "c", Some(("c", "comment")), Some("add a comment to the selected task")),
    b(Char('o'), "o", Some(("o", "open file")), Some("open the task's file in $EDITOR")),
    b(Char('x'), "x", Some(("x", "delete")), Some("delete the selected task (asks first)")),
    b(Char('/'), "/", Some(("/", "search")), Some("search titles and tags; #tag matches tags only")),
    b(Char('f'), "f", Some(("f", "filter")), Some("cycle the list: active → all → done only")),
    b(Char('t'), "t", Some(("t", "tags")), Some("tags: rename, recolor or delete a tag on every task")),
    b(Char('R'), "R", Some(("R", "Reload")), Some("reload all files from disk")),
    b(Char('?'), "?", Some(("?", "keys")), None),
    b(Char('q'), "q", Some(("q", "quit")), Some("quit")),
];

/// The task view (`v`): description, comments and status history.
pub const ISSUE: &[Binding] = &[
    b(Down, "↑↓/jk", Some(("↑↓/jk", "select")), None),
    b(Char('c'), "c", Some(("c", "new comment")), Some("add a comment")),
    b(Char('e'), "e", Some(("e", "edit")), Some("edit the selected comment")),
    b(Char('x'), "x", Some(("x", "delete")), Some("delete the selected comment (asks first)")),
    b(Char('m'), "m", Some(("m", "description")), Some("edit the description")),
    b(Char('s'), "s", Some(("s", "start")), Some("move the task to doing")),
    b(Char('p'), "p", Some(("p", "park")), Some("move the task to parked")),
    b(Char('d'), "d", Some(("d", "done")), Some("move the task to done")),
    b(Char('b'), "b", Some(("b", "back to todo")), Some("move the task back to todo")),
    b(Char('?'), "?", Some(("?", "keys")), None),
    b(Esc, "esc", Some(("esc", "back")), Some("back to the task list")),
];

/// The tags screen (`t`).
pub const TAGS: &[Binding] = &[
    b(Down, "↑↓/jk", Some(("↑↓/jk", "select")), None),
    b(Char('r'), "r", Some(("r", "rename")), Some("rename the tag on every task (an existing name merges them)")),
    b(Char('c'), "c", Some(("c/C", "color")), Some("next color")),
    b(Char('C'), "C", None, Some("previous color")),
    b(Char('x'), "x", Some(("x", "delete")), Some("remove the tag from every task (asks first)")),
    b(Char('?'), "?", Some(("?", "keys")), None),
    b(Esc, "esc", Some(("esc", "back")), Some("back to the task list")),
];

/// Splits a footer hint into (before, key, after) for mnemonic display:
/// ("n", "new") → ("", "n", "ew"), ("c", "new comment") → ("new ", "c", "omment").
/// `None` when the label doesn't contain the key (case-sensitive): then show "(x) delete".
pub fn mnemonic(key: &'static str, label: &'static str) -> Option<(&'static str, &'static str, &'static str)> {
    // Only single-character keys can be marked inside a word ("enter" and "↑↓/jk" can't).
    let mut chars = key.chars();
    if let (Some(k), None) = (chars.next(), chars.next()) {
        // Prefer the key at the start of a word ("new (c)omment"), else anywhere in the label.
        let at_word_start = label.char_indices().find(|&(i, c)| c == k && (i == 0 || label[..i].ends_with(' ')));
        if let Some((i, c)) = at_word_start.or_else(|| label.char_indices().find(|&(_, c)| c == k)) {
            let end = i + c.len_utf8();
            return Some((&label[..i], &label[i..end], &label[end..]));
        }
    }
    None
}

/// Bindings that show up in the `?` menu, in order.
pub fn menu(bindings: &'static [Binding]) -> Vec<&'static Binding> {
    bindings.iter().filter(|b| b.menu.is_some()).collect()
}

#[cfg(test)]
#[path = "../tests/keys.rs"]
mod tests;
