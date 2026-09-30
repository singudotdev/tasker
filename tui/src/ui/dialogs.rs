//! Popups drawn over the current screen: the create/edit form, delete confirmations,
//! the keys menu, the text editor and renaming a tag.

use super::{DIM, hints, popup, tag_chip, tag_chips, truncate, when, wrap};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Clear, List, ListItem, ListState, Paragraph, Wrap};
use tasker_core::app::{App, Back, Confirm, Edit, Field, Form, InputKind, KeysMenu, Mode, RenameTag, Target};
use tasker_core::editor::LineInput;
use tasker_core::model::Task;

/// Width of the form and delete dialogs.
const DIALOG_WIDTH: u16 = 64;

/// Draws the dialog of the current mode, if it has one.
pub fn draw(f: &mut Frame, app: &App) {
    match &app.mode {
        Mode::Input(form) => draw_form(f, app, form),
        Mode::Confirm(action) => draw_confirm(f, app, *action),
        Mode::Keys(menu) => draw_keys(f, menu),
        Mode::Edit(edit) => draw_editor(f, app, edit),
        Mode::RenameTag(rename) => draw_rename_tag(f, app, rename),
        Mode::Normal | Mode::Search | Mode::Issue(_) | Mode::Tags(_) => {}
    }
}

/// Clears `area` and draws `widget` in it.
fn render_popup(f: &mut Frame, widget: impl Widget, area: Rect) {
    f.render_widget(Clear, area);
    f.render_widget(widget, area);
}

/// The create/edit form: title and tags fields, and the tags already in use.
fn draw_form(f: &mut Frame, app: &App, form: &Form) {
    const LABEL_WIDTH: u16 = 7;
    let title = match form.kind {
        InputKind::New => " new task ",
        InputKind::Edit(_) => " edit task ",
    };
    let area = popup(f.area(), DIALOG_WIDTH, 6);
    let block = Block::bordered()
        .title(title)
        .title_bottom(hints::border(&[("tab/↑↓", "switch field"), ("enter", "ok"), ("esc", "cancel")]))
        .border_style(Style::new().cyan());
    let inner = block.inner(area);

    // Long input scrolls sideways to keep the cursor visible.
    let room = usize::from(inner.width.saturating_sub(LABEL_WIDTH));
    let field = |name: &str, input: &LineInput, active: bool| {
        let style = if active { Style::new().cyan().bold() } else { DIM };
        Line::from(vec![
            Span::styled(format!("{name:<w$}", w = usize::from(LABEL_WIDTH)), style),
            Span::raw(input.view(room).0),
        ])
    };

    let known = app.all_tags();
    let known_tags = if known.is_empty() {
        Line::styled("tags: comma or space separated, e.g. prod, support", DIM)
    } else {
        let mut spans = vec![Span::styled("known: ", DIM)];
        spans.extend(tag_chips(app, &known));
        Line::from(spans)
    };
    let lines = vec![
        field("title", &form.title, form.field == Field::Title),
        field("tags", &form.tags, form.field == Field::Tags),
        Line::default(),
        known_tags,
    ];
    render_popup(f, Paragraph::new(lines).block(block), area);

    let (input, row) = match form.field {
        Field::Title => (&form.title, 0),
        Field::Tags => (&form.tags, 1),
    };
    let cursor = input.view(room).1;
    f.set_cursor_position((inner.x + LABEL_WIDTH + cursor as u16, inner.y + row));
}

/// A delete dialog: what will be removed, and how to confirm or cancel.
fn draw_confirm(f: &mut Frame, app: &App, action: Confirm) {
    let (title, lines) = match action {
        Confirm::Delete(id) => {
            let Some(task) = app.task(id) else { return };
            let consequence = "Its markdown file, with description, comments and history, is removed.";
            (" delete task ", vec![question(app, task), Line::styled(consequence, DIM)])
        }
        Confirm::DeleteComment(id, index) => {
            let Some(comment) = app.task(id).and_then(|t| t.comments.get(index)) else { return };
            let preview = comment.body.lines().next().unwrap_or_default();
            let lines = vec![
                Line::from("Delete this comment?"),
                Line::styled(format!("{} · {}", when(comment.created), truncate(preview, 34)), DIM),
            ];
            (" delete comment ", lines)
        }
        Confirm::DeleteTag(sel) => {
            let Some(tag) = app.all_tags().into_iter().nth(sel) else { return };
            (" delete tag ", delete_tag_lines(app, &tag))
        }
    };

    // Long lines wrap, so size the box by the wrapped height; the keys are on the border and can't be cut off.
    let inner_width = usize::from(DIALOG_WIDTH.min(f.area().width).saturating_sub(2));
    let height: usize = lines.iter().map(|line| wrap(&line.to_string(), inner_width).len()).sum();
    let area = popup(f.area(), DIALOG_WIDTH, height as u16 + 2);
    let block = Block::bordered()
        .title(title)
        .title_bottom(hints::border(&[("y", "delete"), ("", "any other key cancels")]))
        .border_style(Style::new().red());
    render_popup(f, Paragraph::new(lines).wrap(Wrap { trim: true }).block(block), area);
}

/// "Delete tag `chip`?" and how many tasks lose it.
fn delete_tag_lines(app: &App, tag: &str) -> Vec<Line<'static>> {
    let count = app.tasks.iter().filter(|t| t.tags.iter().any(|t| t == tag)).count();
    vec![
        Line::from(vec![Span::raw("Delete tag "), tag_chip(app, tag), Span::raw("?")]),
        Line::styled(format!("It's removed from {count} task(s). The tasks themselves stay."), DIM),
    ]
}

/// `Delete #id title?` then its tag chips, as everywhere else a task is shown.
fn question(app: &App, task: &Task) -> Line<'static> {
    let mut spans = vec![
        Span::raw("Delete "),
        Span::styled(format!("#{} {}", task.id, task.title), Style::new().bold()),
        Span::raw("?  "),
    ];
    spans.extend(tag_chips(app, &task.tags));
    Line::from(spans)
}

/// The `?` menu: every action of the current screen; enter runs the selected one.
fn draw_keys(f: &mut Frame, menu: &KeysMenu) {
    let bindings = menu.bindings();
    let items: Vec<ListItem> = bindings
        .iter()
        .map(|b| {
            ListItem::new(Line::from(vec![
                Span::styled(format!(" {:<7}", b.key), Style::new().cyan().bold()),
                Span::raw(b.menu.unwrap_or_default()),
            ]))
        })
        .collect();
    let title = match menu.back {
        Back::List => " keys · tasks ",
        Back::Issue(_) => " keys · task view ",
        Back::Tags(_) => " keys · tags ",
    };
    let area = popup(f.area(), 76, bindings.len() as u16 + 2);
    let list = List::new(items)
        .block(
            Block::bordered()
                .title(title)
                .title_bottom(hints::border(&[("enter", "run"), ("", "or press the key"), ("esc", "close")]))
                .border_style(Style::new().cyan()),
        )
        .highlight_style(Style::new().add_modifier(Modifier::REVERSED));
    let mut state = ListState::default().with_selected(Some(menu.sel));
    f.render_widget(Clear, area);
    f.render_stateful_widget(list, area, &mut state);
}

/// Multi-line editor for a description or a comment.
fn draw_editor(f: &mut Frame, app: &App, edit: &Edit) {
    let screen = f.area();
    let area = popup(screen, screen.width.saturating_sub(8).min(100), (screen.height * 7 / 10).max(10));

    let task = app.task(edit.target.task_id());
    let name = task.map(|t| format!("#{} {}", t.id, t.title)).unwrap_or_default();
    let what = match edit.target {
        Target::Description(_) => "description".to_string(),
        Target::NewComment(_) => "new comment".to_string(),
        Target::Comment(_, index) => task
            .and_then(|t| t.comments.get(index))
            .map_or_else(|| "comment".to_string(), |c| format!("comment of {}", when(c.created))),
    };
    let modified = if edit.editor.modified { " ●" } else { "" };
    let block = Block::bordered()
        .title(format!(" {what} · {name}{modified} "))
        .title_bottom(hints::border(&[("esc", "save & close"), ("ctrl+c", "discard")]))
        .border_style(Style::new().cyan());
    let inner = block.inner(area);

    let (rows, (x, y)) = edit.editor.layout(usize::from(inner.width), usize::from(inner.height));
    let lines: Vec<Line> = rows.into_iter().map(Line::from).collect();
    render_popup(f, Paragraph::new(lines).block(block), area);
    f.set_cursor_position((inner.x + x as u16, inner.y + y as u16));
}

/// A new name for a tag: the current chip, the typed name, and what an existing name does.
fn draw_rename_tag(f: &mut Frame, app: &App, rename: &RenameTag) {
    const LABEL_WIDTH: u16 = 7;
    let Some(from) = app.all_tags().into_iter().nth(rename.sel) else { return };
    let area = popup(f.area(), DIALOG_WIDTH, 6);
    let block = Block::bordered()
        .title(" rename tag ")
        .title_bottom(hints::border(&[("enter", "rename"), ("esc", "cancel")]))
        .border_style(Style::new().cyan());
    let inner = block.inner(area);

    // A long name scrolls sideways to keep the cursor visible.
    let (name, cursor) = rename.name.view(usize::from(inner.width.saturating_sub(LABEL_WIDTH)));
    let cursor_x = inner.x + LABEL_WIDTH + cursor as u16;
    let lines = vec![
        Line::from(vec![Span::styled("from   ", DIM), tag_chip(app, &from)]),
        Line::from(vec![Span::styled("to     ", Style::new().cyan().bold()), Span::raw(name)]),
        Line::default(),
        Line::styled("On every task. A name already in use merges the two tags.", DIM),
    ];
    render_popup(f, Paragraph::new(lines).block(block), area);
    f.set_cursor_position((cursor_x, inner.y + 1));
}

#[cfg(test)]
#[path = "../../tests/ui/dialogs.rs"]
mod tests;
