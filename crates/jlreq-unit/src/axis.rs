// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The four axis types, the two ends of each axis, and the writing direction.
//!
//! Each of the four lengths is a distinct type over an `i32` in the caller's unit, with
//! `new` and `units` and the inherent arithmetic of [`crate::arith`], and **nothing
//! else**. There is no conversion between any two of them and none between any of them
//! and [`crate::Advance`]; `docs/api-frozen.toml` makes that mechanical.
//!
//! `new` and `units` are themselves the residue, and pretending otherwise would be worse
//! than the leak. `BlockExtent::new(inline.units())` is a cross-axis assignment in two
//! well-typed steps, and no arrangement of types removes it: a value the caller supplies
//! must get in, and a position the caller draws must get out. So the untyped channel is
//! narrowed instead — this module and [`crate::length`] are its only home, and every
//! other site in the workspace is a reviewed entry in `docs/scalar-sites.toml`
//! (see `docs/adr/0011`).
//!
//! The four fields are private and not `pub(crate)`, because `InlineExtent(raw)` and
//! `value.0` would be a second channel that names no method and that the `ops` gate's
//! method list therefore cannot reach. `of` and `raw` are that channel with a name: they
//! are crate-visible, they are in the gate's method list, and every use of them outside
//! this module and [`crate::length`] is a reviewed site like any other.

use crate::arith::closed_arithmetic;
use crate::length::{LOWER, UPPER};

/// Generate the untyped channel for one axis type: the neutral value, the constructor
/// from the caller's own integer, the reader back into it, and the crate-visible pair the
/// rest of `jlreq-unit` reaches the channel by.
///
/// One macro rather than four copies, because the four are identical by decision and a
/// copy that drifts is the leak this separation exists to prevent. It lives here because
/// this module is one of the two the `ops` gate permits the channel in.
macro_rules! axis_scalar {
    ($type:ident, $what:literal) => {
        impl $type {
            #[doc = concat!("No ", $what, " at all.")]
            ///
            /// JLReq: n/a (arithmetic)
            pub const ZERO: Self = Self(0);

            #[doc = concat!("A ", $what, " of `units` in the caller's own unit.")]
            ///
            /// `None` beyond [`crate::Advance::LIMIT`], which every length in the
            /// workspace shares.
            ///
            /// JLReq: n/a (arithmetic)
            #[must_use]
            pub const fn new(units: i32) -> Option<Self> {
                if units > UPPER || units < LOWER {
                    None
                } else {
                    Some(Self::of(units))
                }
            }

            #[doc = concat!("The ", $what, ", in the caller's own unit.")]
            ///
            /// JLReq: n/a (arithmetic)
            #[must_use]
            pub const fn units(self) -> i32 {
                self.raw()
            }

            #[doc = concat!("The crate-visible entry half of the channel: a ", $what)]
            #[doc = concat!(" from a count this crate has itself computed, clamped into ")]
            #[doc = concat!("the shared bound rather than refused, because a call site ")]
            #[doc = concat!("that had to handle `None` here would invent a fallback ")]
            #[doc = concat!("length. `new` is this with the bound reported instead.")]
            pub(crate) const fn of(units: i32) -> Self {
                Self($crate::length::bounded(units))
            }

            #[doc = concat!("The crate-visible exit half: the ", $what, " as a plain ")]
            #[doc = concat!("integer. `units` is this under its published name.")]
            pub(crate) const fn raw(self) -> i32 {
                self.0
            }
        }
    };
}

/// A position along the axis a line advances on.
///
/// Whether that is left-to-right, right-to-left, or top-to-bottom is the caller's
/// renderer's business (ADR-0004). There is no conversion to [`BlockOffset`] and no
/// arithmetic accepting one.
///
/// JLReq: §2.3.2
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct InlineOffset(i32);

/// A position along the axis lines stack on. JLReq: §2.3.2
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct BlockOffset(i32);

/// An extent along the inline axis: a line measure, an item's advance. JLReq: §3.8.1
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct InlineExtent(i32);

/// An extent along the block axis: how far ruby or warichu (割注) juts. JLReq: §4.5.1
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct BlockExtent(i32);

axis_scalar!(InlineOffset, "position along the inline axis");
axis_scalar!(BlockOffset, "position along the block axis");
axis_scalar!(InlineExtent, "extent along the inline axis");
axis_scalar!(BlockExtent, "extent along the block axis");

closed_arithmetic!(InlineOffset, "position along the inline axis");
closed_arithmetic!(BlockOffset, "position along the block axis");
closed_arithmetic!(InlineExtent, "extent along the inline axis");
closed_arithmetic!(BlockExtent, "extent along the block axis");

/// The two ends of the inline axis. Appendix B's "line head" is inline-start and its
/// "line end" is inline-end.
///
/// JLReq: §B.1
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineEdge {
    /// Where a line begins: Appendix B's "line head" (行頭). JLReq: §B.1
    Start,
    /// Where a line ends: Appendix B's "line end" (行末). JLReq: §B.1
    End,
}

/// The two sides of the block axis.
///
/// §3.3.4's ruby side is block-start in both directions — "right in vertical, above in
/// horizontal" is one side stated twice — and §4.5.1's first-line and last-line escape
/// rules are its exact dual. A correct implementation produces both of JLReq's sentences
/// from this one value; a conformance case requires it.
///
/// JLReq: §3.3.4, §4.5.1, §4.2.3
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The side ruby and emphasis dots sit on. JLReq: §3.3.4, §3.3.9
    BlockStart,
    /// The opposite side. JLReq: §4.5.1
    BlockEnd,
}

/// The direction a line advances and stacks.
///
/// Exactly three rules read this, each marked direction-conditional in the generated rule
/// inventory, and a gate asserts that the set of rules consulting it equals that set:
/// §3.1.3 (ideographic numerals with `、` and `・` are set solid in vertical writing),
/// §3.2.5 (tate-chu-yoko (縦中横) exists only in vertical writing), and §3.3.5
/// (katatsuki (肩付き) ruby alignment is forbidden in horizontal writing). Everything else JLReq
/// states twice is axis mapping and is expressed with [`Side`] and [`InlineEdge`].
///
/// JLReq: §2.3.1, §2.3.2, ADR-0011
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Horizontal writing (横組, yokogumi). JLReq: §2.3.2
    Horizontal,
    /// Vertical writing (縦組, tategumi). JLReq: §2.3.2
    Vertical,
}

#[cfg(test)]
mod tests {
    use super::{BlockExtent, BlockOffset, InlineEdge, InlineExtent, InlineOffset, Side};
    use crate::length::{LOWER, UPPER};

    #[test]
    fn an_extent_beyond_the_shared_bound_has_no_value() {
        assert!(
            InlineExtent::new(UPPER.saturating_add(1)).is_none(),
            "a measure beyond the bound is refused rather than silently wrapped"
        );
        assert!(
            BlockExtent::new(LOWER.saturating_sub(1)).is_none(),
            "the bound is symmetric, because a jut is signed"
        );
    }

    #[test]
    fn an_extent_at_the_shared_bound_has_a_value() {
        assert_eq!(
            InlineOffset::new(UPPER).map(InlineOffset::units),
            Some(UPPER),
            "the bound itself is valid; it is the first value past it that is refused"
        );
    }

    #[test]
    fn the_neutral_value_of_each_axis_type_is_zero_units() {
        assert_eq!(InlineOffset::ZERO.units(), 0, "a line starts at no offset");
        assert_eq!(BlockOffset::ZERO.units(), 0, "so does a stack of lines");
        assert_eq!(InlineExtent::ZERO.units(), 0, "solid setting adds nothing");
        assert_eq!(BlockExtent::ZERO.units(), 0, "and juts nowhere");
    }

    #[test]
    fn the_ends_of_an_axis_are_two_distinct_values() {
        assert_ne!(
            InlineEdge::Start,
            InlineEdge::End,
            "Appendix B's line head and line end are the two ends of one axis"
        );
        assert_ne!(
            Side::BlockStart,
            Side::BlockEnd,
            "§3.3.4's ruby side and §4.5.1's opposite side are the two ends of the other"
        );
    }
}
