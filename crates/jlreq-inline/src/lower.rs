// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Lowering: [`Constructs`], [`Lowered`], [`Contribution`], [`LowerError`] and [`lower`].
//!
//! The whole of §3.3 that a construct genuinely applies before a boundary answer is
//! reached lives here (ADR-0015). [`RubyStyle::MonoRuby`] is fully lowered this round:
//! [`Contribution::runs`], [`Contribution::separations`] and [`Contribution::block_demand`]
//! are each genuinely computed from a declared [`crate::Ruby`], not merely declared.
//! [`RubyStyle::GroupRuby`] and [`RubyStyle::JukugoRuby`] get their run identity and their
//! block demand — a real, useful slice of both — from `lower` itself, and `lower` still
//! produces no `Separation` for either style. For [`RubyStyle::GroupRuby`] that absence is
//! no longer an unfilled slot the way it once was: §3.3.6's own ruby-not-longer-than-base
//! geometry (`Question::GROUP_RUBY_DISTRIBUTION`) is real now, computed by [`crate::place`]
//! against a composed line's own placements rather than by this function against the bare
//! declaration (`crate::place`'s own module doc states the arithmetic in full), and the
//! ruby-longer-than-base half is a named, declined blocker there too — a `Separation` is
//! what would close *that* half, by widening the base before composition the way
//! [`collect_mono_separation`] already does for §3.3.8 rule 1, and this function does not
//! yet emit one. [`RubyStyle::JukugoRuby`]'s own §3.3.7 distribution is real now too, and in
//! the identical shape: `Question::JUKUGO_RUBY_LAYOUT` is read by [`crate::place`], not by
//! this function — paragraph 1 delegates per base character to §3.3.5's own method
//! (`lower`'s own alignment resolution below is hoisted to cover a jukugo construct for
//! exactly that reason), and paragraph 2's own `group` answer reuses §3.3.6's own geometry,
//! forced to `jis` (`decision:jukugo-group-layout-distribution`; [`crate::place`]'s own
//! module doc states the arithmetic in full). What remains entirely unfilled, in `lower` and
//! in [`crate::place`] alike, is narrower than before: §F's own `phonetic` answer, which
//! [`crate::place`] declines outright rather than implementing any part of, and the
//! `Separation` this function still does not emit for jukugo's own surplus — the identical
//! absence §3.3.6's own paragraph 3 leaves for group-ruby, above.
//!
//! `Question::RUBY_OVERHANG_KANA` and `Question::RUBY_OVERHANG_INDENT` are unfilled slots
//! too, for `Contribution::separations`' own narrower scope: mono-ruby's forced boundary
//! space is computed only for §3.3.8 rule 1's absolute prohibition — a base-adjacent
//! ideographic character (cl-19) — and never for the permitted overhang those two questions
//! govern over kana, punctuation and a line-head indent, which remains placement's own later
//! concern even now that placement is partly real (see [`crate::place`]'s own module doc for
//! exactly which slice) (`docs/decisions/mono-ruby-separation-split.md`).
//!
//! JLReq: §3.3.1–§3.3.8, ADR-0015, `decision:mono-ruby-separation-split`

use alloc::vec::Vec;
use core::num::NonZeroU16;
use core::ops::Range;

use jlreq_class::{Annotation, AnnotationIndex, Class, Text, resolve};
use jlreq_spec::{Policy, Question, RuleId};
use jlreq_unit::{
    Advance, BlockDemand, BlockExtent, Carry, Construct, ConstructKind, ConstructRef, Direction,
    Em, GroupId, InlineExtent, ItemIndex, RunId, Runs, Segment, Separation, distribute,
};

use crate::ruby::{Ruby, RubyAlignment, RubyRun, RubyStyle};
use crate::tcy::NotAvailable;

/// `1`, the weight [`distribute`] splits several different surpluses by, equally between
/// two or more sites: a mono-ruby run's own §3.3.8 rule 1 overhang surplus, split between
/// the run's two boundaries (`docs/decisions/mono-ruby-separation-split.md`); a mono-ruby
/// run's own §3.3.5(b)/(c) centering difference, split the identical way by
/// [`crate::place`], for the reason that file's own "Applies to" line now names both
/// consumers for; and, as of this round, §3.3.6's own two ends under the `jis` method
/// (paired with [`two`]'s own interior weight there) and every interior site under the
/// `flush` method — both [`crate::place`]'s own again. Built once from the literal, which
/// cannot fail. A function and not a `const` initializer: the `ops` gate attributes a
/// crossing to its enclosing item and a bare `const` at crate scope has none, the same
/// reason `jlreq_line::align`'s own `one` is a function.
///
/// Crate-visible rather than private: [`crate::place`] is a second caller in a second
/// module, not a second, independently drifting copy (the same argument
/// `docs/scalar-sites.toml` makes for `jlreq_line`'s own `shift_by`).
pub(crate) const fn one() -> Advance {
    match Advance::new(1) {
        Some(value) => value,
        None => Advance::ZERO,
    }
}

/// `2`, the weight [`distribute`] splits §3.3.6's own `jis` method by at each interior site
/// between two adjacent characters of a run set solid — a group-ruby run's own, or, as of
/// §3.3.7¶2's own `group` answer (`decision:jukugo-group-layout-distribution`), a jukugo
/// compound's own compound-wide synthetic run reusing the identical arithmetic — twice
/// [`one`]'s own weight, exactly the section's own "2 units of inter-character spacing... 1
/// unit of spacing" ratio between a run's interior sites and its two ends. Lives beside
/// [`one`] for the identical reason that function gives for living where it does: built once
/// from the literal, which cannot fail, and a function rather than a `const` initializer
/// because the `ops` gate attributes a crossing to its enclosing item, and [`one`]'s own
/// reviewed entry in `docs/scalar-sites.toml` does not cover this one — `two` is a different
/// item.
///
/// Crate-visible for the identical reason [`one`] is: [`crate::place`] is the one caller
/// outside this module, not a second, independently drifting copy.
pub(crate) const fn two() -> Advance {
    match Advance::new(2) {
        Some(value) => value,
        None => Advance::ZERO,
    }
}

/// The answer [`Contribution::construct_of`] gives for a [`RunId`] this contribution never
/// allocated.
const FALLBACK_CONSTRUCT: ConstructRef = ConstructRef::new(ConstructKind::NonJukugoRuby, 0);

/// The constructs a caller declares over one [`Text`].
///
/// Built from [`Constructs::over`] and configured by `with_*`, with no public fields, so
/// declaring a construct kind that arrives in a later milestone is a minor release
/// (ADR-0012). The neutral value declares nothing, which is what plain text passes.
///
/// This round's own slot: only [`Constructs::with_ruby`] exists. The other eight constructs
/// `docs/design/api-spine.md` names are a later round's addition, and a `with_*` accepting a
/// slice and dropping it would be worse than its absence — an accepted-and-ignored input is
/// a silent defect this crate does not publish.
///
/// JLReq: §3.3–§3.7
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Constructs<'c> {
    text: Text<'c>,
    ruby: &'c [Ruby<'c>],
}

impl<'c> Constructs<'c> {
    /// Declare no constructs yet, over `text`.
    ///
    /// JLReq: n/a (ADR-0015)
    #[must_use]
    pub const fn over(text: Text<'c>) -> Self {
        Self { text, ruby: &[] }
    }

    /// The stream these are declared over. [`lower`] reads it here rather than taking it
    /// again beside [`Constructs`].
    ///
    /// JLReq: n/a (ADR-0015)
    #[must_use]
    pub const fn text(&self) -> Text<'c> {
        self.text
    }

    /// Declare the ruby runs over this text.
    ///
    /// JLReq: §3.3.1–§3.3.8
    #[must_use]
    pub const fn with_ruby(self, ruby: &'c [Ruby<'c>]) -> Self {
        Self { ruby, ..self }
    }

    /// The declared ruby, in the order the caller passed it.
    ///
    /// Crate-visible: [`crate::place`] walks this the same way [`lower`] does, to reach
    /// each [`Ruby`]'s own [`RubyStyle`], [`Ruby::annotation`] and [`Ruby::runs`] — none
    /// of which [`Contribution`] carries back out, because a caller who built its own
    /// `Constructs` still holds the original slice and a seam that duplicated it would be
    /// a second carrier of one fact (ADR-0019). Not part of the published surface: unlike
    /// [`Constructs::text`], which [`crate::place`]'s own public signature hands back to
    /// a caller, nothing outside this crate ever needs the declared ruby slice itself.
    pub(crate) const fn ruby(&self) -> &'c [Ruby<'c>] {
        self.ruby
    }
}

/// Reusable scratch space for [`lower`] and [`crate::place`], so a caller composing many
/// paragraphs allocates once.
///
/// The two functions write disjoint buffers of this one type and each clears only its
/// own half — [`lower`]'s own private `reset` never touches this type's own attachment
/// or declined-run buffers, and [`crate::place`]'s own reset never touches the six
/// buffers above them — so a caller may share one instance across both calls for one
/// line, or keep two separate instances, whichever its own allocation discipline prefers.
/// It usually needs two regardless: [`crate::place`] takes a `&Contribution<'_>`
/// borrowing from whichever `Lowered` produced it, and `out: &mut Lowered` for its own
/// answer, and those cannot be the same borrow at once (`crate::place`'s own doc states
/// this call-site constraint in full).
///
/// JLReq: n/a (ADR-0015)
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct Lowered {
    runs: Vec<Option<Construct>>,
    separations: Vec<Separation>,
    block_demand: Vec<BlockDemand>,
    construct_refs: Vec<(RunId, ConstructRef)>,
    rules: Vec<RuleId>,
    /// The [`RubyAlignment`] resolved for each [`RubyStyle::MonoRuby`] construct and each
    /// [`RubyStyle::JukugoRuby`] construct alike — §3.3.7¶1 delegates to "the method
    /// described in § 3.3.5" without qualification, so a jukugo construct's own alignment is
    /// resolved by the identical read below, not a separate one — and whether that
    /// resolution is §3.3.5's own discouraged combination — katatsuki (肩付き) in horizontal
    /// writing. ADR-0019's own "every answer records which of the two applied", carried here
    /// rather than computed and dropped.
    alignments: Vec<(ConstructRef, RubyAlignment, bool)>,
    /// Every annotation [`crate::place`] has placed so far. Crate-visible: read back by
    /// [`crate::place::Attachments`] and written only by [`crate::place`] itself.
    pub(crate) attachments: Vec<crate::place::Attachment>,
    /// Every run [`crate::place`] declined to place so far, for one of the four reasons
    /// [`crate::place::Attachments::declined`] states in full: §3.3.5(c)'s own
    /// katatsuki-with-overflow choice, reachable through a [`RubyStyle::MonoRuby`] construct
    /// or a [`RubyStyle::JukugoRuby`] compound's own paragraph-1 run alike; §3.3.6 paragraph
    /// 3's own base-spreading method, reachable through a [`RubyStyle::GroupRuby`] run or a
    /// jukugo compound's own paragraph-2 `group` answer alike; §3.3.7¶2's own `phonetic`
    /// answer; and a jukugo compound whose base range one `place` call's own `items` only
    /// partially covers. None of the four is resolved by this round's `lower`. Crate-visible
    /// for the same reason as [`Lowered::attachments`].
    pub(crate) declined: Vec<ConstructRef>,
}

impl Lowered {
    /// Scratch space with nothing in it yet.
    ///
    /// JLReq: n/a (ADR-0015)
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Empty every buffer [`lower`] itself owns and size the run overlay to `items`, so
    /// this call's answers do not see a previous call's data. Leaves
    /// [`Lowered::attachments`] and [`Lowered::declined`] untouched — [`crate::place`]
    /// owns those and clears them itself, on its own call, not on this one's.
    fn reset(&mut self, items: usize) {
        self.runs.clear();
        self.runs.resize(items, None);
        self.separations.clear();
        self.block_demand.clear();
        self.construct_refs.clear();
        self.rules.clear();
        self.alignments.clear();
    }
}

/// Everything the constructs contribute, in the vocabulary the line layer speaks.
///
/// Exactly four things cross the seam to `jlreq-line` (ADR-0015): [`Contribution::runs`],
/// [`Contribution::segments`], [`Contribution::separations`] and
/// [`Contribution::block_demand`]. The other four accessors — [`Contribution::construct_of`],
/// [`Contribution::rules_fired`], [`Contribution::alignment_of`] and
/// [`Contribution::alignment_discouraged`] — report facts the construct layer itself
/// resolved but that reach no boundary answer, so they cross no seam: read by a caller and
/// by a later round's `jlreq::diagnose`, never by `jlreq-line`, which has no edge to read
/// them at all (ADR-0015's no-edge claim). Accessors rather than public fields, so a field
/// added later is detail (ADR-0012).
///
/// JLReq: n/a (ADR-0015)
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Contribution<'a> {
    runs: Runs<'a>,
    segments: &'a [Segment<'a>],
    separations: &'a [Separation],
    block_demand: &'a [BlockDemand],
    construct_refs: &'a [(RunId, ConstructRef)],
    rules: &'a [RuleId],
    alignments: &'a [(ConstructRef, RubyAlignment, bool)],
}

impl<'a> Contribution<'a> {
    /// Per-item run identity, so the same-run predicates need no construct knowledge.
    ///
    /// JLReq: n/a (ADR-0015)
    #[must_use]
    pub const fn runs(&self) -> Runs<'a> {
        self.runs
    }

    /// Spans the line layer does not lay out as ordinary inline text. Always empty this
    /// round — mono-ruby produces none, and `Segment`/`Interior` belong to tate-chu-yoko,
    /// warichu, furiwake and jidori, none of which this round implements.
    ///
    /// JLReq: n/a (ADR-0015)
    #[must_use]
    pub const fn segments(&self) -> &'a [Segment<'a>] {
        self.segments
    }

    /// Least spacing a construct forces at a base-text boundary (§3.3.8 rule 1). Genuinely
    /// computed for [`RubyStyle::MonoRuby`] and empty for the other two styles this round
    /// (see this module's own doc).
    ///
    /// JLReq: §3.3.8, `decision:mono-ruby-separation-split`
    #[must_use]
    pub const fn separations(&self) -> &'a [Separation] {
        self.separations
    }

    /// Block-axis demand per item range, carried through and reported, never acted on.
    ///
    /// JLReq: §4.5.1
    #[must_use]
    pub const fn block_demand(&self) -> &'a [BlockDemand] {
        self.block_demand
    }

    /// Which declared construct one identity came from. Total over every identity [`lower`]
    /// allocated.
    ///
    /// A `run` this contribution never allocated — one built by hand, or one from a
    /// different call — answers with the first construct this contribution itself
    /// allocated, the convention `jlreq_class::Text::size_of` already uses for an ordinal
    /// past its own domain: a stated fallback rather than a panic, for an input outside
    /// what this answer is defined over.
    ///
    /// JLReq: n/a (ADR-0015)
    #[must_use]
    pub fn construct_of(&self, run: RunId) -> ConstructRef {
        self.construct_refs
            .iter()
            .find(|(id, _)| *id == run)
            .map_or(FALLBACK_CONSTRUCT, |(_, reference)| *reference)
    }

    /// Every rule the construct layer applied.
    ///
    /// JLReq: §3.3
    pub fn rules_fired(&self) -> impl Iterator<Item = RuleId> + '_ {
        self.rules.iter().copied()
    }

    /// The [`RubyAlignment`] resolved for one declared construct: the per-construct
    /// [`Ruby::with_alignment`] override where the caller gave one, or
    /// [`jlreq_spec::Question::RUBY_ALIGNMENT`]'s policy answer otherwise — ADR-0019's one
    /// precedence rule for a fact stated through two doors, applied once during [`lower`]
    /// and recorded here rather than read once and discarded. `None` for a `construct`
    /// this contribution never resolved one for: [`RubyStyle::GroupRuby`] — the one style
    /// §3.3.5 never governs, because §3.3.6 states no nakatsuki/katatsuki choice at all —
    /// and any [`ConstructRef`] this contribution never allocated. Real for
    /// [`RubyStyle::MonoRuby`] *and* [`RubyStyle::JukugoRuby`] alike: §3.3.7¶1 delegates a
    /// jukugo compound's own ≤2-character runs to "the method described in § 3.3.5" without
    /// qualification, so [`lower`] resolves the identical fact for it, through the identical
    /// read.
    ///
    /// This is this round's own carrier of the fact ADR-0019 states — "every answer
    /// records which of the two applied, so a report says whether an alignment was the
    /// document's or this construct's". [`crate::place`] now exists and reads this on
    /// every mono-ruby run it places, and on every jukugo compound's own paragraph-1 run
    /// too, but it *consumes* the resolution to compute a geometry rather than republishing
    /// which alignment applied through an accessor of its own — a second carrier of the
    /// identical fact ADR-0019 already forbids. A caller who wants the fact itself, rather
    /// than the geometry it drove, still reads it here; a later round's `jlreq::diagnose`'s
    /// own `AlignmentDiscouraged` (`docs/design/api-spine.md`), which does not exist yet, is
    /// where that same fact would surface without the caller having to ask for it.
    ///
    /// JLReq: §3.3.5, §3.3.7
    #[must_use]
    pub fn alignment_of(&self, construct: ConstructRef) -> Option<RubyAlignment> {
        self.alignments
            .iter()
            .find(|(reference, _, _)| *reference == construct)
            .map(|(_, alignment, _)| *alignment)
    }

    /// Whether the alignment [`Contribution::alignment_of`] reports for `construct` is
    /// §3.3.5's own discouraged combination — katatsuki (肩付き) resolved in horizontal
    /// writing, which the section says "should not be adopted" without forbidding it
    /// (ADR-0011). `false` both for an ordinary resolution and for a `construct` this
    /// contribution never resolved one for, which is the correct answer to "was the
    /// discouraged combination read" when none was.
    ///
    /// Real for a [`RubyStyle::JukugoRuby`] construct exactly as it is for
    /// [`RubyStyle::MonoRuby`], on the settled reading of a question §3.3.7 leaves open: does
    /// §3.3.5's own recommendation transfer to a jukugo compound's own paragraph-1 runs, or
    /// does §F's own stated assumption of a katatsuki-distributed baseline for a *different*
    /// method (paragraph 2's own `phonetic` answer, which this round declines wholesale)
    /// unsettle it? Paragraph 1's own words are "compose ruby characters as described in
    /// § 3.3.5" — the section's own *method*, not its arithmetic in isolation — which reads
    /// as a wholesale delegation, recommendation included; §F's own assumption is stated for
    /// a method this jukugo construct may never even reach (only a compound answering
    /// `phonetic` reads §F at all, and this round implements none of it), so it has nothing
    /// to unsettle for a paragraph-1 construct in the first place. This reading is also what
    /// the code below already does, by construction rather than by a jukugo-specific branch:
    /// the identical `discouraged` computation runs for both styles because the hoist just
    /// below resolves them through the identical read, and a caller-visible flag that meant
    /// two different things for two styles sharing one code path would be the kind of
    /// unstated substitution this project's own discipline exists to name rather than commit
    /// silently.
    ///
    /// [`lower`] proceeds with the caller's own resolved choice regardless of this flag
    /// — honored, never refused — and records it so a caller, or a later round's
    /// `jlreq::diagnose`, can tell the discouraged combination apart from an ordinary
    /// one without re-deriving it from a [`jlreq_spec::Policy`] and a [`Direction`] it
    /// may no longer have in hand.
    ///
    /// JLReq: §3.3.5, §3.3.7
    #[must_use]
    pub fn alignment_discouraged(&self, construct: ConstructRef) -> bool {
        self.alignments
            .iter()
            .any(|(reference, _, discouraged)| *reference == construct && *discouraged)
    }
}

/// Lower the declared constructs. Does **not** remove break candidates: a candidate inside
/// an indivisible construct is refused by the ordinary same-run predicates in `jlreq-line`,
/// the crate that owns break refusal (ADR-0015).
///
/// This is the one place the ruby alignment question is resolved — for a
/// [`RubyStyle::MonoRuby`] construct and, on §3.3.7¶1's own wholesale delegation to "the
/// method described in § 3.3.5," for a [`RubyStyle::JukugoRuby`] construct too — and
/// therefore the one hand-written item in the workspace outside a construct constructor
/// that may name a variant of [`Direction`]: §3.3.5's recommendation against katatsuki is
/// direction-conditional, and `docs/direction-sites.toml` lists this item for it
/// (ADR-0011). The per-construct alignment overrides the policy's, and both are honored
/// regardless of direction — never refused — because the recommendation is exactly that,
/// a recommendation.
///
/// JLReq: §3.3.1–§3.3.8
pub fn lower<'a>(
    constructs: &Constructs<'_>,
    policy: Policy,
    direction: Direction,
    out: &'a mut Lowered,
) -> Result<Contribution<'a>, LowerError> {
    let text = constructs.text();
    out.reset(text.items().len());

    let mut ctx = LowerCtx {
        text,
        policy,
        carry: Carry::new(),
        next_run: NonZeroU16::MIN,
        next_group: NonZeroU16::MIN,
        rules: RuleSet::default(),
        separation_pairs: Vec::new(),
    };

    for (ordinal, ruby) in constructs.ruby.iter().enumerate() {
        let construct_ref = ConstructRef::new(construct_kind_of(ruby.style()), ordinal_of(ordinal));
        check_ruby_bounds(text, *ruby, construct_ref)?;

        // Resolved for `RubyStyle::MonoRuby` *and* `RubyStyle::JukugoRuby` alike: §3.3.7¶1
        // delegates a jukugo compound's own ≤2-character runs to "the method described in
        // § 3.3.5" without qualification, and `crate::place`'s own `place_mono_run` reads
        // `Contribution::alignment_of` on every run it places, mono-ruby's or a jukugo
        // compound's own — without this push a jukugo construct's own alignment would answer
        // `None`, and `place_mono_run`'s own `let Some(alignment) = ... else { return; }`
        // would place nothing at all, silently, the moment paragraph 1's own condition holds
        // (this module's own doc). `RubyStyle::GroupRuby` deliberately stays without one:
        // §3.3.6 states no nakatsuki/katatsuki choice at all, only its own `jis`/`flush`
        // distribution, which `crate::place` reads directly rather than through this
        // resolution.
        //
        // The §3.3.5 rule citation stays mono-only, below: that citation is about which
        // *method* paragraph 1 delegates to, and §3.3.7¶1 is `crate::place`'s own citation to
        // give, once it has actually decided paragraph 1 governs this construct — a decision
        // this function never makes (`lower_jukugo`'s own doc states why in full). Recording
        // the mono citation here for a jukugo construct too would claim §3.3.5 fired for a
        // construct this function never determined even reaches that method.
        if matches!(ruby.style(), RubyStyle::MonoRuby | RubyStyle::JukugoRuby) {
            let resolved = ruby
                .alignment()
                .unwrap_or_else(|| default_alignment(policy));
            // §3.3.5 "should not be adopted" is a recommendation, not a refusal
            // (ADR-0011): `lower` proceeds with `resolved` regardless of `direction`, and
            // `discouraged` is what turns the direction comparison into a fact
            // `Contribution::alignment_discouraged` can report rather than one computed
            // here and dropped (ADR-0019's own "every answer records which of the two
            // applied"). This is the read of `direction` `docs/direction-sites.toml`
            // anchors this item's own §3.3.5 site to — now also for a jukugo construct's own
            // resolution, on the identical reading paragraph 1's own delegation states: §F's
            // own assumption of a katatsuki-distributed baseline governs a different method
            // (paragraph 2's own `phonetic` answer, unimplemented) and does not unsettle
            // paragraph 1's own wholesale delegation to §3.3.5, recommendation included.
            let discouraged =
                resolved == RubyAlignment::Katatsuki && direction == Direction::Horizontal;
            out.alignments.push((construct_ref, resolved, discouraged));
            if matches!(ruby.style(), RubyStyle::MonoRuby) {
                ctx.rules
                    .record(RuleId::POSITIONING_OF_MONO_RUBY_WITH_RESPECT_TO_BASE_CHARACTERS);
            }
        }

        match ruby.style() {
            RubyStyle::MonoRuby => lower_mono(&mut ctx, out, *ruby, construct_ref)?,
            RubyStyle::JukugoRuby => lower_jukugo(&mut ctx, out, *ruby, construct_ref)?,
            RubyStyle::GroupRuby => lower_group(&mut ctx, out, *ruby, construct_ref)?,
        }
    }

    finish_separations(&mut ctx.separation_pairs, out);
    out.rules.extend(ctx.rules.into_vec());

    // `write_slot` refuses an overlap before it can ever reach the overlay, so the overlay
    // this loop built satisfies `Runs::new`'s own invariants by construction; the fallback
    // is a safety net against that argument being wrong, not a normal path (no unit test
    // reaches it, and none should).
    let runs = Runs::new(&out.runs).unwrap_or_else(|_| Runs::none());

    Ok(Contribution {
        runs,
        segments: &[],
        separations: &out.separations,
        block_demand: &out.block_demand,
        construct_refs: &out.construct_refs,
        rules: &out.rules,
        alignments: &out.alignments,
    })
}

/// Refused ways to declare or lower a construct.
///
/// JLReq: n/a (ADR-0015)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LowerError {
    /// Two constructs claim overlapping items in a way the specification does not nest.
    /// Named in the caller's own coordinates, not by identities [`lower`] invented.
    OverlappingConstructs {
        /// The construct already occupying the item.
        a: ConstructRef,
        /// The construct that claimed it a second time.
        b: ConstructRef,
    },
    /// A construct names an item outside the text.
    OutOfRange {
        /// The offending ordinal.
        at: ItemIndex,
        /// The construct that named it.
        construct: ConstructRef,
    },
    /// The construct is not defined in this direction. §3.2.5's tate-chu-yoko is the only
    /// one this round names; nothing this round's [`lower`] does can produce it, because
    /// [`Constructs`] carries no `with_tate_chu_yoko` yet (this module's own doc).
    NotAvailable(NotAvailable),
}

/// The state one [`lower`] call threads through every construct it processes.
struct LowerCtx<'t> {
    /// The stream every construct is declared over.
    text: Text<'t>,
    /// The document's own policy, for `Policy::remainder` and the alignment default.
    policy: Policy,
    /// The one rounding remainder this call carries, shared by every resolution against
    /// one declared size (ADR-0007, ADR-0019).
    carry: Carry,
    /// The next fresh run identity this call will allocate.
    next_run: NonZeroU16,
    /// The next fresh group identity this call will allocate.
    next_group: NonZeroU16,
    /// Every rule the construct layer has applied so far.
    rules: RuleSet,
    /// Every mono-ruby boundary demand collected so far, one entry per boundary, merged as
    /// they arrive (`docs/decisions/mono-ruby-separation-split.md`).
    separation_pairs: Vec<(ItemIndex, InlineExtent)>,
}

/// A set of [`RuleId`]s, recorded in first-seen order, each held once.
#[derive(Debug, Default)]
struct RuleSet(Vec<RuleId>);

impl RuleSet {
    /// Record that `rule` was applied, unless it already was.
    fn record(&mut self, rule: RuleId) {
        if !self.0.contains(&rule) {
            self.0.push(rule);
        }
    }

    /// The rules recorded, in the order they were first applied.
    fn into_vec(self) -> Vec<RuleId> {
        self.0
    }
}

/// The [`ConstructKind`] one [`RubyStyle`] lowers to. cl-22 for mono- and group-ruby, per
/// §A.22's "ruby other than jukugo-ruby"; cl-23 for jukugo-ruby.
///
/// JLReq: §A.22, §A.23
const fn construct_kind_of(style: RubyStyle) -> ConstructKind {
    match style {
        RubyStyle::MonoRuby | RubyStyle::GroupRuby => ConstructKind::NonJukugoRuby,
        RubyStyle::JukugoRuby => ConstructKind::JukugoRuby,
    }
}

/// `Question::RUBY_ALIGNMENT`'s policy answer, absent a per-construct override.
fn default_alignment(policy: Policy) -> RubyAlignment {
    if policy.get(Question::RUBY_ALIGNMENT).name() == "katatsuki" {
        RubyAlignment::Katatsuki
    } else {
        RubyAlignment::Nakatsuki
    }
}

/// `ordinal`, saturated into the `u16` a [`ConstructRef`] holds.
fn ordinal_of(ordinal: usize) -> u16 {
    u16::try_from(ordinal).unwrap_or(u16::MAX)
}

/// A fresh identity from `counter`, which is then advanced.
fn allocate(counter: &mut NonZeroU16) -> NonZeroU16 {
    let id = *counter;
    let bumped = counter.get().saturating_add(1);
    *counter = NonZeroU16::new(bumped).unwrap_or(NonZeroU16::MIN);
    id
}

/// `ruby` was declared over `text` itself, and its own base range lies inside it.
///
/// The first half catches a `Ruby` built against a different, stale `Text` than the one
/// `constructs.text()` now names — `Ruby::new`'s own validation only ever checked the
/// stream it was given, never the one a later `lower` call would use.
fn check_ruby_bounds(
    text: Text<'_>,
    ruby: Ruby<'_>,
    construct_ref: ConstructRef,
) -> Result<(), LowerError> {
    let base = ruby.base();
    if ruby.text() != text {
        return Err(LowerError::OutOfRange {
            at: base.start,
            construct: construct_ref,
        });
    }
    let Ok(end) = usize::try_from(base.end.get()) else {
        return Err(LowerError::OutOfRange {
            at: base.end,
            construct: construct_ref,
        });
    };
    if end > text.items().len() {
        return Err(LowerError::OutOfRange {
            at: base.end,
            construct: construct_ref,
        });
    }
    Ok(())
}

/// Claim `item` for `construct`, or report the construct that already holds it.
fn write_slot(
    out: &mut Lowered,
    item: ItemIndex,
    construct: Construct,
    this_ref: ConstructRef,
) -> Result<(), LowerError> {
    let Ok(index) = usize::try_from(item.get()) else {
        return Err(LowerError::OutOfRange {
            at: item,
            construct: this_ref,
        });
    };
    let Some(slot) = out.runs.get_mut(index) else {
        return Err(LowerError::OutOfRange {
            at: item,
            construct: this_ref,
        });
    };
    if let Some(existing) = *slot {
        let existing_ref = find_construct_ref(&out.construct_refs, existing.run());
        return Err(LowerError::OverlappingConstructs {
            a: existing_ref,
            b: this_ref,
        });
    }
    *slot = Some(construct);
    Ok(())
}

/// Which construct allocated `run`, or the fallback when nothing here did.
fn find_construct_ref(refs: &[(RunId, ConstructRef)], run: RunId) -> ConstructRef {
    refs.iter()
        .find(|(id, _)| *id == run)
        .map_or(FALLBACK_CONSTRUCT, |(_, reference)| *reference)
}

/// The greatest block extent any item of `range` declares, resolved through `annotation`'s
/// own scale table. §3.3.4's block-start side; §4.5.1's demand.
///
/// `carry` is one [`Carry`] shared across every `Ruby` one [`lower`] call processes, each
/// with its own declared annotation scale. That sharing is safe on two independent
/// grounds: `Carry` keys its remainder per em length rather than per caller
/// (`Carry::SIZES` distinct slots), so two different declared ruby ems never address the
/// same slot; and `Em::FULL` resolved against any em length is exact by construction —
/// multiplying by exactly one em leaves no fractional unit, and the residue this call
/// reads back out is the slot's own incoming value, unchanged — so this call never
/// actually writes a nonzero remainder into whichever slot it addresses, regardless of
/// which annotation supplied it.
fn run_block_extent(
    annotation: Annotation<'_>,
    range: Range<AnnotationIndex>,
    carry: &mut Carry,
) -> BlockExtent {
    let mut max = BlockExtent::ZERO;
    for index in range.start.get()..range.end.get() {
        let size = annotation.size_of(AnnotationIndex::new(index));
        max = max.max(Em::FULL.resolve_block(size, carry));
    }
    max
}

/// The sum of every item's own measured advance over `range`: one run's own inline
/// extent. [`collect_mono_separation`] measures §3.3.8 rule 1's surplus against it,
/// [`crate::place`] measures §3.3.5(b)/(c)'s own centering difference and start-fit
/// comparison against the identical value for mono-ruby, and, as of this round,
/// [`crate::place`] measures §3.3.6's own longer-than-base comparison and surplus against it
/// a third time, for group-ruby's own reading — one computation serving three questions
/// about the same quantity, not three independently drifting copies. Crate-visible for the
/// second caller (`crate::place`, both of whose own consumers reach it).
pub(crate) fn sum_advances(
    annotation: Annotation<'_>,
    range: Range<AnnotationIndex>,
) -> InlineExtent {
    let mut total = InlineExtent::ZERO;
    for index in range.start.get()..range.end.get() {
        if let Some(item) = annotation.items().get(index as usize) {
            total = total.add_sat(item.advance());
        }
    }
    total
}

/// Whether the item at `item` resolves to cl-19 (ideographic), the one class §3.3.8 rule 1
/// forbids ruby from ever overhanging.
fn is_adjacent_ideographic(text: Text<'_>, item: ItemIndex, policy: Policy) -> bool {
    resolve(text, item, policy).is_some_and(|answer| answer.value() == Class::Ideographic)
}

/// Merge `least` into the boundary after `after`, taking the greater of the two demands a
/// boundary already carries (`docs/decisions/mono-ruby-separation-split.md`'s question 3).
/// A `least` of zero adds nothing.
fn push_separation(
    pairs: &mut Vec<(ItemIndex, InlineExtent)>,
    after: ItemIndex,
    least: InlineExtent,
) {
    if least == InlineExtent::ZERO {
        return;
    }
    if let Some(existing) = pairs.iter_mut().find(|(at, _)| *at == after) {
        existing.1 = existing.1.max(least);
    } else {
        pairs.push((after, least));
    }
}

/// Sort the collected boundary demands and turn them into [`Separation`]s.
///
/// Extends `out.separations` rather than reassigning it, so the buffer [`Lowered::reset`]
/// already cleared keeps its allocation across calls, which is the "allocates once" [`Lowered`]'s
/// own doc promises a caller composing many paragraphs.
fn finish_separations(pairs: &mut [(ItemIndex, InlineExtent)], out: &mut Lowered) {
    pairs.sort_by_key(|(after, _)| after.get());
    out.separations.extend(
        pairs
            .iter()
            .map(|(after, least)| Separation::new(*after, *least)),
    );
}

/// Lower one [`RubyStyle::MonoRuby`] ruby: one fresh [`RunId`] per base item (§3.3.5,
/// §3.3.1's note — this is what gives adjacent bases §E.2 note 6's quarter-em opportunity),
/// this run's own block demand, and, where the reading is genuinely longer than its own
/// base's supplied advance, the forced boundary space §3.3.8 rule 1 requires.
fn lower_mono<'t>(
    ctx: &mut LowerCtx<'t>,
    out: &mut Lowered,
    ruby: Ruby<'t>,
    construct_ref: ConstructRef,
) -> Result<(), LowerError> {
    let annotation = ruby.annotation();
    let text_len = ctx.text.items().len();

    for run in ruby.runs() {
        let run_id = RunId::new(allocate(&mut ctx.next_run));
        out.construct_refs.push((run_id, construct_ref));

        let base = run.base();
        let item = base.start;
        let construct = Construct::new(ConstructKind::NonJukugoRuby, run_id, None);
        write_slot(out, item, construct, construct_ref)?;

        let extent = run_block_extent(annotation, run.annotation(), &mut ctx.carry);
        out.block_demand
            .push(BlockDemand::new(base, extent, BlockExtent::ZERO));

        collect_mono_separation(ctx, item, annotation, *run, text_len);
    }
    ctx.rules
        .record(RuleId::CHOICE_OF_SIDES_FOR_RUBY_WITH_RESPECT_TO_BASE_CHARACTERS);
    Ok(())
}

/// The §3.3.8 rule 1 surplus of one mono-ruby run, split evenly between its two boundaries
/// and recorded at whichever are genuinely adjacent to a cl-19 character
/// (`docs/decisions/mono-ruby-separation-split.md`).
fn collect_mono_separation<'t>(
    ctx: &mut LowerCtx<'t>,
    item: ItemIndex,
    annotation: Annotation<'t>,
    run: RubyRun,
    text_len: usize,
) {
    let reading_extent = sum_advances(annotation, run.annotation());
    let base_advance = ctx
        .text
        .items()
        .get(item.get() as usize)
        .map_or(InlineExtent::ZERO, |base_item| base_item.advance());
    let surplus = reading_extent.sub_sat(base_advance).max(InlineExtent::ZERO);
    if surplus == InlineExtent::ZERO {
        return;
    }

    let weights = [one(), one()];
    let mut shares = distribute(surplus, &weights, ctx.policy.remainder());
    let leading_share = shares.next().unwrap_or(InlineExtent::ZERO);
    let trailing_share = shares.next().unwrap_or(InlineExtent::ZERO);

    if let Some(previous) = item.get().checked_sub(1) {
        if is_adjacent_ideographic(ctx.text, ItemIndex::new(previous), ctx.policy) {
            push_separation(
                &mut ctx.separation_pairs,
                ItemIndex::new(previous),
                leading_share,
            );
        }
    }
    let next = item.get().saturating_add(1);
    if (next as usize) < text_len
        && is_adjacent_ideographic(ctx.text, ItemIndex::new(next), ctx.policy)
    {
        push_separation(&mut ctx.separation_pairs, item, trailing_share);
    }

    ctx.rules
        .record(RuleId::ADJUSTMENTS_OF_RUBY_WITH_LENGTH_LONGER_THAN_THAT_OF_THE_BASE_CHARACTERS);
}

/// Lower one [`RubyStyle::JukugoRuby`] ruby: one shared [`RunId`] across the whole compound
/// (§B.2#11, §C.2#8 — a break is permitted between two base characters of the same jukugo
/// complex, so they share a run), a fresh [`GroupId`] per base character within it (§C.2#8's
/// own level, so a break is refused between one base character and its own reading), and
/// each run's own block demand. Produces no `Separation`: the surplus this style's own
/// paragraph 2 needs to distribute is entirely a placement-time question now, resolved by
/// [`crate::place`] against a composed line's own placements the identical way §3.3.6's own
/// does for [`RubyStyle::GroupRuby`] (`lower_group`'s own doc, below, states the identical
/// argument) — what remains genuinely unfilled is §F's own `phonetic` answer, which
/// [`crate::place`] declines outright, and this function's own absence of a `Separation` for
/// a paragraph-2 compound whose reading is genuinely longer than its base, the jukugo
/// analogue of §3.3.6 paragraph 3's own unclosed half.
fn lower_jukugo<'t>(
    ctx: &mut LowerCtx<'t>,
    out: &mut Lowered,
    ruby: Ruby<'t>,
    construct_ref: ConstructRef,
) -> Result<(), LowerError> {
    let run_id = RunId::new(allocate(&mut ctx.next_run));
    out.construct_refs.push((run_id, construct_ref));
    let annotation = ruby.annotation();

    for run in ruby.runs() {
        let group_id = GroupId::new(allocate(&mut ctx.next_group));
        let base = run.base();
        for raw in base.start.get()..base.end.get() {
            let construct = Construct::new(ConstructKind::JukugoRuby, run_id, Some(group_id));
            write_slot(out, ItemIndex::new(raw), construct, construct_ref)?;
        }
        let extent = run_block_extent(annotation, run.annotation(), &mut ctx.carry);
        out.block_demand
            .push(BlockDemand::new(base, extent, BlockExtent::ZERO));
    }
    // Not `RuleId::POSITIONING_OF_JUKUGO_RUBY_WITH_RESPECT_TO_BASE_CHARACTERS` (§3.3.7):
    // that rule's own content is the ≤2-char-per-base delegation to §3.3.5's method, or
    // else the `group`/`phonetic` distribution otherwise — real geometry now, but entirely
    // `crate::place`'s to compute, against a composed line's own placements this function
    // never sees, the identical reasoning `lower_group`'s own sibling comment states below
    // for §3.3.6. This function only allocates run/group identity and block demand, the
    // same slice `RubyStyle::GroupRuby` gets. Recording §3.3.7 here would be a second
    // carrier of a fact `crate::place` itself is the one to report, once it has actually
    // decided which of paragraph 1 or paragraph 2 governs a given compound — a decision
    // this function never makes (ADR-0019).
    ctx.rules
        .record(RuleId::CHOICE_OF_SIDES_FOR_RUBY_WITH_RESPECT_TO_BASE_CHARACTERS);
    Ok(())
}

/// Lower one [`RubyStyle::GroupRuby`] ruby: one [`RunId`] over the whole base range,
/// internally unbreakable (§3.3.6), and its own block demand. Produces no `Separation`,
/// still: §3.3.6's own ruby-not-longer-than-base geometry
/// (`Question::GROUP_RUBY_DISTRIBUTION`, paragraphs 1 and 2) is real now, but it is entirely
/// a placement-time question — [`crate::place`]'s own module doc states the arithmetic in
/// full — that reads the base run's own extent from a composed line's own placements,
/// something this function, which runs before any line exists, has no access to at all. A
/// `Separation` would be needed only for §3.3.6 paragraph 3's own ruby-longer-than-base
/// half, whose method spreads the *base* characters apart before composition begins — the
/// mono-ruby analogue [`collect_mono_separation`] already performs for §3.3.8 rule 1 — and
/// that half is a named, declined blocker [`crate::place`] states this round, not yet built
/// here either.
fn lower_group<'t>(
    ctx: &mut LowerCtx<'t>,
    out: &mut Lowered,
    ruby: Ruby<'t>,
    construct_ref: ConstructRef,
) -> Result<(), LowerError> {
    let run_id = RunId::new(allocate(&mut ctx.next_run));
    out.construct_refs.push((run_id, construct_ref));
    let annotation = ruby.annotation();

    for run in ruby.runs() {
        let base = run.base();
        for raw in base.start.get()..base.end.get() {
            let construct = Construct::new(ConstructKind::NonJukugoRuby, run_id, None);
            write_slot(out, ItemIndex::new(raw), construct, construct_ref)?;
        }
        let extent = run_block_extent(annotation, run.annotation(), &mut ctx.carry);
        out.block_demand
            .push(BlockDemand::new(base, extent, BlockExtent::ZERO));
    }
    // Not `RuleId::POSITIONING_OF_GROUP_RUBY_WITH_RESPECT_TO_BASE_CHARACTERS` (§3.3.6): that
    // rule's own content — the length comparison between base and reading, and the
    // solid-set-plus-distribution (or decline) it drives — is `crate::place`'s to compute
    // now, against a composed line's own placements this function never sees; `lower_group`
    // only ever allocates one run and its own block demand, the same slice it always has.
    // Recording the rule here, rather than where the geometry is actually computed, would be
    // a second carrier of one fact (ADR-0019). `crate::place` itself records no `RuleId`
    // either, for the identical reason its own module doc already argues for §3.3.5: the
    // geometry is observable through the `Attachment`s and declined construct refs it
    // emits, and a `rules_fired`-shaped accessor there would answer the identical question a
    // second time.
    ctx.rules
        .record(RuleId::CHOICE_OF_SIDES_FOR_RUBY_WITH_RESPECT_TO_BASE_CHARACTERS);
    Ok(())
}

#[cfg(test)]
mod tests {
    use jlreq_class::{Annotation, AnnotationIndex, Text};
    use jlreq_spec::{Policy, RuleId};
    use jlreq_unit::{
        Advance, ByteOffset, Direction, InlineExtent, Item, ItemIndex, Scale, ScaleId,
    };

    use super::{Constructs, LowerError, Lowered, lower};
    use crate::ruby::{Ruby, RubyAlignment, RubyRun, RubyStyle};

    /// Horizontal writing, for a test that needs one concrete [`Direction`] to build a
    /// [`Constructs`] and call [`lower`] at all — `lower`'s own third parameter has no
    /// default (ADR-0011) — without naming the variant at each call site. None of these
    /// tests exercises §3.1.3, §3.2.5 or §3.3.5 (the direction-conditional rule under
    /// exercise here is §3.3.8's alignment-free separation split,
    /// `docs/decisions/mono-ruby-separation-split.md`), so this is the allowlisted item for
    /// the nearest of the three, the practice `docs/direction-sites.toml` already
    /// establishes for a `compose.rs` fixture that needs a direction and exercises none of
    /// them either.
    fn horizontal() -> Direction {
        Direction::Horizontal
    }

    /// A one-em square size, base or ruby depending on `em`.
    fn scale(em: i32) -> Scale {
        Scale::square(Advance::new(em).unwrap()).expect("a positive em")
    }

    /// One item at `start`, `advance` wide, at the base size.
    fn item(start: u32, advance: i32) -> Item {
        Item::new(
            ByteOffset::new(start),
            InlineExtent::new(advance).unwrap(),
            ScaleId::BASE,
        )
    }

    /// Three one-em ideographs: 鬼, 門, 方 — §3.3.1's own example, all cl-19.
    fn three_kanji() -> ([Item; 3], [Scale; 1]) {
        ([item(0, 1000), item(3, 1000), item(6, 1000)], [scale(1000)])
    }

    /// A run of `n` ruby-sized items, `advance` wide each, over hiragana き repeated.
    fn ruby_items(n: usize, advance: i32) -> alloc::vec::Vec<Item> {
        (0..n)
            .map(|index| item(u32::try_from(index).unwrap().saturating_mul(3), advance))
            .collect()
    }

    #[test]
    fn a_mono_ruby_reading_longer_than_its_base_forces_space_on_both_cl19_neighbors() {
        let (base_items, base_scales) = three_kanji();
        let text = Text::new("鬼門方", &base_items, &base_scales).expect("three ideographs");

        // 門 (item 1) carries four 400-unit ruby characters: 1600 against a 1000-unit base,
        // a surplus of 600, split evenly by `docs/decisions/mono-ruby-separation-split.md`.
        let reading_items = ruby_items(4, 400);
        let reading_scales = [scale(400)];
        let annotation =
            Annotation::new("きももき", &reading_items, &reading_scales).expect("four kana");

        let runs = [RubyRun::new(
            ItemIndex::new(1)..ItemIndex::new(2),
            AnnotationIndex::new(0)..AnnotationIndex::new(4),
        )];
        let ruby = Ruby::new(
            text,
            ItemIndex::new(1)..ItemIndex::new(2),
            annotation,
            &runs,
            RubyStyle::MonoRuby,
        )
        .expect("one run over one base item");
        let declared = [ruby];
        let constructs = Constructs::over(text).with_ruby(&declared);

        let mut scratch = Lowered::new();
        let contribution = lower(&constructs, Policy::JLREQ, horizontal(), &mut scratch)
            .expect("a well-formed mono-ruby lowers");

        let mut separations: alloc::vec::Vec<_> = contribution.separations().to_vec();
        separations.sort_by_key(|separation| separation.after().get());
        assert_eq!(separations.len(), 2, "both neighbors of 門 are cl-19");
        assert_eq!(separations[0].after(), ItemIndex::new(0));
        assert_eq!(separations[0].least(), InlineExtent::new(300).unwrap());
        assert_eq!(separations[1].after(), ItemIndex::new(1));
        assert_eq!(separations[1].least(), InlineExtent::new(300).unwrap());
        assert!(
            contribution.rules_fired().any(|rule| rule
                == RuleId::ADJUSTMENTS_OF_RUBY_WITH_LENGTH_LONGER_THAN_THAT_OF_THE_BASE_CHARACTERS),
            "§3.3.8 fired because a genuine surplus was found"
        );
    }

    #[test]
    fn a_reading_no_longer_than_its_base_forces_no_space() {
        let (base_items, base_scales) = three_kanji();
        let text = Text::new("鬼門方", &base_items, &base_scales).expect("three ideographs");
        let reading_items = ruby_items(2, 500);
        let reading_scales = [scale(500)];
        let annotation =
            Annotation::new("きも", &reading_items, &reading_scales).expect("two kana");
        let runs = [RubyRun::new(
            ItemIndex::new(1)..ItemIndex::new(2),
            AnnotationIndex::new(0)..AnnotationIndex::new(2),
        )];
        let ruby = Ruby::new(
            text,
            ItemIndex::new(1)..ItemIndex::new(2),
            annotation,
            &runs,
            RubyStyle::MonoRuby,
        )
        .expect("one run over one base item");
        let declared = [ruby];
        let constructs = Constructs::over(text).with_ruby(&declared);

        let mut scratch = Lowered::new();
        let contribution = lower(&constructs, Policy::JLREQ, horizontal(), &mut scratch)
            .expect("a well-formed mono-ruby lowers");
        assert!(
            contribution.separations().is_empty(),
            "1000 units of reading against a 1000-unit base has no surplus to force"
        );
    }

    #[test]
    fn two_adjacent_oversized_runs_merge_by_the_greater_share_not_the_sum() {
        let (base_items, base_scales) = three_kanji();
        let text = Text::new("鬼門方", &base_items, &base_scales).expect("three ideographs");

        // 鬼 (item 0) carries a 600-unit surplus (300 trailing, toward the item 0/1
        // boundary); 門 (item 1) carries a 1200-unit surplus (600 leading, toward the same
        // boundary). The boundary they share must read 600, the greater share, not 900.
        let small_reading = ruby_items(4, 400); // 1600 against 1000: surplus 600
        let small_scales = [scale(400)];
        let small = Annotation::new("きももき", &small_reading, &small_scales).expect("four kana");

        let large_reading = ruby_items(4, 550); // 2200 against 1000: surplus 1200
        let large_scales = [scale(550)];
        let large = Annotation::new("かかかか", &large_reading, &large_scales).expect("four kana");

        let first_runs = [RubyRun::new(
            ItemIndex::new(0)..ItemIndex::new(1),
            AnnotationIndex::new(0)..AnnotationIndex::new(4),
        )];
        let second_runs = [RubyRun::new(
            ItemIndex::new(1)..ItemIndex::new(2),
            AnnotationIndex::new(0)..AnnotationIndex::new(4),
        )];
        let first = Ruby::new(
            text,
            ItemIndex::new(0)..ItemIndex::new(1),
            small,
            &first_runs,
            RubyStyle::MonoRuby,
        )
        .expect("one run over item 0");
        let second = Ruby::new(
            text,
            ItemIndex::new(1)..ItemIndex::new(2),
            large,
            &second_runs,
            RubyStyle::MonoRuby,
        )
        .expect("one run over item 1");
        let declared = [first, second];
        let constructs = Constructs::over(text).with_ruby(&declared);

        let mut scratch = Lowered::new();
        let contribution = lower(&constructs, Policy::JLREQ, horizontal(), &mut scratch)
            .expect("two well-formed mono-ruby runs lower");

        let shared = contribution
            .separations()
            .iter()
            .find(|separation| separation.after() == ItemIndex::new(0))
            .expect("both runs demand space at the item 0/1 boundary");
        assert_eq!(
            shared.least(),
            InlineExtent::new(600).unwrap(),
            "the shared boundary reads the greater of the two demands, not their sum"
        );
    }

    #[test]
    fn a_hiragana_neighbor_forces_no_space() {
        // Two items: one ideograph carrying an oversized reading, one hiragana beside it.
        // §3.3.8 rule 1 is a prohibition about cl-19 alone; the kana budget is
        // `Question::RUBY_OVERHANG_KANA`'s own unfilled slot this round.
        let items = [item(0, 1000), item(3, 1000)];
        let scales = [scale(1000)];
        let text = Text::new("鬼き", &items, &scales).expect("one ideograph, one hiragana");
        let reading_items = ruby_items(4, 400);
        let reading_scales = [scale(400)];
        let annotation =
            Annotation::new("きももき", &reading_items, &reading_scales).expect("four kana");
        let runs = [RubyRun::new(
            ItemIndex::new(0)..ItemIndex::new(1),
            AnnotationIndex::new(0)..AnnotationIndex::new(4),
        )];
        let ruby = Ruby::new(
            text,
            ItemIndex::new(0)..ItemIndex::new(1),
            annotation,
            &runs,
            RubyStyle::MonoRuby,
        )
        .expect("one run over the ideograph");
        let declared = [ruby];
        let constructs = Constructs::over(text).with_ruby(&declared);

        let mut scratch = Lowered::new();
        let contribution = lower(&constructs, Policy::JLREQ, horizontal(), &mut scratch)
            .expect("a well-formed mono-ruby lowers");
        assert!(
            contribution.separations().is_empty(),
            "the only neighbor is hiragana, not cl-19, and there is no leading neighbor at all"
        );
    }

    #[test]
    fn overlapping_ruby_is_refused() {
        let (base_items, base_scales) = three_kanji();
        let text = Text::new("鬼門方", &base_items, &base_scales).expect("three ideographs");
        let reading_items = ruby_items(1, 500);
        let reading_scales = [scale(500)];
        let annotation = Annotation::new("き", &reading_items, &reading_scales).expect("one kana");
        let first_runs = [RubyRun::new(
            ItemIndex::new(0)..ItemIndex::new(1),
            AnnotationIndex::new(0)..AnnotationIndex::new(1),
        )];
        let second_runs = [RubyRun::new(
            ItemIndex::new(0)..ItemIndex::new(1),
            AnnotationIndex::new(0)..AnnotationIndex::new(1),
        )];
        let first = Ruby::new(
            text,
            ItemIndex::new(0)..ItemIndex::new(1),
            annotation,
            &first_runs,
            RubyStyle::MonoRuby,
        )
        .expect("one run over item 0");
        let second = Ruby::new(
            text,
            ItemIndex::new(0)..ItemIndex::new(1),
            annotation,
            &second_runs,
            RubyStyle::MonoRuby,
        )
        .expect("one run over item 0, declared a second time");
        let declared = [first, second];
        let constructs = Constructs::over(text).with_ruby(&declared);

        let mut scratch = Lowered::new();
        let refused = lower(&constructs, Policy::JLREQ, horizontal(), &mut scratch)
            .expect_err("two rubies claim item 0");
        assert!(matches!(refused, LowerError::OverlappingConstructs { .. }));
    }

    #[test]
    fn group_ruby_gets_one_run_and_a_block_demand_but_no_separation() {
        let (base_items, base_scales) = three_kanji();
        let text = Text::new("鬼門方", &base_items, &base_scales).expect("three ideographs");
        let reading_items = ruby_items(6, 500);
        let reading_scales = [scale(500)];
        let annotation =
            Annotation::new("かかかかかか", &reading_items, &reading_scales).expect("six kana");
        let runs = [RubyRun::new(
            ItemIndex::new(0)..ItemIndex::new(3),
            AnnotationIndex::new(0)..AnnotationIndex::new(6),
        )];
        let ruby = Ruby::new(
            text,
            ItemIndex::new(0)..ItemIndex::new(3),
            annotation,
            &runs,
            RubyStyle::GroupRuby,
        )
        .expect("one run over the whole base");
        let declared = [ruby];
        let constructs = Constructs::over(text).with_ruby(&declared);

        let mut scratch = Lowered::new();
        let contribution = lower(&constructs, Policy::JLREQ, horizontal(), &mut scratch)
            .expect("a well-formed group-ruby lowers");

        assert!(
            contribution.separations().is_empty(),
            "§3.3.6's ruby-not-longer-than-base geometry is real now, but it is entirely \
             `crate::place`'s own placement-time question; `lower` still emits no \
             `Separation` for it, and would only ever need one for paragraph 3's own \
             base-spreading half"
        );
        assert!(
            contribution.rules_fired().any(
                |rule| rule == RuleId::CHOICE_OF_SIDES_FOR_RUBY_WITH_RESPECT_TO_BASE_CHARACTERS
            ),
            "§3.3.4's side choice is genuinely computed for group-ruby too"
        );
        assert!(
            !contribution.rules_fired().any(|rule| {
                rule == RuleId::POSITIONING_OF_GROUP_RUBY_WITH_RESPECT_TO_BASE_CHARACTERS
            }),
            "§3.3.6 is not fired by `lower`: the length comparison, distribution and decline \
             are all `crate::place`'s to compute, against a composed line's own placements \
             this function never sees, and `crate::place` itself records no `RuleId` either \
             — the geometry is observable through the `Attachment`s and declined construct \
             refs it emits (ADR-0019, one fact one carrier)"
        );
        assert_eq!(contribution.block_demand().len(), 1);
        for item in [ItemIndex::new(0), ItemIndex::new(1), ItemIndex::new(2)] {
            let run = contribution
                .runs()
                .of(item)
                .expect("every base item joined the run");
            assert_eq!(
                contribution.construct_of(run.run()).ordinal(),
                0,
                "the one group-ruby run is the first (and only) construct declared"
            );
        }
    }

    #[test]
    fn jukugo_ruby_shares_one_run_but_gives_each_base_item_its_own_group() {
        let (base_items, base_scales) = three_kanji();
        let text = Text::new("鬼門方", &base_items, &base_scales).expect("three ideographs");
        let reading_items = ruby_items(3, 500);
        let reading_scales = [scale(500)];
        let annotation =
            Annotation::new("かもき", &reading_items, &reading_scales).expect("three kana");
        let runs = [
            RubyRun::new(
                ItemIndex::new(0)..ItemIndex::new(1),
                AnnotationIndex::new(0)..AnnotationIndex::new(1),
            ),
            RubyRun::new(
                ItemIndex::new(1)..ItemIndex::new(2),
                AnnotationIndex::new(1)..AnnotationIndex::new(2),
            ),
            RubyRun::new(
                ItemIndex::new(2)..ItemIndex::new(3),
                AnnotationIndex::new(2)..AnnotationIndex::new(3),
            ),
        ];
        let ruby = Ruby::new(
            text,
            ItemIndex::new(0)..ItemIndex::new(3),
            annotation,
            &runs,
            RubyStyle::JukugoRuby,
        )
        .expect("one run per base item, one jukugo compound");
        let declared = [ruby];
        let constructs = Constructs::over(text).with_ruby(&declared);

        let mut scratch = Lowered::new();
        let contribution = lower(&constructs, Policy::JLREQ, horizontal(), &mut scratch)
            .expect("a well-formed jukugo-ruby lowers");

        let first = contribution
            .runs()
            .of(ItemIndex::new(0))
            .expect("item 0 joined a run");
        let second = contribution
            .runs()
            .of(ItemIndex::new(1))
            .expect("item 1 joined a run");
        assert_eq!(
            first.run(),
            second.run(),
            "the whole compound is one run (§B.2#11, §C.2#8)"
        );
        assert_ne!(
            first.group(),
            second.group(),
            "each base item keeps its own group (§C.2#8)"
        );
        assert_eq!(
            contribution.block_demand().len(),
            3,
            "one demand per declared run"
        );
        assert!(
            contribution.rules_fired().any(
                |rule| rule == RuleId::CHOICE_OF_SIDES_FOR_RUBY_WITH_RESPECT_TO_BASE_CHARACTERS
            ),
            "§3.3.4's side choice is genuinely computed for jukugo-ruby too"
        );
        assert!(
            !contribution.rules_fired().any(|rule| {
                rule == RuleId::POSITIONING_OF_JUKUGO_RUBY_WITH_RESPECT_TO_BASE_CHARACTERS
            }),
            "§3.3.7 is not fired by `lower`: the ≤2-char discrimination and the delegation to \
             §3.3.5's method are real now, but they are `crate::place`'s to compute, against \
             a composed line's own placements this function never sees; `lower_jukugo` only \
             ever allocates run/group identity and block demand, and `crate::place` itself \
             records no `RuleId` either, for the identical reason its own module doc already \
             argues for §3.3.6 (ADR-0019, one fact one carrier)"
        );
    }

    #[test]
    fn default_alignment_resolves_to_nakatsuki_and_is_never_discouraged() {
        let (base_items, base_scales) = three_kanji();
        let text = Text::new("鬼門方", &base_items, &base_scales).expect("three ideographs");
        let reading_items = ruby_items(1, 500);
        let reading_scales = [scale(500)];
        let annotation = Annotation::new("き", &reading_items, &reading_scales).expect("one kana");
        let runs = [RubyRun::new(
            ItemIndex::new(0)..ItemIndex::new(1),
            AnnotationIndex::new(0)..AnnotationIndex::new(1),
        )];
        let ruby = Ruby::new(
            text,
            ItemIndex::new(0)..ItemIndex::new(1),
            annotation,
            &runs,
            RubyStyle::MonoRuby,
        )
        .expect("one run over one base item");
        let declared = [ruby];
        let constructs = Constructs::over(text).with_ruby(&declared);

        let mut scratch = Lowered::new();
        let contribution = lower(&constructs, Policy::JLREQ, horizontal(), &mut scratch)
            .expect("a well-formed mono-ruby lowers");
        let run = contribution
            .runs()
            .of(ItemIndex::new(0))
            .expect("the base item joined a run");
        let construct = contribution.construct_of(run.run());
        assert_eq!(
            contribution.alignment_of(construct),
            Some(RubyAlignment::Nakatsuki),
            "Policy::JLREQ follows §3.3.5's own recommendation, which is nakatsuki, absent \
             a per-construct override"
        );
        assert!(
            !contribution.alignment_discouraged(construct),
            "nakatsuki is never the discouraged combination, in any direction"
        );
    }

    #[test]
    fn katatsuki_is_honored_and_discouraged_only_in_horizontal_writing() {
        // Both `Direction` variants are named directly here rather than through
        // `horizontal()` above, because unlike every other fixture in this module, this
        // test's own point is exactly that §3.3.5's discouraged combination — katatsuki in
        // horizontal writing — differs from an ordinary one by direction. That is a genuine
        // read of §3.3.5's own direction-conditional clause, which is why this item gets its
        // own `docs/direction-sites.toml` entry rather than reusing `horizontal()`'s, whose
        // own `why` states its callers read none of the three direction-conditional rules.
        let (base_items, base_scales) = three_kanji();
        let text = Text::new("鬼門方", &base_items, &base_scales).expect("three ideographs");
        let reading_items = ruby_items(1, 500);
        let reading_scales = [scale(500)];
        let annotation = Annotation::new("き", &reading_items, &reading_scales).expect("one kana");
        let runs = [RubyRun::new(
            ItemIndex::new(0)..ItemIndex::new(1),
            AnnotationIndex::new(0)..AnnotationIndex::new(1),
        )];
        let ruby = Ruby::new(
            text,
            ItemIndex::new(0)..ItemIndex::new(1),
            annotation,
            &runs,
            RubyStyle::MonoRuby,
        )
        .expect("one run over one base item")
        .with_alignment(RubyAlignment::Katatsuki);
        let declared = [ruby];
        let constructs = Constructs::over(text).with_ruby(&declared);

        for (direction, expect_discouraged) in
            [(Direction::Vertical, false), (Direction::Horizontal, true)]
        {
            let mut scratch = Lowered::new();
            let contribution = lower(&constructs, Policy::JLREQ, direction, &mut scratch).expect(
                "§3.3.5's recommendation against katatsuki in horizontal writing is honored, \
                 never refused (ADR-0011)",
            );
            let run = contribution
                .runs()
                .of(ItemIndex::new(0))
                .expect("the base item joined a run");
            let construct = contribution.construct_of(run.run());
            assert_eq!(
                contribution.alignment_of(construct),
                Some(RubyAlignment::Katatsuki),
                "the per-construct override wins over Policy::JLREQ's own nakatsuki default \
                 (ADR-0019's precedence rule), regardless of direction"
            );
            assert_eq!(
                contribution.alignment_discouraged(construct),
                expect_discouraged,
                "§3.3.5's own discouraged combination is katatsuki specifically in horizontal \
                 writing, not katatsuki alone"
            );
        }
    }
}
