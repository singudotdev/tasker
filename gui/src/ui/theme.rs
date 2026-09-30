//! Colors, sizes and text styles.
//!
//! [`Line`] is a row of [`Span`]s like ratatui's, for text mixing styles (a history entry, a field with its cursor),
//! and becomes one [`StyledText`] (wrapping as a whole) when drawn.

use gpui::{FontWeight, HighlightStyle, SharedString, StyledText, rgb};
use tasker_core::model::Status;

/// Window background.
pub const BG: u32 = 0x1b_1c21;
/// Panels, toolbars and cards.
pub const SURFACE: u32 = 0x23_252c;
/// Dialogs.
pub const POPUP: u32 = 0x28_2a32;
/// Text fields and the text editor.
pub const INPUT: u32 = 0x18_191e;
/// A hovered row or button.
pub const HOVER: u32 = 0x2e_313b;
/// Normal text.
pub const TEXT: u32 = 0xe3_e5ea;
/// Secondary text.
pub const DIM: u32 = 0x8b_919e;
/// Primary buttons, focused fields, selected items.
pub const ACCENT: u32 = 0x5b_8def;
/// A hovered primary button.
pub const ACCENT_HOVER: u32 = 0x74_a0f5;
/// Text on accent-colored backgrounds.
pub const ON_ACCENT: u32 = 0xff_ffff;
/// Doing.
pub const GREEN: u32 = 0x98_c379;
/// Parked and status bar messages.
pub const YELLOW: u32 = 0xe5_c07b;
/// Deleting.
pub const RED: u32 = 0xe0_6c75;
/// A hovered delete button.
pub const RED_HOVER: u32 = 0xe8_8890;
/// Borders and dividers.
pub const BORDER: u32 = 0x35_3945;
/// Background of the selected row.
pub const SELECTED: u32 = 0x2a_3650;
/// Background of selected text in a field or the text editor.
pub const SELECTION: u32 = 0x34_4f80;

/// Font of the whole window.
pub const UI_FONT: &str = ".SystemUIFont";
/// Font of the text editor, where descriptions and comments are written in markdown.
pub const EDITOR_FONT: &str = "monospace";

/// How a piece of text looks. `None` colors inherit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Style {
    /// Text color.
    pub fg: Option<u32>,
    /// Background color.
    pub bg: Option<u32>,
    /// Bold.
    pub bold: bool,
}

impl Style {
    /// No styling.
    pub const fn new() -> Self {
        Self { fg: None, bg: None, bold: false }
    }

    /// With text color `fg`.
    pub const fn fg(self, fg: u32) -> Self {
        Self { fg: Some(fg), ..self }
    }

    /// With background `bg`.
    pub const fn bg(self, bg: u32) -> Self {
        Self { bg: Some(bg), ..self }
    }

    /// Bold.
    pub const fn bold(self) -> Self {
        Self { bold: true, ..self }
    }

    /// The style as a GPUI highlight.
    fn highlight(self) -> HighlightStyle {
        HighlightStyle {
            color: self.fg.map(|c| rgb(c).into()),
            background_color: self.bg.map(|c| rgb(c).into()),
            font_weight: self.bold.then_some(FontWeight::BOLD),
            ..HighlightStyle::default()
        }
    }
}

/// Secondary text.
pub const DIM_STYLE: Style = Style::new().fg(DIM);

/// A status's color: doing green, todo plain, parked yellow, done dim.
pub fn status_color(status: Status) -> u32 {
    match status {
        Status::Doing => GREEN,
        Status::Todo => TEXT,
        Status::Parked => YELLOW,
        Status::Done => DIM,
    }
}

/// A status as a word for people: "To do", "Doing", "Parked", "Done".
pub fn status_label(status: Status) -> &'static str {
    match status {
        Status::Doing => "Doing",
        Status::Todo => "To do",
        Status::Parked => "Parked",
        Status::Done => "Done",
    }
}

/// Style for a task's status in text.
pub fn status(status: Status) -> Style {
    let style = Style::new().fg(status_color(status));
    if status == Status::Doing { style.bold() } else { style }
}

/// A piece of text in one style.
#[derive(Clone, Debug, Default)]
pub struct Span {
    /// The text.
    pub content: String,
    /// How it looks.
    pub style: Style,
}

impl Span {
    /// Text in `style`.
    pub fn styled(content: impl Into<String>, style: Style) -> Self {
        Self { content: content.into(), style }
    }

    /// Unstyled text.
    pub fn raw(content: impl Into<String>) -> Self {
        Self::styled(content, Style::new())
    }
}

/// A line of text made of spans; one text element when drawn, wrapping as a whole.
#[derive(Clone, Debug, Default)]
pub struct Line(pub Vec<Span>);

impl Line {
    /// A line of spans.
    pub fn from(spans: Vec<Span>) -> Self {
        Self(spans)
    }

    /// A line of one styled text.
    pub fn styled(content: impl Into<String>, style: Style) -> Self {
        Self(vec![Span::styled(content, style)])
    }

    /// A line of one unstyled text.
    pub fn raw(content: impl Into<String>) -> Self {
        Self(vec![Span::raw(content)])
    }

    /// The text element. Empty lines keep their height.
    pub fn render(self) -> StyledText {
        let mut text = String::new();
        let mut highlights = Vec::new();
        for span in self.0 {
            let start = text.len();
            text.push_str(&span.content);
            if span.style != Style::new() && text.len() > start {
                highlights.push((start..text.len(), span.style.highlight()));
            }
        }
        if text.is_empty() {
            text.push(' ');
        }
        StyledText::new(SharedString::from(text)).with_highlights(highlights)
    }
}
