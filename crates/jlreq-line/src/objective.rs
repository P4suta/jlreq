// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The objective: how good one line is, and how two paragraphs compare.
//!
//! `Badness` and `Demerits` were reachable at M1, before whole-paragraph optimization
//! (`Search::Optimal`, [`crate::compose`]) existed: [`crate::Composition::demerits`] is
//! non-optional and every line [`crate::compose`] produces under `Search::FirstFit` already
//! carried one, a real one — [`crate::Ladder`] is filled (see its own `# Status`), so
//! `reduction_depth`, `expansion_depth`, `last_resort` and `hanging` all reported what the
//! ladder actually did rather than a value fixed at zero because nothing could yet move.
//! [`Preference::compare`] and the ranking it states were implemented here in full even
//! then, because §3.1.12's worked example already needs a comparison between a single
//! paragraph's own two candidate breaks (ADR-0010) — but comparing two *different*
//! paragraphs' demerits against one another, and searching a paragraph for the arrangement
//! that minimizes it, needed more than one *candidate arrangement* to search among:
//! `Search::FirstFit` commits to one candidate break per line before the ladder ever runs
//! (`crate::compose::Search::FirstFit`'s own doc), so it never built a second arrangement of
//! the same line to weigh this against. `Search::Optimal` is that job, real as of this
//! round: `crate::compose::run_dp` is the whole-paragraph search this module's own
//! `Preference::compare` and `Demerits::add_sat` license (that function's own doc states the
//! translation-invariance argument), and it is what makes every claim below about comparing
//! two arrangements — not merely two candidate breaks of one — a fact about running code
//! rather than a forward reference.
//!
//! JLReq: n/a (adjustment quality)

use core::cmp::Ordering;

use jlreq_spacing::{ExpansionStage, ReductionStage};
use jlreq_spec::{Policy, Question, RuleId};
use jlreq_unit::InlineExtent;

/// How badly stretched or squeezed one line is: TeX's quantity, in exact integers.
///
/// The optimizer's only tuning knob, and the only quantity of the objective a caller
/// constructs. It is bounded at [`Badness::WORST`], a value ordinary lines reach, so it
/// is a cap and not a sentinel — a line that *cannot* be set is [`crate::Fit::Infeasible`]
/// and never a large badness (ADR-0010).
///
/// JLReq: n/a (adjustment quality)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Badness(u32);

impl Badness {
    /// No stretch or shrink at all.
    ///
    /// JLReq: n/a (adjustment quality)
    pub const ZERO: Self = Self(0);

    /// `10_000`, TeX's cap.
    ///
    /// JLReq: n/a (adjustment quality)
    pub const WORST: Self = Self(10_000);

    /// `Badness` is the one quantity of the objective with a numeric constructor and a
    /// numeric accessor, and that is deliberate rather than an omission from
    /// `[[no_impl]]`. It is an input, it is bounded, its cap is a value ordinary lines
    /// reach, and no prohibition is ever expressed in it — what the denylist protects is
    /// the type that answers "may a line end here" (ADR-0010).
    ///
    /// JLReq: n/a (adjustment quality)
    #[must_use]
    pub const fn new(value: u32) -> Self {
        if value > Self::WORST.0 {
            Self::WORST
        } else {
            Self(value)
        }
    }

    /// The bare value.
    ///
    /// JLReq: n/a (adjustment quality)
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }

    /// `min(WORST, floor(100 × (residual/flex)^3))`.
    ///
    /// Exact and bit-identical on every target: the cube is taken in `u128`, which the
    /// shared bound on every length in the workspace (`2^30 - 1`) cannot overflow even
    /// cubed and multiplied by 100, so no shift schedule is needed to keep the
    /// intermediate value in range.
    ///
    /// The zero-flex case is defined rather than left to divide: a rigid line with no
    /// residual is [`Badness::ZERO`], and a rigid line with a residual is
    /// [`Badness::WORST`]. [`crate::Fit`] is never actually built anywhere in this crate
    /// (grep `Fit::` across `src/`: every hit is a doc comment) — `crate::compose`'s own
    /// `demerits_of` calls this function directly with `flex` fixed at
    /// [`InlineExtent::ZERO`] for a line the ladder could not place, so the second value is
    /// reached routinely, on every violating line either search composes, not avoided by a
    /// classifier this crate does not run. `Search::Optimal`'s own `tolerance` compares
    /// against exactly this value (its own doc states the two settings that reading makes
    /// reachable), which is what makes this fact load-bearing rather than incidental. The
    /// case is defined regardless of how often it is reached, so the function has no
    /// precondition and no division by zero — which under `clippy::arithmetic_side_effects`
    /// would be a build error the moment it was written. Only the *magnitude* of each
    /// quantity matters — a residual and a flex are both extents relative to the same solid
    /// setting, and the ratio between them is what the formula reads, not either one's sign.
    ///
    /// This is one of the handful of items in `docs/scalar-sites.toml`, because a ratio of
    /// two inline extents is a raw quantity and the axis types have no division
    /// (ADR-0011).
    ///
    /// Not `const`, unlike `docs/design/api-spine.md`'s own signature: the narrowing from
    /// the `u128` product back to `u32` needs either `u32::try_from` (`TryFrom` is not yet
    /// a stable `const` trait — `E0658`) or an `as` cast, and `clippy::cast_possible_truncation`
    /// (part of `clippy::pedantic`, run with `RUSTFLAGS="-D warnings"`) rejects the cast
    /// with no local escape, because `CONTRIBUTING.md` forbids `#[allow]`. `u32::try_from`
    /// is the workspace's own idiom for this narrowing elsewhere (see `compose.rs`), so it
    /// is used here too, and the one price is this function joining every other
    /// candidate-length arithmetic function in the workspace that already runs outside a
    /// `const` context.
    ///
    /// JLReq: n/a (adjustment quality)
    #[must_use]
    pub fn of(residual: InlineExtent, flex: InlineExtent) -> Self {
        let residual = u128::from(residual.units().unsigned_abs());
        let flex = u128::from(flex.units().unsigned_abs());
        if flex == 0 {
            return if residual == 0 {
                Self::ZERO
            } else {
                Self::WORST
            };
        }
        let Some(cube) = cubed(residual) else {
            return Self::WORST;
        };
        let Some(numerator) = cube.checked_mul(100) else {
            return Self::WORST;
        };
        let Some(denominator) = cubed(flex) else {
            return Self::WORST;
        };
        let Some(quotient) = numerator.checked_div(denominator) else {
            return Self::WORST;
        };
        u32::try_from(quotient).map_or(Self::WORST, Self::new)
    }
}

/// `value` cubed, or `None` past `u128`'s range (never reached for a length bounded at
/// `2^30 - 1`, and stated rather than assumed).
const fn cubed(value: u128) -> Option<u128> {
    match value.checked_mul(value) {
        Some(squared) => squared.checked_mul(value),
        None => None,
    }
}

/// How good a feasible line is.
///
/// Components add independently and saturating ([`Demerits::add_sat`]), and are compared
/// by [`Preference`]. There is no value meaning "impossible": infeasibility is
/// [`crate::Fit::Infeasible`], which carries evidence (ADR-0010).
///
/// Sealed (`#[non_exhaustive]`) like every public type this crate declares, which is what
/// makes the doc's own claim below literally true: a caller outside this crate cannot
/// write a struct literal naming its fields, only reach [`Demerits::ZERO`] and build up
/// from it through [`Demerits::add_sat`]. It has no *other* literal form and appears in no
/// input position but [`Preference::compare`]'s, which reads it rather than builds one.
///
/// Reduction and expansion depth are separate components because §3.8.2 orders the two
/// ladders absolutely — "only when there is no spacing that can be reduced is line
/// adjustment by inter-character spacing expansion applied" — and merging them would let
/// a little expansion outrank more reduction.
///
/// JLReq: §3.8.2, §3.8.3, §3.8.4, §3.5.4, §C.3 closing paragraph, `decision:widow-threshold`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Demerits {
    /// Widow adjustment (§3.5.4) and other structural penalties. Populated now:
    /// `crate::compose::demerits_of` — the one cost function both `Search::FirstFit` and
    /// `Search::Optimal` read (`crate::compose::compose_optimal`'s own doc) — sets this to
    /// `Paragraph::with_widow_threshold`'s own shortfall on exactly the paragraph's own
    /// last line, zero on every other line and zero on the last line too once the
    /// threshold is met or the caller never set one. `structural` ranks first in both of
    /// `docs/decisions/adjustment-preference.md`'s own orderings, which is what was
    /// reserved rather than merely declared before this component had a real value to
    /// carry — see that reading's own "Why" for why the ranking itself does not change
    /// now that this component is reachable rather than hypothetical. Rule §3.5.4 is
    /// `[[owned]]` at M3 in `docs/conformance-deferrals.toml` (ADR-0006's independently
    /// authored case phase, `crates/jlreq-conform/cases/3.5.4.json`), but this field itself
    /// is never the JLReq-shaped observable any case asserts: `Demerits` is this crate's own
    /// invention, and what a case actually checks is `ViolationKind::Widow`'s own address
    /// (`crate::compose::ViolationKind::Widow`'s own doc) — see
    /// `docs/decisions/widow-threshold.md`.
    pub structural: u32,
    /// One for a line that reached expansion's own unbounded fourth, re-leveling stage
    /// (`crate::ladder::expand`'s own doc), zero otherwise.
    pub last_resort: u32,
    /// The deepest ordinary expansion stage `crate::ladder::expand` engaged for this
    /// line (2 or 3), or zero if expansion never ran.
    pub expansion_depth: u32,
    /// The deepest reduction stage `crate::ladder::reduce` engaged for this line (2
    /// through 6), or zero if reduction never ran.
    pub reduction_depth: u32,
    /// Summed [`Badness`].
    pub badness: u32,
    /// One for a line whose own last item `crate::ladder::hang` let hang past the
    /// measure, zero otherwise.
    pub hanging: u32,
}

impl Demerits {
    /// No demerit at all: the neutral value, and the one way a caller outside this crate
    /// obtains a `Demerits` to build from with [`Demerits::add_sat`].
    ///
    /// JLReq: n/a (adjustment quality)
    pub const ZERO: Self = Self {
        structural: 0,
        last_resort: 0,
        expansion_depth: 0,
        reduction_depth: 0,
        badness: 0,
        hanging: 0,
    };

    /// Add two demerits component by component, saturating.
    ///
    /// JLReq: n/a (adjustment quality)
    #[must_use]
    pub const fn add_sat(self, rhs: Self) -> Self {
        Self {
            structural: self.structural.saturating_add(rhs.structural),
            last_resort: self.last_resort.saturating_add(rhs.last_resort),
            expansion_depth: self.expansion_depth.saturating_add(rhs.expansion_depth),
            reduction_depth: self.reduction_depth.saturating_add(rhs.reduction_depth),
            badness: self.badness.saturating_add(rhs.badness),
            hanging: self.hanging.saturating_add(rhs.hanging),
        }
    }
}

/// How two `Demerits` compare: a permutation of the six components, applied
/// lexicographically.
///
/// `Demerits` deliberately implements neither `PartialOrd` nor `Ord`, because a derived
/// order would advertise as the specification's a permutation the specification only
/// partly states.
///
/// **One relation is normative and every permutation holds it fixed.** §3.8.2: "Normally
/// line adjustment by inter-character spacing reduction is preferred. Only when there is
/// no spacing that can be reduced is line adjustment by inter-character spacing expansion
/// applied." §3.1.12's worked example applies exactly that to a choice between two breaks:
/// the opening bracket at the line end is ideally avoided by reclaiming a full em so the
/// next line's first character moves up (追い込み), and only because that reduction is
/// impossible is the bracket pushed down and the line expanded (追い出し). Ranking
/// `expansion_depth` before `reduction_depth` reproduces both sentences, so no choice of
/// `Question::ADJUSTMENT_PREFERENCE` reorders that pair.
///
/// **Where the other four sit is a silence.** §C.3's closing paragraph — "the very strict
/// rule is for the best appearance at the line head, while the strict rule is best to
/// avoid inter-character spacing adjustment" — is guidance on choosing a *level*, not a
/// rule for ranking two candidate paragraphs. Their placement is published in
/// `docs/decisions/adjustment-preference.md` with `Standing::Unstated`, and a conformance
/// case pins each of the two permutations:
///
/// - `least-adjustment`, the [`Policy::JLREQ`] value and the declaration order of
///   [`Demerits`]: `structural`, `last_resort`, `expansion_depth`, `reduction_depth`,
///   `badness`, `hanging`. It minimizes how deep into the ladders any line goes.
/// - `even-texture`: `structural`, `last_resort`, `badness`, `expansion_depth`,
///   `reduction_depth`, `hanging`. It minimizes how uneven the lines look, tolerating
///   deeper but more uniform adjustment.
///
/// JLReq: §3.8.2, §3.1.12, §C.3 (silence), `decision:adjustment-preference`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Preference {
    /// Which of the two published permutations is in force.
    order: Order,
}

/// The two permutations `docs/decisions/adjustment-preference.md` publishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Order {
    /// `structural, last_resort, expansion_depth, reduction_depth, badness, hanging`.
    LeastAdjustment,
    /// `structural, last_resort, badness, expansion_depth, reduction_depth, hanging`.
    EvenTexture,
}

impl Preference {
    /// The permutation `policy` selects.
    ///
    /// `Question::ADJUSTMENT_PREFERENCE` is `[[deferred]]` to no milestone at all in
    /// `docs/conformance-deferrals.toml` — `docs/decisions/adjustment-preference.md`'s two
    /// readings are read here by name, exactly as `RECLASSIFY` and `ITERATION_MARK_PERMITTED`
    /// are read by name in `jlreq_class::classify`, rather than by an ordinal a stage-1
    /// derivation could silently renumber.
    ///
    /// JLReq: §C.3 (silence), `decision:adjustment-preference`
    #[must_use]
    pub fn from_policy(policy: Policy) -> Self {
        let order = if policy.get(Question::ADJUSTMENT_PREFERENCE).name() == "even-texture" {
            Order::EvenTexture
        } else {
            Order::LeastAdjustment
        };
        Self { order }
    }

    /// Compare `a` against `b` under this permutation.
    ///
    /// JLReq: §3.8.2, §3.1.12, §C.3 (silence)
    #[must_use]
    pub fn compare(self, a: Demerits, b: Demerits) -> Ordering {
        for component in self.order.components() {
            let ordering = component(a).cmp(&component(b));
            if ordering != Ordering::Equal {
                return ordering;
            }
        }
        Ordering::Equal
    }
}

impl Order {
    /// This permutation's six components, in comparison order. §3.8.2's one fixed relation
    /// — `expansion_depth` ranked before `reduction_depth` — holds in both, because neither
    /// permutation reorders that pair.
    const fn components(self) -> [fn(Demerits) -> u32; 6] {
        match self {
            Self::LeastAdjustment => [
                |demerits: Demerits| demerits.structural,
                |demerits: Demerits| demerits.last_resort,
                |demerits: Demerits| demerits.expansion_depth,
                |demerits: Demerits| demerits.reduction_depth,
                |demerits: Demerits| demerits.badness,
                |demerits: Demerits| demerits.hanging,
            ],
            Self::EvenTexture => [
                |demerits: Demerits| demerits.structural,
                |demerits: Demerits| demerits.last_resort,
                |demerits: Demerits| demerits.badness,
                |demerits: Demerits| demerits.expansion_depth,
                |demerits: Demerits| demerits.reduction_depth,
                |demerits: Demerits| demerits.hanging,
            ],
        }
    }
}

/// How one line's fit against the measure resolved.
///
/// Neither variant carries a bare number standing for "impossible": a feasible line's
/// cost is [`Demerits`] and an infeasible one's evidence is stated, so a caller never has
/// to guess which large [`Badness`] means "did not fit" (ADR-0010).
///
/// JLReq: n/a (adjustment quality)
///
/// Not `Copy`: [`Fit::Feasible`] carries an [`crate::ladder::Adjustment`], whose per-site
/// amounts are a sequence sized by the line rather than a fixed shape.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Fit {
    /// The line fits, at this cost.
    Feasible {
        /// The cost of setting the line this way.
        demerits: Demerits,
        /// What was done to reach it.
        adjustment: crate::ladder::Adjustment,
    },
    /// The line does not fit at all. Carries why and by how much, which an infinity
    /// discards.
    Infeasible {
        /// How far short the best attempt fell.
        shortfall: InlineExtent,
        /// How far the ladder got before giving up.
        deepest: Deepest,
        /// The rule that blocked it, when one specific rule is the reason rather than an
        /// exhausted ladder.
        blocking: Option<RuleId>,
    },
}

impl Fit {
    /// Frozen projection (ADR-0012).
    ///
    /// `docs/design/api-spine.md`'s own preamble lists `Fit` among the by-value `Copy`
    /// types, but its own definition of [`Fit::Feasible`] carries a
    /// [`crate::ladder::Adjustment`], which owns three `Vec`s (`docs/design/api-spine.md`'s
    /// own "everything else is passed by reference" list names `Adjustment` there for
    /// exactly that reason) — a struct cannot be `Copy` while one of its fields is not, so
    /// the preamble's inclusion of `Fit` cannot be literally true of the type as designed.
    /// This takes `&self`, the by-reference convention the same document states for every
    /// non-`Copy` type, rather than `self`: taking `self` here would drop the `Adjustment`
    /// at the end of the call, which is not evaluable in a `const fn` (`E0493`) and would
    /// silently discard the caller's own value in a non-`const` one.
    ///
    /// JLReq: n/a (adjustment quality)
    #[must_use]
    pub const fn is_feasible(&self) -> bool {
        matches!(self, Self::Feasible { .. })
    }
}

/// How far the adjustment got before giving up, and on which ladder. §3.8.2 orders the
/// two absolutely, so "stage 3" without the ladder is two different facts.
///
/// JLReq: §3.8.2, §D, §E
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Deepest {
    /// The reduction ladder, at this stage.
    Reduction(ReductionStage),
    /// The expansion ladder, at this stage.
    Expansion(ExpansionStage),
}
