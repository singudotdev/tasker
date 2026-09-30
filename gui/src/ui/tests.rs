use super::*;
use tasker_core::app::Mode;
use tasker_core::model::{Status, Task, now};
use tasker_core::store::Store;

#[test]
fn keystrokes_become_terminal_keys() {
    let key = |s: &str| key_events(&Keystroke::parse(s).unwrap());
    let char = |c| vec![KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)];
    assert_eq!(key("n"), char('n'));
    assert_eq!(key("shift-r"), char('R'));
    assert_eq!(key("space"), char(' '));
    assert_eq!(key("escape"), [KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)]);
    assert_eq!(key("ctrl-c"), [KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)]);
    assert_eq!(key("shift-left"), [KeyEvent::new(KeyCode::Left, KeyModifiers::SHIFT)]);
    assert!(key("shift-tab").is_empty());
    assert!(key("cmd-q").is_empty());
}

#[test]
fn truncating() {
    assert_eq!(truncate("short", 10), "short");
    assert_eq!(truncate("a longer title", 8), "a longe…");
}

#[gpui::test]
fn keys_reach_the_app(cx: &mut gpui::TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::at(dir.path()).unwrap();
    store.save(&mut Task::new(1, "a".into(), now())).unwrap();
    let app = App::new(store).unwrap();

    let (view, cx) = cx.add_window_view(|window, cx| TaskerView::new(app, window, cx));
    // "s" and "q" are list keys; inside the form they're just letters.
    cx.simulate_keystrokes("n s q tab x enter");
    view.read_with(cx, |view, _| {
        assert!(matches!(view.app.mode, Mode::Normal));
        let titles: Vec<&str> = view.app.tasks.iter().map(|t| t.title.as_str()).collect();
        assert!(titles.contains(&"sq"), "{titles:?}");
    });
    cx.simulate_keystrokes("down s");
    view.read_with(cx, |view, _| {
        let doing: Vec<&str> =
            view.app.tasks.iter().filter(|t| t.status == Status::Doing).map(|t| t.title.as_str()).collect();
        assert_eq!(doing.len(), 1, "s started the selected task");
    });
}

#[test]
fn shortcuts_are_named_for_tooltips() {
    assert_eq!(key_name(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE)), "n");
    assert_eq!(key_name(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)), "Esc");
    assert_eq!(key_name(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)), "Ctrl+C");
}

#[gpui::test]
fn clipboard_shortcuts_copy_cut_and_paste(cx: &mut gpui::TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let app = App::new(Store::at(dir.path()).unwrap()).unwrap();
    let (view, cx) = cx.add_window_view(|window, cx| TaskerView::new(app, window, cx));
    let ctrl = if cfg!(target_os = "macos") { "cmd" } else { "ctrl" };
    // Type a title, select it all, cut it, and paste it twice.
    cx.simulate_keystrokes("n a b");
    cx.simulate_keystrokes(&format!("{ctrl}-a {ctrl}-x"));
    assert_eq!(cx.read_from_clipboard().and_then(|item| item.text()).as_deref(), Some("ab"));
    cx.simulate_keystrokes(&format!("{ctrl}-v {ctrl}-v shift-left shift-left {ctrl}-c"));
    assert_eq!(cx.read_from_clipboard().and_then(|item| item.text()).as_deref(), Some("ab"));
    cx.simulate_keystrokes("enter");
    view.read_with(cx, |view, _| {
        let titles: Vec<&str> = view.app.tasks.iter().map(|t| t.title.as_str()).collect();
        assert_eq!(titles, ["abab"]);
    });
}
