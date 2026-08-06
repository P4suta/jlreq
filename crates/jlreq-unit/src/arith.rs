// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The inherent arithmetic, the inline cursor, and the distribution primitive.
//!
//! No `core::ops` trait is implemented for any type in this workspace. A bare `+` on a
//! length is therefore a compile error rather than a lint finding, no `#[allow]` is
//! written anywhere, and no shared configuration changes. This was measured: clippy's
//! `arithmetic-side-effects-allowed` resolves a crate-root-relative path *without* the
//! crate name, so one entry would silence the lint for every identically-named type in
//! every crate. It is not used. An `xtask ops` gate rejects any future `impl
//! core::ops::*` on these types.
//!
//! The six surfaces are generated from one macro rather than copied six times, because
//! six copies drift and the whole point of the separation is that they cannot. Each is
//! still closed over its own type: there is no cross-axis addition, no `From`, and no
//! shared trait (see `docs/adr/0011`).
//!
//! The macro is *defined* here and *expanded* in [`crate::axis`] and [`crate::length`],
//! where the types it serves are declared. Expanding it here would have opened the untyped
//! channel in a third module — the macro body writes `Self(units)` — and ADR 0011 gives
//! the channel exactly two homes. What is left in this module is the cursor and the
//! distribution, and their two crossings are the reviewed entries in
//! `docs/scalar-sites.toml`.

use core::iter::FusedIterator;

use crate::axis::{InlineExtent, InlineOffset};
use crate::length::{Advance, Carry, Em, LOWER, Size, UPPER, bounded};

/// Generate the closed arithmetic surface for one length type.
///
/// Every path is written `$crate::…` so the macro carries its own imports to whichever of
/// the two home modules expands it.
macro_rules! closed_arithmetic {
    ($type:ident, $what:literal) => {
        impl $type {
            #[doc = concat!("The sum of two ", $what, "s, saturating at the shared bound.")]
            ///
            /// Two valid values sum to less than `i32::MAX`, so the addition cannot wrap
            /// and the saturation can only ever report a breach of the bound rather than
            /// hide a machine wrap (ADR-0007).
            ///
            /// JLReq: n/a (arithmetic)
            #[must_use]
            pub const fn add_sat(self, rhs: Self) -> Self {
                Self($crate::length::bounded(self.0.saturating_add(rhs.0)))
            }

            #[doc = concat!("The difference of two ", $what, "s, saturating at the shared bound.")]
            ///
            /// JLReq: n/a (arithmetic)
            #[must_use]
            pub const fn sub_sat(self, rhs: Self) -> Self {
                Self($crate::length::bounded(self.0.saturating_sub(rhs.0)))
            }

            #[doc = concat!("The negation of a ", $what, ".")]
            ///
            /// Exact for every valid value, because the bound is symmetric: a reduction
            /// delta and hanging punctuation are naturally signed (ADR-0007).
            ///
            /// JLReq: n/a (arithmetic)
            #[must_use]
            pub const fn neg_sat(self) -> Self {
                Self($crate::length::bounded(self.0.saturating_neg()))
            }

            #[doc = concat!("The lesser of two ", $what, "s.")]
            ///
            /// An inherent method and not `Ord`: an ordering trait would let a length be
            /// compared with, sorted against, and clamped by a value of another axis
            /// through generic code (ADR-0011), and `docs/api-frozen.toml` names it under
            /// `[[no_impl]]`.
            ///
            /// JLReq: n/a (arithmetic)
            #[must_use]
            pub const fn min(self, rhs: Self) -> Self {
                if rhs.0 < self.0 { rhs } else { self }
            }

            #[doc = concat!("The greater of two ", $what, "s.")]
            ///
            /// JLReq: n/a (arithmetic)
            #[must_use]
            pub const fn max(self, rhs: Self) -> Self {
                if rhs.0 > self.0 { rhs } else { self }
            }

            #[doc = concat!("This ", $what, " brought inside `low` and `high`.")]
            ///
            /// `high` wins where the two bounds cross, which is the reading a reduction
            /// floor needs: §3.8.3 stops reducing at an eighth em even where the demand
            /// asks for less.
            ///
            /// JLReq: n/a (arithmetic)
            #[must_use]
            pub const fn clamp_to(self, low: Self, high: Self) -> Self {
                self.max(low).min(high)
            }

            #[doc = concat!("The sum of two ", $what, "s, or `None` past the bound.")]
            ///
            /// The reporting twin of [`Self::add_sat`], for the call sites that must say
            /// the measure did not fit rather than compose a page that quietly does not.
            ///
            /// JLReq: n/a (arithmetic)
            #[must_use]
            pub const fn add_checked(self, rhs: Self) -> Option<Self> {
                match self.0.checked_add(rhs.0) {
                    Some(sum) if sum <= $crate::length::UPPER && sum >= $crate::length::LOWER => {
                        Some(Self(sum))
                    },
                    _ => None,
                }
            }

            #[doc = concat!("The difference of two ", $what, "s, or `None` past the bound.")]
            ///
            /// JLReq: n/a (arithmetic)
            #[must_use]
            pub const fn sub_checked(self, rhs: Self) -> Option<Self> {
                match self.0.checked_sub(rhs.0) {
                    Some(difference)
                        if difference <= $crate::length::UPPER
                            && difference >= $crate::length::LOWER =>
                    {
                        Some(Self(difference))
                    },
                    _ => None,
                }
            }

            #[doc = concat!("This ", $what, " scaled by a ratio, exactly or not at all.")]
            ///
            /// `None` when the denominator does not divide the value and when the result
            /// would leave the bound. A proportion is never rounded here: where JLReq
            /// divides by a text-dependent count and no denominator can be exact, the
            /// answer is [`crate::distribute`] and a stated remainder rule, not a quiet
            /// floor.
            ///
            /// JLReq: §3.3.3, §3.4.2
            #[must_use]
            pub const fn scaled(self, ratio: $crate::length::Ratio) -> Option<Self> {
                match $crate::length::scale_exact(self.0, ratio) {
                    Some(units) => Some(Self(units)),
                    None => None,
                }
            }
        }
    };
}

pub(crate) use closed_arithmetic;

/// Accumulates position along the inline axis without rounding drift.
///
/// The only type in the workspace that adds a length to a length in a loop. It does *not*
/// own a [`Carry`]: composition needs both a running position and the extents it feeds to
/// [`distribute`], so a cursor with a private remainder would be a second carrier of the
/// rounding remainder for one em, and interleaving the two would lose a unit — which is
/// the defect ADR-0019 exists to remove, not one to reproduce inside the type that
/// claims to have removed it. One `Carry` is created per line and passed to every
/// resolution on it, cursor and bridge alike.
///
/// Bounded: once accumulation would exceed [`Advance::LIMIT`] the cursor records
/// saturation and [`InlineCursor::position`] answers `None`, so composition can report
/// the overflow with evidence rather than returning a wrong number.
///
/// JLReq: n/a (arithmetic, ADR-0019)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct InlineCursor {
    at: i32,
    saturated: bool,
}

impl InlineCursor {
    /// A cursor at the inline start of the line.
    ///
    /// JLReq: n/a (arithmetic)
    #[must_use]
    pub const fn new() -> Self {
        Self {
            at: 0,
            saturated: false,
        }
    }

    /// Move on by a measured extent.
    ///
    /// JLReq: n/a (arithmetic)
    #[must_use]
    pub const fn advance(mut self, by: InlineExtent) -> Self {
        let moved = self.at.saturating_add(by.raw());
        if moved > UPPER || moved < LOWER {
            self.saturated = true;
        }
        self.at = bounded(moved);
        self
    }

    /// Move on by a writing-system fraction of a given size, spending that em's carried
    /// remainder.
    ///
    /// The signature is [`Em::resolve_inline`]'s, deliberately: the cursor is one more
    /// caller of the one bridge, not a second one. Takes a [`Size`] and not a
    /// [`Scale`](crate::Scale) so that the slot of [`Carry`] it touches is named by the
    /// argument rather than chosen here.
    ///
    /// JLReq: §B.1, ADR-0007, ADR-0019
    #[must_use]
    pub const fn advance_em(mut self, by: Em, size: Size, carry: &mut Carry) -> Self {
        let resolved = by.inline_units(size, carry);
        let moved = self.at.saturating_add(resolved);
        if moved > UPPER || moved < LOWER {
            self.saturated = true;
        }
        self.at = bounded(moved);
        self
    }

    /// `None` once the accumulation has saturated.
    ///
    /// JLReq: n/a (arithmetic)
    #[must_use]
    pub const fn position(self) -> Option<InlineOffset> {
        if self.saturated {
            None
        } else {
            Some(InlineOffset::of(self.at))
        }
    }
}

impl Default for InlineCursor {
    fn default() -> Self {
        Self::new()
    }
}

/// Where the units that do not divide evenly go. JLReq states no rule; both readings are
/// permitted and both have conformance cases. Selected through `Policy`.
///
/// JLReq: n/a (`decision:remainder`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RemainderRule {
    /// Earliest sites in inline order.
    Leading,
    /// Latest sites in inline order.
    Trailing,
}

/// Split `total` across `weights` so the parts sum to `total` exactly.
///
/// Serves every rule whose divisor depends on the text, and which therefore no choice of
/// denominator can make exact: "reduced equally" (§3.8.3), "added equally" (§3.8.4), the
/// group-ruby ratio (§3.3.6), and the proportional jukugo (熟語) expansion (§F.3.4).
///
/// A weight is an [`Advance`] because §3.8.3 says spacing is reduced "in proportion to the
/// character size", and a character size is a length in the caller's own unit. Only the
/// ratios matter, so the unit cancels; the sum is accumulated in `i64`, which a line's
/// worth of [`Advance::LIMIT`]-bounded weights cannot overflow. A narrower weight type
/// would truncate silently for callers whose em is larger than the type, which is the one
/// failure mode a distribution primitive must not have.
///
/// The remainder is a typographic decision JLReq does not make. It is a `Question` in the
/// policy space, and it arrives here as an argument only because `jlreq-unit` does not
/// depend on `jlreq-spec`; one function over `Policy` derives it and every call site in
/// the workspace uses that one, so the policy is still the single carrier (ADR-0019).
///
/// Two degenerate inputs are answered rather than refused, because the signature has no
/// error channel and both have one reading each. A negative weight is no proportion at
/// all — §3.8.3's proportion is over character sizes, which are not negative — so it
/// weighs nothing, and the exactness claim survives unconditionally. Weights that are all
/// zero are all equal, so the split is equal, which is the same answer the proportional
/// case gives for equal weights. With **no** weights there is no site to place anything
/// at, so the iterator is empty and a non-zero total has nowhere to go; a caller holding
/// space and no site has a question this primitive cannot answer.
///
/// JLReq: §3.8.3, §3.8.4, §3.3.6, §F.3.4
#[must_use]
pub fn distribute(
    total: InlineExtent,
    weights: &[Advance],
    remainder: RemainderRule,
) -> Distribution<'_> {
    let whole = i64::from(total.raw());
    let sum = weights.iter().fold(0_i64, |running, weight| {
        running.saturating_add(i64::from(weight.get().max(0)))
    });
    let placed = weights.iter().fold(0_i64, |running, weight| {
        running.saturating_add(proportional_share(whole, *weight, sum))
    });
    let residue = whole.saturating_sub(placed);
    let sites = i64::try_from(weights.len()).unwrap_or(i64::MAX);
    let level = residue.checked_div(sites).unwrap_or(0);
    let odd = residue.checked_rem(sites).unwrap_or(0);

    Distribution {
        weights,
        next: 0,
        total: whole,
        sum,
        level,
        step: odd.signum(),
        extra: usize::try_from(odd.unsigned_abs()).unwrap_or(0),
        rule: remainder,
    }
}

/// One weight's share of the total, rounded toward zero.
///
/// `sum` is the total of the weights that carry any proportion at all. A zero sum leaves
/// every share zero and hands the whole total to the residue, which is what makes the
/// all-equal-weights reading fall out of the same expression rather than being a branch.
fn proportional_share(total: i64, weight: Advance, sum: i64) -> i64 {
    total
        .saturating_mul(i64::from(weight.get().max(0)))
        .checked_div(sum)
        .unwrap_or(0)
}

/// The iterator [`distribute`] returns.
///
/// Its items sum to the total exactly whenever there is at least one weight, which is the
/// qualification [`distribute`] states and the reason it states it: with no weight there
/// is no site, and the iterator is empty.
///
/// JLReq: §3.8.3, §3.8.4
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Distribution<'w> {
    weights: &'w [Advance],
    next: usize,
    total: i64,
    sum: i64,
    level: i64,
    step: i64,
    extra: usize,
    rule: RemainderRule,
}

impl Iterator for Distribution<'_> {
    type Item = InlineExtent;

    fn next(&mut self) -> Option<Self::Item> {
        let weight = *self.weights.get(self.next)?;
        let index = self.next;
        self.next = self.next.saturating_add(1);

        let carries_an_odd_unit = match self.rule {
            RemainderRule::Leading => index < self.extra,
            RemainderRule::Trailing => index >= self.weights.len().saturating_sub(self.extra),
        };
        let odd = if carries_an_odd_unit { self.step } else { 0 };
        let part = proportional_share(self.total, weight, self.sum)
            .saturating_add(self.level)
            .saturating_add(odd);

        // Every part carries the sign of the total and they sum to it, so each one is
        // inside the bound the total already satisfied and the fallback is unreachable.
        let narrowed =
            i32::try_from(part).unwrap_or(if part.is_negative() { LOWER } else { UPPER });
        Some(InlineExtent::of(narrowed))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let rest = self.weights.len().saturating_sub(self.next);
        (rest, Some(rest))
    }
}

impl ExactSizeIterator for Distribution<'_> {}

impl FusedIterator for Distribution<'_> {}

#[cfg(test)]
mod tests {
    use super::{Distribution, InlineCursor, RemainderRule, distribute};
    use crate::axis::{InlineExtent, InlineOffset};
    use crate::length::{Advance, Carry, Em, LOWER, Ratio, Scale, ScaleId, Size, UPPER};

    /// The parts of one distribution, as plain units, one per stated weight.
    fn parts<const SITES: usize>(
        total: i32,
        weights: [i32; SITES],
        rule: RemainderRule,
    ) -> [i32; SITES] {
        let weights: [Advance; SITES] =
            core::array::from_fn(|index| Advance::new(weights[index]).unwrap());
        let mut answer = [0; SITES];
        let split: Distribution<'_> =
            distribute(InlineExtent::new(total).unwrap(), weights.as_slice(), rule);
        for (slot, part) in answer.iter_mut().zip(split) {
            *slot = part.units();
        }
        answer
    }

    /// A square size at `em` caller units, declared first.
    fn base(em: i32) -> Size {
        Size::new(
            ScaleId::BASE,
            Scale::square(Advance::new(em).unwrap()).expect("a character size is positive"),
        )
    }

    #[test]
    fn addition_saturates_at_the_bound_rather_than_wrapping() {
        let limit = Em::from_units(UPPER).unwrap();
        assert_eq!(
            limit.add_sat(limit).units(),
            UPPER,
            "two valid values sum below i32::MAX, so saturation reports the breach of \
             the bound rather than hiding a machine wrap"
        );
    }

    #[test]
    fn checked_addition_reports_the_breach_instead_of_saturating() {
        let limit = Em::from_units(UPPER).unwrap();
        assert!(
            limit.add_checked(Em::from_units(1).unwrap()).is_none(),
            "the reporting twin refuses rather than clamps"
        );
    }

    #[test]
    fn checked_addition_inside_the_bound_answers_the_sum() {
        assert_eq!(
            Em::HALF.add_checked(Em::HALF).map(Em::units),
            Some(Em::FULL.units()),
            "two half ems are one em"
        );
    }

    #[test]
    fn subtraction_saturates_at_the_lower_bound() {
        let floor = Em::from_units(LOWER).unwrap();
        assert_eq!(
            floor.sub_sat(Em::FULL).units(),
            LOWER,
            "the bound is symmetric because both scalars are signed"
        );
    }

    #[test]
    fn negation_is_exact_for_every_valid_value() {
        let limit = Em::from_units(UPPER).unwrap();
        assert_eq!(
            limit.neg_sat().units(),
            LOWER,
            "a symmetric bound is what makes negation lose nothing"
        );
    }

    #[test]
    fn clamping_stops_a_reduction_at_its_floor() {
        let demand = Em::FULL.neg_sat();
        assert_eq!(
            demand.clamp_to(Em::EIGHTH.neg_sat(), Em::ZERO),
            Em::EIGHTH.neg_sat(),
            "§3.8.3 stops reducing at an eighth em however much the line asks for"
        );
    }

    #[test]
    fn the_lesser_and_greater_of_two_amounts_are_the_amounts_themselves() {
        assert_eq!(
            Em::QUARTER.min(Em::HALF),
            Em::QUARTER,
            "a quarter is less than a half"
        );
        assert_eq!(
            Em::QUARTER.max(Em::HALF),
            Em::HALF,
            "and a half is the greater"
        );
    }

    #[test]
    fn a_ratio_that_does_not_divide_the_value_is_refused_rather_than_rounded() {
        assert!(
            Advance::new(1000).unwrap().scaled(Ratio::THIRD).is_none(),
            "a third of 1000 caller units is not a whole number of them"
        );
    }

    #[test]
    fn a_ratio_that_divides_the_value_scales_it_exactly() {
        assert_eq!(
            Advance::new(1000)
                .unwrap()
                .scaled(Ratio::HALF)
                .map(Advance::get),
            Some(500),
            "§3.4.2's warichu (割注) is set at half the size of the surrounding text"
        );
    }

    #[test]
    fn a_distribution_sums_to_the_total_exactly() {
        let split = parts(100, [1, 1, 1], RemainderRule::Leading);
        assert_eq!(
            split.iter().copied().reduce(i32::saturating_add),
            Some(100),
            "no denominator makes a three-way split of 100 exact, so the parts carry the \
             remainder between them"
        );
    }

    #[test]
    fn a_distribution_places_the_odd_units_at_the_end_the_rule_names() {
        let leading = parts(100, [1, 1, 1], RemainderRule::Leading);
        let trailing = parts(100, [1, 1, 1], RemainderRule::Trailing);
        assert_eq!(
            (leading[0], leading[2], trailing[0], trailing[2]),
            (34, 33, 33, 34),
            "JLReq states no rule, so both readings are offered and they are opposites"
        );
    }

    #[test]
    fn a_distribution_over_equal_weights_is_an_equal_split() {
        let split = parts(90, [7, 7, 7], RemainderRule::Leading);
        assert_eq!(
            (split[0], split[1], split[2]),
            (30, 30, 30),
            "equal character sizes take equal shares whatever the sizes are"
        );
    }

    #[test]
    fn a_distribution_is_in_proportion_to_the_weights() {
        let split = parts(120, [1, 2, 3], RemainderRule::Leading);
        assert_eq!(
            (split[0], split[1], split[2]),
            (20, 40, 60),
            "§3.8.3 reduces spacing in proportion to the character size"
        );
    }

    #[test]
    fn a_distribution_over_weights_that_are_all_zero_is_an_equal_split() {
        let split = parts(10, [0, 0, 0, 0], RemainderRule::Leading);
        assert_eq!(
            (split[0], split[1], split[2], split[3]),
            (3, 3, 2, 2),
            "weights that are all zero are all equal, so the shares are equal and the \
             two odd units follow the remainder rule"
        );
    }

    #[test]
    fn a_negative_weight_carries_no_proportion() {
        let split = parts(100, [-5, 1, 1], RemainderRule::Trailing);
        assert_eq!(
            (split[0], split[1], split[2]),
            (0, 50, 50),
            "a character size is not negative, so a negative weight is no proportion and \
             the exactness claim survives it"
        );
    }

    #[test]
    fn a_distribution_of_a_reduction_sums_to_it_exactly() {
        let split = parts(-100, [1, 1, 1], RemainderRule::Leading);
        assert_eq!(
            split.iter().copied().reduce(i32::saturating_add),
            Some(-100),
            "a reduction is a negative total and is divided the same way an addition is"
        );
    }

    #[test]
    fn a_distribution_over_no_weights_has_no_parts() {
        let empty: [Advance; 0] = [];
        let split = distribute(
            InlineExtent::new(100).unwrap(),
            empty.as_slice(),
            RemainderRule::Leading,
        );
        assert_eq!(
            split.count(),
            0,
            "a caller holding space and no site to put it at gets no parts, not a guess"
        );
    }

    #[test]
    fn a_distribution_knows_how_many_parts_it_has_before_it_yields_them() {
        let weights = [Advance::ZERO; 3];
        let split = distribute(
            InlineExtent::ZERO,
            weights.as_slice(),
            RemainderRule::Leading,
        );
        assert_eq!(
            split.len(),
            3,
            "one part per site, which a caller placing them needs before the walk"
        );
    }

    #[test]
    fn a_cursor_accumulates_a_repeated_fraction_without_drift() {
        let size = base(1000);
        let mut carry = Carry::new();
        let cursor = InlineCursor::new()
            .advance_em(Em::THIRD, size, &mut carry)
            .advance_em(Em::THIRD, size, &mut carry)
            .advance_em(Em::THIRD, size, &mut carry);
        assert_eq!(
            cursor.position().map(InlineOffset::units),
            Some(1000),
            "three thirds of an em are one em, and the line's one carry makes that true"
        );
    }

    #[test]
    fn a_cursor_and_a_bridge_on_one_line_share_one_remainder() {
        // The test that separates one carrier from two. Composition needs a position and
        // an extent at the same size on the same line: with a remainder inside the cursor
        // and another beside it, four thirds of a 1000-unit em come to 1332 rather than
        // 1333, and neither number is wrong on its own (ADR-0019).
        let size = base(1000);
        let mut carry = Carry::new();
        let mut cursor = InlineCursor::new();
        cursor = cursor.advance_em(Em::THIRD, size, &mut carry);
        cursor = cursor.advance(Em::THIRD.resolve_inline(size, &mut carry));
        cursor = cursor.advance_em(Em::THIRD, size, &mut carry);
        cursor = cursor.advance(Em::THIRD.resolve_inline(size, &mut carry));
        assert_eq!(
            cursor.position().map(InlineOffset::units),
            Some(1333),
            "four thirds of a 1000-unit em are 1333 units however the line reached them"
        );
    }

    #[test]
    fn a_cursor_adds_measured_extents_as_they_are() {
        let cursor = InlineCursor::new()
            .advance(InlineExtent::new(300).unwrap())
            .advance(InlineExtent::new(450).unwrap());
        assert_eq!(
            cursor.position().map(InlineOffset::units),
            Some(750),
            "a measured advance is the caller's and is never reinterpreted"
        );
    }

    #[test]
    fn a_cursor_past_the_bound_reports_saturation_rather_than_a_wrong_number() {
        let far = InlineExtent::new(UPPER).unwrap();
        let cursor = InlineCursor::new().advance(far).advance(far);
        assert!(
            cursor.position().is_none(),
            "composition must be able to report the overflow with evidence"
        );
    }

    #[test]
    fn a_saturated_cursor_stays_saturated() {
        let far = InlineExtent::new(UPPER).unwrap();
        let cursor = InlineCursor::new()
            .advance(far)
            .advance(far)
            .advance(InlineExtent::new(LOWER).unwrap());
        assert!(
            cursor.position().is_none(),
            "a later subtraction does not make an earlier overflow untrue"
        );
    }
}
