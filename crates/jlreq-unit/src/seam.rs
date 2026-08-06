// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The seam between the construct layer and the line layer.
//!
//! The types that cross from `jlreq-inline` to `jlreq-line` live here so that neither
//! crate names a type the other owns (see `docs/adr/0015`): the [`Segment`] and its
//! [`Interior`], the least [`Separation`] a construct forces, and the [`BlockDemand`].
//! Run identity crosses too and lives in [`crate::run`] beside the construct vocabulary it
//! is built from.
//!
//! The ruby overhang allowance travels the other way — `jlreq-line` resolves it after
//! adjustment, because §3.3.8 rule 3 caps it by the space that survives — and lives here
//! for the same reason.
//!
//! No type here carries the rule that produced it. Provenance has one carrier in this
//! workspace, `jlreq_spec::Answer<T>`, which is strictly richer — up to three rules and
//! the standing of the chain — and both crates at the seam already depend on `jlreq-spec`,
//! so `jlreq-inline` produces an `Answer<Segment<'_>>` and this crate keeps its promise to
//! depend on nothing (see `docs/adr/0019` and `docs/adr/0020`).

use core::ops::Range;

use crate::axis::{BlockExtent, InlineExtent};
use crate::item::ItemIndex;
use crate::length::{Em, ScaleId};

/// A span of items the line layer does not lay out as ordinary inline text.
///
/// One concept, four of JLReq's constructs, and the line layer meets none of their names.
/// A segment carries its own size, because three of the four are set smaller than the
/// text around them.
///
/// JLReq: §3.2.5, §3.4.2, §3.4.3, §3.7.2, §3.7.3
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Segment<'a> {
    /// The first item of the span.
    first: ItemIndex,
    /// One past its last item.
    past: ItemIndex,
    /// Which declared size the interior is set at.
    scale: ScaleId,
    /// How the interior relates to the line containing it.
    interior: Interior<'a>,
}

impl<'a> Segment<'a> {
    /// The span `items`, set at the size `scale` names, laid out as `interior` says.
    ///
    /// `jlreq-inline` builds these and `jlreq-line` reads them, so both a constructor and
    /// accessors are public: a seam type readable at one end and not writable at the
    /// other is a seam with nothing on the far end (ADR-0012).
    ///
    /// JLReq: §3.2.5, §3.4.2, §3.7.2, §3.7.3
    #[must_use]
    pub const fn new(items: Range<ItemIndex>, scale: ScaleId, interior: Interior<'a>) -> Self {
        Self {
            first: items.start,
            past: items.end,
            scale,
            interior,
        }
    }

    /// The items the segment spans.
    ///
    /// Held as its two ends rather than as a `Range`, because a `Range` is not `Copy` and
    /// this type is; the range is rebuilt here so callers still read one.
    ///
    /// JLReq: §3.2.5, §3.4.2, §3.7.2, §3.7.3
    #[must_use]
    pub const fn items(self) -> Range<ItemIndex> {
        self.first..self.past
    }

    /// Which declared size the interior is set at. Three of the four constructs are set
    /// smaller than the text around them. JLReq: §3.4.2, §3.2.5
    #[must_use]
    pub const fn scale(self) -> ScaleId {
        self.scale
    }

    /// How the interior relates to the line containing it. JLReq: §3.4.2, §3.7.2, §3.7.3
    #[must_use]
    pub const fn interior(self) -> Interior<'a> {
        self.interior
    }
}

/// The least inline space a construct forces at a base-text boundary.
///
/// §3.3.8 rule 1 forbids ruby from overhanging an adjacent ideographic character, so a
/// base character carrying more ruby than it is wide pushes its neighbors apart before
/// composition begins. That is natural advance rather than an adjustment opportunity, and
/// conflating the two composes every such line short — §3.3.1's note concludes that such a
/// line "needs some line adjustment processing" rather than that it offers some.
///
/// JLReq: §3.3.8, §3.3.1
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Separation {
    /// The boundary: the item the space follows.
    after: ItemIndex,
    /// How much space the construct forces there.
    least: InlineExtent,
}

impl Separation {
    /// At least `least` after the item `after`.
    ///
    /// JLReq: §3.3.8
    #[must_use]
    pub const fn new(after: ItemIndex, least: InlineExtent) -> Self {
        Self { after, least }
    }

    /// The item the forced space follows. JLReq: §3.3.8
    #[must_use]
    pub const fn after(self) -> ItemIndex {
        self.after
    }

    /// How much space is forced there. JLReq: §3.3.8, §3.3.1
    #[must_use]
    pub const fn least(self) -> InlineExtent {
        self.least
    }
}

/// How a segment's interior relates to the line containing it.
///
/// JLReq: §3.2.5, §3.4.2, §3.4.3, §3.7.2, §3.7.3
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Interior<'a> {
    /// Laid out on an axis this line does not own, and occupying `extent` along the
    /// inline axis. §3.2.5 sets tate-chu-yoko (縦中横) left to right and then centers
    /// "the whole string" on the vertical line, so the outer line sees one advance and
    /// a block-axis jut, not a nested writing mode. JLReq: §3.2.5
    Opaque {
        /// What the outer line gives up to it.
        extent: InlineExtent,
    },
    /// One sub-line whose inter-character spacing is adjusted so the span occupies
    /// exactly `extent`. §3.7.3's jidori (字取り), including its rule that spacing is
    /// not added where a break is prohibited, and its rule that a single character is
    /// set at the inline start of the block. JLReq: §3.7.3
    Filled {
        /// The length the specification states the span must occupy.
        extent: InlineExtent,
    },
    /// `parts` sub-lines as near equal in length as possible, none longer than an
    /// earlier one, split where the break rules permit. §3.4.2's warichu (割注).
    /// `straddle` is the only place it is ever [`Straddle::Permitted`], because §3.4.3
    /// is the only section that permits it. JLReq: §3.4.2, §3.4.3
    Balanced {
        /// How many sub-lines the interior is set as.
        parts: core::num::NonZeroU8,
        /// Whether it may continue onto the following line.
        straddle: Straddle,
    },
    /// Sub-lines split at exactly these positions, each starting at the segment's inline
    /// start, the segment as long as the longest of them. §3.7.2's furiwake (振分け),
    /// whose splits are declared by the document rather than searched for.
    /// JLReq: §3.7.2
    Declared(&'a [ItemIndex]),
}

/// Whether a segment may continue onto the following line.
///
/// §3.4.3 permits it for a warichu (割注) and the Japanese note records two-line straddling as
/// 頻出, frequent. §3.7.2 forbids it for furiwake in one sentence: "One furiwake block
/// should not be extended across multiple base text lines." §3.2.5 and §3.7.3 are within
/// one line by construction.
///
/// JLReq: §3.4.3, §3.7.2
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Straddle {
    /// The segment is set within one line. JLReq: §3.7.2, §3.2.5, §3.7.3
    Forbidden,
    /// The segment may continue onto the following line. JLReq: §3.4.3
    Permitted,
}

/// How far a run of items needs beyond the line on each side of the block axis.
///
/// §4.5.1: the kihon-hanmen (基本版面) line gap is *not* changed to accommodate ruby;
/// on the first or last line of the area the jutting part is placed outside it. Only the
/// page layer knows where that edge is, so the line layer reports this and the caller
/// decides.
///
/// JLReq: §4.5.1, §2.4.2, §2.5.1
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct BlockDemand {
    first: ItemIndex,
    past: ItemIndex,
    start: BlockExtent,
    end: BlockExtent,
}

impl BlockDemand {
    /// The demand `items` makes, `start` toward the ruby side and `end` away from it.
    ///
    /// `jlreq-inline` builds these and `Line::block_demand` returns them, so both a
    /// constructor and accessors are public: a seam type readable at one end and not
    /// writable at the other is a seam with nothing on the far end (ADR-0012).
    ///
    /// JLReq: §4.5.1
    #[must_use]
    pub const fn new(items: Range<ItemIndex>, start: BlockExtent, end: BlockExtent) -> Self {
        Self {
            first: items.start,
            past: items.end,
            start,
            end,
        }
    }

    /// The items making the demand.
    ///
    /// Held as its two ends rather than as a `Range`, because a `Range` is not `Copy`
    /// and this type is; the range is rebuilt here so callers still read one.
    ///
    /// JLReq: §4.5.1
    #[must_use]
    pub const fn items(self) -> Range<ItemIndex> {
        self.first..self.past
    }

    /// Toward the block-start side: the ruby side of §3.3.4 and §3.3.9.
    ///
    /// JLReq: §3.3.4, §3.3.9, §4.5.1
    #[must_use]
    pub const fn start(self) -> BlockExtent {
        self.start
    }

    /// Toward the block-end side. JLReq: §4.5.1
    #[must_use]
    pub const fn end(self) -> BlockExtent {
        self.end
    }
}

/// How far ruby may extend beyond its base at one boundary.
///
/// Appendix B's legend defines two structurally different permissions and this type keeps
/// them apart. `1/2 be hang` and its siblings permit extension *over the space* and say
/// ruby "shall not be extended over the other character", capped by whatever the space is
/// after line adjustment. `ruby hang` sits on a solid cell — there is no space — and
/// permits extension over the adjacent character itself.
///
/// Resolved by `jlreq-line` after adjustment and reported per boundary, because the cap
/// is not known until then (§3.3.8 rule 3).
///
/// JLReq: §B.1, §3.3.8
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RubyOverhang {
    /// No extension at all. JLReq: §3.3.8 rule 1
    None,
    /// Over the inter-character space only, up to `limit` in ruby ems, and never past
    /// the space. JLReq: §B.1 `1/2 be hang`, `1/4 af hang`
    OverSpace {
        /// The permission, in ems of the ruby's own size.
        limit: Em,
    },
    /// Over the adjacent character body, up to `limit` in ruby ems. JLReq: §B.1
    /// `ruby hang`, §B.2#7
    OverCharacter {
        /// The permission, in ems of the ruby's own size.
        limit: Em,
    },
}

#[cfg(test)]
mod tests {
    use core::num::NonZeroU8;

    use super::{BlockDemand, Interior, RubyOverhang, Segment, Separation, Straddle};
    use crate::axis::{BlockExtent, InlineExtent};
    use crate::item::ItemIndex;
    use crate::length::{Em, ScaleId};

    #[test]
    fn a_segment_reports_the_span_and_the_size_it_was_built_over() {
        let warichu = Segment::new(
            ItemIndex::new(4)..ItemIndex::new(9),
            ScaleId::new(1),
            Interior::Balanced {
                parts: NonZeroU8::new(2).unwrap(),
                straddle: Straddle::Permitted,
            },
        );
        assert_eq!(warichu.items(), ItemIndex::new(4)..ItemIndex::new(9));
        assert_eq!(
            warichu.scale(),
            ScaleId::new(1),
            "§3.4.2 sets a warichu (割注) smaller than the text around it, so a segment \
             carries its own size"
        );
        assert!(matches!(warichu.interior(), Interior::Balanced { .. }));
    }

    #[test]
    fn a_separation_reports_the_boundary_and_the_space_it_forces() {
        let forced = Separation::new(ItemIndex::new(2), InlineExtent::new(120).unwrap());
        assert_eq!(
            (forced.after(), forced.least().units()),
            (ItemIndex::new(2), 120),
            "§3.3.8 rule 1 pushes the neighbors apart before composition begins, which is \
             natural advance and not an adjustment opportunity"
        );
    }

    #[test]
    fn a_block_demand_reports_the_span_it_was_built_over() {
        let demand = BlockDemand::new(
            ItemIndex::new(2)..ItemIndex::new(5),
            BlockExtent::new(300).unwrap(),
            BlockExtent::ZERO,
        );
        assert_eq!(
            demand.items(),
            ItemIndex::new(2)..ItemIndex::new(5),
            "the range a caller reads back is the range the producer stated"
        );
    }

    #[test]
    fn a_block_demand_keeps_the_two_sides_apart() {
        let demand = BlockDemand::new(
            ItemIndex::new(0)..ItemIndex::new(1),
            BlockExtent::new(300).unwrap(),
            BlockExtent::new(120).unwrap(),
        );
        assert_eq!(
            (demand.start().units(), demand.end().units()),
            (300, 120),
            "ruby juts toward block-start and §4.5.1's escape is its dual, so the two \
             sides are never one number"
        );
    }

    #[test]
    fn the_two_overhang_permissions_are_different_values() {
        assert_ne!(
            RubyOverhang::OverSpace { limit: Em::HALF },
            RubyOverhang::OverCharacter { limit: Em::HALF },
            "Appendix B's `1/2 be hang` is capped by the surviving space and its \
             `ruby hang` sits on a solid cell; the same amount means different things"
        );
    }

    #[test]
    fn straddling_is_permitted_only_where_a_segment_says_so() {
        let warichu = Interior::Balanced {
            parts: NonZeroU8::new(2).unwrap(),
            straddle: Straddle::Permitted,
        };
        let tate_chu_yoko = Interior::Opaque {
            extent: InlineExtent::new(1000).unwrap(),
        };
        assert_ne!(
            warichu, tate_chu_yoko,
            "§3.4.3 permits a warichu to continue onto the following line and §3.2.5 \
             sets tate-chu-yoko within one line by construction"
        );
    }

    #[test]
    fn a_declared_interior_carries_the_positions_the_document_stated() {
        let splits = [ItemIndex::new(3)];
        let furiwake = Interior::Declared(splits.as_slice());
        assert_eq!(
            furiwake,
            Interior::Declared(splits.as_slice()),
            "§3.7.2's splits are declared by the document rather than searched for"
        );
    }
}
