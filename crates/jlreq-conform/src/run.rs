// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The trait an implementation supplies, and the runner that measures it.
//!
//! Eight methods, each taking data and returning data, and each returning `Option` so that
//! `None` means *not attempted* rather than *failed*. Chrome implements the boundary
//! question and will never expose anything resembling our classification; Typst implements
//! composition and nothing else. Under a non-optional trait both would score as
//! catastrophic failures and the suite would be discarded as hostile by exactly the people
//! it exists to serve (`docs/design/conformance.md`).
//!
//! # Which permitted entry an answer is measured against
//!
//! A `policy` in a case is an **overlay** on `Policy::JLREQ`, not an identity: a partial map
//! from question path to choice name. An entry *applies* to a declared policy when every
//! question it names has that value in the declared policy, so `{}` applies to every
//! policy; the entry *selected* is the applying one that names the most questions.
//! `conform --check` makes that unique by requiring a case's key sets to be totally ordered
//! by inclusion, so the most specific applying entry always exists and is always unique.
//!
//! An implementation that declares nothing is measured against every permitted entry and
//! agrees if it matches any, which is what makes the suite runnable against a browser or
//! InDesign; the report then says which reading it is closest to rather than only that it
//! differs from ours.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use crate::case::{
    Case, CaseAmount, CaseFile, CasePolicy, Expect, ExpectBoundary, ExpectClass, ExpectExpansion,
    ExpectLine, ExpectLower, ExpectPlace, ExpectPullUp, Permitted, Suite,
};

/// What an implementation supplies to be measured.
pub trait Compose {
    /// How a report names this implementation.
    fn name(&self) -> &str;

    /// The policy this implementation claims to follow, if any: a map from question path to
    /// choice name, against which each case's `permitted` overlays are matched by the
    /// selection rule. An implementation declaring none is checked against every permitted
    /// outcome rather than one.
    fn declared_policy(&self) -> Option<CasePolicy> {
        None
    }

    /// The class number, 1 through 30, of one item. JLReq: §3.9.2, §A
    fn classify(&self, input: &crate::case::CaseInput, item: usize) -> Option<CaseClass>;

    /// The spacing, breakability and placement at one boundary: an interior one between
    /// `before` and the item after it when `edge` is `None`, or the line edge `edge` names,
    /// adjacent to `before`, when it is `Some` — `cases.schema.json`'s own `boundary.edge`,
    /// read alongside `before` from the same first stated expectation. JLReq: §B, §C
    fn boundary(
        &self,
        input: &crate::case::CaseInput,
        before: usize,
        edge: Option<Edge>,
    ) -> Option<CaseBoundary>;

    /// The composed lines. JLReq: §3.8, §D, §E
    fn compose(&self, input: &crate::case::CaseInput) -> Option<CaseOutput>;

    /// The single line `jlreq_line::align` produces for a run shorter than the target
    /// length. JLReq: §3.5.3, §3.7.3
    ///
    /// Required rather than defaulted to `None`, matching `classify`, `boundary` and
    /// `compose` above: every other question on this trait is answered by writing the
    /// method and declining per-input by returning `None`, never by omitting the method
    /// itself, and a default impl that also returns `None` would be a second, silent way to
    /// decline that the other three do not have — an implementation that forgets `align`
    /// would compile and read as "not attempted" indistinguishably from one that tried and
    /// genuinely has nothing to answer. Requiring it keeps "not attempted" a fact every
    /// implementation states on purpose, for this question exactly as for the other three.
    fn align(&self, input: &crate::case::CaseInput) -> Option<CaseOutput>;

    /// The runs `jlreq_line::tab_line` places for one caller-declared tab line: one
    /// `CaseLine` per placed run, in the case's own `tab_starts` order. JLReq: §3.6.1,
    /// §3.6.2, §3.6.3
    ///
    /// Required rather than defaulted to `None`, for the identical reason `align`'s own
    /// doc gives directly above: every other question on this trait is declined per-input
    /// by the method itself returning `None`, never by the method being absent, and a
    /// default impl would be a second, silent way to decline that no other method here
    /// has. `tab` is this trait's fifth method, added once the case format gained a fifth
    /// `kind` to ask it with, not a case the argument above stops applying to.
    fn tab(&self, input: &crate::case::CaseInput) -> Option<CaseOutput>;

    /// Which of one caller-declared break candidate kinsoku leaves standing, and which rule
    /// refused it when it does not: `jlreq_line::Feasible::compute`'s own answer for the
    /// `candidate`-th entry of `input.candidates`, `candidate` itself read from the case's
    /// own `expect.feasible.candidate` the identical way `boundary`'s `before` is read from
    /// `expect.boundary.before`. JLReq: §C.2#6, §C.2#7, §C.2#8, §C.2#13
    ///
    /// Required rather than defaulted to `None`, for the identical reason `align`'s and
    /// `tab`'s own docs each give directly above: every other question on this trait is
    /// declined per-input by the method itself returning `None`, never by the method being
    /// absent, and a default impl would be a second, silent way to decline that no other
    /// method here has. `feasible` is this trait's sixth method, added once the case format
    /// gained a sixth `kind` to ask it with, not a case the argument above stops applying
    /// to — and the first of three methods for which a declared `constructs` object is not a
    /// limit this implementation inherits from `classify` and `boundary` but the very thing
    /// that makes an answer possible: `jlreq_line::Feasible::compute` already takes a
    /// `jlreq_unit::Runs` parameter, so an implementation that can build one from a case's
    /// own declared constructs answers a question no other method on this trait but `lower`
    /// and `place` can reach at all (`crates/jlreq-conform/src/kumihan.rs`'s own module doc
    /// states which constructs an implementation may honestly convert).
    fn feasible(&self, input: &crate::case::CaseInput, candidate: usize) -> Option<CaseFeasible>;

    /// What `jlreq_inline::lower` resolved for one declared ruby construct: its run identity
    /// against its neighbors (`same_run`, `cases.schema.json`'s own field), the forced
    /// boundary spacing §3.3.8 rule 1 computes, and, for `RubyStyle::MonoRuby` and
    /// `RubyStyle::JukugoRuby` alike, which `RubyAlignment` §3.3.5 resolved — §3.3.7¶1's own
    /// wholesale delegation to "the method described in §3.3.5" — and whether it is the
    /// discouraged combination. `construct` is the ordinal into `input.constructs.ruby`, read
    /// from the case's own `expect.lower.construct` the identical way `boundary`'s `before`
    /// and `feasible`'s `candidate` are read from their own first stated expectation.
    /// JLReq: §3.3.5, §3.3.7, §3.3.8
    ///
    /// Required rather than defaulted to `None`, for the identical reason every other
    /// question on this trait but `classify`, `boundary` and `compose` already states: a
    /// default impl would be a second, silent way to decline that no other method here has.
    /// `lower` is this trait's seventh method, added once the case format gained a seventh
    /// `kind` to ask it with, not a case the argument above stops applying to — and the
    /// second of three methods for which `constructs` is not a limit but the very subject:
    /// where `feasible` reads `constructs` alongside an otherwise-ordinary break-candidate
    /// question, `lower` asks nothing else at all — it is the one method on this trait that
    /// reaches `jlreq_inline::lower` directly rather than `jlreq_line`, and every fact it
    /// answers is one the inline-construct layer resolved before a boundary or a line ever
    /// entered the picture.
    fn lower(&self, input: &crate::case::CaseInput, construct: usize) -> Option<CaseLower>;

    /// What `jlreq_inline::place` computed for the case's own whole declared `Constructs`:
    /// every annotation it placed, in the order `place` walked the declared ruby —
    /// mono-ruby's own, group-ruby's own and a jukugo compound's own alike, under either of
    /// §3.3.7's own two paragraphs — and every run it declined to place, for one of
    /// `jlreq_inline::place::Attachments::declined`'s own four stated reasons — §3.3.5(c)'s
    /// own katatsuki-with-overflow choice, unresolved for want of a `Question` (task #81),
    /// reachable through a jukugo paragraph-1 run too; §3.3.6 paragraph 3's own
    /// ruby-longer-than-base half, which `place()` cannot perform at all
    /// (`crates/jlreq-conform/cases/3.3.6.json`'s own
    /// `3.3.6/group-ruby-placement/ruby-longer-than-the-base-declines` is the case that makes
    /// it reachable), reachable through a jukugo paragraph-2 compound answering
    /// `Question::JUKUGO_RUBY_LAYOUT`'s own `group` too; a jukugo compound answering
    /// `phonetic`, §F's own distribution being unimplemented; or a jukugo compound whose base
    /// range one `place` call's own line only partially covers. JLReq: §3.3.5, §3.3.6, §3.3.7
    ///
    /// Required rather than defaulted to `None`, for the identical reason every other
    /// question on this trait but `classify`, `boundary` and `compose` already states: a
    /// default impl would be a second, silent way to decline that no other method here has.
    /// `place` is this trait's eighth method, added once the case format gained an eighth
    /// `kind` to ask it with, not a case the argument above stops applying to — and the
    /// third method for which `constructs` is not a limit but the very subject, alongside
    /// `feasible` and `lower`.
    ///
    /// No ordinal parameter, unlike `boundary`'s `before`, `feasible`'s `candidate` and
    /// `lower`'s `construct`. Those three each ask about one *occurrence* — one boundary, one
    /// break candidate, one declared construct — so a case's first stated expectation can
    /// name which one and `ask` can hand it back unchanged. `place`'s own answer,
    /// `jlreq_inline::place::Attachments`, is not shaped that way: it is the whole call's own
    /// answer, every attachment it placed and every run it declined, with no selector that
    /// narrows it to "the k-th thing this call produced" the way the other three narrow to
    /// one occurrence of their own input. Inventing an ordinal here would invent a selector
    /// `place()` itself does not have, so this method takes only `input`, matching `align`'s,
    /// `tab`'s and `compose`'s own shape rather than `boundary`'s, `feasible`'s and
    /// `lower`'s — and this trait's own internal `Answer::Place` correspondingly carries no
    /// asked-ordinal, the identical reason `Answer::Composed` carries none today.
    fn place(&self, input: &crate::case::CaseInput) -> Option<CasePlace>;
}

/// A classification answer, with the reason if the implementation has one.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CaseClass {
    /// The class number §3.9.2 gives, 1 through 30.
    pub class: u8,
    /// The rules that decided it, when the implementation reports them.
    pub rules: Vec<String>,
}

/// Which line edge a boundary sits at, when it is not an interior adjacency between two
/// items of the same stream.
///
/// A named two-variant enum rather than a sentinel folded into `before` (a magic ordinal) or
/// a second `after` ordinal the format would otherwise need: the published address grammar
/// already treats a line edge as its own citable axis value — `line-head` and `line-end`,
/// `cases.schema.json`'s own `address` `$def` — so a case names one the same way the
/// specification's own six matrices do, rather than through a number this module would have
/// to explain a second time. Mirrors `jlreq_spacing::Adjacency`'s own two edge constructors,
/// `at_line_head` and `at_line_end`, one variant each.
///
/// JLReq: §B.1
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Edge {
    /// The boundary before the line's first item — Table 1 through 6's line-head row.
    Head,
    /// The boundary after the line's last item — Table 1 through 6's line-end column.
    End,
}

/// A boundary answer. The conditional spaces, never their sum (ADR 0014).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CaseBoundary {
    /// One conditional space per neighbor that contributes one.
    pub spaces: Vec<CaseSpace>,
    /// Whether a line may break here.
    pub breakable: bool,
    /// Whether the adjacency is permitted at all.
    pub permitted: bool,
    /// How far ruby may overhang here.
    pub ruby_overhang: Option<i64>,
    /// The boundary's own Table 6 opportunity, independent of `spaces` (ADR 0014, amended
    /// by ADR 0021).
    pub expansion: CaseExpansion,
    /// The rules that decided it.
    pub rules: Vec<String>,
}

/// A feasible-break answer for one of the caller's own candidates: whether kinsoku left it
/// standing, and the rules that decided it. Not `CaseBoundary`: spaces, placement, ruby
/// overhang and expansion say nothing about a candidate's own survival, and reusing that
/// type here would carry four fields this answer has no content for.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CaseFeasible {
    /// Whether kinsoku left this candidate standing (`true`, `Feasible::breaks()`) or
    /// refused it (`false`, `Feasible::rejected()`).
    pub breakable: bool,
    /// The rules that decided it.
    pub rules: Vec<String>,
}

/// A `jlreq_inline::lower` answer for one declared ruby construct: run identity, forced
/// boundary spacing, and, for `RubyStyle::MonoRuby` and `RubyStyle::JukugoRuby` alike, the
/// resolved alignment. Not `CaseBoundary` or `CaseFeasible`: none of spacing-at-a-boundary,
/// placement or a candidate's own survival is this answer's subject, which is a fact the
/// inline-construct layer resolved before either question is even reachable.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CaseLower {
    /// Per-item run identity across the whole base stream, opaque and scoped to this one
    /// answer — kumihan's own bookkeeping numbering, never a stable identity across calls or
    /// implementations — `None` for an item no construct covers. What a `same_run`
    /// expectation compares: two items share a run when both are `Some` and equal.
    pub runs: Vec<Option<u32>>,
    /// The complete list of forced boundary spacing across every construct the answer
    /// resolved, `(after, least units)` pairs in ascending `after` order — `jlreq_inline::
    /// Contribution::separations`, translated.
    pub separations: Vec<(usize, i64)>,
    /// The `RubyAlignment` resolved for the identified construct: `"nakatsuki"` or
    /// `"katatsuki"`. `None` for a construct `lower` never resolved one for —
    /// `RubyStyle::GroupRuby`, the one style §3.3.7¶1's own wholesale delegation to §3.3.5
    /// never reaches (`RubyStyle::MonoRuby` and `RubyStyle::JukugoRuby` alike resolve one).
    pub alignment: Option<String>,
    /// Whether that alignment is §3.3.5's own discouraged combination — katatsuki resolved
    /// in horizontal writing. `false` both for an ordinary resolution and for a construct
    /// `lower` never resolved one for.
    pub alignment_discouraged: bool,
    /// The rules that decided it.
    pub rules: Vec<String>,
}

/// A `jlreq_inline::place` answer for the case's own whole declared `Constructs`: every
/// annotation it placed, and every run it declined to place, mono-ruby, group-ruby or a
/// jukugo compound alike. Not `CaseLower`: this is the whole call's own answer rather than
/// one construct's, so it carries no `construct` ordinal of its own (`Compose::place`'s own
/// doc states why). Carries no `rules` either — `jlreq_inline::place::Attachments` publishes
/// none, for the identical reason `Compose::place`'s own doc states.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CasePlace {
    /// Every annotation `place` placed, in `Attachments::attachments`'s own walk order.
    pub attachments: Vec<CaseAttachment>,
    /// The declared `constructs.ruby` ordinal of every run `place` declined — mono-ruby's own
    /// katatsuki-with-overflow choice, group-ruby's own ruby-longer-than-base half, or either
    /// of jukugo's own two further reasons alike (`Compose::place`'s own doc states all
    /// four) — `Attachments::declined`, translated through `ConstructRef::ordinal`, the
    /// inverse of `Compose::lower`'s own `construct` parameter over the identical slice.
    pub declined: Vec<usize>,
}

/// One placed annotation character: `jlreq_inline::place::Attachment`, narrowed to the two
/// facts these cases turn on (`Compose::place`'s own doc, and `cases.schema.json`'s own
/// `attachment` description, both state why `size`, `side`, `run` and `construct` are not
/// carried here).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct CaseAttachment {
    /// This attachment's own inline-axis origin, in kumihan's own unit — `Attachment::
    /// inline`, translated. May be negative.
    pub inline: i64,
    /// The annotation stream's own item ordinal this attachment draws — `Attachment::item`.
    /// `None` only for a construct that repeats one member rather than placing a stream, no
    /// mono-ruby attachment ever among them.
    pub item: Option<usize>,
}

/// One conditional space, in kumihan's own unit.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CaseSpace {
    /// The amount.
    pub units: i64,
    /// Whose em the amount is a fraction of: `preceding` or `trailing`.
    pub referent: String,
    /// How the amount shrinks: `rigid`, `range` or `discrete`.
    pub reduction: String,
    /// The floor the amount shrinks to.
    pub floor_units: i64,
    /// Which ladder the stage belongs to. Only ever `"reduction"` now (ADR 0021):
    /// Appendix E's own stage lives on [`CaseExpansion::stage`] instead.
    pub ladder: String,
    /// The stage of that ladder.
    pub stage: u8,
}

/// One boundary's own expansion opportunity, in kumihan's own unit (ADR 0014, amended by
/// ADR 0021: a fact about the coordinate, not about either neighbor's own contribution).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CaseExpansion {
    /// `"none"`, `"range"` or `"residual"`.
    pub kind: String,
    /// The ceiling, in kumihan's own unit. `None` outside `kind: "range"`.
    pub ceiling_units: Option<i64>,
    /// The priority stage. `None` outside `kind: "range"`.
    pub stage: Option<u8>,
    /// Which rule states this coordinate's row of Table 6, by address — `jlreq_spacing::
    /// Boundary::expansion_rule`'s own answer, rendered. `None` for an implementation that
    /// publishes no specification address (most of them, ADR 0006) *and* for a coordinate
    /// Table 6 carries no row for at all; `check_expansion` never distinguishes the two,
    /// which is the same "not attempted" reading `check_class`'s own doc already gives an
    /// implementation that answers a question without citing a rule for it — the field
    /// cannot itself tell a decliner apart from an honest absence, only the implementation
    /// that knows which one it meant can.
    pub rule: Option<String>,
}

/// The composed lines, and every rule the composition could not satisfy.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CaseOutput {
    /// The lines, in order.
    pub lines: Vec<CaseLine>,
    /// Every rule the composition could not satisfy, by address.
    pub violations: Vec<String>,
    /// The rules the composition fired.
    pub rules: Vec<String>,
}

/// One composed line. The three quantities are ADR 0017's, and none is reconstructed here.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CaseLine {
    /// The caller's own glyph-box origins on the line's inline axis.
    pub placements: Vec<i64>,
    /// The realized conditional space at the line end, in kumihan's own unit.
    pub trailing: i64,
    /// The line in normalized geometry.
    pub extent: i64,
    /// Every unit taken out of a supplied advance, with the sentence that took it.
    pub trims: Vec<CaseTrim>,
    /// The sub-lines of every segment touching this line.
    pub parts: Vec<CasePart>,
    /// How far hanging punctuation extends past the measure.
    pub hanging: Option<i64>,
    /// §3.1.12 ⑤'s repair as `Search::Optimal` applied it to this line — `jlreq_line::
    /// Line::pull_up`'s own answer, `None` on every line `Search::FirstFit` composes and
    /// on any line `Search::Optimal` never ran the comparison for.
    pub pull_up: Option<CasePullUp>,
}

/// §3.1.12 ⑤'s repair, in kumihan's own unit — `jlreq_line::PullUp` in the runner's own
/// vocabulary.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CasePullUp {
    /// How much this line's own reduction reclaimed.
    pub amount: i64,
    /// Which item moved up onto this line as a result.
    pub pulls: usize,
    /// The rule that states the repair, when the implementation publishes one.
    pub rule: Option<String>,
}

/// One unit count taken out of one item's supplied advance.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CaseTrim {
    /// Which item it was taken out of.
    pub item: usize,
    /// The amount, in kumihan's own unit.
    pub units: i64,
    /// Whose em the amount is a fraction of.
    pub referent: String,
    /// The sentence that took it.
    pub rule: String,
}

/// One sub-line of one segment.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CasePart {
    /// Which construct it belongs to.
    pub segment: usize,
    /// Which sub-line of that construct it is.
    pub index: usize,
    /// The half-open range of items it holds.
    pub items: (usize, usize),
    /// Its inline origin relative to the line's.
    pub inline: i64,
    /// Its block origin relative to the line's.
    pub block: i64,
    /// Its own extent.
    pub extent: i64,
    /// One block offset per interior item; non-empty only for §3.2.5.
    pub across: Vec<i64>,
}

impl CaseClass {
    /// The class an implementation answers with, and the rules it says decided it.
    ///
    /// A constructor rather than a literal, because the answer types are
    /// `#[non_exhaustive]`: a field added here is then a field an implementation that
    /// already compiles keeps compiling without (ADR 0012). Pass an empty `rules` where the
    /// implementation publishes no specification address, which is most of them.
    #[must_use]
    pub fn new(class: u8, rules: Vec<String>) -> Self {
        Self { class, rules }
    }
}

impl CaseBoundary {
    /// The answer at one boundary: the conditional spaces, never their sum (ADR 0014).
    #[must_use]
    pub fn new(
        spaces: Vec<CaseSpace>,
        breakable: bool,
        permitted: bool,
        ruby_overhang: Option<i64>,
        expansion: CaseExpansion,
        rules: Vec<String>,
    ) -> Self {
        Self {
            spaces,
            breakable,
            permitted,
            ruby_overhang,
            expansion,
            rules,
        }
    }
}

impl CaseFeasible {
    /// The answer for one candidate: whether kinsoku left it standing, and why.
    #[must_use]
    pub fn new(breakable: bool, rules: Vec<String>) -> Self {
        Self { breakable, rules }
    }
}

impl CaseLower {
    /// The answer for one declared ruby construct.
    #[must_use]
    pub fn new(
        runs: Vec<Option<u32>>,
        separations: Vec<(usize, i64)>,
        alignment: Option<String>,
        alignment_discouraged: bool,
        rules: Vec<String>,
    ) -> Self {
        Self {
            runs,
            separations,
            alignment,
            alignment_discouraged,
            rules,
        }
    }
}

impl CasePlace {
    /// The answer for the case's own whole declared `Constructs`.
    #[must_use]
    pub fn new(attachments: Vec<CaseAttachment>, declined: Vec<usize>) -> Self {
        Self {
            attachments,
            declined,
        }
    }
}

impl CaseAttachment {
    /// One placed annotation character.
    #[must_use]
    pub fn new(inline: i64, item: Option<usize>) -> Self {
        Self { inline, item }
    }
}

impl CaseExpansion {
    /// One boundary's own expansion opportunity.
    #[must_use]
    pub fn new(
        kind: String,
        ceiling_units: Option<i64>,
        stage: Option<u8>,
        rule: Option<String>,
    ) -> Self {
        Self {
            kind,
            ceiling_units,
            stage,
            rule,
        }
    }
}

impl CaseSpace {
    /// One neighbor's contribution to the space at a boundary.
    #[must_use]
    pub fn new(
        units: i64,
        referent: String,
        reduction: String,
        floor_units: i64,
        ladder: String,
        stage: u8,
    ) -> Self {
        Self {
            units,
            referent,
            reduction,
            floor_units,
            ladder,
            stage,
        }
    }
}

impl CaseOutput {
    /// The composed lines, what the composition could not satisfy, and what it fired.
    #[must_use]
    pub fn new(lines: Vec<CaseLine>, violations: Vec<String>, rules: Vec<String>) -> Self {
        Self {
            lines,
            violations,
            rules,
        }
    }
}

impl CaseLine {
    /// One composed line, in the normalized geometry ADR 0017 fixes.
    #[must_use]
    pub fn new(
        placements: Vec<i64>,
        trailing: i64,
        extent: i64,
        trims: Vec<CaseTrim>,
        parts: Vec<CasePart>,
        hanging: Option<i64>,
        pull_up: Option<CasePullUp>,
    ) -> Self {
        Self {
            placements,
            trailing,
            extent,
            trims,
            parts,
            hanging,
            pull_up,
        }
    }
}

impl CasePullUp {
    /// §3.1.12 ⑤'s repair as applied to one line.
    #[must_use]
    pub fn new(amount: i64, pulls: usize, rule: Option<String>) -> Self {
        Self {
            amount,
            pulls,
            rule,
        }
    }
}

impl CaseTrim {
    /// One unit count taken out of one item's supplied advance, with the sentence that
    /// took it.
    #[must_use]
    pub fn new(item: usize, units: i64, referent: String, rule: String) -> Self {
        Self {
            item,
            units,
            referent,
            rule,
        }
    }
}

impl CasePart {
    /// One sub-line of one segment, relative to the line's own origin.
    #[must_use]
    pub fn new(
        segment: usize,
        index: usize,
        items: (usize, usize),
        inline: i64,
        block: i64,
        extent: i64,
        across: Vec<i64>,
    ) -> Self {
        Self {
            segment,
            index,
            items,
            inline,
            block,
            extent,
            across,
        }
    }
}

/// What one run found.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct Report {
    /// How many cases the implementation answered.
    pub attempted: usize,
    /// How many of those answers the specification permits.
    pub agreed: usize,
    /// The ones it does not, each with the sentence and every permitted reading.
    pub disagreed: Vec<Disagreement>,
    /// How many cases the implementation did not attempt.
    pub skipped: usize,
    /// How many permitted entries no declared policy of this run could select, because the
    /// implementation's policy does not have the question the entry names.
    ///
    /// A published reading nothing can select is evaluated by nothing, which is a silence a
    /// green run would otherwise keep: a case may carry three entries and assert only what
    /// its `{}` entry says. The number is reported rather than judged — an implementation
    /// with a smaller policy surface than the suite is exactly what ADR 0006 expects — and it
    /// is what makes "this case has three readings" and "this run measured one of them" two
    /// different statements.
    pub unselectable: usize,
    /// Every rule the run actually exercised. Drives the exercised-coverage gate.
    pub rules_exercised: BTreeSet<String>,
}

impl Report {
    /// Fold another report into this one, so a per-file run and a whole-suite run are the
    /// same arithmetic.
    pub fn absorb(&mut self, other: Self) {
        self.attempted = self.attempted.saturating_add(other.attempted);
        self.agreed = self.agreed.saturating_add(other.agreed);
        self.skipped = self.skipped.saturating_add(other.skipped);
        self.unselectable = self.unselectable.saturating_add(other.unselectable);
        self.disagreed.extend(other.disagreed);
        self.rules_exercised.extend(other.rules_exercised);
    }

    /// One line per count, for a runner's output.
    #[must_use]
    pub fn census(&self) -> String {
        format!(
            "{attempted} attempted, {agreed} agreed, {disagreed} disagreed, {skipped} not \
             attempted, {unselectable} permitted entr(ies) this policy cannot select",
            attempted = self.attempted,
            agreed = self.agreed,
            disagreed = self.disagreed.len(),
            skipped = self.skipped,
            unselectable = self.unselectable,
        )
    }
}

/// One case an implementation answers differently from every reading the specification
/// permits.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Disagreement {
    /// Which case.
    pub case: String,
    /// The rules the case names.
    pub rules: Vec<String>,
    /// The specification sentence, quoted, so the report is readable without our source.
    pub statement: String,
    /// Every outcome the specification permits, so the report says which reading the
    /// implementation is closest to.
    pub permitted: Vec<String>,
    /// What the implementation answered.
    pub got: String,
}

impl Disagreement {
    /// The finding as one paragraph.
    #[must_use]
    pub fn message(&self) -> String {
        let mut out = format!("{case}: answered {got}", case = self.case, got = self.got);
        for reading in &self.permitted {
            let _ = write!(out, "\n    permitted: {reading}");
        }
        let _ = write!(out, "\n    {statement}", statement = self.statement);
        out
    }
}

/// Run every case of a suite against one implementation.
#[must_use]
pub fn run<C: Compose + ?Sized>(suite: &Suite, implementation: &C) -> Report {
    let mut report = Report::default();
    for file in suite.files() {
        report.absorb(run_file(file, implementation));
    }
    report
}

/// Run every case of one file, which is the unit one generated test covers.
#[must_use]
pub fn run_file<C: Compose + ?Sized>(file: &CaseFile, implementation: &C) -> Report {
    let declared = implementation.declared_policy();
    let mut report = Report::default();
    for case in file.cases() {
        measure(case, implementation, declared.as_ref(), &mut report);
    }
    report
}

/// Ask one case's question, and record what came back.
fn measure<C: Compose + ?Sized>(
    case: &Case,
    implementation: &C,
    declared: Option<&CasePolicy>,
    report: &mut Report,
) {
    report.unselectable = report
        .unselectable
        .saturating_add(unselectable(case, declared));
    let Some(answer) = ask(case, implementation) else {
        report.skipped = report.skipped.saturating_add(1);
        return;
    };
    report.attempted = report.attempted.saturating_add(1);
    report.rules_exercised.extend(answer.rules());
    let readings = applicable(case, declared);
    let agrees = !readings.is_empty()
        && readings
            .iter()
            .any(|entry| check(&entry.expect, &answer).is_empty());
    let excluded = case
        .forbidden()
        .iter()
        .find(|entry| !entry.expect.is_silent() && check(&entry.expect, &answer).is_empty());
    if agrees && excluded.is_none() {
        report.agreed = report.agreed.saturating_add(1);
        return;
    }
    let statement = match excluded {
        Some(entry) => format!(
            "{quote}\n    forbidden: {why}",
            quote = case.quote(),
            why = entry.why
        ),
        None => case.quote().to_owned(),
    };
    report.disagreed.push(Disagreement {
        case: case.id().to_owned(),
        rules: case.rules().to_vec(),
        statement,
        permitted: readings.iter().map(|entry| describe(entry)).collect(),
        got: answer.render(),
    });
}

/// The permitted entries an answer is measured against.
///
/// An implementation that declares a policy is measured against the one entry the selection
/// rule picks; one that declares nothing is measured against all of them and agrees if it
/// matches any.
fn applicable<'a>(case: &'a Case, declared: Option<&CasePolicy>) -> Vec<&'a Permitted> {
    let Some(declared) = declared else {
        return case.permitted().iter().collect();
    };
    case.permitted()
        .iter()
        .filter(|entry| {
            entry
                .policy
                .iter()
                .all(|(question, choice)| declared.get(question) == Some(choice))
        })
        .max_by_key(|entry| entry.policy.len())
        .into_iter()
        .collect()
}

/// How many of one case's permitted entries the declared policy could never select.
///
/// An entry naming a question the declared policy does not have applies to nothing, so it is
/// neither matched nor reported as a difference — it is simply not evaluated. That is the
/// right behavior and the wrong silence: at M0 the generated policy space is empty, so
/// every entry carrying an overlay is in this state and a case with three published readings
/// asserts what its `{}` entry says and nothing more.
///
/// An implementation that declares no policy is measured against every entry, so nothing is
/// unselectable for it.
fn unselectable(case: &Case, declared: Option<&CasePolicy>) -> usize {
    let Some(declared) = declared else {
        return 0;
    };
    case.permitted()
        .iter()
        .filter(|entry| {
            entry
                .policy
                .keys()
                .any(|question| declared.get(question).is_none())
        })
        .count()
}

/// How a report names one permitted reading.
fn describe(entry: &Permitted) -> String {
    let overlay = if entry.policy.is_empty() {
        "policy {}".to_owned()
    } else {
        entry
            .policy
            .iter()
            .map(|(question, choice)| format!("{question} = {choice}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "{rendered} under {overlay} — {source}",
        rendered = render_expect(&entry.expect),
        source = entry.source
    )
}

/// One answer, whichever of the eight questions the case asked.
///
/// Six variants for eight questions: `align` and `tab` both share [`Self::Composed`] with
/// `compose` rather than each adding a variant of their own (`ask`'s own doc, and
/// `Compose::align`'s and `Compose::tab`'s); `feasible`, `lower` and `place` do not join
/// them, because none of the three answers — one candidate's own survival and the rule that
/// decided it, one construct's own run identity, forced spacing and resolved alignment, or
/// the whole call's own placed annotations and declined runs — is anything like a composed
/// line (`Compose::feasible`'s, `Compose::lower`'s and `Compose::place`'s own docs).
#[derive(Debug)]
enum Answer {
    /// A classification answer about one item.
    Class(usize, CaseClass),
    /// A boundary answer about one boundary.
    Boundary(usize, CaseBoundary),
    /// The composed lines, the single line an `align` case asked for, or the placed runs
    /// of a `tab` case.
    Composed(CaseOutput),
    /// A feasible-break answer about one of the caller's own candidates.
    Feasible(usize, CaseFeasible),
    /// A `jlreq_inline::lower` answer about one declared ruby construct.
    Lower(usize, CaseLower),
    /// A `jlreq_inline::place` answer about the case's own whole declared `Constructs`. No
    /// asked-ordinal, `Compose::place`'s own doc states why.
    Place(CasePlace),
}

impl Answer {
    /// The rules the implementation says it fired.
    fn rules(&self) -> Vec<String> {
        match self {
            Self::Class(_, answer) => answer.rules.clone(),
            Self::Boundary(_, answer) => answer.rules.clone(),
            Self::Composed(answer) => answer.rules.clone(),
            Self::Feasible(_, answer) => answer.rules.clone(),
            Self::Lower(_, answer) => answer.rules.clone(),
            // `CasePlace` carries no `rules` field (`Compose::place`'s own doc), so a
            // `place` answer never contributes to the exercised-coverage gate through this
            // path; §3.3.5's own citation is `lower`'s to publish, and it already does.
            Self::Place(_) => Vec::new(),
        }
    }

    /// Which of the eight questions produced it, as a report writes it.
    fn question(&self) -> String {
        match self {
            Self::Class(item, _) => format!("the class of item {item}"),
            Self::Boundary(before, _) => format!("the boundary before item {before}"),
            Self::Composed(_) => "the composed lines".to_owned(),
            Self::Feasible(candidate, _) => format!("the feasibility of candidate {candidate}"),
            Self::Lower(construct, _) => format!("what lower resolved for construct {construct}"),
            Self::Place(_) => "what place resolved for the declared constructs".to_owned(),
        }
    }

    /// How a report writes it.
    fn render(&self) -> String {
        match self {
            Self::Class(item, answer) => format!(
                "class cl-{class:02} for item {item}{rules}",
                class = answer.class,
                rules = suffix(&answer.rules)
            ),
            Self::Boundary(before, answer) => format!(
                "breakable {breakable}, permitted {permitted}, {spaces} conditional space(s) \
                 before item {before}{rules}",
                breakable = answer.breakable,
                permitted = answer.permitted,
                spaces = answer.spaces.len(),
                rules = suffix(&answer.rules)
            ),
            Self::Composed(answer) => format!(
                "{lines} line(s), {violations} violation(s)",
                lines = answer.lines.len(),
                violations = answer.violations.len()
            ),
            Self::Feasible(candidate, answer) => format!(
                "breakable {breakable} for candidate {candidate}{rules}",
                breakable = answer.breakable,
                rules = suffix(&answer.rules)
            ),
            Self::Lower(construct, answer) => format!(
                "alignment {alignment:?}, discouraged {discouraged}, {separations} \
                 separation(s) for construct {construct}{rules}",
                alignment = answer.alignment,
                discouraged = answer.alignment_discouraged,
                separations = answer.separations.len(),
                rules = suffix(&answer.rules)
            ),
            Self::Place(answer) => format!(
                "{attachments} attachment(s), {declined} declined run(s)",
                attachments = answer.attachments.len(),
                declined = answer.declined.len()
            ),
        }
    }
}

/// The rules an answer names, as a trailing clause.
fn suffix(rules: &[String]) -> String {
    if rules.is_empty() {
        String::new()
    } else {
        format!(" (by {joined})", joined = rules.join(", "))
    }
}

/// Ask the question one case's `kind` names.
///
/// The ordinal a classification or a boundary case asks about is the expectation's own, and
/// every permitted entry of one case asks about the same one — a case is one input and one
/// question, with several answers to it. `conform --check` enforces that and so does
/// `check_ordinal`, which refuses to read an entry about another occurrence as an agreement.
/// A boundary case's `edge` is the same fact read alongside `before`, from the very same
/// first stated expectation, and `conform --check`'s own `check_question` holds every entry
/// to naming the identical one for the identical reason: an interior boundary and a line
/// edge next to the same item are two different questions, not two readings of one.
///
/// A `kind` this format does not have never reaches here: the reader refuses the case, so
/// the wildcard arm below is what `compose` names and not what everything else falls into
/// — `align` and `tab` each have their own arm precisely so that neither is one more kind
/// the wildcard silently swallows. This is not a formality: a `tab` arm forgotten here would
/// still compile, `case.input().kind == "tab"` would fall through to the wildcard, and
/// every published `tab` case would be silently asked `compose`'s own question instead —
/// answered with whatever `Kumihan::compose` makes of `tab_starts`/`tab_stops` (nothing,
/// since it reads neither field, so `None`, `candidates?` on an absent `measure` failing
/// first) or, worse, with a real but wrong answer if some future implementation's `compose`
/// happened to read the same input fields. Either way the case's own expectations would be
/// tested against the wrong question, which is exactly the ADR-0006 violation an unwired
/// `tab` case would tempt a reader to "fix" by editing the expectations to match rather than
/// noticing the wiring is missing.
///
/// `align` and `tab` both reuse [`Answer::Composed`] rather than a variant of their own:
/// [`Compose::align`] and [`Compose::tab`] each answer with the same [`CaseOutput`] shape
/// [`Compose::compose`] does — one or several [`CaseLine`]s — and every field of
/// `check_composed` below already means the right thing for both (`cases.schema.json`'s own
/// `kind` description, and this module's own `Compose::align` and `Compose::tab` docs for
/// why each trait method is a required method of its own rather than a further `Answer`
/// variant).
///
/// `feasible` is named explicitly too, and for the identical reason `align` and `tab` are —
/// but, unlike them, it does not reuse [`Answer::Composed`]: one candidate's own survival
/// and the rule that decided it is nothing like a composed line, so [`Answer::Feasible`]
/// exists rather than asking `check_composed` to mean a sixth thing it was never shaped for.
/// A forgotten `"feasible" =>` arm would fail exactly the way a forgotten `"tab"` arm once
/// could: `case.input().kind == "feasible"` would fall through to the wildcard and every
/// published `feasible` case would be silently asked `compose`'s own question instead —
/// `Kumihan::compose` reads no `expect.feasible.candidate` and no `constructs`-built
/// `Runs`, so the case's own expectations would be tested against the wrong layer entirely,
/// indistinguishable from a coincidental pass unless the two answer shapes cannot agree by
/// accident, which is exactly what this module's own test for this arm arranges
/// (`a_feasible_case_reaches_compose_feasible_and_not_compose_compose`, below).
///
/// `lower` is the seventh and identically named explicitly, for the identical reason and
/// with the identical hazard: a forgotten `"lower" =>` arm would fall through to the
/// wildcard, and every published `lower` case would be silently asked `compose`'s own
/// question — `Kumihan::compose` reads no `expect.lower.construct` and declines outright the
/// moment a case declares any construct at all (`crates/jlreq-conform/src/kumihan.rs`'s own
/// module doc), so a `lower` case misrouted this way would score as *not attempted* rather
/// than as an agreement, distinguishable but still the wrong layer answering. Like
/// `feasible`, it does not reuse [`Answer::Composed`]: one construct's own run identity,
/// forced spacing and resolved alignment is nothing like a composed line either, so
/// [`Answer::Lower`] exists for it.
///
/// `place` is the eighth and identically named explicitly, for the identical reason and with
/// the identical hazard: a forgotten `"place" =>` arm would fall through to the wildcard,
/// and every published `place` case would be silently asked `compose`'s own question —
/// `Kumihan::compose` reads no `constructs`-built `Constructs` and declines outright the
/// moment a case declares any construct at all, the identical decline `lower`'s own arm
/// names above, so a `place` case misrouted this way would score as *not attempted* rather
/// than as an agreement — the silent failure mode §11 of this round's own brief calls out
/// explicitly, and the reason `a_place_case_reaches_compose_place_and_not_compose_compose`
/// (below) exists. Like `feasible` and `lower`, it does not reuse [`Answer::Composed`]: the
/// whole call's own placed annotations and declined runs is nothing like a composed line
/// either, so [`Answer::Place`] exists for it — and unlike the other two, `place` reads no
/// ordinal at all from the case's own first stated expectation (`Compose::place`'s own doc
/// states why), so this arm hands `implementation.place` nothing but `case.input()`.
fn ask<C: Compose + ?Sized>(case: &Case, implementation: &C) -> Option<Answer> {
    match case.input().kind.as_str() {
        "classify" => {
            let item = case
                .permitted()
                .iter()
                .find_map(|entry| entry.expect.class.as_ref())
                .map_or(0, |class| class.item);
            implementation
                .classify(case.input(), item)
                .map(|answer| Answer::Class(item, answer))
        },
        "boundary" => {
            let boundary = case
                .permitted()
                .iter()
                .find_map(|entry| entry.expect.boundary.as_ref());
            let before = boundary.map_or(0, |boundary| boundary.before);
            let edge = edge_of(boundary);
            implementation
                .boundary(case.input(), before, edge)
                .map(|answer| Answer::Boundary(before, answer))
        },
        "align" => implementation.align(case.input()).map(Answer::Composed),
        "tab" => implementation.tab(case.input()).map(Answer::Composed),
        "feasible" => {
            let candidate = case
                .permitted()
                .iter()
                .find_map(|entry| entry.expect.feasible.as_ref())
                .map_or(0, |feasible| feasible.candidate);
            implementation
                .feasible(case.input(), candidate)
                .map(|answer| Answer::Feasible(candidate, answer))
        },
        "lower" => {
            let construct = case
                .permitted()
                .iter()
                .find_map(|entry| entry.expect.lower.as_ref())
                .map_or(0, |lower| lower.construct);
            implementation
                .lower(case.input(), construct)
                .map(|answer| Answer::Lower(construct, answer))
        },
        "place" => implementation.place(case.input()).map(Answer::Place),
        _ => implementation.compose(case.input()).map(Answer::Composed),
    }
}

/// The edge one boundary expectation names, in the runner's own vocabulary.
///
/// `None` for a case stating no `boundary` at all, for one stating no `edge`, or for a value
/// outside the schema's own two — every one of the last is a case `conform --check` would
/// already have refused, and reading it permissively here matches `direction_of`'s own
/// reading of a direction the format did not enumerate.
fn edge_of(boundary: Option<&ExpectBoundary>) -> Option<Edge> {
    match boundary?.edge.as_deref() {
        Some("head") => Some(Edge::Head),
        Some("end") => Some(Edge::End),
        _ => None,
    }
}

/// Every way one answer differs from one expectation. Empty means they agree.
///
/// A field the expectation does not state asserts nothing, so an expectation that states
/// only some of a line's quantities is matched by every answer agreeing on those — which is
/// the rule that lets a `forbidden` entry state only the fields it forbids, and why
/// `Expect::is_silent` decides whether such an entry excludes anything before this is
/// consulted.
///
/// That rule stops at the question. An expectation about a question the case did not ask, or
/// about a different occurrence of the input it did ask about, differs from the answer rather
/// than agreeing with it, and it differs for both sides of a case: as a `permitted` entry it
/// is satisfied by nothing, and as a `forbidden` one it excludes nothing. Reading it as
/// agreement did both harms at once — a `permitted` entry omitting the very field its case is
/// about could not fail, and a `forbidden` entry naming another item turned a correct answer
/// into a reported disagreement. `conform --check` refuses both shapes outright; this is the
/// runner refusing to read either as an answer in the meantime.
fn check(expect: &Expect, answer: &Answer) -> Vec<String> {
    match (expect, answer) {
        (
            Expect {
                class: Some(want), ..
            },
            Answer::Class(asked, got),
        ) => check_ordinal(want.item, *asked, "item").unwrap_or_else(|| check_class(want, got)),
        (
            Expect {
                boundary: Some(want),
                ..
            },
            Answer::Boundary(asked, got),
        ) => check_ordinal(want.before, *asked, "boundary before item")
            .unwrap_or_else(|| check_boundary(want, got)),
        (
            Expect {
                feasible: Some(want),
                ..
            },
            Answer::Feasible(asked, got),
        ) => check_ordinal(want.candidate, *asked, "feasible candidate")
            .unwrap_or_else(|| check_feasible(want, got)),
        (
            Expect {
                lower: Some(want), ..
            },
            Answer::Lower(asked, got),
        ) => check_ordinal(want.construct, *asked, "lower construct")
            .unwrap_or_else(|| check_lower(want, got)),
        (
            Expect {
                place: Some(want), ..
            },
            Answer::Place(got),
        ) => check_place(want, got),
        (
            Expect {
                lines: Some(lines),
                violations,
                ..
            },
            Answer::Composed(got),
        ) => check_composed(Some(lines), violations.as_deref(), got),
        (
            Expect {
                violations: Some(violations),
                ..
            },
            Answer::Composed(got),
        ) => check_composed(None, Some(violations), got),
        (_, answer) => vec![format!(
            "the case asked for {asked}, and this expectation states nothing about it",
            asked = answer.question()
        )],
    }
}

/// Whether an expectation is about the occurrence the answer is about.
///
/// The ordinal a case asks about is taken from its first stated expectation, and the one
/// answer is then measured against every entry, so an entry naming another ordinal is about
/// an occurrence nobody asked about.
fn check_ordinal(want: usize, asked: usize, noun: &str) -> Option<Vec<String>> {
    if want == asked {
        return None;
    }
    Some(vec![format!(
        "{noun}: this expectation is about {want} and the case asked about {asked}"
    )])
}

/// Compare a classification answer.
///
/// The class number, and not the provenance. `CaseClass` carries the rules "if the
/// implementation has one", and requiring an implementation to reproduce our chain of
/// specification addresses would make the suite unrunnable by every engine that answers the
/// question without publishing an address — which is what ADR 0006 exists to prevent. The
/// rules an implementation does report are unioned into `Report::rules_exercised`, where
/// they drive the coverage gate rather than the pass.
///
/// This stays true even now that two comparisons elsewhere in this module read a provenance
/// field: [`check_expansion`] below compares one, `CaseExpansion::rule`, conditionally, and
/// [`check_rules`] compares a whole array of them, `CaseBoundary::rules`, as a subset. Neither
/// is the same decision read inconsistently, and the three grounds that already justified
/// `check_expansion`'s own exception now have to answer for both rather than one.
///
/// The first ground stood as: no entry in `docs/conformance-deferrals.toml` names
/// classification provenance as its own blocker the way three `E.2` entries once named the
/// expansion citation's prior unobservability. That same census now discriminates *for* the
/// boundary comparison rather than staying neutral about it: three entries — `3.1.5`,
/// `B.2#17` and `B.2#13` — named `check_boundary`'s own silence outright. `B.2#17`'s own
/// entry states the prior state precisely, describing `check_boundary` as a function
/// "which as of round 13 already compared `breakable`, `permitted`, `spaces`,
/// `ruby_overhang` and `expansion` ... now also reads `ExpectBoundary::rules`" — five fields
/// before this round, `rules` the sixth task #44 (round 16) adds. `B.2#13`'s own entry names
/// the identical gap for its own coordinate and, in its own words, states that "that citation
/// now has a comparison path task #44 (round 16) opened" — there was none before. A fourth,
/// `D.2#4`, names a related but distinct gap one layer further upstream — `rules_fired`
/// itself never puts this note's own citation into any of its own six slots at its
/// coordinate, because that citation lives only in a reduction table's per-term loop, which
/// never runs where Table 1 states no term — a gap this round does not close, and `D.2#4`'s
/// own entry says so precisely rather than repeating the other three's claim. Zero entries
/// anywhere in that file name classification provenance as their own blocker, for this
/// function or for anything else.
///
/// The second ground stood as: a classification answer's provenance is a whole chain of rules
/// rather than one table cell's own citation — `ExpectClass::rules` names a *sequence*,
/// `ExpectExpansion::rule` names one address. `ExpectBoundary::rules` is a sequence too, so
/// that fact alone no longer separates the two comparisons this module makes; what separates
/// them is what a sequence is asked to prove. `check_rules`'s own subset semantics ask only
/// that every address a case names appear somewhere among the ones the answer published,
/// never their equality and never their order — materially weaker than reproducing "our
/// chain of specification addresses" in the sense this function's own opening paragraph
/// rejects, which is an ordered, exact accounting of *which* rules fired and *in what
/// sequence*, the very shape
/// `jlreq_spacing::evaluate::rules_fired`'s own fixed-slot array makes an implementation
/// detail rather than a specification fact (`check_rules`'s own doc argues this at length). A
/// classification's `rules` field asks the identical exact-sequence question this function
/// has always declined to ask; a boundary's does not, because this round chose not to let it.
///
/// The third ground stood as: turning class-provenance comparison on now would retroactively
/// hold every already-published classification case to an assertion its own author never
/// stated an intention to make. That risk is exactly why `check_rules` was turned on only as
/// narrowly as it was checked: task #44 (round 16) individually re-verified all twelve
/// pre-existing boundary-level `rules` declarations in the suite — five in `A.16.json`, seven
/// in `A.22.json` — confirming each is `declined` today rather than assuming it: every one
/// sits on a boundary where at least one neighbor is covered by a ruby construct
/// `jlreq-inline` (M4) does not yet exist to answer (`Kumihan::boundary`'s own
/// `construct_covers` guard, checked directly against each case's declared ruby range),
/// cross-checked against `crates/jlreq-conform/tests/suite.rs`'s own committed census (`A.16`'s
/// `[25 attempted, 1 not attempted]`, `A.22`'s `[1 attempted, 11 not attempted]`) rather than
/// taken on faith, before this function's sibling began comparing them, rather than trusting
/// that switching a silent field to a checked one would leave a large, unaudited corpus
/// intact. This is a reachability audit — none of the twelve is ever compared by
/// `check_rules` at all — not a re-derivation of what each address should say.
/// `ExpectClass::rules` carries 413 declarations across the suite, more than thirty times as
/// many, none individually re-verified under this round's own scope boundaries — the
/// retroactive risk the third ground names is real at that scale and was not run here, which
/// is exactly why this function's own behavior stays exactly as it is below.
///
/// `check_expansion`'s and `check_rules`'s own comparisons are each conditional for the
/// identical reason this function's silence always was — an implementation that publishes
/// nothing stays measurable — which is why both are additions to the suite's own reach rather
/// than a reversal of this function's own decision.
fn check_class(want: &ExpectClass, got: &CaseClass) -> Vec<String> {
    match want.class {
        Some(class) if class != got.class => vec![format!(
            "class: expected cl-{class:02}, answered cl-{answered:02}",
            answered = got.class
        )],
        _ => Vec::new(),
    }
}

/// Compare a boundary answer, field by stated field.
fn check_boundary(want: &ExpectBoundary, got: &CaseBoundary) -> Vec<String> {
    let mut found = Vec::new();
    if want.breakable.is_some_and(|wanted| wanted != got.breakable) {
        found.push(format!(
            "breakable: expected {wanted:?}, answered {answered}",
            wanted = want.breakable,
            answered = got.breakable
        ));
    }
    if want.permitted.is_some_and(|wanted| wanted != got.permitted) {
        found.push(format!(
            "permitted: expected {wanted:?}, answered {answered}",
            wanted = want.permitted,
            answered = got.permitted
        ));
    }
    if let Some(spaces) = want.spaces.as_deref() {
        found.extend(check_spaces(spaces, got));
    }
    if let Some(overhang) = want.ruby_overhang {
        if got.ruby_overhang != Some(units_of(overhang)) {
            found.push(format!(
                "ruby_overhang: expected {wanted} unit(s), answered {answered:?}",
                wanted = units_of(overhang),
                answered = got.ruby_overhang
            ));
        }
    }
    if let Some(expansion) = want.expansion.as_ref() {
        found.extend(check_expansion(expansion, &got.expansion));
    }
    if let Some(rules) = want.rules.as_deref() {
        found.extend(check_rules(rules, &got.rules));
    }
    found
}

/// Compare a feasible-break answer, field by stated field.
///
/// `rules` is checked by the identical [`check_rules`] a boundary's own `rules` field is,
/// for the identical reason: [`jlreq_line::Feasible::compute`]'s own citation for one
/// candidate — a single refusing `RuleId` for a refused one, or a chained `Provenance` of
/// up to three for a permitted one (`jlreq_spec::Provenance`'s own bound) — is the same
/// fixed-shape provenance ADR 0006 already keeps this suite from demanding a foreign
/// implementation reproduce exactly, not merely name a subset of. Reusing one function
/// rather than writing a second copy of its reasoning is the literal way to hold both
/// fields to the same standard: a divergence between two near-identical comparisons is
/// exactly the kind of drift a shared function cannot have.
fn check_feasible(want: &crate::case::ExpectFeasible, got: &CaseFeasible) -> Vec<String> {
    let mut found = Vec::new();
    if want.breakable.is_some_and(|wanted| wanted != got.breakable) {
        found.push(format!(
            "breakable: expected {wanted:?}, answered {answered}",
            wanted = want.breakable,
            answered = got.breakable
        ));
    }
    if let Some(rules) = want.rules.as_deref() {
        found.extend(check_rules(rules, &got.rules));
    }
    found
}

/// Compare a `jlreq_inline::lower` answer for one declared ruby construct, field by stated
/// field, `check_boundary`'s and `check_feasible`'s own convention.
///
/// `same_run` compares each declared pair against `got.runs`: two items share a run when
/// both resolve to `Some` and the same identity, which is the exact predicate a caller-facing
/// implementation reads off `jlreq_unit::Runs` too, opaque numbering and all. `separations`
/// is compared as a *total* list, `check_spaces`'s own convention below and not `check_rules`'
/// subset one: a case stating one entry asserts both that it exists and that the answer
/// carries no other, which is what makes a single declared entry able to assert absence
/// everywhere else without a second, negative field this format has no shape for.
/// `alignment` and `alignment_discouraged` are compared by equality when stated; `rules`
/// reuses [`check_rules`], the identical subset semantics `check_boundary`'s and
/// `check_feasible`'s own `rules` fields already give.
fn check_lower(want: &ExpectLower, got: &CaseLower) -> Vec<String> {
    let mut found = Vec::new();
    if let Some(pairs) = want.same_run.as_deref() {
        found.extend(check_same_run(pairs, got));
    }
    if let Some(separations) = want.separations.as_deref() {
        found.extend(check_lower_separations(separations, got));
    }
    if let Some(wanted) = want.alignment.as_deref() {
        let answered = got.alignment.as_deref();
        if Some(wanted) != answered {
            found.push(format!(
                "alignment: expected {wanted:?}, answered {answered:?}"
            ));
        }
    }
    if want
        .alignment_discouraged
        .is_some_and(|wanted| wanted != got.alignment_discouraged)
    {
        found.push(format!(
            "alignment_discouraged: expected {wanted:?}, answered {answered}",
            wanted = want.alignment_discouraged,
            answered = got.alignment_discouraged
        ));
    }
    if let Some(rules) = want.rules.as_deref() {
        found.extend(check_rules(rules, &got.rules));
    }
    found
}

/// Compare the declared same-run pairs against the answer's own per-item run identity.
fn check_same_run(want: &[crate::case::ExpectSameRun], got: &CaseLower) -> Vec<String> {
    let mut found = Vec::new();
    for pair in want {
        let (first, second) = pair.items;
        let left = got.runs.get(first).copied().flatten();
        let right = got.runs.get(second).copied().flatten();
        let same = matches!((left, right), (Some(a), Some(b)) if a == b);
        if same != pair.same {
            found.push(format!(
                "same_run: expected items {first} and {second} to {phrase}, answered {left:?} \
                 against {right:?}",
                phrase = if pair.same {
                    "share a run"
                } else {
                    "not share a run"
                }
            ));
        }
    }
    found
}

/// Compare the declared forced-separation list against the answer's own, as a total list:
/// `check_spaces`'s own convention, not `check_rules`'s subset one — a case stating one entry
/// asserts both that it exists and that the answer carries no other.
fn check_lower_separations(
    want: &[crate::case::ExpectLowerSeparation],
    got: &CaseLower,
) -> Vec<String> {
    if want.len() != got.separations.len() {
        return vec![format!(
            "separations: expected {wanted}, answered {answered}",
            wanted = want.len(),
            answered = got.separations.len()
        )];
    }
    let mut found = Vec::new();
    for (ordinal, (wanted, &(after, least))) in want.iter().zip(&got.separations).enumerate() {
        let mut differs = Vec::new();
        if wanted.after != after {
            differs.push(format!(
                "after {wanted} against {after}",
                wanted = wanted.after
            ));
        }
        if wanted.least.is_some_and(|wanted| wanted != least) {
            differs.push(format!(
                "least {wanted:?} against {least}",
                wanted = wanted.least
            ));
        }
        if !differs.is_empty() {
            found.push(format!(
                "separation {ordinal}: {joined}",
                joined = differs.join(", ")
            ));
        }
    }
    found
}

/// Compare a `jlreq_inline::place` answer for the case's own whole declared `Constructs`,
/// field by stated field, `check_lower`'s own convention: every field optional and a
/// missing one asserts nothing. No `check_ordinal` call precedes this — `place` has no
/// asked-ordinal for one to check (`Compose::place`'s own doc), so `check` above dispatches
/// straight here rather than through a `check_ordinal(...).unwrap_or_else(...)` guard.
fn check_place(want: &ExpectPlace, got: &CasePlace) -> Vec<String> {
    let mut found = Vec::new();
    if let Some(attachments) = want.attachments.as_deref() {
        found.extend(check_attachments(attachments, &got.attachments));
    }
    if let Some(declined) = want.declined.as_deref() {
        found.extend(check_declined(declined, &got.declined));
    }
    found
}

/// Compare the declared attachments against the answer's own, as a *total* list —
/// `check_lower_separations`'s own convention above, not `check_rules`'s subset one: a case
/// stating one entry asserts both that it exists and that the answer carries no other.
fn check_attachments(
    want: &[crate::case::ExpectAttachment],
    got: &[CaseAttachment],
) -> Vec<String> {
    if want.len() != got.len() {
        return vec![format!(
            "attachments: expected {wanted}, answered {answered}",
            wanted = want.len(),
            answered = got.len()
        )];
    }
    let mut found = Vec::new();
    for (ordinal, (wanted, answered)) in want.iter().zip(got).enumerate() {
        let mut differs = Vec::new();
        if wanted
            .inline
            .is_some_and(|wanted| wanted != answered.inline)
        {
            differs.push(format!(
                "inline {wanted:?} against {answered}",
                wanted = wanted.inline,
                answered = answered.inline
            ));
        }
        if wanted
            .item
            .is_some_and(|wanted| Some(wanted) != answered.item)
        {
            differs.push(format!(
                "item {wanted:?} against {answered:?}",
                wanted = wanted.item,
                answered = answered.item
            ));
        }
        if !differs.is_empty() {
            found.push(format!(
                "attachment {ordinal}: {joined}",
                joined = differs.join(", ")
            ));
        }
    }
    found
}

/// Compare the declared declined-construct ordinals against the answer's own, as a total
/// list and in order: asserting the specific ordinal, not merely that the count matches, is
/// the entire point of this field (`ExpectPlace::declined`'s own doc).
fn check_declined(want: &[usize], got: &[usize]) -> Vec<String> {
    if want == got {
        Vec::new()
    } else {
        vec![format!("declined: expected {want:?}, answered {got:?}")]
    }
}

/// Compare a declared rules subset against the ones an answer publishes: presence, not
/// reproduction. Shared by `boundary.rules` (`check_boundary`, above), `feasible.rules`
/// (`check_feasible`, above) and `lower.rules` (`check_lower`, above) — the identical
/// semantics for the identical reason, stated once rather than once per caller.
///
/// `want` states a *subset* of `got.rules`, never their equality and never their order. Every
/// address the expectation names must appear somewhere among the answered rules; an answer
/// that names more addresses than the expectation states is not a difference, because a case
/// asserts that a specific citation fired, not that it fired *alone* or in a particular
/// position. This is the deliberate half of the choice, and the reason is
/// `jlreq_spacing::evaluate::rules_fired`'s own shape: it fills a fixed 6-slot array —
/// breakable, placement, one slot per conditional space, the delegation, then the Table 6
/// expansion citation — whose order is internal slot layout rather than anything stated by
/// the specification, and whose first two slots repeat the identical fallback address,
/// `RuleId::SPACING_BETWEEN_CHARACTERS`, whenever neither breakable nor placement cites
/// anything more specific. Neither the order nor the duplication is a fact a conforming
/// engine that merely answers the boundary question could be expected to reproduce, and
/// holding a case to reproducing them would be exactly the "reproduce our chain of
/// specification addresses" demand `check_class`'s own doc already names as the reason
/// classification provenance is not compared there — the identical ADR-0006 concern, answered
/// here by asking for presence alone rather than by declining to ask at all.
///
/// A declared expectation meeting an empty `got.rules` is passed over rather than failed, the
/// identical third state [`check_expansion`] above already gives its own one provenance
/// field: an implementation that answers a boundary without publishing any specification
/// address at all must stay measurable by every other field `check_boundary` checks. That
/// branch is a foreign-implementation affordance rather than a live path for kumihan's own
/// answers — `rules_fired`'s own doc states the array always yields at least two entries, the
/// breakable and placement fallbacks, so a `got.rules` built from `Kumihan::boundary` is never
/// empty and this function never takes that branch against this workspace's own
/// implementation.
///
/// A declared address absent from a *non-empty* `got.rules` is the one shape that is a real
/// divergence — the case asserts a citation fired and the answer, having published something,
/// did not publish that one — and is reported once per missing address, naming it and the
/// full list of what the answer did publish.
///
/// The pass-over above is argued for a `permitted` entry, where meeting it is what keeps a
/// decliner measurable. On a `forbidden` entry the identical empty-`got.rules` case inverts
/// what "no differences" means: this function returning nothing is what `is_silent`-gated
/// exclusion (`ask`'s own doc, `Expect::is_silent`) reads as *satisfied*, so a `forbidden`
/// entry naming only a `rules` address would exclude an implementation that publishes no
/// citations at all, the one shape that should be unmeasurable rather than excluded.
/// `check_expansion`'s own conditional `rule` field already has this identical shape on its
/// `forbidden` side, so this is not a new inconsistency this function introduces; no case in
/// the corpus states a `forbidden` entry naming `rules` (or `expansion.rule`) alone today, so
/// neither shape has yet had a real case to be wrong about.
fn check_rules(want: &[String], got: &[String]) -> Vec<String> {
    if got.is_empty() {
        return Vec::new();
    }
    want.iter()
        .filter(|address| !got.iter().any(|answered| answered == *address))
        .map(|address| {
            format!("rules: expected {address:?} among the answered rules, which are {got:?}")
        })
        .collect()
}

/// Compare the boundary's own expansion opportunity (ADR 0014, amended by ADR 0021).
///
/// `rule` is compared conditionally and by equality of the one declared address, on
/// semantics distinct from every other field this function checks: the expectation is
/// checked only when it states a `rule` at all (unchanged for every other field, which is
/// the schema's own general "a missing field asserts nothing" rule), but a *declared*
/// expectation that meets a `got.rule` of `None` is passed over rather than failed — the
/// one case where a stated expectation is not held to. That third state exists because
/// `CaseExpansion::rule` is exactly the kind of provenance `check_class`'s own doc argues an
/// implementation may honestly have none of: a conforming engine that answers `boundary`
/// without publishing a Table 6 address must stay measurable by every other field this
/// function checks, which "declared but unanswered fails" would silently stop being true
/// for. Equality only when both sides publish one, because that is the one shape a
/// disagreement is actually informative: two different addresses for the identical
/// coordinate is a real divergence, not a decliner meeting a case that happens to expect
/// more than it can say.
fn check_expansion(want: &ExpectExpansion, got: &CaseExpansion) -> Vec<String> {
    let mut differs = Vec::new();
    if want
        .kind
        .as_deref()
        .is_some_and(|wanted| wanted != got.kind)
    {
        differs.push(format!(
            "kind {wanted:?} against {answered:?}",
            wanted = want.kind,
            answered = got.kind
        ));
    }
    if let Some(ceiling) = want.ceiling {
        if got.ceiling_units != Some(units_of(ceiling)) {
            differs.push(format!(
                "{wanted} unit(s) against {answered:?}",
                wanted = units_of(ceiling),
                answered = got.ceiling_units
            ));
        }
    }
    if want.stage.is_some_and(|stage| Some(stage) != got.stage) {
        differs.push(format!(
            "stage {wanted:?} against {answered:?}",
            wanted = want.stage,
            answered = got.stage
        ));
    }
    if let Some(wanted) = want.rule.as_deref() {
        if let Some(answered) = got.rule.as_deref() {
            if wanted != answered {
                differs.push(format!("rule {wanted:?} against {answered:?}"));
            }
        }
        // `got.rule` is `None`: the expectation names a citation and this implementation
        // publishes none, which is passed over rather than failed, per this function's own
        // doc above.
    }
    if differs.is_empty() {
        Vec::new()
    } else {
        vec![format!("expansion: {joined}", joined = differs.join(", "))]
    }
}

/// Compare the conditional spaces at a boundary, which are the unit of spacing (ADR 0014).
fn check_spaces(want: &[crate::case::ExpectSpace], got: &CaseBoundary) -> Vec<String> {
    if want.len() != got.spaces.len() {
        return vec![format!(
            "spaces: expected {wanted}, answered {answered}",
            wanted = want.len(),
            answered = got.spaces.len()
        )];
    }
    let mut found = Vec::new();
    for (ordinal, (wanted, answered)) in want.iter().zip(&got.spaces).enumerate() {
        let mut differs = Vec::new();
        if units_of(wanted.amount) != answered.units {
            differs.push(format!(
                "{wanted} unit(s) against {answered}",
                wanted = units_of(wanted.amount),
                answered = answered.units
            ));
        }
        for (name, wanted, answered) in [
            (
                "referent",
                wanted.referent.as_deref(),
                answered.referent.as_str(),
            ),
            (
                "reduction",
                wanted.reduction.as_deref(),
                answered.reduction.as_str(),
            ),
            ("ladder", wanted.ladder.as_deref(), answered.ladder.as_str()),
        ] {
            if wanted.is_some_and(|wanted| wanted != answered) {
                differs.push(format!("{name} {wanted:?} against {answered:?}"));
            }
        }
        if wanted.stage.is_some_and(|stage| stage != answered.stage) {
            differs.push(format!("stage {wanted:?}", wanted = wanted.stage));
        }
        if !differs.is_empty() {
            found.push(format!(
                "space {ordinal}: {joined}",
                joined = differs.join(", ")
            ));
        }
    }
    found
}

/// Compare a composition answer, field by stated field.
fn check_composed(
    lines: Option<&[ExpectLine]>,
    violations: Option<&[String]>,
    got: &CaseOutput,
) -> Vec<String> {
    let mut found = Vec::new();
    if let Some(violations) = violations {
        if violations != got.violations.as_slice() {
            found.push(format!(
                "violations: expected {violations:?}, answered {answered:?}",
                answered = got.violations
            ));
        }
    }
    let Some(lines) = lines else {
        return found;
    };
    if lines.len() != got.lines.len() {
        found.push(format!(
            "lines: expected {wanted}, answered {answered}",
            wanted = lines.len(),
            answered = got.lines.len()
        ));
        return found;
    }
    for (ordinal, (wanted, answered)) in lines.iter().zip(&got.lines).enumerate() {
        found.extend(
            check_line(wanted, answered)
                .into_iter()
                .map(|finding| format!("line {ordinal}: {finding}")),
        );
    }
    found
}

/// Compare one composed line.
fn check_line(want: &ExpectLine, got: &CaseLine) -> Vec<String> {
    let mut found = Vec::new();
    if want
        .placements
        .as_deref()
        .is_some_and(|wanted| wanted != got.placements.as_slice())
    {
        found.push(format!(
            "placements: expected {wanted:?}, answered {answered:?}",
            wanted = want.placements,
            answered = got.placements
        ));
    }
    if want.extent.is_some_and(|wanted| wanted != got.extent) {
        found.push(format!(
            "extent: expected {wanted:?}, answered {answered}",
            wanted = want.extent,
            answered = got.extent
        ));
    }
    if let Some(trailing) = want.trailing {
        if units_of(trailing) != got.trailing {
            found.push(format!(
                "trailing: expected {wanted} unit(s), answered {answered}",
                wanted = units_of(trailing),
                answered = got.trailing
            ));
        }
    }
    if want
        .hanging
        .is_some_and(|wanted| Some(wanted) != got.hanging)
    {
        found.push(format!(
            "hanging: expected {wanted:?}, answered {answered:?}",
            wanted = want.hanging,
            answered = got.hanging
        ));
    }
    if let Some(trims) = want.trims.as_deref() {
        found.extend(check_trims(trims, &got.trims));
    }
    if let Some(parts) = want.parts.as_deref() {
        found.extend(check_parts(parts, &got.parts));
    }
    found.extend(check_pull_up(want.pull_up.as_ref(), got.pull_up.as_ref()));
    found
}

/// Compare a line's own pull-up (§3.1.12 ⑤, `Search::Optimal`), unconditionally.
///
/// Every other field `check_line` checks is silent unless the case states it
/// (`cases.schema.json`'s own general "a missing field asserts nothing" rule); `pull_up` is
/// the deliberate exception, on task #44 (round 16)'s own precedent for `ExpectBoundary::
/// rules` — a case that never mentions `pull_up` would otherwise stop constraining it the
/// moment the field existed, which is exactly the decorative-field outcome that round was
/// created to head off. It is the safe reading here for a fact the retroactive risk that
/// precedent had to weigh individually does not apply to at all: `Search::FirstFit`'s own
/// doc states `Line::pull_up` answers `None`, full stop, on every line that search
/// composes, so every one of the 466 cases published before this field existed is provably
/// `None` on the answer side regardless of what its own case ever declared, and turning the
/// comparison on retroactively changes what none of them are measured against.
///
/// `want`'s own `rule` is still compared conditionally within `Some` — `check_expansion`'s
/// own three-state provenance reading directly above, not the coarser two-state rule the
/// outer `Option` gets — because an implementation that reports a pull-up without
/// publishing a specification address for it must stay measurable by `amount` and `pulls`
/// alone, the same as `boundary.expansion.rule`'s own decliner does.
fn check_pull_up(want: Option<&ExpectPullUp>, got: Option<&CasePullUp>) -> Vec<String> {
    match (want, got) {
        (None, None) => Vec::new(),
        (None, Some(answered)) => vec![format!(
            "pull_up: expected none, answered amount {amount} pulling item {pulls}",
            amount = answered.amount,
            pulls = answered.pulls
        )],
        (Some(wanted), None) => vec![format!(
            "pull_up: expected amount {amount} pulling item {pulls}, answered none",
            amount = wanted.amount,
            pulls = wanted.pulls
        )],
        (Some(wanted), Some(answered)) => {
            let mut differs = Vec::new();
            if wanted.amount != answered.amount {
                differs.push(format!(
                    "amount {wanted} against {answered}",
                    wanted = wanted.amount,
                    answered = answered.amount
                ));
            }
            if wanted.pulls != answered.pulls {
                differs.push(format!(
                    "pulls {wanted} against {answered}",
                    wanted = wanted.pulls,
                    answered = answered.pulls
                ));
            }
            if let Some(wanted_rule) = wanted.rule.as_deref() {
                if let Some(answered_rule) = answered.rule.as_deref() {
                    if wanted_rule != answered_rule {
                        differs.push(format!("rule {wanted_rule:?} against {answered_rule:?}"));
                    }
                }
            }
            if differs.is_empty() {
                Vec::new()
            } else {
                vec![format!("pull_up: {joined}", joined = differs.join(", "))]
            }
        },
    }
}

/// Compare the units taken out of the caller's own advances, field by field.
///
/// Counting them is not enough, and the worked case of `docs/design/conformance.md` is the
/// reason: its whole assertion is that an implementation which shortens the caller's advance
/// without saying so passes both `extent` checks and fails on `trims`. A comparison by length
/// alone passes a trim taken out of the wrong item, attributed to the wrong neighbor, or
/// justified by the wrong sentence — which is exactly the evasion of ADR 0002 that naming a
/// rule on a trim exists to prevent.
fn check_trims(want: &[crate::case::ExpectTrim], got: &[CaseTrim]) -> Vec<String> {
    if want.len() != got.len() {
        return vec![format!(
            "trims: expected {wanted}, answered {answered}",
            wanted = want.len(),
            answered = got.len()
        )];
    }
    let mut found = Vec::new();
    for (ordinal, (wanted, answered)) in want.iter().zip(got).enumerate() {
        let mut differs = Vec::new();
        if wanted.item != answered.item {
            differs.push(format!(
                "item {wanted} against {answered}",
                wanted = wanted.item,
                answered = answered.item
            ));
        }
        if units_of(wanted.amount) != answered.units {
            differs.push(format!(
                "{wanted} unit(s) against {answered}",
                wanted = units_of(wanted.amount),
                answered = answered.units
            ));
        }
        if wanted
            .referent
            .as_deref()
            .is_some_and(|referent| referent != answered.referent)
        {
            differs.push(format!(
                "referent {wanted:?} against {answered:?}",
                wanted = wanted.referent,
                answered = answered.referent
            ));
        }
        if wanted
            .rule
            .as_deref()
            .is_some_and(|rule| rule != answered.rule)
        {
            differs.push(format!(
                "rule {wanted:?} against {answered:?}",
                wanted = wanted.rule,
                answered = answered.rule
            ));
        }
        if !differs.is_empty() {
            found.push(format!(
                "trim {ordinal}: {joined}",
                joined = differs.join(", ")
            ));
        }
    }
    found
}

/// Compare the sub-lines of the segments touching one line, field by field.
///
/// The tate-chu-yoko (縦中横) case of `docs/design/conformance.md` turns on `across` alone —
/// an implementation that laid the run out along the line's inline axis produces the same
/// number of parts — so a comparison by length would pass the one input the case exists for.
fn check_parts(want: &[crate::case::ExpectPart], got: &[CasePart]) -> Vec<String> {
    if want.len() != got.len() {
        return vec![format!(
            "parts: expected {wanted}, answered {answered}",
            wanted = want.len(),
            answered = got.len()
        )];
    }
    let mut found = Vec::new();
    for (ordinal, (wanted, answered)) in want.iter().zip(got).enumerate() {
        let mut differs = Vec::new();
        for (name, wanted, answered) in [
            ("segment", wanted.segment, answered.segment),
            ("index", wanted.index, answered.index),
            ("first item", wanted.items.0, answered.items.0),
            ("last item", wanted.items.1, answered.items.1),
        ] {
            if wanted != answered {
                differs.push(format!("{name} {wanted} against {answered}"));
            }
        }
        for (name, wanted, answered) in [
            ("inline", wanted.inline, Some(answered.inline)),
            ("block", wanted.block, Some(answered.block)),
            ("extent", wanted.extent, Some(answered.extent)),
        ] {
            if wanted.is_some_and(|wanted| Some(wanted) != answered) {
                differs.push(format!("{name} {wanted:?} against {answered:?}"));
            }
        }
        if wanted
            .across
            .as_deref()
            .is_some_and(|across| across != answered.across.as_slice())
        {
            differs.push(format!(
                "across {wanted:?} against {answered:?}",
                wanted = wanted.across,
                answered = answered.across
            ));
        }
        if !differs.is_empty() {
            found.push(format!(
                "part {ordinal}: {joined}",
                joined = differs.join(", ")
            ));
        }
    }
    found
}

/// An amount in kumihan's own unit.
///
/// The case states the fraction JLReq writes and, where it also states the unit count,
/// `conform --check` has already asserted the two agree under the current denominator. The
/// fraction is therefore the one that is read here, which is what makes the published suite
/// independent of ADR 0007's 1/720 (`docs/design/conformance.md`).
fn units_of(amount: CaseAmount) -> i64 {
    let (numerator, denominator) = amount.em;
    if denominator == 0 {
        return 0;
    }
    numerator
        .checked_mul(i64::from(jlreq::UNITS_PER_EM))
        .and_then(|scaled| scaled.checked_div(denominator))
        .unwrap_or(0)
}

/// How a report writes one expectation.
fn render_expect(expect: &Expect) -> String {
    if let Some(class) = &expect.class {
        return match class.class {
            Some(number) => format!("class cl-{number:02} for item {item}", item = class.item),
            None => format!("something about item {item}", item = class.item),
        };
    }
    if let Some(boundary) = &expect.boundary {
        return format!(
            "breakable {breakable:?} before item {before}",
            breakable = boundary.breakable,
            before = boundary.before
        );
    }
    if let Some(feasible) = &expect.feasible {
        return format!(
            "breakable {breakable:?} for candidate {candidate}",
            breakable = feasible.breakable,
            candidate = feasible.candidate
        );
    }
    if let Some(lower) = &expect.lower {
        return format!(
            "alignment {alignment:?} for construct {construct}",
            alignment = lower.alignment,
            construct = lower.construct
        );
    }
    match &expect.lines {
        Some(lines) => format!("{count} line(s)", count = lines.len()),
        None => "nothing".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::collections::BTreeSet;

    use super::{
        Answer, CaseAttachment, CaseBoundary, CaseClass, CaseExpansion, CaseFeasible, CaseLine,
        CaseLower, CaseOutput, CasePart, CasePlace, CaseTrim, Compose, Edge, Report, check,
        run_file,
    };
    use crate::case::{Case, CaseFile, CaseInput, CasePolicy, Expect};
    use crate::kumihan::Kumihan;

    /// An implementation that answers whatever a test hands it.
    ///
    /// The suite's own tests only ever construct `Kumihan`, which answers classification and
    /// nothing else and always declares a policy, so three of this module's branches — the
    /// composition comparison, the `forbidden` path and `applicable`'s declares-nothing arm
    /// — were reachable by no test in the workspace. This is what reaches them.
    #[derive(Debug, Default)]
    struct Fixture {
        /// What `classify` answers.
        class: Option<CaseClass>,
        /// What `boundary` answers.
        boundary: Option<CaseBoundary>,
        /// What `compose` and `align` answer.
        composed: Option<CaseOutput>,
        /// What `tab` answers — a separate field from `composed`, deliberately, so a test
        /// can hand the two different answers and prove a `tab` case reached `Compose::tab`
        /// rather than falling through `ask`'s own wildcard arm into `Compose::compose`
        /// (`a_tab_case_reaches_compose_tab_and_not_compose_compose`, below).
        tab: Option<CaseOutput>,
        /// What `feasible` answers — a separate field from `composed`, deliberately, for the
        /// identical reason `tab`'s own field is: a test can hand the two different answers
        /// and prove a `feasible` case reached `Compose::feasible` rather than falling
        /// through `ask`'s own wildcard arm into `Compose::compose`
        /// (`a_feasible_case_reaches_compose_feasible_and_not_compose_compose`, below).
        feasible: Option<CaseFeasible>,
        /// What `lower` answers — a separate field from `composed`, deliberately, for the
        /// identical reason `tab`'s and `feasible`'s own fields are: a test can hand the two
        /// different answers and prove a `lower` case reached `Compose::lower` rather than
        /// falling through `ask`'s own wildcard arm into `Compose::compose`
        /// (`a_lower_case_reaches_compose_lower_and_not_compose_compose`, below).
        lower: Option<CaseLower>,
        /// What `place` answers — a separate field from `composed`, deliberately, for the
        /// identical reason `tab`'s, `feasible`'s and `lower`'s own fields are: a test can
        /// hand the two different answers and prove a `place` case reached `Compose::place`
        /// rather than falling through `ask`'s own wildcard arm into `Compose::compose`
        /// (`a_place_case_reaches_compose_place_and_not_compose_compose`, below).
        place: Option<CasePlace>,
        /// What `declared_policy` answers.
        policy: Option<CasePolicy>,
        /// The `edge` the last call to `boundary` was actually handed — a `Cell` because
        /// `Compose::boundary` takes `&self`, and this is read back by
        /// `a_case_files_edge_reaches_the_implementation_through_the_full_reader` to prove
        /// the whole reading chain, not only `ask`'s own extraction of it.
        received_edge: Cell<Option<Edge>>,
    }

    impl Compose for Fixture {
        fn name(&self) -> &'static str {
            "fixture"
        }

        fn declared_policy(&self) -> Option<CasePolicy> {
            self.policy.clone()
        }

        fn classify(&self, _input: &CaseInput, _item: usize) -> Option<CaseClass> {
            self.class.clone()
        }

        fn boundary(
            &self,
            _input: &CaseInput,
            _before: usize,
            edge: Option<Edge>,
        ) -> Option<CaseBoundary> {
            self.received_edge.set(edge);
            self.boundary.clone()
        }

        fn compose(&self, _input: &CaseInput) -> Option<CaseOutput> {
            self.composed.clone()
        }

        fn align(&self, _input: &CaseInput) -> Option<CaseOutput> {
            self.composed.clone()
        }

        fn tab(&self, _input: &CaseInput) -> Option<CaseOutput> {
            self.tab.clone()
        }

        fn feasible(&self, _input: &CaseInput, _candidate: usize) -> Option<CaseFeasible> {
            self.feasible.clone()
        }

        fn lower(&self, _input: &CaseInput, _construct: usize) -> Option<CaseLower> {
            self.lower.clone()
        }

        fn place(&self, _input: &CaseInput) -> Option<CasePlace> {
            self.place.clone()
        }
    }

    /// A one-item classification input, as the published format writes one.
    const INPUT: &str = r#""input": {
        "kind": "classify",
        "text": "あ",
        "scales": [{ "inline_em": 1000, "block_em": 1000 }],
        "items": [{ "start": 0, "advance": 1000, "frame": "full-em", "scale": 0 }]
    }"#;

    /// The same, asking the boundary question.
    const BOUNDARY_INPUT: &str = r#""input": {
        "kind": "boundary",
        "text": "あ",
        "scales": [{ "inline_em": 1000, "block_em": 1000 }],
        "items": [{ "start": 0, "advance": 1000, "frame": "full-em", "scale": 0 }]
    }"#;

    /// The same, asking the composition question.
    const COMPOSED_INPUT: &str = r#""input": {
        "kind": "compose",
        "text": "あ",
        "scales": [{ "inline_em": 1000, "block_em": 1000 }],
        "items": [{ "start": 0, "advance": 1000, "frame": "full-em", "scale": 0 }],
        "candidates": [{ "at": 0 }, { "at": 3 }],
        "measure": 1000
    }"#;

    /// The same, asking the tab question.
    const TAB_INPUT: &str = r#""input": {
        "kind": "tab",
        "text": "あ",
        "scales": [{ "inline_em": 1000, "block_em": 1000 }],
        "items": [{ "start": 0, "advance": 1000, "frame": "full-em", "scale": 0 }],
        "tab_starts": [0],
        "tab_stops": [{ "position": 0, "kind": "start" }]
    }"#;

    /// The same, asking the feasible-break question.
    const FEASIBLE_INPUT: &str = r#""input": {
        "kind": "feasible",
        "text": "あ",
        "scales": [{ "inline_em": 1000, "block_em": 1000 }],
        "items": [{ "start": 0, "advance": 1000, "frame": "full-em", "scale": 0 }],
        "candidates": [{ "at": 0 }]
    }"#;

    /// The same, asking what `lower` resolved for one declared construct.
    const LOWER_INPUT: &str = r#""input": {
        "kind": "lower",
        "text": "あ",
        "scales": [{ "inline_em": 1000, "block_em": 1000 }],
        "items": [{ "start": 0, "advance": 1000, "frame": "full-em", "scale": 0 }]
    }"#;

    /// The same, asking what `place` resolved for the whole declared `Constructs`.
    const PLACE_INPUT: &str = r#""input": {
        "kind": "place",
        "text": "あ",
        "scales": [{ "inline_em": 1000, "block_em": 1000 }],
        "items": [{ "start": 0, "advance": 1000, "frame": "full-em", "scale": 0 }]
    }"#;

    /// One case with the supplied input and expectations, read through the format's own
    /// reader so a test of the runner is a test of the format.
    fn wrap(input: &str, permitted: &str, forbidden: &str) -> Case {
        let source = format!(
            "{{ \"id\": \"3.1.9/fixture/one\", \"rules\": [\"3.9.2\"], \
             \"standing\": \"normative\", \"quote\": \"q\", \"rationale\": \"r\", {input}, \
             \"permitted\": {permitted}, \"forbidden\": {forbidden}, \"disagreements\": [] }}"
        );
        Case::of(&source).expect("the fixture is a well-formed case")
    }

    /// Run one case against one implementation.
    fn measure(case: Case, implementation: &Fixture) -> Report {
        run_file(&CaseFile::of("3.1.9", vec![case]), implementation)
    }

    /// A one-line composition answer carrying the given trims and parts.
    fn composed(trims: Vec<CaseTrim>, parts: Vec<CasePart>) -> CaseOutput {
        CaseOutput::new(
            vec![CaseLine::new(vec![0], 0, 1000, trims, parts, None, None)],
            Vec::new(),
            Vec::new(),
        )
    }

    /// A two-line answer, distinguishable from `composed`'s own one-line shape by `lines`
    /// alone — what `a_tab_case_reaches_compose_tab_and_not_compose_compose` needs `tab`'s
    /// own answer to be, so that a case wrongly routed to `Compose::compose` disagrees
    /// rather than coincidentally passing.
    fn two_lines() -> CaseOutput {
        CaseOutput::new(
            vec![
                CaseLine::new(vec![0], 0, 500, Vec::new(), Vec::new(), None, None),
                CaseLine::new(vec![500], 0, 1000, Vec::new(), Vec::new(), None, None),
            ],
            Vec::new(),
            Vec::new(),
        )
    }

    #[test]
    fn a_tab_case_reaches_compose_tab_and_not_compose_compose() {
        // The trap this round's own brief names: `ask`'s own wildcard arm is what
        // `"compose"` names, and a `"tab"` case that fell into it anyway — a forgotten
        // `"tab" =>` arm — would still compile and would still return `Some`, answered by
        // `Compose::compose` instead of `Compose::tab` without either method or the runner
        // ever raising an error. `tab` and `composed` are given different-shaped answers
        // here (two lines against one) specifically so that fallthrough is a scored
        // `disagreed`, not a coincidental agreement that would hide the very regression
        // this test exists to catch.
        let fixture = Fixture {
            composed: Some(composed(Vec::new(), Vec::new())),
            tab: Some(two_lines()),
            ..Fixture::default()
        };
        let expected = r#"[{ "policy": {}, "source": "s", "expect": { "lines": [
            { "placements": [0], "extent": 500 },
            { "placements": [500], "extent": 1000 }
        ] } }]"#;
        let report = measure(wrap(TAB_INPUT, expected, "[]"), &fixture);
        assert_eq!(
            (report.attempted, report.agreed, report.disagreed.len()),
            (1, 1, 0),
            "the case's own two-line expectation matches `Compose::tab`'s own answer; a run \
             wrongly routed to `Compose::compose` would answer one line instead and \
             disagree on `lines`: {report:?}"
        );
    }

    #[test]
    fn a_tab_case_with_no_tab_answer_is_not_attempted_even_though_compose_has_one() {
        // The other half of the same trap: an implementation that answers `compose` but
        // declines `tab` must be scored as not having attempted this case, never as having
        // silently answered it through the wrong method because one happened to be handy.
        let fixture = Fixture {
            composed: Some(composed(Vec::new(), Vec::new())),
            tab: None,
            ..Fixture::default()
        };
        let report = measure(
            wrap(
                TAB_INPUT,
                r#"[{ "policy": {}, "source": "s", "expect": { "lines": [] } }]"#,
                "[]",
            ),
            &fixture,
        );
        assert_eq!(
            (report.attempted, report.skipped),
            (0, 1),
            "a `tab` case asks `Compose::tab`, and an implementation that answers only \
             `compose` has not attempted it: {report:?}"
        );
    }

    #[test]
    fn a_feasible_case_reaches_compose_feasible_and_not_compose_compose() {
        // The identical trap `tab`'s own pair of tests above name, now for the sixth
        // question: a forgotten `"feasible" =>` arm in `ask` would still compile and would
        // still fall through to the wildcard, answered by `Compose::compose` instead of
        // `Compose::feasible`. `feasible` and `composed` are given expectations that cannot
        // agree by accident — `breakable: false` here, and `composed`'s own one-line answer
        // never satisfies a `feasible` expectation at all, since `check` refuses to read a
        // `Composed` answer as one (`check`'s own wildcard tuple arm) — so a fallthrough is
        // a scored `disagreed` rather than a coincidental pass.
        let fixture = Fixture {
            composed: Some(composed(Vec::new(), Vec::new())),
            feasible: Some(CaseFeasible::new(false, vec!["C.2#13".to_owned()])),
            ..Fixture::default()
        };
        let expected = r#"[{ "policy": {}, "source": "s",
            "expect": { "feasible": { "candidate": 0, "breakable": false,
                                       "rules": ["C.2#13"] } } }]"#;
        let report = measure(wrap(FEASIBLE_INPUT, expected, "[]"), &fixture);
        assert_eq!(
            (report.attempted, report.agreed, report.disagreed.len()),
            (1, 1, 0),
            "the case's own expectation matches `Compose::feasible`'s own answer; a run \
             wrongly routed to `Compose::compose` would answer no `feasible` field at all \
             and disagree instead: {report:?}"
        );
    }

    #[test]
    fn a_feasible_case_with_no_feasible_answer_is_not_attempted_even_though_compose_has_one() {
        // The other half of the same trap: an implementation that answers `compose` but
        // declines `feasible` must be scored as not having attempted this case.
        let fixture = Fixture {
            composed: Some(composed(Vec::new(), Vec::new())),
            feasible: None,
            ..Fixture::default()
        };
        let report = measure(
            wrap(
                FEASIBLE_INPUT,
                r#"[{ "policy": {}, "source": "s",
                      "expect": { "feasible": { "candidate": 0 } } }]"#,
                "[]",
            ),
            &fixture,
        );
        assert_eq!(
            (report.attempted, report.skipped),
            (0, 1),
            "a `feasible` case asks `Compose::feasible`, and an implementation that answers \
             only `compose` has not attempted it: {report:?}"
        );
    }

    #[test]
    fn a_feasible_rules_expectation_is_a_subset_never_an_equality() {
        // `check_feasible`'s own reuse of `check_rules`, pinned directly: the declared
        // address is a strict subset of what the answer publishes, in a different order,
        // and neither the extra address nor the order is a difference.
        let fixture = Fixture {
            feasible: Some(CaseFeasible::new(
                false,
                vec!["3.1.7".to_owned(), "C.2#13".to_owned()],
            )),
            ..Fixture::default()
        };
        let expected = r#"[{ "policy": {}, "source": "s",
            "expect": { "feasible": { "candidate": 0, "rules": ["C.2#13"] } } }]"#;
        let report = measure(wrap(FEASIBLE_INPUT, expected, "[]"), &fixture);
        assert_eq!(
            (report.agreed, report.disagreed.len()),
            (1, 0),
            "every declared address appears somewhere among the answered ones: {report:?}"
        );
    }

    #[test]
    fn a_feasible_rules_expectation_absent_from_a_non_empty_answer_disagrees() {
        let fixture = Fixture {
            feasible: Some(CaseFeasible::new(true, vec!["3.1.7".to_owned()])),
            ..Fixture::default()
        };
        let expected = r#"[{ "policy": {}, "source": "s",
            "expect": { "feasible": { "candidate": 0, "rules": ["C.2#13"] } } }]"#;
        let report = measure(wrap(FEASIBLE_INPUT, expected, "[]"), &fixture);
        assert_eq!(
            (report.agreed, report.disagreed.len()),
            (0, 1),
            "the answer published something and it was not the declared address: {report:?}"
        );
    }

    #[test]
    fn a_lower_case_reaches_compose_lower_and_not_compose_compose() {
        // The identical trap `tab`'s and `feasible`'s own pairs of tests above name, now for
        // the seventh question: a forgotten `"lower" =>` arm in `ask` would still compile and
        // would still fall through to the wildcard, answered by `Compose::compose` instead of
        // `Compose::lower`. `lower` and `composed` are given expectations that cannot agree by
        // accident — `composed`'s own one-line answer never satisfies a `lower` expectation at
        // all, since `check` refuses to read a `Composed` answer as one (`check`'s own
        // wildcard tuple arm) — so a fallthrough is a scored `disagreed` rather than a
        // coincidental pass.
        let fixture = Fixture {
            composed: Some(composed(Vec::new(), Vec::new())),
            lower: Some(CaseLower::new(
                vec![Some(1)],
                Vec::new(),
                Some("nakatsuki".to_owned()),
                false,
                vec!["3.3.5".to_owned()],
            )),
            ..Fixture::default()
        };
        let expected = r#"[{ "policy": {}, "source": "s",
            "expect": { "lower": { "construct": 0, "alignment": "nakatsuki",
                                    "alignment_discouraged": false } } }]"#;
        let report = measure(wrap(LOWER_INPUT, expected, "[]"), &fixture);
        assert_eq!(
            (report.attempted, report.agreed, report.disagreed.len()),
            (1, 1, 0),
            "the case's own expectation matches `Compose::lower`'s own answer; a run wrongly \
             routed to `Compose::compose` would answer no `lower` field at all and disagree \
             instead: {report:?}"
        );
    }

    #[test]
    fn a_lower_case_with_no_lower_answer_is_not_attempted_even_though_compose_has_one() {
        // The other half of the same trap: an implementation that answers `compose` but
        // declines `lower` must be scored as not having attempted this case.
        let fixture = Fixture {
            composed: Some(composed(Vec::new(), Vec::new())),
            lower: None,
            ..Fixture::default()
        };
        let report = measure(
            wrap(
                LOWER_INPUT,
                r#"[{ "policy": {}, "source": "s",
                      "expect": { "lower": { "construct": 0 } } }]"#,
                "[]",
            ),
            &fixture,
        );
        assert_eq!(
            (report.attempted, report.skipped),
            (0, 1),
            "a `lower` case asks `Compose::lower`, and an implementation that answers only \
             `compose` has not attempted it: {report:?}"
        );
    }

    #[test]
    fn a_lower_rules_expectation_is_a_subset_never_an_equality() {
        // `check_lower`'s own reuse of `check_rules`, pinned directly, `check_feasible`'s own
        // precedent applied a third time: the declared address is a strict subset of what the
        // answer publishes, in a different order, and neither the extra address nor the order
        // is a difference.
        let fixture = Fixture {
            lower: Some(CaseLower::new(
                Vec::new(),
                Vec::new(),
                None,
                false,
                vec!["3.3.4".to_owned(), "3.3.5".to_owned()],
            )),
            ..Fixture::default()
        };
        let expected = r#"[{ "policy": {}, "source": "s",
            "expect": { "lower": { "construct": 0, "rules": ["3.3.5"] } } }]"#;
        let report = measure(wrap(LOWER_INPUT, expected, "[]"), &fixture);
        assert_eq!(
            (report.agreed, report.disagreed.len()),
            (1, 0),
            "every declared address appears somewhere among the answered rules: {report:?}"
        );
    }

    #[test]
    fn a_lower_separations_expectation_is_a_total_list_not_a_subset() {
        // `check_lower_separations`'s own convention, deliberately the opposite of `rules`'
        // subset semantics stated directly above: a declared list that names only some of the
        // answer's own separations is a difference, not a partial match, because a case
        // stating one entry asserts that the answer carries no other.
        let fixture = Fixture {
            lower: Some(CaseLower::new(
                Vec::new(),
                vec![(0, 150), (2, 300)],
                None,
                false,
                Vec::new(),
            )),
            ..Fixture::default()
        };
        let expected = r#"[{ "policy": {}, "source": "s",
            "expect": { "lower": { "construct": 0,
                                    "separations": [{ "after": 0, "least": 150 }] } } }]"#;
        let report = measure(wrap(LOWER_INPUT, expected, "[]"), &fixture);
        assert_eq!(
            (report.agreed, report.disagreed.len()),
            (0, 1),
            "the answer carries a second separation the expectation never named: {report:?}"
        );
    }

    #[test]
    fn a_lower_same_run_expectation_compares_the_answers_own_per_item_identity() {
        // `check_same_run`'s own predicate: two items share a run when both resolve `Some`
        // and equal. Item 0 and item 1 share run 1; item 2 carries no construct at all, so it
        // shares a run with nothing, including another uncovered item.
        let fixture = Fixture {
            lower: Some(CaseLower::new(
                vec![Some(1), Some(1), None],
                Vec::new(),
                None,
                false,
                Vec::new(),
            )),
            ..Fixture::default()
        };
        let expected = r#"[{ "policy": {}, "source": "s",
            "expect": { "lower": { "construct": 0, "same_run": [
                { "items": [0, 1], "same": true },
                { "items": [0, 2], "same": false }
            ] } } }]"#;
        let agreed = measure(wrap(LOWER_INPUT, expected, "[]"), &fixture);
        assert_eq!(
            (agreed.agreed, agreed.disagreed.len()),
            (1, 0),
            "{agreed:?}"
        );

        let wrong = r#"[{ "policy": {}, "source": "s",
            "expect": { "lower": { "construct": 0,
                                    "same_run": [{ "items": [0, 2], "same": true }] } } }]"#;
        let report = measure(wrap(LOWER_INPUT, wrong, "[]"), &fixture);
        assert_eq!(
            (report.agreed, report.disagreed.len()),
            (0, 1),
            "item 2 carries no construct, so it shares a run with nothing: {report:?}"
        );
    }

    #[test]
    fn a_place_case_reaches_compose_place_and_not_compose_compose() {
        // The identical trap `tab`'s, `feasible`'s and `lower`'s own pairs of tests above
        // name, now for the eighth question: a forgotten `"place" =>` arm in `ask` would
        // still compile and would still fall through to the wildcard, answered by
        // `Compose::compose` instead of `Compose::place`. `place` and `composed` are given
        // expectations that cannot agree by accident — `composed`'s own one-line answer
        // never satisfies a `place` expectation at all, since `check` refuses to read a
        // `Composed` answer as one (`check`'s own wildcard tuple arm) — so a fallthrough is a
        // scored `disagreed` rather than a coincidental pass.
        let fixture = Fixture {
            composed: Some(composed(Vec::new(), Vec::new())),
            place: Some(CasePlace::new(
                vec![CaseAttachment::new(250, Some(0))],
                Vec::new(),
            )),
            ..Fixture::default()
        };
        let expected = r#"[{ "policy": {}, "source": "s",
            "expect": { "place": { "attachments": [{ "inline": 250, "item": 0 }] } } }]"#;
        let report = measure(wrap(PLACE_INPUT, expected, "[]"), &fixture);
        assert_eq!(
            (report.attempted, report.agreed, report.disagreed.len()),
            (1, 1, 0),
            "the case's own expectation matches `Compose::place`'s own answer; a run wrongly \
             routed to `Compose::compose` would answer no `place` field at all and disagree \
             instead: {report:?}"
        );
    }

    #[test]
    fn a_place_case_with_no_place_answer_is_not_attempted_even_though_compose_has_one() {
        // The other half of the same trap: an implementation that answers `compose` but
        // declines `place` must be scored as not having attempted this case.
        let fixture = Fixture {
            composed: Some(composed(Vec::new(), Vec::new())),
            place: None,
            ..Fixture::default()
        };
        let report = measure(
            wrap(
                PLACE_INPUT,
                r#"[{ "policy": {}, "source": "s",
                      "expect": { "place": { "attachments": [] } } }]"#,
                "[]",
            ),
            &fixture,
        );
        assert_eq!(
            (report.attempted, report.skipped),
            (0, 1),
            "a `place` case asks `Compose::place`, and an implementation that answers only \
             `compose` has not attempted it: {report:?}"
        );
    }

    #[test]
    fn a_place_attachments_expectation_is_a_total_list_not_a_subset() {
        // `check_attachments`'s own convention, `check_lower_separations`'s own precedent
        // applied to this round's own field: a declared list that names only some of the
        // answer's own attachments is a difference, not a partial match, because a case
        // stating one entry asserts that the answer carries no other.
        let fixture = Fixture {
            place: Some(CasePlace::new(
                vec![
                    CaseAttachment::new(600, Some(0)),
                    CaseAttachment::new(1200, Some(1)),
                ],
                Vec::new(),
            )),
            ..Fixture::default()
        };
        let expected = r#"[{ "policy": {}, "source": "s",
            "expect": { "place": { "attachments": [{ "inline": 600, "item": 0 }] } } }]"#;
        let report = measure(wrap(PLACE_INPUT, expected, "[]"), &fixture);
        assert_eq!(
            (report.agreed, report.disagreed.len()),
            (0, 1),
            "the answer carries a second attachment the expectation never named: {report:?}"
        );
    }

    #[test]
    fn a_place_declined_expectation_names_the_specific_construct_ordinal() {
        // `check_declined`'s own predicate: the declared ordinals must equal the answer's own,
        // not merely agree on count — the point `cases.schema.json`'s own `place.declined`
        // description states explicitly, since "nothing was placed" is satisfiable by an
        // implementation that never placed anything at all.
        let fixture = Fixture {
            place: Some(CasePlace::new(Vec::new(), vec![0])),
            ..Fixture::default()
        };
        let expected = r#"[{ "policy": {}, "source": "s",
            "expect": { "place": { "declined": [0] } } }]"#;
        let agreed = measure(wrap(PLACE_INPUT, expected, "[]"), &fixture);
        assert_eq!(
            (agreed.agreed, agreed.disagreed.len()),
            (1, 0),
            "{agreed:?}"
        );

        let wrong = r#"[{ "policy": {}, "source": "s",
            "expect": { "place": { "declined": [1] } } }]"#;
        let report = measure(wrap(PLACE_INPUT, wrong, "[]"), &fixture);
        assert_eq!(
            (report.agreed, report.disagreed.len()),
            (0, 1),
            "the answer declined construct 0, not construct 1: {report:?}"
        );
    }

    #[test]
    fn a_declared_c_2_note_13_over_a_real_tate_chu_yoko_overlay_agrees_with_kumihan() {
        // The proof that the whole chain is live end to end: a real case, read through the
        // full JSON reader, checked against the real `Kumihan::feasible` — which itself
        // builds a real, non-`Runs::none()` overlay from `input.constructs` and hands it to
        // `jlreq_line::Feasible::compute` — rather than a `Fixture` standing in for either.
        // Two hiragana in one declared `tate_chu_yoko` run, over Table 2's blank cl-15
        // against cl-15 cell (verified against `spec/captured/table2.en.tsv` directly, not
        // by running this evaluator), so the refusal below can only be `same_run_refusal`'s
        // own answer and not a class-pair prohibition it would otherwise be indistinguishable
        // from.
        const ONE_RUN: &str = r#""input": {
            "kind": "feasible",
            "text": "あい",
            "scales": [{ "inline_em": 1000, "block_em": 1000 }],
            "items": [
                { "start": 0, "advance": 1000, "frame": "full-em", "scale": 0 },
                { "start": 3, "advance": 1000, "frame": "full-em", "scale": 0 }
            ],
            "constructs": { "tate_chu_yoko": [{ "items": [0, 2] }] },
            "candidates": [{ "at": 3 }]
        }"#;
        let case = wrap(
            ONE_RUN,
            r#"[{ "policy": {}, "source": "s",
                  "expect": { "feasible": { "candidate": 0, "breakable": false,
                                             "rules": ["C.2#13"] } } }]"#,
            "[]",
        );
        let report = run_file(&CaseFile::of("C.2", vec![case]), &Kumihan::default());
        assert_eq!(
            (report.agreed, report.disagreed.len()),
            (1, 0),
            "kumihan's own overlay marks both items members of the same declared run, and \
             `same_run_refusal` refuses the candidate between them, citing C.2#13: {report:?}"
        );
    }

    #[test]
    fn a_trim_is_compared_field_by_field_and_not_by_how_many_there_are() {
        // The assertion `docs/design/conformance.md`'s worked case rests on: an
        // implementation that shortens the caller's advance without saying so passes both
        // `extent` checks and fails on `trims`. Comparing the two by length alone passed a
        // trim taken out of the wrong item, attributed to the wrong neighbor, or justified
        // by the wrong sentence — every way of taking a unit out of a caller's advance and
        // telling them something else (ADR 0002, ADR 0017).
        let expected = r#"[{ "policy": {}, "source": "s", "expect": { "lines": [
            { "trims": [{ "item": 0, "em": [1, 2], "units": 360,
                          "referent": "preceding", "rule": "3.1.2" }] }] } }]"#;
        let right = CaseTrim::new(0, 360, "preceding".to_owned(), "3.1.2".to_owned());
        let agreed = measure(
            wrap(COMPOSED_INPUT, expected, "[]"),
            &Fixture {
                composed: Some(composed(vec![right], Vec::new())),
                ..Fixture::default()
            },
        );
        assert_eq!((agreed.agreed, agreed.disagreed.len()), (1, 0));

        for wrong in [
            CaseTrim::new(1, 360, "preceding".to_owned(), "3.1.2".to_owned()),
            CaseTrim::new(0, 180, "preceding".to_owned(), "3.1.2".to_owned()),
            CaseTrim::new(0, 360, "trailing".to_owned(), "3.1.2".to_owned()),
            CaseTrim::new(0, 360, "preceding".to_owned(), "3.1.4".to_owned()),
        ] {
            let report = measure(
                wrap(COMPOSED_INPUT, expected, "[]"),
                &Fixture {
                    composed: Some(composed(vec![wrong.clone()], Vec::new())),
                    ..Fixture::default()
                },
            );
            assert_eq!(
                report.disagreed.len(),
                1,
                "{wrong:?} differs from the expected trim and was scored as an agreement"
            );
        }
    }

    #[test]
    fn a_segment_laid_out_along_the_wrong_axis_is_a_difference() {
        // §3.2.5's tate-chu-yoko (縦中横) does not run along the line's inline axis, so its
        // interior items share the segment's `placements` entry and differ only in
        // `parts[].across`. An implementation that laid the run out along the line produces
        // the same number of parts, which is the one input this comparison exists for.
        let expected = r#"[{ "policy": {}, "source": "s", "expect": { "lines": [
            { "parts": [{ "segment": 0, "index": 0, "items": [0, 2], "inline": 0,
                          "block": 0, "extent": 1000, "across": [0, 500] }] }] } }]"#;
        let part = |across: Vec<i64>| CasePart::new(0, 0, (0, 2), 0, 0, 1000, across);
        let agreed = measure(
            wrap(COMPOSED_INPUT, expected, "[]"),
            &Fixture {
                composed: Some(composed(Vec::new(), vec![part(vec![0, 500])])),
                ..Fixture::default()
            },
        );
        assert_eq!((agreed.agreed, agreed.disagreed.len()), (1, 0));
        let report = measure(
            wrap(COMPOSED_INPUT, expected, "[]"),
            &Fixture {
                composed: Some(composed(Vec::new(), vec![part(vec![0, 0])])),
                ..Fixture::default()
            },
        );
        assert_eq!(report.disagreed.len(), 1, "{report:?}");
    }

    #[test]
    fn a_permitted_expectation_about_another_question_matches_no_answer() {
        // The hole that made a case unable to fail: a `classify` case whose only permitted
        // entry stated `violations` was compared against nothing and scored as an agreement.
        // A `forbidden` entry states only the fields it forbids, so the same shape there
        // excludes nothing and stays silent — the asymmetry is the point.
        let answer = CaseClass::new(15, Vec::new());
        let report = measure(
            wrap(
                INPUT,
                r#"[{ "policy": {}, "source": "s", "expect": { "violations": [] } }]"#,
                "[]",
            ),
            &Fixture {
                class: Some(answer.clone()),
                ..Fixture::default()
            },
        );
        assert_eq!(report.disagreed.len(), 1, "{report:?}");

        let report = measure(
            wrap(
                INPUT,
                r#"[{ "policy": {}, "source": "s",
                      "expect": { "class": { "item": 0, "class": 15 } } }]"#,
                r#"[{ "expect": { "lines": [{ "extent": 3500 }] }, "why": "w" }]"#,
            ),
            &Fixture {
                class: Some(answer),
                ..Fixture::default()
            },
        );
        assert_eq!(
            (report.agreed, report.disagreed.len()),
            (1, 0),
            "a forbidden line geometry says nothing about a classification: {report:?}"
        );
    }

    #[test]
    fn an_expectation_about_another_occurrence_is_neither_an_agreement_nor_an_exclusion() {
        // The ordinal a case asks about is its first stated expectation's, and one answer is
        // then measured against every entry. A `forbidden` entry about item 2 used to
        // exclude a correct answer about item 0; a `permitted` entry about item 2 used to be
        // matched against it and to agree.
        let answer = CaseClass::new(1, Vec::new());
        let report = measure(
            wrap(
                INPUT,
                r#"[{ "policy": {}, "source": "s",
                      "expect": { "class": { "item": 0, "class": 1 } } }]"#,
                r#"[{ "expect": { "class": { "item": 2, "class": 1 } }, "why": "w" }]"#,
            ),
            &Fixture {
                class: Some(answer.clone()),
                ..Fixture::default()
            },
        );
        assert_eq!(
            (report.agreed, report.disagreed.len()),
            (1, 0),
            "a forbidden entry about item 2 excludes nothing about item 0: {report:?}"
        );

        let report = measure(
            wrap(
                INPUT,
                r#"[{ "policy": {}, "source": "a",
                      "expect": { "class": { "item": 0, "class": 19 } } },
                    { "policy": { "kinsoku.level": "loose" }, "source": "b",
                      "expect": { "class": { "item": 2, "class": 1 } } }]"#,
                "[]",
            ),
            &Fixture {
                class: Some(answer),
                ..Fixture::default()
            },
        );
        assert_eq!(
            report.disagreed.len(),
            1,
            "and the second entry, which is about item 2, is satisfied by no answer about              item 0 — the ordinal a case asks about is the first stated expectation's, so              matching that entry would be agreeing with a reading of another occurrence:              {report:?}"
        );
    }

    #[test]
    fn an_implementation_that_declares_nothing_is_measured_against_every_reading() {
        // The branch `Kumihan` never reaches, and the one that makes this suite runnable
        // against a browser or InDesign: an implementation declaring no policy agrees if it
        // matches any permitted entry, rather than the one a declared policy would select.
        let readings = r#"[
            { "policy": {}, "source": "a", "expect": { "class": { "item": 0, "class": 1 } } },
            { "policy": { "kinsoku.level": "loose" }, "source": "b",
              "expect": { "class": { "item": 0, "class": 19 } } }]"#;
        let report = measure(
            wrap(INPUT, readings, "[]"),
            &Fixture {
                class: Some(CaseClass::new(19, Vec::new())),
                ..Fixture::default()
            },
        );
        assert_eq!(
            (report.agreed, report.disagreed.len(), report.unselectable),
            (1, 0, 0),
            "the second reading is one of the case's own, and an implementation measured \
             against all of them can select every one: {report:?}"
        );

        let report = measure(
            wrap(INPUT, readings, "[]"),
            &Fixture {
                class: Some(CaseClass::new(19, Vec::new())),
                policy: Some(CasePolicy::new()),
                ..Fixture::default()
            },
        );
        assert_eq!(
            (report.agreed, report.disagreed.len(), report.unselectable),
            (0, 1, 1),
            "a policy that has no `kinsoku.level` selects the fallback entry and cannot \
             reach the second, which is counted rather than passed over: {report:?}"
        );
    }

    #[test]
    fn a_question_the_implementation_does_not_answer_is_not_attempted() {
        let report = measure(
            wrap(
                INPUT,
                r#"[{ "policy": {}, "source": "s",
                      "expect": { "class": { "item": 0, "class": 15 } } }]"#,
                "[]",
            ),
            &Fixture {
                boundary: Some(CaseBoundary::new(
                    Vec::new(),
                    true,
                    true,
                    None,
                    CaseExpansion::new("none".to_owned(), None, None, None),
                    Vec::new(),
                )),
                ..Fixture::default()
            },
        );
        assert_eq!(
            (report.attempted, report.skipped),
            (0, 1),
            "a classification case asks `classify`, and an implementation that answers only \
             boundaries has not attempted it: {report:?}"
        );
    }

    #[test]
    fn a_case_files_edge_reaches_the_implementation_through_the_full_reader() {
        // The middle of the wire nothing else exercises: `"edge": "end"` in a case file,
        // through `read_boundary`, `ExpectBoundary::edge`, `ask`'s own `edge_of`, and into
        // `Compose::boundary` itself. A typo in the JSON key or a spelling mismatch between
        // the schema's enum and `edge_of`'s two arms would leave item 2 dead everywhere else
        // this module tests it, because `Kumihan::boundary` calls above never go through
        // the case reader at all.
        let fixture = Fixture {
            boundary: Some(CaseBoundary::new(
                Vec::new(),
                true,
                true,
                None,
                CaseExpansion::new("none".to_owned(), None, None, None),
                Vec::new(),
            )),
            ..Fixture::default()
        };
        measure(
            wrap(
                BOUNDARY_INPUT,
                r#"[{ "policy": {}, "source": "s",
                      "expect": { "boundary": { "before": 0, "edge": "end" } } }]"#,
                "[]",
            ),
            &fixture,
        );
        assert_eq!(
            fixture.received_edge.get(),
            Some(Edge::End),
            "the case names a line-end boundary, and the implementation must be asked about \
             exactly that, not an interior one"
        );

        let interior = Fixture {
            boundary: Some(CaseBoundary::new(
                Vec::new(),
                true,
                true,
                None,
                CaseExpansion::new("none".to_owned(), None, None, None),
                Vec::new(),
            )),
            ..Fixture::default()
        };
        measure(
            wrap(
                BOUNDARY_INPUT,
                r#"[{ "policy": {}, "source": "s",
                      "expect": { "boundary": { "before": 0 } } }]"#,
                "[]",
            ),
            &interior,
        );
        assert_eq!(
            interior.received_edge.get(),
            None,
            "a case naming no edge at all asks about an interior boundary: {:?}",
            interior.received_edge.get()
        );
    }

    #[test]
    fn an_expectation_that_states_nothing_about_the_question_differs_from_the_answer() {
        // Both sides of a case read it that way, which is what makes a `forbidden` entry
        // about another question exclude nothing and a `permitted` one satisfy nothing. The
        // fully silent expectation never reaches here on the `forbidden` side, because
        // `Expect::is_silent` decides that before `check` is consulted.
        let expect = Expect {
            class: None,
            boundary: None,
            feasible: None,
            lower: None,
            place: None,
            lines: None,
            violations: None,
        };
        let answer = Answer::Class(0, CaseClass::new(15, Vec::new()));
        let found = check(&expect, &answer);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found
                .iter()
                .any(|message| message.contains("the class of item 0"))
        );
    }

    /// A boundary answer carrying the given expansion and nothing else, for
    /// `check_expansion`'s own three tests below.
    fn boundary_with_expansion(expansion: CaseExpansion) -> CaseBoundary {
        CaseBoundary::new(Vec::new(), true, true, None, expansion, Vec::new())
    }

    #[test]
    fn check_expansion_ignores_an_expectation_that_states_no_rule_at_all() {
        // The first of `check_expansion`'s own three branches: an expectation naming no
        // `rule` asserts nothing about it, the schema's own general "an absent field
        // asserts nothing" rule, unchanged by this round for every field this one does not
        // name.
        let expected = r#"[{ "policy": {}, "source": "s",
            "expect": { "boundary": { "before": 0,
                "expansion": { "kind": "range", "ceiling": { "em": [1, 4], "units": 180 },
                                "stage": 3 } } } }]"#;
        let answer = boundary_with_expansion(CaseExpansion::new(
            "range".to_owned(),
            Some(180),
            Some(3),
            Some("E.2#10".to_owned()),
        ));
        let report = measure(
            wrap(BOUNDARY_INPUT, expected, "[]"),
            &Fixture {
                boundary: Some(answer),
                ..Fixture::default()
            },
        );
        assert_eq!(
            (report.agreed, report.disagreed.len()),
            (1, 0),
            "a silent rule field asserts nothing about the address, whatever the answer \
             publishes: {report:?}"
        );
    }

    #[test]
    fn check_expansion_passes_over_a_declared_rule_when_the_answer_publishes_none() {
        // The second branch, and the one this round adds: an expectation that states a
        // `rule` is passed over, not failed, when the answer's own citation is `None` — the
        // reading `check_class`'s own doc already gives an implementation that answers a
        // question without publishing a specification address, applied here to one field of
        // a boundary answer rather than to a whole classification.
        let expected = r#"[{ "policy": {}, "source": "s",
            "expect": { "boundary": { "before": 0,
                "expansion": { "kind": "range", "ceiling": { "em": [1, 4], "units": 180 },
                                "stage": 3, "rule": "E.2#4" } } } }]"#;
        let answer = boundary_with_expansion(CaseExpansion::new(
            "range".to_owned(),
            Some(180),
            Some(3),
            None,
        ));
        let report = measure(
            wrap(BOUNDARY_INPUT, expected, "[]"),
            &Fixture {
                boundary: Some(answer),
                ..Fixture::default()
            },
        );
        assert_eq!(
            (report.agreed, report.disagreed.len()),
            (1, 0),
            "a declared rule met by no published citation is passed over, never failed: \
             {report:?}"
        );
    }

    #[test]
    fn check_expansion_fails_when_both_sides_publish_different_rules() {
        // The third branch: both sides name an address, and the addresses disagree — the
        // one shape that is a real divergence rather than a decliner meeting a case that
        // expects more provenance than it can state.
        let expected = r#"[{ "policy": {}, "source": "s",
            "expect": { "boundary": { "before": 0,
                "expansion": { "kind": "range", "ceiling": { "em": [1, 4], "units": 180 },
                                "stage": 3, "rule": "E.2#4" } } } }]"#;
        let answer = boundary_with_expansion(CaseExpansion::new(
            "range".to_owned(),
            Some(180),
            Some(3),
            Some("E.2#10".to_owned()),
        ));
        let report = measure(
            wrap(BOUNDARY_INPUT, expected, "[]"),
            &Fixture {
                boundary: Some(answer),
                ..Fixture::default()
            },
        );
        assert_eq!(
            (report.agreed, report.disagreed.len()),
            (0, 1),
            "two different declared addresses at the identical coordinate is a real \
             disagreement, not a pass-over: {report:?}"
        );
    }

    #[test]
    fn a_declared_b_2_17_at_the_default_policy_line_head_boundary_agrees_with_kumihan() {
        // The proof that `check_rules` is actually live, which a green suite could otherwise
        // hide: a real case, read through the full JSON reader, checked against the real
        // evaluator under the default policy — not a `Fixture` standing in for one.
        // `Kumihan::default()` declares `Policy::JLREQ`, whose own answer to
        // `spacing.line_head_opening_bracket` is `pattern-1`, so the half-em space this same
        // coordinate can also carry under `pattern-2` never fires here; what this test pins is
        // the citation alone, which `rules_fired` puts into `rules[1]` from Table 1's own
        // `(0, 1)` placement cell under every policy
        // (`crates/jlreq-spacing/src/evaluate.rs`'s own `line_head_opening_bracket_space` doc,
        // point 3, and `docs/conformance-deferrals.toml`'s own `B.2#17` entry). This also
        // asserts, rather than assumes, that `RuleId::B_2_NOTE_17`'s own canonical rendering
        // is exactly the string `"B.2#17"` a case must write: were it ever spelled
        // differently, `Kumihan::boundary`'s real answer would carry the new spelling and this
        // literal would stop matching it.
        const OPENING_BRACKET_AT_LINE_HEAD: &str = r#""input": {
            "kind": "boundary",
            "text": "「",
            "scales": [{ "inline_em": 1000, "block_em": 1000 }],
            "items": [{ "start": 0, "advance": 500, "frame": "half-em", "scale": 0 }]
        }"#;
        let case = wrap(
            OPENING_BRACKET_AT_LINE_HEAD,
            r#"[{ "policy": {}, "source": "s",
                  "expect": { "boundary": { "before": 0, "edge": "head",
                                             "rules": ["B.2#17"] } } }]"#,
            "[]",
        );
        let report = run_file(&CaseFile::of("B.2", vec![case]), &Kumihan::default());
        assert_eq!(
            (report.agreed, report.disagreed.len()),
            (1, 0),
            "kumihan's own evaluator publishes `B.2#17` at cl-01's line-head boundary under \
             every policy, and the case declares only that address, so this must agree: \
             {report:?}"
        );
    }

    #[test]
    fn a_declared_rule_the_answer_does_not_publish_is_reported() {
        // The one shape `check_rules` treats as a real divergence: the answer names
        // something, and the something it names does not include the address the case
        // declared.
        let expected = r#"[{ "policy": {}, "source": "s",
            "expect": { "boundary": { "before": 0, "rules": ["B.2#17"] } } }]"#;
        let answer = CaseBoundary::new(
            Vec::new(),
            true,
            true,
            None,
            CaseExpansion::new("none".to_owned(), None, None, None),
            vec!["3.1.1".to_owned()],
        );
        let report = measure(
            wrap(BOUNDARY_INPUT, expected, "[]"),
            &Fixture {
                boundary: Some(answer),
                ..Fixture::default()
            },
        );
        assert_eq!(
            (report.agreed, report.disagreed.len()),
            (0, 1),
            "the answer published something and it was not the declared address, which is a \
             real disagreement: {report:?}"
        );
    }

    #[test]
    fn extra_answered_rules_and_a_different_order_are_both_accepted() {
        // The subset semantics pinned: `want` names two of the three addresses the answer
        // publishes, and names them in the opposite order the answer does. Neither the extra
        // address nor the order is a difference — `check_rules` asks only whether each
        // declared address appears somewhere among the answered ones.
        let expected = r#"[{ "policy": {}, "source": "s",
            "expect": { "boundary": { "before": 0, "rules": ["B.2#17", "3.1.1"] } } }]"#;
        let answer = CaseBoundary::new(
            Vec::new(),
            true,
            true,
            None,
            CaseExpansion::new("none".to_owned(), None, None, None),
            vec!["3.1.1".to_owned(), "C.2#7".to_owned(), "B.2#17".to_owned()],
        );
        let report = measure(
            wrap(BOUNDARY_INPUT, expected, "[]"),
            &Fixture {
                boundary: Some(answer),
                ..Fixture::default()
            },
        );
        assert_eq!(
            (report.agreed, report.disagreed.len()),
            (1, 0),
            "every declared address appears somewhere among the answered ones, so this agrees \
             regardless of the answer's own extra address and its own different order: \
             {report:?}"
        );
    }

    #[test]
    fn a_declared_rule_met_by_an_empty_answer_is_passed_over() {
        // The third state `check_rules` shares with `check_expansion`'s own conditional
        // `rule` field: an answer that publishes no rules at all meets a declared expectation
        // without failing it, because an implementation that publishes nothing must stay
        // measurable by every other field this suite checks. Kumihan's own answers never take
        // this branch — `rules_fired` always yields at least two entries — so this is a
        // foreign-implementation affordance, exercised here through `Fixture` rather than
        // through `Kumihan`.
        let expected = r#"[{ "policy": {}, "source": "s",
            "expect": { "boundary": { "before": 0, "rules": ["B.2#17"] } } }]"#;
        let answer = CaseBoundary::new(
            Vec::new(),
            true,
            true,
            None,
            CaseExpansion::new("none".to_owned(), None, None, None),
            Vec::new(),
        );
        let report = measure(
            wrap(BOUNDARY_INPUT, expected, "[]"),
            &Fixture {
                boundary: Some(answer),
                ..Fixture::default()
            },
        );
        assert_eq!(
            (report.agreed, report.disagreed.len()),
            (1, 0),
            "an answer publishing no rules at all is passed over, never failed: {report:?}"
        );
    }

    #[test]
    fn rules_exercised_is_built_from_the_answer_alone_not_from_check_rules() {
        // `Report::rules_exercised` drives the coverage gate and is populated from
        // `Answer::rules` at `measure` time, before `check` (and this round's `check_rules`
        // inside it) ever runs — a disagreement over a missing declared address must not
        // change what the report says the answer itself exercised.
        let expected = r#"[{ "policy": {}, "source": "s",
            "expect": { "boundary": { "before": 0, "rules": ["B.2#17"] } } }]"#;
        let answer = CaseBoundary::new(
            Vec::new(),
            true,
            true,
            None,
            CaseExpansion::new("none".to_owned(), None, None, None),
            vec!["3.1.1".to_owned()],
        );
        let report = measure(
            wrap(BOUNDARY_INPUT, expected, "[]"),
            &Fixture {
                boundary: Some(answer),
                ..Fixture::default()
            },
        );
        assert_eq!(
            report.rules_exercised,
            BTreeSet::from(["3.1.1".to_owned()]),
            "the answer published `3.1.1` and nothing else, and that is what the coverage \
             gate reads regardless of the disagreement over the declared `B.2#17`: {report:?}"
        );
    }
}
