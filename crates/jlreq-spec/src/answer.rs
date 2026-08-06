// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Answers, and why they are what they are.
//!
//! One shape carries every layer's answers, and it carries the rules that produced them
//! rather than only the value. A conformance case can then assert that a half em came from
//! §B.2 note 5, so an implementation that gets the right number from the wrong sentence
//! fails — which matters because several answers are reachable by two rules with different
//! policy sensitivity (see `docs/adr/0013`).
//!
//! The chain of rules is bounded and allocation-free, so the crates that allocate nothing
//! can carry it.

use crate::rule::{RuleId, Standing};

/// How many rules one answer's provenance holds.
///
/// The specification bounds the chain at two steps — disambiguate, then at most one
/// reclassification — and the third slot is the headroom that lets [`Provenance::then`]
/// report a longer chain instead of truncating one.
const CHAIN: usize = 3;

/// A value together with why it is that value.
///
/// One shape for every layer. A conformance case can assert "this half em came from
/// §B.2 note 5", so an implementation that gets the right number from the wrong sentence
/// fails — which matters because several answers are reachable by two rules with
/// different policy sensitivity.
///
/// JLReq: n/a (ADR-0013)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Answer<T> {
    /// The answer itself.
    value: T,
    /// The rules that produced it.
    why: Provenance,
}

impl<T: Copy> Answer<T> {
    /// Build an answer.
    ///
    /// Public because `jlreq-class`, `jlreq-spacing`, `jlreq-line` and `jlreq-inline` all
    /// produce answers and none of them is this crate. A type four crates must build and
    /// cannot is an unconstructible input in the sense ADR-0012's gate rejects.
    ///
    /// JLReq: n/a (ADR-0013)
    #[must_use]
    pub const fn new(value: T, why: Provenance) -> Self {
        Self { value, why }
    }

    /// The answer.
    ///
    /// JLReq: n/a (ADR-0013)
    #[must_use]
    pub const fn value(self) -> T {
        self.value
    }

    /// Why it is that answer.
    ///
    /// JLReq: n/a (ADR-0013)
    #[must_use]
    pub const fn why(self) -> Provenance {
        self.why
    }
}

/// Why an answer is what it is. Fixed capacity, no allocation, so the no-alloc crates
/// can carry it.
///
/// The chain is bounded by the specification at two steps: disambiguate, then at most one
/// reclassification. A single slot would lose the first — `%` is chosen between cl-13 and
/// cl-27 by frame and *then* moved to cl-19 by §C.2's alternative.
///
/// JLReq: n/a (ADR-0013)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Provenance {
    /// The rules that produced the answer, in the order they applied.
    rules: [Option<RuleId>; CHAIN],
    /// The standing of the whole chain, which is its least specified step.
    standing: Standing,
}

impl Provenance {
    /// One rule, the ordinary case.
    ///
    /// JLReq: n/a (ADR-0013)
    #[must_use]
    pub const fn of(rule: RuleId, standing: Standing) -> Self {
        let mut rules = [None; CHAIN];
        rules[0] = Some(rule);
        Self { rules, standing }
    }

    /// A rule chained onto an existing provenance: the disambiguation, then at most one
    /// reclassification. `None` when the chain is already full, which the specification
    /// makes unreachable and which is therefore reported rather than truncated.
    ///
    /// The standing of the result is the weaker of the two, because a chain is only as
    /// specified as its least specified step: an answer that passed through a published
    /// reading of a silence is not the specification's own, however normative the step
    /// before it was.
    ///
    /// JLReq: n/a (ADR-0013)
    #[must_use]
    pub const fn then(self, rule: RuleId, standing: Standing) -> Option<Self> {
        let mut rules = self.rules;
        let mut index = 0;
        while index < rules.len() {
            if rules[index].is_none() {
                rules[index] = Some(rule);
                return Some(Self {
                    rules,
                    standing: self.standing.weakest(standing),
                });
            }
            index = index.saturating_add(1);
        }
        None
    }

    /// The rules that produced the answer, in the order they applied.
    ///
    /// JLReq: n/a (ADR-0013)
    pub fn rules(self) -> impl Iterator<Item = RuleId> {
        self.rules.into_iter().flatten()
    }

    /// What kind of claim the answer rests on.
    ///
    /// JLReq: n/a (ADR-0013)
    #[must_use]
    pub const fn standing(self) -> Standing {
        self.standing
    }

    /// Frozen projection (ADR-0012): whether the specification decides this.
    ///
    /// JLReq: n/a (ADR-0013)
    #[must_use]
    pub const fn is_specified(self) -> bool {
        self.standing.is_specified()
    }
}

#[cfg(test)]
mod tests {
    use super::{Answer, CHAIN, Provenance};
    use crate::rule::{RuleId, Standing};

    /// Three distinct rules. The inventory is generated and empty, so these stand for
    /// ordinals rather than for sections; nothing here reads the inventory.
    const FIRST: RuleId = RuleId(0);
    /// The second.
    const SECOND: RuleId = RuleId(1);
    /// The third.
    const THIRD: RuleId = RuleId(2);
    /// One more than the chain holds.
    const FOURTH: RuleId = RuleId(3);

    #[test]
    fn an_answer_carries_its_value_and_its_reason() {
        let answer = Answer::new(720u16, Provenance::of(FIRST, Standing::Normative));
        assert_eq!(answer.value(), 720);
        assert_eq!(answer.why().standing(), Standing::Normative);
        assert!(answer.why().is_specified());
    }

    #[test]
    fn one_rule_is_the_ordinary_provenance() {
        let why = Provenance::of(FIRST, Standing::Normative);
        assert!(why.rules().eq([FIRST]));
    }

    #[test]
    fn a_chain_keeps_the_order_the_rules_applied_in() {
        let why = Provenance::of(FIRST, Standing::Normative)
            .then(SECOND, Standing::Alternative)
            .expect("the second step fits");
        assert!(
            why.rules().eq([FIRST, SECOND]),
            "the disambiguation, then the reclassification"
        );
    }

    #[test]
    fn a_full_chain_is_reported_rather_than_truncated() {
        let full = Provenance::of(FIRST, Standing::Normative)
            .then(SECOND, Standing::Normative)
            .expect("the second step fits")
            .then(THIRD, Standing::Normative)
            .expect("the third step fits");
        assert_eq!(full.rules().count(), CHAIN);
        assert_eq!(
            full.then(FOURTH, Standing::Normative),
            None,
            "the specification bounds the chain; a longer one is reported, not silently cut"
        );
    }

    #[test]
    fn a_chain_is_only_as_specified_as_its_least_specified_step() {
        let mixed = Provenance::of(FIRST, Standing::Normative)
            .then(SECOND, Standing::Unstated)
            .expect("the second step fits");
        assert_eq!(mixed.standing(), Standing::Unstated);
        assert!(
            !mixed.is_specified(),
            "an answer that passed through a published reading is not the specification's"
        );

        let other_order = Provenance::of(FIRST, Standing::Unstated)
            .then(SECOND, Standing::Normative)
            .expect("the second step fits");
        assert_eq!(other_order.standing(), Standing::Unstated);
    }

    #[test]
    fn an_alternative_is_still_the_specifications_answer() {
        let chosen = Provenance::of(FIRST, Standing::Alternative)
            .then(SECOND, Standing::Normative)
            .expect("the second step fits");
        assert_eq!(chosen.standing(), Standing::Alternative);
        assert!(
            chosen.is_specified(),
            "JLReq states the permitted set and the policy picks within it"
        );
    }
}
