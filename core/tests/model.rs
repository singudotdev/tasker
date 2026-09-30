use super::*;
use chrono::Duration;

#[test]
fn status_changes_are_recorded_once() {
    let t0 = parse_ts("2026-09-24 09:00").unwrap();
    let mut task = Task::new(1, "x".into(), t0);
    assert_eq!(task.status_since(), t0);
    assert!(task.set_status(Status::Doing, t0 + Duration::hours(1)));
    assert!(!task.set_status(Status::Doing, t0 + Duration::hours(2)), "already doing");
    assert!(task.set_status(Status::Done, t0 + Duration::hours(3)));
    assert_eq!(
        task.history,
        [
            StatusChange { at: t0 + Duration::hours(1), from: Status::Todo, to: Status::Doing },
            StatusChange { at: t0 + Duration::hours(3), from: Status::Doing, to: Status::Done },
        ]
    );
    assert_eq!(task.status_since(), t0 + Duration::hours(3));
}

#[test]
fn search() {
    let mut task = Task::new(1, "Fix Login page".into(), now());
    task.tags = vec!["prod".into(), "support".into()];
    assert!(task.matches(""));
    assert!(task.matches("login"));
    assert!(task.matches("LOGIN #pro"));
    assert!(task.matches("supp"));
    assert!(!task.matches("#login"));
    assert!(!task.matches("login #backend"));
}
