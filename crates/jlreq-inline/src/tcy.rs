// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Tate-chu-yoko (縦中横): the one construct whose *availability* depends on the writing
//! direction.
//!
//! §3.2.5 defines no horizontal tate-chu-yoko at all — unlike §3.3.5's katatsuki (肩付き)
//! ruby alignment, which is a recommendation and not a refusal (ADR-0011) — so
//! [`TateChuYoko::new`] is a hand-written item that names a [`Direction`] variant, and
//! `docs/direction-sites.toml` lists it for §3.2.5.
//!
//! This round implements exactly that availability fact and nothing past it. The rest of
//! the construct — the [`jlreq_unit::Segment`] with [`jlreq_unit::Interior::Opaque`] a real
//! tate-chu-yoko run lowers to (§3.2.5, ADR-0015), and a `Constructs::with_tate_chu_yoko`
//! wiring it into [`crate::lower`] — is a later round's slot, named rather than stubbed: an
//! accepted-and-ignored `with_tate_chu_yoko` would be worse than an absent one, so this
//! round's [`Constructs`](crate::Constructs) carries no such method, `lower` never sees a
//! [`TateChuYoko`], and [`crate::LowerError::NotAvailable`] is consequently unreachable
//! through it this round — a fact stated here rather than left for a reader to discover by
//! searching for a call site that does not exist.
//!
//! JLReq: §3.2.5, §A.30

use core::ops::Range;

use jlreq_spec::RuleId;
use jlreq_unit::{Direction, ItemIndex, ScaleId};

/// Tate-chu-yoko (縦中横): a short run set across a vertical line.
///
/// Vertical writing only — JLReq defines no horizontal counterpart, so this is the one
/// construct whose *availability* depends on the direction (§3.2.5, §A.30). Once formed it
/// composes through the cl-30 row and column of all six tables like any other class, so
/// nothing downstream branches; lowering that composition to a [`jlreq_unit::Segment`] is a
/// later round's work, not this one's.
///
/// JLReq: §3.2.5, §A.30
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct TateChuYoko {
    /// The first item of the run.
    first: ItemIndex,
    /// One past its last item.
    past: ItemIndex,
    /// Which declared size the run is set at.
    scale: ScaleId,
}

impl TateChuYoko {
    /// The span `items`, set at `scale`, in a document composed in `direction`.
    ///
    /// `Err(`[`NotAvailable`]`)` in horizontal writing. §3.2.5 states tate-chu-yoko for
    /// vertical writing and defines no horizontal counterpart, so there is no rule this
    /// could be checked *against* in that direction — the direction alone decides it, which
    /// is the availability fact ADR-0011 reads §3.2.5 as, distinct from §3.3.5's katatsuki,
    /// a recommendation `lower` honors and reports rather than a refusal at construction.
    ///
    /// JLReq: §3.2.5
    pub fn new(
        items: Range<ItemIndex>,
        scale: ScaleId,
        direction: Direction,
    ) -> Result<Self, NotAvailable> {
        if direction == Direction::Horizontal {
            return Err(NotAvailable {
                rule: RuleId::HANDLING_OF_TATE_CHU_YOKO_HORIZONTAL_IN_VERTICAL_SETTINGS,
                direction,
            });
        }
        Ok(Self {
            first: items.start,
            past: items.end,
            scale,
        })
    }

    /// The items the run spans.
    ///
    /// Held as its two ends rather than as a `Range`, because a `Range` is not `Copy` and
    /// this type is; the range is rebuilt here so callers still read one.
    ///
    /// JLReq: §3.2.5
    #[must_use]
    pub const fn items(self) -> Range<ItemIndex> {
        self.first..self.past
    }

    /// Which declared size the run is set at.
    ///
    /// JLReq: §3.2.5, §3.3.3
    #[must_use]
    pub const fn scale(self) -> ScaleId {
        self.scale
    }
}

/// A construct the specification does not define in this direction.
///
/// §3.2.5's tate-chu-yoko is the only one. §3.3.5's katatsuki ruby alignment "should not be
/// adopted" in horizontal writing, which is a recommendation about a construct that is
/// perfectly well defined there — a policy question [`crate::lower`] honors and reports
/// rather than this refusal (ADR-0011).
///
/// JLReq: §3.2.5
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct NotAvailable {
    /// The rule that defines no counterpart in this direction.
    pub rule: RuleId,
    /// The direction that made the construct unavailable.
    pub direction: Direction,
}

#[cfg(test)]
mod tests {
    use core::ops::Range;

    use jlreq_unit::{Direction, ItemIndex, ScaleId};

    use super::TateChuYoko;

    /// Both writing directions, for a test that needs to build one of each without naming a
    /// variant at its own call site. The allowlisted item for this file's own §3.2.5 site,
    /// matching the pattern `jlreq-spacing`'s `direction_of` and `jlreq-line`'s
    /// `feasible_over` already established in `docs/direction-sites.toml`.
    ///
    /// A tuple and not `[Direction; 2]`: this gate's own scanner clears its accumulated
    /// header at the first `;` before a function's opening brace, including the array
    /// length's, so an array-typed return here would attribute every occurrence in this
    /// function's own body to no enclosing item at all rather than to `directions` itself.
    fn directions() -> (Direction, Direction) {
        (Direction::Horizontal, Direction::Vertical)
    }

    /// A span three items wide.
    fn span() -> Range<ItemIndex> {
        ItemIndex::new(2)..ItemIndex::new(5)
    }

    #[test]
    fn tate_chu_yoko_is_refused_in_horizontal_writing() {
        let (horizontal, _vertical) = directions();
        let refused = TateChuYoko::new(span(), ScaleId::new(1), horizontal)
            .expect_err("§3.2.5 defines no horizontal tate-chu-yoko");
        assert_eq!(
            refused.direction, horizontal,
            "the refusal names the direction that made it unavailable"
        );
    }

    #[test]
    fn tate_chu_yoko_is_available_in_vertical_writing() {
        let (_horizontal, vertical) = directions();
        let run = TateChuYoko::new(span(), ScaleId::new(1), vertical)
            .expect("§3.2.5 defines tate-chu-yoko for vertical writing");
        assert_eq!(run.items(), span());
        assert_eq!(run.scale(), ScaleId::new(1));
    }
}
