// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The plain-old-data shapes the six generated matrices are written in, and the one
//! `const` helper (`em`) that turns a captured amount into an [`Em`] inside a `static`
//! initializer.
//!
//! These types hold no specification knowledge of their own. They exist so the six
//! generated modules under `src/generated/` can each emit one `static` array of literals —
//! the shape `docs/design/generation.md` requires of every generated table, because
//! `clippy::too_many_lines` has a threshold of one hundred and a `const fn` computing a
//! matrix would blow past it on the first table.
//!
//! A row's axis position is a plain `u8`: `0` is the sentinel for the line head (on
//! [`RawSpacingCell::before`] and [`RawReductionCell::before`]) or the line end (on
//! [`RawSpacingCell::after`] and [`RawReductionCell::after`]), and `1..=30` is a class
//! ordinal ([`jlreq_class::Class::number`]). Table 2 and Table 6 carry no line-edge axis at
//! all (`docs/design/generation.md`), so their rows never use the sentinel; nothing here
//! forbids it structurally, because the generator that emits the rows is what the
//! `at-most-one-space-per-referent`-style invariants police, not this module.

use jlreq_spec::RuleId;
use jlreq_unit::Em;

/// The sentinel `before`/`after` value naming the line head or the line end, rather than
/// one of the thirty classes.
pub(crate) const LINE_EDGE: u8 = 0;

/// One term of a Table 1 cell: one neighbor's contribution to the space (ADR-0014).
///
/// JLReq: §B.1
#[derive(Debug, Clone, Copy)]
pub(crate) struct RawTerm {
    /// `false` for `be` (the preceding character's em), `true` for `af` (the trailing
    /// character's em).
    pub(crate) trailing: bool,
    /// The amount, as a fraction of the referent's em.
    pub(crate) amount: Em,
}

/// Appendix B's two structurally different ruby-overhang permissions (ADR-0014).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RawHang {
    /// The cell grants none.
    None,
    /// `hang`: ruby may extend over the space, up to the amount of the term it qualifies.
    OverSpace,
    /// `ruby hang`: the cell is solid and ruby may extend over the character itself.
    OverCharacter,
}

/// One Table 1 cell: at most two [`RawTerm`]s (ADR-0014), a ruby-overhang permission, and
/// whether the adjacency itself is prohibited.
///
/// JLReq: §B.1
#[derive(Debug, Clone, Copy)]
pub(crate) struct RawSpacingCell {
    /// The row: a class ordinal, or [`LINE_EDGE`] for the line head.
    pub(crate) before: u8,
    /// The column: a class ordinal, or [`LINE_EDGE`] for the line end.
    pub(crate) after: u8,
    /// `×`: this adjacency may not occur at all.
    pub(crate) prohibited: bool,
    /// The ruby-overhang permission.
    pub(crate) hang: RawHang,
    /// The rule that states this cell: the qualifying §B.2 note when the token names one,
    /// `SPACING_BETWEEN_CHARACTERS` (§B) otherwise.
    pub(crate) rule: RuleId,
    /// The terms, at most two, one per referent.
    pub(crate) terms: &'static [RawTerm],
}

/// One Table 2 cell: whether a line may end here, and at which of §C.3's four strictness
/// levels.
///
/// JLReq: §C.1
#[derive(Debug, Clone, Copy)]
pub(crate) struct RawBreakCell {
    /// The row: a class ordinal. Table 2 carries no line-edge axis.
    pub(crate) before: u8,
    /// The column: a class ordinal.
    pub(crate) after: u8,
    /// `×`: this adjacency may not occur at all, which is stronger than every level
    /// prohibiting the break — see `xtask`'s `prohibits_at`.
    pub(crate) prohibited: bool,
    /// Bit `N - 1` set means a break is prohibited at strictness level `N` (1..=4).
    pub(crate) levels: u8,
    /// The rule that states this cell: the qualifying §C.2 note, or
    /// `POSSIBILITIES_FOR_LINE_BREAKING_BETWEEN_CHARACTERS` (§C).
    pub(crate) rule: RuleId,
}

/// One cell of Tables 3, 4, 5 or 6: an amount that may move to a limit at a priority stage
/// (ADR-0014's [`Reduction`](crate::space::Reduction) and
/// [`Expansion`](crate::space::Expansion) share this shape because §D and §E publish it
/// identically; which ladder it belongs to is which generated module a `RawRangedCell`
/// came from, never a field of the cell).
///
/// JLReq: §D.1, §E.1
#[derive(Debug, Clone, Copy)]
pub(crate) struct RawRangedCell {
    /// The row: a class ordinal, or [`LINE_EDGE`] for the line head. Tables 3, 4 and 5
    /// carry it; Table 6 never does.
    pub(crate) before: u8,
    /// The column: a class ordinal, or [`LINE_EDGE`] for the line end. Tables 3, 4 and 5
    /// carry it; Table 6 never does.
    pub(crate) after: u8,
    /// The floor (Appendix D) or ceiling (Appendix E) this cell may move to, when it may
    /// move at all.
    ///
    /// This shape deliberately does *not* also carry the unadjusted amount or the `×`
    /// prohibition: both are Table 1's, §D.1 requires the two to agree, and
    /// `xtask attest`'s `unadjusted-amount-is-table1` invariant is what checks the
    /// transcription rather than the crate carrying two representations of one fact
    /// (ADR-0019).
    pub(crate) limit: Option<Em>,
    /// Written `=` in the legend rather than `-`: the amount or the limit, nothing between
    /// (§3.1.9). `false` for a continuously reducible or expandable cell.
    pub(crate) two_valued: bool,
    /// §3.8.4 step (d): expansion with no upper limit at all. Table 6 only.
    pub(crate) residual: bool,
    /// The priority stage this cell moves at, `0` when it does not move.
    pub(crate) stage: u8,
    /// The rule that states this cell: the qualifying §D.2/§E.2 note, or the table's own
    /// legend rule (`LEGEND_OF_TABLES_3_4_AND_5` for Tables 3 through 5,
    /// `OPPORTUNITIES_FOR_INTER_CHARACTER_SPACE_EXPANSION_DURING_LINE_ADJUSTMENT` for
    /// Table 6).
    pub(crate) rule: RuleId,
}

/// Turn a captured amount, in units of 1/720 em, into an [`Em`].
///
/// `xtask attest`'s `amounts-are-multiples-of-the-unit` invariant proves every amount the
/// committed capture holds is exact in this unit and within [`Em`]'s bound (ADR-0007), so
/// the `None` arm is unreachable for every unit count `xtask/src/spacing.rs` actually emits
/// — every captured amount is at most one em, `720`, thousands of units under the bound —
/// and is written out because the alternative is `unwrap` (see `Em::from_units`'s own
/// bound, and `jlreq_unit::length`'s `TWO`/`THREE` for the same pattern over a narrower
/// type).
pub(crate) const fn em(units: i32) -> Em {
    match Em::from_units(units) {
        Some(value) => value,
        None => Em::ZERO,
    }
}
