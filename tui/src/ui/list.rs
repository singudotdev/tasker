//! The main screen: header, task list, details panel and search bar.

use super::{DIM, HEADING, ListArea, heading_with_hints, history_line, status_style, tag_chips, when};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Cell, Paragraph, Row, Table, TableState, Wrap};
use tasker_core::app::{App, DoneView, Mode};
use tasker_core::model::{Status, Task, fmt_age, now};

/// Wide enough for the list and the details panel side by side.
const SIDE_BY_SIDE_MIN_WIDTH: u16 = 110;
/// Tall enough to stack the details panel under the list.
const STACKED_MIN_HEIGHT: u16 = 24;
/// Height of the details panel when it's under the list.
const STACKED_DETAILS_HEIGHT: u16 = 13;
/// How many recent status changes the details panel shows.
const RECENT_CHANGES: usize = 4;
/// Description lines shown in the details panel.
const DESCRIPTION_LINES: usize = 6;
/// Lines of the latest comment shown in the details panel.
const COMMENT_LINES: usize = 4;
/// Width of the list's tags column: enough for its heading, at most this much.
const TAGS_MIN_WIDTH: usize = 4;
/// Longer tag lists are cut off, so the title keeps room.
const TAGS_MAX_WIDTH: usize = 30;

/// How many tasks are in each status.
pub fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::bordered().title(" tasker ");
    let mut spans = vec![Span::raw(" ")];
    for (i, status) in Status::ALL.iter().enumerate() {
        let count = app.tasks.iter().filter(|t| t.status == *status).count();
        let (icon, style) = status_style(*status);
        if i > 0 {
            spans.push(Span::raw("    "));
        }
        spans.push(Span::styled(format!("{icon} {count} {}", status.as_str()), style));
    }
    f.render_widget(Paragraph::new(Line::from(spans)).block(block), area);
}

/// Task list plus a details panel for the selected task, lazygit/lazyssh style:
/// side by side when wide, stacked when tall, list only when small.
pub fn draw(f: &mut Frame, app: &App, list_area: &ListArea, area: Rect) {
    let selected = app.selected_task();
    if area.width >= SIDE_BY_SIDE_MIN_WIDTH {
        let details_width = (area.width * 2 / 5).clamp(40, 60);
        let [list, details] = Layout::horizontal([Constraint::Min(60), Constraint::Length(details_width)]).areas(area);
        draw_tasks(f, app, list_area, list);
        draw_details(f, app, selected, details);
    } else if area.height >= STACKED_MIN_HEIGHT {
        let [list, details] =
            Layout::vertical([Constraint::Min(8), Constraint::Length(STACKED_DETAILS_HEIGHT)]).areas(area);
        draw_tasks(f, app, list_area, list);
        draw_details(f, app, selected, details);
    } else {
        draw_tasks(f, app, list_area, area);
    }
}

/// The task table (and the search bar above it while searching or filtered).
fn draw_tasks(f: &mut Frame, app: &App, list_area: &ListArea, area: Rect) {
    let visible = app.visible();
    let searching = matches!(app.mode, Mode::Search);
    let area = if searching || !app.search.is_empty() {
        let [search, rest] = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).areas(area);
        draw_search(f, app, visible.len(), searching, search);
        rest
    } else {
        area
    };

    let block = Block::bordered().title(list_title(app));
    if visible.is_empty() {
        let text = if app.tasks.is_empty() {
            "\n  No tasks yet.\n\n  n  create a task"
        } else {
            "\n  No tasks match. esc clears the search, f switches active / all / done."
        };
        f.render_widget(Paragraph::new(text).style(DIM).block(block), area);
        return;
    }

    // The tags column is as wide as the widest set of chips, so titles get the rest.
    let tags_width = visible
        .iter()
        .map(|&i| tag_chips(app, &app.tasks[i].tags).iter().map(Span::width).sum::<usize>())
        .max()
        .unwrap_or(0)
        .clamp(TAGS_MIN_WIDTH, TAGS_MAX_WIDTH);
    let now = now();
    let rows = visible.iter().map(|&i| {
        let task = &app.tasks[i];
        let (icon, style) = status_style(task.status);
        Row::new(vec![
            Cell::from(icon),
            Cell::from(format!("#{}", task.id)),
            Cell::from(task.title.clone()),
            Cell::from(Line::from(tag_chips(app, &task.tags))),
            Cell::from(fmt_age(now - task.status_since())),
        ])
        .style(style)
    });
    let widths = [
        Constraint::Length(1),
        Constraint::Length(5),
        Constraint::Min(10),
        Constraint::Length(tags_width as u16),
        Constraint::Length(5),
    ];
    let table = Table::new(rows, widths)
        .header(Row::new(["", "id", "task", "tags", "since"]).style(DIM.bold()))
        .block(block)
        .row_highlight_style(Style::new().add_modifier(Modifier::REVERSED))
        .highlight_symbol("› ");
    let mut state = TableState::default().with_offset(list_area.offset.get()).with_selected(Some(app.selected));
    f.render_stateful_widget(table, area, &mut state);
    // Remember geometry so mouse clicks can be mapped to rows.
    list_area.area.set(area);
    list_area.offset.set(state.offset());
}

/// The list's border title: how done tasks are shown and how many are hidden.
fn list_title(app: &App) -> String {
    let done = app.tasks.iter().filter(|t| t.status == Status::Done).count();
    match app.done_view {
        DoneView::Hide if done > 0 => format!(" tasks · {done} done hidden (f) "),
        DoneView::Hide => " tasks ".to_string(),
        DoneView::Show => " tasks · incl. done ".to_string(),
        DoneView::Only => format!(" done tasks · {done} "),
    }
}

/// The `/ query   N match(es)` line, with the cursor while typing.
fn draw_search(f: &mut Frame, app: &App, matches: usize, editing: bool, area: Rect) {
    let prompt = if editing { Style::new().cyan().bold() } else { DIM };
    let count = format!("   {matches} match(es)");
    let (query, cursor) = app.search.view(usize::from(area.width).saturating_sub(3 + count.len()));
    let line = Line::from(vec![Span::styled(" / ", prompt), Span::raw(query), Span::styled(count, DIM)]);
    f.render_widget(Paragraph::new(line), area);
    if editing {
        f.set_cursor_position((area.x + 3 + cursor as u16, area.y));
    }
}

/// The selected task at a glance: status and since when, recent history, description and latest comment.
fn draw_details(f: &mut Frame, app: &App, task: Option<&Task>, area: Rect) {
    let block = Block::bordered().title(" details ");
    let Some(task) = task else {
        f.render_widget(block, area);
        return;
    };
    let now = now();
    let label = |s: &str| Span::styled(format!("{s:<10}"), DIM);
    let (icon, style) = status_style(task.status);
    let since = task.status_since();

    let mut lines = vec![Line::styled(format!("#{} {}", task.id, task.title), Style::new().bold())];
    if !task.tags.is_empty() {
        lines.push(Line::from(tag_chips(app, &task.tags)));
    }
    lines.extend([
        Line::default(),
        Line::from(vec![
            label("status"),
            Span::styled(format!("{icon} {}", task.status.as_str()), style),
            Span::styled(format!(" for {}", fmt_age(now - since)), DIM),
        ]),
        Line::from(vec![label("since"), Span::raw(when(since))]),
        Line::from(vec![label("created"), Span::raw(when(task.created))]),
        Line::default(),
        Line::styled("History", HEADING),
    ]);
    if task.history.is_empty() {
        lines.push(Line::styled("no status changes yet", DIM));
    }
    for change in task.history.iter().rev().take(RECENT_CHANGES) {
        lines.push(history_line(change));
    }
    if let Some(older) = task.history.len().checked_sub(RECENT_CHANGES).filter(|&n| n > 0) {
        lines.push(Line::styled(format!("… {older} older · v to see all"), DIM));
    }

    lines.push(Line::default());
    lines.push(heading_with_hints("Description".into(), &[("m", "edit"), ("v", "view")]));
    if task.description.is_empty() {
        lines.push(Line::styled("no description yet", DIM));
    } else {
        lines.extend(task.description.lines().take(DESCRIPTION_LINES).map(|l| Line::from(l.to_string())));
        if task.description.lines().count() > DESCRIPTION_LINES {
            lines.push(Line::styled("… v to read it all", DIM));
        }
    }

    lines.push(Line::default());
    lines.push(heading_with_hints(format!("Comments ({})", task.comments.len()), &[("c", "add"), ("v", "view")]));
    match task.comments.last() {
        None => lines.push(Line::styled("no comments yet", DIM)),
        Some(comment) => {
            lines.push(Line::styled(format!("latest · {}", when(comment.created)), DIM));
            lines.extend(comment.body.lines().take(COMMENT_LINES).map(|l| Line::from(l.to_string())));
        }
    }

    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }).block(block), area);
}
