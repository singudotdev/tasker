use super::*;
use crate::model::now;

#[test]
fn save_renames_on_title_change_and_loads_back() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::at(dir.path()).unwrap();
    let mut task = Task::new(3, "First title".into(), now());
    store.save(&mut task).unwrap();
    task.title = "Second title".into();
    store.save(&mut task).unwrap();

    let names: Vec<String> =
        fs::read_dir(dir.path()).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    assert_eq!(names, ["0003-second-title.md"]);

    let (tasks, warnings) = store.load().unwrap();
    assert!(warnings.is_empty());
    assert_eq!(tasks[0].title, "Second title");
}

#[test]
fn unreadable_files_become_warnings() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::at(dir.path()).unwrap();
    fs::write(dir.path().join("README.md"), "# not a task").unwrap();
    fs::write(dir.path().join(TAGS_FILE), "- prod: red").unwrap();
    let (tasks, warnings) = store.load().unwrap();
    assert!(tasks.is_empty());
    assert_eq!(warnings.len(), 1, "tags.md is not a task: {warnings:?}");
}
