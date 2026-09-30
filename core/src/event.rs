//! Keyboard input, as the app logic sees it.
//!
//! The same names and shapes as crossterm's types. Each frontend translates its own input into these:
//! the terminal UI from crossterm, the desktop app from GPUI.

/// A key, without modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
    /// A typed character (already upper case with shift).
    Char(char),
    /// `enter`.
    Enter,
    /// `esc`.
    Esc,
    /// `tab`.
    Tab,
    /// `backspace`.
    Backspace,
    /// `delete`.
    Delete,
    /// `←`.
    Left,
    /// `→`.
    Right,
    /// `↑`.
    Up,
    /// `↓`.
    Down,
    /// `home`.
    Home,
    /// `end`.
    End,
}

/// Modifier keys held with a key: ctrl for shortcuts, shift to select text while moving the cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyModifiers {
    /// Whether ctrl is held.
    control: bool,
    /// Whether shift is held. Typed characters already carry it (`A`); the terminal UI never sets it.
    shift: bool,
}

impl KeyModifiers {
    /// No modifiers.
    pub const NONE: Self = Self { control: false, shift: false };
    /// Ctrl.
    pub const CONTROL: Self = Self { control: true, shift: false };
    /// Shift.
    pub const SHIFT: Self = Self { control: false, shift: true };

    /// Whether every modifier in `other` is held.
    pub fn contains(self, other: Self) -> bool {
        (!other.control || self.control) && (!other.shift || self.shift)
    }
}

impl std::ops::BitOr for KeyModifiers {
    type Output = Self;

    /// Both sets of modifiers held together.
    fn bitor(self, other: Self) -> Self {
        Self { control: self.control || other.control, shift: self.shift || other.shift }
    }
}

/// A key press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEvent {
    /// The key.
    pub code: KeyCode,
    /// Modifiers held with it.
    pub modifiers: KeyModifiers,
}

impl KeyEvent {
    /// A key press of `code` with `modifiers`.
    pub const fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }
}
