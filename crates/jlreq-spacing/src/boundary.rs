// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Everything the six matrices say about one adjacency, assembled into one answer.

use jlreq_spec::{Answer, RuleId};
use jlreq_unit::RubyOverhang;

use crate::space::{ConditionalSpace, Expansion};

/// Whether a line may end between two items.
///
/// A hard constraint: no `Ord`, no arithmetic, and no conversion to a number, so no
/// expression turns a prohibition into a cost (ADR-0010, `docs/api-frozen.toml`'s
/// `[[no_impl]]`).
///
/// Exhaustive (`docs/api-frozen.toml`'s `[[exempt]]`): §C.1's legend has one token for
/// permitted and one for prohibited.
///
/// JLReq: §C.1, §3.1.7, §3.1.8
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Breakable {
    /// A line may end here.
    Yes,
    /// A line may not end here.
    No {
        /// The rule that forbids it.
        rule: RuleId,
    },
}

/// Whether an adjacency may occur at all.
///
/// The tables' `×`. The English legend is vague; the Japanese is decisive and says the
/// placement is prohibited by 行頭禁則, 行末禁則, or another rule — so this is the kinsoku
/// prohibition restated at a line edge. It is policy-dependent, and it is an outcome the
/// composer must avoid, not an assertion that the caller's text is malformed.
///
/// Exhaustive (`docs/api-frozen.toml`'s `[[exempt]]`): the `×` of §B.1, §C.1, §D.1 and
/// §E.1's legends is one token.
///
/// JLReq: §B.1, §C.1, §D.1, §E.1 legends
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// This adjacency may occur.
    Permitted,
    /// This adjacency may not occur.
    Forbidden {
        /// The rule that forbids it.
        rule: RuleId,
    },
}

/// A same-run answer that is a procedure rather than a value.
///
/// §B.2 notes 9 through 11 say to set two adjacent characters of one complex "according to
/// the method explained in §3.7.1 / §3.3.5 / §3.3.6 / §3.3.7". The boundary names the
/// procedure and stops there; `jlreq-inline::place` runs it (a future milestone). The
/// variant exists so the table states what the specification states instead of inventing a
/// number.
///
/// JLReq: §B.2#9, §B.2#10, §B.2#11, §3.7.1
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Delegation {
    /// The rule that names the procedure.
    pub rule: RuleId,
}

/// Everything the six tables say about one adjacency.
///
/// JLReq: §B, §C, §D, §E
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Boundary {
    spaces: [Option<ConditionalSpace>; 2],
    expansion: Expansion,
    expansion_rule: Option<RuleId>,
    breakable: Answer<Breakable>,
    placement: Answer<Placement>,
    ruby_overhang: RubyOverhang,
    delegation: Option<Delegation>,
}

impl Boundary {
    /// Build one. Crate-visible: a `Boundary` is what `evaluate::boundary` assembles from
    /// the generated tables and the policy-conditional overrides, and no public
    /// constructor exists because a caller cannot state a fact the six matrices did not.
    pub(crate) const fn new(
        spaces: [Option<ConditionalSpace>; 2],
        expansion: Expansion,
        expansion_rule: Option<RuleId>,
        breakable: Answer<Breakable>,
        placement: Answer<Placement>,
        ruby_overhang: RubyOverhang,
        delegation: Option<Delegation>,
    ) -> Self {
        Self {
            spaces,
            expansion,
            expansion_rule,
            breakable,
            placement,
            ruby_overhang,
            delegation,
        }
    }

    /// The conditional spaces here, in order. At most two (ADR-0014), which `xtask attest`
    /// checks against the captured tables rather than trusting.
    ///
    /// JLReq: §B.1
    pub fn spaces(self) -> impl Iterator<Item = ConditionalSpace> {
        self.spaces.into_iter().flatten()
    }

    /// Whether a line may end here.
    ///
    /// JLReq: §C.1
    #[must_use]
    pub const fn breakable(self) -> Answer<Breakable> {
        self.breakable
    }

    /// Whether this adjacency may occur at all.
    ///
    /// JLReq: §B.1, §C.1, §D.1, §E.1
    #[must_use]
    pub const fn placement(self) -> Answer<Placement> {
        self.placement
    }

    /// How far ruby may extend here, before line adjustment caps it.
    ///
    /// JLReq: §B.1, §3.3.8
    #[must_use]
    pub const fn ruby_overhang(self) -> RubyOverhang {
        self.ruby_overhang
    }

    /// How far this boundary may be expanded during line adjustment, independent of
    /// whether either neighbor carries a conditional space to attach the fact to.
    ///
    /// Table 6 states this opportunity as one cell per class pair (§E.1's own single
    /// ceiling per coordinate), not one per referent the way Appendix B's amounts are — so
    /// unlike [`Boundary::spaces`], this is a boundary-level fact rather than a per-neighbor
    /// contribution. ADR-0021 amends ADR-0014 to say so explicitly, but the reasoning is
    /// ADR-0014's own: it is the identical move that ADR already made for
    /// [`Boundary::ruby_overhang`], whose own words apply here without change — "the second
    /// has no space to attach to, so the permission belongs to the boundary". A solid Table
    /// 1 cell (`spaces()` empty) can still answer a real [`Expansion`] here: cl-19 against
    /// cl-19 (kanji beside kanji), whose Table 1 cell is blank but whose Table 6 cell is
    /// `0-1/4 stage 3`, is the coordinate that makes the distinction observable at all.
    ///
    /// JLReq: §E, §E.1, §3.8.4
    #[must_use]
    pub const fn expansion(self) -> Expansion {
        self.expansion
    }

    /// Which rule states this coordinate's Table 6 answer — `Some` when a row of Table 6, or
    /// a note that governs one of its rows, decided [`Boundary::expansion`]'s value, `None`
    /// when Table 6 carries no row here at all.
    ///
    /// The `Option` is the fact this accessor exists to carry, not an incidental wrapper: it
    /// is what tells apart "Table 6 states no opportunity here, and names §E.2 note 8 while
    /// doing so" from "this coordinate has no row" — two situations [`Boundary::expansion`]
    /// alone answers identically, `Expansion::None`, because [`Expansion`] is a kind and not
    /// a record (ADR-0010) and carries no citation of its own. A `Some` here is consequently
    /// not a promise that the opportunity is real: at cl-24 against cl-13 (§E.2#8) the row
    /// exists, cites `E.2#8`, and states `limit: None` — the note's own denial, not its
    /// absence — so `expansion()` reads [`Expansion::None`] there while `expansion_rule()`
    /// still reads `Some(RuleId::E_2_NOTE_8)`.
    ///
    /// JLReq: §E, §E.1, §E.2, §3.8.4
    #[must_use]
    pub const fn expansion_rule(self) -> Option<RuleId> {
        self.expansion_rule
    }

    /// Where the same-run answer is a procedure rather than a value.
    ///
    /// JLReq: §B.2#9, §B.2#10, §B.2#11
    #[must_use]
    pub const fn delegation(self) -> Option<Delegation> {
        self.delegation
    }

    /// Frozen projection (ADR-0012): whether a line may end here, with every future
    /// [`Breakable`] variant that still refuses one keeping this `false`.
    ///
    /// JLReq: §C.1
    #[must_use]
    pub const fn is_breakable(self) -> bool {
        matches!(self.breakable.value(), Breakable::Yes)
    }

    /// Frozen projection (ADR-0012): whether this adjacency may occur at all.
    ///
    /// JLReq: §B.1, §C.1, §D.1, §E.1
    #[must_use]
    pub const fn is_permitted(self) -> bool {
        matches!(self.placement.value(), Placement::Permitted)
    }
}

#[cfg(test)]
mod tests {
    use jlreq_spec::{Provenance, RuleId, Standing};
    use jlreq_unit::{Em, RubyOverhang};

    use super::{Answer, Boundary, Breakable, Placement};
    use crate::space::{ConditionalSpace, Expansion, Reduction, Referent};

    fn rule() -> RuleId {
        RuleId::ALL[0]
    }

    #[test]
    fn a_boundary_reports_at_most_two_spaces_in_order() {
        let one = ConditionalSpace::new(Em::HALF, Referent::Preceding, Reduction::Rigid, rule());
        let two = ConditionalSpace::new(Em::QUARTER, Referent::Trailing, Reduction::Rigid, rule());
        let boundary = Boundary::new(
            [Some(one), Some(two)],
            Expansion::None,
            None,
            Answer::new(Breakable::Yes, Provenance::of(rule(), Standing::Normative)),
            Answer::new(
                Placement::Permitted,
                Provenance::of(rule(), Standing::Normative),
            ),
            RubyOverhang::None,
            None,
        );
        assert!(
            boundary.spaces().eq([one, two]),
            "no alloc: compared as iterators"
        );
    }

    #[test]
    fn the_frozen_projections_agree_with_the_open_answers() {
        let breakable = Boundary::new(
            [None, None],
            Expansion::None,
            None,
            Answer::new(Breakable::Yes, Provenance::of(rule(), Standing::Normative)),
            Answer::new(
                Placement::Permitted,
                Provenance::of(rule(), Standing::Normative),
            ),
            RubyOverhang::None,
            None,
        );
        assert!(breakable.is_breakable());
        assert!(breakable.is_permitted());

        let forbidden = Boundary::new(
            [None, None],
            Expansion::None,
            None,
            Answer::new(
                Breakable::No { rule: rule() },
                Provenance::of(rule(), Standing::Normative),
            ),
            Answer::new(
                Placement::Forbidden { rule: rule() },
                Provenance::of(rule(), Standing::Normative),
            ),
            RubyOverhang::None,
            None,
        );
        assert!(!forbidden.is_breakable());
        assert!(!forbidden.is_permitted());
    }

    #[test]
    fn a_boundary_can_answer_a_real_expansion_with_no_conditional_space_at_all() {
        // The plumbing `expansion()` exists to expose: a boundary whose `spaces()` is
        // empty (the shape a solid Table 1 cell answers) can still carry a real
        // `Expansion` — cl-19 against cl-19 is `evaluate.rs`'s own end-to-end test of
        // exactly this fact over real generated data; this is the constructor-level one.
        let ceiling = Expansion::Range {
            ceiling: Em::QUARTER,
            stage: crate::space::ExpansionStage::new(3),
        };
        let boundary = Boundary::new(
            [None, None],
            ceiling,
            Some(rule()),
            Answer::new(Breakable::Yes, Provenance::of(rule(), Standing::Normative)),
            Answer::new(
                Placement::Permitted,
                Provenance::of(rule(), Standing::Normative),
            ),
            RubyOverhang::None,
            None,
        );
        assert_eq!(boundary.spaces().count(), 0);
        assert_eq!(boundary.expansion(), ceiling);
        assert_eq!(
            boundary.expansion_rule(),
            Some(rule()),
            "the citation travels with the ceiling it states, even with no space to attach to"
        );
    }
}
