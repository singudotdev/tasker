//! Drawing. Reads the `App` and renders it; the only thing it records is where the task
//! list was drawn ([`ListArea`]), so mouse clicks can be mapped to rows.
//!
//! - [`list`]: header, task list, details panel and search bar.
//! - [`issue`]: the task view (description and comments).
//! - [`tags`]: the tags screen.
//! - [`dialogs`]: popups drawn over the current screen.
//! - [`hints`]: mnemonic key hints and the footer.

mod dialogs;
mod hints;
mod issue;
mod list;
mod tags;

use chrono::{DateTime, Local};
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;
use std::cell::Cell;
use tasker_core::app::App;
use tasker_core::model::{Status, StatusChange};
use tasker_core::tags::TagColor;

/// Secondary text.
const DIM: Style = Style::new().fg(Color::DarkGray);
/// Section headings ("Description", "History", …).
const HEADING: Style = Style::new().add_modifier(Modifier::BOLD.union(Modifier::UNDERLINED));
/// Height of the header box (border, one line, border).
const HEADER_HEIGHT: u16 = 3;

/// Where the task list was last drawn and its scroll offset, so mouse clicks can be mapped to rows.
#[derive(Debug, Default)]
pub struct ListArea {
    /// The list's box, borders included.
    area: Cell<Rect>,
    /// Index of the first visible row.
    offset: Cell<usize>,
}

impl ListArea {
    /// The task list row (an index into `App::visible`) at a terminal cell, if the cell is on one.
    pub fn row_at(&self, column: u16, row: u16) -> Option<usize> {
        let area = self.area.get();
        // Rows start below the top border and the header row.
        let first_row = area.y + 2;
        let inside = column > area.x && column + 1 < area.right() && row >= first_row && row + 1 < area.bottom();
        inside.then(|| self.offset.get() + usize::from(row - first_row))
    }
}

/// Draws the whole screen: header, the current screen, the footer, and any open dialog.
pub fn draw(f: &mut Frame, app: &App, list_area: &ListArea) {
    let footer = hints::footer(app, f.area().width);
    let [header, body, footer_area] = Layout::vertical([
        Constraint::Length(HEADER_HEIGHT),
        Constraint::Min(1),
        Constraint::Length(footer.len() as u16),
    ])
    .areas(f.area());

    list::draw_header(f, app, header);
    match (issue::shown_behind(&app.mode), tags::shown_behind(&app.mode)) {
        (Some(view), _) => issue::draw(f, app, view, body),
        (_, Some(view)) => tags::draw(f, app, view, body),
        _ => list::draw(f, app, list_area, body),
    }
    f.render_widget(Paragraph::new(footer), footer_area);
    dialogs::draw(f, app);
}

/// Icon and style for a task's status: `▶` doing, `·` todo, `‖` parked, `✓` done.
fn status_style(status: Status) -> (&'static str, Style) {
    match status {
        Status::Doing => ("▶", Style::new().green().bold()),
        Status::Todo => ("·", Style::new()),
        Status::Parked => ("‖", Style::new().yellow()),
        Status::Done => ("✓", DIM),
    }
}

/// A tag as a colored chip, Proxmox style.
fn tag_chip(app: &App, tag: &str) -> Span<'static> {
    Span::styled(format!(" {tag} "), tag_style(app, tag))
}

/// A tag chip's style: its color as background, with readable text on top.
fn tag_style(app: &App, tag: &str) -> Style {
    let (Some(color), Some((_, fg))) = (app.tag_colors.get(tag), app.tag_colors.chip(tag)) else {
        return Style::new().fg(Color::Cyan);
    };
    // Palette colors by their xterm-256 number, so they look the same without truecolor support.
    let bg = match color {
        TagColor::Palette(i) => Color::Indexed(tasker_core::tags::PALETTE[i].1),
        TagColor::Rgb(r, g, b) => Color::Rgb(r, g, b),
    };
    Style::new().bg(bg).fg(if fg == 0 { Color::Black } else { Color::White })
}

/// Tags as colored chips separated by a space.
fn tag_chips(app: &App, tags: &[String]) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for (i, tag) in tags.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" "));
        }
        spans.push(tag_chip(app, tag));
    }
    spans
}

/// `Mon 28 Sep 2026 10:12  todo → doing`, the new status in its color.
fn history_line(change: &StatusChange) -> Line<'static> {
    let (icon, style) = status_style(change.to);
    Line::from(vec![
        Span::styled(format!("{}  ", when(change.at)), DIM),
        Span::raw(format!("{} → ", change.from.as_str())),
        Span::styled(format!("{icon} {}", change.to.as_str()), style),
    ])
}

/// A section heading followed by its key hints: `Comments (2)  (c) new  (e)dit`.
fn heading_with_hints(title: String, keys: &[(&'static str, &'static str)]) -> Line<'static> {
    let mut spans = vec![Span::styled(title, HEADING), Span::raw("  ")];
    spans.extend(hints::spans(keys));
    Line::from(spans)
}

/// A `width` × `height` rectangle centered in `area`.
fn popup(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect { x: area.x + (area.width - width) / 2, y: area.y + (area.height - height) / 2, width, height }
}

/// Cuts `s` to `width` characters, ending with `…` when shortened.
fn truncate(s: &str, width: usize) -> String {
    if s.chars().count() <= width {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(width.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

/// Greedy word wrap; words longer than the width are split.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut out = Vec::new();
    for line in text.lines() {
        let mut current = String::new();
        for word in line.split(' ') {
            let mut word: Vec<char> = word.chars().collect();
            loop {
                let used = current.chars().count();
                let space = usize::from(used > 0);
                if used + space + word.len() <= width {
                    if space == 1 {
                        current.push(' ');
                    }
                    current.extend(word.iter());
                    break;
                }
                if used > 0 {
                    out.push(std::mem::take(&mut current));
                    continue;
                }
                // A single word wider than the line.
                let rest = word.split_off(width);
                out.push(word.iter().collect());
                word = rest;
            }
        }
        out.push(current);
    }
    out
}

/// "Thu 24 Sep 2026 10:15" in local time.
fn when(dt: DateTime<Local>) -> String {
    dt.format("%a %d %b %Y %H:%M").to_string()
}

#[cfg(test)]
#[path = "../../tests/ui.rs"]
mod tests;
