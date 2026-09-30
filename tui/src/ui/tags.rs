//! The tags screen: every tag with its color and how many tasks use it, per status.

use super::{DIM, tag_chip};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Cell, Paragraph, Row, Table, TableState};
use tasker_core::app::{App, Back, Confirm, KeysMenu, Mode, TagsView};
use tasker_core::model::Status;

/// The tags screen to draw behind popups that were opened from it.
pub fn shown_behind(mode: &Mode) -> Option<TagsView> {
    match mode {
        Mode::Tags(view) | Mode::Keys(KeysMenu { back: Back::Tags(view), .. }) => Some(*view),
        Mode::RenameTag(rename) => Some(TagsView { sel: rename.sel }),
        Mode::Confirm(Confirm::DeleteTag(sel)) => Some(TagsView { sel: *sel }),
        _ => None,
    }
}

/// One row per tag, alphabetically, with the selected one highlighted.
pub fn draw(f: &mut Frame, app: &App, view: TagsView, area: Rect) {
    let tags = app.all_tags();
    let block = Block::bordered().title(format!(" tags · {} ", tags.len())).border_style(Style::new().cyan());
    if tags.is_empty() {
        let text = "\n  No tags yet. Add some to a task with e in the task list.";
        f.render_widget(Paragraph::new(text).style(DIM).block(block), area);
        return;
    }

    let rows = tags.iter().map(|tag| {
        let mut cells = vec![Cell::from(tag_chip(app, tag)), Cell::from(app.tag_colors.name(tag).unwrap_or_default())];
        cells.extend(Status::ALL.iter().map(|status| {
            let count = app.tasks.iter().filter(|t| t.status == *status && t.tags.contains(tag)).count();
            Cell::from(count.to_string())
        }));
        Row::new(cells)
    });
    // As wide as the widest chip, so the counts stay next to the tags.
    let tag_width = tags.iter().map(|t| tag_chip(app, t).width()).max().unwrap_or(0).max(3);
    let mut widths = vec![Constraint::Length(tag_width as u16), Constraint::Length(9)];
    widths.extend(Status::ALL.iter().map(|_| Constraint::Length(6)));
    let mut header = vec!["tag", "color"];
    header.extend(Status::ALL.iter().map(|s| s.as_str()));
    let table = Table::new(rows, widths)
        .header(Row::new(header).style(DIM.bold()))
        .block(block)
        .row_highlight_style(Style::new().add_modifier(Modifier::REVERSED))
        .highlight_symbol("› ");
    let mut state = TableState::default().with_selected(Some(view.sel.min(tags.len() - 1)));
    f.render_stateful_widget(table, area, &mut state);
}
