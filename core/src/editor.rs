//! Text editing with a cursor and a selection:
//! - [`TextEditor`]: multi-line (task descriptions and comments): insert, delete, move, soft wrap.
//! - [`LineInput`]: one line (titles, tags, tag names, search): insert, delete, move, horizontal scroll.
//!
//! Moving with shift held, `select_all`, or the mouse (`point`) selects text (only the desktop app does any);
//! typing or pasting replaces the selection, and `backspace`/`delete` erase it.

use crate::event::{KeyCode, KeyEvent, KeyModifiers};
use std::cell::Cell;

/// The text being edited, line by line, with a cursor.
#[derive(Debug)]
pub struct TextEditor {
    /// The text, one entry per line (without newlines).
    lines: Vec<String>,
    /// Cursor: line index and char index within the line.
    row: usize,
    /// Cursor column, in characters (not bytes) within the line.
    col: usize,
    /// Where the selection started, as (line, column); it runs to the cursor. `None` when nothing is selected.
    anchor: Option<(usize, usize)>,
    /// The word or line a double- or triple-click selected, while a drag or shift+click can still grow it.
    grab: Option<Grab<(usize, usize)>>,
    /// First visible display row; kept in view by `layout`.
    scroll: Cell<usize>,
    /// Whether the text changed since the editor opened.
    pub modified: bool,
}

/// What a mouse press or drag does at a position in the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pointer {
    /// A click: moves the cursor there and drops the selection.
    Place,
    /// A drag or shift+click: selects from where the selection started, or the cursor, to there.
    Extend,
    /// A double-click: selects the word there.
    Word,
    /// A triple-click: selects the line there (all of a one-line input).
    Line,
}

/// What a key press means for the editor's owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Keep editing.
    Continue,
    /// Close and save (`esc`).
    Save,
}

impl TextEditor {
    /// An editor with `text`, the cursor at its end.
    pub fn new(text: &str) -> Self {
        let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
        if lines.is_empty() {
            lines.push(String::new());
        }
        // Start at the end, ready to append.
        let row = lines.len() - 1;
        let col = lines[row].chars().count();
        Self { lines, row, col, anchor: None, grab: None, scroll: Cell::new(0), modified: false }
    }

    /// The text, without trailing whitespace.
    pub fn text(&self) -> String {
        self.lines.join("\n").trim_end().to_string()
    }

    /// Length of line `row`, in characters.
    fn line_len(&self, row: usize) -> usize {
        self.lines[row].chars().count()
    }

    /// Byte offset of char index `col` in line `row`.
    /// Strings are indexed by byte but the cursor counts characters (`ñ` is two bytes).
    fn byte(&self, row: usize, col: usize) -> usize {
        // Past the last character means the end of the line.
        self.lines[row].char_indices().nth(col).map_or(self.lines[row].len(), |(i, _)| i)
    }

    /// Applies one key press: typing, deleting, moving (selecting with shift), or `esc` to save.
    pub fn on_key(&mut self, key: KeyEvent) -> Outcome {
        // Ctrl combinations are not text (ctrl+c is handled by the app: discard).
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Esc => return Outcome::Save,
            KeyCode::Char(c) if !ctrl => self.insert(&c.to_string()),
            KeyCode::Tab => self.insert("  "),
            // Split the line at the cursor; the rest moves to a new line below.
            KeyCode::Enter => self.insert("\n"),
            KeyCode::Backspace | KeyCode::Delete if self.delete_selection() => {}
            KeyCode::Backspace => {
                if self.col > 0 {
                    let (a, b) = (self.byte(self.row, self.col - 1), self.byte(self.row, self.col));
                    self.lines[self.row].drain(a..b);
                    self.col -= 1;
                    self.modified = true;
                } else if self.row > 0 {
                    // At the start of a line: join it onto the end of the previous one.
                    let line = self.lines.remove(self.row);
                    self.row -= 1;
                    self.col = self.line_len(self.row);
                    self.lines[self.row].push_str(&line);
                    self.modified = true;
                }
            }
            KeyCode::Delete => {
                if self.col < self.line_len(self.row) {
                    let (a, b) = (self.byte(self.row, self.col), self.byte(self.row, self.col + 1));
                    self.lines[self.row].drain(a..b);
                    self.modified = true;
                } else if self.row + 1 < self.lines.len() {
                    // At the end of a line: pull the next line up onto this one.
                    let next = self.lines.remove(self.row + 1);
                    self.lines[self.row].push_str(&next);
                    self.modified = true;
                }
            }
            code => self.move_cursor(code, key.modifiers.contains(KeyModifiers::SHIFT)),
        }
        Outcome::Continue
    }

    /// Moves the cursor for an arrow, `home` or `end`; with `select`, the selection grows or shrinks with it.
    /// Without, `←`/`→` over a selection go to its start/end, and any move drops it.
    fn move_cursor(&mut self, code: KeyCode, select: bool) {
        let selection = self.selection();
        self.grab = None;
        self.anchor = if select { self.anchor.or(Some((self.row, self.col))) } else { None };
        match (code, selection) {
            (KeyCode::Left, Some((start, _))) if !select => (self.row, self.col) = start,
            (KeyCode::Right, Some((_, end))) if !select => (self.row, self.col) = end,
            (KeyCode::Left, _) => {
                if self.col > 0 {
                    self.col -= 1;
                } else if self.row > 0 {
                    // Wrap to the end of the previous line.
                    self.row -= 1;
                    self.col = self.line_len(self.row);
                }
            }
            (KeyCode::Right, _) => {
                if self.col < self.line_len(self.row) {
                    self.col += 1;
                } else if self.row + 1 < self.lines.len() {
                    // Wrap to the start of the next line.
                    self.row += 1;
                    self.col = 0;
                }
            }
            (KeyCode::Up, _) if self.row > 0 => {
                self.row -= 1;
                self.clamp_col();
            }
            (KeyCode::Down, _) if self.row + 1 < self.lines.len() => {
                self.row += 1;
                self.clamp_col();
            }
            (KeyCode::Home, _) => self.col = 0,
            (KeyCode::End, _) => self.col = self.line_len(self.row),
            _ => {}
        }
    }

    /// Inserts `text` at the cursor, in place of the selection if any, and moves the cursor after it.
    /// Newlines in `text` start new lines (for pasting); `\r` and other control characters but tabs are dropped.
    pub fn insert(&mut self, text: &str) {
        self.delete_selection();
        let text: String = text.chars().filter(|&c| c == '\n' || c == '\t' || !c.is_control()).collect();
        if text.is_empty() {
            return;
        }
        let at = self.byte(self.row, self.col);
        let rest = self.lines[self.row].split_off(at);
        for (i, piece) in text.split('\n').enumerate() {
            if i == 0 {
                self.lines[self.row].push_str(piece);
                self.col += piece.chars().count();
            } else {
                self.row += 1;
                self.lines.insert(self.row, piece.to_string());
                self.col = piece.chars().count();
            }
        }
        self.lines[self.row].push_str(&rest);
        self.modified = true;
    }

    /// Selects the whole text, the cursor at its end.
    pub fn select_all(&mut self) {
        self.grab = None;
        self.anchor = Some((0, 0));
        self.row = self.lines.len() - 1;
        self.col = self.line_len(self.row);
    }

    /// A mouse press or drag at `(row, col)`; positions past the end of a line or the text mean its end.
    pub fn point(&mut self, (row, col): (usize, usize), how: Pointer) {
        let row = row.min(self.lines.len() - 1);
        let col = col.min(self.line_len(row));
        let unit = |by| match by {
            Pointer::Word => {
                let (start, end) = word_at(&self.lines[row], col);
                ((row, start), (row, end))
            }
            _ => ((row, 0), (row, self.line_len(row))),
        };
        let (anchor, cursor) = match (how, self.grab) {
            (Pointer::Place, _) => (None, (row, col)),
            (Pointer::Extend, Some(grab)) => {
                let (anchor, cursor) = grab.grow(unit(grab.by));
                (Some(anchor), cursor)
            }
            (Pointer::Extend, None) => (self.anchor.or(Some((self.row, self.col))), (row, col)),
            (by @ (Pointer::Word | Pointer::Line), _) => {
                let (start, end) = unit(by);
                self.anchor = Some(start);
                (self.row, self.col) = end;
                self.grab = Some(Grab { by, start, end });
                return;
            }
        };
        if how == Pointer::Place {
            self.grab = None;
        }
        self.anchor = anchor;
        (self.row, self.col) = cursor;
    }

    /// The selection as (start, end), each a (line, column), start first; `None` when nothing is selected.
    pub fn selection(&self) -> Option<((usize, usize), (usize, usize))> {
        let anchor = self.anchor.filter(|&a| a != (self.row, self.col))?;
        Some((anchor.min((self.row, self.col)), anchor.max((self.row, self.col))))
    }

    /// The selected text, lines joined with newlines.
    pub fn selected_text(&self) -> Option<String> {
        let ((r1, c1), (r2, c2)) = self.selection()?;
        if r1 == r2 {
            return Some(self.lines[r1][self.byte(r1, c1)..self.byte(r2, c2)].to_string());
        }
        let mut text = self.lines[r1][self.byte(r1, c1)..].to_string();
        for line in &self.lines[r1 + 1..r2] {
            text.push('\n');
            text.push_str(line);
        }
        text.push('\n');
        text.push_str(&self.lines[r2][..self.byte(r2, c2)]);
        Some(text)
    }

    /// Removes the selected text, leaving the cursor where it started. Returns whether there was a selection.
    pub fn delete_selection(&mut self) -> bool {
        self.grab = None;
        let Some(((r1, c1), (r2, c2))) = self.selection() else {
            self.anchor = None;
            return false;
        };
        let tail = self.lines[r2][self.byte(r2, c2)..].to_string();
        let at = self.byte(r1, c1);
        self.lines[r1].truncate(at);
        self.lines[r1].push_str(&tail);
        self.lines.drain(r1 + 1..=r2);
        (self.row, self.col) = (r1, c1);
        self.anchor = None;
        self.modified = true;
        true
    }

    /// Keeps the cursor inside the line after moving up or down.
    fn clamp_col(&mut self) {
        self.col = self.col.min(self.line_len(self.row));
    }

    /// The text, one entry per line.
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// The cursor: line index and character column.
    pub fn cursor(&self) -> (usize, usize) {
        (self.row, self.col)
    }

    /// Soft-wraps the text to `width` columns and keeps the cursor within `height` rows.
    /// Returns the visible rows and the cursor's (x, y) inside them.
    pub fn layout(&self, width: usize, height: usize) -> (Vec<String>, (usize, usize)) {
        let width = width.max(1);
        let mut rows = Vec::new();
        let mut cursor = (0, 0);
        for (i, line) in self.lines.iter().enumerate() {
            let chars: Vec<char> = line.chars().collect();
            if i == self.row {
                // A cursor at the very end of a full row goes to the start of the next one.
                cursor = (self.col % width, rows.len() + self.col / width);
            }
            if chars.is_empty() {
                rows.push(String::new());
            }
            for chunk in chars.chunks(width) {
                rows.push(chunk.iter().collect());
            }
            if i == self.row && !chars.is_empty() && self.col == chars.len() && self.col.is_multiple_of(width) {
                rows.push(String::new());
            }
        }

        let height = height.max(1);
        let mut scroll = self.scroll.get();
        if cursor.1 < scroll {
            scroll = cursor.1;
        } else if cursor.1 >= scroll + height {
            scroll = cursor.1 + 1 - height;
        }
        self.scroll.set(scroll);
        let visible = rows.into_iter().skip(scroll).take(height).collect();
        (visible, (cursor.0, cursor.1 - scroll))
    }
}

/// A one-line text input with a cursor.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LineInput {
    /// The typed text.
    text: String,
    /// Cursor position, in characters (not bytes).
    cursor: usize,
    /// Where the selection started, in characters; it runs to the cursor. `None` when nothing is selected.
    anchor: Option<usize>,
    /// The word or line a double- or triple-click selected, while a drag or shift+click can still grow it.
    grab: Option<Grab<usize>>,
}

impl LineInput {
    /// An input holding `text`, the cursor at its end.
    pub fn new(text: &str) -> Self {
        Self { text: text.to_string(), cursor: text.chars().count(), anchor: None, grab: None }
    }

    /// The typed text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Whether nothing is typed.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Empties the input.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Byte offset of char index `at`; past the last character means the end.
    fn byte(&self, at: usize) -> usize {
        self.text.char_indices().nth(at).map_or(self.text.len(), |(i, _)| i)
    }

    /// Typing, `backspace`/`delete`, and moving with `←`/`→`/`home`/`end` (selecting with shift).
    /// Other keys are ignored. Returns whether the text changed.
    pub fn on_key(&mut self, key: KeyEvent) -> bool {
        let len = self.text.chars().count();
        match key.code {
            // Ctrl combinations are not text.
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => self.insert(&c.to_string()),
            KeyCode::Backspace | KeyCode::Delete if self.delete_selection() => true,
            KeyCode::Backspace if self.cursor > 0 => {
                self.cursor -= 1;
                let at = self.byte(self.cursor);
                self.text.remove(at);
                true
            }
            KeyCode::Delete if self.cursor < len => {
                let at = self.byte(self.cursor);
                self.text.remove(at);
                true
            }
            code @ (KeyCode::Left | KeyCode::Right | KeyCode::Home | KeyCode::End) => {
                let select = key.modifiers.contains(KeyModifiers::SHIFT);
                // Without shift, ←/→ over a selection go to its start/end, and any move drops it.
                let to = match (code, self.selection()) {
                    (KeyCode::Left, Some((start, _))) if !select => start,
                    (KeyCode::Right, Some((_, end))) if !select => end,
                    (KeyCode::Left, _) => self.cursor.saturating_sub(1),
                    (KeyCode::Right, _) => (self.cursor + 1).min(len),
                    (KeyCode::Home, _) => 0,
                    _ => len,
                };
                self.grab = None;
                self.anchor = if select { self.anchor.or(Some(self.cursor)) } else { None };
                self.cursor = to;
                false
            }
            _ => false,
        }
    }

    /// Inserts `text` at the cursor, in place of the selection if any (for typing and pasting).
    /// Newlines and tabs become spaces and other control characters are dropped: the input is one line.
    /// Returns whether the text changed.
    pub fn insert(&mut self, text: &str) -> bool {
        let deleted = self.delete_selection();
        let text: String = text
            .chars()
            .filter_map(|c| match c {
                '\n' | '\t' => Some(' '),
                '\r' => None,
                c if c.is_control() => None,
                c => Some(c),
            })
            .collect();
        let at = self.byte(self.cursor);
        self.text.insert_str(at, &text);
        self.cursor += text.chars().count();
        deleted || !text.is_empty()
    }

    /// Selects the whole text, the cursor at its end.
    pub fn select_all(&mut self) {
        self.grab = None;
        self.anchor = Some(0);
        self.cursor = self.text.chars().count();
    }

    /// A mouse press or drag at character `col`; past the end means the end.
    pub fn point(&mut self, col: usize, how: Pointer) {
        let len = self.text.chars().count();
        let col = col.min(len);
        let unit = |by| if by == Pointer::Word { word_at(&self.text, col) } else { (0, len) };
        let (anchor, cursor) = match (how, self.grab) {
            (Pointer::Place, _) => (None, col),
            (Pointer::Extend, Some(grab)) => {
                let (anchor, cursor) = grab.grow(unit(grab.by));
                (Some(anchor), cursor)
            }
            (Pointer::Extend, None) => (self.anchor.or(Some(self.cursor)), col),
            (by @ (Pointer::Word | Pointer::Line), _) => {
                let (start, end) = unit(by);
                (self.anchor, self.cursor) = (Some(start), end);
                self.grab = Some(Grab { by, start, end });
                return;
            }
        };
        if how == Pointer::Place {
            self.grab = None;
        }
        (self.anchor, self.cursor) = (anchor, cursor);
    }

    /// The selection as a range of characters (start, end); `None` when nothing is selected.
    pub fn selection(&self) -> Option<(usize, usize)> {
        let anchor = self.anchor.filter(|&a| a != self.cursor)?;
        Some((anchor.min(self.cursor), anchor.max(self.cursor)))
    }

    /// The selected text.
    pub fn selected_text(&self) -> Option<&str> {
        let (start, end) = self.selection()?;
        Some(&self.text[self.byte(start)..self.byte(end)])
    }

    /// Removes the selected text, leaving the cursor where it started. Returns whether there was a selection.
    pub fn delete_selection(&mut self) -> bool {
        self.grab = None;
        let selection = self.selection();
        self.anchor = None;
        let Some((start, end)) = selection else { return false };
        let range = self.byte(start)..self.byte(end);
        self.text.drain(range);
        self.cursor = start;
        true
    }

    /// The cursor position, in characters.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// The part of the text that fits in `width` columns, scrolled so the cursor is visible,
    /// and the cursor's column within it.
    pub fn view(&self, width: usize) -> (String, usize) {
        let width = width.max(1);
        // The cursor needs a column of its own, after the last character.
        let start = self.cursor.saturating_sub(width - 1);
        (self.text.chars().skip(start).take(width).collect(), self.cursor - start)
    }
}

/// What a double- or triple-click selected (`by` a word or a line, from `start` to `end`), so that dragging on
/// selects whole words or lines, always including it: a desktop text field's convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Grab<P> {
    /// `Pointer::Word` or `Pointer::Line`.
    by: Pointer,
    /// Where the word or line starts.
    start: P,
    /// Where it ends.
    end: P,
}

impl<P: Ord + Copy> Grab<P> {
    /// The selection as (anchor, cursor) when the mouse is over the word or line `(start, end)`: from the grabbed
    /// one to that one, whichever side it's on.
    fn grow(self, (start, end): (P, P)) -> (P, P) {
        if start < self.start { (self.end, start) } else { (self.start, end.max(self.end)) }
    }
}

/// The run of word characters, spaces or punctuation around character `col` of `line`, as (start, end).
/// At the end of the line it's the run before it, so double-clicking past the last word selects that word.
fn word_at(line: &str, col: usize) -> (usize, usize) {
    let kind = |c: char| {
        if c.is_alphanumeric() || c == '_' {
            0
        } else if c.is_whitespace() {
            1
        } else {
            2
        }
    };
    let chars: Vec<char> = line.chars().collect();
    let Some(&c) = chars.get(col).or_else(|| chars.last()) else { return (0, 0) };
    let col = col.min(chars.len() - 1);
    let start = chars[..col].iter().rposition(|&d| kind(d) != kind(c)).map_or(0, |i| i + 1);
    let end = chars[col..].iter().position(|&d| kind(d) != kind(c)).map_or(chars.len(), |i| col + i);
    (start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn typing(ed: &mut TextEditor, s: &str) {
        for c in s.chars() {
            ed.on_key(key(if c == '\n' { KeyCode::Enter } else { KeyCode::Char(c) }));
        }
    }

    #[test]
    fn edit_lines() {
        let mut ed = TextEditor::new("first");
        typing(&mut ed, " line\nsecond\nthird");
        assert_eq!(ed.text(), "first line\nsecond\nthird");
        // Join "third" back onto "second" with backspace at line start.
        ed.on_key(key(KeyCode::Home));
        ed.on_key(key(KeyCode::Backspace));
        assert_eq!(ed.text(), "first line\nsecondthird");
        // Delete at end of line joins the next one.
        ed.on_key(key(KeyCode::Up));
        ed.on_key(key(KeyCode::End));
        ed.on_key(key(KeyCode::Delete));
        assert_eq!(ed.text(), "first linesecondthird");
        assert!(ed.modified);
    }

    #[test]
    fn unicode() {
        let mut ed = TextEditor::new("");
        typing(&mut ed, "añadir");
        // Delete the "r" at the end.
        ed.on_key(key(KeyCode::Backspace));
        assert_eq!(ed.text(), "añadi");
        // Multi-byte char in the middle of the line.
        ed.on_key(key(KeyCode::Home));
        ed.on_key(key(KeyCode::Right));
        ed.on_key(key(KeyCode::Right));
        ed.on_key(key(KeyCode::Backspace));
        assert_eq!(ed.text(), "aadi");
    }

    #[test]
    fn wrap_and_cursor() {
        let mut ed = TextEditor::new("");
        typing(&mut ed, "abcdefgh");
        let (rows, cursor) = ed.layout(3, 10);
        assert_eq!(rows, ["abc", "def", "gh"]);
        assert_eq!(cursor, (2, 2));
        ed.on_key(key(KeyCode::Char('i')));
        let (rows, cursor) = ed.layout(3, 10);
        assert_eq!(rows, ["abc", "def", "ghi", ""]);
        assert_eq!(cursor, (0, 3));
        // Only two rows fit: the view scrolls to keep the cursor visible.
        let (rows, cursor) = ed.layout(3, 2);
        assert_eq!(rows, ["ghi", ""]);
        assert_eq!(cursor, (0, 1));
    }

    fn line_input(text: &str, keys: &[KeyCode]) -> LineInput {
        let mut input = LineInput::new(text);
        for &code in keys {
            input.on_key(key(code));
        }
        input
    }

    #[test]
    fn line_input_edits_at_the_cursor() {
        use KeyCode::{Backspace, Char, Delete, End, Home, Left, Right};
        let input = line_input("prod supprt", &[Left, Left, Char('o'), Home, Delete, Char('P'), End, Backspace]);
        assert_eq!(input.text(), "Prod suppor");
        // Unicode: the cursor counts characters, not bytes.
        let input = line_input("añadir", &[Home, Right, Right, Backspace, Right, Right, Right, Right, Delete]);
        assert_eq!(input.text(), "aadir");
        // Moving never goes past either end.
        let input = line_input("ab", &[Right, Right, Char('c'), Home, Left, Char('_')]);
        assert_eq!(input.text(), "_abc");
    }

    fn shift(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::SHIFT)
    }

    #[test]
    fn editor_selects_copies_and_replaces() {
        let mut ed = TextEditor::new("one\ntwo\nthree");
        ed.on_key(key(KeyCode::Up));
        ed.on_key(key(KeyCode::Home));
        ed.on_key(key(KeyCode::Right));
        ed.on_key(shift(KeyCode::Down));
        ed.on_key(shift(KeyCode::Right));
        assert_eq!(ed.selected_text().as_deref(), Some("wo\nth"));
        // Typing replaces the selection.
        ed.on_key(key(KeyCode::Char('X')));
        assert_eq!(ed.text(), "one\ntXree");
        assert_eq!(ed.selected_text(), None);
        // Pasting several lines over everything.
        ed.select_all();
        assert_eq!(ed.selected_text().as_deref(), Some("one\ntXree"));
        ed.insert("a\r\nb\nc");
        assert_eq!(ed.text(), "a\nb\nc");
        assert_eq!(ed.cursor(), (2, 1));
        // Backspace erases just the selection; ← without shift goes to its start.
        ed.on_key(shift(KeyCode::Up));
        ed.on_key(key(KeyCode::Backspace));
        assert_eq!(ed.text(), "a\nb");
        ed.on_key(shift(KeyCode::Home));
        ed.on_key(key(KeyCode::Left));
        assert_eq!((ed.cursor(), ed.selection()), ((1, 0), None));
    }

    #[test]
    fn line_input_selects_copies_and_replaces() {
        let mut input = LineInput::new("hello world");
        input.on_key(shift(KeyCode::Left));
        input.on_key(shift(KeyCode::Left));
        assert_eq!(input.selected_text(), Some("ld"));
        assert!(input.on_key(key(KeyCode::Delete)));
        assert_eq!(input.text(), "hello wor");
        input.select_all();
        assert_eq!(input.selected_text(), Some("hello wor"));
        // A pasted line break becomes a space: the input is one line.
        assert!(input.insert("a\r\nb"));
        assert_eq!((input.text(), input.cursor()), ("a b", 3));
        input.on_key(shift(KeyCode::Home));
        input.on_key(key(KeyCode::Right));
        assert_eq!((input.cursor(), input.selection()), (3, None));
    }

    #[test]
    fn the_mouse_places_the_cursor_and_selects() {
        let mut ed = TextEditor::new("one two\nthree, four");
        ed.point((0, 2), Pointer::Place);
        ed.point((1, 3), Pointer::Extend);
        assert_eq!(ed.selected_text().as_deref(), Some("e two\nthr"));
        // Double-click: the word, the spaces or the punctuation under the mouse; past the end, the last word.
        ed.point((1, 7), Pointer::Word);
        assert_eq!(ed.selected_text().as_deref(), Some("four"));
        ed.point((1, 5), Pointer::Word);
        assert_eq!(ed.selected_text().as_deref(), Some(","));
        ed.point((0, 99), Pointer::Word);
        assert_eq!(ed.selected_text().as_deref(), Some("two"));
        ed.point((1, 0), Pointer::Line);
        assert_eq!(ed.selected_text().as_deref(), Some("three, four"));
        // Positions past the text are its end; a click drops the selection.
        ed.point((9, 99), Pointer::Place);
        assert_eq!((ed.cursor(), ed.selection()), ((1, 11), None));

        let mut input = LineInput::new("añadir tag");
        input.point(8, Pointer::Place);
        input.point(2, Pointer::Extend);
        assert_eq!(input.selected_text(), Some("adir t"));
        input.point(0, Pointer::Word);
        assert_eq!(input.selected_text(), Some("añadir"));
        input.point(3, Pointer::Line);
        assert_eq!(input.selected_text(), Some("añadir tag"));
        assert_eq!(word_at("", 0), (0, 0));
    }

    #[test]
    fn dragging_after_a_double_or_triple_click_selects_whole_words_or_lines() {
        let mut input = LineInput::new("one two three");
        input.point(5, Pointer::Word);
        input.point(10, Pointer::Extend);
        assert_eq!(input.selected_text(), Some("two three"));
        // Back past where it started: the double-clicked word stays selected, the anchor moves to its end.
        input.point(1, Pointer::Extend);
        assert_eq!((input.selected_text(), input.cursor()), (Some("one two"), 0));
        input.point(5, Pointer::Extend);
        assert_eq!(input.selected_text(), Some("two"));
        // A plain click or a key ends it: dragging is by characters again.
        input.point(5, Pointer::Place);
        input.point(10, Pointer::Extend);
        assert_eq!(input.selected_text(), Some("wo th"));

        let mut ed = TextEditor::new("one\ntwo\nthree");
        ed.point((1, 1), Pointer::Line);
        ed.point((0, 2), Pointer::Extend);
        assert_eq!((ed.selected_text().as_deref(), ed.cursor()), (Some("one\ntwo"), (0, 0)));
        ed.point((2, 1), Pointer::Extend);
        assert_eq!(ed.selected_text().as_deref(), Some("two\nthree"));
        // After a key, a drag moves only the cursor end, by characters, from the same anchor.
        ed.on_key(shift(KeyCode::Left));
        ed.point((0, 1), Pointer::Extend);
        assert_eq!(ed.selected_text().as_deref(), Some("ne\n"));
    }

    #[test]
    fn line_input_scrolls_to_keep_the_cursor_visible() {
        let mut input = LineInput::new("abcdefgh");
        assert_eq!(input.view(4), ("fgh".to_string(), 3), "at the end: the tail, cursor after it");
        input.on_key(key(KeyCode::Home));
        assert_eq!(input.view(4), ("abcd".to_string(), 0));
        assert_eq!(LineInput::new("ab").view(10), ("ab".to_string(), 2));
    }
}
