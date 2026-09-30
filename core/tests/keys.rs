use super::*;

#[test]
fn mnemonics() {
    assert_eq!(mnemonic("n", "new"), Some(("", "n", "ew")));
    assert_eq!(mnemonic("c", "new comment"), Some(("new ", "c", "omment")));
    assert_eq!(mnemonic("Q", "Quit & pause"), Some(("", "Q", "uit & pause")));
    assert_eq!(mnemonic("q", "Quit & pause"), None, "case-sensitive");
    assert_eq!(mnemonic("x", "delete"), None);
    assert_eq!(mnemonic("enter", "start/pause"), None);
}
