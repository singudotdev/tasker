//! Enums whose variants are listed in exactly one place.
//!
//! Adding, removing, renaming or reordering a variant is a one-line change: the list of all
//! variants, their order, and their names (for files, screen and command line) are generated
//! from the same declaration, so they can't get out of sync.

/// Declares an enum and generates, from its single list of variants:
/// - `ALL`: every variant, in declaration order;
/// - `position()`: a variant's place in that order (usable as a sort key);
/// - `next()`: the following variant, wrapping around after the last (for keys that cycle).
///
/// The enum must derive `Clone`, `Copy` and `PartialEq`.
macro_rules! ordered_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $( $(#[$variant_meta:meta])* $variant:ident ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        $vis enum $name {
            $( $(#[$variant_meta])* $variant ),+
        }

        impl $name {
            /// Every variant, in declaration order.
            pub const ALL: &[Self] = &[$( Self::$variant ),+];

            /// Place in [`Self::ALL`], i.e. in declaration order.
            pub fn position(self) -> usize {
                Self::ALL.iter().position(|v| *v == self).expect("ALL lists every variant")
            }

            /// The next variant in declaration order, wrapping around after the last.
            #[allow(dead_code)] // not every enum cycles through its variants
            pub fn next(self) -> Self {
                Self::ALL[(self.position() + 1) % Self::ALL.len()]
            }
        }
    };
}

/// Like [`ordered_enum!`], with a text name per variant (`Variant => "name"`). Also generates:
/// - `as_str()`: the name, as written in files, shown on screen and typed on the command line;
/// - `parse()`: the variant with a given name — the exact inverse of `as_str`;
/// - `names()`: every name joined by a separator, for help texts (`hour|day|week|month`).
macro_rules! named_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $( $(#[$variant_meta:meta])* $variant:ident => $text:literal ),+ $(,)?
        }
    ) => {
        $crate::enums::ordered_enum! {
            $(#[$meta])*
            $vis enum $name {
                $( $(#[$variant_meta])* $variant ),+
            }
        }

        impl $name {
            /// The variant's name, as written in files, shown on screen and typed on the command line.
            pub fn as_str(self) -> &'static str {
                match self {
                    $( Self::$variant => $text ),+
                }
            }

            /// The variant whose name is `s` (surrounding whitespace ignored); the inverse of `as_str`.
            pub fn parse(s: &str) -> Option<Self> {
                Self::ALL.iter().copied().find(|v| v.as_str() == s.trim())
            }

            /// Every name joined by `separator`, e.g. `hour|day|week|month`.
            #[allow(dead_code)] // only enums that appear in help texts need it
            pub fn names(separator: &str) -> String {
                Self::ALL.iter().map(|v| v.as_str()).collect::<Vec<_>>().join(separator)
            }
        }
    };
}

pub(crate) use {named_enum, ordered_enum};

#[cfg(test)]
#[path = "../tests/enums.rs"]
mod tests;
