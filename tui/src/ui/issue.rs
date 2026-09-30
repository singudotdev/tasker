//! The task view: a task laid out like a GitHub/GitLab issue, with its status history.

use super::{DIM, heading_with_hints, history_line, status_style, tag_chips, when, wrap};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Paragraph};
use tasker_core::app::{App, Back, Confirm, Edit, IssueView, KeysMenu, Mode};
use tasker_core::model::{Comment, fmt_age, now};

/// The task view to draw behind popups that were opened from it.
pub fn shown_behind(mode: &Mode) -> Option<IssueView> {
    match mode {
        Mode::Issue(view) | Mode::Keys(KeysMenu { back: Back::Issue(view), .. }) => Some(*view),
        Mode::Edit(Edit { back, .. }) => *back,
        Mode::Confirm(Confirm::DeleteComment(id, index)) => Some(IssueView { id: *id, sel: *index }),
        _ => None,
    }
}

/// Header line, description, history, then comments; scrolled so the selected comment is visible.
pub fn draw(f: &mut Frame, app: &App, view: IssueView, area: Rect) {
    let Some(task) = app.task(view.id) else { return };
    let now = now();
    let (icon, style) = status_style(task.status);
    let block = Block::bordered()
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled(format!("{icon} "), style),
            Span::styled(format!("#{} {} ", task.id, task.title), Style::new().bold()),
        ]))
        .border_style(Style::new().cyan());
    let inner = block.inner(area);
    f.render_widget(block, area);
    // One column of padding on each side.
    let text_area = Rect { x: inner.x + 1, width: inner.width.saturating_sub(2), ..inner };
    let width = usize::from(text_area.width);

    let mut meta = vec![Span::styled(task.status.as_str(), style), Span::raw("  ")];
    meta.extend(tag_chips(app, &task.tags));
    meta.push(Span::styled(
        format!("  for {} · created {}", fmt_age(now - task.status_since()), task.created.format("%Y-%m-%d")),
        DIM,
    ));

    let mut lines = vec![Line::from(meta), Line::default(), heading_with_hints("Description".into(), &[("m", "edit")])];
    if task.description.is_empty() {
        lines.push(Line::styled("no description yet · m to write one", DIM));
    } else {
        lines.extend(wrap(&task.description, width).into_iter().map(Line::from));
    }
    lines.push(Line::default());
    lines.push(heading_with_hints(
        "History".into(),
        &[("s", "start"), ("p", "park"), ("d", "done"), ("b", "back to todo")],
    ));
    if task.history.is_empty() {
        lines.push(Line::styled(format!("{} since it was created", task.status.as_str()), DIM));
    }
    lines.extend(task.history.iter().map(history_line));
    lines.push(Line::default());
    lines.push(heading_with_hints(
        format!("Comments ({})", task.comments.len()),
        &[("c", "new"), ("e", "edit"), ("x", "delete"), ("↑↓/jk", "select")],
    ));
    if task.comments.is_empty() {
        lines.push(Line::styled("no comments yet · c to add one", DIM));
    }

    // Line range of the selected comment, to keep it on screen.
    let mut selected = 0..0;
    for (index, comment) in task.comments.iter().enumerate() {
        let is_selected = index == view.sel;
        let start = lines.len();
        lines.push(Line::default());
        lines.extend(comment_lines(comment, is_selected, width));
        if is_selected {
            selected = start..lines.len();
        }
    }

    // Scroll only when the selected comment would end below the screen; then show it from its top
    // if it fits, otherwise as much of its end as fits.
    let height = usize::from(text_area.height);
    let scroll = if selected.end <= height { 0 } else { selected.start.min(selected.end - height) };
    f.render_widget(Paragraph::new(lines).scroll((scroll as u16, 0)), text_area);
}

/// A comment's header (date, edited) and its wrapped body behind a bar; the selected one is highlighted.
fn comment_lines(comment: &Comment, selected: bool, width: usize) -> Vec<Line<'static>> {
    let accent = Style::new().cyan().bold();
    let bar = if selected { accent } else { DIM };
    let mut header = vec![
        Span::styled(if selected { "› " } else { "  " }, bar),
        Span::styled(when(comment.created), if selected { accent } else { Style::new().bold() }),
    ];
    if let Some(edited) = comment.edited {
        let format = if edited.date_naive() == comment.created.date_naive() { "%H:%M" } else { "%a %d %b %Y %H:%M" };
        header.push(Span::styled(format!(" · edited {}", edited.format(format)), DIM));
    }
    let mut lines = vec![Line::from(header)];
    lines.extend(
        wrap(&comment.body, width.saturating_sub(2))
            .into_iter()
            .map(|row| Line::from(vec![Span::styled("│ ", bar), Span::raw(row)])),
    );
    lines
}
