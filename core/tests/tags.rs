use super::*;

#[test]
fn no_repeats_until_palette_is_used_up() {
    let mut colors = TagColors::default();
    let tags: Vec<String> = (0..PALETTE.len() + 3).map(|i| format!("tag{i}")).collect();
    assert!(colors.assign(&tags));
    let mut first: Vec<TagColor> = tags[..PALETTE.len()].iter().map(|t| colors.get(t).unwrap()).collect();
    first.sort_by_key(|c| c.name());
    first.dedup();
    assert_eq!(first.len(), PALETTE.len());
    assert!(!colors.assign(&tags), "already assigned tags keep their color");
}

#[test]
fn round_trip_and_custom_colors() {
    let colors = TagColors::parse("# Tag colors\n\n- prod: red\n- #Support: #33AAff\n- bad: nope\n");
    assert_eq!(colors.get("prod"), Some(TagColor::Palette(0)));
    assert_eq!(colors.get("support"), Some(TagColor::Rgb(0x33, 0xaa, 0xff)));
    assert_eq!(colors.get("bad"), None);
    let again = TagColors::parse(&colors.to_markdown());
    assert_eq!(again.get("support"), colors.get("support"));
    // Palette index 160 is xterm (215, 0, 0).
    assert_eq!(TagColor::Palette(0).rgb(), (215, 0, 0));
}

#[test]
fn rename_remove_and_cycle() {
    let mut colors = TagColors::parse("- prod: red\n- ops: blue\n- web: #33aaff\n");
    colors.rename("prod", "live");
    assert_eq!(colors.get("live"), Some(TagColor::Palette(0)), "the color moves with the name");
    assert_eq!(colors.get("prod"), None);
    colors.rename("live", "ops");
    assert_eq!(colors.get("ops"), Some(TagColor::Palette(6)), "merging keeps the target's color");
    assert_eq!(colors.get("live"), None);

    colors.cycle("ops", false);
    assert_eq!(colors.name("ops").as_deref(), Some("indigo"));
    colors.cycle("ops", true);
    colors.cycle("ops", true);
    assert_eq!(colors.name("ops").as_deref(), Some("teal"));
    colors.cycle("web", true);
    assert_eq!(colors.name("web").as_deref(), Some("sky"), "hex steps back to the last palette color");

    colors.remove("web");
    assert_eq!(colors.get("web"), None);
}
