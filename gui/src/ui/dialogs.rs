//! Dialogs drawn over the current screen: the create/edit form, delete confirmations,
//! the keyboard shortcuts, the text editor and renaming a tag. Each has its buttons at the bottom.

use super::caret::caret;
use super::pointer::{Hit, TextBox, pointable};
use super::theme::{self, ACCENT, BORDER, DIM_STYLE, Line, Span, Style};
use super::widgets::{Kind, button, dim, tag_chip, tag_chips, tag_span, text_field};
use super::{Scrolls, TaskerView, action, byte_at, edited, truncate, when};
use gpui::{ClickEvent, Context, Div, FontWeight, ScrollHandle, Stateful, div, prelude::*, px, relative, rgb, rgba};
use tasker_core::app::{App, Back, Click, Confirm, Edit, Field, Form, InputKind, KeysMenu, Mode, RenameTag, Target};
use tasker_core::event::{KeyCode, KeyEvent, KeyModifiers};
use tasker_core::model::Task;

/// Width of the form and delete dialogs.
const DIALOG_WIDTH: f32 = 520.;
/// Width of the keyboard shortcuts dialog.
const KEYS_WIDTH: f32 = 600.;
/// Widest the text editor gets.
const EDITOR_WIDTH: f32 = 860.;

/// Draws the dialog of the current mode, if it has one.
pub fn draw(app: &App, scroll: &Scrolls, follow: bool, cx: &mut Context<TaskerView>) -> Option<Div> {
    let dialog = match &app.mode {
        Mode::Input(form) => draw_form(app, form, cx),
        Mode::Confirm(action) => draw_confirm(app, *action, cx)?,
        Mode::Keys(menu) => draw_keys(menu, &scroll.keys, follow, cx),
        Mode::Edit(edit) => draw_editor(app, edit, &scroll.editor, follow, cx),
        Mode::RenameTag(rename) => draw_rename_tag(app, rename, cx)?,
        Mode::Normal | Mode::Search | Mode::Issue(_) | Mode::Tags(_) => return None,
    };
    Some(modal(dialog))
}

/// Centers `dialog` over the dimmed window; nothing behind it can be clicked.
fn modal(dialog: Div) -> Div {
    div()
        .absolute()
        .inset_0()
        .flex()
        .justify_center()
        .items_center()
        .p_4()
        .bg(rgba(0x0000_0099))
        .occlude()
        .child(dialog.max_w(relative(1.)).max_h(relative(1.)))
}

/// A dialog box: title, content, and buttons at the bottom right.
fn dialog(title: impl Into<String>, content: impl IntoElement, buttons: Vec<Stateful<Div>>) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_4()
        .p_5()
        .rounded_lg()
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(theme::POPUP))
        .shadow_lg()
        .child(div().text_size(px(16.)).font_weight(FontWeight::SEMIBOLD).child(title.into()))
        .child(content)
        .child(div().flex().justify_end().gap_2().children(buttons))
}

/// A label above a field.
fn labeled(label: &str, field: impl IntoElement) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(div().text_size(px(12.)).font_weight(FontWeight::MEDIUM).child(label.to_string()))
        .child(field)
}

/// The create/edit form: title and tags fields, and the tags already in use.
fn draw_form(app: &App, form: &Form, cx: &mut Context<TaskerView>) -> Div {
    let (title, submit) = match form.kind {
        InputKind::New => ("New task", "Create task"),
        InputKind::Edit(_) => ("Edit task", "Save"),
    };
    let field = |which: Field, placeholder: &str, cx: &mut Context<TaskerView>| {
        let (input, text_box) = match which {
            Field::Title => (&form.title, TextBox::Title),
            Field::Tags => (&form.tags, TextBox::Tags),
        };
        text_field(text_box, input, form.field == which, placeholder, cx)
    };
    let known = app.all_tags();
    let content = div()
        .flex()
        .flex_col()
        .gap_3()
        .child(labeled("Title", field(Field::Title, "What needs doing?", cx)))
        .child(labeled("Tags", field(Field::Tags, "e.g. prod, support", cx)))
        .child(dim("Separate tags with commas or spaces.").text_size(px(12.)))
        .when(!known.is_empty(), |d| {
            d.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(dim("Tags in use").text_size(px(12.)))
                    .child(tag_chips(app, &known)),
            )
        });
    let buttons = vec![
        action("cancel", "Cancel", Kind::Secondary, KeyCode::Esc, cx),
        action("submit", submit, Kind::Primary, KeyCode::Enter, cx),
    ];
    dialog(title, content, buttons).w(px(DIALOG_WIDTH))
}

/// A delete dialog: what will be removed, with Cancel and Delete buttons.
fn draw_confirm(app: &App, action_: Confirm, cx: &mut Context<TaskerView>) -> Option<Div> {
    let (title, lines) = match action_ {
        Confirm::Delete(id) => {
            let task = app.task(id)?;
            let consequence = "Its markdown file, with description, comments and history, is removed.";
            ("Delete task?", vec![question(task), Line::styled(consequence, DIM_STYLE)])
        }
        Confirm::DeleteComment(id, index) => {
            let comment = app.task(id).and_then(|t| t.comments.get(index))?;
            let preview = comment.body.lines().next().unwrap_or_default();
            let lines = vec![
                Line::raw("This comment is removed from the task."),
                Line::styled(format!("{} · {}", when(comment.created), truncate(preview, 48)), DIM_STYLE),
            ];
            ("Delete comment?", lines)
        }
        Confirm::DeleteTag(sel) => {
            let tag = app.all_tags().into_iter().nth(sel)?;
            ("Delete tag?", delete_tag_lines(app, &tag))
        }
    };
    let content = div().flex().flex_col().gap_1().children(lines.into_iter().map(Line::render));
    let buttons = vec![
        action("cancel", "Cancel", Kind::Secondary, KeyCode::Esc, cx),
        action("confirm", "Delete", Kind::Danger, KeyCode::Char('y'), cx),
    ];
    Some(dialog(title, content, buttons).w(px(DIALOG_WIDTH)))
}

/// "Remove tag `chip` from every task?" and how many tasks lose it.
fn delete_tag_lines(app: &App, tag: &str) -> Vec<Line> {
    let count = app.tasks.iter().filter(|t| t.tags.iter().any(|t| t == tag)).count();
    vec![
        Line::from(vec![Span::raw("Remove "), tag_span(app, tag), Span::raw(" from every task?")]),
        Line::styled(format!("It's removed from {count} task(s). The tasks themselves stay."), DIM_STYLE),
    ]
}

/// `#id title` in bold: the task about to be deleted.
fn question(task: &Task) -> Line {
    Line::from(vec![Span::styled(format!("#{} {}", task.id, task.title), Style::new().bold())])
}

/// Every keyboard shortcut of the screen it was opened from; enter or a click runs one.
fn draw_keys(menu: &KeysMenu, scroll: &ScrollHandle, follow: bool, cx: &mut Context<TaskerView>) -> Div {
    let title = match menu.back {
        Back::List => "Keyboard shortcuts · task list",
        Back::Issue(_) => "Keyboard shortcuts · task",
        Back::Tags(_) => "Keyboard shortcuts · tags",
    };
    if follow {
        scroll.scroll_to_item(menu.sel);
    }
    let bindings = menu.bindings();
    let items = bindings.iter().enumerate().map(|(index, b)| {
        div()
            .id(("binding", index))
            .flex()
            .items_center()
            .gap_3()
            .px_2()
            .py_1()
            .rounded_sm()
            .cursor_pointer()
            .when(index == menu.sel, |d| d.bg(rgb(theme::SELECTED)))
            .when(index != menu.sel, |d| d.hover(|s| s.bg(rgb(theme::HOVER))))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.follow = true;
                this.click(Click::Binding(index), cx);
            }))
            .child(
                div().w(px(64.)).flex_none().flex().child(
                    div()
                        .px_1p5()
                        .rounded_sm()
                        .border_1()
                        .border_color(rgb(BORDER))
                        .bg(rgb(theme::INPUT))
                        .text_size(px(12.))
                        .font_family(theme::EDITOR_FONT)
                        .child(b.key),
                ),
            )
            .child(div().flex_1().child(b.menu.unwrap_or_default()))
    });
    let list = div().id("keys").max_h(px(420.)).overflow_y_scroll().track_scroll(scroll).children(items);
    let buttons = vec![action("close", "Close", Kind::Primary, KeyCode::Esc, cx)];
    dialog(title, list, buttons).w(px(KEYS_WIDTH))
}

/// Multi-line editor for a description or a comment.
fn draw_editor(app: &App, edit: &Edit, scroll: &ScrollHandle, follow: bool, cx: &mut Context<TaskerView>) -> Div {
    let task = app.task(edit.target.task_id());
    let name = task.map(|t| format!("#{} {}", t.id, t.title)).unwrap_or_default();
    let what = match edit.target {
        Target::Description(_) => "Description".to_string(),
        Target::NewComment(_) => "New comment".to_string(),
        Target::Comment(_, index) => task
            .and_then(|t| t.comments.get(index))
            .map_or_else(|| "Comment".to_string(), |c| format!("Comment of {}", when(c.created))),
    };
    let modified = if edit.editor.modified { " •" } else { "" };

    // Long lines wrap; the cursor's line is kept in view while typing.
    let (row, col) = edit.editor.cursor();
    if follow {
        scroll.scroll_to_item(row);
    }
    let selection = edit.editor.selection();
    let mut lines = Vec::with_capacity(edit.editor.lines().len());
    let mut hits = Vec::with_capacity(lines.capacity());
    for (index, line) in edit.editor.lines().iter().enumerate() {
        // The part of this line inside the selection, if any.
        let selected = selection.and_then(|((r1, c1), (r2, c2))| {
            let len = line.chars().count();
            (r1..=r2).contains(&index).then_some((if index == r1 { c1 } else { 0 }, if index == r2 { c2 } else { len }))
        });
        let text = edited(line, selected).render();
        let cursor = (index == row).then(|| caret(&text, byte_at(line, col)));
        hits.push(Hit::new(&text, line.chars().count()));
        lines.push(div().child(text).children(cursor));
    }
    let text = div()
        .id(TextBox::Editor.id())
        .flex_1()
        .min_h(px(160.))
        .overflow_y_scroll()
        .track_scroll(scroll)
        .p_3()
        .rounded_md()
        .bg(rgb(theme::INPUT))
        .border_1()
        .border_color(rgb(ACCENT))
        .font_family(theme::EDITOR_FONT)
        .text_size(px(13.))
        .children(lines);
    let text = pointable(text, TextBox::Editor, hits, cx);
    let content = div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .gap_2()
        .child(dim(name).text_size(px(12.)))
        .child(text)
        .child(dim("Markdown is fine. Esc also saves and closes.").text_size(px(12.)));
    // ctrl+c discards in the terminal; here it copies, so the button names no shortcut.
    let discard = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    let buttons = vec![
        button("discard", "Discard changes", Kind::Secondary)
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.press(None, discard, cx))),
        action("save", "Save", Kind::Primary, KeyCode::Esc, cx),
    ];
    dialog(format!("{what}{modified}"), content, buttons).w(px(EDITOR_WIDTH)).h(relative(0.8))
}

/// A new name for a tag: the current chip, the name field, and what an existing name does.
fn draw_rename_tag(app: &App, rename: &RenameTag, cx: &mut Context<TaskerView>) -> Option<Div> {
    let from = app.all_tags().into_iter().nth(rename.sel)?;
    let content = div()
        .flex()
        .flex_col()
        .gap_3()
        .child(labeled("Current name", div().flex().child(tag_chip(app, &from))))
        .child(labeled("New name", text_field(TextBox::Rename, &rename.name, true, "", cx)))
        .child(dim("Renamed on every task. A name already in use merges the two tags.").text_size(px(12.)));
    let buttons = vec![
        action("cancel", "Cancel", Kind::Secondary, KeyCode::Esc, cx),
        action("rename", "Rename", Kind::Primary, KeyCode::Enter, cx),
    ];
    Some(dialog("Rename tag", content, buttons).w(px(DIALOG_WIDTH)))
}
