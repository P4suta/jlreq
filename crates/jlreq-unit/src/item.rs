// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The item vocabulary: what the caller measured, and the two index spaces over it.
//!
//! One item is one occurrence of one Appendix A key (see `docs/adr/0018`). The stream
//! built from items lives in `jlreq-class`, where the table that validates it is; this
//! module holds the vocabulary and makes no claim about Appendix A.

use crate::axis::InlineExtent;
use crate::length::ScaleId;

/// What the caller's supplied advance covers: the character frame (字幅).
///
/// One field carries two rules the specification states separately, because they are the
/// same distinction. Appendix A's Remarks column disambiguates 473 of its 1133 keys on
/// this axis (§3.2.4 puts full-width and fixed-space Western characters in cl-19, §3.2.6
/// puts proportional ones in cl-27 and half-width numerals in cl-24). And §3.1.2 states
/// that the advance of commas (cl-07), full stops (cl-06), opening brackets (cl-01),
/// closing brackets (cl-02) and middle dots (cl-05) is half-width, with Table 1's amount
/// being what "makes them appear as if they were intrinsically full-width".
///
/// So for those five classes the frame decides which way the conditional space runs. A
/// closing bracket declared [`Frame::HalfEm`] has the Table 1 amount *added*; the same
/// bracket declared [`Frame::FullEm`] — the advance a modern font reports — already
/// contains it, and it is *trimmed*. Both are correct and they are the same geometry
/// reached from opposite directions; a library that assumed one would overshoot by half
/// an em at the commonest adjacency in Japanese text.
///
/// The trim is not silent. Composition normalizes to the specification's geometry and
/// reports every unit it took from a supplied advance, with the rule that states it, in
/// `Line::trims`. The consequence is the property the pair of worked conformance cases
/// asserts: the same text on either frame produces identical placements, identical
/// trailing space, and an identical extent (ADR-0017).
///
/// The default is [`Frame::Unstated`], not a guess.
///
/// JLReq: §3.1.2, §3.2.4, §3.2.6, §A Remarks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Frame {
    /// Not stated. Multi-class keys resolve to their candidates rather than to a class.
    Unstated,
    /// The full ideographic em (全角, full-width).
    FullEm,
    /// Half an em (半角, half-width).
    HalfEm,
    /// A third of an em (三分角). JLReq: §A.25 U+002F
    ThirdEm,
    /// A quarter em (四分角). JLReq: §A.3 U+2010
    QuarterEm,
    /// A per-glyph advance (プロポーショナル). JLReq: §3.2.6
    Proportional,
}

/// The syntactic job the document gives this occurrence.
///
/// Needed by six code points and no others; leaving it unstated is safe everywhere else,
/// and where it is not, `jlreq::diagnose` names the item and the section.
///
/// JLReq: §3.1.3, §B.2#12, §C.2#11, §A.24
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Role {
    /// Not stated, which is the answer everywhere the role does not change a class or an
    /// amount.
    Unstated,
    /// `U+002E` or `U+30FB` separating a fraction. JLReq: §3.1.3, §A.24
    DecimalPoint,
    /// `U+002C` or `U+0020` grouping digits (位取り). JLReq: §A.24
    DigitGroupSeparator,
    /// A component of an SI unit symbol; both sides set solid. JLReq: §B.2#12, §3.9.2
    UnitSymbol,
    /// A Western character used as a quantity symbol. JLReq: §C.2#11, §E.2#10
    QuantitySymbol,
    /// A full stop or comma terminating a sentence. JLReq: §3.1.1
    SentenceTerminator,
    /// A dividing punctuation mark (cl-04) inside a sentence rather than ending one;
    /// §3.1.6's Note gives it either solid setting or a quarter em, a caller choice.
    /// JLReq: §3.1.6
    SentenceMedial,
}

/// A byte offset into one stream. Distinct from [`ItemIndex`] so the two index spaces
/// cannot be confused at a call site.
///
/// Deliberately *not* split per stream the way the ordinals are. A byte offset is only ever
/// dereferenced through the stream that owns the item carrying it, and the two places a
/// bare one appears in the surface — a break `Candidate` and `Line::bytes` — are
/// running text by definition, because annotation streams are not broken into lines
/// (ADR-0016).
///
/// JLReq: n/a (addressing)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub struct ByteOffset(u32);

impl ByteOffset {
    /// The offset `bytes` into the stream that owns the item carrying it.
    ///
    /// JLReq: n/a (addressing)
    #[must_use]
    pub const fn new(bytes: u32) -> Self {
        Self(bytes)
    }

    /// The offset, in bytes from the head of that stream.
    ///
    /// JLReq: n/a (addressing)
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// An ordinal into one running-text stream's items.
///
/// Annotation streams are indexed by `AnnotationIndex` instead, so a base range and an
/// annotation range cannot be swapped at a call site or inside a struct: the invariant is a
/// compile error rather than a comment (ADR-0016). Depth is exactly one — every construct
/// that owns a stream attaches to running text, and none sits inside another's — so two
/// ordinal types are enough and no stream identifier is threaded anywhere.
///
/// JLReq: n/a (addressing)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub struct ItemIndex(u32);

impl ItemIndex {
    /// The item at ordinal `index` of the running-text stream.
    ///
    /// JLReq: n/a (addressing)
    #[must_use]
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// The ordinal.
    ///
    /// JLReq: n/a (addressing)
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// One occurrence of one Appendix A key, as the caller already holds it.
///
/// `start` is a byte offset into the stream this item belongs to; the extent is implied by
/// the next item's `start`. Constructed from [`Item::new`] and configured by `with_*`,
/// never as a struct literal (ADR-0012).
///
/// **The granularity is exact and it is checked** (ADR-0018). One item is one Appendix A
/// key, so `classify` is total over items and an adjacency between two items is an
/// adjacency between two keys, which is what Appendices B through E are indexed by. Two
/// mismatches would otherwise be silent and both are refused by `Text::new`:
///
/// - A key split across two items. Appendix A keys twenty-five entries on an ordered pair,
///   and a shaper may emit either half as its own glyph. `<02E5, 02E9>` is a cl-27 falling
///   tone contour whose first code point is *also* listed alone, so splitting it yields two
///   plausible cl-27 answers; `<31F7, 309A>` is a cl-11 small kana whose second code point
///   is listed nowhere, so splitting it yields cl-11 followed by an unlisted reading. The
///   caller merges the glyphs into one item whose advance is their sum, which loses nothing:
///   no cell of any matrix is indexed inside a key.
/// - An item covering more than one key, **unless** it declares [`Frame::Proportional`] and
///   every key in it is listed in cl-27. That exception is the shaper's own output and
///   nothing else — §3.2.6 puts proportional Western characters in cl-27, Table 1 sets
///   cl-27 against cl-27 solid, and §C.2 note 12 requires a caller-supplied hyphen before a
///   Western word may be divided at all — so a Latin ligature contains no amount and no
///   break for the merge to have destroyed. §3.2.1's own example of Western text in
///   Japanese, the word `editor`, is six items or one, and both are well formed.
///
/// The three facts that always matter are constructor arguments, not builder steps. An
/// omitted advance is a zero-width character and a silently short line, and an omitted
/// size is the wrong size on every annotated line — the two loudest instances of the
/// failure this library exists to prevent, and neither has an "unstated" answer that
/// could be reported instead. The role is genuinely optional and does have one. The frame
/// has one for the cl-19 against cl-27 axis and does *not* have one for the five classes of
/// §3.1.2, where it names a geometry rather than a class, so `Text::new` requires it there.
///
/// JLReq: §A, §3.1.2, §3.2.6, §C.2#12, ADR-0002, ADR-0018
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Item {
    start: ByteOffset,
    advance: InlineExtent,
    scale: ScaleId,
    frame: Frame,
    role: Role,
}

impl Item {
    /// One occurrence, at `start`, `advance` wide, set at the size `scale` names.
    ///
    /// JLReq: §A, ADR-0002, ADR-0018
    #[must_use]
    pub const fn new(start: ByteOffset, advance: InlineExtent, scale: ScaleId) -> Self {
        Self {
            start,
            advance,
            scale,
            frame: Frame::Unstated,
            role: Role::Unstated,
        }
    }

    /// State what the supplied advance covers.
    ///
    /// JLReq: §3.1.2, §3.2.6
    #[must_use]
    pub const fn with_frame(mut self, frame: Frame) -> Self {
        self.frame = frame;
        self
    }

    /// State the syntactic job this occurrence does.
    ///
    /// JLReq: §3.1.3, §A.24
    #[must_use]
    pub const fn with_role(mut self, role: Role) -> Self {
        self.role = role;
        self
    }

    /// Where this occurrence begins, in bytes of its own stream.
    ///
    /// JLReq: n/a (addressing)
    #[must_use]
    pub const fn start(self) -> ByteOffset {
        self.start
    }

    /// How wide the caller measured it.
    ///
    /// JLReq: n/a (ADR-0002)
    #[must_use]
    pub const fn advance(self) -> InlineExtent {
        self.advance
    }

    /// What that advance covers. JLReq: §3.1.2, §3.2.6
    #[must_use]
    pub const fn frame(self) -> Frame {
        self.frame
    }

    /// The syntactic job it does. JLReq: §3.1.3
    #[must_use]
    pub const fn role(self) -> Role {
        self.role
    }

    /// Which declared size it is set at. JLReq: §B.1, §3.3.3
    #[must_use]
    pub const fn scale(self) -> ScaleId {
        self.scale
    }
}

#[cfg(test)]
mod tests {
    use super::{ByteOffset, Frame, Item, ItemIndex, Role};
    use crate::axis::InlineExtent;
    use crate::length::ScaleId;

    /// One ordinary full-width occurrence at the head of a stream.
    fn occurrence() -> Item {
        Item::new(
            ByteOffset::new(0),
            InlineExtent::new(1000).unwrap(),
            ScaleId::BASE,
        )
    }

    #[test]
    fn an_occurrence_states_nothing_it_was_not_told() {
        assert_eq!(
            (occurrence().frame(), occurrence().role()),
            (Frame::Unstated, Role::Unstated),
            "the default is unstated, not a guess"
        );
    }

    #[test]
    fn stating_the_frame_leaves_the_three_required_facts_alone() {
        let stated = occurrence().with_frame(Frame::HalfEm);
        assert_eq!(
            (stated.start(), stated.advance(), stated.scale()),
            (
                occurrence().start(),
                occurrence().advance(),
                occurrence().scale()
            ),
            "a builder step configures; it does not restate what the constructor took"
        );
    }

    #[test]
    fn stating_the_frame_and_the_role_states_both() {
        let stated = occurrence()
            .with_frame(Frame::Proportional)
            .with_role(Role::QuantitySymbol);
        assert_eq!(
            (stated.frame(), stated.role()),
            (Frame::Proportional, Role::QuantitySymbol),
            "the two builder steps are independent"
        );
    }

    #[test]
    fn the_advance_is_returned_as_the_caller_measured_it() {
        assert_eq!(
            occurrence().advance().units(),
            1000,
            "ADR-0002 makes the caller's advance authoritative and it is never reinterpreted"
        );
    }

    #[test]
    fn byte_offsets_order_by_their_value() {
        assert!(
            ByteOffset::new(0) < ByteOffset::new(3),
            "a byte offset is an ordinal into one stream and comparing two is meaningful"
        );
    }

    #[test]
    fn item_ordinals_order_by_their_value() {
        assert!(
            ItemIndex::new(0) < ItemIndex::new(1),
            "the ordinals of one stream are ordered, which a range over items needs"
        );
    }
}
