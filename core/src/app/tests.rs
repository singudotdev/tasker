//! Behaviour tests: drive the app with key presses against a temporary data folder.

use super::*;
use crate::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::model::{Status, now};
use crate::store::Store;
use tempfile::TempDir;

/// An app over a fresh folder holding `tasks`; the folder lives as long as the fixture.
struct Fixture {
    app: App,
    dir: TempDir,
}

impl Fixture {
    fn new(tasks: Vec<Task>) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::at(dir.path()).unwrap();
        for mut task in tasks {
            store.save(&mut task).unwrap();
        }
        let mut app = App::new(Store::at(dir.path()).unwrap()).unwrap();
        app.mode = Mode::Normal;
        Self { app, dir }
    }

    fn with_titles(titles: &[&str]) -> Self {
        let tasks = titles.iter().zip(1..).map(|(title, id)| Task::new(id, (*title).to_string(), now())).collect();
        Self::new(tasks)
    }

    fn press(&mut self, code: KeyCode) {
        self.app.on_key(KeyEvent::new(code, KeyModifiers::NONE));
    }

    fn ctrl(&mut self, c: char) {
        self.app.on_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL));
    }

    fn type_text(&mut self, text: &str) {
        text.chars().for_each(|c| self.press(KeyCode::Char(c)));
    }

    fn select(&mut self, id: u32) {
        self.app.select_id(id);
        assert_eq!(self.app.selected_task().map(|t| t.id), Some(id));
    }

    fn task(&self, id: u32) -> &Task {
        self.app.task(id).unwrap()
    }

    /// What's on disk, independent of the app's in-memory state.
    fn reloaded(&self) -> Vec<Task> {
        let (mut tasks, warnings) = Store::at(self.dir.path()).unwrap().load().unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        tasks.sort_by_key(|t| t.id);
        tasks
    }
}

#[test]
fn status_keys_move_tasks_and_record_when() {
    let mut f = Fixture::with_titles(&["feature", "bug"]);
    f.select(1);
    f.press(KeyCode::Char('s'));
    f.select(2);
    f.press(KeyCode::Char('s'));
    assert_eq!(f.app.tasks.iter().filter(|t| t.status == Status::Doing).count(), 2, "several can be doing");

    f.select(1);
    f.press(KeyCode::Char('d'));
    let on_disk = &f.reloaded()[0];
    assert_eq!(on_disk.status, Status::Done);
    let steps: Vec<(Status, Status)> = on_disk.history.iter().map(|c| (c.from, c.to)).collect();
    assert_eq!(steps, [(Status::Todo, Status::Doing), (Status::Doing, Status::Done)]);
    assert!(on_disk.history.iter().all(|c| c.at >= on_disk.created));
    assert_eq!(f.app.visible().len(), 1, "done tasks are hidden by default");

    f.press(KeyCode::Char('f'));
    f.select(1);
    f.press(KeyCode::Char('b'));
    assert_eq!(f.task(1).status, Status::Todo);
    f.press(KeyCode::Char('b'));
    assert_eq!(f.task(1).history.len(), 3, "no change, nothing recorded");
}

#[test]
fn parked_tasks_stay_in_the_list_after_todo() {
    let mut f = Fixture::with_titles(&["feature", "bug"]);
    f.select(1);
    f.press(KeyCode::Char('s'));
    f.press(KeyCode::Char('p'));
    let on_disk = &f.reloaded()[0];
    assert_eq!(on_disk.status, Status::Parked);
    assert_eq!(on_disk.history.last().map(|c| (c.from, c.to)), Some((Status::Doing, Status::Parked)));
    let order: Vec<u32> = f.app.visible().iter().map(|&i| f.app.tasks[i].id).collect();
    assert_eq!(order, [2, 1], "todo before parked, and parked isn't hidden");
}

#[test]
fn form_fields_edit_at_the_cursor() {
    let mut f = Fixture::with_titles(&["fix login"]);
    f.press(KeyCode::Char('e'));
    f.press(KeyCode::Home);
    f.type_text("PROD ");
    f.press(KeyCode::Down);
    f.type_text("prod suport");
    (0..3).for_each(|_| f.press(KeyCode::Left));
    f.type_text("p");
    f.press(KeyCode::Home);
    f.press(KeyCode::Delete);
    f.type_text("P");
    f.press(KeyCode::Enter);
    let task = &f.reloaded()[0];
    assert_eq!(task.title, "PROD fix login");
    assert_eq!(task.tags, ["prod", "support"], "tags are lowercased on save");
}

#[test]
fn new_tasks_start_as_todo() {
    let mut f = Fixture::with_titles(&[]);
    f.press(KeyCode::Char('n'));
    f.type_text("write docs");
    f.press(KeyCode::Tab);
    f.type_text("#Docs");
    f.press(KeyCode::Enter);
    let task = &f.reloaded()[0];
    assert_eq!(
        (task.title.as_str(), task.status, task.tags.as_slice()),
        ("write docs", Status::Todo, &["docs".to_string()][..])
    );
    assert!(task.history.is_empty());
}

#[test]
fn status_keys_work_in_the_task_view() {
    let mut f = Fixture::with_titles(&["feature"]);
    f.press(KeyCode::Char('v'));
    f.press(KeyCode::Char('s'));
    assert!(matches!(f.app.mode, Mode::Issue(_)), "stays in the task view");
    assert_eq!(f.reloaded()[0].status, Status::Doing);
}

#[test]
fn only_y_confirms_a_delete() {
    let mut f = Fixture::with_titles(&["feature"]);
    f.press(KeyCode::Char('x'));
    f.press(KeyCode::Enter);
    assert_eq!(f.reloaded().len(), 1);
    f.press(KeyCode::Char('x'));
    f.press(KeyCode::Char('y'));
    assert!(f.reloaded().is_empty());
}

#[test]
fn comments_can_be_added_edited_and_deleted() {
    let mut f = Fixture::with_titles(&["feature"]);
    f.press(KeyCode::Char('c'));
    f.type_text("found it");
    f.press(KeyCode::Esc);
    assert_eq!(f.reloaded()[0].comments[0].body, "found it");

    f.press(KeyCode::Char('v'));
    f.press(KeyCode::Char('e'));
    f.type_text(", fixed");
    f.press(KeyCode::Esc);
    let comment = &f.reloaded()[0].comments[0];
    assert_eq!(comment.body, "found it, fixed");
    assert!(comment.edited.is_some());

    f.press(KeyCode::Char('x'));
    f.press(KeyCode::Char('y'));
    assert!(matches!(f.app.mode, Mode::Issue(_)), "stays in the task view");
    assert!(f.reloaded()[0].comments.is_empty());
}

#[test]
fn ctrl_c_discards_the_editor_and_does_not_quit() {
    let mut f = Fixture::with_titles(&["feature"]);
    f.press(KeyCode::Char('m'));
    f.type_text("draft");
    f.ctrl('c');
    assert!(matches!(f.app.mode, Mode::Normal));
    assert!(!f.app.quit);
    assert!(f.reloaded()[0].description.is_empty());

    f.ctrl('c');
    assert!(!f.app.quit, "ctrl+c doesn't quit from the list either");
}

#[test]
fn unlisted_keys_do_nothing() {
    let mut f = Fixture::with_titles(&["feature", "bug"]);
    f.select(1);
    for c in [' ', 'a', 'h', 'g', 'G', 'i', 'l', 'r', 'z', 'Q'] {
        f.press(KeyCode::Char(c));
        assert!(matches!(f.app.mode, Mode::Normal), "{c:?} changed the screen");
        assert_eq!(f.app.selected_task().map(|t| t.id), Some(1), "{c:?} moved the selection");
    }
    f.press(KeyCode::Delete);
    f.press(KeyCode::Home);
    f.press(KeyCode::Enter);
    assert!(matches!(f.app.mode, Mode::Normal));
    assert!(f.app.tasks.iter().all(|t| t.status == Status::Todo) && !f.app.quit);
}

#[test]
fn every_listed_list_key_does_something() {
    for binding in crate::keys::LIST.iter().filter(|b| b.code != KeyCode::Down) {
        let mut f = Fixture::with_titles(&["feature", "bug"]);
        let snapshot =
            |app: &App| (app.selected, app.done_view, app.quit, app.tasks.iter().map(|t| t.status).collect::<Vec<_>>());
        let before = snapshot(&f.app);
        f.press(binding.code);
        let changed_screen = !matches!(f.app.mode, Mode::Normal);
        let changed_state = snapshot(&f.app) != before || f.app.edit_request.is_some() || f.app.message().is_some();
        assert!(changed_screen || changed_state, "{} did nothing", binding.key);
    }
}

#[test]
fn search_filters_by_title_and_tag() {
    let mut fix = Task::new(1, "Fix login".into(), now());
    fix.tags = vec!["prod".into()];
    let mut f = Fixture::new(vec![fix, Task::new(2, "Write docs".into(), now())]);
    f.press(KeyCode::Char('/'));
    f.type_text("#pro");
    assert_eq!(f.app.visible().len(), 1);
    f.press(KeyCode::Esc);
    assert_eq!(f.app.visible().len(), 2);
}

/// Tasks 1–3 tagged `prod support`, `prod` and `ops`.
fn tagged() -> Fixture {
    let tags: [&[&str]; 3] = [&["prod", "support"], &["prod"], &["ops"]];
    let tasks = tags
        .iter()
        .zip(1..)
        .map(|(tags, id)| {
            let mut task = Task::new(id, format!("task {id}"), now());
            task.tags = tags.iter().map(|t| (*t).to_string()).collect();
            task
        })
        .collect();
    Fixture::new(tasks)
}

fn tags_on_disk(f: &Fixture) -> Vec<Vec<String>> {
    f.reloaded().into_iter().map(|t| t.tags).collect()
}

#[test]
fn renaming_a_tag_changes_every_task_and_keeps_its_color() {
    let mut f = tagged();
    let color = f.app.tag_colors.get("prod");
    f.press(KeyCode::Char('t'));
    f.press(KeyCode::Down); // ops, prod, support
    f.press(KeyCode::Char('r'));
    (0..4).for_each(|_| f.press(KeyCode::Backspace));
    f.type_text("#Live");
    f.press(KeyCode::Enter);

    assert_eq!(tags_on_disk(&f), [vec!["live", "support"], vec!["live"], vec!["ops"]]);
    assert_eq!(f.app.tag_colors.get("live"), color);
    assert_eq!(f.app.tag_colors.get("prod"), None);
    let tags_file = std::fs::read_to_string(f.dir.path().join("tags.md")).unwrap();
    assert!(tags_file.contains("- live:") && !tags_file.contains("- prod:"), "{tags_file}");
    assert!(matches!(f.app.mode, Mode::Tags(TagsView { sel: 0 })), "the renamed tag stays selected");
}

#[test]
fn renaming_to_an_existing_tag_merges_them() {
    let mut f = tagged();
    let support = f.app.tag_colors.get("support");
    f.press(KeyCode::Char('t'));
    f.press(KeyCode::Down);
    f.press(KeyCode::Char('r'));
    (0..4).for_each(|_| f.press(KeyCode::Backspace));
    f.type_text("support");
    f.press(KeyCode::Enter);

    assert_eq!(tags_on_disk(&f), [vec!["support"], vec!["support"], vec!["ops"]]);
    assert_eq!(f.app.tag_colors.get("support"), support);
}

#[test]
fn renaming_needs_exactly_one_tag_name() {
    let mut f = tagged();
    f.press(KeyCode::Char('t'));
    f.press(KeyCode::Char('r'));
    f.type_text(" two");
    f.press(KeyCode::Enter);
    assert!(matches!(f.app.mode, Mode::RenameTag(_)), "two words keep the dialog open");
    (0..7).for_each(|_| f.press(KeyCode::Backspace));
    f.press(KeyCode::Enter);
    assert!(matches!(f.app.mode, Mode::RenameTag(_)), "an empty name keeps the dialog open");
    f.press(KeyCode::Esc);
    assert!(matches!(f.app.mode, Mode::Tags(_)));
    assert_eq!(tags_on_disk(&f)[2], ["ops"]);
}

#[test]
fn deleting_a_tag_asks_then_removes_it_everywhere() {
    let mut f = tagged();
    f.press(KeyCode::Char('t'));
    f.press(KeyCode::Down);
    f.press(KeyCode::Char('x'));
    f.press(KeyCode::Enter);
    assert_eq!(f.app.all_tags(), ["ops", "prod", "support"], "enter doesn't confirm a delete");

    f.press(KeyCode::Char('x'));
    f.press(KeyCode::Char('y'));
    assert_eq!(tags_on_disk(&f), [vec!["support"], vec![], vec!["ops"]]);
    assert_eq!(f.app.tag_colors.get("prod"), None);
    assert!(matches!(f.app.mode, Mode::Tags(_)));
}

#[test]
fn recoloring_a_tag_is_saved() {
    let mut f = tagged();
    let before = f.app.tag_colors.name("ops");
    f.press(KeyCode::Char('t'));
    f.press(KeyCode::Char('c'));
    assert_ne!(f.app.tag_colors.name("ops"), before);
    f.press(KeyCode::Char('C'));
    assert_eq!(f.app.tag_colors.name("ops"), before);
    f.press(KeyCode::Char('c'));
    let saved = Store::at(f.dir.path()).unwrap().load_tag_colors().unwrap();
    assert_eq!(saved.name("ops"), f.app.tag_colors.name("ops"));
}

#[test]
fn buttons_leave_the_search_box_before_acting() {
    let mut f = Fixture::with_titles(&["feature", "bug"]);
    f.app.click(Click::Search);
    f.type_text("bug");
    assert!(matches!(f.app.mode, Mode::Search));
    // Pressed as a key, "s" would be typed into the query; as a button it starts the task.
    f.app.press(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
    assert!(matches!(f.app.mode, Mode::Normal));
    assert_eq!(f.app.search.text(), "bug", "the filter stays");
    assert_eq!(f.app.selected_task().unwrap().status, Status::Doing);
    // The Clear button.
    f.app.press(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(f.app.search.is_empty());
}

#[test]
fn clicks_select_open_and_filter() {
    let mut f = Fixture::with_titles(&["feature", "bug"]);
    f.app.click(Click::Row(1));
    assert_eq!(f.app.selected, 1);
    f.app.click(Click::Row(7));
    assert_eq!(f.app.selected, 1, "rows past the end are ignored");

    let id = f.app.selected_task().unwrap().id;
    f.app.click(Click::OpenRow(1));
    assert!(matches!(f.app.mode, Mode::Issue(IssueView { id: shown, .. }) if shown == id));
    // Rows aren't clickable behind the task view.
    f.app.click(Click::Row(0));
    assert!(matches!(f.app.mode, Mode::Issue(_)));
    f.press(KeyCode::Esc);

    f.press(KeyCode::Char('d'));
    f.app.click(Click::DoneView(DoneView::Only));
    assert_eq!(f.app.visible().len(), 1);
    assert_eq!(f.app.selected, 0);
}

#[test]
fn clicking_a_form_field_types_into_it() {
    let mut f = Fixture::with_titles(&[]);
    f.press(KeyCode::Char('n'));
    f.type_text("title");
    f.app.click(Click::Field(Field::Tags));
    f.type_text("ops");
    f.app.click(Click::Field(Field::Title));
    f.type_text("!");
    f.app.press(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let task = &f.app.tasks[0];
    assert_eq!((task.title.as_str(), task.tags.as_slice()), ("title!", ["ops".to_string()].as_slice()));
}

#[test]
fn comment_and_tag_buttons_act_on_their_own_row() {
    let mut f = tagged();
    // Tags are sorted: ops, prod, support. Delete "prod" with its row's button.
    f.press(KeyCode::Char('t'));
    f.app.click(Click::Tag(1));
    f.app.press(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
    f.app.press(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
    assert_eq!(f.app.all_tags(), ["ops", "support"]);

    f.press(KeyCode::Esc);
    f.press(KeyCode::Char('c'));
    f.type_text("first");
    f.press(KeyCode::Esc);
    f.press(KeyCode::Char('v'));
    f.press(KeyCode::Char('c'));
    f.type_text("second");
    f.press(KeyCode::Esc);
    let Mode::Issue(view) = f.app.mode else { panic!("not in the task view") };
    f.app.click(Click::Comment(0));
    f.app.press(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
    f.app.press(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
    let bodies: Vec<&str> = f.app.task(view.id).unwrap().comments.iter().map(|c| c.body.as_str()).collect();
    assert_eq!(bodies, ["second"]);
}

#[test]
fn clipboard_edits_the_field_being_typed_into() {
    let mut f = Fixture::with_titles(&["alpha", "beta"]);
    // Outside a text input there's nothing to select, copy or paste into.
    f.app.select_all();
    assert_eq!(f.app.selected_text(), None);
    f.app.paste("x");
    assert!(matches!(f.app.mode, Mode::Normal));

    // The search box: paste filters, select all + cut empties it again.
    f.press(KeyCode::Char('/'));
    f.app.paste("bet");
    assert_eq!(f.app.search.text(), "bet");
    assert_eq!(f.app.visible().len(), 1);
    f.app.select_all();
    assert_eq!(f.app.cut().as_deref(), Some("bet"));
    assert!(f.app.search.is_empty());
    f.press(KeyCode::Esc);

    // The text editor: pasted lines are kept, and copying doesn't discard.
    f.select(1);
    f.press(KeyCode::Char('m'));
    f.app.paste("line one\nline two");
    f.app.select_all();
    assert_eq!(f.app.selected_text().as_deref(), Some("line one\nline two"));
    f.press(KeyCode::Esc);
    assert_eq!(f.task(1).description, "line one\nline two");
}
