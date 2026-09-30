//! Key hints in mnemonic style (`(n)ew  new (c)omment  (x) delete`) and the footer.

use super::DIM;
use ratatui::prelude::*;
use tasker_core::app::{App, Mode};
use tasker_core::keys::{self, Binding};

/// Style of the key inside a hint.
const KEY: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);
/// Space between two hints.
const SEPARATOR: &str = "  ";

/// A (key, label) pair; an empty key is plain explanatory text.
type Hint = (&'static str, &'static str);

/// One hint: the key in parentheses, inside the label when it fits.
fn hint(key: &'static str, label: &'static str) -> Vec<Span<'static>> {
    if key.is_empty() {
        return vec![Span::styled(label, DIM)];
    }
    match keys::mnemonic(key, label) {
        Some((before, key, after)) => vec![
            Span::styled(before, DIM),
            Span::styled("(", DIM),
            Span::styled(key, KEY),
            Span::styled(")", DIM),
            Span::styled(after, DIM),
        ],
        None => vec![Span::styled("(", DIM), Span::styled(key, KEY), Span::styled(format!(") {label}"), DIM)],
    }
}

/// Several hints, separated.
pub fn spans(items: &[Hint]) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for (i, &(key, label)) in items.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(SEPARATOR));
        }
        spans.extend(hint(key, label));
    }
    spans
}

/// Hints for a popup border, padded with a space on each side.
pub fn border(items: &[Hint]) -> Line<'static> {
    let mut line = vec![Span::raw(" ")];
    line.extend(spans(items));
    line.push(Span::raw(" "));
    Line::from(line)
}

/// A temporary message, or every key of the current screen wrapped between hints.
pub fn footer(app: &App, width: u16) -> Vec<Line<'static>> {
    if let Some(msg) = app.message() {
        return vec![Line::styled(format!(" {msg}"), Style::new().yellow())];
    }
    // Fill lines left to right; a hint that doesn't fit starts a new line, so none is cut in half.
    let mut lines = Vec::new();
    let mut current = vec![Span::raw(" ")];
    let mut used = 1; // columns taken on the current line, including the leading space
    for (key, label) in footer_hints(app) {
        let hint = hint(key, label);
        let hint_width: usize = hint.iter().map(|s| s.content.chars().count()).sum();
        if used > 1 && used + SEPARATOR.len() + hint_width > usize::from(width) {
            lines.push(Line::from(std::mem::replace(&mut current, vec![Span::raw(" ")])));
            used = 1;
        }
        if used > 1 {
            current.push(Span::raw(SEPARATOR));
            used += SEPARATOR.len();
        }
        current.extend(hint);
        used += hint_width;
    }
    lines.push(Line::from(current));
    lines
}

/// The keys to show for the current screen.
fn footer_hints(app: &App) -> Vec<Hint> {
    let from = |bindings: &'static [Binding]| -> Vec<Hint> { bindings.iter().filter_map(|b| b.footer).collect() };
    match app.mode {
        Mode::Normal => {
            let mut hints = from(keys::LIST);
            // esc only does something in the list while a search filter is active.
            if !app.search.is_empty() {
                hints.insert(0, ("esc", "clear search"));
            }
            hints
        }
        Mode::Issue(_) => from(keys::ISSUE),
        Mode::Tags(_) => from(keys::TAGS),
        Mode::RenameTag(_) => vec![("", "type"), ("←→ home end", "cursor"), ("enter", "rename"), ("esc", "cancel")],
        Mode::Search => {
            vec![
                ("", "type to filter, #tag = tags only"),
                ("←→", "cursor"),
                ("↑/↓", "move"),
                ("enter", "keep"),
                ("esc", "clear"),
            ]
        }
        Mode::Keys(_) => vec![("↑↓/jk", "move"), ("enter", "run"), ("", "or press an action's key"), ("esc", "close")],
        Mode::Edit(_) => vec![
            ("", "type to edit"),
            ("enter", "new line"),
            ("arrows home end", "move"),
            ("esc", "save & close"),
            ("ctrl+c", "discard"),
        ],
        Mode::Input(_) => vec![
            ("", "type"),
            ("←→ home end", "cursor"),
            ("tab/↑↓", "switch field"),
            ("enter", "ok"),
            ("esc", "cancel"),
        ],
        Mode::Confirm(_) => vec![("y", "delete"), ("", "any other key cancels")],
    }
}
