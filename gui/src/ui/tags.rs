//! The tags screen: every tag with its color and how many tasks use it per status,
//! with buttons to recolor, rename or delete it.

use super::theme::{self, BORDER, DIM};
use super::widgets::{Kind, card, dim, small, tag_chip};
use super::{TaskerView, action, action_after};
use gpui::{AnyElement, ClickEvent, Context, Div, FontWeight, ScrollHandle, div, prelude::*, px, rgb};
use tasker_core::app::{App, Back, Click, Confirm, KeysMenu, Mode, TagsView};
use tasker_core::event::{KeyCode, KeyEvent, KeyModifiers};
use tasker_core::model::Status;

/// Width of the tag column.
const TAG_WIDTH: f32 = 200.;
/// Width of the color column: the name between the previous / next buttons.
const COLOR_WIDTH: f32 = 150.;
/// Width of each count column.
const COUNT_WIDTH: f32 = 64.;

/// The tags screen to draw behind dialogs that were opened from it.
pub fn shown_behind(mode: &Mode) -> Option<TagsView> {
    match mode {
        Mode::Tags(view) | Mode::Keys(KeysMenu { back: Back::Tags(view), .. }) => Some(*view),
        Mode::RenameTag(rename) => Some(TagsView { sel: rename.sel }),
        Mode::Confirm(Confirm::DeleteTag(sel)) => Some(TagsView { sel: *sel }),
        _ => None,
    }
}

/// A back button, then one row per tag, alphabetically.
pub fn draw(
    app: &App,
    view: TagsView,
    scroll: &ScrollHandle,
    follow: bool,
    cx: &mut Context<TaskerView>,
) -> AnyElement {
    let tags = app.all_tags();
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
        .child(div().font_weight(FontWeight::SEMIBOLD).child("Tags"))
        .child(dim(format!("{}", tags.len())))
        .child(div().flex_1())
        .child(action("shortcuts", "Shortcuts", Kind::Ghost, KeyCode::Char('?'), cx));

    let body = if tags.is_empty() {
        card()
            .flex_1()
            .items_center()
            .justify_center()
            .gap_2()
            .child(div().text_size(px(18.)).font_weight(FontWeight::SEMIBOLD).child("No tags yet"))
            .child(dim("Add tags to a task with its Edit button."))
    } else {
        let mut header = row().py_1().text_size(px(12.)).text_color(rgb(DIM));
        header = header.child(div().w(px(TAG_WIDTH)).flex_none().child("Tag"));
        header = header.child(div().w(px(COLOR_WIDTH)).flex_none().child("Color"));
        header = header.children(Status::ALL.iter().map(|s| count_cell(theme::status_label(*s).to_string())));

        let sel = view.sel.min(tags.len() - 1);
        if follow {
            scroll.scroll_to_item(sel);
        }
        let rows = tags.iter().enumerate().map(|(index, tag)| {
            let select = Some(Click::Tag(index));
            let key = |c| KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE);
            let color = div()
                .w(px(COLOR_WIDTH))
                .flex_none()
                .flex()
                .items_center()
                .gap_1()
                .child(small(action_after(("prev-color", index), "‹", Kind::Ghost, select, key('C'), cx)))
                .child(div().w(px(64.)).text_center().child(app.tag_colors.name(tag).unwrap_or_default()))
                .child(small(action_after(("next-color", index), "›", Kind::Ghost, select, key('c'), cx)));
            let counts = Status::ALL.iter().map(|status| {
                let count = app.tasks.iter().filter(|t| t.status == *status && t.tags.contains(tag)).count();
                count_cell(count.to_string())
            });
            let buttons = div()
                .flex()
                .flex_1()
                .justify_end()
                .gap_1()
                .child(small(action_after(("rename", index), "Rename", Kind::Secondary, select, key('r'), cx)))
                .child(small(action_after(("delete", index), "Delete", Kind::Ghost, select, key('x'), cx)));
            row()
                .id(("tag", index))
                .min_h(px(40.))
                .border_b_1()
                .border_color(rgb(theme::BG))
                .cursor_pointer()
                .when(index == sel, |d| d.bg(rgb(theme::SELECTED)))
                .when(index != sel, |d| d.hover(|s| s.bg(rgb(theme::HOVER))))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.click(Click::Tag(index), cx)))
                .child(div().w(px(TAG_WIDTH)).flex_none().flex().child(tag_chip(app, tag)))
                .child(color)
                .children(counts)
                .child(buttons)
        });
        card()
            .flex_1()
            .overflow_hidden()
            .child(header.border_b_1().border_color(rgb(BORDER)))
            .child(div().id("tags").flex_1().min_h_0().overflow_y_scroll().track_scroll(scroll).children(rows))
    };
    div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .child(bar)
        .child(div().flex_1().min_h_0().flex().p_3().child(body))
        .into_any_element()
}

/// A table row's layout.
fn row() -> Div {
    div().flex().items_center().gap_3().px_3().whitespace_nowrap()
}

/// A right-aligned count column.
fn count_cell(text: String) -> Div {
    div().w(px(COUNT_WIDTH)).flex_none().text_right().child(text)
}
