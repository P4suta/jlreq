// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The adjustment ladder: [`Ladder`], [`Site`], and their record, [`Adjustment`].
//!
//! # Status
//!
//! `reduce`, `hang` and `expand` are real. Every line [`crate::compose`] produces now
//! drains this module rather than reporting [`Adjustment::empty`] unconditionally:
//!
//! - [`reduce`] drains reduction Tables 3 through 5 (whichever `Question::REDUCTION_TABLE`
//!   selects, already resolved into each [`Site`]'s own [`jlreq_spacing::Reduction`] by
//!   [`jlreq_spacing::boundary`]) in priority order, evenly *within* a stage as a fraction
//!   of each site's own em ([`jlreq_unit::distribute`], weighted by the referent's own
//!   character size). JLReq: §3.8.3, §D, §D.1
//! - [`hang`] is §3.8.2's ぶら下げ (burasage), between reduction and expansion: a line
//!   still overfull once reduction is exhausted, whose last item is a full stop (cl-06) or
//!   a comma (cl-07), may let that character sit past the measure instead of being
//!   reduced or expanded for, gated by `Question::HANGING_PUNCTUATION`. JLReq: §3.8.2, §2.5.1
//! - [`expand`] is §3.8.4's mirror procedure over Table 6's second and third stages, and
//!   the fourth, re-leveling stage `Adjustment::releveled` reports. JLReq: §3.8.4, §E, §E.1
//!   As of this round it genuinely reaches a *solid* boundary too — cl-19 against cl-19
//!   (kanji beside kanji) is `blank` in Table 1 but `0-1/4 stage 3` in Table 6, and until
//!   ADR-0021 moved Table 6's opportunity off `jlreq_spacing::ConditionalSpace` and onto
//!   `jlreq_spacing::Boundary` itself, a boundary with no conditional space could never
//!   produce a [`Site`] for `expand` to drain at all: the whole procedure was structurally
//!   unreachable on ordinary Japanese running text, and the fix is what makes the
//!   behavioral claim below actually observable rather than only true of the fixtures a
//!   test happened to pick.
//!
//! **Two things this module does not do, and one choice it makes where JLReq gives none,
//! all stated rather than silently absent or silently picked:**
//!
//! - Expansion's own *first* priority stage — §3.8.4 (a): Western word spaces (cl-26),
//!   "usually one third em, added equally... up to a maximum of a half em". §E's own
//!   preamble states this is *outside* Table 6 ("the tables are for the second and
//!   subsequent stages... assuming the first stage... is already done"), so no
//!   [`jlreq_spacing::ConditionalSpace`] ever carries it and no [`Site`] here can either.
//!   This is the same accounting Appendix D already gives reduction's own first stage
//!   (`jlreq_spacing::ReductionStage`'s own doc: "a `ReductionStage` this crate produces
//!   is always 2 through 6"), stated here for expansion's stage 1 for the same reason.
//! - `Question::JAPANESE_LATIN_EXPANSION_CEILING` (§3.8.4 (b): the quarter-em Japanese/
//!   Latin boundary may be read as expandable to a half em, a third em, or fixed and
//!   never expanded at all) is not read here, because [`jlreq_spacing::evaluate::boundary`]
//!   does not read it either — Table 6's captured cells answer this boundary the same way
//!   regardless of this question's value (confirmed against `crate::generated::table6`
//!   and `jlreq-spacing`'s own `spaces_of`/`cell_expansion`). Reading it here, over data
//!   that was never conditioned on it, would let this module disagree with the boundary
//!   this line's own geometry was built from — worse than the stated gap. The same holds
//!   for §E.1 item 655's own cross-table coupling ("if Table 5... is adopted... this
//!   quarter em space shall not be expanded"), which `evaluate::boundary` also answers
//!   the same way under every `Question::REDUCTION_TABLE` reading. Both are `jlreq-spacing`
//!   gaps, not `jlreq-line` ones, and are not patched here — doing so would re-derive
//!   Appendix E's own data rather than consume it (this crate's own discipline).
//! - The weight a solid boundary's own expansion [`Site`] is drained by. §3.8.3's and
//!   §3.8.4's own "in proportion to the character size" presupposes a referent whose em
//!   supplies that weight, and [`Site`]'s own doc states the fact plainly: a site built
//!   with `space: None` has no referent to pick one from — cl-19 against cl-19 is a Table 6
//!   opportunity with no `be` or `af` named anywhere in §E's own text, only a "place". This
//!   module resolves the site's weight to the *preceding* neighbor's own character size
//!   ([`crate::compose::boundary_spaces`]'s own choice, threaded through as [`Site::new`]'s
//!   `size` parameter), and that reading is exact whenever the two neighbors share one em —
//!   the ordinary case for a line of running Japanese text, but not one this round's own
//!   `xtask attest` invariant (`expansion-needs-no-referent`) establishes: it proves only
//!   that no captured coordinate needs both a two-referent Table 1 conditional space and an
//!   independent Table 6 opportunity at once, and says nothing about which classes occupy a
//!   term-free coordinate or what size the caller gives each occurrence there (`Size` is
//!   supplied per occurrence, ADR-0002, not a static property of a class). It is genuinely
//!   arbitrary only where the two neighbors' own declared sizes differ, and
//!   nothing in §3.8.3, §3.8.4 or Appendix E names which of the two a class-pair-level (not
//!   referent-level) opportunity should weigh against in that case. What would resolve it:
//!   a sentence of the specification, or a published errata, naming one — until then this
//!   is the one place this pass had to pick rather than read an answer, and it is named as
//!   a slot rather than adjudicated in `docs/decisions/`, because that file family is for
//!   readings this project *takes* of a genuine ambiguity, not an answer the specification
//!   withholds outright (see `docs/decisions/README.md`'s own header).
//!
//! `Question::EXPANSION_ORDER` ("jis" — follow §3.8.4's own numbered order — or
//! "implementation" — the order is left to the implementation) is not read either: every
//! preset [`jlreq_spec::Policy`] publishes selects "jis", and the stage-ordered procedure
//! below *is* §3.8.4's own numbered order, so it is the correct reading under "jis" and a
//! valid one under "implementation" (which grants freedom rather than forbidding this
//! order). No branch on this question changes what either reading requires.
//!
//! **A genuinely unresolvable question survives from before this pass, unchanged**:
//! §3.8.3's steps 4 through 6 say "reduced equally in proportion to the character size" in
//! English against 文字サイズ比で均等に in Japanese, but step 1 says "the same width
//! reduction is applied to all spaces on the target line at the same time" in English. This
//! module follows the Japanese (weighted by character size throughout, not a flat amount
//! per site), and the divergence remains a recorded defect with both readings owed a
//! conformance case — see this module's own historical note preserved below.
//!
//! **One further reading this pass had to adjudicate, because §3.8.3/§D state no
//! mechanism for it:** [`jlreq_spacing::Reduction::Discrete`] sites ("the full amount or
//! the floor, nothing between", §3.1.9) cannot take a *partial* share the way
//! [`jlreq_spacing::Reduction::Range`] sites can — there is no fraction of a binary flip.
//! [`reduce`] therefore drains every `Range` site in a stage first, up to that stage's own
//! capacity, and only flips `Discrete` sites, in site order, when `Range` capacity in that
//! stage cannot supply what is still needed — one at a time, stopping as soon as the
//! shortfall is covered, so a stage with several discrete sites does not flip a second one
//! once the first already closed the gap. This can still overshoot the exact amount
//! needed on the *last* site flipped — flipping a `Discrete` site is all-or-nothing, so the
//! actual reduction achieved may exceed the shortfall by up to that one site's own floor —
//! which is not a defect on a **non-last** line: it is a line reduced slightly past the
//! target, one [`expand`] call away from fitting exactly, and `crate::compose::adjust_line`
//! runs exactly that call when it happens — its own `residual = overflow(target,
//! after_reduction)` is nonzero precisely when reduction overshot, which is what routes an
//! overshot line into `expand` rather than reporting it early. A paragraph's **last** line
//! is a different case, stated here rather than left for
//! a reader to discover the two differ: `adjust_line` gates `expand` on
//! `kind.is_none() && !is_last_line`, so an overshoot on the last line never gets that
//! correcting call at all — which is not a defect either, because §3.8.1's own Note already
//! excuses the last line from ever needing to reach the target ("the last line still takes
//! reduction, never expansion"), so a last line left slightly short by a `Discrete` flip is
//! exactly the un-aligned line end that Note already permits, not a shortfall anything owes
//! a repair to. Flipping every discrete site in the stage regardless of whether it was
//! needed, by contrast, would be a defect on either kind of line — an unbounded overshoot no
//! single `expand` call could be relied on to close, and one a last line would never even
//! attempt to close — which is why the loop below stops the moment `remaining` reaches zero
//! rather than after it.
//!
//! [`Ladder`] and [`Site`] are consequently no longer opaque markers: [`Ladder::of`]
//! builds one from the sites [`crate::compose::geometry_of`] collects while it walks a
//! line's own boundaries — one [`Site`] per boundary that carries a non-`None`
//! [`jlreq_spacing::ConditionalSpace`] term, a real [`jlreq_spacing::Expansion`] with no
//! term behind it, or both at once (never two `Site`s for one physical gap — see
//! [`Site`]'s own doc) — in boundary order.
//!
//! **Residual expansion (§3.8.4 step (d)) is no longer the rare case ADR-0021's own move
//! makes it stop being.** Before this round, [`Expansion::Residual`] was reachable only at
//! the 94 class pairs where Table 1 already gave the boundary a term; after it, the other
//! 106 residual cells Table 6 states at a solid Table 1 coordinate are reachable too, and an
//! ordinary line of running Japanese text — cl-19 against cl-08, cl-19 against cl-13, and
//! the like — now offers `expand` far more `Expansion::Residual` sites than it used to.
//! [`expand`]'s own re-leveling code was already written generally enough for this: the
//! `union` it re-levels across is built from every stage-2, stage-3 and residual site
//! *unconditionally* (`union.extend` runs before either stage's own `if remaining ==
//! InlineExtent::ZERO` guard), so nothing here assumed residual sites would stay rare, and
//! nothing needs to change for them not to be. What changes is only how often a real line
//! reaches the branch at all: a line of solid kanji that used to have zero expandable sites
//! — `union` empty, `expand` reporting `still_short != InlineExtent::ZERO`, an
//! [`crate::ViolationKind::ExpansionExhausted`] the caller could not repair — now typically
//! has at least one residual site to re-level across, turning what used to be an
//! unconditional violation into a conforming, `Adjustment::releveled` line. That is an
//! improvement `Demerits::last_resort` already scores correctly (`docs/design/api-spine.md`'s
//! own ordering already ranks a releveled line above an unfit one), not a change this pass
//! had to make to the demerit accounting to earn.
//!
//! # §3.1.11's non-separation, read once and not re-derived
//!
//! §3.1.11 states that spacing must never be *increased* between certain character
//! sequences (opening/closing brackets, full stops, commas, middle dots, dividing marks,
//! hyphens, and — item 2(g) — the base characters under one jukugo-ruby). Its own second
//! note settles how this is expressed: "combinations of character classes which allow
//! spacing to be inserted for line alignment are described as a **complete table** in §E."
//! Table 6 is that complete table, and [`expand`] only ever drains a [`Site`] whose own
//! `expansion` field — [`jlreq_spacing::Boundary::expansion`], the carrier ADR-0021 gives
//! it — is not [`jlreq_spacing::Expansion::None`], so every class-pair-level prohibition
//! §3.1.11 states (items 2(a) through 2(f), and item 1's cross-reference to §3.1.10) is
//! already satisfied by construction, by the same generated data every other boundary fact
//! in this workspace reads, not by a second lookup here.
//!
//! That claim used to be true by accident and is now true on purpose, which is worth
//! recording rather than leaving the reader to notice: before this round, expansion lived
//! on [`jlreq_spacing::ConditionalSpace`] and was consequently readable only at a boundary
//! Table 1 also gave a term — so a class pair §3.1.11 forbids from widening could not have
//! been wrongly expanded, but only because *no* class pair with a blank Table 1 cell could
//! be expanded at all, prohibited ones included among the ones this argument needed to
//! rule out and the ones cl-19-against-cl-19 shows it wrongly ruled out along with them.
//! Today the same conclusion holds for the reason stated above — a prohibited class pair's
//! own Table 6 cell reads `Expansion::None` regardless of what Table 1 states there — which
//! is a fact about the coordinate, checked the identical way for every boundary this crate
//! evaluates, not a side effect of a term being missing.
//!
//! Item 2(g) — jukugo-ruby base runs — is the one clause a *class-pair* cell cannot
//! express: an ordinary cl-19×cl-19 boundary is expandable, but the same class pair
//! inside one jukugo-ruby complex's base run is not, and only run identity
//! ([`jlreq_unit::Runs`]) distinguishes the two. This is unreachable through
//! [`crate::compose::compose`] at M1 for the identical reason [`crate::feasible`]'s own
//! `same_run_refusal` is unreachable there too: [`crate::compose::compose`] never carries
//! a [`jlreq_unit::Runs`] other than [`jlreq_unit::Runs::none`] (jukugo-ruby is M4-a; see
//! `crate` root's own `# Status`), so there is no run data for a check here, or for
//! `same_run_refusal`, to read once `compose` is the caller. The two are no longer the
//! same gap, though, and the difference is worth naming rather than papering over:
//! `same_run_refusal` is filled and directly reachable through the public
//! [`crate::Feasible::compute`], while this module has no counterpart to reach at
//! all — `expand` adds no call that can never fire, because doing so would be a live seam
//! nobody can exercise standing in for a real one, which is worse than the honest gap
//! stated here: item 2(g) remains unchecked, named, until M4-a gives [`expand`] a
//! [`jlreq_unit::Runs`] worth reading.
//!
//! ## Historical note: the reduction reading
//!
//! That reading is an adjudication, recorded as such: §3.8.3's steps 4 through 6 say
//! "reduced equally in proportion to the character size", but its step 1 says "the same
//! width reduction is applied to all spaces on the target line at the same time" in
//! English against 文字サイズ比で均等に in Japanese. This project follows the Japanese, and
//! the divergence is a recorded defect with both readings owed a conformance case.

use alloc::vec;
use alloc::vec::Vec;

use jlreq_class::{Class, Text, resolve};
use jlreq_spec::{Policy, Question};
use jlreq_unit::{Advance, Carry, Em, InlineExtent, ItemIndex, RemainderRule, Size, distribute};

use jlreq_spacing::{ConditionalSpace, Expansion, Reduction};

use crate::compose::Hanging;

/// A line's flexibility, as an ordered ladder rather than a stretch/shrink pair.
///
/// This is the single most common way to get Japanese line adjustment wrong. TeX has one
/// proportional glue; JLReq has ordered stages — six for reduction (§3.8.3, Appendix D)
/// and four for expansion (§3.8.4, Appendix E) — drained in order and *equally within a
/// stage*, where "equally" means equal as a fraction of each site's own em, not equal in
/// absolute units. On a line mixing base-size and ruby-size runs those differ, and no test
/// of uniform-size text reveals it. See this module's own `# Status` for the recorded
/// English/Japanese divergence in step 1's own wording.
///
/// **Hanging punctuation (ぶら下げ) is a stage of this ladder**, between the reduction
/// stages and the expansion stages, and not a repair applied after a break is chosen —
/// `hang` runs between `reduce` and `expand` in `crate::compose::compose`'s own control
/// flow.
///
/// Not `Copy`, now that it holds data: a line's own site count is text-dependent, so
/// `Ladder::of` owns a `Vec<Site>`. No divergence to record here — `docs/design/api-spine.md`'s
/// own preamble already places `Ladder` directly in the by-reference, non-`Copy` bucket
/// alongside [`Adjustment`] (line 44), unlike [`crate::Fit`]'s own case, which *is* a
/// recorded tension (see [`crate::Fit`]'s own doc).
///
/// JLReq: §3.8.2, §3.8.3, §3.8.4, §2.5.1, §D, §E
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Ladder {
    sites: Vec<Site>,
}

impl Ladder {
    /// Build a ladder from the sites [`crate::compose::geometry_of`] (and
    /// [`crate::compose::trailing_of`]) collect while walking one line's own boundaries,
    /// in boundary order — the ordered list [`reduce`], [`hang`] and [`expand`] drain.
    ///
    /// Crate-visible: only [`crate::compose::compose`] and [`crate::align::align`]
    /// (which never calls [`reduce`], [`hang`] or [`expand`] — see `crate::align`'s own
    /// `# What this is not`) ever hold the sites to build one from.
    ///
    /// JLReq: §B.1, §D, §E
    #[must_use]
    pub(crate) fn of(sites: Vec<Site>) -> Self {
        Self { sites }
    }

    /// This ladder's sites, in boundary order. Crate-visible: [`crate::compose`] reads
    /// [`Site::shift_from`] to turn a drained [`Adjustment`]'s per-site deltas into
    /// placements, without reaching into this module's other, drain-only fields.
    ///
    /// JLReq: §B.1, §D, §E
    pub(crate) fn sites(&self) -> &[Site] {
        &self.sites
    }
}

/// One adjustable site: the room at one boundary for `reduce` and `expand` to drain,
/// together with the referent's own character size as a weight (§3.8.3's "in proportion to
/// the character size").
///
/// A site carries an *optional* [`jlreq_spacing::ConditionalSpace`] and its own
/// [`jlreq_spacing::Expansion`] separately, not a `ConditionalSpace` alone (ADR-0021 amends
/// ADR-0014 on this point): Table 6 states one opportunity per class pair, a fact about the
/// boundary, and a solid Table 1 cell — no term of either referent's — can still name one.
/// cl-19 against cl-19 (kanji beside kanji) is `blank` in Table 1 and `0-1/4 stage 3` in
/// Table 6, and is consequently a `Site` with `space: None` and a real `expansion`. The
/// physical gap between two characters is still exactly one `Site` either way: this crate's
/// own `xtask attest` invariant (`expansion-needs-no-referent`) checks, over the captured
/// tables, that no coordinate ever needs both a `ConditionalSpace` *and* an independent
/// per-referent `Expansion` at once, which is what lets `crate::compose::boundary_spaces`
/// attach one boundary's `expansion` to whichever single site it already built (or, when it
/// built none, synthesize the one site the gap needs) rather than ever risking two.
///
/// JLReq: §B.1, §D, §E
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Site {
    /// The conditional space this site realizes, when the boundary carries one. `None` for
    /// a site synthesized purely from a boundary's own [`jlreq_spacing::Expansion`] at a
    /// solid Table 1 cell.
    space: Option<ConditionalSpace>,
    /// This site's own expansion opportunity — [`jlreq_spacing::Boundary::expansion`],
    /// carried here rather than read back through `space` because it is not `space`'s fact
    /// to carry (ADR-0021).
    expansion: Expansion,
    /// The weight [`jlreq_unit::distribute`] splits a stage's demand by — the character
    /// size the caller passed to [`Site::new`]. For a term-carrying site this is always the
    /// referent's own size, the same em the term's realized amount was resolved against.
    /// For a site with `space: None` there is no referent to select one from, and the
    /// choice is consequently arbitrary — see this module's own gap-enumeration block for
    /// which one the caller picks and why the choice cannot matter except when the two
    /// neighbors differ in size.
    weight: Advance,
    /// How far this site may still be reduced: `amount − floor`, resolved once against
    /// the size passed to [`Site::new`]. Zero for a [`Reduction::Rigid`] site and for a
    /// site with no conditional space at all — there is no amount to reduce.
    reduce_headroom: InlineExtent,
    /// How far this site may still be expanded: `ceiling − amount`, resolved once against
    /// the size passed to [`Site::new`], where `amount` is the conditional space's own
    /// realized amount when this site has one and zero otherwise (a solid boundary's own
    /// realized amount, pre-adjustment, is nothing — see this module's own gap-enumeration
    /// block for the one reading this needs, §3.1.3's vertical withdrawal). Zero for
    /// [`Expansion::None`] and for [`Expansion::Residual`] (unbounded, not a magnitude —
    /// see [`Adjustment::releveled`]'s own doc).
    expand_headroom: InlineExtent,
    /// The item ordinal this site's own boundary sits before, in the *text's* own
    /// numbering — the same numbering [`crate::compose::geometry_of`] built it from and
    /// every other [`ItemIndex`] this crate reports uses — not an offset already relative
    /// to the line: every placement at or after it shifts by this site's realized delta,
    /// which is [`crate::compose::apply_adjustment`]'s own job, and that function is the
    /// one place this ordinal is rebased against a line-relative placement vector before
    /// it is used as an index (its own doc states why). `None` for the line-end boundary,
    /// which has no item after it to shift — only [`crate::Line::trailing`] and
    /// [`crate::Line::extent`] move.
    shift_from: Option<ItemIndex>,
}

impl Site {
    /// Build one site from the [`jlreq_spacing::ConditionalSpace`]
    /// [`crate::compose::geometry_of`] already resolved for one boundary term (`None` for a
    /// solid boundary), the boundary's own [`jlreq_spacing::Expansion`], the size headroom
    /// is resolved against, and the item ordinal placements shift from.
    ///
    /// `size` is the referent's own [`Size`] when `space` is `Some` (so headroom is resolved
    /// against the same em the space's own realized amount already was), and the caller's
    /// own choice otherwise — there is no referent to select one from, which this module's
    /// own gap-enumeration block names as an open slot rather than a silent default.
    ///
    /// The two headroom resolutions each use a fresh [`Carry`]: each is the *only*
    /// resolution of its own `Em` value this ladder ever performs (a floor and a ceiling
    /// are read once per site, not accumulated across a run of calls the way
    /// `crate::compose::geometry_of`'s own cursor is), so a fresh carry's remainder slot
    /// starts, and stays, exact for it (`docs/adr/0019`).
    ///
    /// JLReq: §B.1, §D.1, §E.1
    pub(crate) fn new(
        space: Option<ConditionalSpace>,
        expansion: Expansion,
        size: Size,
        shift_from: Option<ItemIndex>,
    ) -> Self {
        let weight = size.scale().inline_em();

        // `Reduction::Rigid` states no floor, so it has no room to drain; the wildcard
        // is `Reduction`'s `#[non_exhaustive]` (a variant this crate cannot yet see),
        // read the same conservative way, so both collapse to one arm. A site with no
        // conditional space at all collapses to the identical zero: there is no floor
        // because there is no amount above one to begin with.
        let reduce_headroom = space.map_or(InlineExtent::ZERO, |space| match space.reduction() {
            Reduction::Range { floor, .. } | Reduction::Discrete { floor, .. } => {
                let mut carry = Carry::new();
                space
                    .amount()
                    .sub_sat(floor)
                    .resolve_inline(size, &mut carry)
            },
            _ => InlineExtent::ZERO,
        });

        // `Expansion::None` states no opportunity; `Expansion::Residual` states an
        // unbounded one with no ceiling to subtract from, which `expand`'s own
        // re-leveling pass reads this site's own `expansion` field directly for rather
        // than through this headroom (see `crate::ladder::expand`'s own doc). The
        // wildcard is `Expansion`'s `#[non_exhaustive]`, read the same conservative way,
        // so all three collapse to one arm. `base` is the amount already realized before
        // adjustment, to subtract the ceiling from: a term's own amount when there is one,
        // and zero for a solid boundary — nothing was ever placed there to begin with, the
        // same reading `evaluate::boundary`'s own §3.1.3 withdrawal argues at its own site.
        let expand_headroom = match expansion {
            Expansion::Range { ceiling, .. } => {
                let mut carry = Carry::new();
                let base = space.map_or(Em::ZERO, ConditionalSpace::amount);
                ceiling.sub_sat(base).resolve_inline(size, &mut carry)
            },
            _ => InlineExtent::ZERO,
        };

        Self {
            space,
            expansion,
            weight,
            reduce_headroom,
            expand_headroom,
            shift_from,
        }
    }

    /// The item ordinal placements shift from once this site's delta is realized, or
    /// `None` for the line-end boundary.
    ///
    /// JLReq: n/a (addressing)
    pub(crate) const fn shift_from(self) -> Option<ItemIndex> {
        self.shift_from
    }
}

/// What was done to a line. Deterministic and replayable.
///
/// JLReq: §3.8.2, §3.8.3, §3.8.4
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Adjustment {
    /// The realized amount at each site, in site order: negative where reduced, positive
    /// where expanded, zero where the site was left solid.
    per_site: Vec<InlineExtent>,
    /// The sites that were reduced, and by how much, in site order.
    reduced: Vec<InlineExtent>,
    /// The sites that were expanded, and by how much, in site order.
    expanded: Vec<InlineExtent>,
    /// Whether the fourth expansion stage's re-leveling ran.
    releveled: bool,
}

impl Adjustment {
    /// The record of a line that needed neither reduction nor expansion: every site left
    /// solid.
    ///
    /// JLReq: n/a (ADR-0010)
    #[must_use]
    pub(crate) const fn empty() -> Self {
        Self {
            per_site: Vec::new(),
            reduced: Vec::new(),
            expanded: Vec::new(),
            releveled: false,
        }
    }

    /// Build the record of what [`reduce`], [`expand`] and the final re-leveling pass
    /// actually did: `per_site` is exactly what [`reduce`] and [`expand`] returned,
    /// summed element-wise over the same [`Ladder::sites`] order (a site can carry both a
    /// [`reduce`]-authored delta and an [`expand`]-authored one across one line's own
    /// pipeline — a `Reduction::Discrete` overshoot corrected by a small expansion, see
    /// this module's own `# Status`); `reduced` and `expanded` are the nonzero magnitudes
    /// filtered out of it, in site order, dropping the zero entries `per_site` states
    /// explicitly and they do not need to restate.
    ///
    /// JLReq: §3.8.2, §3.8.3, §3.8.4
    pub(crate) fn of(per_site: Vec<InlineExtent>, releveled: bool) -> Self {
        let mut reduced = Vec::new();
        let mut expanded = Vec::new();
        for &delta in &per_site {
            if delta == InlineExtent::ZERO {
                continue;
            }
            if delta.min(InlineExtent::ZERO) == delta {
                reduced.push(delta.neg_sat());
            } else {
                expanded.push(delta);
            }
        }
        Self {
            per_site,
            reduced,
            expanded,
            releveled,
        }
    }

    /// The realized *delta* at each site, in the ladder's own boundary order: negative
    /// where reduced, positive where expanded, zero where the site was left solid — the
    /// same value this struct's own `per_site` field holds, see its doc.
    ///
    /// **A recorded divergence from `docs/design/api-spine.md`'s own purpose clause**,
    /// stated here rather than silently resolved either way: the spine's prose reads "so
    /// ruby overhang can be capped by what survived (§3.3.8 rule 3) rather than by the
    /// nominal amount", which points at the *post-adjustment magnitude* at a site (nominal
    /// amount plus this delta) — a different quantity from the delta this accessor actually
    /// returns. §3.3.8's ruby overhang is uncalled at M1 ([`crate::Line::overhang`] is
    /// always empty, this crate's own `# Status`), so there is no M4 consumer yet to
    /// adjudicate the two readings against, and picking one now would be a guess this
    /// module's own discipline forbids. When M4-a wires ruby overhang, it needs either a
    /// second accessor for the survived magnitude or [`Site`] to publish the nominal amount
    /// this delta is relative to (`Site` does not carry it today — only the referent's own
    /// weight and its reduce/expand headroom, not the resolved amount itself) so the caller
    /// can add the two; changing what this method itself returns, rather than adding
    /// alongside it, would also change what [`Adjustment::reduced`]/[`Adjustment::expanded`]
    /// can be derived from (both classify by this same delta's sign), which is a second
    /// reason this is left as a recorded gap rather than resolved unilaterally here.
    ///
    /// JLReq: §3.3.8
    #[must_use]
    pub fn per_site(&self) -> &[InlineExtent] {
        &self.per_site
    }

    /// The sites that were reduced, and by how much.
    ///
    /// JLReq: §3.8.3, §D
    #[must_use]
    pub fn reduced(&self) -> &[InlineExtent] {
        &self.reduced
    }

    /// The sites that were expanded, and by how much.
    ///
    /// JLReq: §3.8.4, §E
    #[must_use]
    pub fn expanded(&self) -> &[InlineExtent] {
        &self.expanded
    }

    /// §E: "When the 4th step is needed, evenly add space to equalize the spacing of
    /// 1st, 2nd, 3rd and 4th steps." A re-leveling, not another bucket.
    ///
    /// JLReq: §3.8.4, §E
    #[must_use]
    pub const fn releveled(&self) -> bool {
        self.releveled
    }
}

/// §3.8.3's reduction procedure: drain reduction stages 2 through 6 in priority order —
/// stage 1, the Western word space (cl-26), lies outside Appendix D and is not a site
/// this ladder ever carries (`jlreq_spacing::ReductionStage`'s own doc) — stopping as
/// soon as `need` is fully placed. Returns the per-site deltas (dense, in [`Ladder::sites`]
/// order, each `<= 0`), how much of `need` could not be placed (zero once fully drained),
/// and the deepest stage ordinal actually engaged (zero if none was).
///
/// Within one stage, [`Reduction::Range`] sites drain first, proportionally by each
/// site's own character-size weight and capped at each site's own floor
/// ([`drain_capped`]); [`Reduction::Discrete`] sites — which cannot take a partial share,
/// §3.1.9 — flip to their floor one at a time, in site order, only once `Range` capacity in
/// the stage is insufficient, and only as many of them as it takes to cover what is still
/// needed. See this module's own `# Status` for why that order is this pass's own
/// adjudication and not a stated procedure.
///
/// JLReq: §3.8.3, §D, §D.1
#[must_use]
pub(crate) fn reduce(
    ladder: &Ladder,
    need: InlineExtent,
    policy: Policy,
) -> (Vec<InlineExtent>, InlineExtent, u8) {
    let remainder = policy.remainder();
    let sites = ladder.sites();
    let mut deltas = vec![InlineExtent::ZERO; sites.len()];
    let mut remaining = need;
    let mut deepest = 0u8;

    for stage_number in 2..=6u8 {
        if remaining == InlineExtent::ZERO {
            break;
        }

        // A site with no conditional space (`space: None`) has no `Reduction` to read at
        // all, and `Option::map` over it never matches either arm below — the same "no
        // room to drain" reading `Site::new`'s own `reduce_headroom` already gives it.
        let range: Vec<usize> = (0..sites.len())
            .filter(|&i| matches!(sites[i].space.map(ConditionalSpace::reduction), Some(Reduction::Range { stage, .. }) if stage.ordinal() == stage_number))
            .collect();
        let discrete: Vec<usize> = (0..sites.len())
            .filter(|&i| matches!(sites[i].space.map(ConditionalSpace::reduction), Some(Reduction::Discrete { stage, .. }) if stage.ordinal() == stage_number))
            .collect();

        if range.is_empty() && discrete.is_empty() {
            continue;
        }

        let range_weights: Vec<Advance> = range.iter().map(|&i| sites[i].weight).collect();
        let range_caps: Vec<InlineExtent> =
            range.iter().map(|&i| sites[i].reduce_headroom).collect();
        let range_capacity = range_caps
            .iter()
            .fold(InlineExtent::ZERO, |a, &b| a.add_sat(b));
        let take_from_range = remaining.min(range_capacity);
        if take_from_range != InlineExtent::ZERO {
            let placed = drain_capped(&range_weights, &range_caps, take_from_range, remainder);
            for (&i, &share) in range.iter().zip(&placed) {
                deltas[i] = deltas[i].sub_sat(share);
            }
            remaining = remaining.sub_sat(take_from_range);
            deepest = deepest.max(stage_number);
        }

        if remaining == InlineExtent::ZERO {
            continue;
        }

        for &i in &discrete {
            if remaining == InlineExtent::ZERO {
                break;
            }
            let cap = sites[i].reduce_headroom;
            if cap == InlineExtent::ZERO {
                continue;
            }
            deltas[i] = deltas[i].sub_sat(cap);
            remaining = remaining.sub_sat(cap.min(remaining));
            deepest = deepest.max(stage_number);
        }
    }

    (deltas, remaining, deepest)
}

/// §3.8.2's ぶら下げ (burasage): whether the line's own last item may hang past the
/// measure instead of being reduced or expanded for. Only cl-06 (full stops) and cl-07
/// (commas) are ever eligible (§2.5.1), and only up to `shortfall` — the amount still
/// needed after [`reduce`] — capped at the item's own advance, so a line is never
/// credited more hang than it actually needed. `None` when
/// `Question::HANGING_PUNCTUATION` reads "none" (`jlreq_spec`'s own default), when
/// `shortfall` is already zero, or when the last item is not cl-06 or cl-07.
///
/// The classification reads [`Policy::JLREQ`] rather than the caller's own `policy`, the
/// same choice `jlreq_spacing::evaluate`'s own `class_of` makes and for the same reason:
/// [`jlreq_class::resolve`] already folds classification ambiguity into this project's
/// one published reading, a fact about the character and not about the caller's own
/// adjustment policy.
///
/// JLReq: §3.8.2, §2.5.1
#[must_use]
pub(crate) fn hang(
    text: Text<'_>,
    last: ItemIndex,
    shortfall: InlineExtent,
    policy: Policy,
) -> Option<Hanging> {
    if shortfall == InlineExtent::ZERO {
        return None;
    }
    if policy.get(Question::HANGING_PUNCTUATION).name() != "hanging" {
        return None;
    }
    let class = resolve(text, last, Policy::JLREQ)?.value();
    if !matches!(class, Class::FullStop | Class::Comma) {
        return None;
    }
    let item = text.items().get(last.get() as usize).copied()?;
    let beyond = shortfall.min(item.advance());
    Some(Hanging {
        item: last,
        beyond,
        rule: Question::HANGING_PUNCTUATION.rule(),
    })
}

/// §3.8.4's expansion procedure: drain Table 6's own second and third priority stages —
/// its first, cl-26's Western word space, is outside Table 6 (see this module's own
/// `# Status`) — then, if `need` still is not fully placed, re-level: §E.1's own words,
/// "in addition to the amount already processed at stages 1 through 3 [here, 2 and 3,
/// for the reason above], evenly space out the sites of priority stages 1 through 4" —
/// read here as adding the *remaining* need, undistinguished by each site's own stage
/// ceiling, across the union of every stage-2, stage-3 and [`Expansion::Residual`] site,
/// on top of whatever stages 2 and 3 already placed. Returns the per-site deltas (dense,
/// in [`Ladder::sites`] order, each `>= 0`), how much of `need` could not be placed (zero
/// unless the ladder has no expandable site at all), the deepest ordinary stage engaged
/// (2, 3, or 0 if neither was), and whether re-leveling ran.
///
/// JLReq: §3.8.4, §E, §E.1
#[must_use]
pub(crate) fn expand(
    ladder: &Ladder,
    need: InlineExtent,
    policy: Policy,
) -> (Vec<InlineExtent>, InlineExtent, u8, bool) {
    let remainder = policy.remainder();
    let sites = ladder.sites();
    let mut deltas = vec![InlineExtent::ZERO; sites.len()];
    let mut remaining = need;
    let mut deepest = 0u8;
    let mut union: Vec<usize> = Vec::new();

    for stage_number in [2u8, 3u8] {
        let indices: Vec<usize> = (0..sites.len())
            .filter(|&i| matches!(sites[i].expansion, Expansion::Range { stage, .. } if stage.ordinal() == stage_number))
            .collect();
        union.extend(indices.iter().copied());
        if remaining == InlineExtent::ZERO || indices.is_empty() {
            continue;
        }
        let weights: Vec<Advance> = indices.iter().map(|&i| sites[i].weight).collect();
        let caps: Vec<InlineExtent> = indices.iter().map(|&i| sites[i].expand_headroom).collect();
        let capacity = caps.iter().fold(InlineExtent::ZERO, |a, &b| a.add_sat(b));
        let take = remaining.min(capacity);
        if take != InlineExtent::ZERO {
            let placed = drain_capped(&weights, &caps, take, remainder);
            for (&i, &share) in indices.iter().zip(&placed) {
                deltas[i] = deltas[i].add_sat(share);
            }
            remaining = remaining.sub_sat(take);
            deepest = deepest.max(stage_number);
        }
    }

    let mut releveled = false;
    if remaining != InlineExtent::ZERO {
        let residual_indices: Vec<usize> = (0..sites.len())
            .filter(|&i| matches!(sites[i].expansion, Expansion::Residual))
            .collect();
        union.extend(residual_indices);
        if !union.is_empty() {
            let weights: Vec<Advance> = union.iter().map(|&i| sites[i].weight).collect();
            for (&i, share) in union.iter().zip(distribute(remaining, &weights, remainder)) {
                deltas[i] = deltas[i].add_sat(share);
            }
            remaining = InlineExtent::ZERO;
            releveled = true;
        }
    }

    (deltas, remaining, deepest, releveled)
}

/// Split `need` across the sites named by `weights`/`caps` (same index, same order),
/// proportionally by weight, never assigning a site more than its own `caps` entry:
/// where a proportional share would exceed a site's cap, that site takes its cap and the
/// leftover is redistributed among the sites not yet at theirs, weighted the same way.
/// Terminates in at most one pass per site, because each pass either places all of
/// `need` or removes at least one site from further consideration.
///
/// The precondition every caller here holds: `need <= caps.iter().sum()`. Under it this
/// places all of `need`; a caller that breaks the precondition gets whatever the capped
/// sites can hold and no more, which is the same answer a correct caller would compute
/// for itself, not a silent truncation invented here.
fn drain_capped(
    weights: &[Advance],
    caps: &[InlineExtent],
    need: InlineExtent,
    remainder: RemainderRule,
) -> Vec<InlineExtent> {
    let n = weights.len();
    let mut result = vec![InlineExtent::ZERO; n];
    let mut active: Vec<usize> = (0..n).filter(|&i| caps[i] != InlineExtent::ZERO).collect();
    let mut left = need;

    for _ in 0..=n {
        if left == InlineExtent::ZERO || active.is_empty() {
            break;
        }
        let active_weights: Vec<Advance> = active.iter().map(|&i| weights[i]).collect();
        let mut placed_total = InlineExtent::ZERO;
        let mut newly_capped: Vec<usize> = Vec::new();
        for (&i, share) in active
            .iter()
            .zip(distribute(left, &active_weights, remainder))
        {
            let room = caps[i].sub_sat(result[i]);
            let take = share.min(room);
            result[i] = result[i].add_sat(take);
            placed_total = placed_total.add_sat(take);
            if take == room {
                newly_capped.push(i);
            }
        }
        left = left.sub_sat(placed_total);
        if newly_capped.is_empty() {
            break;
        }
        active.retain(|i| !newly_capped.contains(i));
    }

    result
}

#[cfg(test)]
mod tests {
    use jlreq_unit::{Advance, InlineExtent, RemainderRule};

    use super::drain_capped;

    fn extent(units: i32) -> InlineExtent {
        InlineExtent::new(units).expect("a valid extent")
    }

    fn advance(units: i32) -> Advance {
        Advance::new(units).expect("a valid advance")
    }

    #[test]
    fn a_capped_drain_matches_plain_distribution_under_the_cap() {
        let weights = [advance(1), advance(1), advance(1)];
        let caps = [extent(100), extent(100), extent(100)];
        let result = drain_capped(&weights, &caps, extent(90), RemainderRule::Leading);
        let total = result
            .iter()
            .copied()
            .fold(extent(0), InlineExtent::add_sat);
        assert_eq!(
            total,
            extent(90),
            "well under every cap, the full need is placed"
        );
        assert_eq!(
            (result[0], result[1], result[2]),
            (extent(30), extent(30), extent(30))
        );
    }

    #[test]
    fn a_capped_drain_redistributes_past_an_exhausted_site() {
        // One tiny site, two roomy ones, equal weights: naive equal thirds would try to
        // give the tiny site more than it has room for.
        let weights = [advance(1), advance(1), advance(1)];
        let caps = [extent(10), extent(100), extent(100)];
        let result = drain_capped(&weights, &caps, extent(90), RemainderRule::Leading);
        let total = result
            .iter()
            .copied()
            .fold(extent(0), InlineExtent::add_sat);
        assert_eq!(
            total,
            extent(90),
            "the full need is still placed, past the small cap"
        );
        assert_eq!(
            result[0],
            extent(10),
            "the small site never exceeds its own cap"
        );
        assert_eq!(result[1], extent(40));
        assert_eq!(result[2], extent(40));
    }

    #[test]
    fn a_capped_drain_over_no_sites_places_nothing() {
        let result = drain_capped(&[], &[], extent(50), RemainderRule::Leading);
        assert!(result.is_empty());
    }
}
