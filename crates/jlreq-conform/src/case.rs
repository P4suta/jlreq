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

    /// What the case supplies, which is what the three trait methods are asked about.
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
    /// Which of the three questions this case asks.
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
    /// The writing direction, when the case is specific to one.
    pub direction: Option<String>,
    /// The paragraph's first-line indent, in the caller's own unit.
    pub first_line_indent: Option<i64>,
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
/// over, which the format spells `base`, `items` or `mark`.
#[derive(Debug)]
#[non_exhaustive]
pub struct CaseConstruct {
    /// `ruby`, `warichu`, `tate_chu_yoko`, and the rest.
    pub kind: String,
    /// Its position in its own array, which is how a report names it (ADR 0015).
    pub index: usize,
    /// The half-open ranges of base-stream ordinals it runs over.
    pub ranges: Vec<(usize, usize)>,
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
    /// The boundary asked about: the ordinal of the item it precedes.
    pub before: usize,
    /// One conditional space per neighbor that contributes one.
    pub spaces: Option<Vec<ExpectSpace>>,
    /// Whether a line may break here.
    pub breakable: Option<bool>,
    /// Whether the adjacency is permitted at all.
    pub permitted: Option<bool>,
    /// How far ruby may overhang here.
    pub ruby_overhang: Option<CaseAmount>,
    /// The rules the case says decided it.
    pub rules: Option<Vec<String>>,
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
    /// Which ladder the stage belongs to: `reduction` or `expansion`.
    pub ladder: Option<String>,
    /// The stage of that ladder.
    pub stage: Option<u8>,
    /// The rule that states it.
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
        spaces,
        breakable: value.get("breakable").and_then(Json::as_truth),
        permitted: value.get("permitted").and_then(Json::as_truth),
        ruby_overhang: value.get("ruby_overhang").map(read_amount).transpose()?,
        rules: value
            .get("rules")
            .map(|entries| texts(entries.as_array().unwrap_or_default())),
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

/// The three questions a case may ask, which is the schema's own `kind` enum.
///
/// Read here and refused here rather than defaulted by the runner. A `kind` the reader did
/// not recognize used to reach the composition arm, so a case naming a fourth question — or
/// misspelling one of the three — was quietly asked a different one, and the schema's enum
/// could not express that fallback for anyone reading the published contract.
const KINDS: [&str; 3] = ["classify", "boundary", "compose"];

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
                    `boundary` and `compose`",
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
        direction: owned(value, "direction"),
        first_line_indent: value.get("first_line_indent").and_then(Json::as_integer),
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
            });
        }
    }
    constructs
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
