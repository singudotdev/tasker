use super::*;

#[test]
fn hand_typed_and_stored_formats_agree() {
    let typed = parse_ts("2026-09-24 10:12").unwrap();
    assert_eq!(parse_ts(&fmt_ts(typed)), Some(typed));
    assert_eq!(parse_ts("yesterday"), None);
}

#[test]
fn ages() {
    assert_eq!(fmt_age(Duration::seconds(59)), "0m");
    assert_eq!(fmt_age(Duration::minutes(-5)), "0m");
    assert_eq!(fmt_age(Duration::minutes(65)), "1h");
    assert_eq!(fmt_age(Duration::days(3)), "3d");
    assert_eq!(fmt_age(Duration::days(15)), "2w");
}
