use super::*;
use crate::model::now;

fn parse(args: &[&str]) -> Result<Command> {
    Command::parse(args.iter().map(|s| (*s).to_string()))
}

#[test]
fn commands() {
    assert_eq!(parse(&[]).unwrap(), Command::Open);
    assert_eq!(parse(&["path"]).unwrap(), Command::Path);
    assert_eq!(parse(&["--version"]).unwrap(), Command::Version);
    assert!(parse(&["nope"]).is_err());
    assert!(parse(&["path", "extra"]).is_err());
}

#[test]
fn list_options() {
    let list = |args: &[&str]| match parse(args).unwrap() {
        Command::List(args) => args,
        other => panic!("expected a list command, got {other:?}"),
    };
    assert_eq!(list(&["list"]).statuses, [Status::Doing, Status::Todo, Status::Parked]);
    assert_eq!(
        list(&["list", "done", "doing", "--tag", "#Prod", "done"]),
        ListArgs { statuses: vec![Status::Doing, Status::Done], tag: Some("prod".into()) }
    );
    assert_eq!(list(&["list", "all"]).statuses, Status::ALL);
    assert!(parse(&["list", "paused"]).is_err());
    assert!(parse(&["list", "--tag"]).is_err());
}

#[test]
fn list_output() {
    let mut fix = Task::new(1, "Fix login".into(), now());
    fix.tags = vec!["prod".into(), "support".into()];
    fix.set_status(Status::Doing, now());
    let docs = Task::new(2, "Write docs".into(), now());
    let mut old = Task::new(3, "Old thing".into(), now());
    old.set_status(Status::Done, now());
    let tasks = [fix, docs, old];

    let all = ListArgs { statuses: Status::ALL.to_vec(), tag: None };
    assert_eq!(
        list_markdown(&tasks, &all),
        "## doing\n\n- #1 Fix login #prod #support\n\n## todo\n\n- #2 Write docs\n\n## done\n\n- #3 Old thing\n"
    );
    let prod = ListArgs { statuses: Status::ALL.to_vec(), tag: Some("prod".into()) };
    assert_eq!(list_markdown(&tasks, &prod), "## doing\n\n- #1 Fix login #prod #support\n");
}
