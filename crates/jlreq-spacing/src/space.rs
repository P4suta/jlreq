// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The conditional space: the unit of Appendix B's data (ADR-0014), not the table cell.
//!
//! §B.2 note 3 makes the space between two middle dots "the sum of a quarter em of the
//! preceding middle dots and a quarter em of the trailing middle dots" — two quantities,
//! taken from two different characters' ems, in one printed cell. §D.2 note 3 then gives
//! those two quantities different reduction priorities in the same table. A type that held
//! one number per cell could not state either sentence; [`ConditionalSpace`] holds one
//! number per *referent*, and a boundary carries at most two of them (`xtask attest`'s
//! `at-most-one-space-per-referent` invariant is what proves the captured data never needs
//! a third).

use jlreq_spec::RuleId;
use jlreq_unit::{Carry, Em, InlineExtent, Size};

/// Which of the two adjacent characters' ems an amount is a fraction of.
///
/// Appendix B writes these `be` and `af`, and its legend explains why they must be
/// distinguished: a line composed with several character sizes at once has several
/// ideographic ems in it, so a bare fraction does not name a unique quantity. Every note
/// that assigns a conditional space assigns owner and referent together — "the conditional
/// half em space accompanying the preceding comma" — so this is one concept and one field.
///
/// Exhaustive (`docs/api-frozen.toml`'s `[[exempt]]`): §B.1's referent vocabulary is
/// exactly `be` and `af`, because a space between two characters has exactly two owners.
///
/// JLReq: §B.1
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Referent {
    /// `be`: the preceding character's em.
    Preceding,
    /// `af`: the trailing character's em.
    Trailing,
}

/// A priority stage in one of Appendix D's three reduction tables. Six steps (§3.8.3);
/// stage 1, the Western word space (cl-26), lies outside the tables — Appendix D covers
/// "the second and subsequent stages" — so a `ReductionStage` this crate produces is
/// always 2 through 6.
///
/// Distinct from [`ExpansionStage`] because the two ladders are two orderings of two
/// different things and §3.8.2 orders the ladders themselves absolutely: expansion is
/// reached only when nothing is left to reduce. One shared ordinal type would let "stage 2"
/// mean two things in one report and in one published case field (ADR-0014).
///
/// JLReq: §D.1, §3.8.3
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct ReductionStage(u8);

impl ReductionStage {
    /// Build one from the ordinal a captured cell states.
    pub(crate) const fn new(ordinal: u8) -> Self {
        Self(ordinal)
    }

    /// The stage number, as §3.8.3 and Appendix D print it.
    ///
    /// JLReq: §D.1, §3.8.3
    #[must_use]
    pub const fn ordinal(self) -> u8 {
        self.0
    }
}

/// A priority stage in Appendix E. Four steps (§3.8.4), the last unbounded.
///
/// JLReq: §E.1, §3.8.4
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct ExpansionStage(u8);

impl ExpansionStage {
    /// Build one from the ordinal a captured cell states.
    pub(crate) const fn new(ordinal: u8) -> Self {
        Self(ordinal)
    }

    /// The stage number, as §3.8.4 and Appendix E print it.
    ///
    /// JLReq: §E.1, §3.8.4
    #[must_use]
    pub const fn ordinal(self) -> u8 {
        self.0
    }
}

/// Whether, and how far, a conditional space may be reduced during line adjustment.
///
/// A kind rather than a range (ADR-0010): §3.1.9 says twice that at the line end "the
/// possibilities are only half em spacing or solid. Other spacing, such as quarter em
/// spacing should not be used", so [`Reduction::Discrete`] and [`Reduction::Range`] are two
/// different things a caller cannot confuse by rounding.
///
/// JLReq: §D.1, §3.1.9
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Reduction {
    /// A bare `1/2` or `1/4` in Appendix D: fixed, not reducible.
    Rigid,
    /// `1/2–0`, `1/2–1/4`, `1/4–1/8`: continuously reducible to a floor.
    Range {
        /// The lowest this space may be reduced to.
        floor: Em,
        /// The priority stage it reduces at.
        stage: ReductionStage,
    },
    /// `1/2=0`: the full amount or the floor, nothing between (§3.1.9).
    Discrete {
        /// The floor, the only value besides the full amount this may take.
        floor: Em,
        /// The priority stage it reduces at.
        stage: ReductionStage,
    },
}

/// Whether, and how far, a conditional space may be expanded during line adjustment.
///
/// JLReq: §E.1, §3.8.4
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Expansion {
    /// No expansion opportunity here.
    None,
    /// `1/4–1/2` and `1/4`: expandable to a ceiling at a stage.
    Range {
        /// The highest this space may be expanded to.
        ceiling: Em,
        /// The priority stage it expands at.
        stage: ExpansionStage,
    },
    /// §3.8.4 step (d): no upper limit, and §E's fourth step re-levels across every stage
    /// rather than filling its own. A kind, not a magnitude (ADR-0010).
    Residual,
}

/// One conditional space: one neighbor's contribution to the space at a boundary.
///
/// This and not the cell is the unit of spacing data (ADR-0014). §B.2 note 3 makes the
/// space between two middle dots "the sum of a quarter em of the preceding middle dots and
/// a quarter em of the trailing middle dots", and §D.2 note 3 then gives those two
/// components different reduction priorities in the same table. A cell holding one number
/// cannot state that.
///
/// JLReq: §B.1, §B.2#3, §B.2#5, §D.2#3
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ConditionalSpace {
    amount: Em,
    referent: Referent,
    reduction: Reduction,
    expansion: Expansion,
    rule: RuleId,
}

impl ConditionalSpace {
    /// Build one. Crate-visible: the evaluator assembles a `ConditionalSpace` from the
    /// generated tables and the notes that override them, and no public constructor exists
    /// because a caller cannot state a fact the six matrices did not.
    pub(crate) const fn new(
        amount: Em,
        referent: Referent,
        reduction: Reduction,
        expansion: Expansion,
        rule: RuleId,
    ) -> Self {
        Self {
            amount,
            referent,
            reduction,
            expansion,
            rule,
        }
    }

    /// The amount, as a fraction of the referent's em. Not confined to Table 1's tokens:
    /// §3.1.6 requires a full em after a sentence-final cl-04.
    ///
    /// JLReq: §B.1
    #[must_use]
    pub const fn amount(self) -> Em {
        self.amount
    }

    /// Whose em, and equivalently which character this space accompanies.
    ///
    /// JLReq: §B.1
    #[must_use]
    pub const fn referent(self) -> Referent {
        self.referent
    }

    /// How far this space may be reduced during line adjustment.
    ///
    /// JLReq: §D.1
    #[must_use]
    pub const fn reduction(self) -> Reduction {
        self.reduction
    }

    /// How far this space may be expanded during line adjustment.
    ///
    /// JLReq: §E.1
    #[must_use]
    pub const fn expansion(self) -> Expansion {
        self.expansion
    }

    /// The rule that states this space.
    ///
    /// JLReq: §B.1, §B.2
    #[must_use]
    pub const fn rule(self) -> RuleId {
        self.rule
    }

    /// Resolve to the caller's unit against the two neighbors' sizes.
    ///
    /// Selects the referent's [`Size`] and calls [`Em::resolve_inline`], which is the
    /// workspace's only bridge from a writing-system fraction to a caller-unit length —
    /// there is deliberately no second path that would round differently and make a case's
    /// boundary answer disagree with its placements.
    ///
    /// The remainder belongs to the referent's size and is never named here: the [`Size`]
    /// the referent selects carries the ordinal, and [`Carry`] is keyed by it, so the
    /// question "which size does this remainder belong to" has no wrong answer available
    /// (ADR-0007, ADR-0019).
    ///
    /// JLReq: §B.1
    #[must_use]
    pub fn resolve(self, before: Size, after: Size, carry: &mut Carry) -> InlineExtent {
        let size = match self.referent {
            Referent::Preceding => before,
            Referent::Trailing => after,
        };
        self.amount.resolve_inline(size, carry)
    }
}

#[cfg(test)]
mod tests {
    use jlreq_spec::RuleId;
    use jlreq_unit::{Carry, Em, Scale, ScaleId, Size};

    use super::{ConditionalSpace, Expansion, Reduction, ReductionStage, Referent};

    fn size(units: i32) -> Size {
        let em = jlreq_unit::Advance::new(units).expect("a positive advance");
        Size::new(ScaleId::BASE, Scale::square(em).expect("a positive scale"))
    }

    #[test]
    fn a_conditional_space_reports_its_own_fields() {
        let space = ConditionalSpace::new(
            Em::HALF,
            Referent::Preceding,
            Reduction::Range {
                floor: Em::ZERO,
                stage: ReductionStage::new(5),
            },
            Expansion::None,
            RuleId::ALL[0],
        );
        assert_eq!(space.amount(), Em::HALF);
        assert_eq!(space.referent(), Referent::Preceding);
        assert_eq!(
            space.reduction(),
            Reduction::Range {
                floor: Em::ZERO,
                stage: ReductionStage::new(5)
            }
        );
        assert_eq!(space.expansion(), Expansion::None);
    }

    #[test]
    fn resolve_selects_the_referents_size() {
        let mut carry = Carry::new();
        let preceding = ConditionalSpace::new(
            Em::HALF,
            Referent::Preceding,
            Reduction::Rigid,
            Expansion::None,
            RuleId::ALL[0],
        );
        let trailing = ConditionalSpace::new(
            Em::HALF,
            Referent::Trailing,
            Reduction::Rigid,
            Expansion::None,
            RuleId::ALL[0],
        );
        let before = size(1000);
        let after = size(500);
        assert_eq!(
            preceding.resolve(before, after, &mut carry).units(),
            500,
            "the preceding referent resolves against the before size's em"
        );
        assert_eq!(
            trailing.resolve(before, after, &mut carry).units(),
            250,
            "the trailing referent resolves against the after size's em"
        );
    }
}
