//! The task view: a task laid out like a GitHub/GitLab issue, with its status, description, history
//! and comments, each with its buttons.

use super::list::{divider, paragraph, status_block};
use super::theme::{self, ACCENT, BORDER, DIM};
use super::widgets::{Kind, card, dim, section, small, status_pill, tag_chips};
use super::{TaskerView, action, action_after, history_line, menu, when};
use gpui::{
    AnyElement, ClickEvent, Context, Div, FontWeight, MouseButton, MouseDownEvent, ScrollHandle, Stateful, div,
    prelude::*, px, rgb,
};
use tasker_core::app::{App, Back, Click, Confirm, Edit, IssueView, KeysMenu, Mode};
use tasker_core::event::{KeyCode, KeyEvent, KeyModifiers};
use tasker_core::model::{Comment, fmt_age, now};

/// Widest the task view's content gets, so lines stay readable on wide windows.
const MAX_WIDTH: f32 = 860.;

/// The task view to draw behind dialogs that were opened from it.
pub fn shown_behind(mode: &Mode) -> Option<IssueView> {
    match mode {
        Mode::Issue(view) | Mode::Keys(KeysMenu { back: Back::Issue(view), .. }) => Some(*view),
        Mode::Edit(Edit { back, .. }) => *back,
        Mode::Confirm(Confirm::DeleteComment(id, index)) => Some(IssueView { id: *id, sel: *index }),
        _ => None,
    }
}

/// A back button, then the task: everything above the comments, then one card per comment,
/// scrolled so the selected comment is visible after the keyboard moved it.
pub fn draw(
    app: &App,
    view: IssueView,
    scroll: &ScrollHandle,
    follow: bool,
    cx: &mut Context<TaskerView>,
) -> AnyElement {
    let Some(task) = app.task(view.id) else { return div().into_any_element() };
    let bar = div()
        .flex()
        .flex_none()
        .items_center()
        .gap_3()
        .px_3()
        .py_2()
        .border_b_1()
        .border_color(rgb(BORDER))
        .bg(rgb(theme::SURFACE))
        .child(action("back", "← Back to tasks", Kind::Ghost, KeyCode::Esc, cx))
        .child(dim(format!("Task #{}", task.id)))
        .child(div().flex_1())
        .child(action("shortcuts", "Shortcuts", Kind::Ghost, KeyCode::Char('?'), cx));

    let meta = div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap_2()
        .child(status_pill(task.status))
        .child(tag_chips(app, &task.tags))
        .child(
            dim(format!(
                "{} for {} · created {}",
                theme::status_label(task.status),
                fmt_age(now() - task.status_since()),
                when(task.created)
            ))
            .text_size(px(12.)),
        );
    let description =
        if task.description.is_empty() { dim("No description yet.") } else { paragraph(task.description.lines()) };
    let history = div()
        .flex()
        .flex_col()
        .gap_1()
        .text_size(px(13.))
        .when(task.history.is_empty(), |d| {
            d.child(dim(format!("{} since it was created.", theme::status_label(task.status))))
        })
        .children(task.history.iter().map(|change| history_line(change).render()));

    // The first child is everything above the comments; comment `i` is child `i + 1`.
    let top = div()
        .flex()
        .flex_col()
        .gap_4()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .text_size(px(22.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(div().text_color(rgb(DIM)).child(format!("#{}", task.id)))
                        .child(div().child(task.title.clone())),
                )
                .child(meta),
        )
        .child(status_block(task, cx))
        .child(
            card()
                .p_4()
                .gap_2()
                .child(
                    section(
                        "Description",
                        [small(action("edit-description", "Edit", Kind::Secondary, KeyCode::Char('m'), cx))],
                    )
                    .pb_1(),
                )
                .child(description),
        )
        .child(card().p_4().gap_2().child(section("History", [])).child(history))
        .child(divider())
        .child(section(
            format!("Comments ({})", task.comments.len()),
            [action("add-comment", "Add comment", Kind::Primary, KeyCode::Char('c'), cx)],
        ))
        .when(task.comments.is_empty(), |d| d.child(dim("No comments yet.")));

    let sel = view.sel.min(task.comments.len().saturating_sub(1));
    let comments =
        task.comments.iter().enumerate().map(|(index, comment)| draw_comment(comment, index, index == sel, cx));
    if follow && !task.comments.is_empty() {
        scroll.scroll_to_item(sel + 1);
    }
    let content = div()
        .id("issue")
        .flex_1()
        .min_h_0()
        .overflow_y_scroll()
        .track_scroll(scroll)
        .child(div().w_full().max_w(px(MAX_WIDTH)).mx_auto().p_4().child(top))
        .children(comments.map(|comment| div().w_full().max_w(px(MAX_WIDTH)).mx_auto().px_4().pb_3().child(comment)));
    div().flex_1().min_h_0().flex().flex_col().child(bar).child(content).into_any_element()
}

/// A comment card: date, edited, Edit and Delete buttons, and the text. Clicking it selects it;
/// right-clicking also opens a menu with Edit and Delete.
fn draw_comment(comment: &Comment, index: usize, selected: bool, cx: &mut Context<TaskerView>) -> Stateful<Div> {
    let mut header = vec![when(comment.created)];
    if let Some(edited) = comment.edited {
        let format = if edited.date_naive() == comment.created.date_naive() { "%H:%M" } else { "%a %d %b %Y %H:%M" };
        header.push(format!("edited {}", edited.format(format)));
    }
    let key = |c| KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE);
    let select = Some(Click::Comment(index));
    let buttons = div()
        .flex()
        .gap_1()
        .child(small(action_after(("edit-comment", index), "Edit", Kind::Secondary, select, key('e'), cx)))
        .child(small(action_after(("delete-comment", index), "Delete", Kind::Ghost, select, key('x'), cx)));
    card()
        .id(("comment", index))
        .p_3()
        .gap_2()
        .when(selected, |d| d.border_color(rgb(ACCENT)))
        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.click(Click::Comment(index), cx)))
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                this.app.click(Click::Comment(index));
                this.open_menu(event.position, menu::for_comment(), cx);
            }),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(dim(header.join(" · ")).text_size(px(12.)))
                .child(buttons),
        )
        .child(paragraph(comment.body.lines()))
}
