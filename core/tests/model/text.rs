use super::*;

#[test]
fn tags() {
    assert_eq!(parse_tags("#Prod, support  prod,,#meeting"), ["prod", "support", "meeting"]);
    assert!(parse_tags(" , ").is_empty());
}

#[test]
fn slugs() {
    assert_eq!(slugify("Fix: prod  login!!"), "fix-prod-login");
    assert_eq!(slugify("¿?"), "task");
    assert_eq!(slugify(&"a".repeat(100)).len(), MAX_SLUG_LEN);
}

#[test]
fn headings_are_demoted() {
    assert_eq!(sanitize_description("## a\n### b"), ("### a\n### b".into(), true));
    assert_eq!(sanitize_comment("## a\n### b\n#### c"), ("#### a\n#### b\n#### c".into(), true));
    assert_eq!(sanitize_comment("plain"), ("plain".into(), false));
}
