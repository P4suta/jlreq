// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! This workspace, as one implementation of [`Compose`].
//!
//! The suite is written to be run against anyone's implementation, so the workspace's own
//! is an adapter like every other: it builds the `Text` a case's input describes, asks
//! `jlreq` the question the case asks, and reports `None` for a question this workspace
//! does not answer yet. Nothing here knows what a case expects.
//!
//! # What is answered today, and what is not
//!
//! All eight methods are wired to a real evaluator. `classify` asks `jlreq::resolve`;
//! `boundary` asks `jlreq::boundary`, which reads Tables 1 and 2 through
//! `jlreq_spacing::Adjacency`; `compose` asks `jlreq::compose` over the same tables, under
//! whichever of `jlreq-line`'s two searches the case's own `input.search` names —
//! `jlreq-line`'s greedy `Search::FirstFit` when it names none, the reading every case
//! published before that field existed already assumed, or `Search::Optimal` when it does
//! (`search_of`'s own doc); `align` asks `jlreq::align`, the same crate's
//! four single-line-alignment methods (§3.5.3, §3.7.3), added once the case format gained a
//! fourth `kind` to ask it with; `tab` asks `jlreq::tab_line`, §3.6's own placement
//! algorithm over the same crate's four tab kinds (§3.6.1, §3.6.2, §3.6.3), added once the
//! case format gained a fifth `kind` to ask it with; `feasible` asks
//! `jlreq::Feasible::compute`, §C.2's own kinsoku evaluator, added once the case format
//! gained a sixth `kind` to ask it with, and builds a real `jlreq_unit::Runs` overlay from a
//! case's own `constructs` (`overlay_of`, below) rather than the `Runs::none()` every one of
//! `classify`, `boundary`, `compose`, `align` and `tab` still passes; `lower` asks
//! `jlreq::lower` directly — not `jlreq_line` at all — added once the case format gained a
//! seventh `kind` to ask it with, and builds a real `jlreq::Constructs` and a real
//! `jlreq::Ruby` slice from a case's own `constructs.ruby` (`rubies_of`, below), which is
//! this method's own answer to the identical question `overlay_of` answers for `feasible`,
//! asked of a different layer's own input type; `place` asks `jlreq::place`, §3.3.5's own
//! positioning half, §3.3.6's own group-ruby geometry (since M4-a round 5) and §3.3.7's own
//! jukugo-ruby placement (since M4-a round 7) alike, added once the case format gained an
//! eighth `kind` to ask it with —
//! reusing `lower`'s own `rubies_of`-built `jlreq::Constructs` for an identical
//! `jlreq::lower` call, then deriving the line layout `place` positions each attachment
//! against from the case's own declared item advances and that same call's own forced
//! separations (`derived_placements`, below, and this method's own doc states the
//! derivation and its scope limit in full). None of the eight invents an answer no
//! evaluator produced, and a `None` from any of them is still exactly what it always was —
//! *not attempted* — for one of two reasons stated once here rather than as a list of case
//! ids.
//!
//! The first is the schedule. `jlreq-line`'s own `# Status` states its ladder —
//! `ladder::reduce`, `ladder::hang` and `ladder::expand` — as real: a line whose natural
//! extent overflows the measure is actually reduced, offered hanging punctuation, and
//! expanded, rather than reported set solid unconditionally. `crates/jlreq-conform/cases/`
//! publishes a growing set of `"kind": "compose"` cases against it — `3.5.1` and `3.8.1`
//! stay scoped to text whose interior boundaries carry no reducible or expandable
//! conditional space (`亜亜亜`, cl-19 against cl-19, a blank Table 1 cell), on purpose, so
//! that each exercises the control flow's *shape* without also depending on the ladder's own
//! arithmetic; `3.8.2`, `3.8.3`, `3.8.4`, `3.1.12`, `D`, `D.1` and `D.2` are the cases built
//! over a real, nonzero shortfall instead, each isolating one stage of that arithmetic
//! (`3.8.1.json`'s own rationale states the split and why it is drawn there). This method is
//! consequently wired and correct over what the corpus actually reaches, growing case by
//! case rather than covered once and left.
//!
//! The second is the layer, and it governs `classify`, `boundary`, `compose`, `align` and
//! `tab` alike — every method but `feasible`, `lower` and `place`. `classify` takes a text, an
//! ordinal and a policy, and takes no construct: a construct is a run over a stream rather
//! than a property of one item, so it belongs to `jlreq-inline` (ADR 0015). Where a case
//! declares a construct that runs over the very occurrence `classify` is asked about, the
//! answer this crate could give would be an answer to a different question — it would be
//! the class the occurrence has read as bare running text, which is not what the case
//! asked. Nine of the thirty classes are membership *in* a construct, five of them
//! enumerate no keys at all, and neither fact is reachable from an item, so the occurrence
//! is reported as not attempted.
//!
//! `boundary` inherits the identical limit rather than a milder one: the class either
//! neighbor of an adjacency resolves to is read by `jlreq_spacing::Adjacency`'s own
//! `class_of`, over the same construct-blind `resolve` `classify` calls, so an interior
//! boundary touching a construct-covered item is exactly as unanswerable, for exactly the
//! same reason, and is declined before `jlreq::boundary` is ever asked. `compose`, `align`
//! and `tab` all decline whenever a case declares any construct at all, for the coarser
//! version of the same fact: neither `jlreq_line::Paragraph`, `jlreq_line::align` nor
//! `jlreq_line::tab_line` has a way to carry one through (`jlreq-line`'s own `# Status`), so
//! every paragraph, run or tab line any of the three builds is `Runs::none()` — plain text
//! — and a construct-bearing case answered that way would be answered as a different input
//! than the one it describes.
//!
//! `feasible` was the first exception, and it is the reason M1 round 11 exists. `jlreq_line::
//! Feasible::compute` already takes a `jlreq_unit::Runs` parameter — real since
//! `crates/jlreq-line/src/feasible.rs`'s own same-run refusal was wired in
//! (`same_run_refusal`, that module's own `# Status`) — so a declared construct is not a
//! limit `Compose::feasible` inherits from `classify`, `boundary`, `compose`, `align` and
//! `tab`, its five siblings that do inherit it; it was the one input every sibling's own
//! limit kept unreachable by any case until then. `overlay_of` (below) turns a case's own
//! `constructs` into a real `jlreq_unit::Runs` overlay honestly, kind by kind, and
//! `Compose::feasible`'s own doc states what declines and what does not.
//!
//! `lower` is the second exception, and it is the reason this round exists. It is not a
//! milder version of `feasible`'s own exception: `feasible` still asks a question about a
//! break candidate, with `constructs` supplying the overlay that lets kinsoku's own
//! same-run refusal see it, so `jlreq_class::resolve`'s and `jlreq_line::Paragraph`'s own
//! construct-blindness is still the limit it is exempt from. `lower` is not exempt from
//! that limit; it is not subject to it at all, because it never asks `jlreq_class::resolve`
//! or any `jlreq_line` entry point anything — `jlreq::lower` (`jlreq_inline::lower`,
//! re-exported) is a construct-declaring layer's own function, taking a `jlreq::Constructs`
//! directly rather than a bare `Text`. `rubies_of` (below) turns a case's own declared
//! `constructs.ruby` into a real `jlreq::Ruby` slice, honestly, and declines the whole case
//! — never a partial answer — the moment any declared construct is not `ruby`, or a
//! declared `ruby` entry fails `jlreq::Ruby::new` (`RubyError`), or `jlreq::lower` itself
//! refuses the result (`LowerError`): every one of those is a case this method cannot
//! honestly convert, on the identical "malformed case, or a case this layer cannot yet
//! answer" standard `overlay_of`'s own doc states for `feasible`.
//!
//! `place` is the third exception, and it is the reason this round exists. It shares
//! `lower`'s own construct-declaring-layer exemption in full — it never asks
//! `jlreq_class::resolve` or `jlreq_line` anything either — and adds one fact of its own:
//! `jlreq::place` takes `items: Range<ItemIndex>` and `placements: &[InlineOffset]`, the
//! line layout `jlreq_line::Line` would ordinarily supply, and this method has no `Line` to
//! read one from — a `place` case declares no `measure` and no `candidates` (`cases.schema.
//! json`'s own `kind` description), so there is nothing for `jlreq_line::compose` or `align`
//! to build one over even if this crate's own construct-blindness limit did not already
//! forbid calling either. `derived_placements` (below) builds the two instead, from data
//! this method already has in hand: `items` is `0..input.items.len()` and `placements[k]`
//! is the sum of every earlier item's own declared advance plus every forced §3.3.8 rule 1
//! separation a prior `jlreq::lower` call resolved at a boundary before `k`. This is a
//! **derivation**, not a restatement of a caller-declared field, and it is faithful only
//! where every interior boundary of the declared base stream is Table 1 `blank` — solid, by
//! Table 1's own silence, with nothing conditional to omit — which is why every multi-item
//! `place` case the suite publishes states which coordinate it verified blank and against
//! which transcription (`crates/jlreq-conform/cases/3.3.5.json`'s and `3.3.6.json`'s own case
//! rationales).
//! A single-base-item fixture has no interior boundary at all, so the question is vacuous
//! for it and no such citation is needed. Honoring the forced separation specifically (as
//! opposed to advances alone) is what lets a fixture exercise §3.3.5(c)'s own oversized-ruby
//! geometry without also having to dodge §3.3.8 rule 1 — a construct's own reading may
//! genuinely be wider than its base while its neighbor is still forced solidly apart from it
//! by the identical rule, and `derived_placements` reads both facts rather than only one.
//! No case in the suite yet actually exercises the separations term of the sum (every
//! multi-item fixture across `3.3.5.json` and `3.3.6.json` is deliberately chosen so no
//! separation arises at all — the latter's own group-ruby base doubly so, since `jlreq_
//! inline::lower::lower_group` emits no `Separation` for any neighbor yet regardless of the
//! declared advances — and every other fixture has only one base item), so that half of this
//! derivation is adapter code no published case reaches yet, stated here rather than left for
//! a reader to assume is covered.

use std::num::NonZeroU16;

use jlreq::{
    Adjacency, Advance, Alignment, Annotation, AnnotationIndex, Attachment, Badness, ByteOffset,
    Candidate, CandidateIndex, ConditionalSpace, Construct, ConstructKind, ConstructRef,
    Constructs, Contribution, Direction, Em, Expansion, Feasible, Frame, InlineExtent,
    InlineOffset, Item, ItemIndex, Line, Lowered, Paragraph, PullUp, Reduction, Referent, Role,
    Ruby, RubyAlignment, RubyOverhang, RubyRun, RubyStyle, RuleId, RunId, Runs, Scale, ScaleId,
    Search, TabKind, TabStop, Text, TextError, Trim, align, lower, place, tab_line,
};
// The specification's own vocabulary comes from `jlreq-spec` rather than through the
// facade, which is the reason the crate graph gives this crate both edges: the suite has to
// reach the rule inventory and the policy space to report coverage without depending on the
// whole of the layout stack for it (`docs/design/api-spine.md`).
use jlreq_spec::{Choice, Policy, Question};

use crate::case::{
    CaseConstruct, CaseInput, CaseItem, CasePolicy, CaseRun, CaseSearch, CaseTabStop, Suite,
};
use crate::run::{
    CaseAttachment, CaseBoundary, CaseClass, CaseExpansion, CaseFeasible, CaseLine, CaseLower,
    CaseOutput, CasePlace, CasePullUp, CaseSpace, CaseTrim, Compose, Edge,
};

/// This workspace's implementation, as the suite measures it.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct Kumihan {
    /// The policy this run declares.
    policy: Policy,
}

impl Kumihan {
    /// The workspace under one policy.
    #[must_use]
    pub const fn new(policy: Policy) -> Self {
        Self { policy }
    }
}

impl Default for Kumihan {
    /// JLReq's own preference wherever it states one, which is the preset the library's own
    /// documentation is written against.
    fn default() -> Self {
        Self::new(Policy::JLREQ)
    }
}

impl Compose for Kumihan {
    fn name(&self) -> &'static str {
        "kumihan"
    }

    /// Every question of the generated policy space, answered as this run's policy answers
    /// it.
    ///
    /// The map is total over the questions that exist, which is what the selection rule
    /// reads. Where the policy space is smaller than `spec/derived/questions.tsv` — stage 2
    /// of the derivation emits the `Question` constants and has not run — the map is
    /// correspondingly smaller, and a case entry keyed on a question this workspace cannot
    /// yet answer applies to nothing. That is the honest reading: the entry names a knob
    /// this implementation does not have, so it is not the entry this implementation is
    /// measured against.
    fn declared_policy(&self) -> Option<CasePolicy> {
        Some(
            Question::ALL
                .iter()
                .map(|question| {
                    (
                        question.path().to_owned(),
                        Choice::name(self.policy.get(*question)).to_owned(),
                    )
                })
                .collect(),
        )
    }

    fn classify(&self, input: &CaseInput, item: usize) -> Option<CaseClass> {
        if input.construct_covers(item) {
            return None;
        }
        let stream = Stream::of(input).ok()?;
        let answer = jlreq::resolve(stream.text().ok()?, ordinal(item)?, self.policy)?;
        Some(CaseClass {
            class: answer.value().number(),
            rules: answer
                .why()
                .rules()
                .map(|rule| rule.address().to_string())
                .collect(),
        })
    }

    /// The spacing, breakability and placement at one boundary — interior, or at a line edge.
    ///
    /// `before` is the published format's own name for `jlreq_spacing::Adjacency::between`'s
    /// `before` parameter for an interior boundary (`edge: None`), and for
    /// `Adjacency::at_line_end`'s `last` parameter at the line end (`edge: Some(Edge::End)`):
    /// in both readings it is the item ordinal immediately *preceding* the boundary, not the
    /// ordinal of the item the boundary is "before" in the sense of coming ahead of it. A
    /// case naming `before: 4` and no `edge` in an eight-item stream asks about the boundary
    /// between item 4 and item 5, matching the library's own vocabulary exactly
    /// (`A.29/before-the-closing-bracket/no-line-break` is the case that pins this: its
    /// `before: 4` is the boundary immediately preceding the warichu closing bracket at item
    /// 5, and its own break candidate sits at that bracket's byte offset). At the line head
    /// (`edge: Some(Edge::Head)`) there is no preceding item to name, so `before` there reads
    /// as `Adjacency::at_line_head`'s `first` instead — the one item the boundary genuinely
    /// *is* before — because a line edge has exactly one real neighbor and `cases.schema.json`
    /// states which side it is on rather than asking for a second ordinal on the side that
    /// has none.
    ///
    /// The published format's own `boundary.edge` is what makes a line-edge answer reachable
    /// at all: without it, a rule whose only citation is a line-edge cell — Table 3's
    /// `(before: 5, after: line-edge)` cell among them — was unreachable by any case forever,
    /// because every `before` this method read was handed to `Adjacency::between` alone, and
    /// a `before` naming the stream's last item produced only the `None` that constructor
    /// answers for an out-of-range pair, never the line-end boundary `Adjacency::at_line_end`
    /// exists to build. `compose.rs`'s own `geometry_of` and `trailing_of` already call
    /// `Adjacency::at_line_end` for exactly this boundary; this method now does too.
    ///
    /// Declines when the one real neighbor `edge` names is construct-covered — or, for an
    /// interior boundary, when either one is — for the reason this module's own doc gives
    /// once: the class either side resolves to, without the run `jlreq-inline` would supply,
    /// is not the class the case's construct declares.
    fn boundary(
        &self,
        input: &CaseInput,
        before: usize,
        edge: Option<Edge>,
    ) -> Option<CaseBoundary> {
        let after_construct_covered =
            edge.is_none() && input.construct_covers(before.saturating_add(1));
        if input.construct_covers(before) || after_construct_covered {
            return None;
        }
        let stream = Stream::of(input).ok()?;
        let text = stream.text().ok()?;
        let before_index = ordinal(before)?;
        let direction = direction_of(input.direction.as_deref());
        let adjacency = match edge {
            None => Adjacency::between(text, Runs::none(), before_index, direction)?,
            Some(Edge::Head) => {
                Adjacency::at_line_head(text, Runs::none(), before_index, direction)
            },
            Some(Edge::End) => Adjacency::at_line_end(text, Runs::none(), before_index, direction),
        };
        let answer = jlreq::boundary(adjacency, self.policy);
        let rules = jlreq::rules_fired(adjacency, self.policy)
            .map(|rule| rule.address().to_string())
            .collect();
        Some(CaseBoundary::new(
            answer.spaces().map(case_space_of).collect(),
            answer.is_breakable(),
            answer.is_permitted(),
            Some(ruby_overhang_units(answer.ruby_overhang())),
            case_expansion_of(answer.expansion(), answer.expansion_rule()),
            rules,
        ))
    }

    /// The composed lines of a plain-text paragraph: `jlreq::compose`, over the case's own
    /// candidates, measure and search — `Search::FirstFit`, `jlreq-line`'s own greedy scan,
    /// when the case states no `search` at all (`search_of`'s own doc; every case published
    /// before this workspace could read `Search::Optimal` keeps asking exactly the question
    /// it always asked), or `Search::Optimal` when it names one.
    ///
    /// Declines when the case states no `measure`, a `measure`/`first_line_indent`/
    /// `head_indent`/`end_indent` this crate's own `InlineExtent` refuses, a `search` this
    /// crate's own vocabulary does not hold (`search_of`'s own doc), when a candidate's byte
    /// offset does not fit a `u32`, or when composition itself refuses the input — every one
    /// of those is a malformed case that would already fail `conform --check`, held to the
    /// same standard `Stream::of` documents for the base stream. Declines outright when the
    /// case declares any construct at all, for the reason this module's own doc gives once.
    ///
    /// `head_indent` and `end_indent` are read together, into one `with_indents` call,
    /// rather than each guarded by its own `if let`: `Paragraph::with_indents` takes both at
    /// once and defaults each to `InlineExtent::ZERO` inside `Paragraph::new`, so reading
    /// them independently — narrowing only when its own field is present — would silently
    /// drop `end_indent` whenever a case stated it without also stating `head_indent`, the
    /// one shape `3.5.2`'s own two-part assertion does not happen to exercise but a future
    /// case naming `end_indent` alone would. Neither field forces the other to be present:
    /// an absent one reads as `InlineExtent::ZERO`, `Paragraph::new`'s own default, so a
    /// case naming only one of the two narrows only that side.
    ///
    /// `widow_threshold` is not paired with either indent call and takes the plain `if let`
    /// guard instead: `Paragraph::with_widow_threshold` takes one field, not two, so there is
    /// no sibling for a case to state without it and no drop-on-partial-input hazard for a
    /// paired read to guard against — the hazard `head_indent`/`end_indent` share is a fact
    /// about `with_indents`' own two-argument shape, not a general rule every paragraph
    /// builder method obeys. A threshold this crate cannot hold as a `u16` — negative, or
    /// past `65535` — declines the whole case, the same "malformed case" standard every other
    /// out-of-range field on this object is already held to.
    fn compose(&self, input: &CaseInput) -> Option<CaseOutput> {
        if !input.constructs.is_empty() {
            return None;
        }
        let stream = Stream::of(input).ok()?;
        let text = stream.text().ok()?;
        let candidates = candidates_of(input)?;
        let measure = inline_extent(input.measure?)?;
        let search = search_of(input.search.as_ref())?;
        let direction = direction_of(input.direction.as_deref());
        let mut paragraph = Paragraph::new(text, &candidates, measure, direction);
        if let Some(indent) = input.first_line_indent {
            paragraph = paragraph.with_first_line_indent(inline_extent(indent)?);
        }
        if input.head_indent.is_some() || input.end_indent.is_some() {
            let head = match input.head_indent {
                Some(value) => inline_extent(value)?,
                None => InlineExtent::ZERO,
            };
            let end = match input.end_indent {
                Some(value) => inline_extent(value)?,
                None => InlineExtent::ZERO,
            };
            paragraph = paragraph.with_indents(head, end);
        }
        if let Some(threshold) = input.widow_threshold {
            paragraph = paragraph.with_widow_threshold(u16::try_from(threshold).ok()?);
        }
        let composition = jlreq::compose(paragraph, self.policy, search).ok()?;
        Some(CaseOutput::new(
            composition.lines().iter().map(case_line_of).collect(),
            composition
                .violations()
                .iter()
                .map(|violation| violation.rule.address().to_string())
                .collect(),
            composition
                .rules_fired()
                .map(|rule| rule.address().to_string())
                .collect(),
        ))
    }

    /// The single line `jlreq_line::align` produces for a run shorter than a caller-stated
    /// target — §3.5.3's own methods, and §3.7.3's jidori among them.
    ///
    /// `measure` is reused as `align`'s own `target` rather than a dedicated field: the case
    /// format already reads `measure` as "the line length, in the caller's own unit"
    /// (`cases.schema.json`'s own description), and `align`'s `target` is the identical
    /// quantity read for a run rather than a paragraph, so a second field would restate what
    /// `measure` already means. `candidates` is consequently never read here: `align` never
    /// breaks, so a case asking this question supplies none (`cases.schema.json`'s own
    /// `kind` description states both halves of the reuse).
    ///
    /// Declines when the case states no `measure`, no `alignment`, or an `alignment`
    /// `alignment_of` does not recognize, on the same "malformed case" standard `compose`
    /// holds its own required fields to, and outright when the case declares any construct,
    /// for the reason this module's own doc gives once. `align` returns exactly one `Line`,
    /// so the answer is a single-element `lines` vector, and `violations` and the fired-rule
    /// list are both empty: `jlreq_line::align` reports neither (its own `# What this is
    /// not` — no `Ladder` stage ever runs), so there is nothing to translate into either.
    fn align(&self, input: &CaseInput) -> Option<CaseOutput> {
        if !input.constructs.is_empty() {
            return None;
        }
        let stream = Stream::of(input).ok()?;
        let text = stream.text().ok()?;
        let target = inline_extent(input.measure?)?;
        let alignment = alignment_of(input.alignment.as_deref())?;
        let direction = direction_of(input.direction.as_deref());
        let line = align(
            text,
            Runs::none(),
            target,
            alignment,
            self.policy,
            direction,
        )
        .ok()?;
        Some(CaseOutput::new(
            vec![case_line_of(&line)],
            Vec::new(),
            Vec::new(),
        ))
    }

    /// The runs `jlreq_line::tab_line` places for one caller-declared tab line — §3.6.1's
    /// own vocabulary (tab positions, tab types, target characters), §3.6.2's four kinds,
    /// and §3.6.3's own placement algorithm.
    ///
    /// `tab_starts` and `tab_stops` are read directly rather than through `measure`: a
    /// `tab` case never asks for a caller-stated target length at all
    /// (`cases.schema.json`'s own `kind` description states this asymmetry directly), so
    /// unlike `compose` and `align` there is no existing length field to reuse or narrow.
    ///
    /// Declines when the case states no `tab_starts`, no `tab_stops`, or a `tab_stops`
    /// entry whose `position` or `kind` this crate's own vocabulary does not hold —
    /// `starts_of`'s and `stops_of`'s own `None` returns, on the same "malformed case"
    /// standard `alignment_of` already holds `align`'s own `alignment` field to — and
    /// outright when the case declares any construct, for the reason this module's own doc
    /// gives once. `tab_line`'s own `Err(ComposeError::InsufficientTabStops)` and
    /// `Err(ComposeError::OutOfRange)` both decline the same way `compose`'s and `align`'s
    /// own `Result` already do (`.ok()?`): `docs/conformance-deferrals.toml`'s own `3.6.1`
    /// entry states why no case in this suite asks for the first of those two refusals
    /// directly — the published case format has no field for stating that an input is
    /// expected to be refused (`docs/design/conformance.md`'s own unmet bullet on this).
    ///
    /// `tab_line` returns one [`Line`] per placed run and never a violation or a fired
    /// rule (`jlreq_line::tab::TabLine`'s own doc: no ladder stage ever drains one), so
    /// `violations` and the fired-rule list are both empty here exactly as they already are
    /// for `align`. `TabLine::deferred` — §3.6.3(d)'s own runs left homeless on this line —
    /// is read by nothing here: `CaseOutput` carries no channel for it, a design decision
    /// this round makes deliberately rather than by oversight (`docs/conformance-
    /// deferrals.toml`'s own `3.6.3` entry states the scope this leaves uncovered).
    fn tab(&self, input: &CaseInput) -> Option<CaseOutput> {
        if !input.constructs.is_empty() {
            return None;
        }
        let stream = Stream::of(input).ok()?;
        let text = stream.text().ok()?;
        let starts = starts_of(input)?;
        let stops = stops_of(input)?;
        let direction = direction_of(input.direction.as_deref());
        let line = tab_line(text, Runs::none(), &starts, &stops, direction, self.policy).ok()?;
        Some(CaseOutput::new(
            line.placed().iter().map(case_line_of).collect(),
            Vec::new(),
            Vec::new(),
        ))
    }

    /// Which of one caller-declared break candidate kinsoku leaves standing, and which rule
    /// refused it when it does not: `jlreq::Feasible::compute`'s own answer for the
    /// `candidate`-th entry of `input.candidates`.
    ///
    /// This is the one method of the six that builds a real `Runs` overlay rather than
    /// `Runs::none()`: `overlay_of` turns `input.constructs` into one slot per item of the
    /// base stream, honestly, kind by kind — its own doc states which of the schema's nine
    /// construct arrays convert and why the other six decline — and the overlay, the `Text`
    /// it is read against, and the `Feasible` `Feasible::compute` returns all live inside
    /// this method's own stack frame; only the two facts an answer needs, `breakable` and
    /// the rule that decided it, ever escape it.
    ///
    /// Declines when the case states no `Stream::of`-buildable base stream, no `candidates`
    /// at all, no candidate at the stated ordinal, when `overlay_of` cannot honestly convert
    /// a declared construct, or when the overlay it does build is one `Runs::new` itself
    /// refuses (a `RunsError`) — every one of those on the same "malformed case, or a case
    /// this layer cannot yet answer" standard the other five methods already hold their own
    /// inputs to.
    ///
    /// `Feasible::compute` silently passes a candidate over — reports it in neither
    /// `breaks()` nor `rejected()` — when its own byte offset names no item boundary this
    /// stream's own segmentation carries (`Feasible::compute`'s own doc: "kinsoku has no
    /// adjacency to evaluate there"). This method declines the whole case in that state
    /// rather than answering `breakable: false`, which would misreport a fact this
    /// evaluator never decided, and rather than inventing a third schema state for it:
    /// `cases.schema.json`'s own `feasible` `$def` carries no field for "passed over"
    /// because the trait's own `Option` already carries that state, the identical reading
    /// every other decline on this trait already gets.
    fn feasible(&self, input: &CaseInput, candidate: usize) -> Option<CaseFeasible> {
        let stream = Stream::of(input).ok()?;
        let text = stream.text().ok()?;
        let candidates = candidates_of(input)?;
        let index = CandidateIndex::new(u32::try_from(candidate).ok()?);
        let slots = overlay_of(input, input.items.len())?;
        let runs = Runs::new(&slots).ok()?;
        let direction = direction_of(input.direction.as_deref());
        let answer = Feasible::compute(text, runs, &candidates, self.policy, direction);
        if let Some(found) = answer
            .breaks()
            .iter()
            .find(|found| found.candidate() == index)
        {
            return Some(CaseFeasible::new(
                true,
                found
                    .why()
                    .rules()
                    .map(|rule| rule.address().to_string())
                    .collect(),
            ));
        }
        let &(_, rule) = answer
            .rejected()
            .iter()
            .find(|&&(rejected, _)| rejected == index)?;
        Some(CaseFeasible::new(false, vec![rule.address().to_string()]))
    }

    /// What `jlreq_inline::lower` resolved for one declared ruby construct.
    ///
    /// `rubies_of` (below) is the one place in this crate that builds a real `jlreq::Ruby`
    /// slice — the identical shape `overlay_of` holds for the one real `jlreq_unit::Runs`
    /// overlay `feasible` builds — and `construct` selects which of the resulting slice's
    /// entries this answer is about, by the identical position `input.constructs.ruby`
    /// itself declares it at (`rubies_of`'s own doc states why the two orderings agree).
    ///
    /// Declines when the case declares any construct that is not `ruby`, when `rubies_of`
    /// cannot honestly convert a declared `ruby` entry (an absent `style`, a `runs` pairing
    /// naming a stream or a range `jlreq::Ruby::new` refuses), when `construct` names no
    /// entry of the resulting slice, or when `jlreq::lower` itself refuses the whole
    /// declaration (`LowerError`) — every one of those on the same "malformed case, or a
    /// case this layer cannot yet answer" standard `feasible`'s own decline conditions
    /// already state.
    fn lower(&self, input: &CaseInput, construct: usize) -> Option<CaseLower> {
        if input.constructs.iter().any(|entry| entry.kind != "ruby") {
            return None;
        }
        let stream = Stream::of(input).ok()?;
        let text = stream.text().ok()?;
        let annotation_streams = annotation_streams_of(input)?;
        let annotations = annotations_of(input, &annotation_streams)?;
        let ruby_runs = ruby_runs_of(input)?;
        let rubies = rubies_of(input, text, &annotations, &ruby_runs)?;
        let target = input.constructs.get(construct)?;
        let target_kind = construct_kind_of(target)?;
        let target_ref = ConstructRef::new(target_kind, construct_ordinal(construct));

        let constructs = Constructs::over(text).with_ruby(&rubies);
        let direction = direction_of(input.direction.as_deref());
        let mut scratch = Lowered::new();
        let contribution = lower(&constructs, self.policy, direction, &mut scratch).ok()?;

        let runs: Vec<Option<u32>> = (0..input.items.len())
            .map(|item| {
                let index = ordinal(item)?;
                let run = contribution.runs().of(index)?;
                Some(u32::from(run.run().get().get()))
            })
            .collect();
        let separations = contribution
            .separations()
            .iter()
            .map(|separation| {
                let after = usize::try_from(separation.after().get()).unwrap_or(usize::MAX);
                (after, i64::from(separation.least().units()))
            })
            .collect();
        let alignment = contribution.alignment_of(target_ref).map(alignment_name);
        let alignment_discouraged = contribution.alignment_discouraged(target_ref);
        let rules = contribution
            .rules_fired()
            .map(|rule| rule.address().to_string())
            .collect();
        Some(CaseLower::new(
            runs,
            separations,
            alignment,
            alignment_discouraged,
            rules,
        ))
    }

    /// What `jlreq_inline::place` computed for the case's own whole declared `Constructs`,
    /// positioned against a line layout this method derives rather than accepts as a
    /// further caller-declared field — this module's own doc states the derivation, its
    /// honesty requirement and its scope limit in full.
    ///
    /// Reuses `lower`'s own front half nearly verbatim — `Stream::of`, `annotation_streams_
    /// of`, `annotations_of`, `ruby_runs_of`, `rubies_of`, `Constructs::over(text).
    /// with_ruby(&rubies)`, `direction_of`, then a `jlreq::lower` call — because `place`
    /// needs the identical `Contribution` `lower` itself needs: `jlreq::place` reads the
    /// alignment a prior `jlreq::lower` call already resolved rather than re-deriving it
    /// (`jlreq_inline::place`'s own module doc). That call writes into its own `Lowered`
    /// scratch buffer, and `jlreq::place` writes into a **second** one — never the first,
    /// which `contribution` still borrows from — for the reason `jlreq_inline::place`'s own
    /// module doc states in full ("Two `Lowered` buffers, not one"): the borrow checker
    /// refuses a second mutable borrow of a buffer a live `&Contribution<'_>` already reads.
    /// `items` is always `ItemIndex::new(0)..ItemIndex::new(input.items.len())` — a `place`
    /// case has no notion of "this line" narrower than its own whole declared base stream,
    /// unlike a real caller composing many lines from one paragraph — which is exactly what
    /// makes `derived_placements`' own `placements[k]` and the item at absolute ordinal `k`
    /// coincide here (`derived_placements`'s own doc).
    ///
    /// Declines on the identical "malformed case, or a case this layer cannot yet answer"
    /// standard `lower`'s own decline conditions already state: any declared construct that
    /// is not `ruby`, a `rubies_of` that cannot honestly convert, a `LowerError`, an item
    /// count `ItemIndex` cannot hold, or a `derived_placements` that cannot honestly build
    /// the line layout `jlreq::place` needs.
    fn place(&self, input: &CaseInput) -> Option<CasePlace> {
        if input.constructs.iter().any(|entry| entry.kind != "ruby") {
            return None;
        }
        let stream = Stream::of(input).ok()?;
        let text = stream.text().ok()?;
        let annotation_streams = annotation_streams_of(input)?;
        let annotations = annotations_of(input, &annotation_streams)?;
        let ruby_runs = ruby_runs_of(input)?;
        let rubies = rubies_of(input, text, &annotations, &ruby_runs)?;

        let constructs = Constructs::over(text).with_ruby(&rubies);
        let direction = direction_of(input.direction.as_deref());
        let mut lower_scratch = Lowered::new();
        let contribution = lower(&constructs, self.policy, direction, &mut lower_scratch).ok()?;

        let item_count = u32::try_from(input.items.len()).ok()?;
        let items = ItemIndex::new(0)..ItemIndex::new(item_count);
        let placements = derived_placements(input, &contribution)?;

        let mut place_scratch = Lowered::new();
        let attachments = place(
            &constructs,
            &contribution,
            items,
            &placements,
            self.policy,
            &mut place_scratch,
        );
        Some(CasePlace::new(
            attachments
                .attachments()
                .iter()
                .copied()
                .map(case_attachment_of)
                .collect(),
            attachments
                .declined()
                .map(|construct_ref| usize::from(construct_ref.ordinal()))
                .collect(),
        ))
    }
}

/// The `jlreq_unit::Runs` overlay one case's own declared `constructs` describe, one slot
/// per item of the base stream — the one place in this crate that ever builds an overlay
/// that is not `Runs::none()` (this module's own doc, above). `None` when any declared
/// construct cannot be honestly converted, which is `Compose::feasible`'s own signal to
/// decline the whole case rather than answer a different input than the one it describes.
///
/// # Total over the schema's nine construct arrays, wired for three
///
/// `ornaments` converts to `ConstructKind::Ornamented`; `tate_chu_yoko` converts to
/// `ConstructKind::TateChuYoko`; `ruby` converts to `ConstructKind::NonJukugoRuby` for
/// `style: "mono"` or `"group"` and to `ConstructKind::JukugoRuby` for `style: "jukugo"` —
/// every one of the three over the construct's own declared range, unmodified
/// (`construct_kind_of`, below). Every one of the other six declines, each for a reason
/// named once here rather than discovered from a silent `None`:
///
/// - `ruby` with no declared `style` — the schema makes it optional
///   (`cases.schema.json`'s own `ruby` `$def`) — cannot choose between the two ruby kinds
///   above; guessing either would answer a construct the case did not declare.
/// - `emphasis` (圏点) has no `ConstructKind` variant at all: no §C.2 note reaches
///   emphasis-dot same-run breakability, so this workspace's own vocabulary has nowhere to
///   send it. `jidori` (字取り) is the identical absence for the identical reason — no
///   §C.2 note reaches it either, and neither is named among `same_run_refusal`'s own four
///   governed kinds (`crates/jlreq-line/src/feasible.rs`'s own doc on that function).
/// - `formulae` would need a `ConstructKind::MathFormula(FormulaSetting)`, and the schema's
///   own `formulae` array pins no shape a caller could state `InLine` or `IndependentLine`
///   through; nothing here guesses the setting.
/// - `warichu`, `furiwake` and `reference_marks` decline on a subtler ground than the
///   three above: `cases.schema.json`'s own `constructs` `$def` pins an interior shape for
///   `ruby` and `emphasis` alone (`crates/jlreq-conform/src/case.rs`'s own `read_constructs`
///   states this directly), so a caller-declared range under any of the other seven keys is
///   read generically, off whichever of `base`, `items` or `mark` the entry happens to
///   carry, with no schema commitment that the span means what the matching
///   `ConstructKind` variant means. A `warichu` entry's declared range plausibly includes
///   its own cl-28/cl-29 delimiters, while `ConstructKind::WarichuInterior` is, by name,
///   the interior alone; nothing in this format states whether the two coincide, so
///   converting either would risk answering a span the case never declared.
///   `ConstructKind::Furiwake` and `ConstructKind::ReferenceMark` are real variants with the
///   identical unpinned-shape problem, so they decline on the same ground rather than a
///   different one.
///
/// # `ruby.runs` is a declared slot this function does not read
///
/// Every slot's `group` is `None`: §C.2#8's own level below the run needs a caller-declared
/// `jlreq_unit::GroupId`, and `overlay_of` never builds one. `cases.schema.json`'s own `ruby`
/// `$def` still requires `annotation` and `runs` of every declared `ruby` construct — the
/// base/annotation pairing the schema reserves for exactly this note ("§B.2 notes 9 through
/// 11 and §C.2 notes 6 through 8 all turn on whether two neighbors are in the same run") —
/// and this function leaves both unread on purpose rather than by omission, in the "Slots"
/// sense `crates/jlreq-line/src/lib.rs`'s own module doc names: a seam a later,
/// independently authored phase fills, named by the address it answers, not a gap this round
/// left behind for a future one to close by accident. Three facts settle it, not one missing
/// line of code.
///
/// First, a declared `GroupId` changes exactly one downstream answer: `same_run_refusal`'s
/// own `ConstructKind::JukugoRuby` arm, the only place this workspace's own vocabulary reads
/// `jlreq_unit::Construct::group` at all. Reading `runs` here would move that one branch and
/// nothing else. Second, §C.2#8's own group is one base character and its own accompanying
/// reading, never a span across two: §3.3.7's own body states the boundary in so many words —
/// "Jukugo-ruby can be split into two lines at the boundary of each unit of ruby text
/// attached to one ideographic character (cl-19)" — and §3.1.10 item 8's own Note says the
/// identical thing from the spacing side, that "a group of ruby characters is attached to
/// each base character" and "a line break should not occur between ruby characters related
/// to a given base character." So two adjacent base characters of one jukugo-ruby complex
/// are never one group by either sentence's own account, whatever a caller-declared `runs`
/// pairing might say; the equal-group refusal `same_run_refusal` guards is a fact about a
/// base character and its own ruby, not about two neighboring base characters. Third, and
/// independently sufficient on its own: `Feasible::compute` sees the base item stream alone,
/// and a jukugo-ruby run's accompanying reading is a nested `Segment`
/// `jlreq_inline::Contribution` would place — the crate graph gives `jlreq-line` no edge to
/// `jlreq-inline` (`same_run_refusal`'s own doc, in full) — so the level §C.2#8's third
/// sentence is actually about, ruby-to-ruby indivisibility, is not a level this crate's own
/// item stream can reach regardless of what a case declares in `runs`.
///
/// `docs/decisions/jukugo-ruby-unset-group.md` already reads an absent group as permitted
/// rather than refused for the identical reason; this paragraph states why building one from
/// `runs` would not change that answer today, rather than leaving a reader to wonder why a
/// field the schema requires goes unread. What would change it: once the crate graph gives
/// `jlreq-line` an edge to `jlreq-inline` (M4-a) and a real ruby-text run can reach
/// `same_run_refusal` tagged `ConstructKind::JukugoRuby` alongside its own base character,
/// `runs`' own base/annotation pairing becomes the caller-declared fact that settles the
/// question from data rather than from adjudication. Until then, every case that declares
/// a `ruby` construct states `annotation` and `runs` for the format's own completeness and
/// for that later reader, not because reading either one here would move any answer this
/// crate gives.
///
/// # Why a whole case declines rather than a partial answer
///
/// A construct's own `ranges` may name several disjoint spans (`CaseConstruct::ranges` is a
/// `Vec`, one entry per `base`/`items`/`mark` key the JSON entry happens to state), while
/// one `RunId` names exactly one contiguous block (`Runs::new`'s own doc). A construct
/// naming more than one range has no single block to place, so this function declines
/// rather than flattening the ranges together or silently taking the first — either would
/// build an overlay answering a span the case never declared. And because one declined
/// construct leaves a gap in the overlay this function has no honest way to fill (leaving
/// those items `None` would misreport them as plain text the case never said they were),
/// one inconvertible construct anywhere in `input.constructs` fails the whole conversion,
/// exactly as `Runs::new`'s own validation failing anywhere fails the whole overlay.
fn overlay_of(input: &CaseInput, item_count: usize) -> Option<Vec<Option<Construct>>> {
    let mut slots: Vec<Option<Construct>> = vec![None; item_count];
    for (ordinal, construct) in input.constructs.iter().enumerate() {
        let kind = construct_kind_of(construct)?;
        if construct.ranges.len() != 1 {
            return None;
        }
        let (first, past) = construct.ranges[0];
        let id = u16::try_from(ordinal).ok()?.checked_add(1)?;
        let run = RunId::new(NonZeroU16::new(id)?);
        for slot in slots.get_mut(first..past)? {
            if slot.is_some() {
                return None;
            }
            *slot = Some(Construct::new(kind, run, None));
        }
    }
    Some(slots)
}

/// The `jlreq_unit::ConstructKind` one declared construct converts to, or `None` when this
/// crate cannot honestly say — `overlay_of`'s own doc states the reason for every kind that
/// declines, in full, rather than repeated here as a comment with no code behind it.
fn construct_kind_of(construct: &CaseConstruct) -> Option<ConstructKind> {
    match construct.kind.as_str() {
        "ornaments" => Some(ConstructKind::Ornamented),
        "tate_chu_yoko" => Some(ConstructKind::TateChuYoko),
        "ruby" => match construct.style.as_deref() {
            Some("mono" | "group") => Some(ConstructKind::NonJukugoRuby),
            Some("jukugo") => Some(ConstructKind::JukugoRuby),
            _ => None,
        },
        _ => None,
    }
}

/// Every annotation stream a `lower` case declares, read into the library's own item and
/// scale vocabulary but not yet built into a `jlreq_class::Annotation` — the first of two
/// phases `annotations_of` (below) needs, staged the same way `Stream::of`'s own `items` and
/// `scales` fields are: an `Annotation` borrows its items and scales, so the owned buffer
/// they are read into has to outlive it, and a temporary built and dropped inside a single
/// expression cannot.
fn annotation_streams_of(input: &CaseInput) -> Option<Vec<(Vec<Item>, Vec<Scale>)>> {
    input
        .annotations
        .iter()
        .map(|stream| {
            let items = stream
                .items
                .iter()
                .map(item_of)
                .collect::<Result<Vec<_>, _>>()
                .ok()?;
            let scales = stream
                .scales
                .iter()
                .map(|scale| {
                    let inline = advance(scale.inline_em).ok()?;
                    let block = advance(scale.block_em).ok()?;
                    Scale::new(inline, block)
                })
                .collect::<Option<Vec<_>>>()?;
            Some((items, scales))
        })
        .collect()
}

/// The `jlreq_class::Annotation`s a `lower` case declares, borrowing each stream's own text
/// out of `input` directly and its items and scales out of `streams`
/// (`annotation_streams_of`, above) — the second of the two phases, now that both buffers
/// outlive the borrow.
fn annotations_of<'a>(
    input: &'a CaseInput,
    streams: &'a [(Vec<Item>, Vec<Scale>)],
) -> Option<Vec<Annotation<'a>>> {
    input
        .annotations
        .iter()
        .zip(streams)
        .map(|(stream, (items, scales))| Annotation::new(&stream.text, items, scales).ok())
        .collect()
}

/// Every declared `ruby` construct's own `runs` pairing, read into `jlreq::RubyRun`, one
/// inner `Vec` per construct in `input.constructs`' own order — staged for the identical
/// reason `annotation_streams_of` is: `jlreq::Ruby::new` borrows the `RubyRun` slice it is
/// given, so the buffer has to outlive the `Ruby` `rubies_of` (below) builds from it.
fn ruby_runs_of(input: &CaseInput) -> Option<Vec<Vec<RubyRun>>> {
    input
        .constructs
        .iter()
        .map(|construct| {
            construct
                .runs
                .iter()
                .copied()
                .map(ruby_run_of)
                .collect::<Option<Vec<_>>>()
        })
        .collect()
}

/// One declared run pairing, in the library's own vocabulary.
fn ruby_run_of(run: CaseRun) -> Option<RubyRun> {
    let base = ordinal(run.base.0)?..ordinal(run.base.1)?;
    let annotation = annotation_ordinal(run.annotation.0)?..annotation_ordinal(run.annotation.1)?;
    Some(RubyRun::new(base, annotation))
}

/// The `jlreq::Ruby` slice a `lower` case's own declared `constructs.ruby` describe, in
/// declaration order — the one place in this crate that ever builds a real `jlreq::Ruby`,
/// the identical shape `overlay_of`'s own doc holds for the one real `jlreq_unit::Runs`
/// overlay `feasible` builds. `Compose::lower`'s own `construct` parameter indexes this
/// slice by the identical position `input.constructs` itself declares an entry at: the
/// whole case already declines when any declared construct is not `ruby` (`Compose::lower`'s
/// own guard), so this slice's own ordinals and `input.constructs`' own ordinals agree
/// one-for-one, with no filtering step between them to make the two orderings drift.
///
/// `None` when any declared entry cannot be honestly converted: an absent `style` — the
/// schema makes it optional, and nothing here may guess between the three — an `annotation`
/// ordinal naming no declared stream, or a base range or `runs` pairing `jlreq::Ruby::new`
/// itself refuses (`RubyError`).
fn rubies_of<'a>(
    input: &'a CaseInput,
    text: Text<'a>,
    annotations: &'a [Annotation<'a>],
    runs: &'a [Vec<RubyRun>],
) -> Option<Vec<Ruby<'a>>> {
    input
        .constructs
        .iter()
        .zip(runs)
        .map(|(construct, runs)| ruby_of(construct, text, annotations, runs))
        .collect()
}

/// One declared `ruby` construct, converted.
fn ruby_of<'a>(
    construct: &CaseConstruct,
    text: Text<'a>,
    annotations: &'a [Annotation<'a>],
    runs: &'a [RubyRun],
) -> Option<Ruby<'a>> {
    let style = ruby_style_of(construct.style.as_deref())?;
    let annotation = *annotations.get(construct.annotation?)?;
    let &(first, past) = construct.ranges.first()?;
    let base = ordinal(first)?..ordinal(past)?;
    Ruby::new(text, base, annotation, runs, style).ok()
}

/// The schema's own spelling of a ruby style, read into `jlreq::RubyStyle`. `Option` rather
/// than a fallback variant, `alignment_of`'s own reasoning below applied here too: no one of
/// the three styles is "more common" for an absent or misspelled value to fall back to.
fn ruby_style_of(style: Option<&str>) -> Option<RubyStyle> {
    match style {
        Some("mono") => Some(RubyStyle::MonoRuby),
        Some("group") => Some(RubyStyle::GroupRuby),
        Some("jukugo") => Some(RubyStyle::JukugoRuby),
        _ => None,
    }
}

/// The schema's own spelling of a `RubyAlignment`, the reverse of the schema's own reading
/// direction every other `_of` function in this module takes.
fn alignment_name(alignment: RubyAlignment) -> String {
    match alignment {
        RubyAlignment::Katatsuki => "katatsuki",
        // `RubyAlignment::Nakatsuki`, and — `RubyAlignment` is `#[non_exhaustive]`
        // (ADR-0012) — any future variant this workspace's own `jlreq-inline` has not been
        // asked to add. Read as nakatsuki, the same conservative reading `case_space_of`'s
        // own unrecognized-reduction fallback already takes.
        _ => "nakatsuki",
    }
    .to_owned()
}

/// `ordinal`, saturated into the `u16` a `jlreq::ConstructRef` holds — the identical
/// saturating cast `jlreq_inline::lower`'s own private `ordinal_of` performs, so the
/// `ConstructRef` this method reconstructs names the identical ordinal `lower` itself
/// allocated one for.
fn construct_ordinal(ordinal: usize) -> u16 {
    u16::try_from(ordinal).unwrap_or(u16::MAX)
}

/// One annotation-stream ordinal in the library's own vocabulary.
fn annotation_ordinal(index: usize) -> Option<AnnotationIndex> {
    u32::try_from(index).ok().map(AnnotationIndex::new)
}

/// The line layout a `place` case's own declared base stream derives, indexed exactly as
/// `jlreq_line::Line::placements` documents its own return — one entry per item of `input.
/// items`, in order — which is what lets `Compose::place` hand it to `jlreq::place` alongside
/// `ItemIndex::new(0)..ItemIndex::new(input.items.len())` and have the two coincide (this
/// module's own doc states the derivation, its honesty requirement and its scope limit in
/// full; `Compose::place`'s own doc states why the two ranges coincide here specifically
/// rather than in general).
///
/// `placements[k]` is the sum of every earlier item's own declared `advance`, plus the sum of
/// `least()` over every `contribution.separations()` entry whose `after()` names an item
/// before `k` — `jlreq_unit::Separation::after`'s own "the item the forced space follows"
/// reading (`crates/jlreq-unit/src/seam.rs`), so an entry with `after() == j` first applies
/// to `placements[j + 1]` and every `placements[k]` after it. `None` when an accumulated
/// offset overflows what `InlineOffset` can hold — the same "malformed case" standard every
/// other length this crate reads is already held to.
fn derived_placements(
    input: &CaseInput,
    contribution: &Contribution<'_>,
) -> Option<Vec<InlineOffset>> {
    let mut placements = Vec::with_capacity(input.items.len());
    let mut cursor: i64 = 0;
    for (k, item) in input.items.iter().enumerate() {
        let extra: i64 = contribution
            .separations()
            .iter()
            .filter(|separation| {
                usize::try_from(separation.after().get()).unwrap_or(usize::MAX) < k
            })
            .map(|separation| i64::from(separation.least().units()))
            .sum();
        let offset = cursor.checked_add(extra)?;
        placements.push(InlineOffset::new(i32::try_from(offset).ok()?)?);
        cursor = cursor.saturating_add(item.advance);
    }
    Some(placements)
}

/// One placed annotation, translated into the case format.
fn case_attachment_of(attachment: Attachment) -> CaseAttachment {
    CaseAttachment::new(
        i64::from(attachment.inline().units()),
        attachment
            .item()
            .map(|index| usize::try_from(index.get()).unwrap_or(usize::MAX)),
    )
}

/// The `tab_starts` a `tab` case declares, in the library's own vocabulary. `None` when the
/// case states none at all — the same "malformed case" reading `Compose::compose` already
/// gives an absent `measure`.
fn starts_of(input: &CaseInput) -> Option<Vec<ItemIndex>> {
    if input.tab_starts.is_empty() {
        return None;
    }
    input.tab_starts.iter().copied().map(ordinal).collect()
}

/// The `tab_stops` a `tab` case declares, in the library's own vocabulary.
fn stops_of(input: &CaseInput) -> Option<Vec<TabStop>> {
    if input.tab_stops.is_empty() {
        return None;
    }
    input.tab_stops.iter().map(tab_stop_of).collect()
}

/// One declared tab stop, in the library's own vocabulary.
fn tab_stop_of(stop: &CaseTabStop) -> Option<TabStop> {
    let position = inline_extent(stop.position)?;
    let kind = tab_kind_of(Some(stop.kind.as_str()), stop.at)?;
    Some(TabStop::new(position, kind))
}

/// The schema's own spelling of a tab kind, read into `jlreq_line::TabKind`.
///
/// `Option` rather than a fallback variant, mirroring `alignment_of`'s own reasoning
/// directly above: no one of the four kinds is "more common" for an absent or misspelled
/// value to fall back to, every candidate fallback would silently answer a different `tab`
/// case than the one asked, and this declines instead — the same "not attempted"
/// `Compose::tab` reports for it. `kind: "character"` alone reads `at`
/// (`TabKind::Character`'s own occurrence); the other three ignore it, the same reading a
/// `tab_stop`'s own unused fields already get from `cases.schema.json`.
fn tab_kind_of(kind: Option<&str>, at: Option<usize>) -> Option<TabKind> {
    match kind {
        Some("start") => Some(TabKind::Start),
        Some("end") => Some(TabKind::End),
        Some("centered") => Some(TabKind::Centered),
        Some("character") => Some(TabKind::Character { at: ordinal(at?)? }),
        _ => None,
    }
}

/// The schema's own spelling of an alignment method, read into `jlreq_line::Alignment`.
///
/// `Option` rather than a fallback variant: `direction_of` below has a stated default
/// because JLReq's own more common case is horizontal, but no one of the four `Alignment`
/// methods is "more common" for an absent or misspelled value to fall back to — every
/// candidate fallback would silently answer a different `align` case than the one asked, so
/// this declines instead, the same "not attempted" `Compose::align` reports for it.
fn alignment_of(alignment: Option<&str>) -> Option<Alignment> {
    match alignment {
        Some("centered") => Some(Alignment::Centered),
        Some("line-head") => Some(Alignment::LineHead),
        Some("line-end") => Some(Alignment::LineEnd),
        Some("even-spacing") => Some(Alignment::EvenSpacing),
        _ => None,
    }
}

/// The search a `compose` case declares, in the library's own vocabulary. `None` when the
/// case names none at all reads as [`Search::FirstFit`] — `cases.schema.json`'s own
/// `search` description states why an absent field is that variant specifically rather
/// than a third, undeclared default — matching `direction_of`'s own shape one field above:
/// a fallback for an absent field, and a decline (`Option`) for one this crate's own
/// vocabulary does not recognize, on the same "malformed case" standard `alignment_of`
/// already holds `align`'s own `alignment` field to. `Badness::new` clamps a `tolerance`
/// past its own `10_000` cap rather than refusing it, so a case naming a caller unit beyond
/// that range still reads as [`Badness::WORST`] here rather than being declined for it.
fn search_of(search: Option<&CaseSearch>) -> Option<Search> {
    match search {
        None => Some(Search::FirstFit),
        Some(CaseSearch { kind, .. }) if kind == "first-fit" => Some(Search::FirstFit),
        Some(CaseSearch {
            kind,
            tolerance: Some(tolerance),
        }) if kind == "optimal" => {
            let tolerance = u32::try_from(*tolerance).ok()?;
            Some(Search::Optimal {
                tolerance: Badness::new(tolerance),
            })
        },
        Some(_) => None,
    }
}

/// The writing direction a case declares, defaulting to horizontal when it names neither.
/// `jlreq_unit::Direction` has no default of its own (ADR 0011: direction is a datum, never
/// assumed), so a case that does not care about it still needs one supplied — every
/// currently-published `boundary` case that omits the field is written to read the same
/// either way, and horizontal is JLReq's own more common case.
fn direction_of(direction: Option<&str>) -> Direction {
    match direction {
        Some("vertical") => Direction::Vertical,
        _ => Direction::Horizontal,
    }
}

/// The schema's own spelling of a referent: the reverse of `frame_of` and `role_of` above,
/// which read a case's vocabulary into the library's. `Referent` is `#[non_exhaustive]` with
/// exactly two variants today (B.1's `be`/`af`), so the match has no wildcard arm to hide a
/// third one arriving unnoticed.
fn referent_name(referent: Referent) -> String {
    match referent {
        Referent::Preceding => "preceding",
        Referent::Trailing => "trailing",
    }
    .to_owned()
}

/// One conditional space, translated into the case format.
///
/// `ladder` is always `"reduction"` now (ADR-0021 amends ADR-0014): a `ConditionalSpace` no
/// longer carries an `Expansion` at all — Table 6's own opportunity is
/// [`jlreq::Boundary::expansion`]'s fact, read by [`case_expansion_of`] instead and
/// published on `boundary.expansion` rather than folded into whichever space happened to be
/// rigid. `reduction`, `floor_units` and `stage` consequently answer `reduction()`'s own
/// shape directly, with no fallback to consult: a rigid space simply has no floor or stage
/// to report, the same reading `special_reduction`'s own callers already give an
/// unrecognized note.
fn case_space_of(space: ConditionalSpace) -> CaseSpace {
    let (kind, floor, stage) = match space.reduction() {
        Reduction::Range { floor, stage } => ("range", floor, stage.ordinal()),
        Reduction::Discrete { floor, stage } => ("discrete", floor, stage.ordinal()),
        // `Reduction::Rigid`, and — `Reduction` is `#[non_exhaustive]` (ADR-0012) — any
        // future variant this crate's own copy of Appendix D has not been asked to add.
        // Reading an unrecognized one as rigid is the same conservative default
        // `special_reduction`'s own callers fall back to when a coordinate carries no
        // cited note.
        _ => ("rigid", Em::ZERO, 0),
    };
    CaseSpace::new(
        i64::from(space.amount().units()),
        referent_name(space.referent()),
        kind.to_owned(),
        i64::from(floor.units()),
        "reduction".to_owned(),
        stage,
    )
}

/// A boundary's own expansion opportunity, translated into the case format (ADR-0014,
/// amended by ADR-0021): a fact about the coordinate, published on `boundary.expansion`
/// rather than folded into any one conditional space.
///
/// `rule` is read once, alongside `expansion`, and republished on every one of the three
/// branches below rather than only on `Range` — including on `None`, which is the entire
/// point `jlreq_spacing::Boundary::expansion_rule`'s own doc argues for: a `Some` here can
/// sit beside a `"none"` `kind` when a Table 6 row (or a note that governs one) denied the
/// opportunity rather than never having spoken about the coordinate at all, and folding the
/// citation into only the `Range` arm would silently discard exactly the fact this round
/// exists to publish.
fn case_expansion_of(expansion: Expansion, rule: Option<RuleId>) -> CaseExpansion {
    let rule = rule.map(|rule| rule.address().to_string());
    match expansion {
        Expansion::Range { ceiling, stage } => CaseExpansion::new(
            "range".to_owned(),
            Some(i64::from(ceiling.units())),
            Some(stage.ordinal()),
            rule,
        ),
        Expansion::Residual => CaseExpansion::new("residual".to_owned(), None, None, rule),
        // `Expansion::None`, and — `Expansion` is `#[non_exhaustive]` (ADR-0012) — any
        // future kind this workspace's own `jlreq-spacing` does not yet grant. Read as "no
        // opportunity", the same reading `Expansion::None` itself states; `rule` still
        // travels here, unlike before this round.
        _ => CaseExpansion::new("none".to_owned(), None, None, rule),
    }
}

/// A ruby-overhang permission as a unit count. `Some(0)` for §3.3.8 rule 1's "no extension
/// at all", never bare `None`: `jlreq::boundary` always answers this question, so `None` is
/// reserved for a boundary `Compose::boundary` already declined before reaching here.
fn ruby_overhang_units(overhang: RubyOverhang) -> i64 {
    match overhang {
        RubyOverhang::OverSpace { limit } | RubyOverhang::OverCharacter { limit } => {
            i64::from(limit.units())
        },
        // `RubyOverhang::None`, and — `RubyOverhang` is `#[non_exhaustive]` (ADR-0012) —
        // any future permission this workspace's own `jlreq-spacing` does not yet grant.
        // Zero is the same reading `RubyOverhang::None` itself states: no extension at all.
        _ => 0,
    }
}

/// The break opportunities a case declares, in the library's own vocabulary.
///
/// Every one is `Candidate::At`: the published format has no field for §C.2 note 12's
/// discretionary hyphen (there is nowhere to state the inserted glyph's own advance), so a
/// case that needed one is a question this crate cannot yet ask `jlreq_line::Candidate` —
/// not a case in the suite today, and `conform --check`'s own schema does not offer the
/// shape either.
fn candidates_of(input: &CaseInput) -> Option<Vec<Candidate>> {
    input
        .candidates
        .iter()
        .map(|&at| {
            u32::try_from(at)
                .ok()
                .map(|offset| Candidate::At(ByteOffset::new(offset)))
        })
        .collect()
}

/// One length in the caller's own unit, as a case states `measure`, `first_line_indent`,
/// `head_indent` or `end_indent` for `compose`, or `measure` read as `align`'s own `target`
/// for `align`: an `InlineExtent`, never an `Em` — every one of those fields is not a
/// writing-system fraction, it is the caller's own measured length on the axis `Paragraph`
/// or `align` composes against, exactly as an item's own `advance` is (`item_of`'s own
/// doc).
fn inline_extent(value: i64) -> Option<InlineExtent> {
    InlineExtent::new(i32::try_from(value).ok()?)
}

/// One composed line, translated into the case format.
fn case_line_of(line: &Line) -> CaseLine {
    CaseLine::new(
        line.placements()
            .iter()
            .map(|offset| i64::from(offset.units()))
            .collect(),
        i64::from(line.trailing().units()),
        i64::from(line.extent().units()),
        line.trims().iter().copied().map(case_trim_of).collect(),
        // `Line::parts` is provably empty at M1, not merely stated so: `Part` carries a
        // private `core::convert::Infallible` field (`jlreq-line`'s own doc), so nothing
        // outside that crate can ever construct one for this to map over. The empty vector
        // here is exact, not a placeholder standing in for work this milestone left undone.
        Vec::new(),
        line.hanging()
            .map(|hanging| i64::from(hanging.beyond.units())),
        line.pull_up().map(case_pull_up_of),
    )
}

/// One pull-up (§3.1.12 ⑤, `Search::Optimal`), translated into the case format.
fn case_pull_up_of(pull_up: PullUp) -> CasePullUp {
    CasePullUp::new(
        i64::from(pull_up.amount.units()),
        usize::try_from(pull_up.pulls.get()).unwrap_or(usize::MAX),
        Some(pull_up.rule.address().to_string()),
    )
}

/// One trim, translated into the case format.
fn case_trim_of(trim: Trim) -> CaseTrim {
    CaseTrim::new(
        usize::try_from(trim.at.get()).unwrap_or(usize::MAX),
        i64::from(trim.amount.units()),
        referent_name(trim.referent),
        trim.rule.address().to_string(),
    )
}

/// One case's base stream, in the library's own vocabulary.
///
/// Held as a value rather than built inline because `Text` borrows the items and the scales,
/// so both have to outlive it.
#[derive(Debug)]
#[non_exhaustive]
pub struct Stream<'a> {
    /// The stream's own text.
    text: &'a str,
    /// One item per occurrence.
    items: Vec<Item>,
    /// The character sizes the stream declares.
    scales: Vec<Scale>,
}

impl<'a> Stream<'a> {
    /// Read one case's base stream, or say which part of it the vocabulary cannot hold.
    ///
    /// A case that reaches here has already passed `conform --check`, which holds the same
    /// input to the same schema, so a failure here is a disagreement between two readings of
    /// one format rather than a malformed case.
    pub fn of(input: &'a CaseInput) -> Result<Self, String> {
        let scales = input
            .scales
            .iter()
            .map(|scale| {
                let inline = advance(scale.inline_em)?;
                let block = advance(scale.block_em)?;
                Scale::new(inline, block).ok_or_else(|| "a character size is positive".to_owned())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let items = input
            .items
            .iter()
            .map(item_of)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            text: &input.text,
            items,
            scales,
        })
    }

    /// The stream this crate's own constructor accepts, or the refusal it answered with.
    pub fn text(&self) -> Result<Text<'_>, TextError> {
        Text::new(self.text, &self.items, &self.scales)
    }
}

/// One item in the library's own vocabulary.
fn item_of(item: &CaseItem) -> Result<Item, String> {
    let start = u32::try_from(item.start).map_err(|_| "an offset is a byte of the stream")?;
    let width = i32::try_from(item.advance).map_err(|_| "an advance is a length".to_owned())?;
    let advance =
        InlineExtent::new(width).ok_or_else(|| "an advance is not negative".to_owned())?;
    let scale = u8::try_from(item.scale).map_err(|_| "a stream declares at most 32 sizes")?;
    Ok(
        Item::new(ByteOffset::new(start), advance, ScaleId::new(scale))
            .with_frame(frame_of(item.frame.as_deref()))
            .with_role(role_of(item.role.as_deref())),
    )
}

/// One length in the library's own vocabulary.
fn advance(value: i64) -> Result<Advance, String> {
    let units = i32::try_from(value).map_err(|_| "a length is an i32 of caller units")?;
    Advance::new(units).ok_or_else(|| "a length is not negative".to_owned())
}

/// The frame a case names, which is the schema's own vocabulary.
fn frame_of(frame: Option<&str>) -> Frame {
    match frame {
        Some("full-em") => Frame::FullEm,
        Some("half-em") => Frame::HalfEm,
        Some("third-em") => Frame::ThirdEm,
        Some("quarter-em") => Frame::QuarterEm,
        Some("proportional") => Frame::Proportional,
        _ => Frame::Unstated,
    }
}

/// The role a case names, which is the schema's own vocabulary.
fn role_of(role: Option<&str>) -> Role {
    match role {
        Some("decimal-point") => Role::DecimalPoint,
        Some("digit-group-separator") => Role::DigitGroupSeparator,
        Some("unit-symbol") => Role::UnitSymbol,
        Some("quantity-symbol") => Role::QuantitySymbol,
        Some("sentence-terminator") => Role::SentenceTerminator,
        Some("sentence-medial") => Role::SentenceMedial,
        _ => Role::Unstated,
    }
}

/// One ordinal in the library's own vocabulary.
fn ordinal(item: usize) -> Option<ItemIndex> {
    u32::try_from(item).ok().map(ItemIndex::new)
}

/// Every case input this workspace's own `Text::new` refuses.
///
/// `conform --check` and `Text::new` are two implementations of ADR 0018's invariants — one
/// over the published format, one over the library's constructor — so a case the gate
/// accepts and the constructor refuses is a divergence between them and a finding in its own
/// right, never a case to skip. It is reported here rather than folded into the run, because
/// a refusal is not an answer.
#[must_use]
pub fn refusals(suite: &Suite) -> Vec<String> {
    let mut found = Vec::new();
    for case in suite.cases() {
        match Stream::of(case.input()).and_then(|stream| {
            stream
                .text()
                .map(|_| ())
                .map_err(|error| format!("Text::new refused it: {error:?}"))
        }) {
            Ok(()) => {},
            Err(reason) => found.push(format!("{id}: {reason}", id = case.id())),
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use core::num::NonZeroU16;

    use jlreq::{Construct, ConstructKind, RunId};

    use crate::case::{CaseConstruct, CaseInput, CaseItem, CaseScale};
    use crate::run::{Compose, Edge};

    use super::{Kumihan, construct_kind_of, overlay_of};

    /// Two middle dots (・・), the same pair `D.2/two-middle-dots/...` composes its own
    /// case from: base for both the interior and the line-end reading below.
    fn two_middle_dots() -> CaseInput {
        CaseInput {
            kind: "boundary".to_owned(),
            text: "・・".to_owned(),
            scales: vec![CaseScale {
                inline_em: 1000,
                block_em: 1000,
            }],
            items: vec![
                CaseItem {
                    start: 0,
                    advance: 500,
                    scale: 0,
                    frame: Some("half-em".to_owned()),
                    role: None,
                },
                CaseItem {
                    start: 3,
                    advance: 500,
                    scale: 0,
                    frame: Some("half-em".to_owned()),
                    role: None,
                },
            ],
            annotations: Vec::new(),
            constructs: Vec::new(),
            candidates: Vec::new(),
            measure: None,
            search: None,
            direction: None,
            first_line_indent: None,
            head_indent: None,
            end_indent: None,
            widow_threshold: None,
            alignment: None,
            tab_starts: Vec::new(),
            tab_stops: Vec::new(),
        }
    }

    #[test]
    fn a_line_end_boundary_answers_a_different_question_than_an_interior_one() {
        // The regression `Compose::boundary` used to have unconditionally: a case naming
        // `edge: "end"` was indistinguishable from one naming no edge at all, because every
        // `before` was handed to `Adjacency::between` alone. Table 1's `(before: 5, after:
        // line-edge)` cell carries one term — the preceding middle dot's own quarter em,
        // §B.2 note 4 — where the interior `(before: 5, after: 5)` cell §B.2 note 3 states
        // carries two, one per middle dot. If `at_line_end` were not really wired in, this
        // boundary would answer exactly like the interior one, or not answer at all.
        let input = two_middle_dots();
        let kumihan = Kumihan::default();
        let interior = kumihan
            .boundary(&input, 0, None)
            .expect("Adjacency::between answers a real pair of middle dots");
        let at_end = kumihan
            .boundary(&input, 0, Some(Edge::End))
            .expect("Adjacency::at_line_end answers too, now that it is wired in");
        assert_eq!(
            interior.spaces.len(),
            2,
            "the interior boundary carries both middle dots' quarter ems: {interior:?}"
        );
        assert_eq!(
            at_end.spaces.len(),
            1,
            "the line-end boundary has only the preceding middle dot to carry one: {at_end:?}"
        );
        assert_eq!(at_end.spaces[0].referent, "preceding");
        assert!(
            !at_end.rules.is_empty(),
            "a real evaluator answer names the rule it read, same as the interior one does"
        );
    }

    /// The run identified by `id`, which is never zero in a test.
    fn run(id: u16) -> RunId {
        RunId::new(NonZeroU16::new(id).expect("a nonzero run id"))
    }

    /// One declared construct of `kind`, covering `[first, past)`, with no `style`,
    /// `annotation` or `runs`.
    fn construct(kind: &str, first: usize, past: usize) -> CaseConstruct {
        CaseConstruct {
            kind: kind.to_owned(),
            index: 0,
            ranges: vec![(first, past)],
            style: None,
            annotation: None,
            runs: Vec::new(),
        }
    }

    /// A `CaseInput` with the given `constructs` and nothing else stated — `overlay_of`'s
    /// own tests read only `constructs`, and the `item_count` every one of them passes
    /// alongside `&input` is independent of whatever `input.items` happens to hold.
    fn input_with(constructs: Vec<CaseConstruct>) -> CaseInput {
        CaseInput {
            kind: "feasible".to_owned(),
            text: String::new(),
            scales: Vec::new(),
            items: Vec::new(),
            annotations: Vec::new(),
            constructs,
            candidates: Vec::new(),
            measure: None,
            search: None,
            direction: None,
            first_line_indent: None,
            head_indent: None,
            end_indent: None,
            widow_threshold: None,
            alignment: None,
            tab_starts: Vec::new(),
            tab_stops: Vec::new(),
        }
    }

    #[test]
    fn construct_kind_of_wires_the_three_kinds_a_case_can_honestly_declare() {
        assert_eq!(
            construct_kind_of(&construct("ornaments", 0, 3)),
            Some(ConstructKind::Ornamented)
        );
        assert_eq!(
            construct_kind_of(&construct("tate_chu_yoko", 0, 2)),
            Some(ConstructKind::TateChuYoko)
        );
        let mono = CaseConstruct {
            style: Some("mono".to_owned()),
            ..construct("ruby", 0, 1)
        };
        assert_eq!(
            construct_kind_of(&mono),
            Some(ConstructKind::NonJukugoRuby),
            "§A.22's own 'ruby other than jukugo-ruby' covers mono-ruby and group-ruby alike"
        );
        let group = CaseConstruct {
            style: Some("group".to_owned()),
            ..construct("ruby", 0, 2)
        };
        assert_eq!(
            construct_kind_of(&group),
            Some(ConstructKind::NonJukugoRuby)
        );
        let jukugo = CaseConstruct {
            style: Some("jukugo".to_owned()),
            ..construct("ruby", 0, 2)
        };
        assert_eq!(construct_kind_of(&jukugo), Some(ConstructKind::JukugoRuby));
    }

    #[test]
    fn construct_kind_of_declines_a_ruby_with_no_declared_style() {
        assert_eq!(
            construct_kind_of(&construct("ruby", 0, 1)),
            None,
            "the schema makes `style` optional, and nothing here may guess between \
             `NonJukugoRuby` and `JukugoRuby` for a construct that did not declare one"
        );
    }

    #[test]
    fn construct_kind_of_declines_every_kind_with_no_honest_conversion() {
        for kind in [
            "emphasis",
            "formulae",
            "warichu",
            "furiwake",
            "jidori",
            "reference_marks",
        ] {
            assert_eq!(
                construct_kind_of(&construct(kind, 0, 1)),
                None,
                "`{kind}` has no honest `ConstructKind` this adapter converts to \
                 (`overlay_of`'s own doc states which of the two reasons applies)"
            );
        }
    }

    #[test]
    fn overlay_of_gives_two_declared_tate_chu_yoko_runs_two_fresh_run_ids() {
        let input = input_with(vec![
            construct("tate_chu_yoko", 0, 2),
            construct("tate_chu_yoko", 2, 4),
        ]);
        let slots = overlay_of(&input, 4).expect("two well-formed, disjoint runs convert");
        assert_eq!(slots[0].map(Construct::run), Some(run(1)));
        assert_eq!(slots[1].map(Construct::run), Some(run(1)));
        assert_eq!(slots[2].map(Construct::run), Some(run(2)));
        assert_eq!(slots[3].map(Construct::run), Some(run(2)));
        assert!(
            slots
                .iter()
                .flatten()
                .all(|construct| construct.kind() == ConstructKind::TateChuYoko)
        );
    }

    #[test]
    fn overlay_of_leaves_an_item_no_construct_covers_as_none() {
        let input = input_with(vec![construct("tate_chu_yoko", 1, 3)]);
        let slots = overlay_of(&input, 4).expect("one well-formed run converts");
        assert!(slots[0].is_none(), "before the declared run: {slots:?}");
        assert!(slots[1].is_some());
        assert!(slots[2].is_some());
        assert!(slots[3].is_none(), "after the declared run: {slots:?}");
    }

    #[test]
    fn overlay_of_declines_a_construct_naming_more_than_one_range() {
        let mut split = construct("ornaments", 0, 1);
        split.ranges.push((2, 3));
        let input = input_with(vec![split]);
        assert_eq!(
            overlay_of(&input, 4),
            None,
            "one `RunId` names one contiguous block (`Runs::new`'s own doc); a construct \
             naming two spans has no single block to place, so this declines rather than \
             flattening the ranges together or silently taking the first"
        );
    }

    #[test]
    fn overlay_of_declines_the_whole_case_over_one_inconvertible_construct() {
        let input = input_with(vec![
            construct("tate_chu_yoko", 0, 2),
            construct("warichu", 2, 4),
        ]);
        assert_eq!(
            overlay_of(&input, 4),
            None,
            "leaving the warichu's own items as `None` would misreport them as plain text \
             the case never said they were, so one inconvertible construct fails the whole \
             conversion rather than only its own slots"
        );
    }

    #[test]
    fn feasible_declines_a_candidate_kinsoku_has_no_adjacency_to_evaluate_at() {
        // Byte offset 1 falls inside the first middle dot's own three UTF-8 bytes: it names
        // neither an item start nor the text's own end, so `Feasible::compute` reports it
        // in neither `breaks()` nor `rejected()`. `Compose::feasible` must decline the
        // whole case rather than invent `breakable: false` for a fact never decided.
        let input = CaseInput {
            candidates: vec![1],
            ..two_middle_dots()
        };
        let kumihan = Kumihan::default();
        assert_eq!(kumihan.feasible(&input, 0), None);
    }

    #[test]
    fn feasible_answers_a_real_candidate_with_no_declared_overlay() {
        // Two hiragana, cl-15 against cl-15: verified blank in Table 1 and Table 2
        // (`spec/captured/table1.en.tsv`, `spec/captured/table2.en.tsv`) rather than
        // assumed, so this candidate's answer turns on nothing but the plain "no construct
        // declared" path — `overlay_of` over an empty `constructs` builds the all-`None`
        // overlay `Runs::none()` itself is, and `same_run_refusal` declines for both sides
        // the identical way it declines for an item outside every construct.
        let input = CaseInput {
            kind: "feasible".to_owned(),
            text: "あい".to_owned(),
            scales: vec![CaseScale {
                inline_em: 1000,
                block_em: 1000,
            }],
            items: vec![
                CaseItem {
                    start: 0,
                    advance: 1000,
                    scale: 0,
                    frame: Some("full-em".to_owned()),
                    role: None,
                },
                CaseItem {
                    start: 3,
                    advance: 1000,
                    scale: 0,
                    frame: Some("full-em".to_owned()),
                    role: None,
                },
            ],
            candidates: vec![3],
            ..input_with(Vec::new())
        };
        let kumihan = Kumihan::default();
        let answer = kumihan
            .feasible(&input, 0)
            .expect("a real interior candidate between two ordinary hiragana");
        assert!(answer.breakable, "{answer:?}");
    }
}
