// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Table axes: a preceding position and a trailing position, which are not the same shape.
//!
//! Appendix B's legend gives Table 1 a last row labeled "line head" and a last column
//! labeled "line end" — one row and one column, not a symmetric axis — and Appendix C
//! names no line-edge axis at all. `Before` and `After` hold that asymmetry as two types
//! rather than as one type with an invalid half, so a query for `After::LineHead` is a
//! compile error rather than a lookup that always misses.
//!
//! Both are exhaustive (`docs/api-frozen.toml`'s `[[exempt]]`): §B.1 gives Table 1 exactly
//! one line-head row and one line-end column, so a caller matching every class and the one
//! line edge is not writing a catch-all over a set that could grow.

use jlreq_class::Class;

/// A preceding position: a class, or the line head.
///
/// JLReq: §B.1
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Before {
    /// One of the thirty character classes.
    Class(Class),
    /// The start of a line, or of an inline cutting note (warichu, 割注).
    LineHead,
}

/// A trailing position: a class, or the line end.
///
/// There is deliberately no `After::LineHead`: the specification gives the line head one
/// row and the line end one column, and no matrix has both.
///
/// JLReq: §B.1
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum After {
    /// One of the thirty character classes.
    Class(Class),
    /// The end of a line, or of an inline cutting note (warichu, 割注).
    LineEnd,
}

impl Before {
    /// The raw sentinel form `crates/jlreq-spacing/src/raw.rs`'s generated tables use:
    /// `0` for the line head, `1..=30` for a class ordinal.
    pub(crate) fn raw(self) -> u8 {
        match self {
            Self::Class(class) => class.number(),
            Self::LineHead => crate::raw::LINE_EDGE,
        }
    }

    /// The class here, or `None` at the line head.
    pub(crate) const fn class(self) -> Option<Class> {
        match self {
            Self::Class(class) => Some(class),
            Self::LineHead => None,
        }
    }
}

impl After {
    /// The raw sentinel form: `0` for the line end, `1..=30` for a class ordinal.
    pub(crate) fn raw(self) -> u8 {
        match self {
            Self::Class(class) => class.number(),
            Self::LineEnd => crate::raw::LINE_EDGE,
        }
    }

    /// The class here, or `None` at the line end.
    pub(crate) const fn class(self) -> Option<Class> {
        match self {
            Self::Class(class) => Some(class),
            Self::LineEnd => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use jlreq_class::Class;

    use super::{After, Before};

    #[test]
    fn a_class_position_carries_its_ordinal() {
        assert_eq!(Before::Class(Class::MiddleDot).raw(), 5);
        assert_eq!(After::Class(Class::MiddleDot).raw(), 5);
    }

    #[test]
    fn the_line_edges_are_the_sentinel() {
        assert_eq!(Before::LineHead.raw(), 0);
        assert_eq!(After::LineEnd.raw(), 0);
    }

    #[test]
    fn there_is_no_after_line_head_or_before_line_end() {
        // The absence itself is the property under test: `After` has no `LineHead`
        // variant and `Before` has no `LineEnd` variant, so the following match is
        // exhaustive without one. If a variant were ever added this would fail to
        // compile rather than silently pass, which is the point.
        let after = After::Class(Class::OpeningBracket);
        match after {
            After::Class(_) | After::LineEnd => {},
        }
        let before = Before::Class(Class::OpeningBracket);
        match before {
            Before::Class(_) | Before::LineHead => {},
        }
    }
}
