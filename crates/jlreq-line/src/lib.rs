// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Line composition: turning a sequence of characters into lines.
//!
//! The Unicode line breaking algorithm (UAX #14) says where a break is *permitted*. JLReq
//! says which of those breaks are *acceptable* — kinsoku (禁則) forbids `、` and `。` from
//! starting a line and forbids an opening bracket from ending one — and what to do when
//! no acceptable break exists: compress the line to pull the character back (追い込み,
//! oikomi) or expand it to push the character down (追い出し, oidashi), optionally hanging
//! the punctuation past the line end (ぶら下げ).
//!
//! That gap is why text laid out by a UAX #14 implementation alone breaks in places a
//! Japanese reader immediately recognizes as wrong. This crate closes it. It consumes
//! break opportunities rather than discovering them (see `docs/adr/0003`).
//!
//! Lines advance along an *inline* axis and stack along a *block* axis. There is no `x`
//! and no `y` here, and consequently no separate vertical implementation (see
//! `docs/adr/0004`).
//!
//! Uses `alloc`: a paragraph's breaks, its lines and their trims are sequences whose
//! length depends on the text, which is the one boundary in the workspace
//! `ARCHITECTURE.md` names as allocating (see `docs/design/api-spine.md`).
//!
//! # Module map
//!
//! ```text
//! src/
//!   lib.rs        re-exports; crate docs (this file)
//!   feasible.rs   Candidate, CandidateIndex, Feasible, FeasibleBreak
//!   ladder.rs     Ladder, Site, Adjustment
//!   objective.rs  Badness, Demerits, Preference, Fit, Deepest
//!   compose.rs    Paragraph, Composition, Line, Trim, Violation, Search, compose(),
//!                 PullUp, Hanging, Rewrite
//!   align.rs      Alignment, align()
//!   tab.rs        TabKind, TabStop, TabLine, tab_line()
//! ```
//!
//! Two files `docs/design/api-spine.md`'s own module map names are not here, and both are
//! absent for the same reason rather than by oversight: `segment.rs` (the nested
//! composition of a [`jlreq_unit::Segment`]) and `generated/figures.rs` (§3.4.3's and
//! §3.7.2's straddle arrangements, captured only as images). Both need
//! `jlreq_inline::Contribution` to ever be non-trivial, and the crate graph gives
//! `jlreq-line` no edge to `jlreq-inline` at all ([ADR 0015](../adr/0015)): "a warichu that
//! does not fit in what is left of the current line wraps onto the following one... every
//! break selection in the workspace consequently happens in `jlreq-line`... the construct
//! layer hands it a `Segment` instead," and the reverse edge is refused with the same
//! argument. `Paragraph::with_contribution` is consequently not offered either — an empty
//! file or a builder step nothing can call would be exactly the placeholder this workspace
//! forbids, so the honest statement is their absence, here, until M4 gives them something
//! to hold ([ROADMAP.md] M4).
//!
//! A fourth name the spine's own module map lists for `ladder.rs` — `adjust()` — is also
//! not offered, for a reason worth stating rather than leaving silent: `reduce`, `hang`
//! and `expand` (`crate::ladder`'s own `# Status`) are `pub(crate)` precisely because
//! orchestrating the three is [`crate::compose`]'s own job, not `crate::ladder`'s — its
//! `adjust_line` interleaves the ladder's three ordered stages with facts only a *line*
//! carries (whether it is the paragraph's own last, per §3.8.1's Note; how much of the
//! shortfall each stage still owes the next), which no `Ladder` method has to hand. A
//! single `Ladder::adjust()` folding that orchestration in would duplicate `adjust_line`
//! rather than replace it, or strip the logic out of `compose` and hand a caller three raw
//! calls to sequence correctly themselves — worse than the module boundary as built.
//! `reduce`, `hang` and `expand` are consequently the three real, separately callable
//! stages the spine's own single name would have had to merge.
//!
//! # Status
//!
//! M1-b's frame. The vocabulary and the composition control flow are implemented, and so
//! is every rule table that decides *which* character may not start or end a line and
//! *how far* a space may be reduced or expanded — the latter now that this pass fills
//! [`crate::Ladder`] (see its own `# Status`). What remains an explicit slot for a later,
//! independently authored phase (see `docs/adr/0006`: an implementation and its
//! conformance cases are written apart) is exactly the three items "Slots", below, names:
//! the "matrix" reading of `Question::RELAXATION_MECHANISM`, §3.1.11 item 2(g)'s
//! jukugo-ruby same-run non-separation, and §3.1.5's own paragraph-first-line half of the
//! opening-bracket line-head patterns `jlreq_spacing` now answers the other half of. The
//! same-run breakability a class-pair table cell
//! cannot express — §C.2 notes 6 through 8 and 13 — is real this pass and moves to "Wired,
//! not slotted" below, with its own scope limit stated there.
//!
//! [`align`] is a separate case rather than one more slot to fill later: §3.8.1's own Note
//! states single line alignment is a distinct process from line adjustment, and every one
//! of its four [`Alignment`] methods is real and complete at this milestone — not because
//! [`crate::Ladder`] happens to be unfilled, but because §3.8.1's Note makes alignment a
//! process line adjustment's own ladder never enters (`align`'s own `# What this is not`).
//! [`Alignment::EvenSpacing`] (§3.7.3's jidori) is the one method that distributes space
//! the way expansion eventually will, and it is real *as its own computation* over
//! [`jlreq_unit::distribute`] — see `align`'s own module doc for the shared spacing
//! computation the four methods share, the two exceptions §3.7.3 states for
//! `EvenSpacing`, and why `EvenSpacing` is not an early, partial implementation of
//! [`crate::Ladder`]'s own expansion stage.
//!
//! §3.6's tab setting (3.6.1, 3.6.2, 3.6.3) is a third case, alongside [`align`] rather
//! than a mode of either it or [`compose`]: [`crate::tab::tab_line`] places §3.6.1's own
//! declared target runs against §3.6.1's own declared tab positions, choosing among
//! §3.6.2's four kinds and applying §3.6.3's own placement algorithm — the ordinary case,
//! the overflow a too-long run causes, the overlap clamp that follows from it, and the
//! deferral to the next line a caller's own declared stops can run out of — all real, not
//! slotted (`crate::tab`'s own module doc states each in full, including the design
//! decisions §3.6 itself leaves open: a declared-stop shortage caught at the call versus a
//! runtime one only the search discovers, an unenforced declaration order, and how the
//! specified-character kind names its own occurrence). What is *not* part of this
//! milestone, stated once here rather than
//! left for a reader to infer from the absence: `compose` never calls into `crate::tab`
//! and `crate::tab` never calls into `compose`, so a paragraph containing tab-set material
//! has nowhere in this crate yet that decides where a line break falls around one — an
//! undeclared boundary would be the defect; a declared one, stated here and in `crate::tab`
//! itself, is not.
//!
//! **Wired, not slotted** — these read specification content that another crate already
//! implements and tests, so calling it is composition rather than invention:
//!
//! - `feasible::same_run_refusal` answers §C.2 notes 6 through 8 and 13's same-run
//!   breakability, which Table 2's per-class-pair cell cannot express and which
//!   `docs/design/api-spine.md` states explicitly belongs here: "the same-run refusals of
//!   §C.2 notes 6 through 8 and 13 are decided here, in the crate that owns break refusal"
//!   (ADR 0015). It is real and called from every candidate [`Feasible::compute`]
//!   evaluates (see `feasible`'s own module doc and `same_run_refusal`'s own doc for the
//!   four notes, the one reading this pass had to adjudicate, and the construct kinds none
//!   of the four governs) — but its scope limit today is [`crate::compose::compose`]'s,
//!   not its own: `compose` still composes plain text, passing [`jlreq_unit::Runs::none`]
//!   unconditionally (see that function's own comment at the call), so the refusal is
//!   reachable only through the public [`Feasible::compute`], called directly with a
//!   caller-built [`jlreq_unit::Runs`] overlay — this module's own tests do exactly that.
//! - [`Feasible::compute`] evaluates every candidate against
//!   [`jlreq_spacing::boundary`]'s already-built Table 1 placement (`×`) and
//!   Table 2 breakability, at every strictness level `Question::KINSOKU_LEVEL` selects.
//!   That is the whole of §3.1.7 and §3.1.8's line-start and line-end prohibition: Table 2
//!   answers "may a line end between these two classes" per class pair already, which is
//!   what a character being unable to *start* a line reduces to at an ordinary interior
//!   break.
//! - §C.2 note 12's caller-supplied hyphenation discretionary
//!   ([`Candidate::Discretionary`]) is read directly off the generated cl-27×cl-27 cell of
//!   Table 2 (`prohibited: false, levels: 0b1111` — every level refuses it by default) and
//!   is the one candidate kind this milestone's evaluator exempts from that refusal, because
//!   without the exemption the variant would mean nothing.
//! - [`Composition::demerits`] and every [`Line::adjustment`] read
//!   [`Badness::of`]'s own defined zero-flex behavior (`docs/design/api-spine.md`: "a rigid
//!   line with no residual is `Badness::ZERO`, and a rigid line with a residual is
//!   `Badness::WORST`"), which is exact for a line whose own ladder is genuinely
//!   exhausted — every site fully drained, nothing left to weigh a residual against — not
//!   this milestone approximating badness for a ladder that cannot move at all
//!   (`crate::compose`'s own `demerits_of` reads the residual [`Badness::of`] takes the
//!   same way: what the fully-drained ladder could not place). `Search::Optimal`'s own
//!   `tolerance` now reads this identical value when it decides whether a line is admitted
//!   to the search (`crate::compose::run_dp`'s own doc), which is why that variant's own
//!   doc states the two settings this zero-flex reading makes reachable rather than a
//!   graduated scale.
//! - [`Alignment::EvenSpacing`]'s eligibility test reads the same
//!   [`jlreq_spacing::boundary`] Table 2 breakability [`Feasible::compute`] reads for an
//!   ordinary interior break, at the same policy-selected strictness level
//!   (`Question::KINSOKU_LEVEL`), so which interior boundaries receive a share of the
//!   distributed residual is exactly as real as which interior boundaries are feasible
//!   break candidates elsewhere in this crate.
//! - `ladder::reduce` and `ladder::expand` drain reduction Tables 3 through 5
//!   (`Question::REDUCTION_TABLE`'s three readings) and Table 6, over the
//!   `jlreq_spacing::Reduction` each [`jlreq_spacing::ConditionalSpace`] carries and the
//!   `jlreq_spacing::Expansion` each [`jlreq_spacing::Boundary`] carries — the priority-stage
//!   data and the amounts are `jlreq-spacing`'s, read and not re-derived. The two are no
//!   longer the same carrier (ADR-0021 amends ADR-0014): Table 6 states one opportunity per
//!   class pair, so it is the boundary's own fact, and `crate::ladder::Site` reads it
//!   independently of whether the boundary also gave either neighbor a conditional space —
//!   see `Site`'s own doc for the shape this gives a solid boundary's own expansion site.
//!   `ladder::hang` reads `Question::HANGING_PUNCTUATION` and [`jlreq_class::resolve`]'s own
//!   published classification the same way. See `crate::ladder`'s own `# Status` for exactly
//!   what each drains, the readings this pass had to adjudicate (`Reduction::Discrete`'s
//!   binary flip has no stated priority against `Reduction::Range` within a stage, and a
//!   solid boundary's own expansion site has no referent to weigh it against), and the two
//!   `jlreq-spacing` gaps (`Question::JAPANESE_LATIN_EXPANSION_CEILING` and §E.1's own
//!   Table-5 cross-coupling) this pass found unwired downstream and did not patch here.
//!
//! **Slots** — module-private seams a later, independently authored phase fills, named by
//! the rule address each one implements. Each slot's own doc names the address; none is
//! called from the control flow that does not yet have an answer to feed it, so an unfilled
//! slot is an honest gap (`Feasible::rejected` and `Composition::violations` report the
//! consequence rather than hiding it) and never a wrong answer:
//!
//! - the "matrix" arm of `Question::RELAXATION_MECHANISM` — §C.3's alternative to
//!   `jlreq-class`'s already-wired reclassification, applying a relaxed breakability
//!   directly at the member or pair granularity `jlreq_spacing::Predicate::Relaxes`
//!   names and that evaluator does not yet match.
//! - §3.1.11 item 2(g)'s jukugo-ruby same-run non-separation, the one clause of §3.1.11
//!   Table 6's own completeness cannot express (`crate::ladder`'s own `# Status`),
//!   unreachable while [`crate::compose::compose`] carries no [`jlreq_unit::Runs`] other
//!   than [`jlreq_unit::Runs::none`] (jukugo-ruby is M4-a).
//! - §3.1.5's own paragraph-first-line half (改行行頭) of Figure 71's three opening-bracket
//!   line-head patterns. `jlreq_spacing::evaluate::boundary` now answers the *other* half —
//!   the ordinary in-paragraph wrap's own line head (折返し行頭) — directly:
//!   `Question::LINE_HEAD_OPENING_BRACKET`'s `pattern-2` answer synthesizes a half em at
//!   Table 1's `(0, 1)` coordinate (`docs/decisions/line-head-opening-bracket.md`), and
//!   `pattern-1`/`pattern-3` correctly answer nothing there. `crate::compose` never queries
//!   that coordinate at a paragraph's own first line, and [`crate::compose::Paragraph`]'s own
//!   [`crate::compose::Paragraph::with_first_line_indent`] is a caller-declared amount with
//!   no reader of this Question at all. Wiring it would complete two of the three patterns,
//!   not all three: with the caller's own declared one-em indent in place (§3.1.5's own body
//!   states this indent "is assumed to be a one em space across all the patterns"), pattern 1
//!   composes to one em (the indent plus nothing) and pattern 2 to one and a half em (the
//!   indent plus the wrapped line head's own half em, reused at the first line), both
//!   complete in both halves — but pattern 3's own half-em first line *replaces* the ordinary
//!   one-em indent rather than adding to it, which no purely additive `InlineExtent` (the
//!   shape `with_first_line_indent` already commits to) can express. `Policy::BOOK` answers
//!   `pattern-3` (`docs/design/api-spine.md`: "Book practice: reduction Table 5, §3.1.5
//!   pattern 3, hanging punctuation"), so stated plainly rather than buried: under the
//!   default book preset, a bracket-initial paragraph's own first line keeps whatever indent
//!   the caller declared, until this slot is filled.
//!
//! Composition is consequently no longer set solid whenever a line needs help: greedy
//! composition still takes the last candidate whose *unadjusted* extent fits the measure,
//! or the first candidate at all when even that overflows, but the ladder — reduction,
//! then hanging, then expansion — now runs on whichever candidate was chosen, and only a
//! line the ladder cannot fit even once fully drained is reported through
//! [`ViolationKind::Overfull`] or [`ViolationKind::ExpansionExhausted`]. What remains
//! honestly short of §3.8's requirement (no ragged setting), for text this milestone can
//! actually compose, is the one slot above: a §C.3 relaxation mechanism this milestone
//! does not yet match. Neither of the two same-run facts beside it — §C.2 notes 6 through
//! 8 and 13's own breakability, real this pass, and §3.1.11 item 2(g)'s jukugo-ruby
//! same-run non-separation, still a slot — is a live §3.8 gap for text this milestone can
//! actually compose, and for the identical reason: [`crate::compose::compose`] carries no
//! [`jlreq_unit::Runs`] other than [`jlreq_unit::Runs::none`], so neither one's own answer
//! is ever handed anything to refuse (jukugo-ruby is M4-a). The difference between the two
//! is real but does not change that conclusion — `feasible::same_run_refusal` is filled and
//! directly reachable through the public [`Feasible::compute`] today, where item 2(g) has
//! no counterpart check to reach at all — because what "text this milestone can actually
//! compose" excludes is [`crate::compose::compose`]'s own pipeline specifically, and both
//! facts sit outside it the identical way. §3.1.5's own slot, above, is not a third §3.8 gap
//! at all, for a simpler reason than either: it is §3.5's own paragraph-first-line indent, a
//! fact fixed once at a paragraph's own construction and never touched by the ladder's own
//! reduce/hang/expand stages, so its absence has no bearing on this paragraph's own
//! accounting of what §3.8 still owes.
//!
//! §3.6's tab setting is named in neither list above, and not because it is absent: it is
//! [`crate::tab::tab_line`], a third real, complete case beside [`align`] — see this
//! file's own `# Status` for what it does and `crate::tab`'s own module doc for how. It
//! answers to neither list because it is not a rule table [`Feasible::compute`] or
//! `crate::ladder` reads (there is nothing "Wired, not slotted" to say about it beyond what
//! `crate::tab` already says of itself) and it fills no seam left unfilled by
//! `jlreq-inline` (there is no "Slots" entry to add for it either): §3.6 is its own self-
//! contained placement problem, stated and answered inside `crate::tab` alone.
//!
//! [`crate::compose::Search::Optimal`] — this round's own addition — is named in neither
//! list above either, for a reason different from either §3.6's or the reasons already
//! stated: it is not "Wired, not slotted" (the whole-paragraph dynamic program
//! `crate::compose::run_dp` runs, `Preference::compare` and `Demerits::add_sat`'s own
//! translation-invariance, is this crate's own new logic, not another crate's rule table
//! called through), and it is not a "Slots" entry either — a slot is an *unfilled* seam a
//! later phase completes, and `Optimal` is filled, real, and reachable through the public
//! [`crate::compose::compose`] today. Its one genuinely open design choice — what
//! `compose` does when a caller's own `tolerance` admits no complete arrangement at all —
//! is answered instead the way this crate answers every genuine silence: published as a
//! reading, `docs/decisions/tolerance-exhaustion.md`, wired into `compose_optimal` rather
//! than left as a seam for a later phase to fill.
//!
//! §3.5.4's widow threshold no longer sits beside `Optimal` as a named gap — a later round
//! closed it, the same way: `crate::compose::demerits_of`, the one cost function both
//! `compose_first_fit` and `evaluate_edge` call, now reads `Paragraph::with_widow_threshold`
//! on exactly a paragraph's own last line, adding a shortfall-proportional term to
//! `Demerits::structural` (`crate::objective`'s own field doc) and reporting a
//! `crate::compose::ViolationKind::Widow` naming `RuleId::WIDOW_ADJUSTMENT_OF_PARAGRAPHS`
//! when the arrangement a search finally settles on still falls short. `Search::Optimal`
//! genuinely steers away from a widow the same way it steers away from any other structural
//! cost — `structural` ranks first in both of `docs/decisions/adjustment-preference.md`'s
//! own orderings — while `Search::FirstFit` cannot: it commits to one candidate break per
//! line and never compares arrangements at all (`Search::FirstFit`'s own doc), so setting
//! the threshold never moves a break under `FirstFit`, only makes the shortfall of whichever
//! last line greedy composition already chose observable, through the identical violation.
//! This asymmetry is filled and real, not a slot, for the identical reason `Optimal` itself
//! is neither "Wired, not slotted" nor a "Slots" entry: the widow term is this crate's own
//! new logic reading a caller-declared field, not another crate's rule table called through,
//! and nothing about it remains unfilled.
//!
//! What is genuinely open is what JLReq's own silence left for kumihan to answer, not what
//! is left to implement: what counts as "a character" when the ladder can leave one hanging
//! past the measure, whether a paragraph that occupies a single line can have a widow at
//! all, whether the penalty for falling short should be flat or shortfall-proportional, and
//! what an unsatisfiable threshold means given ADR-0010's own refusal to leave `compose`
//! silent or force it to invent a forbidden break. All four are published,
//! `docs/decisions/widow-threshold.md`, the model `docs/decisions/tolerance-exhaustion.md`
//! already set for a genuine silence beside a filled search rather than an unfilled seam.
//!
//! §3.5.4 is `[[owned]]` at M3 in `docs/conformance-deferrals.toml`, no longer deferred:
//! ADR-0006's independently authored case phase (`crates/jlreq-conform/cases/3.5.4.json`)
//! measures Q1 and Q2 above directly, and Q4 through the same channel a non-empty `lines`
//! beside a real violation already gives; the entry's own `why` states honestly which of
//! the four readings above a case reaches and which two — the penalty's own shape, and the
//! hanging wrinkle — it does not, rather than letting the ledger's binary covered/deferred
//! model read as "fully measured" for a rule whose coverage is real but scoped.
//!
//! [ROADMAP.md]: https://github.com/P4suta/kumihan/blob/main/ROADMAP.md

#![no_std]

extern crate alloc;

mod align;
mod compose;
mod feasible;
mod ladder;
mod objective;
mod tab;

pub use crate::align::{Alignment, align};
pub use crate::compose::{
    ComposeError, Composition, Hanging, Line, Paragraph, Part, PullUp, Rewrite, Search, Trim,
    Violation, ViolationKind, compose,
};
pub use crate::feasible::{Candidate, CandidateIndex, Feasible, FeasibleBreak};
pub use crate::ladder::{Adjustment, Ladder, Site};
pub use crate::objective::{Badness, Deepest, Demerits, Fit, Preference};
pub use crate::tab::{TabKind, TabLine, TabStop, tab_line};
