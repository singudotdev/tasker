//! The text cursor, as in desktop apps: a thin line over the text, blinking while nothing is typed.
//! It's drawn over the text instead of into it, so the text doesn't shift when it moves.

use super::theme::TEXT;
use gpui::{Bounds, Global, StyledText, canvas, fill, prelude::*, px, rgb, size};
use std::time::Duration;

/// How long the cursor stays shown, then hidden, while blinking.
pub const BLINK: Duration = Duration::from_millis(530);
/// Width of the cursor line, in pixels.
const WIDTH: f32 = 1.;

/// Whether the blinking cursor is in its shown half: one phase for every field, set by the root view.
pub struct Blink(pub bool);

impl Global for Blink {}

/// The cursor before byte `index` of `text`, drawn once the text is laid out. Hidden in the blink's off half
/// and while the window isn't focused.
pub fn caret(text: &StyledText, index: usize) -> impl IntoElement + use<> {
    let layout = text.layout().clone();
    canvas(
        |_, _, _| {},
        move |_, (), window, cx| {
            let shown = cx.try_global::<Blink>().is_none_or(|blink| blink.0);
            if !shown || !window.is_window_active() {
                return;
            }
            if let Some(at) = layout.position_for_index(index) {
                window.paint_quad(fill(Bounds::new(at, size(px(WIDTH), layout.line_height())), rgb(TEXT)));
            }
        },
    )
    .absolute()
    .size_0()
}
