//! The main screen: a toolbar (new task, search, filters), the task list, and a details panel
//! with the selected task's status, actions, description, comments and history.

use super::theme::{self, BORDER, DIM, Line, TEXT};
use super::widgets::{self, Kind, card, dim, section, segment, segmented, small, status_pill, tag_chips};
use super::{Scrolls, TaskerView, action, history_line, when};
use gpui::{AnyElement, ClickEvent, Context, Div, FontWeight, Window, div, prelude::*, px, rgb};
use tasker_core::app::{App, Click, DoneView, Mode};
use tasker_core::event::{KeyCode, KeyEvent, KeyModifiers};
use tasker_core::model::{Status, Task, fmt_age, now};

/// Wide enough for the list and the details panel side by side.
const SIDE_BY_SIDE_MIN_WIDTH: f32 = 900.;
/// Tall enough to stack the details panel under the list.
const STACKED_MIN_HEIGHT: f32 = 560.;
/// Height of the details panel when it's under the list.
const STACKED_DETAILS_HEIGHT: f32 = 300.;
/// How many recent status changes the details panel shows.
const RECENT_CHANGES: usize = 4;
/// Description lines shown in the details panel.
const DESCRIPTION_LINES: usize = 6;
/// Lines of the latest comment shown in the details panel.
const COMMENT_LINES: usize = 4;
/// The status buttons, in the order a task usually moves through them, with the key that sets each.
pub const STATUS_KEYS: [(Status, char); 4] =
    [(Status::Todo, 'b'), (Status::Doing, 's'), (Status::Parked, 'p'), (Status::Done, 'd')];

/// Toolbar, then the task list and the details panel: side by side when wide, stacked when tall,
/// the list only when small (double-click opens a task then).
pub fn draw(app: &App, window: &Window, scroll: &Scrolls, follow: bool, cx: &mut Context<TaskerView>) -> AnyElement {
    let viewport = window.viewport_size();
    let tasks = draw_tasks(app, scroll, follow, cx);
    let body = div().flex_1().min_h_0().flex().gap_3().p_3();
    let body = if f32::from(viewport.width) >= SIDE_BY_SIDE_MIN_WIDTH {
        let details_width = (f32::from(viewport.width) * 2. / 5.).clamp(340., 520.);
        body.child(tasks.flex_1().min_w_0()).child(draw_details(app, scroll, cx).w(px(details_width)).flex_none())
    } else if f32::from(viewport.height) >= STACKED_MIN_HEIGHT {
        body.flex_col()
            .child(tasks.flex_1())
            .child(draw_details(app, scroll, cx).h(px(STACKED_DETAILS_HEIGHT)).flex_none())
    } else {
        body.child(tasks.flex_1())
    };
    div().flex_1().min_h_0().flex().flex_col().child(toolbar(app, cx)).child(body).into_any_element()
}

/// New task, the search box, the filters, and the tags / reload / shortcuts buttons.
fn toolbar(app: &App, cx: &mut Context<TaskerView>) -> Div {
    let searching = matches!(app.mode, Mode::Search);
    let search = widgets::text_field("search", &app.search, searching, "Search tasks, or #tag")
        .w(px(260.))
        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.click(Click::Search, cx)));
    let clear = (!app.search.is_empty()).then(|| {
        let matches = app.visible().len();
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(widgets::small(action("clear-search", "Clear", Kind::Ghost, KeyCode::Esc, cx)))
            .child(dim(format!("{matches} match{}", if matches == 1 { "" } else { "es" })).text_size(px(12.)))
    });

    let done = app.tasks.iter().filter(|t| t.status == Status::Done).count();
    let filters = [
        (DoneView::Hide, "Active", app.tasks.len() - done),
        (DoneView::Show, "All", app.tasks.len()),
        (DoneView::Only, "Done", done),
    ]
    .map(|(view, label, count)| {
        segment(label, format!("{label}  {count}"), app.done_view == view)
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.click(Click::DoneView(view), cx)))
    });

    div()
        .flex()
        .flex_none()
        .flex_wrap()
        .items_center()
        .gap_2()
        .px_3()
        .py_2()
        .border_b_1()
        .border_color(rgb(BORDER))
        .bg(rgb(theme::SURFACE))
        .child(action("new-task", "+ New task", Kind::Primary, KeyCode::Char('n'), cx))
        .child(search)
        .children(clear)
        .child(segmented(filters))
        .child(div().flex_1())
        .child(action("tags", "Tags", Kind::Secondary, KeyCode::Char('t'), cx))
        .child(action("reload", "Reload", Kind::Ghost, KeyCode::Char('R'), cx))
        .child(action("shortcuts", "Shortcuts", Kind::Ghost, KeyCode::Char('?'), cx))
}

/// The task table in a card, or what to do when it's empty.
fn draw_tasks(app: &App, scroll: &Scrolls, follow: bool, cx: &mut Context<TaskerView>) -> Div {
    let visible = app.visible();
    let counts = Status::ALL
        .iter()
        .map(|status| {
            let count = app.tasks.iter().filter(|t| t.status == *status).count();
            format!("{count} {}", theme::status_label(*status).to_lowercase())
        })
        .collect::<Vec<_>>()
        .join(" · ");
    let header = div()
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .px_3()
        .py_2()
        .border_b_1()
        .border_color(rgb(BORDER))
        .child(div().font_weight(FontWeight::SEMIBOLD).child(list_title(app)))
        .child(dim(counts).text_size(px(12.)));

    if visible.is_empty() {
        return card().child(header).child(empty_state(app, cx));
    }

    let now = now();
    let columns = div()
        .flex()
        .gap_3()
        .px_3()
        .py_1()
        .text_size(px(12.))
        .text_color(rgb(DIM))
        .child(div().w(px(64.)).flex_none().child("Status"))
        .child(div().w(px(40.)).flex_none().child("ID"))
        .child(div().flex_1().child("Task"))
        .child(div().w(px(56.)).flex_none().text_right().child("Since"));
    if follow {
        scroll.list.scroll_to_item(app.selected);
    }
    let rows = visible.iter().enumerate().map(|(index, &i)| {
        let task = &app.tasks[i];
        let selected = index == app.selected;
        div()
            .id(("task", index))
            .flex()
            .items_center()
            .gap_3()
            .min_h(px(38.))
            .px_3()
            .border_b_1()
            .border_color(rgb(theme::BG))
            .cursor_pointer()
            .when(selected, |d| d.bg(rgb(theme::SELECTED)))
            .when(!selected, |d| d.hover(|s| s.bg(rgb(theme::HOVER))))
            .when(task.status == Status::Done, |d| d.text_color(rgb(DIM)))
            .on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
                let click = if event.click_count() >= 2 { Click::OpenRow(index) } else { Click::Row(index) };
                this.click(click, cx);
            }))
            .child(div().w(px(64.)).flex_none().child(status_pill(task.status)))
            .child(div().w(px(40.)).flex_none().text_color(rgb(DIM)).child(format!("#{}", task.id)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().min_w_0().truncate().child(task.title.clone()))
                    .child(tag_chips(app, &task.tags).flex_nowrap().flex_none()),
            )
            .child(
                div()
                    .w(px(56.))
                    .flex_none()
                    .text_right()
                    .text_size(px(12.))
                    .text_color(rgb(DIM))
                    .child(fmt_age(now - task.status_since())),
            )
    });
    card()
        .overflow_hidden()
        .child(header)
        .child(columns)
        .child(div().id("tasks").flex_1().min_h_0().overflow_y_scroll().track_scroll(&scroll.list).children(rows))
}

/// No tasks at all, or none matching: say so, and offer the way out.
fn empty_state(app: &App, cx: &mut Context<TaskerView>) -> Div {
    let centered = div().flex_1().flex().flex_col().items_center().justify_center().gap_2().p_6();
    if app.tasks.is_empty() {
        return centered
            .child(div().text_size(px(18.)).font_weight(FontWeight::SEMIBOLD).child("No tasks yet"))
            .child(dim("Create your first task to get started."))
            .child(div().pt_2().child(action("empty-new", "+ New task", Kind::Primary, KeyCode::Char('n'), cx)));
    }
    let clear =
        (!app.search.is_empty()).then(|| action("empty-clear", "Clear search", Kind::Secondary, KeyCode::Esc, cx));
    let show_all = (app.done_view != DoneView::Show).then(|| {
        widgets::button("empty-all", "Show all tasks", Kind::Secondary)
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.click(Click::DoneView(DoneView::Show), cx)))
    });
    centered
        .child(div().text_size(px(18.)).font_weight(FontWeight::SEMIBOLD).child("No matching tasks"))
        .child(dim("Try another search or filter."))
        .child(div().flex().gap_2().pt_2().children(clear).children(show_all))
}

/// The list's heading: which tasks it shows.
fn list_title(app: &App) -> &'static str {
    match app.done_view {
        DoneView::Hide => "Active tasks",
        DoneView::Show => "All tasks",
        DoneView::Only => "Done tasks",
    }
}

/// The selected task at a glance, with buttons for everything that can be done to it.
fn draw_details(app: &App, scroll: &Scrolls, cx: &mut Context<TaskerView>) -> Div {
    let Some(task) = app.selected_task() else {
        return card().items_center().justify_center().child(dim("Select a task to see its details."));
    };
    let content = div()
        .id("details")
        .flex_1()
        .min_h_0()
        .overflow_y_scroll()
        .track_scroll(&scroll.details)
        .flex()
        .flex_col()
        .gap_4()
        .p_4()
        .child(title_block(app, task))
        .child(status_block(task, cx))
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_2()
                .child(action("open", "Open", Kind::Primary, KeyCode::Char('v'), cx))
                .child(action("edit", "Edit", Kind::Secondary, KeyCode::Char('e'), cx))
                .child(action("open-file", "Open file", Kind::Secondary, KeyCode::Char('o'), cx))
                .child(action("delete", "Delete", Kind::Danger, KeyCode::Char('x'), cx)),
        )
        .child(divider())
        .child(description_block(task, cx))
        .child(comments_block(task, cx))
        .child(history_block(task, cx));
    card().child(content)
}

/// Title, id and creation date, and tags.
fn title_block(app: &App, task: &Task) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(div().text_size(px(18.)).font_weight(FontWeight::SEMIBOLD).child(task.title.clone()))
        .child(dim(format!("#{} · created {}", task.id, when(task.created))).text_size(px(12.)))
        .when(!task.tags.is_empty(), |d| d.child(tag_chips(app, &task.tags).pt_1()))
}

/// The status buttons (the current one selected) and how long the task has had it.
pub fn status_block(task: &Task, cx: &mut Context<TaskerView>) -> Div {
    let segments = STATUS_KEYS.map(|(status, key)| {
        segment(("status", key as usize), theme::status_label(status), task.status == status)
            .tooltip(widgets::tooltip(format!("Shortcut: {key}")))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.press(None, KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE), cx);
            }))
    });
    let since = task.status_since();
    div().flex().flex_col().gap_1p5().child(segmented(segments).self_start()).child(
        dim(format!("{} for {}, since {}", theme::status_label(task.status), fmt_age(now() - since), when(since)))
            .text_size(px(12.)),
    )
}

/// The first lines of the description, and a button to edit it.
fn description_block(task: &Task, cx: &mut Context<TaskerView>) -> Div {
    let edit = small(action("edit-description", "Edit", Kind::Secondary, KeyCode::Char('m'), cx));
    let body = if task.description.is_empty() {
        dim("No description yet.")
    } else {
        let more = task.description.lines().count() > DESCRIPTION_LINES;
        paragraph(task.description.lines().take(DESCRIPTION_LINES)).when(more, |d| {
            d.child(small(action("read-more", "Read more", Kind::Ghost, KeyCode::Char('v'), cx)).self_start())
        })
    };
    div().flex().flex_col().gap_2().child(section("Description", [edit])).child(body)
}

/// The latest comment, and buttons to add one or see them all.
fn comments_block(task: &Task, cx: &mut Context<TaskerView>) -> Div {
    let add = small(action("add-comment", "Add", Kind::Secondary, KeyCode::Char('c'), cx));
    let body = match task.comments.last() {
        None => dim("No comments yet."),
        Some(comment) => div()
            .flex()
            .flex_col()
            .gap_1()
            .p_2()
            .rounded_md()
            .bg(rgb(theme::BG))
            .child(dim(format!("Latest · {}", when(comment.created))).text_size(px(12.)))
            .child(paragraph(comment.body.lines().take(COMMENT_LINES))),
    };
    let all = (task.comments.len() > 1).then(|| {
        small(action("all-comments", format!("View all {}", task.comments.len()), Kind::Ghost, KeyCode::Char('v'), cx))
            .self_start()
    });
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(section(format!("Comments ({})", task.comments.len()), [add]))
        .child(body)
        .children(all)
}

/// The latest status changes.
fn history_block(task: &Task, cx: &mut Context<TaskerView>) -> Div {
    let changes = task.history.iter().rev().take(RECENT_CHANGES).map(|change| history_line(change).render());
    let older = task.history.len().saturating_sub(RECENT_CHANGES);
    div()
        .flex()
        .flex_col()
        .gap_1()
        .text_size(px(13.))
        .child(section("History", []))
        .when(task.history.is_empty(), |d| d.child(dim("No status changes yet.")))
        .children(changes)
        .when(older > 0, |d| {
            d.child(small(action("older", format!("{older} older…"), Kind::Ghost, KeyCode::Char('v'), cx)).self_start())
        })
}

/// Lines of text; empty lines keep their height.
pub fn paragraph<'a>(lines: impl Iterator<Item = &'a str>) -> Div {
    div().flex().flex_col().text_color(rgb(TEXT)).children(lines.map(|l| Line::raw(l.to_string()).render()))
}

/// A thin horizontal line.
pub fn divider() -> Div {
    div().h(px(1.)).flex_none().bg(rgb(BORDER))
}
