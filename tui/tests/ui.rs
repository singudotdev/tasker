use super::*;

#[test]
fn wrapping() {
    assert_eq!(wrap("one two three", 7), ["one two", "three"]);
    assert_eq!(wrap("abcdefghij", 4), ["abcd", "efgh", "ij"]);
    assert_eq!(wrap("a\n\nb", 10), ["a", "", "b"]);
}

#[test]
fn truncating() {
    assert_eq!(truncate("short", 10), "short");
    assert_eq!(truncate("a longer title", 8), "a longe…");
}

#[test]
fn popup_is_centered_and_clamped() {
    let area = Rect::new(0, 0, 100, 40);
    assert_eq!(popup(area, 60, 10), Rect::new(20, 15, 60, 10));
    assert_eq!(popup(Rect::new(0, 0, 30, 5), 60, 10), Rect::new(0, 0, 30, 5));
}
