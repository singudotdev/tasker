//! Tag colors: a 16-color palette assigned without repeats, stored in `tags.md`.

use std::fmt::Write;
use std::hash::{DefaultHasher, Hash, Hasher};

/// Named xterm-256 colors, so they render the same in terminals without truecolor support.
pub const PALETTE: [(&str, u8); 16] = [
    ("red", 160),
    ("orange", 166),
    ("gold", 178),
    ("green", 70),
    ("emerald", 35),
    ("teal", 37),
    ("blue", 32),
    ("indigo", 62),
    ("purple", 98),
    ("magenta", 133),
    ("pink", 168),
    ("brown", 130),
    ("steel", 67),
    ("olive", 107),
    ("salmon", 174),
    ("sky", 110),
];

/// A tag's color: a palette entry or a custom hex color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TagColor {
    /// Index into `PALETTE`.
    Palette(usize),
    /// A custom `#rrggbb` color (needs a truecolor terminal).
    Rgb(u8, u8, u8),
}

impl TagColor {
    /// Parses a palette name (`red`) or a hex color (`#33aaff`).
    fn parse(s: &str) -> Option<Self> {
        let s = s.trim().to_ascii_lowercase();
        if let Some(i) = PALETTE.iter().position(|(name, _)| *name == s) {
            return Some(TagColor::Palette(i));
        }
        let hex = s.strip_prefix('#')?;
        if hex.len() != 6 {
            return None;
        }
        let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
        Some(TagColor::Rgb(byte(0)?, byte(2)?, byte(4)?))
    }

    /// How the color is written in `tags.md`.
    fn name(self) -> String {
        match self {
            TagColor::Palette(i) => PALETTE[i].0.to_string(),
            TagColor::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        }
    }

    /// The color's red, green and blue values, to pick a readable text color.
    fn rgb(self) -> (u8, u8, u8) {
        match self {
            TagColor::Rgb(r, g, b) => (r, g, b),
            TagColor::Palette(i) => {
                // xterm 6×6×6 color cube.
                const LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];
                let n = usize::from(PALETTE[i].1) - 16;
                (LEVELS[n / 36], LEVELS[n / 6 % 6], LEVELS[n % 6])
            }
        }
    }
}

/// Persistent tag → color assignments, stored as `tags.md` in the data dir.
#[derive(Debug, Default)]
pub struct TagColors {
    /// In the order they were assigned, which is the order of `tags.md`.
    entries: Vec<(String, TagColor)>,
}

impl TagColors {
    /// Reads `tags.md`; lines that aren't `- tag: color` are ignored.
    pub fn parse(text: &str) -> Self {
        let entries = text
            .lines()
            .filter_map(|line| line.trim().strip_prefix("- ")?.split_once(':'))
            .filter_map(|(tag, color)| {
                let tag = tag.trim().trim_start_matches('#').to_lowercase();
                Some((tag, TagColor::parse(color)?))
            })
            .collect();
        Self { entries }
    }

    /// The contents of `tags.md`, with a header listing the palette names.
    pub fn to_markdown(&self) -> String {
        let names: Vec<&str> = PALETTE.iter().map(|(name, _)| *name).collect();
        let entries = self.entries.iter().fold(String::new(), |mut out, (tag, color)| {
            writeln!(out, "- {tag}: {}", color.name()).expect("writing to a String cannot fail");
            out
        });
        format!("# Tag colors\n\nEdit freely. Colors: {}, or #rrggbb.\n\n{entries}", names.join(", "))
    }

    /// The color of `tag`, if one is assigned.
    pub fn get(&self, tag: &str) -> Option<TagColor> {
        self.entries.iter().find(|(t, _)| t == tag).map(|(_, c)| *c)
    }

    /// Gives every uncolored tag a random color among the least used ones,
    /// so colors only repeat once the whole palette is taken. Returns whether anything changed.
    pub fn assign<'a>(&mut self, tags: impl IntoIterator<Item = &'a String>) -> bool {
        let mut changed = false;
        for tag in tags {
            if self.get(tag).is_some() {
                continue;
            }
            // How many tags already use each palette color (custom hex colors don't count).
            let mut uses = [0usize; PALETTE.len()];
            for (_, c) in &self.entries {
                if let TagColor::Palette(i) = c {
                    uses[*i] += 1;
                }
            }
            // Only the least used colors are candidates, so none repeats until all are taken.
            let least = *uses.iter().min().expect("palette is not empty");
            let candidates: Vec<usize> = (0..PALETTE.len()).filter(|&i| uses[i] == least).collect();
            // Pick one "randomly" but reproducibly: from a hash of the tag name.
            let mut h = DefaultHasher::new();
            tag.hash(&mut h);
            let pick = candidates[(h.finish() % candidates.len() as u64) as usize];
            self.entries.push((tag.clone(), TagColor::Palette(pick)));
            changed = true;
        }
        changed
    }

    /// The color's name as written in `tags.md` (`red`, `#33aaff`), if one is assigned.
    pub fn name(&self, tag: &str) -> Option<String> {
        self.get(tag).map(TagColor::name)
    }

    /// Moves `from`'s color to `to`. If `to` already has a color it keeps it, and `from`'s is dropped.
    pub fn rename(&mut self, from: &str, to: &str) {
        if self.get(to).is_some() {
            self.remove(from);
        } else if let Some(entry) = self.entries.iter_mut().find(|(t, _)| t == from) {
            entry.0 = to.to_string();
        }
    }

    /// Forgets `tag`'s color.
    pub fn remove(&mut self, tag: &str) {
        self.entries.retain(|(t, _)| t != tag);
    }

    /// Gives `tag` the next palette color (or the previous one, going `back`).
    /// A custom hex color steps to the first (or last) palette color.
    pub fn cycle(&mut self, tag: &str, back: bool) {
        let n = PALETTE.len();
        let next = match self.get(tag) {
            Some(TagColor::Palette(i)) if back => (i + n - 1) % n,
            Some(TagColor::Palette(i)) => (i + 1) % n,
            _ if back => n - 1,
            _ => 0,
        };
        match self.entries.iter_mut().find(|(t, _)| t == tag) {
            Some(entry) => entry.1 = TagColor::Palette(next),
            None => self.entries.push((tag.to_string(), TagColor::Palette(next))),
        }
    }

    /// Chip colors as `0xrrggbb`: the tag color as background and a readable text color on top.
    /// `None` when the tag has no color assigned.
    pub fn chip(&self, tag: &str) -> Option<(u32, u32)> {
        let (r, g, b) = self.get(tag)?.rgb();
        let luma = 0.299 * f32::from(r) + 0.587 * f32::from(g) + 0.114 * f32::from(b);
        let fg = if luma > 140.0 { 0x00_0000 } else { 0xff_ffff };
        Some(((u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b), fg))
    }
}

#[cfg(test)]
#[path = "../tests/tags.rs"]
mod tests;
