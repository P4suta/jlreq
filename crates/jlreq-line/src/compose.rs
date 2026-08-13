// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Composition: [`Paragraph`], [`Search`], [`compose`], [`Composition`] and [`Line`].
//!
//! # Status
//!
//! The control flow is real: [`compose`] walks [`crate::Feasible`]'s breaks greedily,
//! taking the last one whose unadjusted line still fits the measure (§3.1.12 ⑤ read
//! greedily, per the frozen spine's own note on [`Search::FirstFit`]) or the first one at
//! all when even that overflows, and every line's [`Line::placements`], [`Line::extent`],
//! [`Line::trailing`] and [`Line::trims`] are computed for real from
//! [`jlreq_spacing::boundary`]'s conditional spaces, normalized per
//! `docs/adr/0017-normalized-line-geometry.md`. Adjustment is real too, now that
//! [`crate::Ladder`] is filled (see its own `# Status`): a line whose natural extent
//! overflows the measure is reduced ([`crate::ladder::reduce`]), then, if still overfull,
//! offered hanging punctuation ([`crate::ladder::hang`]); a line that remains short after
//! any reduction — including a line that was short from the start, and a line a
//! `Reduction::Discrete` flip overshot past the target — is expanded
//! ([`crate::ladder::expand`]), except a paragraph's own last line, which §3.8.1's Note
//! exempts from expansion though not from reduction. Only once the ladder is genuinely
//! drained and the line still does not fit is it reported through
//! [`ViolationKind::Overfull`] or [`ViolationKind::ExpansionExhausted`] rather than
//! silently accepted or silently improved. [`Line::adjustment`] and [`Line::hanging`]
//! report exactly what happened; [`Line::pull_up`] remains `None` for every line
//! [`Search::FirstFit`] composes — see its own doc for why that is a search-scope fact
//! and not a ladder one — and is now real under [`Search::Optimal`], which genuinely runs
//! the comparison [`Line::pull_up`] reports the outcome of (`compose_optimal`'s own doc).
//!
//! [`Paragraph::with_contribution`] does not exist: it would take a
//! `jlreq_inline::Contribution`, and the crate graph gives `jlreq-line` no edge to
//! `jlreq-inline` at all (see `src/lib.rs`'s own `# Status`). Every paragraph this
//! milestone composes is consequently plain text — `Runs::none()`, no segment, and
//! [`Line::parts`] is always empty. [`Composition::rewrites`] is always empty too: §B.2
//! note 14 (c)'s `々` replacement is a fact about the *composed* line head, which needs a
//! real composition to observe and is not evaluated at M1.

use alloc::vec;
use alloc::vec::Vec;
use core::cmp::Ordering;
use core::ops::Range;

use jlreq_class::Text;
use jlreq_spacing::{Adjacency, Boundary, Expansion, Referent, boundary};
use jlreq_spec::{Policy, RuleId};
use jlreq_unit::{
    BlockDemand, ByteOffset, Carry, Direction, Frame, InlineCursor, InlineExtent, InlineOffset,
    Item as UnitItem, ItemIndex, RubyOverhang, Runs, Size,
};

use crate::feasible::{Candidate, Feasible, FeasibleBreak};
use crate::ladder::{self, Adjustment, Ladder, Site};
use crate::objective::{Badness, Demerits, Preference};

/// What to compose, and against what.
///
/// JLReq: §3.5.1, §3.5.2, §3.5.4
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct Paragraph<'r> {
    text: Text<'r>,
    candidates: &'r [Candidate],
    measure: InlineExtent,
    direction: Direction,
    first_line_indent: InlineExtent,
    head_indent: InlineExtent,
    end_indent: InlineExtent,
    widow_threshold: u16,
}

impl<'r> Paragraph<'r> {
    /// Candidates are a required argument, not a builder step: ADR-0003 makes them an
    /// input, and omitting them would leave the library either breakless or inventing
    /// breaks. `measure` is a parameter name and not an item name, which is why the
    /// `[[forbidden]]` name guard does not fire on it (ADR-0012).
    ///
    /// JLReq: n/a (ADR-0003)
    #[must_use]
    pub const fn new(
        text: Text<'r>,
        candidates: &'r [Candidate],
        measure: InlineExtent,
        direction: Direction,
    ) -> Self {
        Self {
            text,
            candidates,
            measure,
            direction,
            first_line_indent: InlineExtent::ZERO,
            head_indent: InlineExtent::ZERO,
            end_indent: InlineExtent::ZERO,
            widow_threshold: 0,
        }
    }

    /// §3.5.1's paragraph line head indent: the first line's content starts this far in,
    /// and consequently has this much less measure to compose against than every other
    /// line.
    ///
    /// JLReq: §3.5.1
    #[must_use]
    pub const fn with_first_line_indent(mut self, amount: InlineExtent) -> Self {
        self.first_line_indent = amount;
        self
    }

    /// §3.5.2's line head and line end indents, which narrow every line's measure rather
    /// than only the first.
    ///
    /// JLReq: §3.5.2
    #[must_use]
    pub const fn with_indents(mut self, head: InlineExtent, end: InlineExtent) -> Self {
        self.head_indent = head;
        self.end_indent = end;
        self
    }

    /// §3.5.4's widow threshold: the number of characters, given by the caller, a
    /// paragraph's own last line should carry at minimum. `demerits_of` — the one cost
    /// function `compose_first_fit` and `evaluate_edge` both call, so `FirstFit` and
    /// `Optimal` are never scored by two different formulas (`compose_optimal`'s own doc,
    /// "this round's own C1") — reads this field on exactly the last line of any
    /// arrangement it costs, adding a shortfall-proportional term to
    /// [`Demerits::structural`] and, when the arrangement a search finally settles on still
    /// falls short, reporting a [`ViolationKind::Widow`] naming
    /// [`RuleId::WIDOW_ADJUSTMENT_OF_PARAGRAPHS`] in [`Composition::violations`].
    ///
    /// **The two searches read this fact differently, and that asymmetry is real rather
    /// than a defect to fix.** [`Search::Optimal`] genuinely *steers* toward satisfying it:
    /// `structural` ranks first in both of `docs/decisions/adjustment-preference.md`'s own
    /// orderings, so a paragraph with more than one feasible arrangement of its closing
    /// lines prefers the one whose last line meets this threshold over one that does not,
    /// weighed before any other component is even consulted. `Search::FirstFit` cannot do
    /// the same — it commits to one candidate break per line and never compares
    /// arrangements at all (`Search::FirstFit`'s own doc) — so setting this field never
    /// moves a break under `FirstFit`; it only makes the shortfall of whichever last line
    /// greedy composition already chose observable, through the identical violation.
    ///
    /// A threshold of `0`, [`Paragraph::new`]'s own default, is a no-op by construction and
    /// not by a special case checked anywhere: the shortfall `demerits_of` computes is
    /// `u32::from(threshold).saturating_sub(count)`, which is `0` for any `count` once
    /// `threshold` is `0`, so a caller who never calls this method composes exactly as it
    /// would have before this round existed.
    ///
    /// `docs/decisions/widow-threshold.md` publishes the four readings JLReq's own silence
    /// forces: what counts as "a character," whether a paragraph that occupies a single
    /// line can have a widow, the shape of the penalty, and what an unsatisfiable threshold
    /// means.
    ///
    /// JLReq: §3.5.4, `decision:widow-threshold`
    #[must_use]
    pub const fn with_widow_threshold(mut self, characters: u16) -> Self {
        self.widow_threshold = characters;
        self
    }

    /// The measure this paragraph's line composes against before any indent narrows it.
    const fn head_indent_for(self, first_line: bool) -> InlineExtent {
        if first_line {
            self.head_indent.add_sat(self.first_line_indent)
        } else {
            self.head_indent
        }
    }
}

/// How breaks are chosen.
///
/// There is no companion "what to compose to": §3.8.1's Note records that Japanese
/// composition has no concept corresponding to ragged right, so justification is not one
/// choice among several at the paragraph level.
///
/// **Both variants exist as of M3.** `docs/design/api-spine.md` froze this enum
/// `#[non_exhaustive]` at M1 specifically so that adding `Optimal { tolerance: Badness }`
/// later would be a minor release and not a breaking one (ADR-0012); an M1 adopter that
/// matches `FirstFit` was consequently never broken by this round's own addition.
/// [`crate::Ladder`] gives one line more than one possible [`crate::Adjustment`] (reduction,
/// hanging and expansion are all real — see its own `# Status`), and `Optimal` is what
/// *searches* that choice: it compares whole-paragraph *arrangements* of breaks against one
/// another's summed [`Demerits`] under [`Preference::compare`], over the same
/// [`crate::Feasible`] break set and the same `adjust_line` pipeline `FirstFit`'s own single
/// candidate per line already ran — declaring `Optimal` before that search existed would
/// have been exactly the kind of stub this project forbids: a value a caller could construct
/// and pass, silently answered by the greedy algorithm it does not name. That is no longer
/// true; see `Optimal`'s own doc for what the search actually does.
///
/// JLReq: n/a (ADR-0012)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Search {
    /// Take the last feasible break on each line that still fits the measure unadjusted,
    /// or the first feasible break at all when even that overflows.
    ///
    /// §3.1.12 ⑤'s two outcomes — taking the *last* feasible break is the pull-up
    /// (追い込み), taking an earlier one is the push-down (追い出し) — are a choice
    /// *between two candidate breaks*, compared by the reduction and expansion cost each
    /// would pay ([`Line::pull_up`]'s own doc). `FirstFit` never makes that comparison: it
    /// commits to one candidate (the geometry-based scan above) before the ladder ever
    /// runs, and drains reduction, hanging and expansion only to make *that* candidate
    /// fit. Comparing this candidate against a different one is exactly what
    /// `Search::Optimal` now adds, so [`Line::pull_up`] stays `None` for every line
    /// `FirstFit` composes — not because the ladder is unfilled (it is not, see
    /// `crate::Ladder`'s own `# Status`), but because nothing in this variant's own control
    /// flow ever runs the comparison [`Line::pull_up`] reports the outcome of.
    ///
    /// JLReq: §3.1.12, §3.8.2
    FirstFit,
    /// Minimize the paragraph's own summed [`Demerits`] under [`Preference::compare`], over
    /// every arrangement [`crate::Feasible`]'s break set and `tolerance` together admit —
    /// discarding, in the ordinary case, any line whose own [`Badness`] exceeds `tolerance`.
    ///
    /// **What "discarding" means given this milestone's own zero-flex reading.**
    /// `compose`'s own `demerits_of` — read by both search variants alike, so `FirstFit`'s
    /// existing answers are unaffected by this fact rather than exposed to it — computes a
    /// line's badness as [`Badness::of`] over a *rigid* residual (`flex` fixed at
    /// [`InlineExtent::ZERO`]): [`Badness::of`]'s own doc states the zero-flex case
    /// precisely, "a rigid line with no residual is `Badness::ZERO`, and a rigid line with a
    /// residual is `Badness::WORST`". A line this milestone's ladder can fit consequently
    /// always reports exactly [`Badness::ZERO`], and a line the fully-drained ladder still
    /// cannot fit always reports exactly [`Badness::WORST`] — nothing in between, because no
    /// stage of this milestone's ladder reports a graduated residual once it has genuinely
    /// given up. `tolerance` therefore has exactly two reachable settings today, not a
    /// spectrum: [`Badness::WORST`] is the neutral, most-permissive tolerance — it admits a
    /// ladder-exhausted line exactly as freely as `FirstFit` already does (`FirstFit` never
    /// discards its own chosen candidate for being infeasible; it reports the violation and
    /// composes on) — and every other value, [`Badness::ZERO`] included, admits only a line
    /// the ladder can actually fit. `Badness::ZERO` is consequently not a degenerate,
    /// over-strict setting under today's reading: it behaves identically to any tolerance
    /// short of [`Badness::WORST`], because a feasible line's own badness is always zero
    /// regardless of how little flex closed its gap.
    ///
    /// **Tolerance exhaustion.** A paragraph can still have no complete arrangement whose
    /// every line stays within a caller's own strict `tolerance` — most concretely, when
    /// `tolerance` is below [`Badness::WORST`] and some line of the paragraph is
    /// unavoidably ladder-exhausted regardless of where its neighbors break. ADR-0010
    /// forbids both a panic and a forbidden break here, and `compose` still must emit
    /// something (its own doc). `docs/decisions/tolerance-exhaustion.md` publishes the
    /// reading this crate takes: `compose` re-minimizes over the full, un-pruned edge set
    /// exactly once more, still choosing only among breaks [`crate::Feasible::compute`]
    /// itself permits, and the arrangement that search finds is reported the ordinary way,
    /// with every unfit line's own [`ViolationKind`] in [`Composition::violations`] — never
    /// silently, and never worse than what `FirstFit` alone would have produced (see
    /// `compose_optimal`'s own doc for why that second minimization is always reachable).
    ///
    /// JLReq: §3.8, §3.1.12, `decision:tolerance-exhaustion`
    Optimal {
        /// The worst per-line [`Badness`] a caller is willing to accept without this search
        /// looking further for something better — see this variant's own doc for the two
        /// settings this milestone's own zero-flex badness makes reachable, and
        /// [`Badness::WORST`] for the neutral one.
        tolerance: Badness,
    },
}

/// Compose. One entry point for greedy and optimal search, sharing one feasibility
/// computation and one candidate-validity check; each search then runs its own control flow
/// (`compose_first_fit`, `compose_optimal`) over the identical `runs`/`boundaries` the two
/// never disagree about building.
///
/// Returns `Err` only for input that is not well formed. A paragraph that cannot be
/// composed within the rules returns lines together with [`Composition::violations`],
/// because every real adopter must render something.
///
/// JLReq: §3.8, §C, §D, §E
pub fn compose(
    paragraph: Paragraph<'_>,
    policy: Policy,
    search: Search,
) -> Result<Composition, ComposeError> {
    let text = paragraph.text;
    let items = text.items();
    let item_count = u32::try_from(items.len()).unwrap_or(u32::MAX);

    for candidate in paragraph.candidates {
        let offset = candidate.at().get() as usize;
        if offset > text.as_str().len() {
            return Err(ComposeError::CandidateOutOfRange { at: candidate.at() });
        }
    }

    // `compose` composes plain text at this milestone: every paragraph it builds passes
    // `Runs::none()` unconditionally, so a caller-declared construct overlay is reachable
    // today only through the public `Feasible::compute`, called directly (`crate`'s own
    // `# Status`, "Wired, not slotted"). This is deferred rather than wired
    // opportunistically because `crates/jlreq-spacing/src/evaluate.rs`'s own
    // `delegation_of` switches on run identity too — `NonJukugoRuby` to §B.2#10,
    // `JukugoRuby` to §B.2#11 — and no case in this suite measures that spacing
    // delegation yet; a real overlay reaching `compose` would silently switch it on with
    // nothing behind it.
    let runs = Runs::none();
    let feasible = Feasible::compute(
        text,
        runs,
        paragraph.candidates,
        policy,
        paragraph.direction,
    );
    let boundaries = ordered_boundaries(feasible.breaks(), item_count);

    Ok(match search {
        Search::FirstFit => compose_first_fit(paragraph, runs, &boundaries, item_count, policy),
        Search::Optimal { tolerance } => {
            compose_optimal(paragraph, runs, &boundaries, item_count, policy, tolerance)
        },
    })
}

/// §3.1.12 ⑤ read greedily: the last feasible break on each line that still fits the measure
/// unadjusted, or the first feasible break at all when even that overflows — exactly
/// [`Search::FirstFit`]'s own doc, moved out of [`compose`] so the two searches are two
/// functions rather than one growing `match` arm apiece
/// (`clippy::too_many_lines`, the same reason `adjust_line` was already split out).
///
/// This was the M1 control flow, relocated and not otherwise rewritten, when
/// [`Search::Optimal`] was added: every statement was the body `compose` already ran,
/// unchanged, which is what kept every one of the 466 existing conformance cases and every
/// existing unit test on byte-identical answers (that round's own C6) — a shared per-line
/// evaluator serving both searches would have been the natural-looking refactor and is
/// exactly what this function declines to be: `compose_optimal` runs its own, separately
/// written pipeline, so a defect in one can never silently reach the other's
/// already-verified answers. This round's own addition is the one change since: §3.5.4's
/// widow check, pushed once here for the last line only and fed into the same
/// `demerits_of` call [`compose_optimal`]'s own reconstruction loop also feeds — one more
/// fact both pipelines read identically rather than a fork in either one.
///
/// JLReq: §3.1.12, §3.8, §3.5.4, `decision:widow-threshold`
fn compose_first_fit(
    paragraph: Paragraph<'_>,
    runs: Runs<'_>,
    boundaries: &[ItemIndex],
    item_count: u32,
    policy: Policy,
) -> Composition {
    let text = paragraph.text;
    let mut lines = Vec::new();
    let mut violations = Vec::new();
    let mut demerits = Demerits::ZERO;
    let mut start = ItemIndex::new(0);

    while start.get() < item_count || lines.is_empty() {
        let first_line = lines.is_empty();
        let target = paragraph.measure.sub_sat(paragraph.end_indent);
        let indent = paragraph.head_indent_for(first_line);

        let mut chosen: Option<ItemIndex> = None;
        let mut chosen_geometry: Option<Geometry> = None;
        for &candidate in boundaries.iter().filter(|at| at.get() > start.get()) {
            let geometry = geometry_of(
                text,
                runs,
                start..candidate,
                indent,
                paragraph.direction,
                policy,
                Edges::BOTH,
            );
            if fits(geometry.extent, target) {
                chosen = Some(candidate);
                chosen_geometry = Some(geometry);
            } else if chosen.is_none() {
                chosen = Some(candidate);
                chosen_geometry = Some(geometry);
                break;
            } else {
                break;
            }
        }

        let (Some(end), Some(geometry)) = (chosen, chosen_geometry) else {
            violations.push(Violation {
                line: u32::try_from(lines.len()).unwrap_or(u32::MAX),
                at: start,
                rule: RuleId::POSSIBILITIES_FOR_LINE_BREAKING_BETWEEN_CHARACTERS,
                kind: ViolationKind::NoFeasibleBreak,
            });
            break;
        };

        let line_number = u32::try_from(lines.len()).unwrap_or(u32::MAX);
        let is_last_line = end.get() >= item_count;

        let ladder = Ladder::of(geometry.sites.clone());
        let outcome = adjust_line(
            text,
            &ladder,
            geometry.extent,
            target,
            start..end,
            is_last_line,
            policy,
        );

        if let Some(kind) = outcome.kind {
            violations.push(Violation {
                line: line_number,
                at: end,
                rule: RuleId::POSSIBILITIES_FOR_LINE_BREAKING_BETWEEN_CHARACTERS,
                kind,
            });
        }

        if is_last_line {
            push_widow_violation(
                &mut violations,
                line_number,
                paragraph.widow_threshold,
                start,
                end,
            );
        }

        let adjusted = apply_adjustment(&geometry, &ladder, &outcome.deltas, start);
        let line_demerits = demerits_of(
            &outcome,
            target,
            adjusted.extent,
            start..end,
            is_last_line,
            paragraph.widow_threshold,
        );
        let adjustment = Adjustment::of(outcome.deltas, outcome.releveled);
        demerits = demerits.add_sat(line_demerits);

        lines.push(Line::from_geometry(
            start..end,
            byte_range(text, start, end),
            adjusted,
            line_demerits,
            is_last_line,
            adjustment,
            outcome.hanging,
        ));

        start = end;
    }

    Composition {
        lines,
        demerits,
        violations,
        rewrites: Vec::new(),
    }
}

/// Every fact one candidate line's own cost is computed against, threaded by value through
/// [`run_dp`]'s own many calls into [`evaluate_edge`] rather than recomputed or re-passed as
/// four more of `clippy::too_many_arguments`' own budget. `Paragraph` and `Runs` are both
/// `Copy` (their own preambles), so bundling them costs nothing this crate does not already
/// pay at every other call site that holds a `Paragraph` across a loop.
#[derive(Debug, Clone, Copy)]
struct SearchContext<'r> {
    paragraph: Paragraph<'r>,
    runs: Runs<'r>,
    item_count: u32,
    policy: Policy,
}

/// One candidate line's own full cost, computed once and shared by [`run_dp`]'s own search
/// and [`compose_optimal`]'s own reconstruction of the winning path — the identical
/// `geometry_of` → [`Ladder::of`] → `adjust_line` → [`apply_adjustment`] → `demerits_of`
/// pipeline [`compose_first_fit`] already runs for its own single chosen candidate per line,
/// run here for every candidate [`Search::Optimal`] considers. This is this round's own C1,
/// stated where a reader can check it against the code rather than only in prose: whichever
/// candidates the two searches compare, the cost compared for each is the one
/// [`Composition::demerits`] already reports for it, because both searches call the same
/// four functions in the same order over the same [`Geometry`].
struct EdgeCost {
    /// This line's own cost, exactly what `compose_first_fit`'s own `demerits_of` computes
    /// for its one chosen candidate.
    demerits: Demerits,
    /// The adjusted geometry [`Line::from_geometry`] needs.
    adjusted: Geometry,
    /// The full outcome of draining the ladder for this candidate, kept rather than reduced
    /// to `demerits` alone: [`Line::from_geometry`] needs `hanging`, [`Adjustment::of`]
    /// needs `deltas` and `releveled`, and `pull_up_of` needs `reduction_depth` — four
    /// different further facts off one shared computation, not four independent ones.
    outcome: LineAdjustment,
}

/// One candidate line's own cost and geometry, computed identically regardless of which
/// search asks. `first_line` and `is_last_line` — §3.5.1's own extra indent and §3.8.1's own
/// exemption from expansion — are read from `range`'s own two ends (`start.get() == 0`,
/// `end.get() >= context.item_count`) and never from anything about how the caller arrived
/// at `range`: this round's own C3, stated once here rather than left to a reader to notice
/// that [`run_dp`]'s own scan and [`compose_optimal`]'s own reconstruction loop agree on it
/// only by inspection. A range built from two boundaries that are genuinely a paragraph's own
/// first and last item answers both facts correctly however many other lines the winning path
/// happens to have either side of it, because neither fact is asked about the path at all.
/// `is_last_line` is now load-bearing for a third reason beside expansion's own exemption
/// and `Line::is_last`: it is what gates `demerits_of`'s own §3.5.4 term, so the identical
/// derivation this doc already argues for is what keeps the widow penalty on exactly one
/// edge of any complete path rather than on however many candidates happen to end where the
/// text does.
///
/// JLReq: §3.5.1, §3.8.1, §3.8.2, §3.8.3, §3.8.4, §3.5.4, `decision:widow-threshold`
fn evaluate_edge(context: SearchContext<'_>, range: Range<ItemIndex>) -> EdgeCost {
    let first_line = range.start.get() == 0;
    let is_last_line = range.end.get() >= context.item_count;
    let paragraph = context.paragraph;
    let target = paragraph.measure.sub_sat(paragraph.end_indent);
    let indent = paragraph.head_indent_for(first_line);

    let geometry = geometry_of(
        paragraph.text,
        context.runs,
        range.clone(),
        indent,
        paragraph.direction,
        context.policy,
        Edges::BOTH,
    );
    let ladder = Ladder::of(geometry.sites.clone());
    let outcome = adjust_line(
        paragraph.text,
        &ladder,
        geometry.extent,
        target,
        range.clone(),
        is_last_line,
        context.policy,
    );
    let adjusted = apply_adjustment(&geometry, &ladder, &outcome.deltas, range.start);
    let demerits = demerits_of(
        &outcome,
        target,
        adjusted.extent,
        range,
        is_last_line,
        paragraph.widow_threshold,
    );

    EdgeCost {
        demerits,
        adjusted,
        outcome,
    }
}

/// Shortest-path-by-topological-order over the shared boundary sequence `nodes`, minimizing
/// each arrangement's own summed [`Demerits`] under `preference` — the dynamic program
/// [`Search::Optimal`] runs, and the whole of what makes it more than `FirstFit` read twice.
///
/// **Why a DP is licensed here at all, not merely plausible (this round's own C2).**
/// [`Preference::compare`] orders [`Demerits`] lexicographically over a tuple of `u32`
/// components, and lexicographic order over such a tuple is translation-invariant under
/// componentwise addition: if `preference.compare(a, b)` is [`Ordering::Less`], then
/// `preference.compare(a.add_sat(c), b.add_sat(c))` is too, for any `c`, because two sums
/// that share a common addend compare exactly as the two summands it was added to do — the
/// shared addend cannot change which of the leading components differs first, or in which
/// direction. That is exactly the property "the best arrangement of a prefix does not depend
/// on what follows it" needs, which is what licenses building the paragraph's own best
/// arrangement out of the best arrangement of each of its own prefixes, one boundary at a
/// time, rather than an arrangement that only resembles that shape.
///
/// [`Demerits::add_sat`] is where the property stops holding exactly: once one component
/// saturates at `u32::MAX`, a strict inequality between two sums sharing that addend can
/// collapse to equality, and the comparison then falls through to the next component
/// instead of reflecting the true, unsaturated difference. The bound this needs is concrete,
/// not gestured at: every component but `badness` saturates far later than `badness` does —
/// `reduction_depth` is at most 6 per line, `expansion_depth` at most 3, `last_resort` and
/// `hanging` at most 1 each — so `badness`, capped at [`Badness::WORST`] (10,000) per line,
/// is the first component any real paragraph could ever push toward saturation, at
/// `u32::MAX / 10_000 ≈ 429_496` lines every one of which scores the worst badness this
/// milestone's own zero-flex reading allows (`Search::Optimal`'s own doc). No paragraph this
/// milestone can compose reaches that many lines, so the bound is theoretical, and it is
/// recorded here rather than left an undeclared assumption.
///
/// `structural` is not merely "later than `badness`" the way the four above are — it cannot
/// saturate at all, for any input a caller could ever construct. §3.5.4's own widow term
/// (`demerits_of`, `docs/decisions/widow-threshold.md`) is added only when `is_last_line`,
/// and exactly one edge of any complete path from a paragraph's own start to its own end has
/// `is_last_line` true (`evaluate_edge`'s own doc: read from the range's two ends, never
/// from the path). A single edge's own `structural` is bounded at `u16::MAX` = 65,535, the
/// widest `u32::from(threshold)` a `u16` threshold can ever name, so the sum over a complete
/// path is bounded at the identical 65,535 — nowhere near `u32::MAX`, and nowhere near the
/// point it could collide with `badness`'s own running total either. `badness` consequently
/// remains, unconditionally, the one component this bound has to name.
///
/// **The window this scans per `start` (this round's own C5).** For each node, `run_dp`
/// evaluates candidate ends in increasing boundary order and stops after the first one whose
/// own ladder-drained result is [`ViolationKind::Overfull`] — never on
/// [`ViolationKind::ExpansionExhausted`], which names a line still *short* even once every
/// expandable site is drained, exactly the state a *longer* candidate is the natural repair
/// for, so stopping there would silently drop the very candidate that could fix it. Every
/// item a longer candidate adds only ever contributes that item's own full, non-negative
/// advance to the line's natural extent, while the most that same item's own boundary could
/// still give back is a fraction of an em — Appendix D's own per-stage reduction ceilings,
/// never a whole item's advance — so once reduction and hanging have both been drained and
/// the line is still overfull, no longer candidate is expected to recover: extending only
/// ever adds more than it could plausibly reclaim. This is a stated, reasoned bound and not
/// a proof over pathological input: a longer candidate that happens to end on cl-06 or cl-07
/// could still become hang-rescuable in a way a shorter one was not, and this rule does not
/// chase that case — see `crate::ladder`'s own gap-enumeration block for this house's own
/// precedent of naming a bound's known exception rather than silently over-fitting to it.
///
/// **`tolerance` is a search-space filter here, not a demerit (ADR-0010).** `None` runs the
/// full, un-pruned search — every edge this scan evaluates is an admissible transition — and
/// `Some(limit)` additionally requires `edge.demerits.badness <= limit.get()` before an edge
/// may be used, which is exactly [`Search::Optimal`]'s own "discarding any line worse than
/// tolerance." Discarding an edge here never removes a break kinsoku itself permits: the
/// scanned candidates are `nodes`, built from [`Feasible::compute`]'s own break set start to
/// finish and never widened either way — only whether the DP may *use* an edge to reach a
/// later boundary is what `tolerance` decides.
///
/// Returns, per node index, the best reachable [`Demerits`] and the predecessor node index
/// that reaches it, or `None` for a node no admitted edge reaches at all.
///
/// JLReq: §3.8.2, §3.1.12, §3.5.4, ADR-0010, `decision:widow-threshold`
fn run_dp(
    context: SearchContext<'_>,
    nodes: &[ItemIndex],
    preference: Preference,
    tolerance: Option<Badness>,
) -> Vec<Option<(Demerits, usize)>> {
    let mut best: Vec<Option<(Demerits, usize)>> = vec![None; nodes.len()];
    best[0] = Some((Demerits::ZERO, 0));

    for start_index in 0..nodes.len().saturating_sub(1) {
        let Some((cost_so_far, _)) = best[start_index] else {
            continue;
        };
        let start = nodes[start_index];

        for end_index in (start_index.saturating_add(1))..nodes.len() {
            let end = nodes[end_index];
            let edge = evaluate_edge(context, start..end);

            let admissible = match tolerance {
                None => true,
                Some(limit) => edge.demerits.badness <= limit.get(),
            };
            if admissible {
                let total = cost_so_far.add_sat(edge.demerits);
                let better = match best[end_index] {
                    None => true,
                    Some((existing, _)) => preference.compare(total, existing) == Ordering::Less,
                };
                if better {
                    best[end_index] = Some((total, start_index));
                }
            }

            if matches!(edge.outcome.kind, Some(ViolationKind::Overfull(_))) {
                break;
            }
        }
    }

    best
}

/// Walk `best`'s own recorded predecessors from `last_index` back to the source (index
/// zero), returning the visited node indices in increasing-position order — `None` when
/// `best` does not mark `last_index` reachable, or if its own recorded predecessor chain
/// does not strictly decrease (which `run_dp` never records, since every edge it admits runs
/// from a lower node index to a higher one; checked here anyway rather than trusted, so a
/// broken invariant becomes an honest, reported gap instead of a loop with no exit).
fn reconstruct_path(best: &[Option<(Demerits, usize)>], last_index: usize) -> Option<Vec<usize>> {
    let mut path = vec![last_index];
    let mut current = last_index;
    while current != 0 {
        let entry = best.get(current)?;
        let (_, predecessor) = (*entry)?;
        if predecessor >= current {
            return None;
        }
        path.push(predecessor);
        current = predecessor;
    }
    path.reverse();
    Some(path)
}

/// The result [`compose_optimal`] reports for a paragraph with no boundary past its own
/// start — most concretely, an empty text, whose own `item_count` is zero, so there is no
/// candidate for the search to end a line on at all. The same shape
/// `compose_first_fit`'s own loop reports for the identical input (its `NoFeasibleBreak`
/// branch), named once here because it is not a search's own answer, only the absence of
/// anything for a search to answer — ADR-0010's own "composition never refuses to produce
/// lines" still holds: an empty `lines` together with the violation that explains it is a
/// result, not a refusal.
fn no_feasible_break() -> Composition {
    Composition {
        lines: Vec::new(),
        demerits: Demerits::ZERO,
        violations: vec![Violation {
            line: 0,
            at: ItemIndex::new(0),
            rule: RuleId::POSSIBILITIES_FOR_LINE_BREAKING_BETWEEN_CHARACTERS,
            kind: ViolationKind::NoFeasibleBreak,
        }],
        rewrites: Vec::new(),
    }
}

/// §3.1.12 ⑤ as [`Search::Optimal`] actually compared it for one line of the winning path.
///
/// `Some` exactly when two facts both hold: `end_index` is not the boundary immediately
/// after `start_index` in the shared `nodes` order, meaning a *shorter* candidate for this
/// same line's own start was also scanned and evaluated (`run_dp`'s own per-`start` window
/// always visits every closer boundary before a farther one, so if `end_index` was reached
/// at all, everything nearer to `start_index` was visited first) — and `reduction_depth > 0`,
/// meaning the longer choice this line actually took needed real reduction to fit, not a
/// natural fit the shorter alternative would have needed no repair to match either. Together
/// these are the "reduction-preferring comparison... applied to two candidate breaks that
/// both exist" ADR-0010 describes: a nearer, shorter stopping point was available, and the
/// search took the farther one anyway, reclaiming the gap by reduction — which is exactly
/// what "pull-up" (追い込み) names.
///
/// `amount` is this line's own realized reduction, read back off the [`Adjustment`] already
/// built for it (`Adjustment::reduced`'s own doc: "the sites that were reduced, and by how
/// much") rather than summed a second time from raw deltas. `pulls` is the shorter
/// alternative's own boundary itself — `nodes[end_index - 1]` — the item that would have
/// opened the following line under it, and sits on this line instead because the search
/// preferred the longer choice.
///
/// A line whose own chosen `end` *is* the nearest candidate after `start` never reaches
/// `Some`: there was no shorter break to compare it against, so nothing was pulled up, and
/// reporting one anyway would be inventing a comparison the search never ran — the discipline
/// [`Line::pull_up`]'s own doc states.
///
/// JLReq: §3.1.12
fn pull_up_of(
    adjustment: &Adjustment,
    reduction_depth: u8,
    nodes: &[ItemIndex],
    start_index: usize,
    end_index: usize,
) -> Option<PullUp> {
    if reduction_depth == 0 || end_index <= start_index.saturating_add(1) {
        return None;
    }
    let amount = adjustment
        .reduced()
        .iter()
        .copied()
        .fold(InlineExtent::ZERO, InlineExtent::add_sat);
    let pulls = *nodes.get(end_index.saturating_sub(1))?;
    Some(PullUp {
        amount,
        pulls,
        rule: RuleId::EXAMPLES_OF_LINE_ADJUSTMENT,
    })
}

/// [`Search::Optimal`]: minimize the paragraph's own summed [`Demerits`] under
/// [`Preference::from_policy`] over [`crate::Feasible`]'s break set and `tolerance` together,
/// via [`run_dp`], then rebuild the winning path's own [`Line`]s the same way
/// [`compose_first_fit`] builds its one chosen line per iteration.
///
/// **Reachability, and why `Optimal` never scores worse than `FirstFit` (this round's own
/// experiment, recorded here where the code that proves it lives).** `run_dp`'s own C5
/// window always evaluates and admits, in its `tolerance: None` form, the exact candidate
/// `compose_first_fit`'s own loop would choose at every step: that loop only ever picks the
/// last candidate whose *natural* extent still fits, or the first candidate at all when even
/// that overflows, and both are always inside `run_dp`'s own wider window (which keeps
/// scanning past a natural overflow and only stops once the *ladder-adjusted* result is
/// itself `Overfull`, always evaluating and admitting the failing candidate itself before
/// giving up on anything past it). By induction over `FirstFit`'s own sequence of chosen
/// breaks, every edge that sequence uses is available to the untoleranced DP, so
/// `best[last_index]` after the untoleranced pass is never worse than `FirstFit`'s own total
/// under the same `preference` — and, since `nodes.len() >= 2` is checked before either pass
/// runs, it is never unreachable either. This is what makes the second, fallback minimization
/// below always succeed rather than merely usually succeed.
///
/// **Tolerance exhaustion (`docs/decisions/tolerance-exhaustion.md`).** The first pass
/// filters `run_dp`'s own edges by `tolerance`, exactly `Search::Optimal`'s own "discarding
/// any line worse than tolerance." When that leaves the paragraph's own end unreachable —
/// every complete arrangement needs at least one line no admitted edge can reach it through —
/// this function re-minimizes once more over the full, un-pruned edge set (`tolerance: None`)
/// rather than panicking or inventing a forbidden break (ADR-0010), and the result of that
/// second minimization, reachability just proved, is reported the ordinary way: every unfit
/// line's own [`ViolationKind`] still lands in [`Composition::violations`], and the caller
/// still gets an arrangement at least as good as `FirstFit` would have chosen.
///
/// **§3.5.4's widow term does not disturb either argument above.** Both rest on `run_dp`
/// and `compose_first_fit` scoring every candidate through the identical `demerits_of`
/// (`decision:widow-threshold`'s own "Why"), never on what any one component of `Demerits`
/// happens to hold — reachability is about which *edges* the window and the tolerance
/// filter admit, and the FirstFit-comparability bound is about `FirstFit`'s own sequence
/// being one path among the ones the untoleranced DP already searches, both unrelated to
/// whether `structural` is zero. `Optimal` consequently still never scores worse than
/// `FirstFit` under a nonzero `widow_threshold`, and a tolerance-exhausted paragraph still
/// always reaches a reported answer, exactly as before this round.
///
/// JLReq: §3.8, §3.1.12, §3.5.4, ADR-0010, `decision:tolerance-exhaustion`, `decision:widow-threshold`
fn compose_optimal(
    paragraph: Paragraph<'_>,
    runs: Runs<'_>,
    boundaries: &[ItemIndex],
    item_count: u32,
    policy: Policy,
    tolerance: Badness,
) -> Composition {
    let context = SearchContext {
        paragraph,
        runs,
        item_count,
        policy,
    };
    let text = paragraph.text;

    // The paragraph's own start, deduplicated against `boundaries`' own leading entry —
    // `boundaries` already carries `ItemIndex::new(0)` when (and only when) `item_count` is
    // itself zero (`ordered_boundaries`'s own ADR-0018 fallback), which would otherwise make
    // this a zero-length edge from item zero to itself rather than the "no candidate exists
    // at all" case `nodes.len() < 2` below reports honestly.
    let mut nodes = Vec::with_capacity(boundaries.len().saturating_add(1));
    nodes.push(ItemIndex::new(0));
    nodes.extend(boundaries.iter().copied());
    nodes.dedup();

    if nodes.len() < 2 {
        return no_feasible_break();
    }

    let preference = Preference::from_policy(policy);
    let last_index = nodes.len().saturating_sub(1);

    let mut best = run_dp(context, &nodes, preference, Some(tolerance));
    if best[last_index].is_none() {
        best = run_dp(context, &nodes, preference, None);
    }

    let Some(path_indices) = reconstruct_path(&best, last_index) else {
        return no_feasible_break();
    };

    let mut lines = Vec::new();
    let mut violations = Vec::new();
    let mut demerits = Demerits::ZERO;

    for window in path_indices.windows(2) {
        let start_index = window[0];
        let end_index = window[1];
        let start = nodes[start_index];
        let end = nodes[end_index];
        let is_last_line = end.get() >= item_count;

        let edge = evaluate_edge(context, start..end);
        let line_number = u32::try_from(lines.len()).unwrap_or(u32::MAX);

        if let Some(kind) = edge.outcome.kind {
            violations.push(Violation {
                line: line_number,
                at: end,
                rule: RuleId::POSSIBILITIES_FOR_LINE_BREAKING_BETWEEN_CHARACTERS,
                kind,
            });
        }

        if is_last_line {
            push_widow_violation(
                &mut violations,
                line_number,
                paragraph.widow_threshold,
                start,
                end,
            );
        }

        let adjustment = Adjustment::of(edge.outcome.deltas, edge.outcome.releveled);
        let pull_up = pull_up_of(
            &adjustment,
            edge.outcome.reduction_depth,
            &nodes,
            start_index,
            end_index,
        );
        demerits = demerits.add_sat(edge.demerits);

        let mut line = Line::from_geometry(
            start..end,
            byte_range(text, start, end),
            edge.adjusted,
            edge.demerits,
            is_last_line,
            adjustment,
            edge.outcome.hanging,
        );
        if let Some(pull_up) = pull_up {
            line = line.with_pull_up(pull_up);
        }
        lines.push(line);
    }

    Composition {
        lines,
        demerits,
        violations,
        rewrites: Vec::new(),
    }
}

/// What draining the ladder for one line produced: [`compose`]'s own extraction of
/// `ladder::reduce`, `ladder::hang` and `ladder::expand`'s combined result, split out so
/// that neither `compose` nor this function alone carries the whole pipeline
/// (`clippy::too_many_lines`).
struct LineAdjustment {
    /// The per-site deltas, dense and in [`Ladder::sites`] order, summed element-wise
    /// over whichever of `reduce`/`expand` touched each site.
    deltas: Vec<InlineExtent>,
    /// `Some` when the ladder, fully drained, still could not make the line fit.
    kind: Option<ViolationKind>,
    /// `Some` when [`ladder::hang`] let the line's own last item hang.
    hanging: Option<Hanging>,
    /// The deepest reduction stage engaged (2 through 6), or zero if none was.
    reduction_depth: u8,
    /// The deepest ordinary expansion stage engaged (2 or 3), or zero if none was.
    expansion_depth: u8,
    /// Whether expansion's own fourth, re-leveling stage ran.
    releveled: bool,
}

/// Run one line's own three-stage pipeline — §3.8.3's reduction, §3.8.2's hanging, then
/// §3.8.4's expansion — against `ladder`, and report what happened.
///
/// `ladder::reduce` is called unconditionally rather than guarded by a separate overfull
/// check: its own `need` is zero for an already-fitting line, and it is total over a zero
/// need (every stage's own `if remaining == ZERO { break }` fires immediately), so an
/// already-fitting line's own call is a no-op that returns the all-zero deltas
/// `apply_adjustment` needs anyway. §3.8.4's expansion is skipped for `is_last_line`
/// (§3.8.1's own Note: the last line still takes reduction, never expansion) and for a
/// line reduction and any hang already resolved — which also covers the case a
/// `Reduction::Discrete` flip overshot the target, leaving the line short by a small
/// amount `ladder::expand` then closes (`crate::ladder`'s own `# Status`).
///
/// `line` is a range and not two parameters: `clippy::too_many_arguments` (limit 7,
/// `geometry_of`'s own precedent) is exactly why, the same reason `trailing_of` builds
/// its own `line_end` once rather than taking the inputs that produce it.
fn adjust_line(
    text: Text<'_>,
    ladder: &Ladder,
    natural_extent: InlineExtent,
    target: InlineExtent,
    line: Range<ItemIndex>,
    is_last_line: bool,
    policy: Policy,
) -> LineAdjustment {
    let shortfall = overflow(natural_extent, target);
    let (mut deltas, mut remaining, reduction_depth) = ladder::reduce(ladder, shortfall, policy);

    let mut hanging = None;
    if remaining != InlineExtent::ZERO && line.end.get() > line.start.get() {
        let last = ItemIndex::new(line.end.get().saturating_sub(1));
        if let Some(candidate) = ladder::hang(text, last, remaining, policy) {
            remaining = remaining.sub_sat(candidate.beyond);
            hanging = Some(candidate);
        }
    }

    let mut kind = if remaining == InlineExtent::ZERO {
        None
    } else {
        Some(ViolationKind::Overfull(remaining))
    };

    let mut expansion_depth = 0u8;
    let mut releveled = false;
    if kind.is_none() && !is_last_line {
        let applied = deltas
            .iter()
            .copied()
            .fold(InlineExtent::ZERO, InlineExtent::add_sat);
        let after_reduction = natural_extent.add_sat(applied);
        let residual = overflow(target, after_reduction);
        if residual != InlineExtent::ZERO {
            let (expand_deltas, still_short, depth, did_relevel) =
                ladder::expand(ladder, residual, policy);
            for (slot, delta) in deltas.iter_mut().zip(expand_deltas) {
                *slot = slot.add_sat(delta);
            }
            expansion_depth = depth;
            releveled = did_relevel;
            if still_short != InlineExtent::ZERO {
                kind = Some(ViolationKind::ExpansionExhausted);
            }
        }
    }

    LineAdjustment {
        deltas,
        kind,
        hanging,
        reduction_depth,
        expansion_depth,
        releveled,
    }
}

/// The two facts §3.5.4 needs about one candidate line, once it is known to be the
/// paragraph's own last: how many characters it actually carries, and how far short of
/// `threshold` that count falls. Computed by one function, [`widow_facts_of`], and read by
/// both [`demerits_of`]'s own structural penalty and by [`compose_first_fit`] and
/// [`compose_optimal`]'s own violation report, so "how much the search was steered toward
/// this line" and "whether the arrangement finally chosen still falls short" are never two
/// independent formulas that could silently disagree
/// (`docs/decisions/widow-threshold.md`'s own Q1 and Q3).
#[derive(Debug, Clone, Copy)]
struct WidowFacts {
    /// How many characters this line actually carries.
    have: u32,
    /// How many characters short of `threshold` it falls, zero once `have` meets or
    /// exceeds it — [`Demerits::structural`]'s own value for this line.
    shortfall: u32,
}

/// §3.5.4's own count and shortfall for one line running from `start` to `end`.
///
/// **"Characters" reads as items.** `have` is the count `end.get().saturating_sub
/// (start.get())`, the same item-count reading [`evaluate_edge`]'s own doc already uses
/// for `is_last_line` and `first_line` — an occurrence, not a code point, is what this
/// crate ever classifies or counts at all (ADR-0008), so no other reading of "a character"
/// was available to take. A last item [`ladder::hang`] let hang past the measure is still
/// one of the characters this counts, without a special case: `hang`'s own `last` item
/// sits at `end`'s own immediate predecessor, inside `start..end` and never past it, so it
/// was already included in `have` before this function ever runs.
///
/// **The shortfall is proportional, not flat.** `u32::from(threshold).saturating_sub(have)`
/// is zero once satisfied and grows with how far short `have` falls otherwise, so a search
/// comparing two arrangements that both miss an unsatisfiable threshold still prefers the
/// one that misses by less, rather than tying every unsatisfiable arrangement and handing
/// the choice to whichever component of [`Demerits`] happens to rank next
/// (`docs/decisions/widow-threshold.md`'s own Q3).
///
/// JLReq: §3.5.4, `decision:widow-threshold`
fn widow_facts_of(threshold: u16, start: ItemIndex, end: ItemIndex) -> WidowFacts {
    let have = end.get().saturating_sub(start.get());
    let shortfall = u32::from(threshold).saturating_sub(have);
    WidowFacts { have, shortfall }
}

/// Push §3.5.4's own violation onto `violations` when the line running from `start` to
/// `end` — the caller's own last line, checked before this is called rather than inside it,
/// since only the caller knows the sense in which it is one — still falls short of
/// `threshold`. Called once from [`compose_first_fit`]'s own loop and once from
/// [`compose_optimal`]'s own reconstruction loop, both for the last line only, so the check
/// and the report are written once rather than diverging between the two.
///
/// JLReq: §3.5.4, `decision:widow-threshold`
fn push_widow_violation(
    violations: &mut Vec<Violation>,
    line_number: u32,
    threshold: u16,
    start: ItemIndex,
    end: ItemIndex,
) {
    let widow = widow_facts_of(threshold, start, end);
    if widow.shortfall > 0 {
        violations.push(Violation {
            line: line_number,
            at: start,
            rule: RuleId::WIDOW_ADJUSTMENT_OF_PARAGRAPHS,
            kind: ViolationKind::Widow {
                have: widow.have,
                want: threshold,
            },
        });
    }
}

/// This line's own cost, from what [`adjust_line`] reported and the extent it actually
/// realized once adjusted, plus §3.5.4's own structural penalty when this is the
/// paragraph's own last line.
///
/// The residual [`Badness::of`] reads is what the ladder, once fully drained, could not
/// place: zero for a conforming line (the fit is then exact, `realized_extent == target`),
/// and the leftover [`LineAdjustment::kind`] names for a violating one. The zero-flex
/// reading is [`Badness::of`]'s own defined case for a line whose ladder is genuinely
/// exhausted — see `crate` root's own `# Status` on this reading.
///
/// `line` and `is_last_line` are threaded the same way [`adjust_line`]'s own signature,
/// one call earlier in the identical pipeline, already threads them
/// (`clippy::too_many_arguments`, limit 7, `geometry_of`'s own precedent) — the shape was
/// already in every caller's own scope, not invented for this round. `widow_threshold` is
/// [`Paragraph::with_widow_threshold`]'s own stored field, threaded here as a plain `u16`
/// because this is the one function both [`compose_first_fit`] and [`evaluate_edge`] call
/// (`compose_optimal`'s own doc, "this round's own C1") to price a line.
/// [`push_widow_violation`] reads the same field too, separately, for the report rather
/// than the cost — but both routes converge on the identical [`widow_facts_of`] (its own
/// doc), so the widow term is still never scored by two formulas the two searches could
/// disagree about.
///
/// JLReq: §3.5.4, `decision:widow-threshold`
fn demerits_of(
    outcome: &LineAdjustment,
    target: InlineExtent,
    realized_extent: InlineExtent,
    line: Range<ItemIndex>,
    is_last_line: bool,
    widow_threshold: u16,
) -> Demerits {
    let leftover = match outcome.kind {
        Some(ViolationKind::Overfull(over)) => over,
        Some(ViolationKind::ExpansionExhausted) => overflow(target, realized_extent),
        _ => InlineExtent::ZERO,
    };
    let badness = Badness::of(leftover, InlineExtent::ZERO);
    let structural = if is_last_line {
        widow_facts_of(widow_threshold, line.start, line.end).shortfall
    } else {
        0
    };
    Demerits {
        structural,
        last_resort: u32::from(outcome.releveled),
        expansion_depth: u32::from(outcome.expansion_depth),
        reduction_depth: u32::from(outcome.reduction_depth),
        badness: badness.get(),
        hanging: u32::from(outcome.hanging.is_some()),
    }
}

/// Every item ordinal a feasible break could end a line at, in increasing order, always
/// ending with the text's own length (ADR-0018: the last line ends where the text does,
/// whether or not a candidate said so).
fn ordered_boundaries(breaks: &[FeasibleBreak], item_count: u32) -> Vec<ItemIndex> {
    let mut boundaries: Vec<ItemIndex> = breaks
        .iter()
        .copied()
        .map(FeasibleBreak::at)
        .filter(|at| at.get() > 0)
        .collect();
    boundaries.sort_by_key(|at| at.get());
    boundaries.dedup();
    if boundaries.last().copied().map(ItemIndex::get) != Some(item_count) {
        boundaries.push(ItemIndex::new(item_count));
    }
    boundaries
}

/// Whether `extent` fits inside `measure`: `extent <= measure`, read through the one
/// inherent ordering [`InlineExtent`] offers (ADR-0011 forbids `Ord`).
///
/// Crate-visible: [`crate::tab`] reads this the same way `compose` does here — "has the
/// cursor reached this tab stop yet" is the identical question as "does this line's own
/// extent still fit the measure", over the same ordering — rather than a second
/// `extent.min(measure) == extent` written out again at a second call site.
pub(crate) fn fits(extent: InlineExtent, measure: InlineExtent) -> bool {
    extent.min(measure) == extent
}

/// How far `extent` exceeds `target`, or zero when it does not — `InlineExtent::sub_sat`
/// alone cannot answer this: it is signed and saturates only at the axis's own bound, so
/// `extent.sub_sat(target)` for an `extent` short of `target` is a genuine negative value,
/// not zero, which is exactly the confusion this function exists to remove from every one
/// of its callers (`adjust_line`'s own `shortfall` and `residual`, each read as "how much
/// is owed", never as a signed difference).
pub(crate) fn overflow(extent: InlineExtent, target: InlineExtent) -> InlineExtent {
    if fits(extent, target) {
        InlineExtent::ZERO
    } else {
        extent.sub_sat(target)
    }
}

/// The byte range a line's items span.
///
/// Crate-visible: [`crate::align::align`] aligns the whole of one text as a single line and
/// needs the same byte accounting `compose` does, rather than a second computation of it.
pub(crate) fn byte_range(text: Text<'_>, start: ItemIndex, end: ItemIndex) -> Range<ByteOffset> {
    let items = text.items();
    let end_of_text = ByteOffset::new(u32::try_from(text.as_str().len()).unwrap_or(u32::MAX));
    let from = items
        .get(start.get() as usize)
        .copied()
        .map_or(end_of_text, UnitItem::start);
    let upto = items
        .get(end.get() as usize)
        .copied()
        .map_or(end_of_text, UnitItem::start);
    from..upto
}

/// One line's normalized geometry, computed once and reused for both the fit check and
/// the eventual [`Line`] this milestone builds directly from it.
///
/// Crate-visible for the same reason [`geometry_of`] is: [`crate::align::align`] is a
/// second producer of a [`Line`], over the same normalized-geometry pass, not a second
/// geometry computation (`docs/design/api-spine.md`'s own §3.7.3 note: "a whole line set to
/// a length and a span inside a line set to a length" share the computation, not just the
/// intent).
pub(crate) struct Geometry {
    pub(crate) placements: Vec<InlineOffset>,
    pub(crate) extent: InlineExtent,
    pub(crate) trailing: InlineExtent,
    pub(crate) trims: Vec<Trim>,
    /// The ladder sites this geometry's own boundaries carry, in boundary order —
    /// [`crate::ladder::Ladder::of`]'s own input. Always empty for a [`Geometry`] this
    /// module or [`crate::align`] builds from an *already-adjusted* pass, because there
    /// is no second ladder to drain over an adjusted line (`crate::compose::compose`
    /// drains the one this field carries once, before adjustment, and never rebuilds it).
    pub(crate) sites: Vec<Site>,
}

/// The site a boundary's own [`jlreq_spacing::Expansion`] needs when nothing else already
/// built one for it — a solid Table 1 cell with a real Table 6 ceiling, cl-19 against cl-19
/// (kanji beside kanji) being the coordinate that makes this reachable at all (ADR-0021).
/// `None` when a term-carrying site already exists for this boundary (`built > 0`: that
/// site already carries the identical `expansion`, attached alongside its own reduction by
/// [`Site::new`]'s own two-part construction, so a second site here would double-count the
/// same physical gap during [`crate::ladder::expand`]'s own headroom sum — this crate's own
/// `xtask attest` invariant, `expansion-needs-no-referent`, is what proves `built` is never
/// more than 1 at a coordinate this ever fires for) or when there is no opportunity to
/// report at all. `built` counts sites *actually pushed*, not `answer.spaces().count()`: a
/// term the `owner`/`owner_item` guard below skips leaves its own gap siteless, and this
/// fallback is the one place that gap's own expansion is still reachable.
fn expansion_only_site(
    built: usize,
    expansion: Expansion,
    size: Size,
    shift_from: Option<ItemIndex>,
) -> Option<Site> {
    if built > 0 || matches!(expansion, Expansion::None) {
        return None;
    }
    Some(Site::new(None, expansion, size, shift_from))
}

/// Every conditional space one boundary carries, folded into a `gap` (extra room between
/// two boxes, added to the cursor) or a [`Trim`] (room already inside a caller-supplied
/// box, per `docs/adr/0017-normalized-line-geometry.md`), plus the ladder [`Site`] each
/// term produces — exactly what `geometry_of`'s own per-item loop did with the
/// boundary's own [`Boundary::spaces`] before this helper existed. Split out to keep
/// `geometry_of` under `clippy::too_many_lines` without changing what it computes.
///
/// Also collects the boundary's own [`Boundary::expansion`], which is a fact about this
/// coordinate rather than about either neighbor's own contribution (ADR-0021): every
/// term-carrying [`Site`] this loop pushes is built with `answer.expansion()` already
/// attached (never `Expansion::None` twice over for the same boundary — see
/// `expansion_only_site`'s own doc for why that never double-counts), and
/// [`expansion_only_site`] is what still reports the opportunity when the loop pushed no
/// site at all — a solid Table 1 cell, `spaces()` empty, that Table 6 still expands.
fn boundary_spaces(
    text: Text<'_>,
    index: ItemIndex,
    answer: Boundary,
    carry: &mut Carry,
) -> (InlineExtent, InlineExtent, Vec<Trim>, Vec<Site>) {
    let items = text.items();
    let ordinal = index.get();
    let mut gap = InlineExtent::ZERO;
    let mut leading_trim = InlineExtent::ZERO;
    let mut trims = Vec::new();
    let mut sites = Vec::new();

    let before_index = if ordinal > 0 {
        ItemIndex::new(ordinal.saturating_sub(1))
    } else {
        index
    };
    let before_size = text.size_of(before_index);

    for space in answer.spaces() {
        let owner_is_this_item = space.referent() == Referent::Trailing;
        let owner = if owner_is_this_item {
            Some(index)
        } else if ordinal > 0 {
            Some(before_index)
        } else {
            None
        };
        let Some(owner) = owner else { continue };
        let Some(owner_item) = items.get(owner.get() as usize) else {
            continue;
        };
        let after_size = text.size_of(index);
        let amount = space.resolve(before_size, after_size, carry);
        let owner_size = if owner_is_this_item {
            after_size
        } else {
            before_size
        };
        sites.push(Site::new(
            Some(space),
            answer.expansion(),
            owner_size,
            Some(index),
        ));

        if owner_item.frame() == Frame::FullEm {
            trims.push(Trim {
                at: owner,
                amount,
                referent: space.referent(),
                rule: space.rule(),
            });
            if owner_is_this_item {
                leading_trim = leading_trim.add_sat(amount);
            }
        } else {
            gap = gap.add_sat(amount);
        }
    }

    if let Some(site) =
        expansion_only_site(sites.len(), answer.expansion(), before_size, Some(index))
    {
        sites.push(site);
    }

    (gap, leading_trim, trims, sites)
}

/// Which of a [`geometry_of`] call's own two edges are a genuine paragraph edge, as
/// opposed to a boundary this call's own `range` merely happens to start or end at.
///
/// Every call before this round intended both edges literally: `compose`'s own per-line
/// calls (a break the caller's own candidates chose, or the paragraph's own start) and
/// [`crate::align::align`]'s own single whole-text call (the run's own two ends, by
/// definition — see its own module doc). [`crate::tab`] is the first caller for which that
/// is sometimes false: §3.6.3's own tab-separated runs share one physical line, so only
/// the run [`crate::tab::tab_line`] places first genuinely opens it (行頭) and only the run
/// it places last genuinely closes it (行末); every run between sits at a tab position,
/// which JLReq never calls either — "an interior run starts at a tab position, which
/// JLReq never calls a line head" is this round's own brief, stated once here rather than
/// duplicated at every caller.
///
/// `head` and `end` therefore each answer one question independently: does this range's
/// own first (respectively last) item get [`Adjacency::at_line_head`] (respectively
/// [`Adjacency::at_line_end`]) treatment, or none? "None" is not a third table lookup —
/// it is the deliberate absence of one (see [`geometry_of`]'s own leading-edge branch and
/// its trailing-edge guard), because the specification never states a rule for the
/// boundary a tab creates and inventing one — even by routing it through the *other*
/// item's own true adjacency, across the gap — would be exactly the "boundary lookup
/// across a tab" §3.6.3's own second sentence forbids (`crate::tab`'s own module doc).
///
/// JLReq: §3.6.3
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Edges {
    head: bool,
    end: bool,
}

impl Edges {
    /// Both ends of `range` are genuine: every line `compose` builds and the one run
    /// `align` places, because neither ever sits beside a tab.
    pub(crate) const BOTH: Self = Self {
        head: true,
        end: true,
    };

    /// State which of the two edges are genuine independently. [`crate::tab::tab_line`]'s
    /// own first placed run is `head`; its own last placed run is `end`; a run may be
    /// neither, and — a single-run tab line — a run may be both, the same as
    /// [`Edges::BOTH`] states directly.
    pub(crate) const fn new(head: bool, end: bool) -> Self {
        Self { head, end }
    }
}

/// The adjacency this range's own first item consults for its leading edge, or `None`
/// when there is none to consult at all.
///
/// `edges.head` names which: `Some(Adjacency::at_line_head(..))` when this position is a
/// genuine line head (§3.1.2's own line-head row, Table 1's `before: 0`, blank in every
/// one of its 29 cells — see [`geometry_of`]'s own history of getting exactly this
/// question wrong once already), `None` when it is a tab position instead
/// ([`crate::tab`]'s own module doc). `None` is deliberately not routed through
/// [`Adjacency::between`] against whatever item precedes `range` — that would read a real
/// interior cell across a gap §3.6.3's own second sentence states no mojikumi rule
/// crosses — so the caller answers `None` with the same empty contribution
/// [`boundary_spaces`] would compute from a genuinely blank Table 1 row, without naming
/// this position a line head to get there.
fn range_head_adjacency<'r>(
    text: Text<'r>,
    runs: Runs<'r>,
    index: ItemIndex,
    head: bool,
    direction: Direction,
) -> Option<Adjacency<'r>> {
    head.then(|| Adjacency::at_line_head(text, runs, index, direction))
}

/// Compute one line's placements, extent, trailing space, trims and ladder sites, per
/// `docs/adr/0017-normalized-line-geometry.md`: every conditional space is explicit at a
/// boundary, and a caller-declared ideographic frame that already contains one is
/// normalized by subtracting it (a trim, reported rather than silently dropped).
///
/// No reduction and no expansion is applied here — this is the *unadjusted*, solid-setting
/// geometry, exact for a line the ladder never touches and, for every other line, the
/// starting point [`crate::ladder::reduce`] and [`crate::ladder::expand`] drain
/// [`Geometry::sites`] from and `crate::compose::apply_adjustment` shifts into the
/// adjusted result.
///
/// `range` bundles the two item bounds `start`/`end` used to be, the same
/// `clippy::too_many_arguments` reasoning `adjust_line`'s own `line: Range<ItemIndex>`
/// states, freeing the slot [`Edges`] now occupies.
///
/// Crate-visible: [`crate::align::align`] calls this directly over the whole of one text,
/// rather than composing a second implementation of §3.1.2's normalized geometry — and
/// never reads the `sites` this call collects, per its own `# What this is not`. Every
/// caller before this round passes [`Edges::BOTH`]; only [`crate::tab::tab_line`] passes
/// anything else.
pub(crate) fn geometry_of(
    text: Text<'_>,
    runs: Runs<'_>,
    range: Range<ItemIndex>,
    indent: InlineExtent,
    direction: Direction,
    policy: Policy,
    edges: Edges,
) -> Geometry {
    let items = text.items();
    let mut carry = Carry::new();
    let mut cursor = InlineCursor::new().advance(indent);
    let mut placements = Vec::new();
    let mut trims = Vec::new();
    let mut sites = Vec::new();

    for ordinal in range.start.get()..range.end.get() {
        let index = ItemIndex::new(ordinal);
        let Some(item) = items.get(ordinal as usize) else {
            break;
        };

        let adjacency = if ordinal == range.start.get() {
            // This range's own first item — not necessarily the paragraph's first
            // (`ordinal == 0` tested only the latter, and every later line's own head item
            // then fell into the `between` branch below, wrongly treating the break that
            // opened this line as still interior-adjacent to whatever item closed the
            // previous one). `boundary_spaces`'s own `ordinal > 0` branches never see the
            // consequence either way — Table 1's line-head row (`before: 0`) carries
            // `terms: &[]` in every one of its 29 cells, so a line head never produces a
            // conditional space for that code to misattribute. `range_head_adjacency`
            // reads `edges.head` to decide whether this item is a genuine line head at
            // all — see its own doc and [`Edges`]'s own.
            range_head_adjacency(text, runs, index, edges.head, direction)
        } else {
            let before = ItemIndex::new(ordinal.saturating_sub(1));
            Some(
                Adjacency::between(text, runs, before, direction)
                    .unwrap_or_else(|| Adjacency::at_line_head(text, runs, index, direction)),
            )
        };

        let (gap, leading_trim, boundary_trims, boundary_sites) = match adjacency {
            Some(adjacency) => {
                let answer = boundary(adjacency, policy);
                boundary_spaces(text, index, answer, &mut carry)
            },
            None => (
                InlineExtent::ZERO,
                InlineExtent::ZERO,
                Vec::new(),
                Vec::new(),
            ),
        };
        trims.extend(boundary_trims);
        sites.extend(boundary_sites);

        cursor = cursor.advance(gap);
        let Some(position) = cursor.position() else {
            break;
        };
        placements.push(
            InlineOffset::new(position.units().saturating_sub(leading_trim.units()))
                .unwrap_or(position),
        );
        cursor = cursor.advance(item.advance());
    }

    let last = range
        .end
        .get()
        .checked_sub(1)
        .and_then(|ordinal| items.get(ordinal as usize).map(|_| ItemIndex::new(ordinal)));
    let mut trailing = InlineExtent::ZERO;
    if edges.end {
        if let Some(last_index) = last {
            if let Some(&last_item) = items.get(last_index.get() as usize) {
                let line_end = boundary(
                    Adjacency::at_line_end(text, runs, last_index, direction),
                    policy,
                );
                let trailing_sites;
                (trailing, cursor, trailing_sites) = trailing_of(
                    text, last_index, last_item, line_end, &mut carry, cursor, &mut trims,
                );
                sites.extend(trailing_sites);
            }
            // No last item at `last_index` (defensive only: `last` is built from a
            // checked lookup at the same ordinal) leaves `trailing` at zero and `cursor`
            // untouched, which is the same answer the extent computation below already
            // gives a line with no last item — one fewer formula to keep in sync with the
            // other.
        }
        // `edges.end` false (a tab position, `crate::tab`'s own interior runs) skips this
        // whole block rather than computing it and discarding the result: §B.2's own
        // line-end rules — a trailing half em after cl-06/cl-07, most visibly — are a fact
        // about a genuine line end, and a run followed by another run across a tab is not
        // one (this round's own demonstrated failure mode; see `crate::tab`'s own
        // regression test).
    }

    let extent = cursor
        .position()
        .map(InlineOffset::units)
        .and_then(InlineExtent::new)
        .unwrap_or(InlineExtent::ZERO);

    Geometry {
        placements,
        extent,
        trailing,
        trims,
        sites,
    }
}

/// The realized conditional space after a line's last item (§3.1.9, §B.2#2), added to
/// `cursor` and, where the item's caller-declared frame already contains it, recorded as
/// a trim instead of an advance — `geometry_of`'s own per-item loop makes the identical
/// choice at every other boundary, and the line's last boundary is not an exception to it.
/// The [`Site`]s returned carry no `shift_from` (`None`): the line-end boundary has no
/// item after it to shift, only [`crate::Line::trailing`] and [`crate::Line::extent`]
/// move when one of these is drained.
///
/// Split out of [`geometry_of`] to keep both functions under `clippy::too_many_lines`
/// without changing what either computes: this is exactly the block that used to run
/// after the per-item loop, unchanged, so it is a name for existing arithmetic rather
/// than a new computation. `line_end` is computed by the caller, which already has
/// `runs`, `direction` and `policy` in scope, rather than threaded through here too —
/// `clippy::too_many_arguments` (limit 7) is exactly why `geometry_of` builds it once and
/// hands over the answer instead of the four inputs that produce it, and why the sites
/// this collects are returned rather than threaded through as an eighth `&mut` parameter.
///
/// Threads `line_end.expansion()` into every [`Site`] this pushes, the same as
/// [`boundary_spaces`] does — real code, not a stub, even though it is provably a no-op
/// here: §E.1 states outright that Table 6 carries no line-edge axis at all ("there are no
/// cells involving line head or line end"), so `line_end.expansion()` is always
/// `Expansion::None` for this boundary and [`expansion_only_site`] never fires here for the
/// same structural reason. That is a different thing from a call that could never fire
/// standing in for a real evaluator (`crate::ladder`'s own doc draws that line for §3.1.11
/// item 2(g)): this reads the identical field every other boundary in this crate reads,
/// through the identical helper, and happens to always get the identical answer at this one
/// edge — not a seam waiting on data that does not exist yet.
fn trailing_of(
    text: Text<'_>,
    last_index: ItemIndex,
    last_item: UnitItem,
    line_end: Boundary,
    carry: &mut Carry,
    mut cursor: InlineCursor,
    trims: &mut Vec<Trim>,
) -> (InlineExtent, InlineCursor, Vec<Site>) {
    let mut trailing = InlineExtent::ZERO;
    let mut sites = Vec::new();
    let size = text.size_of(last_index);

    for space in line_end.spaces() {
        if space.referent() != Referent::Preceding {
            continue;
        }
        let amount = space.resolve(size, size, carry);
        trailing = trailing.add_sat(amount);
        sites.push(Site::new(Some(space), line_end.expansion(), size, None));
        if last_item.frame() == Frame::FullEm {
            trims.push(Trim {
                at: last_index,
                amount,
                referent: space.referent(),
                rule: space.rule(),
            });
        } else {
            cursor = cursor.advance(amount);
        }
    }

    if let Some(site) = expansion_only_site(sites.len(), line_end.expansion(), size, None) {
        sites.push(site);
    }

    (trailing, cursor, sites)
}

/// Turn a natural [`Geometry`] and the final, combined per-site deltas
/// [`crate::ladder::reduce`] and [`crate::ladder::expand`] produced (already summed
/// element-wise over one [`Ladder`]'s own site order — a site can carry both a
/// reduction-authored delta and an expansion-authored one across one line's own
/// pipeline, see `crate::ladder`'s own `# Status`) into the adjusted geometry
/// [`Line::from_geometry`] reports.
///
/// Every site's delta is realized the same way regardless of whether the *natural* pass
/// expressed that boundary as a `gap` (extra room between two boxes) or a [`Trim`] (room
/// already folded into one caller-supplied box, per `docs/adr/0017`): every placement at
/// or after the site's own boundary shifts by the delta, because a reduced or expanded
/// conditional space moves everything downstream by exactly that much either way.
/// [`Geometry::trims`] is carried over unchanged — a trim records what a caller-supplied
/// advance already contained, which the ladder does not touch, only how much room
/// surrounds it.
///
/// `line_start` rebases [`Site::shift_from`]'s ordinal, which — like every other
/// [`ItemIndex`] this crate reports — is the *text's* own ordinal (`boundary_spaces`
/// builds it from `geometry_of`'s per-item loop over the absolute `range.start.get()..
/// range.end.get()` span), against `natural.placements`, which is *line*-relative: `geometry_of` pushes one
/// entry per item of the line it was asked for, starting from index zero regardless of
/// where that line sits in the paragraph (`crate::align::align` reads this identically —
/// its own `placements[0]` is item zero of the run it aligns, not of some larger stream).
/// A first line, whose `line_start` is always item zero, has the two coincide, which is
/// exactly why a single-line or first-line case cannot surface a mismatch between them; a
/// second or later line does not, and skipping by the raw text ordinal there either walks
/// off the end of this — shorter — vector (dropping the shift entirely) or, worse, lands on
/// the wrong entry. Subtracting `line_start` first is the one correction this needs.
///
/// JLReq: §3.8.2, §3.8.3, §3.8.4
fn apply_adjustment(
    natural: &Geometry,
    ladder: &Ladder,
    deltas: &[InlineExtent],
    line_start: ItemIndex,
) -> Geometry {
    let mut placements = natural.placements.clone();
    let mut extent_delta = InlineExtent::ZERO;
    let mut trailing_delta = InlineExtent::ZERO;

    for (site, &delta) in ladder.sites().iter().zip(deltas) {
        if delta == InlineExtent::ZERO {
            continue;
        }
        extent_delta = extent_delta.add_sat(delta);
        match site.shift_from() {
            Some(from) => {
                let relative = from.get().saturating_sub(line_start.get());
                for placement in placements.iter_mut().skip(relative as usize) {
                    *placement = shift_by(*placement, delta);
                }
            },
            None => trailing_delta = trailing_delta.add_sat(delta),
        }
    }

    Geometry {
        placements,
        extent: natural.extent.add_sat(extent_delta),
        trailing: natural.trailing.add_sat(trailing_delta),
        trims: natural.trims.clone(),
        sites: Vec::new(),
    }
}

/// `docs/adr/0017-normalized-line-geometry.md`'s own definition of extent, in full:
/// "including the realized conditional space at the line end and excluding any character
/// placed outside the measure by §2.5.1's hanging punctuation." [`apply_adjustment`]
/// realizes the first half — every [`Site`] delta [`ladder::reduce`] and [`ladder::expand`]
/// produced — but hanging is never a `Site` ([`ladder::hang`]'s own doc), so
/// [`apply_adjustment`] has nothing to subtract it with. [`compose`] has both the adjusted
/// extent and the line's own [`Hanging`] in scope too, but calling this there would push
/// it past `clippy::too_many_lines`; [`Line::from_geometry`] is where a [`Geometry`]
/// becomes the [`Line`] this crate reports, already takes `hanging` as its own parameter,
/// and is the one other place both are in scope, so it calls this over its own result
/// instead.
///
/// JLReq: n/a (ADR-0017)
fn exclude_hung_overhang(extent: InlineExtent, hanging: Option<Hanging>) -> InlineExtent {
    match hanging {
        Some(hanging) => extent.sub_sat(hanging.beyond),
        None => extent,
    }
}

/// Move a placement by a signed, already-realized extent — the same crossing of the
/// untyped channel `align.rs`'s own `shift_by` makes (`docs/scalar-sites.toml`):
/// `InlineOffset` and `InlineExtent` share no arithmetic (ADR-0011), so this reads back
/// the two raw unit counts and re-enters the typed channel through `InlineOffset::new`.
/// `by` is signed rather than always positive, unlike `align.rs`'s own call sites: a
/// reduction moves a placement *back*, an expansion moves it *forward*, and
/// `saturating_add` on the raw `i32` handles both directions identically.
///
/// Crate-visible: [`crate::align::even_spacing_placements`] shifts jidori's own
/// distributed shares through the identical crossing, over a different set of sites, and
/// is the second and only other caller of it, rather than a second, independently
/// drifting copy.
///
/// Saturating on overflow rather than refusing: every `by` this module ever passes is
/// bounded by one line's own extent, itself bounded by the shared length bound, so the
/// fallback is reached only past inputs no caller-stated measure produces in practice;
/// the fallback is the pre-shift offset, unmoved.
pub(crate) fn shift_by(offset: InlineOffset, by: InlineExtent) -> InlineOffset {
    InlineOffset::new(offset.units().saturating_add(by.units())).unwrap_or(offset)
}

/// The composed result: every line, its cost, and whatever could not be satisfied.
///
/// JLReq: §3.8
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Composition {
    lines: Vec<Line>,
    demerits: Demerits,
    violations: Vec<Violation>,
    rewrites: Vec<Rewrite>,
}

impl Composition {
    /// The composed lines.
    ///
    /// JLReq: §3.8
    #[must_use]
    pub fn lines(&self) -> &[Line] {
        &self.lines
    }

    /// The total cost of composing this way. Non-optional: [`Search::FirstFit`] reaches
    /// one for every paragraph.
    ///
    /// JLReq: §3.8.2, §3.8.3, §3.8.4
    #[must_use]
    pub fn demerits(&self) -> Demerits {
        self.demerits
    }

    /// Rules the composition could not satisfy. Empty for a conforming result.
    ///
    /// JLReq: §3.8
    #[must_use]
    pub fn violations(&self) -> &[Violation] {
        &self.violations
    }

    /// §B.2 note 14 (c)'s `々` replacement. Always empty at M1: no case in this milestone's
    /// composition detects a line-head iteration mark, which is real logic over the
    /// composed line rather than a rule table, and is not implemented — see this module's
    /// own `# Status`.
    ///
    /// JLReq: §B.2#14
    #[must_use]
    pub fn rewrites(&self) -> &[Rewrite] {
        &self.rewrites
    }

    /// Every rule this composition applied.
    ///
    /// JLReq: n/a (ADR-0013)
    pub fn rules_fired(&self) -> impl Iterator<Item = RuleId> + '_ {
        self.violations.iter().map(|violation| violation.rule)
    }

    /// Frozen projection (ADR-0012): whether every line is conforming.
    ///
    /// JLReq: n/a (ADR-0012)
    #[must_use]
    pub fn is_conforming(&self) -> bool {
        self.violations.is_empty()
    }
}

/// One composed line.
///
/// JLReq: §3.8
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Line {
    items: Range<ItemIndex>,
    bytes: Range<ByteOffset>,
    placements: Vec<InlineOffset>,
    extent: InlineExtent,
    trailing: InlineExtent,
    trims: Vec<Trim>,
    adjustment: Adjustment,
    demerits: Demerits,
    is_last: bool,
    hanging: Option<Hanging>,
    pull_up: Option<PullUp>,
}

impl Line {
    /// Build one line from its normalized [`Geometry`] and the pieces [`compose`] and
    /// [`crate::align::align`] each compute independently: which items and bytes it covers,
    /// its cost, whether it is the paragraph's last, what the ladder did to it, and
    /// whether its own last item hangs. Crate-visible, and the only place either producer
    /// assembles the struct, so `Line`'s own fields are never named twice.
    ///
    /// `geometry.extent` is passed through [`exclude_hung_overhang`] before it becomes
    /// `Line::extent` — ADR-0017's own definition of extent excludes the amount `hanging`
    /// names. [`compose`] has both values in scope too, but is already at
    /// `clippy::too_many_lines`'s own limit; calling this here instead, where a `Geometry`
    /// is turned into the `Line` this crate reports and `hanging` is already a parameter,
    /// needs no new one.
    ///
    /// [`crate::align::align`] always passes [`Adjustment::empty`] and `None`: §3.8.1's
    /// own Note is why single line alignment is a distinct process from line adjustment
    /// (`crate::align`'s own module doc), so no call there ever drains [`crate::Ladder`]
    /// or offers hanging — `exclude_hung_overhang` is consequently always a no-op for it.
    ///
    /// `pull_up` starts `None` here unconditionally, for every caller: naming it as an
    /// eighth parameter would trip `clippy::too_many_arguments` and would force
    /// [`crate::align::align`] and [`crate::tab::tab_line`] — the other two callers, neither
    /// of which ever runs a comparison [`Line::pull_up`] could report — to each pass a value
    /// they do not have an opinion about. [`compose_optimal`] is the one caller that ever has
    /// one, and it sets it with [`Line::with_pull_up`] after this returns, which is what
    /// keeps this constructor's own signature, and the two other call sites, untouched by
    /// this round (this round's own C6).
    ///
    /// JLReq: §3.8
    pub(crate) fn from_geometry(
        items: Range<ItemIndex>,
        bytes: Range<ByteOffset>,
        geometry: Geometry,
        demerits: Demerits,
        is_last: bool,
        adjustment: Adjustment,
        hanging: Option<Hanging>,
    ) -> Self {
        Self {
            items,
            bytes,
            placements: geometry.placements,
            extent: exclude_hung_overhang(geometry.extent, hanging),
            trailing: geometry.trailing,
            trims: geometry.trims,
            adjustment,
            demerits,
            is_last,
            hanging,
            pull_up: None,
        }
    }

    /// Record §3.1.12 ⑤'s own outcome for this line, once [`compose_optimal`] has actually
    /// run the comparison [`Line::pull_up`] reports. Crate-visible and a builder rather than
    /// an [`Line::from_geometry`] parameter — see that constructor's own doc for why.
    ///
    /// JLReq: §3.1.12
    #[must_use]
    pub(crate) fn with_pull_up(mut self, pull_up: PullUp) -> Self {
        self.pull_up = Some(pull_up);
        self
    }

    /// The base stream's items on this line.
    ///
    /// JLReq: n/a (addressing)
    #[must_use]
    pub fn items(&self) -> Range<ItemIndex> {
        self.items.clone()
    }

    /// The bytes this line covers.
    ///
    /// JLReq: n/a (addressing)
    #[must_use]
    pub fn bytes(&self) -> Range<ByteOffset> {
        self.bytes.clone()
    }

    /// The caller's own glyph-box origins, on this line's own inline axis and relative to
    /// this line's own origin. One entry per item of [`Line::items`].
    ///
    /// JLReq: §3.1.2
    #[must_use]
    pub fn placements(&self) -> &[InlineOffset] {
        &self.placements
    }

    /// From the line-head origin to the line end, in normalized geometry: including
    /// [`Line::trailing`] and excluding the character [`Line::hanging`] names, which §2.5.1
    /// permits past the measure (`docs/adr/0017-normalized-line-geometry.md`'s own
    /// definition, in full).
    ///
    /// JLReq: n/a (ADR-0017)
    #[must_use]
    pub const fn extent(&self) -> InlineExtent {
        self.extent
    }

    /// The realized conditional space at the line end, whether or not it lives inside the
    /// last item's supplied advance.
    ///
    /// JLReq: §3.1.9, §B.2#2
    #[must_use]
    pub const fn trailing(&self) -> InlineExtent {
        self.trailing
    }

    /// Every unit composition took out of a caller-supplied advance, with the rule that
    /// states it.
    ///
    /// JLReq: §3.1.2
    #[must_use]
    pub fn trims(&self) -> &[Trim] {
        &self.trims
    }

    /// The sub-lines of every segment that touches this line. Always empty at M1: no
    /// paragraph this milestone composes ever carries a segment (this crate's own
    /// `# Status`).
    ///
    /// `&self` and not an associated function: [`Part`] is uninhabited only until M4 gives
    /// it real variants, and this milestone's own emptiness must not fix the signature
    /// M4's non-empty answer needs (`docs/design/api-spine.md`'s frozen shape).
    ///
    /// JLReq: §3.2.5, §3.4.2, §3.4.3, §3.7.2, §3.7.3
    #[must_use]
    pub fn parts(&self) -> &[Part<'_>] {
        let _: &Self = self;
        &[]
    }

    /// What was done to this line to make it fit.
    ///
    /// JLReq: §3.8.2, §3.8.3, §3.8.4
    #[must_use]
    pub const fn adjustment(&self) -> &Adjustment {
        &self.adjustment
    }

    /// This line's own cost.
    ///
    /// JLReq: §3.8.2
    #[must_use]
    pub const fn demerits(&self) -> Demerits {
        self.demerits
    }

    /// Ruby overhang allowances. Always empty at M1: no item on any line this milestone
    /// composes belongs to a ruby construct, because [`Paragraph`] never carries one (this
    /// crate's own `# Status`). `&self` is kept real for the same reason [`Line::parts`]'s
    /// is.
    ///
    /// JLReq: §3.3.8, §B.1, §B.2#8
    #[must_use]
    pub fn overhang(&self) -> &[RubyOverhang] {
        let _: &Self = self;
        &[]
    }

    /// §3.1.12 ⑤ as it happened on this line. Always `None` under [`Search::FirstFit`],
    /// full stop, on every line that search ever composes: the choice it reports is
    /// between two *different* candidate breaks compared by their reduction and expansion
    /// cost, and `FirstFit` commits to one candidate before the ladder ever runs — see
    /// [`Search::FirstFit`]'s own doc for why that is a search-scope fact rather than a
    /// ladder one, unrelated to whether the ladder itself is filled.
    ///
    /// `Some` under [`Search::Optimal`] exactly when the search actually ran that
    /// comparison for this line and took the pull-up (追い込み) reading of it — a shorter,
    /// evaluated alternative break existed for this same line and the search chose the
    /// longer one instead, reduction closing what the shorter choice would not have needed
    /// to close at all. `None` under `Optimal` too when no such alternative existed to
    /// compare against (`compose_optimal`'s own `pull_up_of`), which is the ordinary case
    /// for a short paragraph with few feasible breaks per line — a search finding nothing
    /// to compare is not a defect, only the absence of a choice to report.
    ///
    /// `&self` is kept real for the same reason [`Line::parts`]'s is.
    ///
    /// JLReq: §3.1.12, §3.8.2
    #[must_use]
    pub const fn pull_up(&self) -> Option<PullUp> {
        self.pull_up
    }

    /// A character placed outside the measure (ぶら下げ). `Some` exactly when
    /// `crate::ladder::hang` let this line's own last item hang rather than be reduced
    /// or expanded for.
    ///
    /// JLReq: §3.8.2, §2.5.1
    #[must_use]
    pub const fn hanging(&self) -> Option<Hanging> {
        self.hanging
    }

    /// Carried through opaquely and reported; never acted on. Always empty at M1: no
    /// paragraph this milestone composes ever carries a construct that could demand block
    /// space (this crate's own `# Status`), the same reason [`Line::overhang`] is always
    /// empty. `&self` is kept real for the same reason [`Line::parts`]'s is.
    ///
    /// JLReq: §4.5.1
    #[must_use]
    pub fn block_demand(&self) -> &[BlockDemand] {
        let _: &Self = self;
        &[]
    }

    /// Whether this line is exempt from expansion. §3.8.1's Note: the last line still
    /// takes reduction.
    ///
    /// JLReq: §3.8.1
    #[must_use]
    pub const fn is_last(&self) -> bool {
        self.is_last
    }
}

/// One sub-line of one segment, on a [`Line`]. Uninhabited at M1: nothing here ever
/// constructs one, because [`Line::parts`] is always empty (this crate's own `# Status`).
///
/// JLReq: §3.2.5, §3.4.2, §3.4.3, §3.7.2, §3.7.3
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct Part<'l> {
    _never: core::convert::Infallible,
    _lifetime: core::marker::PhantomData<&'l ()>,
}

/// One conditional space that was already inside a caller-supplied advance and has been
/// taken out of it.
///
/// JLReq: §3.1.2, §B.1
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Trim {
    /// The item the trim came off.
    pub at: ItemIndex,
    /// How much was trimmed.
    pub amount: InlineExtent,
    /// Which side of the boundary the trimmed space belongs to.
    pub referent: Referent,
    /// The rule that states this space.
    pub rule: RuleId,
}

/// A rule composition could not satisfy on one line.
///
/// JLReq: n/a (ADR-0013)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Violation {
    /// Which line, in composition order.
    pub line: u32,
    /// Where on that line.
    pub at: ItemIndex,
    /// The rule that was not satisfied.
    pub rule: RuleId,
    /// What kind of violation it is.
    pub kind: ViolationKind,
}

/// What kind of violation one line has.
///
/// JLReq: n/a (ADR-0013)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ViolationKind {
    /// The ladder drained every reduction stage, hanging punctuation offered nothing more
    /// (or was not eligible), and the line's own extent still exceeds the measure by this
    /// much.
    Overfull(InlineExtent),
    /// The ladder drained every expansion stage and the line is still short.
    ExpansionExhausted,
    /// No candidate the caller supplied is feasible at this position.
    NoFeasibleBreak,
    /// A placement the tables forbid.
    ForbiddenPlacement,
    /// §3.5.4: the arrangement a search finally settled on still leaves the paragraph's own
    /// last line short of `Paragraph::with_widow_threshold`'s own count. `have` and `want`
    /// are the same two facts [`Demerits::structural`]'s own shortfall is computed from
    /// (`WidowFacts`), carried here because a demerit is this crate's own invention and
    /// never itself a JLReq-shaped fact a conformance case may assert
    /// (`docs/decisions/widow-threshold.md`'s own "Why"), while this variant — reached
    /// through [`Violation::rule`] naming [`RuleId::WIDOW_ADJUSTMENT_OF_PARAGRAPHS`] — is.
    ///
    /// A minor addition under `#[non_exhaustive]` (ADR-0012): every existing match on this
    /// enum outside this crate was already required to carry a wildcard arm, so no caller's
    /// own exhaustive match is broken by this variant's existence.
    Widow {
        /// How many characters the chosen last line actually carries.
        have: u32,
        /// How many `Paragraph::with_widow_threshold` asked for.
        want: u16,
    },
}

/// Why [`compose`], [`crate::align::align`] or [`crate::tab::tab_line`] refused an input
/// outright, rather than reporting it as a rule the composition could not satisfy.
///
/// JLReq: n/a (addressing)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ComposeError {
    /// A length exceeded the range invariant.
    OutOfRange {
        /// Where.
        at: ItemIndex,
    },
    /// A candidate lies outside the text.
    CandidateOutOfRange {
        /// The offset that is out of range.
        at: ByteOffset,
    },
    /// §3.6.1 requires as many declared tab stops as tab signs — "it is necessary to set
    /// the same numbers of tab positions and tab types as the number of tab signs". Fewer
    /// stops than target runs is consequently a malformed declaration, caught once at the
    /// call itself, and is a different state from §3.6.3(d)'s own runtime outcome — a run
    /// that finds no *remaining* stop once an earlier run's own overflow has consumed one
    /// or more of the stops this rule requires there to be enough of, which
    /// [`crate::tab::TabLine::deferred`] reports rather than refusing the call (see
    /// `crate::tab`'s own module doc for why the two are not the same thing).
    InsufficientTabStops {
        /// How many target runs [`crate::tab::tab_line`] was asked to place.
        targets: u32,
        /// How many tab stops were declared for them.
        stops: u32,
    },
}

/// §3.1.12 ⑤'s repair, as applied: `amount` was reclaimed in this line so the next line's
/// first item moved up.
///
/// JLReq: §3.1.12
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct PullUp {
    /// How much was reclaimed.
    pub amount: InlineExtent,
    /// Which item moved up as a result.
    pub pulls: ItemIndex,
    /// The rule that states the repair.
    pub rule: RuleId,
}

/// A character placed outside the measure (ぶら下げ, burasage). Only cl-06 and cl-07 are
/// ever hung.
///
/// JLReq: §3.8.2, §2.5.1
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Hanging {
    /// The item hung past the measure.
    pub item: ItemIndex,
    /// By how much it juts past the measure.
    pub beyond: InlineExtent,
    /// The rule that permits it.
    pub rule: RuleId,
}

/// A required edit to the character stream. §B.2 note 14 (c) replaces `々` with the
/// character it repeats.
///
/// JLReq: §B.2#14
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Rewrite {
    /// The item to replace.
    pub at: ItemIndex,
    /// What to replace it with.
    pub replace_with: jlreq_class::Member,
    /// The rule that requires it.
    pub rule: RuleId,
}

#[cfg(test)]
mod tests {
    use jlreq_class::Text;
    use jlreq_spec::{Policy, Question};
    use jlreq_unit::{
        Advance, ByteOffset, Direction, Frame, InlineExtent, InlineOffset, Item, ItemIndex, Runs,
        Scale, ScaleId,
    };

    use super::{
        Badness, Candidate, Demerits, Edges, Paragraph, RuleId, Search, ViolationKind, compose,
        geometry_of,
    };

    fn scale(units: i32) -> Scale {
        let em = Advance::new(units).expect("a positive advance");
        Scale::new(em, em).expect("a positive scale")
    }

    fn offset(units: i32) -> InlineOffset {
        InlineOffset::new(units).expect("a valid offset")
    }

    /// A one-em ideograph (cl-19), full frame, at `byte_start` — the shared fixture unit
    /// every DP test below builds a text out of: cl-19 against cl-19 is blank in Table 1 (no
    /// interior gap, no reduce headroom at all) and `0-1/4 stage 3` in Table 6 (a real,
    /// bounded expansion opportunity), which is what makes an arrangement of these items
    /// hand-computable — every candidate line's own extent is exactly its item count times
    /// 1000, and closing any shortfall goes through Table 6 alone.
    fn kanji(byte_start: u32) -> Item {
        Item::new(
            ByteOffset::new(byte_start),
            InlineExtent::new(1000).expect("a valid extent"),
            ScaleId::new(0),
        )
        .with_frame(Frame::FullEm)
    }

    /// Regression for the bug this pass's own verification found: `apply_adjustment` used
    /// to index into `natural.placements` — which is *line*-relative, one entry per item
    /// of the line being built, starting from zero regardless of where that line sits in
    /// the paragraph — with [`crate::ladder::Site::shift_from`]'s own ordinal, which is the
    /// *text's* absolute one. On a paragraph's first line the two coincide, which is
    /// exactly why the two existing compose conformance cases (§3.5.1, §3.8.1), both
    /// two-line paragraphs whose text carries no reducible boundary at all, never
    /// surfaced it: their second line's every delta is zero, so the broken indexing was
    /// never reached.
    ///
    /// The fixture: "亜亜・・". A first-line indent pushes line 1 to end exactly at the
    /// first ideograph (`亜`), so line 2 starts at the *second* item of the text — the
    /// smallest offset at which `shift_from`'s absolute ordinal and a line-relative index
    /// genuinely diverge. Line 2 is then forced onto the one remaining candidate spanning
    /// its own three items (second `亜`, then the two middle dots `・・`) — no candidate is
    /// offered between them — whose natural extent overflows the shared measure by 500
    /// units, exactly the two middle dots' own combined reducible headroom (§D.2 note 1)
    /// plus the second ideograph's own boundary against the first middle dot, so
    /// `ladder::reduce` has real, multi-site work to do and the line still fits exactly
    /// once it is done — no `Violation` survives.
    #[test]
    fn a_second_lines_placements_are_shifted_by_the_reduction_the_ladder_applied() {
        let items = [
            Item::new(
                ByteOffset::new(0),
                InlineExtent::new(1000).expect("a valid extent"),
                ScaleId::new(0),
            )
            .with_frame(Frame::FullEm),
            Item::new(
                ByteOffset::new(3),
                InlineExtent::new(1000).expect("a valid extent"),
                ScaleId::new(0),
            )
            .with_frame(Frame::FullEm),
            Item::new(
                ByteOffset::new(6),
                InlineExtent::new(500).expect("a valid extent"),
                ScaleId::new(0),
            )
            .with_frame(Frame::HalfEm),
            Item::new(
                ByteOffset::new(9),
                InlineExtent::new(500).expect("a valid extent"),
                ScaleId::new(0),
            )
            .with_frame(Frame::HalfEm),
        ];
        let scales = [scale(1000)];
        let text = Text::new("亜亜・・", &items, &scales).expect("a well formed stream");
        let candidates = [
            Candidate::At(ByteOffset::new(0)),
            Candidate::At(ByteOffset::new(3)),
            Candidate::At(ByteOffset::new(12)),
        ];
        let measure = InlineExtent::new(2500).expect("a valid extent");
        let paragraph = Paragraph::new(text, &candidates, measure, Direction::Horizontal)
            .with_first_line_indent(InlineExtent::new(1500).expect("a valid extent"));

        let composition =
            compose(paragraph, Policy::JLREQ, Search::FirstFit).expect("a well formed input");

        assert!(
            composition.violations().is_empty(),
            "the line fits once the ladder is fully drained: {:?}",
            composition.violations()
        );
        let composed = composition.lines();
        assert_eq!(
            composed.len(),
            2,
            "the fixture is built to force exactly two lines"
        );

        let second = &composed[1];
        assert_eq!(
            second.items().start.get(),
            1,
            "line 2 starts at the text's second item, not its first — the case this bug needed"
        );
        assert_eq!(
            second.placements(),
            &[offset(0), offset(1166), offset(2000)],
            "the buggy, unrebased indexing computes [0, 1250, 2166] for this fixture — a \
             different, wrong answer — because it skips too far into a 3-entry vector for \
             two of the four sites and not far enough for the other two"
        );
        assert_eq!(
            second.extent(),
            InlineExtent::new(2500).expect("a valid extent"),
            "500 units of reduction across the line's own four sites closes the gap to \
             the shared measure exactly"
        );
    }

    /// Regression for the line-head bug this pass's own verification found: `geometry_of`
    /// tested `ordinal == 0` — true only for the *paragraph's* very first item — rather
    /// than `ordinal == start.get()`, so the first item of every line but the paragraph's
    /// own first fell into the `Adjacency::between` branch, wrongly read against whatever
    /// item closed the *previous* line, instead of `Adjacency::at_line_head`.
    ///
    /// Invisible on ordinary fixtures because Table 1's own line-head row (`before: 0`) is
    /// `terms: &[]` in every one of its 29 cells — a correctly-computed line head never
    /// produces a conditional space, so only a fixture whose wrongly-consulted *interior*
    /// cell carries a real term exposes the difference. cl-02 (closing bracket) before
    /// cl-19 (ideograph) does, at half an em, on the half-em frame (added as a gap, not
    /// trimmed).
    ///
    /// Calls [`geometry_of`] directly with `start` past the paragraph's own first item —
    /// the same shape [`compose`] builds for a line's own head — rather than routing
    /// through `compose`, because forcing a break exactly here also needs Table 2 to
    /// permit a line to *end* on the closing bracket, which it does not (`×`, kinsoku
    /// line-end prohibition): no [`Candidate`] here could ever reach this boundary through
    /// [`compose`]'s own feasibility check, only `geometry_of` called with an already-cut
    /// range exercises it.
    #[test]
    fn geometry_of_reads_a_lines_own_head_as_the_line_head_not_the_previous_lines_close() {
        let items = [
            Item::new(
                ByteOffset::new(0),
                InlineExtent::new(500).expect("a valid extent"),
                ScaleId::new(0),
            )
            .with_frame(Frame::HalfEm),
            Item::new(
                ByteOffset::new(3),
                InlineExtent::new(1000).expect("a valid extent"),
                ScaleId::new(0),
            )
            .with_frame(Frame::FullEm),
        ];
        let scales = [scale(1000)];
        let text = Text::new("」亜", &items, &scales).expect("a well formed stream");

        let geometry = geometry_of(
            text,
            Runs::none(),
            ItemIndex::new(1)..ItemIndex::new(2),
            InlineExtent::ZERO,
            Direction::Horizontal,
            Policy::JLREQ,
            Edges::BOTH,
        );

        assert_eq!(
            geometry.placements,
            [offset(0)],
            "the buggy reading, consulting the interior cl-02×cl-19 cell instead of the \
             blank line-head row, pushes this item's own placement to 500 by the closing \
             bracket's own half-em space — a boundary this line does not own"
        );
        assert_eq!(
            geometry.extent,
            InlineExtent::new(1000).expect("a valid extent"),
            "the buggy reading computes 1500 for this fixture — the ideograph's own 1000 \
             units plus 500 units of gap the previous line's own closing bracket has no \
             business contributing to this line's geometry"
        );
    }

    /// Regression for the extent-vs-hanging bug this pass's own verification found:
    /// `Line::extent()` included the amount [`crate::ladder::hang`] let the line's own last
    /// item hang past the measure, contradicting `docs/adr/0017-normalized-line-geometry.md`'s
    /// own definition in full: "excluding any character placed outside the measure by
    /// §2.5.1's hanging punctuation." `apply_adjustment` only ever realizes `reduce`'s and
    /// `expand`'s ladder deltas — hanging is never a [`crate::ladder::Site`] — so nothing
    /// subtracted the hung amount from the extent it folded into.
    ///
    /// No case in the existing corpus exercises this: `Policy::JLREQ` reads
    /// `Question::HANGING_PUNCTUATION` as `"none"`, so this test builds its own policy with
    /// the alternative §3.8.2 names turned on.
    ///
    /// The fixture: "亜。" (an ideograph, then a full stop) at `measure = 1300`. The
    /// interior cl-19×cl-06 boundary carries no conditional space, so the line's natural
    /// extent is just the two items' own advances (1000 + 500) plus the half-em trailing
    /// space §B.2 note 6 adds after a line-ending full stop (500) — 2000 in all, 700 over
    /// measure. That trailing space is `Reduction::Discrete { floor: 0, stage: 2 }`
    /// (Table 3's own `before: 6, after: 0` cell), so `reduce` closes 500 of the 700-unit
    /// shortfall by flipping it to zero, leaving 200 still owed — exactly what `hang` then
    /// credits to the full stop, the line's own last item and one of the two classes
    /// §2.5.1 permits to hang.
    #[test]
    fn a_hung_characters_overhang_is_excluded_from_the_lines_own_extent() {
        let hanging = Question::HANGING_PUNCTUATION
            .permits()
            .iter()
            .copied()
            .find(|choice| choice.name() == "hanging")
            .expect("§3.8.2 names this alternative");
        let policy = Policy::JLREQ
            .with(hanging)
            .expect("hanging punctuation conflicts with nothing else JLREQ selects");

        let items = [
            Item::new(
                ByteOffset::new(0),
                InlineExtent::new(1000).expect("a valid extent"),
                ScaleId::new(0),
            )
            .with_frame(Frame::FullEm),
            Item::new(
                ByteOffset::new(3),
                InlineExtent::new(500).expect("a valid extent"),
                ScaleId::new(0),
            )
            .with_frame(Frame::HalfEm),
        ];
        let scales = [scale(1000)];
        let text = Text::new("亜。", &items, &scales).expect("a well formed stream");
        let measure = InlineExtent::new(1300).expect("a valid extent");
        let paragraph = Paragraph::new(text, &[], measure, Direction::Horizontal);

        let composition =
            compose(paragraph, policy, Search::FirstFit).expect("a well formed input");

        assert!(
            composition.violations().is_empty(),
            "reduction closes 500 of the 700-unit shortfall and hanging closes the rest: \
             {:?}",
            composition.violations()
        );
        let lines = composition.lines();
        assert_eq!(
            lines.len(),
            1,
            "the fixture is built to force exactly one line"
        );

        let line = &lines[0];
        assert_eq!(
            line.hanging().map(|hanging| hanging.beyond),
            Some(InlineExtent::new(200).expect("a valid extent")),
            "reduction alone (500) does not close the full 700-unit shortfall"
        );
        assert_eq!(
            line.extent(),
            measure,
            "the buggy reading reports 1500 — natural_extent(2000) minus reduce's own 500, \
             never subtracting the 200 units hanging separately accounted for — which is \
             an extent exceeding the very measure `Composition::is_conforming` just \
             reported this line as fitting"
        );
    }

    /// The behavioral proof this whole round exists for: an underfull line of ordinary
    /// kanji actually expands, which it structurally could not before ADR-0021 moved
    /// Table 6's opportunity off `jlreq_spacing::ConditionalSpace` and onto
    /// `jlreq_spacing::Boundary` — a solid Table 1 cell produced no `Site` at all for
    /// `ladder::expand` to drain, on any text, regardless of what Table 6 stated there.
    ///
    /// The fixture: "亜亜亜", three ideographs (cl-19), split by a candidate at byte 6 into
    /// a first line of two ("亜亜", items 0 and 1) and a second of one ("亜", item 2) — the
    /// second is the paragraph's own last line, `is_last_line`, so it never calls
    /// `ladder::expand` at all (§3.8.1's own Note) and stays at its own natural 1000-unit
    /// extent regardless of `measure`, which both cases below assert as a control. The
    /// first line is the one under test: `Table 1`'s own cl-19-against-cl-19 cell is blank
    /// (`spec/captured/table1.en.tsv`), so its natural extent is exactly the two items' own
    /// advances with no gap between them, 2000 units, but `Table 6`'s own cell there is
    /// `0-1/4 stage 3` (`spec/captured/table6.en.tsv`) — a quarter em ceiling (250 units at
    /// each item's own 1000-unit em) at the third priority stage.
    #[test]
    fn an_underfull_line_of_ordinary_kanji_actually_expands() {
        let items = [
            Item::new(
                ByteOffset::new(0),
                InlineExtent::new(1000).expect("a valid extent"),
                ScaleId::new(0),
            )
            .with_frame(Frame::FullEm),
            Item::new(
                ByteOffset::new(3),
                InlineExtent::new(1000).expect("a valid extent"),
                ScaleId::new(0),
            )
            .with_frame(Frame::FullEm),
            Item::new(
                ByteOffset::new(6),
                InlineExtent::new(1000).expect("a valid extent"),
                ScaleId::new(0),
            )
            .with_frame(Frame::FullEm),
        ];
        let scales = [scale(1000)];
        let text = Text::new("亜亜亜", &items, &scales).expect("a well formed stream");
        let candidates = [Candidate::At(ByteOffset::new(6))];

        // Case one: the shortfall (250 units) exactly matches the site's own third-stage
        // ceiling, so the ordinary stage alone closes it and re-leveling never runs.
        let measure = InlineExtent::new(2250).expect("a valid extent");
        let paragraph = Paragraph::new(text, &candidates, measure, Direction::Horizontal);
        let composition =
            compose(paragraph, Policy::JLREQ, Search::FirstFit).expect("a well formed input");
        assert!(
            composition.violations().is_empty(),
            "the site's own quarter-em ceiling exactly covers a 250-unit shortfall: {:?}",
            composition.violations()
        );
        let lines = composition.lines();
        assert_eq!(
            lines.len(),
            2,
            "the fixture is built to force exactly two lines"
        );
        assert_eq!(
            lines[0].placements(),
            &[offset(0), offset(1250)],
            "the second ideograph shifts by the full 250-unit expansion — nothing here \
             before this round, since the boundary between two blank-Table-1 ideographs \
             produced no `Site` for `ladder::expand` to drain at all"
        );
        assert_eq!(lines[0].extent(), measure);
        assert!(
            !lines[0].adjustment().releveled(),
            "the ordinary third stage alone supplies the full 250 units; nothing is left \
             over for the fourth stage's own re-leveling to add"
        );
        assert_eq!(
            lines[1].extent(),
            InlineExtent::new(1000).expect("a valid extent"),
            "the last line takes no expansion at all (§3.8.1's own Note) and stays at its \
             own natural extent regardless of `measure`"
        );

        // Case two: the shortfall (400 units) exceeds the site's own 250-unit ceiling, so
        // the ordinary third stage drains it to that ceiling and the fourth step's own
        // re-leveling — "evenly add space to equalize the spacing of 1st, 2nd, 3rd and 4th
        // steps", §E.1 — supplies the remaining 150 with no ceiling of its own to stop at.
        let measure = InlineExtent::new(2400).expect("a valid extent");
        let paragraph = Paragraph::new(text, &candidates, measure, Direction::Horizontal);
        let composition =
            compose(paragraph, Policy::JLREQ, Search::FirstFit).expect("a well formed input");
        assert!(
            composition.violations().is_empty(),
            "the fourth stage's own unbounded re-leveling closes what the third stage's \
             own ceiling alone could not: {:?}",
            composition.violations()
        );
        let lines = composition.lines();
        assert_eq!(
            lines.len(),
            2,
            "the fixture is built to force exactly two lines"
        );
        assert_eq!(
            lines[0].placements(),
            &[offset(0), offset(1400)],
            "250 units at the third stage's own ceiling, then 150 more the fourth stage's \
             own re-leveling adds past it — a site expanded further than its own captured \
             ceiling, which only the fourth step permits"
        );
        assert_eq!(lines[0].extent(), measure);
        assert!(
            lines[0].adjustment().releveled(),
            "the third stage's own 250-unit ceiling falls 150 units short of the 400-unit \
             shortfall, so the fourth stage's own re-leveling must have run"
        );
        assert_eq!(
            lines[1].extent(),
            InlineExtent::new(1000).expect("a valid extent"),
            "the last line takes no expansion at all (§3.8.1's own Note) and stays at its \
             own natural extent regardless of `measure`"
        );
    }

    /// Regression for this round's own §3.6 tab-setting work: `geometry_of`'s `edges`
    /// parameter must be able to suppress §B.2's own line-end rules for a range whose last
    /// item does not actually end a printed line — the failure mode `crate::tab::tab_line`
    /// would reproduce at every interior tab run if `geometry_of` still assumed both of its
    /// own edges were always genuine, the same way it once wrongly assumed every range's
    /// own first item was
    /// (`geometry_of_reads_a_lines_own_head_as_the_line_head_not_the_previous_lines_close`,
    /// above).
    ///
    /// The fixture: "亜。" (an ideograph, then a full stop) — the same fixture
    /// `a_hung_characters_overhang_is_excluded_from_the_lines_own_extent` uses, under the
    /// same default `Policy::JLREQ`, called directly rather than through `compose` so no
    /// reduction or expansion ever runs and the two edge treatments are compared on the
    /// otherwise-identical natural geometry alone. §B.2 note 6 adds a half-em (500-unit)
    /// trailing space after a line-ending full stop; the two items' own advances alone sum
    /// to 1000 + 500 = 1500.
    #[test]
    fn edges_end_false_suppresses_the_line_end_trailing_space() {
        let items = [
            Item::new(
                ByteOffset::new(0),
                InlineExtent::new(1000).expect("a valid extent"),
                ScaleId::new(0),
            )
            .with_frame(Frame::FullEm),
            Item::new(
                ByteOffset::new(3),
                InlineExtent::new(500).expect("a valid extent"),
                ScaleId::new(0),
            )
            .with_frame(Frame::HalfEm),
        ];
        let scales = [scale(1000)];
        let text = Text::new("亜。", &items, &scales).expect("a well formed stream");
        let range = ItemIndex::new(0)..ItemIndex::new(2);

        let interior = geometry_of(
            text,
            Runs::none(),
            range.clone(),
            InlineExtent::ZERO,
            Direction::Horizontal,
            Policy::JLREQ,
            Edges::new(true, false),
        );
        assert_eq!(
            interior.extent,
            InlineExtent::new(1500).expect("a valid extent"),
            "the buggy reading applies §B.2 note 6's trailing half em regardless of \
             `edges.end` and reports 2000 for this fixture — a tab-separated run's own \
             line-end space it never earned, because more text follows it across the tab \
             rather than a genuine line end"
        );
        assert_eq!(
            interior.trailing,
            InlineExtent::ZERO,
            "no trailing space is realized at all when this edge is not genuine"
        );

        let genuine = geometry_of(
            text,
            Runs::none(),
            range,
            InlineExtent::ZERO,
            Direction::Horizontal,
            Policy::JLREQ,
            Edges::BOTH,
        );
        assert_eq!(
            genuine.extent,
            InlineExtent::new(2000).expect("a valid extent"),
            "a genuine line end still gets §B.2 note 6's own trailing half em; `Edges::BOTH` \
             must not have lost the behavior `Edges::new(true, false)` above deliberately \
             suppresses"
        );
        assert_eq!(
            genuine.trailing,
            InlineExtent::new(500).expect("a valid extent"),
            "the realized trailing space itself, unsuppressed"
        );
    }

    /// A multi-line paragraph whose optimum is verifiable by hand: four one-em ideographs
    /// at `measure = 2000`. A 2-item line's own natural extent (2000) matches `measure`
    /// exactly, so the 2+2 split needs no reduction, no expansion and no hanging on either
    /// line — `Demerits::ZERO` overall. Every other split is strictly worse: a 1-item
    /// non-last line has no interior boundary at all to expand from
    /// (`ladder::expand`'s own union is empty), so it is `ExpansionExhausted`; a 3-item line
    /// overflows `measure` by 1000 units with no reducible boundary to close it (cl-19
    /// against cl-19 is blank in Table 1), so it is `Overfull`. Both are `Badness::WORST`
    /// under this milestone's own zero-flex reading (`Search::Optimal`'s own doc), so the
    /// 2+2 split is the unique minimum under either `Preference` permutation — this test
    /// does not need to pick one.
    #[test]
    fn optimal_finds_the_hand_verified_two_two_split() {
        let items = [kanji(0), kanji(3), kanji(6), kanji(9)];
        let scales = [scale(1000)];
        let text = Text::new("亜亜亜亜", &items, &scales).expect("a well formed stream");
        let candidates = [
            Candidate::At(ByteOffset::new(3)),
            Candidate::At(ByteOffset::new(6)),
            Candidate::At(ByteOffset::new(9)),
        ];
        let measure = InlineExtent::new(2000).expect("a valid extent");
        let paragraph = Paragraph::new(text, &candidates, measure, Direction::Horizontal);

        let composition = compose(
            paragraph,
            Policy::JLREQ,
            Search::Optimal {
                tolerance: Badness::WORST,
            },
        )
        .expect("a well formed input");

        assert!(
            composition.violations().is_empty(),
            "the 2+2 split fits both lines exactly: {:?}",
            composition.violations()
        );
        assert_eq!(
            composition.demerits(),
            Demerits::ZERO,
            "neither line needs any reduction, expansion or hanging"
        );
        let lines = composition.lines();
        assert_eq!(
            lines.len(),
            2,
            "the fixture is built to force exactly two lines"
        );
        assert_eq!(
            lines[0].items(),
            ItemIndex::new(0)..ItemIndex::new(2),
            "the search finds the 2+2 split, not the 1+3 or 3+1 splits every other candidate \
             boundary would force a violation on"
        );
        assert_eq!(lines[1].items(), ItemIndex::new(2)..ItemIndex::new(4));
    }

    /// The two `Preference` permutations disagree over the same paragraph, because they
    /// disagree over where `badness` sits relative to `expansion_depth`: three one-em
    /// ideographs at `measure = 2250`, candidates after the first and the second.
    ///
    /// One complete arrangement is the whole text as a single line (first and last at
    /// once): natural extent 3000, 750 over `measure`, no reducible boundary to close it
    /// (blank Table 1 cells) — `Overfull(750)`, `Badness::WORST`, and *no* ladder ever
    /// engaged (`expansion_depth = 0`, `reduction_depth = 0`), because an already-overfull
    /// line never reaches `ladder::expand` at all.
    ///
    /// The other is the two-line split this fixture's own sibling test
    /// (`even_texture_prefers_the_feasible_arrangement`) also uses: a 2-item first line
    /// (extent 2000, 250 short of `measure`) closed exactly by Table 6's own third-stage
    /// quarter-em ceiling on the one interior cl-19×cl-19 boundary (`expansion_depth = 3`,
    /// `badness = 0`), followed by a 1-item last line, short but exempt (§3.8.1's Note,
    /// `badness = 0`).
    ///
    /// `least-adjustment` ranks `expansion_depth` ahead of `badness`
    /// (`docs/decisions/adjustment-preference.md`), so it prefers the single, *violating*
    /// line (`expansion_depth = 0`) over the two-line, fully expanded arrangement
    /// (`expansion_depth = 3`) — `tolerance: Badness::WORST` is required for this test
    /// precisely because a stricter tolerance would discard the violating candidate before
    /// `Preference::compare` ever got to choose between the two.
    ///
    /// Also a second, directly run instance of the round's own required experiment
    /// (`firstfit_and_optimal_disagree_about_whether_a_trailing_full_stop_hangs` is the
    /// first): `Search::FirstFit` never runs `Preference::compare` at all, so it cannot face
    /// this choice — it greedily takes the last candidate whose *unadjusted* extent still
    /// fits `measure` (the 2-item candidate, 2000 units, under 2250), which is exactly the
    /// two-line split's own first line, and `ladder::expand` then closes its own 250-unit
    /// residual at Table 6's own third stage alone, with nothing left to re-level.
    /// `FirstFit` consequently reaches the two-line, fully expanded arrangement
    /// `least-adjustment` declines below, over the identical text and measure — asserted
    /// directly here rather than left as hand-derived prose, because "do not treat a passing
    /// test as evidence" cuts against an untested claim exactly as hard as it cuts against
    /// an unexamined one.
    #[test]
    fn least_adjustment_prefers_the_shallow_but_overfull_arrangement() {
        let items = [kanji(0), kanji(3), kanji(6)];
        let scales = [scale(1000)];
        let text = Text::new("亜亜亜", &items, &scales).expect("a well formed stream");
        let candidates = [
            Candidate::At(ByteOffset::new(3)),
            Candidate::At(ByteOffset::new(6)),
        ];
        let measure = InlineExtent::new(2250).expect("a valid extent");
        let paragraph = Paragraph::new(text, &candidates, measure, Direction::Horizontal);

        let first_fit =
            compose(paragraph, Policy::JLREQ, Search::FirstFit).expect("a well formed input");
        assert!(
            first_fit.violations().is_empty(),
            "FirstFit greedily takes the 2-item candidate (2000 units, under measure) and \
             closes the remaining 250-unit residual at Table 6's own third stage alone: {:?}",
            first_fit.violations()
        );
        assert_eq!(
            first_fit.lines().len(),
            2,
            "FirstFit reaches the two-line, fully expanded arrangement least-adjustment \
             declines below — blind to `Preference` entirely, it cannot prefer the single \
             overfull line the way `Search::Optimal` does over the identical text and measure"
        );

        let composition = compose(
            paragraph,
            Policy::JLREQ,
            Search::Optimal {
                tolerance: Badness::WORST,
            },
        )
        .expect("a well formed input");

        assert_eq!(
            composition.lines().len(),
            1,
            "least-adjustment (Policy::JLREQ's own default) ranks expansion_depth ahead of \
             badness, so it prefers the single overfull line over the two-line, fully \
             expanded arrangement `even_texture_prefers_the_feasible_arrangement` finds over \
             the identical text — the same two-line arrangement `first_fit` above just \
             reached over the identical paragraph"
        );
        assert_eq!(
            composition.violations().len(),
            1,
            "the single line this preference chooses is the one this fixture's own doc \
             names Overfull(750)"
        );
        assert!(matches!(
            composition.violations()[0].kind,
            ViolationKind::Overfull(_)
        ));
    }

    /// `even-texture`'s own reading of the identical paragraph
    /// `least_adjustment_prefers_the_shallow_but_overfull_arrangement` composes: ranking
    /// `badness` ahead of `expansion_depth` makes the fully feasible two-line arrangement
    /// (`badness = 0`) strictly better than the single overfull line (`badness =
    /// Badness::WORST`), regardless of how much expansion the feasible arrangement needed.
    #[test]
    fn even_texture_prefers_the_feasible_arrangement() {
        let items = [kanji(0), kanji(3), kanji(6)];
        let scales = [scale(1000)];
        let text = Text::new("亜亜亜", &items, &scales).expect("a well formed stream");
        let candidates = [
            Candidate::At(ByteOffset::new(3)),
            Candidate::At(ByteOffset::new(6)),
        ];
        let measure = InlineExtent::new(2250).expect("a valid extent");
        let even_texture = Question::ADJUSTMENT_PREFERENCE
            .permits()
            .iter()
            .copied()
            .find(|choice| choice.name() == "even-texture")
            .expect("§C.3 names this alternative");
        let policy = Policy::JLREQ
            .with(even_texture)
            .expect("adjustment preference conflicts with nothing else JLREQ selects");
        let paragraph = Paragraph::new(text, &candidates, measure, Direction::Horizontal);

        let composition = compose(
            paragraph,
            policy,
            Search::Optimal {
                tolerance: Badness::WORST,
            },
        )
        .expect("a well formed input");

        assert!(
            composition.violations().is_empty(),
            "even-texture ranks badness ahead of expansion_depth, so it prefers the fully \
             feasible two-line arrangement over the single overfull line \
             least_adjustment_prefers_the_shallow_but_overfull_arrangement finds over the \
             identical text: {:?}",
            composition.violations()
        );
        assert_eq!(composition.lines().len(), 2);
    }

    /// C4's tolerance-exhaustion fallback (`docs/decisions/tolerance-exhaustion.md`): one
    /// ideograph at `measure = 500`, half its own 1000-unit advance. The only possible line
    /// is unavoidably `Overfull(500)` — no reducible boundary exists to close it — so
    /// `tolerance: Badness::ZERO` leaves the tolerance-respecting pass with no complete path
    /// at all. ADR-0010 forbids both a panic and inventing a break kinsoku does not permit,
    /// so `compose` still emits the one line there is, with the violation named, exactly as
    /// `tolerance: Badness::WORST` would have accepted directly — the fallback never does
    /// worse than the neutral tolerance would.
    #[test]
    fn tolerance_exhaustion_falls_back_to_the_full_search() {
        let items = [kanji(0)];
        let scales = [scale(1000)];
        let text = Text::new("亜", &items, &scales).expect("a well formed stream");
        let measure = InlineExtent::new(500).expect("a valid extent");
        let paragraph = Paragraph::new(text, &[], measure, Direction::Horizontal);

        let composition = compose(
            paragraph,
            Policy::JLREQ,
            Search::Optimal {
                tolerance: Badness::ZERO,
            },
        )
        .expect("a well formed input");

        assert_eq!(
            composition.lines().len(),
            1,
            "ADR-0010: composition never refuses to produce lines, even when no arrangement \
             stays within so strict a tolerance"
        );
        assert_eq!(composition.violations().len(), 1);
        assert!(matches!(
            composition.violations()[0].kind,
            ViolationKind::Overfull(_)
        ));
    }

    /// C3: `first_line` is read from the edge (`start.get() == 0`), never from where the DP's
    /// own reconstruction loop happens to be. Four one-em ideographs, `first_line_indent =
    /// 2000`, `measure = 3000`: the first line's own extent already includes the indent
    /// (`geometry_of`'s cursor starts advanced by it), so only a single ideograph
    /// (`2000 + 1000 = 3000`, exact) fits the first line at all — a second would overflow to
    /// 4000. The remaining three ideographs, indent-free because they do not open the
    /// paragraph, sum to exactly 3000 too, and this line is also the paragraph's last, so
    /// §3.8.1's Note would have excused it from expansion even had it fallen short.
    ///
    /// A version of `evaluate_edge` that inherited `first_line` from loop history instead of
    /// `range.start` — for instance, one that read it off "is this the first edge
    /// `compose_optimal`'s own reconstruction loop visits" — would apply the indent to
    /// *every* line it evaluates while scanning outward from item zero during `run_dp`, not
    /// only the line that starts there; every non-first candidate this fixture offers would
    /// then read as overfull by 2000 units it never earned, and the DP would report no
    /// feasible arrangement at all where this test asserts one exists cleanly.
    #[test]
    fn first_line_indent_is_read_from_the_edge_not_loop_position() {
        let items = [kanji(0), kanji(3), kanji(6), kanji(9)];
        let scales = [scale(1000)];
        let text = Text::new("亜亜亜亜", &items, &scales).expect("a well formed stream");
        let candidates = [
            Candidate::At(ByteOffset::new(3)),
            Candidate::At(ByteOffset::new(6)),
            Candidate::At(ByteOffset::new(9)),
        ];
        let measure = InlineExtent::new(3000).expect("a valid extent");
        let paragraph = Paragraph::new(text, &candidates, measure, Direction::Horizontal)
            .with_first_line_indent(InlineExtent::new(2000).expect("a valid extent"));

        let composition = compose(
            paragraph,
            Policy::JLREQ,
            Search::Optimal {
                tolerance: Badness::WORST,
            },
        )
        .expect("a well formed input");

        assert!(
            composition.violations().is_empty(),
            "both lines reach exactly `measure`: {:?}",
            composition.violations()
        );
        assert_eq!(composition.demerits(), Demerits::ZERO);
        let lines = composition.lines();
        assert_eq!(
            lines.len(),
            2,
            "the fixture is built to force exactly two lines"
        );
        assert_eq!(
            lines[0].items(),
            ItemIndex::new(0)..ItemIndex::new(1),
            "only one ideograph fits the indented first line"
        );
        assert_eq!(lines[1].items(), ItemIndex::new(1)..ItemIndex::new(4));
        assert!(lines[1].is_last());
    }

    /// The experiment this round exists to run: an actively constructed paragraph on which
    /// `FirstFit` and `Optimal` disagree, including about when a character hangs —
    /// falsifying `docs/design/api-spine.md`'s former "which is why the two searches agree"
    /// (on `Search::FirstFit`'s own doc), its former "they cannot disagree about when a
    /// character hangs" (a second, independent copy of the same stale claim on `Ladder`'s
    /// own doc), and `ROADMAP.md`'s former "cannot disagree about when a character hangs"
    /// (all three repaired alongside this test).
    ///
    /// Text "亜亜。" (two ideographs, one full stop), `measure = 2300`, with hanging
    /// punctuation enabled (`Question::HANGING_PUNCTUATION = "hanging"`, not
    /// `Policy::JLREQ`'s own default).
    ///
    /// `FirstFit` reads only *unadjusted* geometry: it takes the last candidate whose
    /// natural extent still fits `measure` (two ideographs, 2000 units), never asking what
    /// the *next* line would cost. That leaves the full stop alone on its own last line —
    /// short, but exempt (§3.8.1's Note) — so no line of `FirstFit`'s own answer is ever
    /// overfull, and hanging never applies to anything: `Line::hanging()` is `None` on both
    /// lines.
    ///
    /// `Optimal` compares whole arrangements. Composing all three items as a single line
    /// (both first and last) overflows by 700 units; reduction alone can close 500 of it —
    /// the trailing half em §B.2 note 6 adds after a line-ending full stop is
    /// `Reduction::Discrete { floor: 0, stage: 2 }`, so it flips fully to zero — and hanging
    /// closes the remaining 200 against the full stop's own 500-unit advance, landing on a
    /// feasible, `Badness::ZERO` single line with `hanging: Some(_)`. Splitting into two
    /// lines instead — two ideographs, then the full stop alone — is also feasible, but its
    /// own first line needs Table 6's third stage plus the unbounded fourth-stage
    /// re-leveling to close its own 300-unit residual shortfall
    /// (`Demerits::last_resort = 1`). `least-adjustment` (`Policy::JLREQ`'s own default)
    /// ranks `last_resort` ahead of every other free component, so the single, hanging line
    /// — `last_resort = 0` — wins outright; `tolerance: Badness::ZERO` is enough to prove
    /// it, because both arrangements are already feasible and the comparison never needs a
    /// tolerance-driven prohibition to decide it.
    #[test]
    fn firstfit_and_optimal_disagree_about_whether_a_trailing_full_stop_hangs() {
        let hanging = Question::HANGING_PUNCTUATION
            .permits()
            .iter()
            .copied()
            .find(|choice| choice.name() == "hanging")
            .expect("§3.8.2 names this alternative");
        let policy = Policy::JLREQ
            .with(hanging)
            .expect("hanging punctuation conflicts with nothing else JLREQ selects");

        let items = [
            kanji(0),
            kanji(3),
            Item::new(
                ByteOffset::new(6),
                InlineExtent::new(500).expect("a valid extent"),
                ScaleId::new(0),
            )
            .with_frame(Frame::HalfEm),
        ];
        let scales = [scale(1000)];
        let text = Text::new("亜亜。", &items, &scales).expect("a well formed stream");
        let candidates = [
            Candidate::At(ByteOffset::new(3)),
            Candidate::At(ByteOffset::new(6)),
        ];
        let measure = InlineExtent::new(2300).expect("a valid extent");
        let paragraph = Paragraph::new(text, &candidates, measure, Direction::Horizontal);

        let first_fit = compose(paragraph, policy, Search::FirstFit).expect("a well formed input");
        let optimal = compose(
            paragraph,
            policy,
            Search::Optimal {
                tolerance: Badness::ZERO,
            },
        )
        .expect("a well formed input");

        assert_eq!(
            first_fit.lines().len(),
            2,
            "FirstFit never looks past its own chosen candidate's unadjusted fit"
        );
        assert!(
            first_fit
                .lines()
                .iter()
                .all(|line| line.hanging().is_none()),
            "the full stop sits alone on FirstFit's own short, exempt last line — nothing \
             here is ever overfull, so `ladder::hang` is never even reached: {:?}",
            first_fit
                .lines()
                .iter()
                .map(super::Line::hanging)
                .collect::<alloc::vec::Vec<_>>()
        );

        assert!(
            optimal.violations().is_empty(),
            "the single-line arrangement is fully feasible once reduction and hanging both \
             close their own share of the 700-unit overflow: {:?}",
            optimal.violations()
        );
        assert_eq!(
            optimal.lines().len(),
            1,
            "least-adjustment prefers the single line (last_resort 0) over the two-line \
             split (last_resort 1, from the first line's own re-leveling)"
        );
        assert!(
            optimal.lines()[0].hanging().is_some(),
            "the full stop hangs under Optimal precisely because Optimal chose to keep it on \
             the same line as both ideographs, where FirstFit never put it"
        );
    }

    /// `pull_up_of`'s own two conditions, tested directly rather than through a full
    /// composition: a shorter, evaluated alternative must have existed (`end_index` is not
    /// `start_index + 1`), *and* real reduction must have actually closed the gap the
    /// longer choice needed it for (`reduction_depth > 0`) — either alone is not enough, and
    /// only together do they report the pull-up (追い込み) §3.1.12 ⑤ names.
    #[test]
    fn pull_up_of_reports_only_when_a_shorter_alternative_existed_and_reduction_reclaimed_the_gap()
    {
        let deltas = alloc::vec![
            InlineExtent::ZERO,
            InlineExtent::new(-300).expect("a valid extent"),
        ];
        let adjustment = super::Adjustment::of(deltas, false);
        let nodes = [
            ItemIndex::new(0),
            ItemIndex::new(2),
            ItemIndex::new(3),
            ItemIndex::new(5),
        ];

        assert_eq!(
            super::pull_up_of(&adjustment, 3, &nodes, 0, 1),
            None,
            "end_index (1) is start_index (0) plus one: no shorter alternative was ever \
             scanned to compare against, so nothing was pulled up regardless of how much \
             reduction ran"
        );
        assert_eq!(
            super::pull_up_of(&adjustment, 0, &nodes, 0, 2),
            None,
            "a shorter alternative existed (`nodes[1]`), but reduction_depth is zero: \
             nothing was actually reclaimed to prefer the longer choice for"
        );

        let pull_up = super::pull_up_of(&adjustment, 3, &nodes, 0, 2)
            .expect("both conditions hold: a shorter alternative existed and reduction ran");
        assert_eq!(
            pull_up.amount,
            InlineExtent::new(300).expect("a valid extent"),
            "the realized reduction magnitude, read off `Adjustment::reduced`"
        );
        assert_eq!(
            pull_up.pulls, nodes[1],
            "the shorter alternative's own boundary — the item that would have opened the \
             next line under it"
        );
        assert_eq!(pull_up.rule, RuleId::EXAMPLES_OF_LINE_ADJUSTMENT);
    }

    /// The regression guard for all 834 existing tests and all 467 existing conformance
    /// cases (`docs/decisions/widow-threshold.md`'s own "no-op by construction, not by a
    /// special case"): `demerits_of`'s own shortfall is
    /// `u32::from(threshold).saturating_sub(count)`, which is `0` for any `count` once
    /// `threshold` is `0`, so a `Paragraph` that calls `with_widow_threshold(0)` composes
    /// byte-identically, under either search, to one that never called it at all — not
    /// merely "close", checked line by line and demerit by demerit.
    #[test]
    fn a_zero_widow_threshold_changes_nothing_for_either_search() {
        let items = [kanji(0), kanji(3), kanji(6), kanji(9)];
        let scales = [scale(1000)];
        let text = Text::new("亜亜亜亜", &items, &scales).expect("a well formed stream");
        let candidates = [
            Candidate::At(ByteOffset::new(3)),
            Candidate::At(ByteOffset::new(6)),
            Candidate::At(ByteOffset::new(9)),
        ];
        let measure = InlineExtent::new(2000).expect("a valid extent");
        let plain = Paragraph::new(text, &candidates, measure, Direction::Horizontal);
        let zeroed = plain.with_widow_threshold(0);

        for search in [
            Search::FirstFit,
            Search::Optimal {
                tolerance: Badness::WORST,
            },
        ] {
            let a = compose(plain, Policy::JLREQ, search).expect("a well formed input");
            let b = compose(zeroed, Policy::JLREQ, search).expect("a well formed input");

            assert_eq!(
                a.demerits(),
                b.demerits(),
                "threshold 0 under {search:?}: structural stays zero, nothing else moves"
            );
            assert_eq!(a.violations().len(), b.violations().len());
            assert_eq!(a.lines().len(), b.lines().len());
            for (line_a, line_b) in a.lines().iter().zip(b.lines()) {
                assert_eq!(line_a.items(), line_b.items());
                assert_eq!(line_a.demerits(), line_b.demerits());
            }
        }
    }

    /// `Search::Optimal` steers away from a widow the way `docs/decisions/
    /// widow-threshold.md`'s own Q3 argues it must, under `Policy::JLREQ`'s own default
    /// `least-adjustment` ordering — not only under the `even-texture` alternative — which
    /// matters because `structural` ranking first in *both* of `docs/decisions/
    /// adjustment-preference.md`'s own orderings is the round's own load-bearing claim: an
    /// adopter who never touches `Question::ADJUSTMENT_PREFERENCE` gets this exact
    /// behavior. Four one-em ideographs, candidates after each, the same `measure = 2000`
    /// fixture `first_fit_reports_a_widow_but_never_moves_the_break_to_avoid_it` composes
    /// through the other search, so the two tests read as one fixture pinning both halves
    /// of the asymmetry rather than two a reader has to line up by hand.
    ///
    /// With no threshold, the 2+2 split is the *only* fully feasible arrangement this
    /// candidate set admits at this measure (every other split forces at least one line
    /// that is either a lone, non-last ideograph — no interior boundary to expand at all,
    /// `badness = Badness::WORST` — or a 3-or-4-item line, unreducibly `Overfull`) —
    /// checked directly, not merely asserted: `composition.demerits()` is `Demerits::ZERO`
    /// and no other split could be cheaper once one candidate already scores zero on every
    /// component.
    ///
    /// A threshold of 3 makes that same 2+2 split a widow (`have = 2`, short by one). The
    /// whole-paragraph single line (`have = 4`, satisfies the threshold, and cheaper still —
    /// one unreducibly `Overfull(2000)` line is `badness = 10_000`, half the 1+3 split's own
    /// total below) is never in this comparison at all: `run_dp`'s own C5 window (this
    /// file's own doc on `run_dp`) scans from `start = 0` in increasing boundary order and
    /// stops at the first `Overfull` result, the 3-item candidate, so the 4-item candidate is
    /// never costed — a search-space exclusion settled before this test's own comparison ever
    /// starts, not a conclusion this test draws. Among what the window does admit, the only
    /// reachable arrangement whose last line meets the threshold is the 1+3 split — whose
    /// first line is infeasible (a lone ideograph, `ExpansionExhausted`) and whose own last
    /// line is unreducibly `Overfull(1000)`, `badness = 20_000` in all, immensely worse
    /// than the 2+2 split on every ladder and badness component there is, and worse under
    /// `least-adjustment` specifically because that ordering ranks `expansion_depth` ahead
    /// of `badness` — the 1+3 split's own zero `expansion_depth` (a lone ideograph never
    /// reaches an expansion site to engage one) does not save it either. `structural` is
    /// compared before any of that — the DP takes the 1+3 split anyway, which is what proves
    /// `structural` dominates among the arrangements this window admits, not an unconditional
    /// guarantee over the full arrangement space (the excluded whole-paragraph line is
    /// `run_dp`'s own C5 to credit, not this comparison's).
    #[test]
    fn optimal_steers_toward_a_widow_free_last_line_even_when_every_other_component_is_worse() {
        let items = [kanji(0), kanji(3), kanji(6), kanji(9)];
        let scales = [scale(1000)];
        let text = Text::new("亜亜亜亜", &items, &scales).expect("a well formed stream");
        let candidates = [
            Candidate::At(ByteOffset::new(3)),
            Candidate::At(ByteOffset::new(6)),
            Candidate::At(ByteOffset::new(9)),
        ];
        let measure = InlineExtent::new(2000).expect("a valid extent");

        let unwidowed = Paragraph::new(text, &candidates, measure, Direction::Horizontal);
        let plain = compose(
            unwidowed,
            Policy::JLREQ,
            Search::Optimal {
                tolerance: Badness::WORST,
            },
        )
        .expect("a well formed input");
        assert!(
            plain.violations().is_empty(),
            "the 2+2 split is the only fully feasible arrangement this candidate set \
             admits: {:?}",
            plain.violations()
        );
        assert_eq!(
            plain.demerits(),
            Demerits::ZERO,
            "the 2+2 split needs no reduction, expansion or hanging at this measure — the \
             same fixture optimal_finds_the_hand_verified_two_two_split already confirms"
        );
        assert_eq!(plain.lines().len(), 2);
        assert_eq!(
            plain.lines()[0].items(),
            ItemIndex::new(0)..ItemIndex::new(2)
        );
        assert_eq!(
            plain.lines()[1].items(),
            ItemIndex::new(2)..ItemIndex::new(4)
        );

        let widowed = unwidowed.with_widow_threshold(3);
        let steered = compose(
            widowed,
            Policy::JLREQ,
            Search::Optimal {
                tolerance: Badness::WORST,
            },
        )
        .expect("a well formed input");

        assert_eq!(
            steered.lines().len(),
            2,
            "the search still produces two lines — it moved the break, not the line count"
        );
        assert_eq!(
            steered.lines()[0].items(),
            ItemIndex::new(0)..ItemIndex::new(1),
            "the break moved from after the second item to after the first, even though \
             this choice is worse on every ladder and badness component there is"
        );
        assert_eq!(
            steered.lines()[1].items(),
            ItemIndex::new(1)..ItemIndex::new(4),
            "the last line now carries exactly three items, meeting the threshold of 3"
        );
        assert_eq!(
            steered.demerits().structural,
            0,
            "the arrangement finally chosen satisfies the threshold exactly, so structural \
             contributes nothing despite having decided the search"
        );
        assert_eq!(
            steered.violations().len(),
            2,
            "both ladder violations survive (ExpansionExhausted on line 1, Overfull on line \
             2) and neither is a widow: the last line's own three items already meet the \
             threshold"
        );
        for violation in steered.violations() {
            assert_eq!(
                violation.rule,
                RuleId::POSSIBILITIES_FOR_LINE_BREAKING_BETWEEN_CHARACTERS,
                "neither violation is the widow rule — the threshold this search steered \
                 toward is exactly satisfied"
            );
        }
    }

    /// `Search::FirstFit`'s own asymmetry, pinned: its break choice never reads
    /// `widow_threshold` at all (`fits(geometry.extent, target)` is the only test it runs),
    /// so the identical greedy 2+2 split is chosen whether or not a threshold is set — only
    /// the reported violation differs. The fixture is the same 2+2 split
    /// `optimal_steers_toward_a_widow_free_last_line_even_when_every_other_component_is_worse`'s
    /// own no-threshold case reaches, at `measure = 2000` so `FirstFit`'s own greedy scan
    /// reaches it directly rather than through a search that compares arrangements.
    #[test]
    fn first_fit_reports_a_widow_but_never_moves_the_break_to_avoid_it() {
        let items = [kanji(0), kanji(3), kanji(6), kanji(9)];
        let scales = [scale(1000)];
        let text = Text::new("亜亜亜亜", &items, &scales).expect("a well formed stream");
        let candidates = [
            Candidate::At(ByteOffset::new(3)),
            Candidate::At(ByteOffset::new(6)),
            Candidate::At(ByteOffset::new(9)),
        ];
        let measure = InlineExtent::new(2000).expect("a valid extent");
        let plain = Paragraph::new(text, &candidates, measure, Direction::Horizontal);
        let widowed = plain.with_widow_threshold(3);

        let a = compose(plain, Policy::JLREQ, Search::FirstFit).expect("a well formed input");
        assert!(a.violations().is_empty());
        assert_eq!(a.lines().len(), 2);
        assert_eq!(a.lines()[0].items(), ItemIndex::new(0)..ItemIndex::new(2));
        assert_eq!(a.lines()[1].items(), ItemIndex::new(2)..ItemIndex::new(4));

        let b = compose(widowed, Policy::JLREQ, Search::FirstFit).expect("a well formed input");
        assert_eq!(
            b.lines().len(),
            a.lines().len(),
            "FirstFit commits to one candidate per line before the ladder ever runs; a \
             threshold it has no mechanism to act on cannot change how many lines result"
        );
        for (line_a, line_b) in a.lines().iter().zip(b.lines()) {
            assert_eq!(
                line_a.items(),
                line_b.items(),
                "the greedy break choice is identical with or without a threshold — \
                 FirstFit never runs the comparison that could move it"
            );
        }

        assert_eq!(
            b.violations().len(),
            1,
            "the last line's own two items fall one short of the threshold of three"
        );
        let violation = b.violations()[0];
        assert_eq!(violation.rule, RuleId::WIDOW_ADJUSTMENT_OF_PARAGRAPHS);
        assert_eq!(violation.line, 1, "the second, zero-indexed line");
        assert_eq!(
            violation.at,
            ItemIndex::new(2),
            "the last line's own start — the break that could have moved, not the \
             paragraph's own end"
        );
        assert!(matches!(
            violation.kind,
            ViolationKind::Widow { have: 2, want: 3 }
        ));
    }

    /// A last line can be both overfull and a widow at once, and both violations survive,
    /// in a fixed order: the ladder violation `push_widow_violation`'s own caller already
    /// pushed, first, then the widow second (both loops push in that order, unconditionally
    /// — this test is what keeps it true rather than assumed). Three ideographs, no
    /// declared candidates — one line by construction — at `measure = 2500`: unreducibly
    /// `Overfull(500)` (blank Table 1 between ideographs gives reduction nothing to close),
    /// and, at `with_widow_threshold(5)`, two short of the threshold as well. Round 22's own
    /// cases will assert a violation list; this pins the order they can rely on.
    #[test]
    fn a_last_line_reports_both_an_overfull_violation_and_a_widow_in_that_order() {
        let items = [kanji(0), kanji(3), kanji(6)];
        let scales = [scale(1000)];
        let text = Text::new("亜亜亜", &items, &scales).expect("a well formed stream");
        let measure = InlineExtent::new(2500).expect("a valid extent");
        let paragraph =
            Paragraph::new(text, &[], measure, Direction::Horizontal).with_widow_threshold(5);

        for search in [
            Search::FirstFit,
            Search::Optimal {
                tolerance: Badness::WORST,
            },
        ] {
            let composition =
                compose(paragraph, Policy::JLREQ, search).expect("a well formed input");
            assert_eq!(composition.lines().len(), 1, "under {search:?}");
            assert_eq!(
                composition.violations().len(),
                2,
                "one ladder violation and one widow, under {search:?}: {:?}",
                composition.violations()
            );
            assert!(
                matches!(
                    composition.violations()[0].kind,
                    ViolationKind::Overfull(over) if over == InlineExtent::new(500).expect("a valid extent")
                ),
                "the ladder violation is pushed first, under {search:?}: {:?}",
                composition.violations()[0]
            );
            assert!(
                matches!(
                    composition.violations()[1].kind,
                    ViolationKind::Widow { have: 3, want: 5 }
                ),
                "the widow is pushed second, under {search:?}: {:?}",
                composition.violations()[1]
            );
            assert_eq!(
                composition.violations()[1].rule,
                RuleId::WIDOW_ADJUSTMENT_OF_PARAGRAPHS,
                "under {search:?}"
            );
        }
    }

    /// Question 2 (`docs/decisions/widow-threshold.md`): a paragraph that occupies a single
    /// line can have a widow, read literally rather than exempted. Two ideographs, no
    /// declared candidates at all — there is by construction no earlier break either search
    /// could move, `Search::Optimal` included — composed against a threshold of five no
    /// arrangement of two items could ever reach. Pinned on both halves the exempting
    /// reading would instead leave silent: a `ViolationKind::Widow` naming the shortfall,
    /// and a nonzero `Demerits::structural` reporting the identical number
    /// (`u32::from(5).saturating_sub(2) == 3`, shortfall-proportional per Q3, never a flat
    /// `1`). A test built on the exempting reading would find neither.
    #[test]
    fn a_single_line_paragraph_reports_a_widow_the_literal_reading_requires() {
        let items = [kanji(0), kanji(3)];
        let scales = [scale(1000)];
        let text = Text::new("亜亜", &items, &scales).expect("a well formed stream");
        let measure = InlineExtent::new(3000).expect("a valid extent");
        let paragraph =
            Paragraph::new(text, &[], measure, Direction::Horizontal).with_widow_threshold(5);

        for search in [
            Search::FirstFit,
            Search::Optimal {
                tolerance: Badness::WORST,
            },
        ] {
            let composition =
                compose(paragraph, Policy::JLREQ, search).expect("a well formed input");
            assert_eq!(
                composition.lines().len(),
                1,
                "no candidate was declared at all, under {search:?}"
            );
            assert_eq!(
                composition.lines()[0].items(),
                ItemIndex::new(0)..ItemIndex::new(2)
            );
            assert_eq!(
                composition.demerits().structural,
                3,
                "the shortfall is proportional (5 - 2 = 3), not a flat penalty, under \
                 {search:?}"
            );
            let widow = composition
                .violations()
                .iter()
                .find(|violation| violation.rule == RuleId::WIDOW_ADJUSTMENT_OF_PARAGRAPHS)
                .unwrap_or_else(|| {
                    panic!(
                        "the exempting reading of Q2 would report none here, under {search:?}: \
                         {:?}",
                        composition.violations()
                    )
                });
            assert!(matches!(
                widow.kind,
                ViolationKind::Widow { have: 2, want: 5 }
            ));
        }
    }

    /// An unsatisfiable threshold still composes, still reports the evidence, and neither
    /// panics nor loops (ADR-0010: composition never refuses to produce lines). A single
    /// ideograph — fewer items than any threshold past one could ever satisfy — composed
    /// under both searches.
    #[test]
    fn an_unsatisfiable_threshold_still_composes_without_panicking_or_looping() {
        let items = [kanji(0)];
        let scales = [scale(1000)];
        let text = Text::new("亜", &items, &scales).expect("a well formed stream");
        let measure = InlineExtent::new(1000).expect("a valid extent");
        let paragraph =
            Paragraph::new(text, &[], measure, Direction::Horizontal).with_widow_threshold(10);

        for search in [
            Search::FirstFit,
            Search::Optimal {
                tolerance: Badness::WORST,
            },
        ] {
            let composition =
                compose(paragraph, Policy::JLREQ, search).expect("a well formed input");
            assert_eq!(composition.lines().len(), 1);
            assert_eq!(
                composition.demerits().structural,
                9,
                "10 - 1 = 9, under {search:?}"
            );
            assert_eq!(composition.violations().len(), 1, "under {search:?}");
            assert!(matches!(
                composition.violations()[0].kind,
                ViolationKind::Widow { have: 1, want: 10 }
            ));
        }
    }

    /// The zero-item paragraph (`compose::no_feasible_break`, `compose_first_fit`'s own
    /// loop condition) reports `NoFeasibleBreak` and nothing else: there is no last line at
    /// all for a widow check to run against, and this test is what keeps that true rather
    /// than assumed. `Text::new` accepts an empty stream (`crates/jlreq-class`'s own
    /// contract) precisely so `ordered_boundaries`'s own ADR-0018 fallback has an input to
    /// exercise.
    #[test]
    fn a_zero_item_paragraph_never_grows_a_widow_violation() {
        let items: [Item; 0] = [];
        let scales = [scale(1000)];
        let text = Text::new("", &items, &scales).expect("a well formed stream");
        let measure = InlineExtent::new(1000).expect("a valid extent");
        let paragraph =
            Paragraph::new(text, &[], measure, Direction::Horizontal).with_widow_threshold(5);

        for search in [
            Search::FirstFit,
            Search::Optimal {
                tolerance: Badness::WORST,
            },
        ] {
            let composition =
                compose(paragraph, Policy::JLREQ, search).expect("a well formed input");
            assert!(composition.lines().is_empty(), "under {search:?}");
            assert_eq!(
                composition.violations().len(),
                1,
                "NoFeasibleBreak alone, never also a widow with nothing to report have/want \
                 for, under {search:?}"
            );
            assert!(matches!(
                composition.violations()[0].kind,
                ViolationKind::NoFeasibleBreak
            ));
        }
    }
}
