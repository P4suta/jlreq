// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The trait an implementation supplies, and the runner that measures it.
//!
//! Three methods, each taking data and returning data, and each returning `Option` so that
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
    Case, CaseAmount, CaseFile, CasePolicy, Expect, ExpectBoundary, ExpectClass, ExpectLine,
    Permitted, Suite,
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

    /// The spacing, breakability and placement at one boundary. JLReq: §B, §C
    fn boundary(&self, input: &crate::case::CaseInput, before: usize) -> Option<CaseBoundary>;

    /// The composed lines. JLReq: §3.8, §D, §E
    fn compose(&self, input: &crate::case::CaseInput) -> Option<CaseOutput>;
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
    /// The rules that decided it.
    pub rules: Vec<String>,
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
    /// Which ladder the stage belongs to: `reduction` or `expansion`.
    pub ladder: String,
    /// The stage of that ladder.
    pub stage: u8,
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
        rules: Vec<String>,
    ) -> Self {
        Self {
            spaces,
            breakable,
            permitted,
            ruby_overhang,
            rules,
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
    ) -> Self {
        Self {
            placements,
            trailing,
            extent,
            trims,
            parts,
            hanging,
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

/// One answer, whichever of the three questions the case asked.
#[derive(Debug)]
enum Answer {
    /// A classification answer about one item.
    Class(usize, CaseClass),
    /// A boundary answer about one boundary.
    Boundary(usize, CaseBoundary),
    /// The composed lines.
    Composed(CaseOutput),
}

impl Answer {
    /// The rules the implementation says it fired.
    fn rules(&self) -> Vec<String> {
        match self {
            Self::Class(_, answer) => answer.rules.clone(),
            Self::Boundary(_, answer) => answer.rules.clone(),
            Self::Composed(answer) => answer.rules.clone(),
        }
    }

    /// Which of the three questions produced it, as a report writes it.
    fn question(&self) -> String {
        match self {
            Self::Class(item, _) => format!("the class of item {item}"),
            Self::Boundary(before, _) => format!("the boundary before item {before}"),
            Self::Composed(_) => "the composed lines".to_owned(),
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
///
/// A `kind` this format does not have never reaches here: the reader refuses the case, so
/// the composition arm below is what `compose` names and not what everything else falls
/// into.
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
            let before = case
                .permitted()
                .iter()
                .find_map(|entry| entry.expect.boundary.as_ref())
                .map_or(0, |boundary| boundary.before);
            implementation
                .boundary(case.input(), before)
                .map(|answer| Answer::Boundary(before, answer))
        },
        _ => implementation.compose(case.input()).map(Answer::Composed),
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
    found
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
    found
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
    match &expect.lines {
        Some(lines) => format!("{count} line(s)", count = lines.len()),
        None => "nothing".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Answer, CaseBoundary, CaseClass, CaseLine, CaseOutput, CasePart, CaseTrim, Compose, Report,
        check, run_file,
    };
    use crate::case::{Case, CaseFile, CaseInput, CasePolicy, Expect};

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
        /// What `compose` answers.
        composed: Option<CaseOutput>,
        /// What `declared_policy` answers.
        policy: Option<CasePolicy>,
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

        fn boundary(&self, _input: &CaseInput, _before: usize) -> Option<CaseBoundary> {
            self.boundary.clone()
        }

        fn compose(&self, _input: &CaseInput) -> Option<CaseOutput> {
            self.composed.clone()
        }
    }

    /// A one-item classification input, as the published format writes one.
    const INPUT: &str = r#""input": {
        "kind": "classify",
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
            vec![CaseLine::new(vec![0], 0, 1000, trims, parts, None)],
            Vec::new(),
            Vec::new(),
        )
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
                boundary: Some(CaseBoundary::new(Vec::new(), true, true, None, Vec::new())),
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
    fn an_expectation_that_states_nothing_about_the_question_differs_from_the_answer() {
        // Both sides of a case read it that way, which is what makes a `forbidden` entry
        // about another question exclude nothing and a `permitted` one satisfy nothing. The
        // fully silent expectation never reaches here on the `forbidden` side, because
        // `Expect::is_silent` decides that before `check` is consulted.
        let expect = Expect {
            class: None,
            boundary: None,
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
}
