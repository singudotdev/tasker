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
