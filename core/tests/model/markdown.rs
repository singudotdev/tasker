use super::*;
use chrono::Duration;

#[test]
fn round_trip() {
    let t0 = parse_ts("2026-09-24 09:00").unwrap();
    let mut task = Task::new(7, "Fix prod login".into(), t0);
    task.description = "customer X reported it\n\n- check logs\n### steps".into();
    task.comments = vec![
        Comment { created: t0, edited: None, body: "first\n\nwith a blank line".into() },
        Comment { created: t0 + Duration::minutes(5), edited: Some(t0 + Duration::minutes(9)), body: "second".into() },
        Comment { created: t0 + Duration::minutes(7), edited: None, body: String::new() },
    ];
    task.tags = vec!["prod".into(), "support".into()];
    task.set_status(Status::Doing, t0 + Duration::minutes(10));
    task.set_status(Status::Done, t0 + Duration::hours(2));

    let back = Task::from_markdown(&task.to_markdown()).unwrap();
    assert_eq!(back.id, 7);
    assert_eq!(back.title, task.title);
    assert_eq!(back.status, Status::Done);
    assert_eq!(back.created, t0);
    assert_eq!(back.description, task.description);
    assert_eq!(back.comments, task.comments);
    assert_eq!(back.tags, task.tags);
    assert_eq!(back.history, task.history);
}

#[test]
fn hand_typed_history_and_missing_status() {
    let md = "# x\n\n- id: 1\n\n## History\n\n- 2026-09-24 10:00 todo -> doing\n";
    let task = Task::from_markdown(md).unwrap();
    assert_eq!(task.status, Status::Todo, "the status line decides, not the history");
    assert_eq!(task.history[0].at, parse_ts("2026-09-24 10:00").unwrap());
    assert_eq!(task.history[0].to, Status::Doing);
    assert!(Task::from_markdown("# x\n\n- id: 1\n\n## History\n\n- 2026-09-24 10:00 todo -> nope\n").is_err());
}

#[test]
fn missing_id_is_an_error() {
    assert!(Task::from_markdown("# no id\n").is_err());
}
