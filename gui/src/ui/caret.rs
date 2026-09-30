//! The text cursor, as in desktop apps: a thin line over the text, blinking while nothing is typed.
//! It's drawn over the text instead of into it, so the text doesn't shift when it moves.

use super::theme::TEXT;
use gpui::{Bounds, Global, StyledText, canvas, fill, point, prelude::*, px, rgb, size};
use std::time::Duration;

/// How long the cursor stays shown, then hidden, while blinking.
pub const BLINK: Duration = Duration::from_millis(530);
/// Width of the cursor line, in pixels.
const WIDTH: f32 = 1.;
/// Height of the cursor line, relative to the font size: about a glyph's ascent plus descent.
/// The line height is taller (GPUI's default is about 1.6 font sizes), so the cursor is centered in it.
const HEIGHT: f32 = 1.2;

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
            let Some(at) = layout.position_for_index(index) else { return };
            // The canvas is inside the text's element, so this is the text's own size.
            let font_size = window.text_style().font_size.to_pixels(window.rem_size());
            let line_height = layout.line_height();
            let height = (font_size * HEIGHT).min(line_height);
            let top = at + point(px(0.), (line_height - height) / 2.);
            window.paint_quad(fill(Bounds::new(top, size(px(WIDTH), height)), rgb(TEXT)));
        },
    )
    .absolute()
    .size_0()
}
