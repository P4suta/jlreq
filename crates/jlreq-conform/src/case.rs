// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The published case format, read into types.
//!
//! One file per JLReq section, each holding cases addressed to that section. The shape is
//! `crates/jlreq-conform/cases.schema.json` and the prose is `docs/design/conformance.md`;
//! the two are one contract stated twice, and `cargo run -p xtask -- conform --check`
//! validates the committed files against it before anything here reads them. What this
//! module adds is the reading, so the same bytes a browser engineer validates against the
//! schema are the bytes this workspace is measured against.
//!
//! A field a case does not state is `None` and asserts nothing. That is the schema's own
//! rule for a `forbidden` entry — "a `forbidden` entry states only the fields it forbids"
//! — and making it uniform over `permitted` is what lets a classification case say what it
//! knows about a class without also having to state a line geometry it is not about.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::json::{Json, JsonError};

/// The file extension a case file carries.
const EXTENSION: &str = "json";

/// One (question path, choice name) map: an overlay on `Policy::JLREQ` in a case, and a
/// total map in an implementation's own declaration.
///
/// Ordered rather than hashed, because a report naming the entry that was selected must
/// name the same one on every platform.
pub type CasePolicy = BTreeMap<String, String>;

/// Why a case file could not be read.
#[derive(Debug)]
#[non_exhaustive]
pub struct LoadError {
    /// The file it happened in.
    path: PathBuf,
    /// The case it happened in, when the reader had got that far.
    case: Option<String>,
    /// What was wrong.
    reason: String,
}

impl LoadError {
    /// A reading error against a whole file.
    fn file(path: &Path, reason: impl Into<String>) -> Self {
        Self {
            path: path.to_path_buf(),
            case: None,
            reason: reason.into(),
        }
    }

    /// The same, named to the case it happened in.
    fn within(mut self, case: &str) -> Self {
        if self.case.is_none() {
            self.case = Some(case.to_owned());
        }
        self
    }
}

impl fmt::Display for LoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.case {
            Some(case) => write!(
                formatter,
                "{path}: {case}: {reason}",
                path = self.path.display(),
                case = case,
                reason = self.reason
            ),
            None => write!(
                formatter,
                "{path}: {reason}",
                path = self.path.display(),
                reason = self.reason
            ),
        }
    }
}

impl std::error::Error for LoadError {}

impl From<(&Path, io::Error)> for LoadError {
    fn from((path, error): (&Path, io::Error)) -> Self {
        Self::file(path, error.to_string())
    }
}

impl From<(&Path, JsonError)> for LoadError {
    fn from((path, error): (&Path, JsonError)) -> Self {
        Self::file(path, error.to_string())
    }
}

/// The whole published suite: every case file under one directory.
#[derive(Debug)]
#[non_exhaustive]
pub struct Suite {
    /// The files, in the order the directory sorts them.
    files: Vec<CaseFile>,
}

impl Suite {
    /// The files, in a stable order.
    #[must_use]
    pub fn files(&self) -> &[CaseFile] {
        &self.files
    }

    /// Every case of every file, in reading order.
    pub fn cases(&self) -> impl Iterator<Item = &Case> {
        self.files.iter().flat_map(CaseFile::cases)
    }

    /// The one file addressed to a section, when the suite has one.
    #[must_use]
    pub fn file(&self, section: &str) -> Option<&CaseFile> {
        self.files.iter().find(|file| file.section == section)
    }
}

/// One case file: the section it is addressed to, and the cases it publishes.
#[derive(Debug)]
#[non_exhaustive]
pub struct CaseFile {
    /// The JLReq section, in the canonical address rendering. Also the file's own name.
    section: String,
    /// Where it was read from.
    path: PathBuf,
    /// The cases, in the order the file writes them.
    cases: Vec<Case>,
}

impl CaseFile {
    /// The section this file is addressed to.
    #[must_use]
    pub fn section(&self) -> &str {
        &self.section
    }

    /// Where it was read from.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The cases, in the order the file writes them.
    #[must_use]
    pub fn cases(&self) -> &[Case] {
        &self.cases
    }

    /// One file's worth of cases, for the runner's own tests.
    #[cfg(test)]
    pub(crate) fn of(section: &str, cases: Vec<Case>) -> Self {
        Self {
            section: section.to_owned(),
            path: PathBuf::from(section),
            cases,
        }
    }
}

/// One case: one input, and what the specification permits and forbids as an answer.
#[derive(Debug)]
#[non_exhaustive]
pub struct Case {
    /// `<section>/<subject>/<variant>`, unique across the suite.
    id: String,
    /// Every rule this case exercises, by address.
    rules: Vec<String>,
    /// What kind of claim it makes: `normative`, `alternative`, `unstated`, `adjudicated`.
    standing: String,
    /// The specification sentence, verbatim.
    quote: String,
    /// Why the expectations follow from it.
    rationale: String,
    /// What the case supplies.
    input: CaseInput,
    /// The (policy, expectation) pairs the specification permits.
    permitted: Vec<Permitted>,
    /// Outcomes the specification excludes.
    forbidden: Vec<Forbidden>,
}

impl Case {
    /// `<section>/<subject>/<variant>`, unique across the suite.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Every rule this case exercises, by address.
    #[must_use]
    pub fn rules(&self) -> &[String] {
        &self.rules
    }

    /// What kind of claim it makes.
    #[must_use]
    pub fn standing(&self) -> &str {
        &self.standing
    }

    /// The specification sentence, verbatim, so a report is readable without our source.
    #[must_use]
    pub fn quote(&self) -> &str {
        &self.quote
    }

    /// Why the expectations follow from that sentence.
    #[must_use]
    pub fn rationale(&self) -> &str {
        &self.rationale
    }

    /// What the case supplies, which is what the eight trait methods are asked about.
    #[must_use]
    pub fn input(&self) -> &CaseInput {
        &self.input
    }

    /// The (policy, expectation) pairs the specification permits.
    #[must_use]
    pub fn permitted(&self) -> &[Permitted] {
        &self.permitted
    }

    /// Outcomes the specification excludes even though they lie between two permitted ones.
    #[must_use]
    pub fn forbidden(&self) -> &[Forbidden] {
        &self.forbidden
    }
}

/// One permitted answer, with the policy that selects it and the source that states it.
#[derive(Debug)]
#[non_exhaustive]
pub struct Permitted {
    /// The overlay on `Policy::JLREQ` that selects this entry.
    pub policy: CasePolicy,
    /// Who states this answer.
    pub source: String,
    /// What an implementation should answer.
    pub expect: Expect,
}

/// One outcome the specification excludes, and the sentence that excludes it.
#[derive(Debug)]
#[non_exhaustive]
pub struct Forbidden {
    /// The excluded outcome, stating only the fields it forbids.
    pub expect: Expect,
    /// The sentence that excludes it.
    pub why: String,
}

/// What the case supplies: the base stream, the annotation streams, the constructs over
/// them, the break opportunities, and the measure.
#[derive(Debug)]
#[non_exhaustive]
pub struct CaseInput {
    /// Which of the eight questions this case asks.
    pub kind: String,
    /// The base running-text stream, in reading order.
    pub text: String,
    /// The character sizes the stream declares.
    pub scales: Vec<CaseScale>,
    /// One item per occurrence, in reading order.
    pub items: Vec<CaseItem>,
    /// Further streams, each the same shape (ADR 0016).
    pub annotations: Vec<CaseStream>,
    /// The constructs declared over the streams.
    pub constructs: Vec<CaseConstruct>,
    /// The break opportunities the caller's UAX #14 implementation found.
    pub candidates: Vec<usize>,
    /// The line length, in the caller's own unit.
    pub measure: Option<i64>,
    /// Which of `jlreq_line::Search`'s two variants a `compose` case is measured under.
    /// `None` reads as `Search::FirstFit`, the reading every case published before this
    /// field existed already assumed (`cases.schema.json`'s own `search` description).
    pub search: Option<CaseSearch>,
    /// The writing direction, when the case is specific to one.
    pub direction: Option<String>,
    /// The paragraph's first-line indent, in the caller's own unit.
    pub first_line_indent: Option<i64>,
    /// The paragraph's line head indent, in the caller's own unit, narrowing every line's
    /// own measure rather than only the first.
    pub head_indent: Option<i64>,
    /// The paragraph's line end indent, in the caller's own unit, narrowing every line's own
    /// composition target from the line end side.
    pub end_indent: Option<i64>,
    /// §3.5.4's own widow threshold: the fewest items the paragraph's own last line must
    /// carry, read in the caller's own unit-free item count rather than a length. `None`
    /// reads as `0`, `Paragraph::new`'s own default and a no-op by construction
    /// (`cases.schema.json`'s own `widow_threshold` description).
    pub widow_threshold: Option<i64>,
    /// Which of `jlreq_line::Alignment`'s four methods an `align` case asks for.
    pub alignment: Option<String>,
    /// For a `tab` case: `starts[k]` is the item ordinal where the run after the `k`-th
    /// tab sign begins — `jlreq_line::tab_line`'s own `starts`. Empty for any other kind.
    pub tab_starts: Vec<usize>,
    /// For a `tab` case: the caller's own declared pool of tab positions and their
    /// alignment kinds — `jlreq_line::tab_line`'s own `stops`. Empty for any other kind.
    pub tab_stops: Vec<CaseTabStop>,
}

impl CaseInput {
    /// Whether a declared construct covers this item of the base stream.
    ///
    /// The construct axis is not a parameter of classification and cannot be: a construct
    /// is a run over a stream rather than a property of one item, so it lives in
    /// `jlreq-inline` (ADR 0015). An implementation that is not given it says so by
    /// answering `None` for an occurrence a construct covers, which is what the trait's
    /// `Option` means.
    #[must_use]
    pub fn construct_covers(&self, item: usize) -> bool {
        self.constructs
            .iter()
            .any(|construct| construct.covers(item))
    }
}

/// One declared character size, in the caller's own unit.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct CaseScale {
    /// The em along the inline axis.
    pub inline_em: i64,
    /// The em along the block axis. Anisotropic on purpose (§3.3.3).
    pub block_em: i64,
}

/// One occurrence of one Appendix A key (ADR 0018).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct CaseItem {
    /// Where this occurrence begins, in bytes of its own stream.
    pub start: usize,
    /// How wide the caller measured it, in their own unit.
    pub advance: i64,
    /// Which of the stream's declared sizes it is set at.
    pub scale: usize,
    /// What the supplied advance covers: the character frame (字幅).
    pub frame: Option<String>,
    /// The syntactic job the document gives this occurrence.
    pub role: Option<String>,
}

/// Which search a `compose` case is measured under: `jlreq_line::Search` in the case
/// format's own spelling, `cases.schema.json`'s own `search` `$def`.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct CaseSearch {
    /// `"first-fit"` or `"optimal"`.
    pub kind: String,
    /// `Search::Optimal`'s own `tolerance`. Required alongside `kind: "optimal"`, absent
    /// for `"first-fit"`.
    pub tolerance: Option<i64>,
}

/// One declared tab stop: `jlreq_line::TabStop` and `jlreq_line::TabKind` in the case
/// format's own spelling. `at` is present only for `kind: "character"`, the same
/// flattening `cases.schema.json`'s own `tab_stop` `$def` states in full.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct CaseTabStop {
    /// Where this stop sits, in the caller's own unit.
    pub position: i64,
    /// `start`, `end`, `centered`, or `character`.
    pub kind: String,
    /// Which occurrence `kind: "character"` names, absent for the other three.
    pub at: Option<usize>,
}

/// One annotation stream: the same shape as the base one (ADR 0016).
#[derive(Debug)]
#[non_exhaustive]
pub struct CaseStream {
    /// The stream's text, in reading order.
    pub text: String,
    /// The character sizes it declares.
    pub scales: Vec<CaseScale>,
    /// One item per occurrence.
    pub items: Vec<CaseItem>,
}

/// One declared construct, named by its kind and its position in its own array.
///
/// The interior shape differs by kind and only two of the nine are pinned by the schema, so
/// what is read here is the part every kind states: the ranges of the base stream it runs
/// over, which the format spells `base`, `items` or `mark`, plus `ruby`'s own `style`,
/// `annotation` and `runs` — present on no other kind, read here rather than left for
/// `Compose::feasible`'s or `Compose::lower`'s own adapter to reach into the raw JSON a
/// second time.
#[derive(Debug)]
#[non_exhaustive]
pub struct CaseConstruct {
    /// `ruby`, `warichu`, `tate_chu_yoko`, and the rest.
    pub kind: String,
    /// Its position in its own array, which is how a report names it (ADR 0015).
    pub index: usize,
    /// The half-open ranges of base-stream ordinals it runs over.
    pub ranges: Vec<(usize, usize)>,
    /// `ruby`'s own `"mono"`, `"group"` or `"jukugo"`, when the entry states one. `None` for
    /// every other kind, and for a `ruby` entry that leaves it unstated — the schema makes
    /// it optional, and `crates/jlreq-conform/src/kumihan.rs`'s own construct-to-
    /// `ConstructKind` map declines rather than guessing between `NonJukugoRuby` and
    /// `JukugoRuby` when this is `None`.
    pub style: Option<String>,
    /// `ruby`'s own `annotation` stream ordinal, indexing `CaseInput::annotations`. `None`
    /// for every other kind, and for a `ruby` entry that leaves it unstated.
    pub annotation: Option<usize>,
    /// `ruby`'s own `runs` pairing, in declaration order. Empty for every other kind.
    pub runs: Vec<CaseRun>,
}

/// One run pairing inside a declared `ruby` construct: which base characters this reading
/// belongs to (`cases.schema.json`'s own `run` `$def`).
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct CaseRun {
    /// The half-open range of base-stream ordinals this run covers.
    pub base: (usize, usize),
    /// The half-open range of the ruby's own annotation stream this run reads.
    pub annotation: (usize, usize),
}

impl CaseConstruct {
    /// Whether this construct runs over the given base-stream ordinal.
    #[must_use]
    pub fn covers(&self, item: usize) -> bool {
        self.ranges
            .iter()
            .any(|&(first, past)| item >= first && item < past)
    }
}

/// What an implementation should answer. Every field is optional and a missing one asserts
/// nothing, which is the schema's own rule for a `forbidden` entry made uniform.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct Expect {
    /// A classification answer.
    pub class: Option<ExpectClass>,
    /// A boundary answer.
    pub boundary: Option<ExpectBoundary>,
    /// A feasible-break answer.
    pub feasible: Option<ExpectFeasible>,
    /// A `jlreq_inline::lower` answer for one declared ruby construct.
    pub lower: Option<ExpectLower>,
    /// A `jlreq_inline::place` answer for the case's own whole declared `Constructs`.
    pub place: Option<ExpectPlace>,
    /// The composed lines.
    pub lines: Option<Vec<ExpectLine>>,
    /// Every rule the composition could not satisfy.
    pub violations: Option<Vec<String>>,
}

impl Expect {
    /// Whether this expectation states nothing at all.
    #[must_use]
    pub fn is_silent(&self) -> bool {
        self.class.is_none()
            && self.boundary.is_none()
            && self.feasible.is_none()
            && self.lower.is_none()
            && self.place.is_none()
            && self.lines.is_none()
            && self.violations.is_none()
    }
}

/// A classification answer: the class number §3.9.2 gives, and the rules that decided it.
#[derive(Debug)]
#[non_exhaustive]
pub struct ExpectClass {
    /// Which item of the base stream, defaulting to the first.
    pub item: usize,
    /// The class number, 1 through 30.
    pub class: Option<u8>,
    /// The rules the case says decided it.
    pub rules: Option<Vec<String>>,
}

/// A boundary answer. The conditional spaces, never their sum (ADR 0014).
#[derive(Debug)]
#[non_exhaustive]
pub struct ExpectBoundary {
    /// The boundary asked about: the ordinal of the one item on its real side — the item it
    /// follows for an interior boundary or a line end, the item it precedes at a line head.
    pub before: usize,
    /// Which line edge this boundary sits at (`"head"` or `"end"`), or `None` for an
    /// ordinary interior boundary. `cases.schema.json`'s own `boundary.edge`.
    pub edge: Option<String>,
    /// One conditional space per neighbor that contributes one.
    pub spaces: Option<Vec<ExpectSpace>>,
    /// Whether a line may break here.
    pub breakable: Option<bool>,
    /// Whether the adjacency is permitted at all.
    pub permitted: Option<bool>,
    /// How far ruby may overhang here.
    pub ruby_overhang: Option<CaseAmount>,
    /// The boundary's own Table 6 opportunity, independent of `spaces` (ADR 0014, amended
    /// by ADR 0021): `cases.schema.json`'s own `boundary.expansion`.
    pub expansion: Option<ExpectExpansion>,
    /// The rules the case says decided it.
    pub rules: Option<Vec<String>>,
}

/// A feasible-break answer: which of the caller's own candidates kinsoku leaves standing,
/// and which rule refused it when it does not. Every field optional and a missing one
/// asserts nothing, `ExpectBoundary`'s own convention made uniform here.
#[derive(Debug)]
#[non_exhaustive]
pub struct ExpectFeasible {
    /// The ordinal into `input.candidates` this expectation is about, defaulting to the
    /// first — `ExpectBoundary::before`'s own convention, so `ask` can select which
    /// candidate a case asks about from its first stated expectation exactly the way it
    /// already selects `before` and `edge`.
    pub candidate: usize,
    /// Whether kinsoku left this candidate standing.
    pub breakable: Option<bool>,
    /// The rules the case says decided it: a subset of the answer's own published rules,
    /// compared the way `ExpectBoundary::rules` already is.
    pub rules: Option<Vec<String>>,
}

/// A `jlreq_inline::lower` answer for one declared ruby construct: run identity against its
/// neighbors, forced boundary spacing, and, for `RubyStyle::MonoRuby` and
/// `RubyStyle::JukugoRuby` alike (§3.3.7¶1 delegates a jukugo compound's own ≤2-character
/// runs to the identical method), the resolved
/// `RubyAlignment`. Every field but `construct` is optional and a missing one asserts
/// nothing, `ExpectFeasible`'s own convention made uniform here.
#[derive(Debug)]
#[non_exhaustive]
pub struct ExpectLower {
    /// The ordinal into `input.constructs.ruby` this expectation is about, defaulting to the
    /// first — `ExpectBoundary::before`'s and `ExpectFeasible::candidate`'s own convention.
    pub construct: usize,
    /// Whether two items of the base stream share a run.
    pub same_run: Option<Vec<ExpectSameRun>>,
    /// The complete list of forced boundary spacing across every construct the case
    /// declares, not only `construct` — compared by count and by position, `ExpectBoundary::
    /// spaces`' own convention.
    pub separations: Option<Vec<ExpectLowerSeparation>>,
    /// The `RubyAlignment` resolved for `construct`: `"nakatsuki"` or `"katatsuki"`.
    pub alignment: Option<String>,
    /// Whether that alignment is §3.3.5's own discouraged combination.
    pub alignment_discouraged: Option<bool>,
    /// The rules the case says decided it: a subset of the answer's own published rules,
    /// compared the way `ExpectBoundary::rules` and `ExpectFeasible::rules` already are.
    pub rules: Option<Vec<String>>,
}

/// One same-run assertion: whether two items of the base stream share a run.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExpectSameRun {
    /// The two item ordinals compared, into the base stream.
    pub items: (usize, usize),
    /// Whether the two items share a run.
    pub same: bool,
}

/// One forced boundary spacing `lower` computed, in the case's own caller unit — never a
/// `CaseAmount` fraction, because unlike Table 1's own terms this amount is not a fraction of
/// an em JLReq states anywhere (`cases.schema.json`'s own `lower.separations` description).
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExpectLowerSeparation {
    /// The item this boundary follows.
    pub after: usize,
    /// The forced amount, in the case's own caller unit. Absent asserts only that the
    /// boundary carries a forced separation, nothing about its amount.
    pub least: Option<i64>,
}

/// A `jlreq_inline::place` answer for the case's own whole declared `Constructs`: every
/// annotation it placed — mono-ruby's own, group-ruby's own and a jukugo compound's own
/// alike — and every run it declined to place, for one of `Attachments::declined`'s own four
/// stated reasons (`crates/jlreq-inline/src/place.rs`'s own module doc states all four in
/// full). Not anchored to one ordinal the way `ExpectFeasible::candidate` and
/// `ExpectLower::construct` are —
/// `Attachments` answers the whole call, never one construct's own question, so forcing a
/// per-construct selector onto this expectation would invent one `place()` does not have
/// (`Compose::place`'s own doc states why in full).
///
/// Carries no `rules` field, on purpose: `Attachments` publishes no `rules_fired` of its own
/// (`crates/jlreq-inline/src/place.rs`'s own module doc), so a `rules` field here would
/// invite a case to assert a citation this kind never publishes. A reader who expects one and
/// does not find it should read this doc comment, not assume the omission is an oversight.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct ExpectPlace {
    /// Every annotation the case expects `place` to have placed, in `Attachments::
    /// attachments`'s own walk order. Absent asserts nothing about placement; `Some(vec![])`
    /// asserts that nothing was placed.
    pub attachments: Option<Vec<ExpectAttachment>>,
    /// The declared `constructs.ruby` ordinal of every run the case expects `place` to have
    /// declined — for one of `Attachments::declined`'s own four stated reasons alike —
    /// `Attachments::declined` translated through `ConstructRef::ordinal` — a total list
    /// compared by count and by position,
    /// `ExpectLower::separations`' own convention: a case stating one entry asserts both that
    /// it exists and that the answer declines no other. Absent asserts nothing about declines.
    pub declined: Option<Vec<usize>>,
}

/// One placed annotation character a `place` case expects — `jlreq_inline::place::
/// Attachment`, narrowed to the two facts these cases turn on (`cases.schema.json`'s own
/// `attachment` description states why `size`, `side`, `run` and `construct` are not here).
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExpectAttachment {
    /// This attachment's own inline-axis origin, in the case's own caller unit. Absent
    /// asserts only that this attachment exists, nothing about its own position.
    pub inline: Option<i64>,
    /// The annotation stream's own item ordinal this attachment draws. Absent asserts
    /// nothing about which annotation item it is.
    pub item: Option<usize>,
}

/// One conditional space: one neighbor's contribution to the space at a boundary.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExpectSpace {
    /// The amount, as JLReq states it and in units.
    pub amount: CaseAmount,
    /// Whose em the amount is a fraction of: `preceding` or `trailing`.
    pub referent: Option<String>,
    /// How the amount shrinks: `rigid`, `range` or `discrete`.
    pub reduction: Option<String>,
    /// The floor the amount shrinks to.
    pub floor: Option<(i64, i64)>,
    /// The same floor in units.
    pub floor_units: Option<i64>,
    /// Which ladder the stage belongs to. Only ever `reduction` now (ADR 0021): Appendix
    /// E's own stage lives on [`ExpectExpansion::stage`] instead.
    pub ladder: Option<String>,
    /// The stage of that ladder.
    pub stage: Option<u8>,
    /// The rule that states it.
    pub rule: Option<String>,
}

/// One boundary's own expansion opportunity — `cases.schema.json`'s own
/// `boundary_expansion`, ADR 0014's amendment (ADR 0021): a fact about the coordinate, not
/// about either neighbor's own contribution, so it lives beside [`ExpectSpace`] rather than
/// inside it.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExpectExpansion {
    /// `none`, `range` or `residual`.
    pub kind: Option<String>,
    /// The highest this boundary may be expanded to. Present only for `kind: "range"`.
    pub ceiling: Option<CaseAmount>,
    /// The priority stage this opportunity expands at. Present only for `kind: "range"`.
    pub stage: Option<u8>,
    /// The rule that states it, present for `kind: "none"` exactly as for the other two —
    /// a Table 6 row can deny an opportunity as citably as it can grant one. Compared by
    /// `jlreq-conform`'s own `check_expansion` under conditional-equality semantics rather
    /// than the plain presence check most fields here get: silent when this is `None`,
    /// passed over when this is `Some` and the answer publishes none, and a real
    /// disagreement only when both sides publish and differ (`run.rs`'s own doc on that
    /// function states the reasoning).
    pub rule: Option<String>,
}

/// One amount, stated as the fraction JLReq writes and, where the case says so, in
/// kumihan's unit and in the case's own caller units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct CaseAmount {
    /// The numerator and denominator JLReq writes.
    pub em: (i64, i64),
    /// The same amount in kumihan's 1/720 unit.
    pub units: Option<i64>,
    /// The same amount in the case's own caller units.
    pub resolved: Option<i64>,
}

/// One composed line. The three quantities are ADR 0017's.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct ExpectLine {
    /// The caller's own glyph-box origins on the line's inline axis.
    pub placements: Option<Vec<i64>>,
    /// The realized conditional space at the line end.
    pub trailing: Option<CaseAmount>,
    /// The line in normalized geometry.
    pub extent: Option<i64>,
    /// Every unit taken out of a supplied advance, with the sentence that took it.
    pub trims: Option<Vec<ExpectTrim>>,
    /// The sub-lines of every segment touching this line.
    pub parts: Option<Vec<ExpectPart>>,
    /// How far hanging punctuation extends past the measure.
    pub hanging: Option<i64>,
    /// §3.1.12 ⑤'s repair as `Search::Optimal` applied it to this line. `None` is a
    /// positive assertion that `Line::pull_up` answers `None`, not "unchecked" —
    /// `cases.schema.json`'s own `line.pull_up` description states why that reading is
    /// safe retroactively and states it once rather than here too.
    pub pull_up: Option<ExpectPullUp>,
}

/// §3.1.12 ⑤'s repair, as a case expects it: `jlreq_line::PullUp` in the case format's own
/// spelling, `cases.schema.json`'s own `pull_up` `$def`.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExpectPullUp {
    /// How much this line's own reduction reclaimed, in the caller's own unit.
    pub amount: i64,
    /// Which item moved up onto this line as a result.
    pub pulls: usize,
    /// The rule that states the repair, when the case names one.
    pub rule: Option<String>,
}

/// One unit count taken out of one item's supplied advance, with the sentence that took it.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExpectTrim {
    /// Which item it was taken out of.
    pub item: usize,
    /// The amount.
    pub amount: CaseAmount,
    /// Whose em the amount is a fraction of.
    pub referent: Option<String>,
    /// The sentence that took it.
    pub rule: Option<String>,
}

/// One sub-line of one segment.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExpectPart {
    /// Which construct it belongs to.
    pub segment: usize,
    /// Which sub-line of that construct it is.
    pub index: usize,
    /// The half-open range of items it holds.
    pub items: (usize, usize),
    /// Its inline origin relative to the line's.
    pub inline: Option<i64>,
    /// Its block origin relative to the line's.
    pub block: Option<i64>,
    /// Its own extent.
    pub extent: Option<i64>,
    /// One block offset per interior item; non-empty only for §3.2.5.
    pub across: Option<Vec<i64>>,
}

/// Read every case file under a directory.
///
/// One file per JLReq section, flat: anything else in the directory is left alone, because
/// the suite is a published directory and a README beside the cases is not a case.
pub fn load(directory: &Path) -> Result<Suite, LoadError> {
    let mut paths: Vec<PathBuf> = fs::read_dir(directory)
        .map_err(|error| LoadError::from((directory, error)))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().is_some_and(|suffix| suffix == EXTENSION))
        .collect();
    paths.sort();
    let mut files = Vec::with_capacity(paths.len());
    for path in paths {
        files.push(read_file(&path)?);
    }
    Ok(Suite { files })
}

/// Read one case file.
fn read_file(path: &Path) -> Result<CaseFile, LoadError> {
    let source = fs::read_to_string(path).map_err(|error| LoadError::from((path, error)))?;
    let value = Json::parse(&source).map_err(|error| LoadError::from((path, error)))?;
    let section = text_of(&value, "section")
        .ok_or_else(|| LoadError::file(path, "states no `section`"))?
        .to_owned();
    let entries = value
        .get("cases")
        .and_then(Json::as_array)
        .ok_or_else(|| LoadError::file(path, "states no `cases` array"))?;
    let mut cases = Vec::with_capacity(entries.len());
    for entry in entries {
        let id = text_of(entry, "id")
            .ok_or_else(|| LoadError::file(path, "a case states no `id`"))?
            .to_owned();
        cases.push(
            read_case(entry, &id).map_err(|reason| LoadError::file(path, reason).within(&id))?,
        );
    }
    Ok(CaseFile {
        section,
        path: path.to_path_buf(),
        cases,
    })
}

#[cfg(test)]
impl Case {
    /// One case read from its published JSON, for the runner's own tests.
    ///
    /// A fixture goes through the reader the committed files go through, so a test of the
    /// runner is a test of the format and not of a Rust literal that happens to resemble it.
    pub(crate) fn of(source: &str) -> Result<Self, String> {
        let value = Json::parse(source).map_err(|error| error.to_string())?;
        let id = text_of(&value, "id").unwrap_or("fixture").to_owned();
        read_case(&value, &id)
    }
}

/// Read one case.
fn read_case(value: &Json, id: &str) -> Result<Case, String> {
    let input = read_input(value.get("input").ok_or("states no `input`")?)?;
    let permitted = value
        .get("permitted")
        .and_then(Json::as_array)
        .ok_or("states no `permitted` array")?
        .iter()
        .map(read_permitted)
        .collect::<Result<Vec<_>, _>>()?;
    let forbidden = array_of(value, "forbidden")
        .iter()
        .map(read_forbidden)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Case {
        id: id.to_owned(),
        rules: strings_of(value, "rules"),
        standing: text_of(value, "standing").unwrap_or_default().to_owned(),
        quote: text_of(value, "quote").unwrap_or_default().to_owned(),
        rationale: text_of(value, "rationale").unwrap_or_default().to_owned(),
        input,
        permitted,
        forbidden,
    })
}

/// Read one `permitted` entry.
fn read_permitted(value: &Json) -> Result<Permitted, String> {
    Ok(Permitted {
        policy: read_policy(value.get("policy"))?,
        source: text_of(value, "source").unwrap_or_default().to_owned(),
        expect: read_expect(
            value
                .get("expect")
                .ok_or("a `permitted` entry states no `expect`")?,
        )?,
    })
}

/// Read one `forbidden` entry.
fn read_forbidden(value: &Json) -> Result<Forbidden, String> {
    Ok(Forbidden {
        expect: read_expect(
            value
                .get("expect")
                .ok_or("a `forbidden` entry states no `expect`")?,
        )?,
        why: text_of(value, "why").unwrap_or_default().to_owned(),
    })
}

/// Read a policy overlay: a partial map from question path to choice name.
fn read_policy(value: Option<&Json>) -> Result<CasePolicy, String> {
    let Some(value) = value else {
        return Ok(CasePolicy::new());
    };
    let members = value
        .as_object()
        .ok_or("a `policy` is an object of question path to choice name")?;
    let mut policy = CasePolicy::new();
    for (question, choice) in members {
        let choice = choice
            .as_text()
            .ok_or("a policy overlay answers a question with a choice name")?;
        policy.insert(question.clone(), choice.to_owned());
    }
    Ok(policy)
}

/// Read an expectation, in which every field is optional.
fn read_expect(value: &Json) -> Result<Expect, String> {
    let lines = match value.get("lines").and_then(Json::as_array) {
        Some(entries) => Some(
            entries
                .iter()
                .map(read_line)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        None => None,
    };
    Ok(Expect {
        class: value.get("class").map(read_class).transpose()?,
        boundary: value.get("boundary").map(read_boundary).transpose()?,
        feasible: value.get("feasible").map(read_feasible),
        lower: value.get("lower").map(read_lower).transpose()?,
        place: value.get("place").map(read_place),
        lines,
        violations: value
            .get("violations")
            .map(|entries| texts(entries.as_array().unwrap_or_default())),
    })
}

/// Read a classification expectation.
fn read_class(value: &Json) -> Result<ExpectClass, String> {
    let class = match value.get("class").map(Json::as_integer) {
        Some(Some(number)) => {
            Some(u8::try_from(number).map_err(|_| "states a class outside 1..30")?)
        },
        Some(None) => return Err("states a `class` that is not a number".to_owned()),
        None => None,
    };
    Ok(ExpectClass {
        item: ordinal_of(value, "item").unwrap_or(0),
        class,
        rules: value
            .get("rules")
            .map(|entries| texts(entries.as_array().unwrap_or_default())),
    })
}

/// Read a boundary expectation.
fn read_boundary(value: &Json) -> Result<ExpectBoundary, String> {
    let spaces = match value.get("spaces").and_then(Json::as_array) {
        Some(entries) => Some(
            entries
                .iter()
                .map(read_space)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        None => None,
    };
    Ok(ExpectBoundary {
        before: ordinal_of(value, "before").unwrap_or(0),
        edge: owned(value, "edge"),
        spaces,
        breakable: value.get("breakable").and_then(Json::as_truth),
        permitted: value.get("permitted").and_then(Json::as_truth),
        ruby_overhang: value.get("ruby_overhang").map(read_amount).transpose()?,
        expansion: value.get("expansion").map(read_expansion).transpose()?,
        rules: value
            .get("rules")
            .map(|entries| texts(entries.as_array().unwrap_or_default())),
    })
}

/// Read a feasible-break expectation, on `read_boundary`'s own "every field optional"
/// convention. Infallible — unlike `read_boundary`, nothing here reads a nested amount or
/// expansion that could itself be malformed — so this returns the value directly rather
/// than a `Result` with no `Err` arm.
fn read_feasible(value: &Json) -> ExpectFeasible {
    ExpectFeasible {
        candidate: ordinal_of(value, "candidate").unwrap_or(0),
        breakable: value.get("breakable").and_then(Json::as_truth),
        rules: value
            .get("rules")
            .map(|entries| texts(entries.as_array().unwrap_or_default())),
    }
}

/// Read a `lower` expectation, on `read_boundary`'s own "every field optional" convention.
/// `same_run` and `separations` are the two nested arrays that can themselves be malformed,
/// so this returns a `Result`, unlike `read_feasible`.
fn read_lower(value: &Json) -> Result<ExpectLower, String> {
    let same_run = match value.get("same_run").and_then(Json::as_array) {
        Some(entries) => Some(
            entries
                .iter()
                .map(read_same_run)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        None => None,
    };
    let separations = match value.get("separations").and_then(Json::as_array) {
        Some(entries) => Some(
            entries
                .iter()
                .map(read_lower_separation)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        None => None,
    };
    Ok(ExpectLower {
        construct: ordinal_of(value, "construct").unwrap_or(0),
        same_run,
        separations,
        alignment: owned(value, "alignment"),
        alignment_discouraged: value.get("alignment_discouraged").and_then(Json::as_truth),
        rules: value
            .get("rules")
            .map(|entries| texts(entries.as_array().unwrap_or_default())),
    })
}

/// Read one same-run assertion.
fn read_same_run(value: &Json) -> Result<ExpectSameRun, String> {
    Ok(ExpectSameRun {
        items: value
            .get("items")
            .and_then(read_range)
            .ok_or("a `same_run` entry states no `[i, j]` item pair")?,
        same: value
            .get("same")
            .and_then(Json::as_truth)
            .ok_or("a `same_run` entry states no `same`")?,
    })
}

/// Read one forced boundary spacing `lower` computed.
fn read_lower_separation(value: &Json) -> Result<ExpectLowerSeparation, String> {
    Ok(ExpectLowerSeparation {
        after: ordinal_of(value, "after").ok_or("a `separations` entry states no `after`")?,
        least: value.get("least").and_then(Json::as_integer),
    })
}

/// Read a `place` expectation, on `read_boundary`'s own "every field optional" convention.
/// Infallible — like `read_feasible`, and unlike `read_lower` — because neither of `place`'s
/// own two nested arrays holds a field that can itself be malformed: an attachment reads two
/// plain optional scalars and `declined` reads a bare integer array, both fallible only in
/// the "not present" sense every other optional field here already reads permissively.
fn read_place(value: &Json) -> ExpectPlace {
    let attachments = value
        .get("attachments")
        .and_then(Json::as_array)
        .map(|entries| entries.iter().map(read_attachment).collect());
    let declined = value
        .get("declined")
        .and_then(Json::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| usize::try_from(entry.as_integer()?).ok())
                .collect()
        });
    ExpectPlace {
        attachments,
        declined,
    }
}

/// Read one placed-annotation expectation.
fn read_attachment(value: &Json) -> ExpectAttachment {
    ExpectAttachment {
        inline: value.get("inline").and_then(Json::as_integer),
        item: ordinal_of(value, "item"),
    }
}

/// Read one boundary-level expansion opportunity.
fn read_expansion(value: &Json) -> Result<ExpectExpansion, String> {
    Ok(ExpectExpansion {
        kind: owned(value, "kind"),
        ceiling: value.get("ceiling").map(read_amount).transpose()?,
        stage: value
            .get("stage")
            .and_then(Json::as_integer)
            .map(|stage| u8::try_from(stage).unwrap_or(u8::MAX)),
        rule: owned(value, "rule"),
    })
}

/// Read one conditional space.
fn read_space(value: &Json) -> Result<ExpectSpace, String> {
    Ok(ExpectSpace {
        amount: read_amount(value)?,
        referent: owned(value, "referent"),
        reduction: owned(value, "reduction"),
        floor: value.get("floor").map(read_fraction).transpose()?,
        floor_units: value.get("floor_units").and_then(Json::as_integer),
        ladder: owned(value, "ladder"),
        stage: value
            .get("stage")
            .and_then(Json::as_integer)
            .map(|stage| u8::try_from(stage).unwrap_or(u8::MAX)),
        rule: owned(value, "rule"),
    })
}

/// Read one line expectation.
fn read_line(value: &Json) -> Result<ExpectLine, String> {
    let trims = match value.get("trims").and_then(Json::as_array) {
        Some(entries) => Some(
            entries
                .iter()
                .map(read_trim)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        None => None,
    };
    let parts = match value.get("parts").and_then(Json::as_array) {
        Some(entries) => Some(
            entries
                .iter()
                .map(read_part)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        None => None,
    };
    Ok(ExpectLine {
        placements: value.get("placements").map(integers),
        trailing: value.get("trailing").map(read_amount).transpose()?,
        extent: value.get("extent").and_then(Json::as_integer),
        trims,
        parts,
        hanging: value.get("hanging").and_then(Json::as_integer),
        pull_up: value.get("pull_up").map(read_pull_up).transpose()?,
    })
}

/// Read one `pull_up` expectation.
fn read_pull_up(value: &Json) -> Result<ExpectPullUp, String> {
    Ok(ExpectPullUp {
        amount: value
            .get("amount")
            .and_then(Json::as_integer)
            .ok_or("a `pull_up` states no `amount`")?,
        pulls: ordinal_of(value, "pulls").ok_or("a `pull_up` states no `pulls`")?,
        rule: owned(value, "rule"),
    })
}

/// Read one trim.
fn read_trim(value: &Json) -> Result<ExpectTrim, String> {
    Ok(ExpectTrim {
        item: ordinal_of(value, "item").unwrap_or(0),
        amount: read_amount(value)?,
        referent: owned(value, "referent"),
        rule: owned(value, "rule"),
    })
}

/// Read one sub-line of one segment.
fn read_part(value: &Json) -> Result<ExpectPart, String> {
    Ok(ExpectPart {
        segment: ordinal_of(value, "segment").unwrap_or(0),
        index: ordinal_of(value, "index").unwrap_or(0),
        items: value
            .get("items")
            .and_then(read_range)
            .ok_or("a `part` states no `items` range")?,
        inline: value.get("inline").and_then(Json::as_integer),
        block: value.get("block").and_then(Json::as_integer),
        extent: value.get("extent").and_then(Json::as_integer),
        across: value.get("across").map(integers),
    })
}

/// Read an amount: the fraction, and the two restatements of it a case may carry.
fn read_amount(value: &Json) -> Result<CaseAmount, String> {
    Ok(CaseAmount {
        em: value
            .get("em")
            .map(read_fraction)
            .transpose()?
            .ok_or("an amount states no `em` fraction")?,
        units: value.get("units").and_then(Json::as_integer),
        resolved: value.get("resolved").and_then(Json::as_integer),
    })
}

/// Read a fraction: a numerator and a denominator.
fn read_fraction(value: &Json) -> Result<(i64, i64), String> {
    let entries = value
        .as_array()
        .ok_or("a fraction is a numerator and a denominator")?;
    match (
        entries.first().and_then(Json::as_integer),
        entries.get(1).and_then(Json::as_integer),
    ) {
        (Some(numerator), Some(denominator)) => Ok((numerator, denominator)),
        _ => Err("a fraction is a numerator and a denominator".to_owned()),
    }
}

/// The eight questions a case may ask, which is the schema's own `kind` enum.
///
/// Read here and refused here rather than defaulted by the runner. A `kind` the reader did
/// not recognize used to reach the composition arm, so a case naming a ninth question — or
/// misspelling one of the eight — was quietly asked a different one, and the schema's enum
/// could not express that fallback for anyone reading the published contract.
const KINDS: [&str; 8] = [
    "classify", "boundary", "compose", "align", "tab", "feasible", "lower", "place",
];

/// Read one input object.
fn read_input(value: &Json) -> Result<CaseInput, String> {
    let annotations = array_of(value, "annotations")
        .iter()
        .map(read_stream)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(CaseInput {
        kind: text_of(value, "kind")
            .filter(|kind| KINDS.contains(kind))
            .ok_or(
                "an `input` states no `kind` this format has; it is one of `classify`, \
                    `boundary`, `compose`, `align`, `tab`, `feasible`, `lower` and `place`",
            )?
            .to_owned(),
        text: text_of(value, "text")
            .ok_or("an `input` states no `text`")?
            .to_owned(),
        scales: array_of(value, "scales")
            .iter()
            .map(read_scale)
            .collect::<Result<Vec<_>, _>>()?,
        items: array_of(value, "items")
            .iter()
            .map(read_item)
            .collect::<Result<Vec<_>, _>>()?,
        annotations,
        constructs: read_constructs(value.get("constructs")),
        candidates: array_of(value, "candidates")
            .iter()
            .filter_map(|entry| ordinal_of(entry, "at"))
            .collect(),
        measure: value.get("measure").and_then(Json::as_integer),
        search: value.get("search").map(read_search).transpose()?,
        direction: owned(value, "direction"),
        first_line_indent: value.get("first_line_indent").and_then(Json::as_integer),
        head_indent: value.get("head_indent").and_then(Json::as_integer),
        end_indent: value.get("end_indent").and_then(Json::as_integer),
        widow_threshold: value.get("widow_threshold").and_then(Json::as_integer),
        alignment: owned(value, "alignment"),
        tab_starts: array_of(value, "tab_starts")
            .iter()
            .filter_map(ordinal_of_self)
            .collect(),
        tab_stops: array_of(value, "tab_stops")
            .iter()
            .map(read_tab_stop)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

/// One bare integer of an array, as an ordinal — `tab_starts`' own shape, unlike
/// `candidates`' array of `{ "at": N }` objects, is already the bare integer.
fn ordinal_of_self(value: &Json) -> Option<usize> {
    usize::try_from(value.as_integer()?).ok()
}

/// The two search kinds a `compose` case may name, which is the schema's own `search.kind`
/// enum.
const SEARCH_KINDS: [&str; 2] = ["first-fit", "optimal"];

/// Read one `search` declaration.
///
/// Refuses a `kind` this format does not have, on the identical standard `read_input`'s own
/// `kind` reader holds the eight questions to (`cases.schema.json`'s own `search`
/// description): a case whose `search.kind` this reader did not recognize must not silently
/// fall through to either variant. `kind: "optimal"` additionally requires `tolerance`,
/// which JSON Schema's own `required` cannot state conditioned on a sibling field without a
/// second `if`/`then` branch this format does not otherwise use, so it is enforced here
/// instead, on the same "malformed case" standard `Kumihan::compose` already holds an
/// absent `measure` to.
fn read_search(value: &Json) -> Result<CaseSearch, String> {
    let kind = text_of(value, "kind")
        .filter(|kind| SEARCH_KINDS.contains(kind))
        .ok_or(
            "a `search` states no `kind` this format has; it is one of `first-fit` and `optimal`",
        )?
        .to_owned();
    let tolerance = value.get("tolerance").and_then(Json::as_integer);
    if kind == "optimal" && tolerance.is_none() {
        return Err("a `search` naming `kind: \"optimal\"` states no `tolerance`".to_owned());
    }
    Ok(CaseSearch { kind, tolerance })
}

/// Read one declared tab stop.
fn read_tab_stop(value: &Json) -> Result<CaseTabStop, String> {
    Ok(CaseTabStop {
        position: value
            .get("position")
            .and_then(Json::as_integer)
            .ok_or("a `tab_stop` states no `position`")?,
        kind: text_of(value, "kind")
            .ok_or("a `tab_stop` states no `kind`")?
            .to_owned(),
        at: ordinal_of(value, "at"),
    })
}

/// Read one annotation stream.
fn read_stream(value: &Json) -> Result<CaseStream, String> {
    Ok(CaseStream {
        text: text_of(value, "text")
            .ok_or("an annotation stream states no `text`")?
            .to_owned(),
        scales: array_of(value, "scales")
            .iter()
            .map(read_scale)
            .collect::<Result<Vec<_>, _>>()?,
        items: array_of(value, "items")
            .iter()
            .map(read_item)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

/// Read one declared character size.
fn read_scale(value: &Json) -> Result<CaseScale, String> {
    match (
        value.get("inline_em").and_then(Json::as_integer),
        value.get("block_em").and_then(Json::as_integer),
    ) {
        (Some(inline_em), Some(block_em)) => Ok(CaseScale {
            inline_em,
            block_em,
        }),
        _ => Err("a `scale` states an `inline_em` and a `block_em`".to_owned()),
    }
}

/// Read one item.
fn read_item(value: &Json) -> Result<CaseItem, String> {
    Ok(CaseItem {
        start: ordinal_of(value, "start").ok_or("an `item` states no `start`")?,
        advance: value
            .get("advance")
            .and_then(Json::as_integer)
            .ok_or("an `item` states no `advance`")?,
        scale: ordinal_of(value, "scale").ok_or("an `item` states no `scale`")?,
        frame: owned(value, "frame"),
        role: owned(value, "role"),
    })
}

/// Read the constructs declared over the streams.
///
/// Only `ruby` and `emphasis` have a pinned interior in the committed schema, so what is
/// read is the part every kind states: the base-stream ranges it runs over. `base`, `items`
/// and `mark` are all such ranges; `annotation` names a stream and is not one.
fn read_constructs(value: Option<&Json>) -> Vec<CaseConstruct> {
    let Some(members) = value.and_then(Json::as_object) else {
        return Vec::new();
    };
    let mut constructs = Vec::new();
    for (kind, entries) in members {
        for (index, entry) in entries.as_array().unwrap_or_default().iter().enumerate() {
            constructs.push(CaseConstruct {
                kind: kind.clone(),
                index,
                ranges: ["base", "items", "mark"]
                    .into_iter()
                    .filter_map(|name| entry.get(name).and_then(read_range))
                    .collect(),
                style: owned(entry, "style"),
                annotation: ordinal_of(entry, "annotation"),
                runs: entry
                    .get("runs")
                    .and_then(Json::as_array)
                    .unwrap_or_default()
                    .iter()
                    .filter_map(read_case_run)
                    .collect(),
            });
        }
    }
    constructs
}

/// Read one `ruby` construct's own `runs` entry: a base range and the annotation range that
/// reads it.
fn read_case_run(value: &Json) -> Option<CaseRun> {
    Some(CaseRun {
        base: value.get("base").and_then(read_range)?,
        annotation: value.get("annotation").and_then(read_range)?,
    })
}

/// Read a half-open range of ordinals.
fn read_range(value: &Json) -> Option<(usize, usize)> {
    let entries = value.as_array()?;
    let first = usize::try_from(entries.first()?.as_integer()?).ok()?;
    let past = usize::try_from(entries.get(1)?.as_integer()?).ok()?;
    Some((first, past))
}

/// The string under a name, when the value has one.
fn text_of<'a>(value: &'a Json, name: &str) -> Option<&'a str> {
    value.get(name).and_then(Json::as_text)
}

/// The same, owned.
fn owned(value: &Json, name: &str) -> Option<String> {
    text_of(value, name).map(str::to_owned)
}

/// A non-negative integer under a name, as an ordinal.
fn ordinal_of(value: &Json, name: &str) -> Option<usize> {
    usize::try_from(value.get(name)?.as_integer()?).ok()
}

/// The array under a name, or an empty one.
fn array_of<'a>(value: &'a Json, name: &str) -> &'a [Json] {
    value.get(name).and_then(Json::as_array).unwrap_or_default()
}

/// The strings of the array under a name.
fn strings_of(value: &Json, name: &str) -> Vec<String> {
    texts(array_of(value, name))
}

/// The strings of an array, skipping anything that is not one.
fn texts(entries: &[Json]) -> Vec<String> {
    entries
        .iter()
        .filter_map(Json::as_text)
        .map(str::to_owned)
        .collect()
}

/// The integers of an array, skipping anything that is not one.
fn integers(value: &Json) -> Vec<i64> {
    value
        .as_array()
        .unwrap_or_default()
        .iter()
        .filter_map(Json::as_integer)
        .collect()
}
