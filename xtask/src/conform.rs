// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `conform` gate.
//!
//! The conformance suite is the deliverable [ADR
//! 0006](../../docs/adr/0006-conformance-suite-as-artifact.md) treats as a published
//! artifact. `conform --check` validates the protocol-v1 schema and every case in
//! `crates/kumihan-conformance/suite.ndjson`, then subtracts their declared coverage from
//! `spec/derived/rules.tsv`.
//!
//! Each case is checked as a language-independent black-box request and expected response:
//! protocol and specification identifiers are pinned, inputs use pre-shaped clusters and
//! UTF-8 byte ranges, and outputs expose only lines, placements and diagnostics. The CLI's
//! integration tests provide the dynamic half by running those requests through an external
//! engine process.
//!
//! `docs/conformance-deferrals.toml` classifies the subtraction's remainder. A machine-
//! observable rule must be covered; only evidence-backed `editorial` and `non-observable`
//! rows may remain, and a stale classification fails once a case covers it. The old
//! Rust-only corpus reader remains below solely for historical fixtures; the absence of its
//! deleted directory is the expected 1.0 state and never weakens protocol-v1 coverage.
//!
//! See `docs/design/conformance.md`, `docs/adr/0006` and `docs/adr/0013`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::deferral::{self, LEDGER, Ledger, Milestones};
use crate::shared::{self, Gate};

/// The `conform` gate, as the dispatcher sees it.
pub(crate) const GATE: Gate = Gate {
    name: "conform",
    purpose: concat!(
        "every conformance case holds the published format, case ids are unique, and ",
        "every inventoried rule has a case, is deferred to a named milestone, or carries ",
        "an evidence-backed editorial/non-observable classification — the census says ",
        "which inputs were readable and how much implementation debt remains"
    ),
    reference: "docs/design/conformance.md",
    run,
};

/// The published suite, relative to the workspace root.
///
/// `pub(crate)` so `xtask::attest`'s own `conformance-cases-agree-with-the-cells` checker
/// reads the identical path rather than a second copy of the string that could drift from
/// this one.
pub(crate) const CASES_DIR: &str = "crates/jlreq-conform/cases";
/// The language-independent black-box suite for the unified product.
pub(crate) const PROTOCOL_CASES_FILE: &str = "crates/kumihan-conformance/suite.ndjson";
const PROTOCOL_SCHEMA_FILE: &str = "crates/kumihan-conformance/protocol.schema.json";
const PROTOCOL: &str = "kumihan.conformance/1";
const SPECIFICATION: &str = "jlreq-2020-08-11+unicode-17.0.0";
/// The committed schema, published so nobody else has to use our reader.
const SCHEMA_FILE: &str = "crates/jlreq-conform/cases.schema.json";
/// The rule inventory `RuleId::ALL` is generated from.
const RULES_INVENTORY: &str = "spec/derived/rules.tsv";
/// The policy space the overlay keys of a case are generated alongside.
const QUESTIONS_INVENTORY: &str = "spec/derived/questions.tsv";
/// Every section the published rendering numbers, which is what an address inside an
/// expectation resolves against where the rule inventory does not carry it: Appendix A is
/// held as data rather than as rules, so §A.28 is a section a case may cite as provenance
/// and never a rule a case may cover.
const ANCHORS_INVENTORY: &str = "spec/derived/anchors.tsv";
/// The crate that declares the fixed-point denominator.
const UNIT_CRATE: &str = "crates/kumihan/src";

/// Validate the conformance suite and report one message per malformed thing.
fn run(arguments: &[String]) -> io::Result<Vec<String>> {
    accept_arguments(arguments)?;
    let suite = Suite::read(&shared::workspace_root()?)?;
    let (census, violations) = suite.examine();
    for line in &census {
        println!("{name}: {line}", name = GATE.name);
    }
    Ok(violations)
}

/// Accept the spelling `docs/design/conformance.md` uses, and refuse anything else.
///
/// Validating the suite is the only thing this subcommand does — the runner and the judge
/// are `jlreq-conform`'s own binary — so the bare form does the same work. An unrecognized
/// option is refused rather than ignored, so no caller believes a switch took effect.
fn accept_arguments(arguments: &[String]) -> io::Result<()> {
    if arguments.is_empty() || matches!(arguments, [only] if only == "--check") {
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        format!(
            "conform takes `--check` or no argument; got `{given}`",
            given = arguments.join(" ")
        ),
    ))
}

/// Everything the gate reads before it checks anything.
#[derive(Debug)]
struct Suite {
    /// Whether the cases directory exists at all. The empty set is valid; its absence is
    /// reported rather than silently treated as an empty directory.
    directory_exists: bool,
    /// Every case file, as its path relative to the workspace root and its contents.
    files: Vec<(String, String)>,
    /// The protocol-v1 NDJSON suite, when present.
    protocol_cases: Option<String>,
    /// The committed JSON Schema for the language-independent protocol.
    protocol_schema: Option<String>,
    /// The committed JSON Schema, once it is written.
    schema: Option<String>,
    /// Every address in the rule inventory, once it is generated.
    rules: Option<BTreeSet<String>>,
    /// Every question path in the policy space, once it is generated.
    questions: Option<BTreeMap<String, BTreeSet<String>>>,
    /// Every numbered section of the rendering, once it is derived.
    anchors: Option<BTreeSet<String>>,
    /// Units per ideographic em, read from the crate that declares it.
    units_per_em: Option<i64>,
    /// The rules a later milestone covers, and which milestone that is.
    deferrals: Ledger,
    /// The milestones `ROADMAP.md` declares, which a deferral names one of.
    milestones: Milestones,
}

impl Suite {
    /// Read the suite and the two inventories it is checked against.
    fn read(root: &Path) -> io::Result<Self> {
        let directory = root.join(CASES_DIR);
        let mut files = Vec::new();
        for path in case_files(&directory)? {
            let name = shared::relative_name(&path, root).replace('\\', "/");
            files.push((name, fs::read_to_string(&path)?));
        }
        Ok(Self {
            directory_exists: directory.is_dir(),
            files,
            protocol_cases: read_if_present(&root.join(PROTOCOL_CASES_FILE))?,
            protocol_schema: read_if_present(&root.join(PROTOCOL_SCHEMA_FILE))?,
            schema: read_if_present(&root.join(SCHEMA_FILE))?,
            rules: read_inventory(&root.join(RULES_INVENTORY), "address")?,
            questions: read_questions(&root.join(QUESTIONS_INVENTORY))?,
            anchors: read_inventory(&root.join(ANCHORS_INVENTORY), "address")?,
            units_per_em: units_per_em(root)?,
            deferrals: Ledger::read(root)?,
            milestones: Milestones::read(root)?,
        })
    }

    /// What the per-case checks need from outside the case.
    fn reference(&self) -> Reference<'_> {
        Reference {
            units_per_em: self.units_per_em,
            questions: self.questions.as_ref(),
        }
    }

    /// Run every check, and say what was examined either way.
    fn examine(&self) -> (Vec<String>, Vec<String>) {
        let mut violations = Vec::new();
        let mut cases = Vec::new();
        for (name, source) in &self.files {
            let (found, read) = examine_file(name, source, self.reference());
            violations.extend(found);
            cases.extend(read);
        }
        if let Some(source) = &self.protocol_cases {
            let (found, read) = examine_protocol_suite(PROTOCOL_CASES_FILE, source);
            violations.extend(found);
            cases.extend(read);
        }
        violations.extend(unique_ids(&cases));
        violations.extend(frame_pairs(&cases));
        violations.extend(self.coverage(&cases));
        violations.extend(self.protocol_coverage(&cases));
        violations.extend(check_protocol_schema(
            self.protocol_schema.as_deref(),
            self.protocol_cases.is_some(),
        ));
        violations.extend(self.deferrals.examine(deferral::Reference {
            inventory: self.rules.as_ref(),
            covered: Some(&self.covered(&cases)),
            milestones: &self.milestones,
        }));
        violations.extend(check_schema(self.schema.as_deref(), !self.files.is_empty()));
        (self.census(&cases), violations)
    }

    /// Every rule the suite covers today: the addresses the cases name, and the addresses
    /// the `covers` families credit.
    ///
    /// Without the inventory a family credits nothing, because a pattern names rules by
    /// being matched against them and there is nothing to match. That is the same reading
    /// `coverage` takes, and it is what keeps a deferral from being called stale by a
    /// pattern nobody could resolve.
    fn covered<'a>(&'a self, cases: &'a [Case]) -> BTreeSet<&'a str> {
        match self.rules.as_ref() {
            Some(inventory) => declared_addresses(cases, inventory),
            None => cases.iter().flat_map(Case::rules).collect(),
        }
    }

    /// Whether the published suite exists at all.
    ///
    /// Not "is empty": a directory holding no case is a suite someone has started, and the
    /// subtraction below runs over it in full. This is the state before that — no directory
    /// and no case file anywhere — in which the suite is the operand that is *absent* rather
    /// than empty, exactly as the inventory is absent before `xtask derive` writes it.
    fn suite_is_unstarted(&self, cases: &[Case]) -> bool {
        !self.directory_exists && cases.is_empty()
    }

    /// Subtract the cases from the inventory, in both directions (ADR 0013).
    ///
    /// The subtraction needs both operands, and until this milestone only one of them could
    /// go missing: the inventory. Now that `spec/derived/rules.tsv` is generated, the other
    /// side can be the absent one, and it is treated the same way — the half that needs it
    /// does not run, the census says which half and over how many rules, and no sentence
    /// anywhere claims it held. A gate that reported "every rule has a case" by subtracting
    /// from a suite that does not exist would be the false pass `xtask/src/main.rs` says no
    /// gate may give; a gate that fails because a later milestone has not happened yet
    /// reports the schedule rather than the invariant. Creating the directory turns the
    /// check on, whether or not a case is in it.
    ///
    /// What the subtraction leaves is *uncovered*, which is not the same thing as *not
    /// written yet*. `docs/conformance-deferrals.toml` is the difference: a rule named there
    /// is deferred to a milestone, is counted in the census, and is subtracted here, and a
    /// rule that is neither covered nor deferred fails. That is the whole reason this gate
    /// can hold "every rule gets a conformance case" from the first case file rather than
    /// from the last milestone (`crate::deferral`).
    fn coverage(&self, cases: &[Case]) -> Vec<String> {
        let Some(inventory) = self.rules.as_ref() else {
            if cases.is_empty() {
                return Vec::new();
            }
            return vec![format!(
                "{count} case(s) declare rules but {RULES_INVENTORY} has not been generated, \
                 so neither their addresses nor the coverage they close can be checked",
                count = cases.len()
            )];
        };
        if self.suite_is_unstarted(cases) {
            return Vec::new();
        }
        let mut found = unresolved_addresses(cases, inventory, self.anchors.as_ref());
        let declared = declared_addresses(cases, inventory);
        let accounted = self.deferrals.accounted();
        let uncovered: Vec<&str> = inventory
            .iter()
            .filter(|rule| !declared.contains(rule.as_str()) && !accounted.contains(rule.as_str()))
            .map(String::as_str)
            .collect();
        if !uncovered.is_empty() {
            found.push(format!(
                "{count} inventoried rule(s) have neither a conformance case, a deferral, \
                 nor an evidence-bearing classification, \
                 the first being {sample}; CONTRIBUTING.md makes a rule without a case \
                 incomplete, and a rule a later milestone covers is declared in {LEDGER} \
                 rather than left out (ADR 0013)",
                count = uncovered.len(),
                sample = sample_of(&uncovered)
            ));
        }
        found
    }

    /// Require the language-independent suite to cover every observable inventoried rule.
    fn protocol_coverage(&self, cases: &[Case]) -> Vec<String> {
        if self.protocol_cases.is_none() {
            return Vec::new();
        }
        let Some(inventory) = self.rules.as_ref() else {
            return Vec::new();
        };
        let declared: BTreeSet<&str> = cases
            .iter()
            .filter(|case| case.body.get("protocol").is_some())
            .flat_map(Case::rules)
            .filter(|rule| inventory.contains(*rule))
            .collect();
        let classified = self.deferrals.classified();
        let missing: Vec<&str> = inventory
            .iter()
            .map(String::as_str)
            .filter(|rule| !classified.contains(rule) && !declared.contains(rule))
            .collect();
        if missing.is_empty() {
            Vec::new()
        } else {
            vec![format!(
                "{count} observable inventoried rule(s) lack a protocol-v1 black-box case, \
                 the first being {sample}; an external process must be able to run the full \
                 suite without the retained Rust-only corpus",
                count = missing.len(),
                sample = sample_of(&missing),
            )]
        }
    }

    /// What was examined, stated whether or not anything was found.
    fn census(&self, cases: &[Case]) -> Vec<String> {
        let mut lines = vec![
            if self.directory_exists {
                format!(
                    "read {count} case file(s) under {CASES_DIR}",
                    count = self.files.len()
                )
            } else {
                "the retired Rust-only differential corpus is absent; protocol-v1 is the authoritative suite".to_owned()
            },
            match &self.protocol_cases {
                Some(source) => format!(
                    "read {count} protocol-v1 case(s) from {PROTOCOL_CASES_FILE}",
                    count = source
                        .lines()
                        .filter(|line| !line.trim().is_empty())
                        .count()
                ),
                None => format!("{PROTOCOL_CASES_FILE} does not exist yet"),
            },
            kind_census(cases),
            format!(
                "{declaring} case(s) declare {total} matrix coordinate(s) under `cells`, \
                 checked here for shape and by `xtask attest`'s own \
                 `conformance-cases-agree-with-the-cells` invariant against the transcription",
                declaring = cases
                    .iter()
                    .filter(|case| case.declared_cells() > 0)
                    .count(),
                total = cases.iter().map(Case::declared_cells).sum::<usize>()
            ),
            optimal_search_census(cases),
            match self.rules.as_ref() {
                Some(rules) => format!(
                    "{RULES_INVENTORY} inventories {count} rule(s)",
                    count = rules.len()
                ),
                None => format!(
                    "{RULES_INVENTORY} has not been generated, so declared coverage has \
                     nothing to close over yet"
                ),
            },
            match self.questions.as_ref() {
                Some(questions) => format!(
                    "{QUESTIONS_INVENTORY} inventories {count} policy question(s)",
                    count = questions.len()
                ),
                None => format!(
                    "{QUESTIONS_INVENTORY} has not been generated, so overlay keys were \
                     checked for shape and not for existence"
                ),
            },
            match (self.protocol_schema.is_some(), self.units_per_em) {
                (true, Some(per_em)) => {
                    format!("{PROTOCOL_SCHEMA_FILE} is committed; one em is {per_em} units")
                },
                (true, None) => {
                    format!("{PROTOCOL_SCHEMA_FILE} is committed; no em denominator found")
                },
                (false, Some(per_em)) => {
                    format!("{PROTOCOL_SCHEMA_FILE} is missing; one em is {per_em} units")
                },
                (false, None) => {
                    format!("{PROTOCOL_SCHEMA_FILE} is missing and no em denominator was found")
                },
            },
        ];
        lines.push(self.deferrals.census(self.rules.as_ref()));
        if self.suite_is_unstarted(cases) {
            lines.push(match self.rules.as_ref() {
                Some(rules) => format!(
                    "declared coverage: did not run, {CASES_DIR} does not exist, so the \
                     {count} inventoried rule(s) were subtracted from nothing rather than \
                     found covered (ADR 0013)",
                    count = rules.len()
                ),
                None => format!(
                    "declared coverage: did not run, neither {CASES_DIR} nor \
                     {RULES_INVENTORY} exists yet (ADR 0013)"
                ),
            });
            return lines;
        }
        if let Some(inventory) = self.rules.as_ref() {
            let (with_case, deferred, classified, neither) = split(
                inventory,
                &self.covered(cases),
                &self.deferrals.rules(),
                &self.deferrals.classified(),
            );
            lines.push(format!(
                "declared coverage: {with_case} of {total} inventoried rule(s) have a case, \
                 {deferred} are deferred to a later milestone, {classified} are classified \
                 editorial/non-observable, {neither} are none of these",
                total = inventory.len()
            ));
        }
        lines
    }
}

/// Require the shipped protocol schema whenever the protocol suite is present.
fn check_protocol_schema(schema: Option<&str>, required: bool) -> Vec<String> {
    let Some(source) = schema else {
        return required
            .then(|| format!("{PROTOCOL_SCHEMA_FILE} is missing beside the protocol suite"))
            .into_iter()
            .collect();
    };
    if let Err(error) = Json::parse(source) {
        return vec![format!("{PROTOCOL_SCHEMA_FILE}: {}", error.message())];
    }
    [PROTOCOL, SPECIFICATION]
        .into_iter()
        .filter(|value| !source.contains(value))
        .map(|value| format!("{PROTOCOL_SCHEMA_FILE} does not pin `{value}`"))
        .collect()
}

/// The "`{cases}` case(s) naming `{rules}` distinct rule address(es): ..." line, as its own
/// function so `Suite::census` stays under `clippy::too_many_lines` — the same reason
/// `optimal_search_census` immediately below and `Ledger::census` are calls rather than
/// inlined `format!`s there. `feasible`, `lower` and `place` are this round's and the two
/// previous rounds' own additions to the count, alongside the five kinds already here
/// (`Case::asks`'s own doc).
fn kind_census(cases: &[Case]) -> String {
    let declared: BTreeSet<&str> = cases.iter().flat_map(Case::rules).collect();
    format!(
        concat!(
            "{cases} case(s) naming {rules} distinct rule address(es): {protocol} protocol-v1, ",
            "{classify} classify, {boundary} boundary, {compose} compose, {align} align, ",
            "{tab} tab, {feasible} feasible, {lower} lower, {place} place, of which {pairs} ",
            "form a §3.1.2 frame pair"
        ),
        cases = cases.len(),
        rules = declared.len(),
        protocol = cases
            .iter()
            .filter(|case| case.body.get("protocol").is_some())
            .count(),
        classify = cases.iter().filter(|case| case.asks("classify")).count(),
        boundary = cases.iter().filter(|case| case.asks("boundary")).count(),
        compose = cases.iter().filter(|case| case.asks("compose")).count(),
        align = cases.iter().filter(|case| case.asks("align")).count(),
        tab = cases.iter().filter(|case| case.asks("tab")).count(),
        feasible = cases.iter().filter(|case| case.asks("feasible")).count(),
        lower = cases.iter().filter(|case| case.asks("lower")).count(),
        place = cases.iter().filter(|case| case.asks("place")).count(),
        pairs = cases
            .iter()
            .filter(|case| case.asks("compose")
                && matches!(
                    case.id.rsplit_once('/').map(|(_, variant)| variant),
                    Some("half-em-frame" | "full-em-frame")
                ))
            .count()
    )
}

/// Read runner-only coverage metadata from the language-independent NDJSON suite.
///
/// The product validator owns the complete protocol grammar. This dependency-free gate
/// independently checks the envelope facts on which coverage relies, so a malformed case
/// cannot close an inventoried rule merely because another test was not run.
fn examine_protocol_suite(name: &str, source: &str) -> (Vec<String>, Vec<Case>) {
    let mut problems = Vec::new();
    let mut cases = Vec::new();
    if source.as_bytes().contains(&b'\r') {
        problems.push(format!(
            "{name}: the published protocol suite must use LF only; CR is not permitted"
        ));
    }
    for (offset, line) in source.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let line_number = offset.saturating_add(1);
        let location = format!("{name}:{line_number}");
        let body = match Json::parse(line) {
            Ok(body) => body,
            Err(error) => {
                problems.push(format!("{location}: {}", error.message()));
                continue;
            },
        };
        let Some(object) = body.as_object() else {
            problems.push(format!(
                "{location}: a protocol suite case must be an object"
            ));
            continue;
        };
        let mut valid = true;
        for (field, _) in object {
            if !["protocol", "spec", "id", "rules", "request", "expected"].contains(&field.as_str())
            {
                problems.push(format!(
                    "{location}: unknown protocol suite field `{field}`"
                ));
                valid = false;
            }
        }
        for (field, expected) in [("protocol", PROTOCOL), ("spec", SPECIFICATION)] {
            if body.get(field).and_then(Json::as_text) != Some(expected) {
                problems.push(format!("{location}: `{field}` must be `{expected}`"));
                valid = false;
            }
        }
        let id = body
            .get("id")
            .and_then(Json::as_text)
            .filter(|id| !id.is_empty());
        if id.is_none() {
            problems.push(format!("{location}: `id` must be a non-empty string"));
            valid = false;
        }
        let rules = body.get("rules").and_then(Json::as_array);
        if rules.is_none_or(|rules| {
            rules.is_empty()
                || rules
                    .iter()
                    .any(|rule| rule.as_text().is_none_or(str::is_empty))
        }) {
            problems.push(format!(
                "{location}: a protocol suite case needs a non-empty rules array of strings"
            ));
            valid = false;
        }
        for field in ["request", "expected"] {
            if body.get(field).and_then(Json::as_object).is_none() {
                problems.push(format!("{location}: `{field}` must be an object"));
                valid = false;
            }
        }
        if valid {
            cases.push(Case {
                file: location,
                id: id.unwrap_or_default().to_owned(),
                body,
            });
        }
    }
    (problems, cases)
}

/// How many `compose` cases name `Search::Optimal` rather than leaving `search` absent
/// (`Search::FirstFit`), as its own function so `Suite::census` stays under
/// `clippy::too_many_lines` — the same reason `Ledger::census` is a call rather than an
/// inlined `format!` there.
fn optimal_search_census(cases: &[Case]) -> String {
    format!(
        "{count} compose case(s) name `Search::Optimal` rather than leaving `search` absent \
         (`Search::FirstFit`)",
        count = cases
            .iter()
            .filter(|case| case.asks("compose") && case.names_optimal_search())
            .count()
    )
}

/// How the inventory divides into its four observable accounting states.
///
/// A rule that is both covered and deferred counts as covered here, because it is: the
/// deferral is the thing that has gone stale, and `Ledger::examine` reports it as such
/// rather than this line reporting the same fact as a second number.
fn split(
    inventory: &BTreeSet<String>,
    covered: &BTreeSet<&str>,
    deferred: &BTreeSet<&str>,
    classified: &BTreeSet<&str>,
) -> (usize, usize, usize, usize) {
    let mut with_case = 0usize;
    let mut with_deferral = 0usize;
    let mut with_classification = 0usize;
    let mut with_neither = 0usize;
    for rule in inventory {
        if covered.contains(rule.as_str()) {
            with_case = with_case.saturating_add(1);
        } else if deferred.contains(rule.as_str()) {
            with_deferral = with_deferral.saturating_add(1);
        } else if classified.contains(rule.as_str()) {
            with_classification = with_classification.saturating_add(1);
        } else {
            with_neither = with_neither.saturating_add(1);
        }
    }
    (with_case, with_deferral, with_classification, with_neither)
}

/// One case, kept for the checks that span cases.
#[derive(Debug)]
struct Case {
    /// The case file, relative to the workspace root.
    file: String,
    /// The `id`, which is unique across the suite.
    id: String,
    /// The case itself, for the checks that compare two of them.
    body: Json,
}

impl Case {
    /// The rule addresses this case declares, unparsed.
    fn rules(&self) -> Vec<&str> {
        self.body
            .get("rules")
            .and_then(Json::as_array)
            .unwrap_or_default()
            .iter()
            .filter_map(Json::as_text)
            .collect()
    }

    /// Whether this case asks one of the eight questions.
    fn asks(&self, kind: &str) -> bool {
        self.body
            .get("input")
            .and_then(|input| input.get("kind"))
            .and_then(Json::as_text)
            == Some(kind)
    }

    /// Whether this `compose` case names `Search::Optimal` rather than leaving `search`
    /// absent (`Search::FirstFit`, the reading every case published before this field
    /// existed already assumed).
    fn names_optimal_search(&self) -> bool {
        self.body
            .get("input")
            .and_then(|input| input.get("search"))
            .and_then(|search| search.get("kind"))
            .and_then(Json::as_text)
            == Some("optimal")
    }

    /// Every address this case names inside an expectation, unparsed.
    ///
    /// `expect.class.rules` is the case's own statement of what decided the answer, and the
    /// runner deliberately never compares it: ADR 0006 requires an implementation to answer
    /// the question rather than to reproduce this project's chain of specification
    /// addresses. Nothing held it to anything either, so it was the one place §C.3, §C.2#2,
    /// §B.2#15 and §E.2#2 appeared as the provenance of a specific answer and the one place
    /// no gate could read. It is provenance rather than coverage — a rule named only here is
    /// not covered by the case and is not subtracted from the inventory — so it is held to
    /// the grammar and to the inventory and to nothing else.
    fn cited(&self) -> Vec<&str> {
        let mut found = Vec::new();
        cited_rules(&self.body, false, &mut found);
        found
    }

    /// The family patterns this case claims, unparsed.
    fn covers(&self) -> Vec<&str> {
        self.body
            .get("covers")
            .and_then(Json::as_array)
            .unwrap_or_default()
            .iter()
            .filter_map(Json::as_text)
            .collect()
    }

    /// How many matrix coordinates this case declares under `cells`.
    ///
    /// A count and not the coordinates themselves: this gate's own job is the field's
    /// shape, not the semantic cross-check against the transcription, which
    /// `xtask::attest`'s own `conformance-cases-agree-with-the-cells` reads independently.
    fn declared_cells(&self) -> usize {
        self.body
            .get("cells")
            .and_then(Json::as_array)
            .map_or(0, <[Json]>::len)
    }
}

/// What the per-case checks need from outside the case.
#[derive(Debug, Clone, Copy)]
struct Reference<'a> {
    /// Units per ideographic em, when the crate declaring it could be read.
    units_per_em: Option<i64>,
    /// The generated policy space, when it exists: each question path with every choice
    /// name `spec/derived/questions.tsv` records it as permitting.
    ///
    /// The answers are read as well as the paths, because a choice was checked for being a
    /// non-empty string and nothing else: `kinsoku.level = banana` passed the gate and then
    /// applied to nothing at run time, which is the failure the stable dotted paths exist to
    /// prevent, one column over.
    questions: Option<&'a BTreeMap<String, BTreeSet<String>>>,
}

/// The case files of the suite, in a stable order.
///
/// One file per JLReq section, flat: anything else in the directory is left alone, because
/// the suite is a published directory and a README beside the cases is not a case.
///
/// `pub(crate)` for the same reason [`CASES_DIR`] is: `xtask::attest`'s own
/// `conformance-cases-agree-with-the-cells` checker walks the identical directory to read
/// each case's own declared matrix coordinates, and a second directory walk here would be a
/// second implementation of this one rather than a different one.
pub(crate) fn case_files(directory: &Path) -> io::Result<Vec<PathBuf>> {
    if !directory.is_dir() {
        return Ok(Vec::new());
    }
    let mut found = Vec::new();
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_file() && path.extension().is_some_and(|kind| kind == "json") {
            found.push(path);
        }
    }
    found.sort();
    Ok(found)
}

/// Read a file that is allowed not to exist yet.
fn read_if_present(path: &Path) -> io::Result<Option<String>> {
    if path.is_file() {
        return fs::read_to_string(path).map(Some);
    }
    Ok(None)
}

/// Read one column of a tab-separated inventory, or nothing when it is not generated yet.
///
/// The column is found by name in the header row rather than by position, so an inventory
/// that grows a column keeps working and one that renames the column this gate needs fails
/// with a sentence naming it instead of silently reading the wrong field.
///
/// The rows are the lines *after* the header, found the same way the header is rather than
/// by counting one line. Every file `xtask derive` writes opens with a comment block naming
/// the derivation and the digest of the snapshot it read, so the header is not line 1 and
/// skipping one line hands the column line back as a row — which reads as a rule addressed
/// `address` and inflates every count this gate reports by exactly one.
fn read_inventory(path: &Path, column: &str) -> io::Result<Option<BTreeSet<String>>> {
    let Some(source) = read_if_present(path)? else {
        return Ok(None);
    };
    inventory_column(&source, column, path).map(Some)
}

/// The generated policy space: every question path with the choices it permits.
///
/// Two columns of one file rather than one, because a case names a (question, choice) pair
/// and only half of it was ever held to the document. `permits` is the column
/// `xtask/src/policy.rs` derives from the sentence each row quotes, so a choice name it does
/// not list is a name the specification does not permit.
fn read_questions(path: &Path) -> io::Result<Option<BTreeMap<String, BTreeSet<String>>>> {
    let Some(source) = read_if_present(path)? else {
        return Ok(None);
    };
    let questions = inventory_rows(&source, &["question", "permits"], path)?;
    Ok(Some(
        questions
            .into_iter()
            .map(|row| {
                let permits = row
                    .get(1)
                    .map(|permits| {
                        permits
                            .split_whitespace()
                            .map(str::to_owned)
                            .collect::<BTreeSet<String>>()
                    })
                    .unwrap_or_default();
                (row.first().cloned().unwrap_or_default(), permits)
            })
            .collect(),
    ))
}

/// Several columns of a tab-separated inventory, row by row, read the way one column is.
fn inventory_rows(source: &str, columns: &[&str], path: &Path) -> io::Result<Vec<Vec<String>>> {
    let Some(header) = source.lines().find(|line| !is_skippable(line)) else {
        return Ok(Vec::new());
    };
    let mut indices = Vec::new();
    for column in columns {
        let Some(index) = header.split('\t').position(|name| name.trim() == *column) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{path} has no `{column}` column; docs/design/generation.md names it",
                    path = path.display()
                ),
            ));
        };
        indices.push(index);
    }
    Ok(source
        .lines()
        .skip_while(|line| is_skippable(line))
        .skip(1)
        .filter(|line| !is_skippable(line))
        .map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            indices
                .iter()
                .map(|index| {
                    fields
                        .get(*index)
                        .map(|field| field.trim().to_owned())
                        .unwrap_or_default()
                })
                .collect::<Vec<String>>()
        })
        .filter(|row| row.first().is_some_and(|first| !first.is_empty()))
        .collect())
}

/// One column of an inventory that has already been read, as its own function so the row
/// rule above is exercised by a fixture rather than only by the committed file.
fn inventory_column(source: &str, column: &str, path: &Path) -> io::Result<BTreeSet<String>> {
    let Some(header) = source.lines().find(|line| !is_skippable(line)) else {
        return Ok(BTreeSet::new());
    };
    let Some(index) = header.split('\t').position(|name| name.trim() == column) else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{path} has no `{column}` column; docs/design/generation.md names it",
                path = path.display()
            ),
        ));
    };
    Ok(source
        .lines()
        .skip_while(|line| is_skippable(line))
        .skip(1)
        .filter(|line| !is_skippable(line))
        .filter_map(|line| line.split('\t').nth(index))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect())
}

/// Whether a line of a tab-separated file carries no row.
fn is_skippable(line: &str) -> bool {
    line.trim().is_empty() || line.starts_with('#')
}

/// Read `UNITS_PER_EM` from the crate that declares it.
///
/// The denominator is not repeated here. conformance.md promises the published suite is
/// denominator independent — a future change to ADR 0007's 1/720 unit is a mechanical
/// re-derivation of our code rather than a rewrite of the suite — and a copy in this file
/// would be the second carrier that makes the promise false (ADR 0019).
fn units_per_em(root: &Path) -> io::Result<Option<i64>> {
    for source in shared::rust_sources(&root.join(UNIT_CRATE))? {
        let code = shared::code_only(&fs::read_to_string(&source)?);
        if let Some(value) = declared_units_per_em(&code) {
            return Ok(Some(value));
        }
    }
    Ok(None)
}

/// The value of a `const UNITS_PER_EM` declaration, if this source has one.
fn declared_units_per_em(code: &str) -> Option<i64> {
    for line in code.lines() {
        let Some((declaration, value)) = line.split_once('=') else {
            continue;
        };
        let tokens: Vec<&str> = declaration
            .split(|character: char| !character.is_alphanumeric() && character != '_')
            .collect();
        if !tokens.contains(&"const") || !tokens.contains(&"UNITS_PER_EM") {
            continue;
        }
        let digits: String = value.chars().filter(char::is_ascii_digit).collect();
        return digits.parse().ok();
    }
    None
}

/// Name the first few of a long list, so a report stays readable.
fn sample_of(addresses: &[&str]) -> String {
    addresses
        .iter()
        .take(8)
        .map(|address| format!("`{address}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The fields of a case file. `heading_en` and `heading_ja` are both optional and both
/// known, because JLReq publishes every heading in two locales and a case file that
/// carries one carries the other.
const FILE_REQUIRED: &[&str] = &["section", "cases"];
/// The fields a case file may also carry.
const FILE_OPTIONAL: &[&str] = &["$schema", "heading_en", "heading_ja"];
/// The fields of one case. `quote` and `rationale` are fields rather than comments
/// because JSON has none and because a published disagreement must appear in a report.
const CASE_REQUIRED: &[&str] = &[
    "id",
    "rules",
    "standing",
    "quote",
    "rationale",
    "input",
    "permitted",
];
/// The fields a case may also carry.
const CASE_OPTIONAL: &[&str] = &["covers", "forbidden", "disagreements", "cells"];
/// The fields of one `cells` entry: a captured matrix coordinate, named the way
/// `spec/captured/table<N>.<locale>.tsv` and `xtask::attest` key it rather than through the
/// `address` grammar, which a multi-table legend like §D.1 cannot spell unambiguously
/// (`cases.schema.json`'s own `matrix_cell` description states why in full).
const CELL_REQUIRED: &[&str] = &["table", "before", "after"];
/// The fields of a case's `input`. `measure` and `candidates` are additionally required of
/// a `compose` case, where a line cannot be composed without them.
const INPUT_REQUIRED: &[&str] = &["kind", "text", "scales", "items"];
/// The eight questions `input.kind` may name, `cases.schema.json`'s own enum. A value
/// outside this list used to fall through `check_input`'s own `match` silently, requiring
/// only the base fields every kind shares and asking `check_question`'s own wildcard arm to
/// hold it to `compose`'s shape regardless of what it actually named — the identical hazard
/// an unwired trait-dispatcher arm has, now closed on the reading side too.
const INPUT_KINDS: &[&str] = &[
    "classify", "boundary", "compose", "align", "tab", "feasible", "lower", "place",
];
/// The fields an `input` may also carry.
const INPUT_OPTIONAL: &[&str] = &[
    "direction",
    "candidates",
    "measure",
    "search",
    "annotations",
    "constructs",
    "first_line_indent",
    "head_indent",
    "end_indent",
    "widow_threshold",
    "alignment",
    "tab_starts",
    "tab_stops",
];
/// The fields a `search` declaration may carry: `jlreq_line::Search` in the case format's
/// own spelling.
const SEARCH_REQUIRED: &[&str] = &["kind"];
/// The one field a `search` may also carry, required alongside `kind: "optimal"` and
/// ignored for `"first-fit"`.
const SEARCH_OPTIONAL: &[&str] = &["tolerance"];
/// `jlreq_line::Search`'s two variants, in the case format's own spelling.
const SEARCHES: &[&str] = &["first-fit", "optimal"];
/// `Badness::WORST`'s own cap: the highest `tolerance` a `search` may name.
const BADNESS_WORST: i64 = 10_000;
/// The fields of one stream: the base one, and every annotation (ADR 0016).
const STREAM_REQUIRED: &[&str] = &["text", "scales", "items"];
/// The fields of one item.
const ITEM_REQUIRED: &[&str] = &["start", "advance", "scale"];
/// The fields an item may also carry.
const ITEM_OPTIONAL: &[&str] = &["frame", "role"];
/// The fields of one declared character size, anisotropic per ADR 0007.
const SCALE_REQUIRED: &[&str] = &["inline_em", "block_em"];
/// The fields of one permitted entry: a (policy, expectation) pair with its provenance.
const PERMITTED_REQUIRED: &[&str] = &["policy", "source", "expect"];
/// The fields of one forbidden outcome.
const FORBIDDEN_REQUIRED: &[&str] = &["expect", "why"];
/// The fields of one recorded disagreement with another implementation.
const DISAGREEMENT_REQUIRED: &[&str] = &[
    "implementation",
    "version",
    "behavior",
    "our_reading",
    "evidence",
];
/// The fields of one ruby construct: a base range, an annotation stream, a run pairing.
const RUBY_REQUIRED: &[&str] = &["base", "annotation", "runs"];
/// The fields a ruby construct may also carry.
const RUBY_OPTIONAL: &[&str] = &["style"];
/// The fields of one emphasis construct. §3.3.9 fixes the size and the side and repeats
/// one mark, so there is no stream and no side.
const EMPHASIS_REQUIRED: &[&str] = &["base", "symbol", "advance"];
/// The fields of one run pairing inside a ruby construct.
const RUN_REQUIRED: &[&str] = &["base", "annotation"];

/// What kind of claim a case makes, matching `Standing`.
const STANDINGS: &[&str] = &["normative", "alternative", "unstated", "adjudicated"];
/// The declared character frame (字幅), matching `Frame`. The worked case of
/// conformance.md pins the spelling of two of them and the rest follow it.
const FRAMES: &[&str] = &[
    "unstated",
    "full-em",
    "half-em",
    "third-em",
    "quarter-em",
    "proportional",
];
/// The two writing directions of §2.3.1. A case naming neither is composed both ways by
/// the direction-parity gate, so the field is optional rather than defaulted.
const DIRECTIONS: &[&str] = &["horizontal", "vertical"];
/// `jlreq_line::Alignment`'s four methods, in the case format's own spelling. Required of
/// an `align` case (`check_input`'s own per-`kind` required-fields branch) and merely
/// ignored elsewhere, the same reading `first_line_indent` already gets outside `compose`.
const ALIGNMENTS: &[&str] = &["centered", "line-head", "line-end", "even-spacing"];
/// The fields of one declared tab stop.
const TAB_STOP_REQUIRED: &[&str] = &["position", "kind"];
/// The one field a tab stop may also carry, present only for `kind: "character"`.
const TAB_STOP_OPTIONAL: &[&str] = &["at"];
/// `jlreq_line::TabKind`'s four variants, in the case format's own spelling. Required of a
/// `tab_stop` entry (`check_tab_stops`'s own per-entry check) the same way `ALIGNMENTS` is
/// required of an `align` case's own `alignment`.
const TAB_KINDS: &[&str] = &["start", "end", "centered", "character"];
/// Appendix B's `be` and `af`: the two owners a conditional space can have (ADR 0014).
const REFERENTS: &[&str] = &["preceding", "trailing"];
/// Every construct kind a caller can declare, named as `Constructs` names them. Only ruby
/// and emphasis have a shape conformance.md fixes; the rest are known keys whose entries
/// are checked as arrays until their milestone pins them.
const CONSTRUCT_KINDS: &[&str] = &[
    "ruby",
    "emphasis",
    "tate_chu_yoko",
    "warichu",
    "furiwake",
    "jidori",
    "reference_marks",
    "ornaments",
    "formulae",
];

/// Every field this gate requires, by the object it belongs to.
///
/// The committed schema must require each of them. The two are one contract stated twice —
/// once for this workspace and once for everyone else — and this is what stops them
/// drifting apart.
const REQUIRED_BY_SHAPE: &[(&str, &[&str])] = &[
    ("case file", FILE_REQUIRED),
    ("case", CASE_REQUIRED),
    ("input", INPUT_REQUIRED),
    ("stream", STREAM_REQUIRED),
    ("item", ITEM_REQUIRED),
    ("scale", SCALE_REQUIRED),
    ("permitted entry", PERMITTED_REQUIRED),
    ("forbidden entry", FORBIDDEN_REQUIRED),
    ("disagreement", DISAGREEMENT_REQUIRED),
    ("ruby", RUBY_REQUIRED),
    ("emphasis", EMPHASIS_REQUIRED),
    ("ruby run", RUN_REQUIRED),
    ("tab stop", TAB_STOP_REQUIRED),
    ("cells entry", CELL_REQUIRED),
];

/// Check one case file and return the cases it holds.
fn examine_file(name: &str, source: &str, reference: Reference) -> (Vec<String>, Vec<Case>) {
    let mut found = Vec::new();
    let mut cases = Vec::new();
    if source.contains('\r') {
        found.push("holds a CR; a case file is published with LF line endings".to_owned());
    }
    let file = match Json::parse(source) {
        Ok(file) => file,
        Err(error) => {
            found.push(error.message());
            return (prefix_all(name, found), cases);
        },
    };
    let Some(members) = file.as_object() else {
        found.push(format!("is {kind}, not an object", kind = file.kind()));
        return (prefix_all(name, found), cases);
    };
    found.extend(check_keys(members, FILE_REQUIRED, FILE_OPTIONAL));
    found.extend(check_file_section(&file, name));
    found.extend(check_schema_pointer(&file));
    let entries = file
        .get("cases")
        .and_then(Json::as_array)
        .unwrap_or_default();
    match file.get("cases") {
        Some(Json::Array(list)) if list.is_empty() => {
            found.push("holds no case; a case file is one JLReq section's cases".to_owned());
        },
        Some(other) if other.as_array().is_none() => found.push(format!(
            "`cases` is {kind}, not an array",
            kind = other.kind()
        )),
        _ => {},
    }
    let section = file
        .get("section")
        .and_then(Json::as_text)
        .unwrap_or_default();
    for (index, case) in entries.iter().enumerate() {
        let id = case.get("id").and_then(Json::as_text);
        let at = id.map_or_else(|| format!("cases[{index}]"), str::to_owned);
        found.extend(prefix_all(&at, check_case(case, section, reference)));
        if let Some(id) = id {
            cases.push(Case {
                file: name.to_owned(),
                id: id.to_owned(),
                body: case.clone(),
            });
        }
    }
    (prefix_all(name, found), cases)
}

/// The `section` names one JLReq section and the file is named after it.
fn check_file_section(file: &Json, name: &str) -> Vec<String> {
    let Some(section) = file.get("section") else {
        return Vec::new();
    };
    let Some(section) = section.as_text() else {
        return vec!["`section` is not a string".to_owned()];
    };
    let mut found = Vec::new();
    if parse_address(section).is_none_or(|address| address.note.is_some() || address.cell.is_some())
    {
        found.push(format!(
            "`section` is `{section}`, which is not a JLReq section number (ADR 0013)"
        ));
    }
    let stem = name
        .rsplit('/')
        .next()
        .and_then(|file| file.strip_suffix(".json"));
    if stem.is_some_and(|stem| stem != section) {
        found.push(format!(
            "is named for a different section than the `{section}` it declares; \
             the suite is one file per section"
        ));
    }
    found
}

/// The `$schema` pointer, when present, names the committed schema beside the cases.
fn check_schema_pointer(file: &Json) -> Vec<String> {
    match file.get("$schema").map(Json::as_text) {
        None | Some(Some("../cases.schema.json")) => Vec::new(),
        Some(_) => vec![
            "`$schema` does not point at `../cases.schema.json`, the committed schema".to_owned(),
        ],
    }
}

/// Check one case against the format conformance.md fixes.
fn check_case(case: &Json, section: &str, reference: Reference) -> Vec<String> {
    let Some(members) = case.as_object() else {
        return vec![format!("is {kind}, not an object", kind = case.kind())];
    };
    let mut found = check_keys(members, CASE_REQUIRED, CASE_OPTIONAL);
    found.extend(check_id(case, section));
    found.extend(check_addresses(case, "rules", true));
    found.extend(check_addresses(case, "covers", false));
    found.extend(check_cells(case));
    found.extend(check_standing(case));
    found.extend(check_input(case.get("input")));
    found.extend(check_permitted(case.get("permitted"), reference));
    found.extend(check_forbidden(case.get("forbidden")));
    found.extend(check_disagreements(case.get("disagreements")));
    found.extend(check_question(case));
    walk_amounts(case, "", false, reference, &mut found);
    found
}

/// The `cells` field: every declared coordinate a case exercises against the transcription.
///
/// Optional and list-valued: `docs/design/conformance.md`'s own worked example for
/// `covers` already established a family as several cells credited by one entry, and a
/// boundary case can equally exercise several tables at one coordinate — Appendix D's own
/// worked legend example drains a term across Tables 3, 4 and 5 at once. Not the `address`
/// grammar's `@` suffix: a multi-table legend such as §D.1 cannot spell one cell
/// unambiguously through it (`cases.schema.json`'s own `matrix_cell` description states
/// this in full, and `xtask::attest`'s own `conformance-cases-agree-with-the-cells`
/// registration comment states why the natural per-table prefix is not even inventoried).
/// This gate checks the field's own shape only; the semantic cross-check against
/// `spec/captured/` is `xtask attest`'s job, read independently over the committed files
/// rather than over whatever this run happened to validate.
fn check_cells(case: &Json) -> Vec<String> {
    let Some(value) = case.get("cells") else {
        return Vec::new();
    };
    let Some(entries) = value.as_array() else {
        return vec!["`cells` is not an array".to_owned()];
    };
    if entries.is_empty() {
        return vec![
            "`cells` is empty; a case that declares no coordinate omits the field rather \
             than stating an empty list"
                .to_owned(),
        ];
    }
    let mut found = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        found.extend(prefix_all(&format!("cells[{index}]"), check_cell(entry)));
    }
    found
}

/// One `cells` entry: a table number and the two axis labels, checked against the parsers
/// the transcription and the case format already share so a class out of range or an axis
/// on the wrong side is refused identically everywhere.
fn check_cell(entry: &Json) -> Vec<String> {
    let Some(members) = entry.as_object() else {
        return vec![format!("is {kind}, not an object", kind = entry.kind())];
    };
    let mut found = check_keys(members, CELL_REQUIRED, &[]);
    let table = check_cell_table(entry, &mut found);
    let before = check_cell_axis(
        entry,
        "before",
        "line-head",
        |label| shared::before(label).is_some(),
        &mut found,
    );
    let after = check_cell_axis(
        entry,
        "after",
        "line-end",
        |label| shared::after(label).is_some(),
        &mut found,
    );
    if let (Some(table), Some(before), Some(after)) = (table, before, after) {
        if !matches!(table, 1 | 3 | 4 | 5) && (before == "line-head" || after == "line-end") {
            found.push(format!(
                "table {table} has no `line-head` row or `line-end` column; only Tables 1, 3, \
                 4 and 5 do (§C.1, §E.1)"
            ));
        }
    }
    found
}

/// The `table` field: an integer from 1 through 6.
fn check_cell_table(entry: &Json, found: &mut Vec<String>) -> Option<i64> {
    match entry.get("table").map(Json::as_integer) {
        None => None,
        Some(Some(number)) if (1..=6).contains(&number) => Some(number),
        Some(Some(number)) => {
            found.push(format!("`table` is {number}; the matrices are 1 through 6"));
            None
        },
        Some(None) => {
            found.push("`table` is not an integer".to_owned());
            None
        },
    }
}

/// One axis field of a `cells` entry, `before` or `after`.
fn check_cell_axis<'a>(
    entry: &'a Json,
    field: &str,
    edge: &str,
    valid: fn(&str) -> bool,
    found: &mut Vec<String>,
) -> Option<&'a str> {
    match entry.get(field).map(Json::as_text) {
        None => None,
        Some(None) => {
            found.push(format!("`{field}` is not a string"));
            None
        },
        Some(Some(label)) if !valid(label) => {
            found.push(format!(
                "`{field}` is `{label}`, which is not `cl-01` through `cl-30` or `{edge}`"
            ));
            None
        },
        Some(Some(label)) => Some(label),
    }
}

/// Every expectation of one case is about the question the case asked, and about the same
/// occurrence of it.
///
/// Two holes in the published format, and each of them made a case that cannot fail. A
/// `permitted` entry stating no `class` over a `classify` case was compared against nothing
/// and scored as agreement, so an expectation that omits the very field its case is about
/// passed both this gate and the runner. And the ordinal a case asks about is taken from its
/// first stated expectation, after which every entry is measured against that one answer, so
/// an entry naming another ordinal is about an occurrence nobody asked about: as a
/// `forbidden` entry it silently excluded a correct answer, and as a `permitted` one it
/// could never be satisfied.
///
/// A `forbidden` entry may still state a different *kind* — it states only the fields it
/// forbids, and a line geometry says nothing about a classification — which is why the field
/// requirement is `permitted`'s alone and the ordinal requirement is both's.
///
/// A boundary case's `edge` is the same fact one level finer, checked the identical way: the
/// runner reads `before` and `edge` together from a case's first stated `boundary`
/// expectation and measures the one answer they name against every entry
/// (`jlreq_conform::run::ask`), so an entry naming a different `edge` at the same `before` is
/// not a finer reading of that answer, it is a question about a different boundary — an
/// interior adjacency and a line edge next to the same item are not the same one — and
/// deserves the same refusal a mismatched ordinal already gets.
fn check_question(case: &Json) -> Vec<String> {
    let Some(kind) = case
        .get("input")
        .and_then(|input| input.get("kind"))
        .and_then(Json::as_text)
    else {
        return Vec::new();
    };
    // `align` is named explicitly alongside the wildcard rather than left to fall into it
    // silently: `jlreq_line::align` answers with the identical `CaseOutput`/`CaseLine` shape
    // `jlreq_line::compose` does — one `Line` instead of several — so `lines` and
    // `violations` are what an `align` case's own expectation is about too
    // (`crates/jlreq-conform/src/run.rs`'s own `ask` reuses `Answer::Composed` for exactly
    // this reason). Folding it into one arm with `_` — rather than a second arm with an
    // identical body — is what a `compose`-shaped `align` case is, not a distinction this
    // gate collapses by accident.
    let (field, ordinal, alternatives) = match kind {
        "classify" => ("class", "item", &["class"][..]),
        "boundary" => ("boundary", "before", &["boundary"][..]),
        "feasible" => ("feasible", "candidate", &["feasible"][..]),
        "lower" => ("lower", "construct", &["lower"][..]),
        // "place" is named explicitly too, but with an empty `ordinal`, `align`'s, `tab`'s
        // and `compose`'s own convention rather than `feasible`'s and `lower`'s: `place`
        // answers the case's whole declared `Constructs`, not one occurrence of it, so it
        // has no per-case ordinal for the loop below to hold consistent
        // (`crates/jlreq-conform/src/run.rs`'s own `Compose::place` doc states why). Folding
        // it into the wildcard below would hold a `place` case to `lines`/`violations`
        // instead of `place`, silently accepting an expectation that never mentions the
        // field the case is actually about.
        "place" => ("place", "", &["place"][..]),
        // "align" falls in here rather than into a row of its own: see the comment above.
        _ => ("lines", "", &["lines", "violations"][..]),
    };
    let mut found = Vec::new();
    let mut asked: Option<i64> = None;
    let mut asked_edge: Option<&str> = None;
    for (side, entries) in [
        ("permitted", case.get("permitted")),
        ("forbidden", case.get("forbidden")),
    ] {
        for (index, entry) in entries
            .and_then(Json::as_array)
            .unwrap_or_default()
            .iter()
            .enumerate()
        {
            let at = format!("{side}[{index}]");
            let Some(expect) = entry.get("expect") else {
                continue;
            };
            let stated = alternatives
                .iter()
                .any(|alternative| expect.get(alternative).is_some());
            if side == "permitted" && !stated {
                found.push(format!(
                    "`{at}.expect` states none of {alternatives:?}, and the case asks the \
                     `{kind}` question; an expectation that omits the field its case is \
                     about is one no answer can fail"
                ));
            }
            if ordinal.is_empty() {
                continue;
            }
            let Some(about) = expect.get(field) else {
                continue;
            };
            if let Some(stated) = about.get(ordinal).and_then(Json::as_integer) {
                match asked {
                    None => asked = Some(stated),
                    Some(first) if first != stated => found.push(format!(
                        "`{at}.expect.{field}.{ordinal}` is {stated} and an earlier \
                         expectation of this case names {first}; a case is one input and one \
                         question, and the runner measures one answer against every entry"
                    )),
                    Some(_) => {},
                }
            }
            if field != "boundary" {
                continue;
            }
            let stated_edge = about.get("edge").and_then(Json::as_text).unwrap_or("");
            match asked_edge {
                None => asked_edge = Some(stated_edge),
                Some(first) if first != stated_edge => found.push(format!(
                    "`{at}.expect.boundary.edge` is {stated_edge:?} and an earlier \
                     expectation of this case names {first:?}; an interior boundary and a \
                     line edge next to the same item are two different questions, not two \
                     readings of one"
                )),
                Some(_) => {},
            }
        }
    }
    found
}

/// The id is `<section>/<subject>/<variant>` and its section is the file's.
fn check_id(case: &Json, section: &str) -> Vec<String> {
    let Some(id) = case.get("id") else {
        return Vec::new();
    };
    let Some(id) = id.as_text() else {
        return vec!["`id` is not a string".to_owned()];
    };
    let parts: Vec<&str> = id.split('/').collect();
    if parts.len() != 3
        || parts
            .iter()
            .any(|part| part.is_empty() || part.contains(char::is_whitespace))
    {
        return vec![format!(
            "`id` is `{id}`; an id is `<section>/<subject>/<variant>`"
        )];
    }
    if parts.first().copied() != Some(section) {
        return vec![format!(
            "`id` is `{id}`, whose section is not the file's `{section}`"
        )];
    }
    Vec::new()
}

/// Every entry of an address-valued field parses in ADR 0013's grammar.
///
/// `rules` names rules and `covers` names families, which are the same grammar with a `*`
/// permitted in a table coordinate, so one function reads both.
fn check_addresses(case: &Json, field: &str, required: bool) -> Vec<String> {
    let Some(value) = case.get(field) else {
        return Vec::new();
    };
    let Some(entries) = value.as_array() else {
        return vec![format!("`{field}` is not an array")];
    };
    if required && entries.is_empty() {
        return vec![format!(
            "`{field}` is empty; a case that names no rule covers none"
        )];
    }
    let mut found = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        match entry.as_text() {
            None => found.push(format!("`{field}[{index}]` is not a string")),
            Some(text) if parse_address(text).is_none() => found.push(format!(
                "`{field}[{index}]` is `{text}`, which is not a specification address \
                 (ADR 0013: `3.1.9`, `B.2#3`, `B.1@cl-05,cl-05`)"
            )),
            Some(_) => {},
        }
    }
    found
}

/// The standing is one of the four kinds of claim, and it decides two further things.
///
/// A case recording that JLReq does not decide records *both* readings, because nothing in
/// this format lets a silence be laundered into a requirement; and only such a case may
/// leave `quote` empty, because there is no sentence to quote.
fn check_standing(case: &Json) -> Vec<String> {
    let standing = case.get("standing").and_then(Json::as_text).unwrap_or("");
    let mut found = Vec::new();
    if case.get("standing").is_some() && !STANDINGS.contains(&standing) {
        found.push(format!(
            "`standing` is `{standing}`; it is normative, alternative, unstated or adjudicated"
        ));
    }
    if case
        .get("quote")
        .is_some_and(|quote| quote.as_text().is_none_or(|text| text.trim().is_empty()))
        && standing != "unstated"
    {
        found.push(
            "`quote` is empty; only an `unstated` case has no specification sentence to quote"
                .to_owned(),
        );
    }
    if case.get("rationale").is_some_and(|rationale| {
        rationale
            .as_text()
            .is_none_or(|text| text.trim().is_empty())
    }) {
        found
            .push("`rationale` is empty; a case says why it reads the section this way".to_owned());
    }
    if matches!(standing, "alternative" | "unstated" | "adjudicated") {
        let readings = case
            .get("permitted")
            .and_then(Json::as_array)
            .map_or(0, <[Json]>::len);
        if readings < 2 {
            found.push(format!(
                "`standing` is `{standing}` but `permitted` records {readings} reading(s); \
                 a case records every permitted outcome rather than choosing one (ADR 0006)"
            ));
        }
    }
    found
}

/// Report a required field that is absent and a field that is not in the format.
fn check_keys(members: &[(String, Json)], required: &[&str], optional: &[&str]) -> Vec<String> {
    let mut found = Vec::new();
    for key in required {
        if !members.iter().any(|(name, _)| name.as_str() == *key) {
            found.push(format!("has no `{key}`"));
        }
    }
    for (name, _) in members {
        if !required.contains(&name.as_str()) && !optional.contains(&name.as_str()) {
            found.push(format!("has an unknown field `{name}`"));
        }
    }
    found
}

/// Tag every message with the place it was found.
fn prefix_all(at: &str, messages: Vec<String>) -> Vec<String> {
    messages
        .into_iter()
        .map(|message| format!("{at}: {message}"))
        .collect()
}

/// Check the `input` object: the streams, the break candidates, and the constructs.
///
/// These are the properties ADR 0018 makes properties of a well-formed input rather than
/// of an implementation. A case whose input `Text::new` would refuse is malformed rather
/// than failing, because it tests nothing.
fn check_input(input: Option<&Json>) -> Vec<String> {
    let Some(input) = input else {
        return Vec::new();
    };
    let Some(members) = input.as_object() else {
        return vec![format!(
            "`input` is {kind}, not an object",
            kind = input.kind()
        )];
    };
    let mut required = INPUT_REQUIRED.to_vec();
    let kind = input.get("kind").and_then(Json::as_text);
    match kind {
        Some("compose") => required.extend(["measure", "candidates"]),
        // `align` needs a target and a method, never `candidates`: `jlreq_line::align`
        // never breaks, so a case asking this question supplies none.
        Some("align") => required.extend(["measure", "alignment"]),
        // `tab` needs neither a target nor break candidates: `jlreq_line::tab_line` places
        // every run against its own declared `tab_stops`, not a caller-stated length, and
        // it never breaks either. It needs the two fields §3.6.1 itself names instead.
        Some("tab") => required.extend(["tab_starts", "tab_stops"]),
        // `feasible` needs the candidates kinsoku is asked to adjudicate, but no `measure`:
        // `jlreq_line::Feasible::compute` refuses or permits a candidate, it never composes
        // a line, so this question has no line length to read.
        Some("feasible") => required.extend(["candidates"]),
        // `lower` needs the constructs it asks `jlreq_inline::lower` about, and reads
        // neither `measure` nor `candidates`: it never composes a line and never breaks
        // one, so neither field is this question's own. `place` needs the identical
        // `constructs` `lower` is answered over — the case's own subject in full, not a
        // further field alongside it — and reads neither `measure` nor `candidates`
        // either, for the identical reason: `jlreq_inline::place` never composes a line
        // and never breaks one either.
        Some("lower" | "place") => required.extend(["constructs"]),
        _ => {},
    }
    let mut found = check_keys(members, &required, INPUT_OPTIONAL);
    if let Some(kind) = kind {
        if !INPUT_KINDS.contains(&kind) {
            found.push(format!(
                "`input.kind` is {kind:?}, which is not one of {INPUT_KINDS:?}; an \
                 unrecognized kind used to fall through to `compose`'s own requirements \
                 silently, asking every case naming one the wrong question"
            ));
        }
    }
    found.extend(check_direction(input));
    found.extend(check_search(input));
    found.extend(check_alignment(input));
    found.extend(check_tab_starts(input));
    found.extend(check_tab_stops(input));
    found.extend(check_positive(input, "measure"));
    found.extend(check_widow_threshold(input));
    found.extend(check_stream(input, "input"));
    found.extend(check_candidates(input));
    for (index, annotation) in annotations_of(input).iter().enumerate() {
        let at = format!("input.annotations[{index}]");
        if let Some(members) = annotation.as_object() {
            found.extend(prefix_all(&at, check_keys(members, STREAM_REQUIRED, &[])));
        }
        found.extend(check_stream(annotation, &at));
    }
    found.extend(check_constructs(input));
    found
}

/// The direction, when the case states one, is one of §2.3.1's two.
fn check_direction(input: &Json) -> Vec<String> {
    match input.get("direction").map(Json::as_text) {
        None => Vec::new(),
        Some(Some(direction)) if DIRECTIONS.contains(&direction) => Vec::new(),
        Some(_) => vec![
            "`input.direction` is neither `horizontal` nor `vertical`; a case stating \
             neither is composed both ways"
                .to_owned(),
        ],
    }
}

/// The `search` a `compose` case states, when it states one: `jlreq_line::Search`'s own
/// shape, checked whether or not the case that states it is the `compose` case that reads
/// it — the same reading `check_alignment` and `check_tab_starts` already give a field their
/// own kind does not require. `kind` is one of `SEARCHES`, and `tolerance` is required
/// alongside `kind: "optimal"` and bounded at `Badness::WORST`'s own cap whenever it is
/// stated at all, `first-fit` included: a `tolerance` a caller cannot even construct
/// (`Badness::new` clamps rather than refuses, but this format's own contract states the
/// range it is a fraction of, not the clamped one) tests nothing about `Search::Optimal`
/// either way.
fn check_search(input: &Json) -> Vec<String> {
    let Some(search) = input.get("search") else {
        return Vec::new();
    };
    let Some(members) = search.as_object() else {
        return vec![format!(
            "`input.search` is {kind}, not an object",
            kind = search.kind()
        )];
    };
    let mut found = prefix_all(
        "input.search",
        check_keys(members, SEARCH_REQUIRED, SEARCH_OPTIONAL),
    );
    let kind = search.get("kind").and_then(Json::as_text);
    match kind {
        None => {},
        Some(kind) if SEARCHES.contains(&kind) => {},
        Some(_) => found.push(format!(
            "`input.search.kind` is not one of {SEARCHES:?}, the two variants \
             `jlreq_line::Search` has"
        )),
    }
    let tolerance = search.get("tolerance");
    if kind == Some("optimal") && tolerance.is_none() {
        found.push(
            "`input.search` names `kind: \"optimal\"` but states no `tolerance`, which that \
             variant requires"
                .to_owned(),
        );
    }
    if let Some(tolerance) = tolerance {
        match tolerance.as_integer() {
            Some(value) if (0..=BADNESS_WORST).contains(&value) => {},
            _ => found.push(format!(
                "`input.search.tolerance` is not an integer between 0 and {BADNESS_WORST}, \
                 `jlreq_line::Badness`'s own range"
            )),
        }
    }
    found
}

/// The alignment, when the case states one, is one of `jlreq_line::Alignment`'s four.
///
/// Checked regardless of `kind`, the same way `check_direction` is: `alignment` is required
/// of an `align` case by `check_input`'s own per-`kind` branch, and this is the separate
/// question of whether a *stated* value is one of the four the type has, asked whether or
/// not the case that states it is the one that needs it.
fn check_alignment(input: &Json) -> Vec<String> {
    match input.get("alignment").map(Json::as_text) {
        None => Vec::new(),
        Some(Some(alignment)) if ALIGNMENTS.contains(&alignment) => Vec::new(),
        Some(_) => vec![format!(
            "`input.alignment` is not one of {ALIGNMENTS:?}, the four methods \
             `jlreq_line::Alignment` has"
        )],
    }
}

/// The `tab_starts` a case declares: each entry the schema's own `minimum: 0` (unenforced by
/// any JSON-schema validator — this hand-written check is what actually holds it, the same
/// way every other shape this file states is held) and, since it is read as
/// `jlreq_line::tab_line`'s own item ordinal into the base stream, no larger than the
/// stream's own item count — the same bound `tab_line` itself refuses past
/// (`ComposeError::OutOfRange`, which fires only when the ordinal exceeds the count, so an
/// ordinal equal to the count is accepted and this check does not reject it either). Checked
/// regardless of `kind`, the same way `check_tab_stops` is: `tab_starts` is required of a
/// `tab` case by `check_input`'s own per-`kind` branch, and this is the separate question of
/// whether a *stated* entry is well formed, asked whether or not the case that states it is
/// the one that needs it.
fn check_tab_starts(input: &Json) -> Vec<String> {
    let Some(starts) = input.get("tab_starts") else {
        return Vec::new();
    };
    let Some(starts) = starts.as_array() else {
        return vec!["`input.tab_starts` is not an array".to_owned()];
    };
    let items = input
        .get("items")
        .and_then(Json::as_array)
        .unwrap_or_default();
    let item_count = i64::try_from(items.len()).unwrap_or(i64::MAX);
    let mut found = Vec::new();
    for (index, start) in starts.iter().enumerate() {
        let at = format!("input.tab_starts[{index}]");
        match start.as_integer() {
            Some(value) if (0..=item_count).contains(&value) => {},
            Some(_) => found.push(format!(
                "`{at}` is not an item ordinal inside the {item_count}-item base stream"
            )),
            None => found.push(format!("`{at}` is not an integer")),
        }
    }
    found
}

/// The `tab_stops` a case declares, each checked the way `check_candidates` already checks
/// one candidate: its own shape (`TAB_STOP_REQUIRED`/`TAB_STOP_OPTIONAL`), and its own
/// `kind` against `TAB_KINDS` the same way `check_alignment` checks `alignment` against
/// `ALIGNMENTS`. Checked regardless of `kind`, the same way `check_alignment` is: `tab_stops`
/// is required of a `tab` case by `check_input`'s own per-`kind` branch, and this is the
/// separate question of whether a *stated* entry is well formed, asked whether or not the
/// case that states it is the one that needs it.
fn check_tab_stops(input: &Json) -> Vec<String> {
    let Some(stops) = input.get("tab_stops") else {
        return Vec::new();
    };
    let Some(stops) = stops.as_array() else {
        return vec!["`input.tab_stops` is not an array".to_owned()];
    };
    let mut found = Vec::new();
    for (index, stop) in stops.iter().enumerate() {
        let at = format!("input.tab_stops[{index}]");
        let Some(members) = stop.as_object() else {
            found.push(format!(
                "`{at}` is {kind}, not an object",
                kind = stop.kind()
            ));
            continue;
        };
        found.extend(prefix_all(
            &at,
            check_keys(members, TAB_STOP_REQUIRED, TAB_STOP_OPTIONAL),
        ));
        let kind = stop.get("kind").and_then(Json::as_text);
        match kind {
            None => {},
            Some(kind) if TAB_KINDS.contains(&kind) => {},
            Some(_) => found.push(format!(
                "`{at}.kind` is not one of {TAB_KINDS:?}, the four kinds \
                 `jlreq_line::TabKind` has"
            )),
        }
        let names_the_character_kind = kind == Some("character");
        if names_the_character_kind && stop.get("at").is_none() {
            found.push(format!(
                "`{at}` names `kind: \"character\"` but states no `at`; that kind names the \
                 occurrence its own anchor reads"
            ));
        }
        if !names_the_character_kind && stop.get("at").is_some() {
            found.push(format!(
                "`{at}` states `at`, which only `kind: \"character\"` reads"
            ));
        }
    }
    found
}

/// One integer field that must be present and above zero when the case states it.
fn check_positive(input: &Json, field: &str) -> Vec<String> {
    match input.get(field).map(Json::as_integer) {
        None => Vec::new(),
        Some(Some(value)) if value > 0 => Vec::new(),
        Some(_) => vec![format!("`input.{field}` is not a positive integer")],
    }
}

/// `input.widow_threshold`, when the case states one, is an integer `Paragraph::
/// with_widow_threshold` can hold as a `u16` — non-negative and no larger than `u16::MAX`.
/// `check_positive` does not fit this field: `0` is `widow_threshold`'s own documented
/// no-op reading (`cases.schema.json`'s own description), not a value to refuse the way an
/// unusable `measure` is, so this is its own function rather than a second caller of that
/// one. Mirrors `check_search`'s own bound on `tolerance`: a threshold this reader cannot
/// even construct tests nothing about §3.5.4 either way, and `kumihan.rs`'s own
/// `u16::try_from(threshold).ok()?` would decline the case silently rather than fail this
/// gate, which is precisely the failure mode a shape check exists to close.
fn check_widow_threshold(input: &Json) -> Vec<String> {
    match input.get("widow_threshold").map(Json::as_integer) {
        None => Vec::new(),
        Some(Some(value)) if (0..=i64::from(u16::MAX)).contains(&value) => Vec::new(),
        Some(_) => vec![format!(
            "`input.widow_threshold` is not an integer between 0 and {max}, the range \
             `Paragraph::with_widow_threshold`'s own `u16` can hold",
            max = u16::MAX
        )],
    }
}

/// The annotation streams of an input, which are absent from most cases.
fn annotations_of(input: &Json) -> &[Json] {
    input
        .get("annotations")
        .and_then(Json::as_array)
        .unwrap_or_default()
}

/// Check one stream: its text, its declared sizes, and its items (ADR 0016, ADR 0018).
fn check_stream(stream: &Json, at: &str) -> Vec<String> {
    let text = stream.get("text").and_then(Json::as_text).unwrap_or("");
    let scales = stream
        .get("scales")
        .and_then(Json::as_array)
        .unwrap_or_default();
    let items = stream
        .get("items")
        .and_then(Json::as_array)
        .unwrap_or_default();
    let mut found = Vec::new();
    if stream.get("scales").is_some() && scales.is_empty() {
        found.push(format!(
            "`{at}.scales` is empty; an item names a declared size"
        ));
    }
    if stream.get("items").is_some() && items.is_empty() {
        found.push(format!(
            "`{at}.items` is empty; a stream with no item is not one"
        ));
    }
    for (index, scale) in scales.iter().enumerate() {
        found.extend(check_scale(scale, &format!("{at}.scales[{index}]")));
    }
    let mut previous: Option<i64> = None;
    for (index, item) in items.iter().enumerate() {
        found.extend(check_item(
            item,
            &format!("{at}.items[{index}]"),
            scales.len(),
        ));
        found.extend(check_offset(
            item.get("start"),
            text,
            &mut previous,
            &format!("{at}.items[{index}].start"),
            false,
        ));
    }
    if let Some(first) = items.first().and_then(|item| item.get("start")) {
        if first.as_integer() != Some(0) {
            found.push(format!(
                "`{at}.items[0].start` is not 0; the items tile the stream from its start"
            ));
        }
    }
    found
}

/// One declared character size. §3.3.3 scales the two axes differently, so both are
/// stated and neither is derived from the other (ADR 0007).
fn check_scale(scale: &Json, at: &str) -> Vec<String> {
    let Some(members) = scale.as_object() else {
        return vec![format!(
            "`{at}` is {kind}, not an object",
            kind = scale.kind()
        )];
    };
    let mut found = prefix_all(at, check_keys(members, SCALE_REQUIRED, &[]));
    for &axis in SCALE_REQUIRED {
        if scale
            .get(axis)
            .is_some_and(|value| value.as_integer().is_none_or(|em| em <= 0))
        {
            found.push(format!("`{at}.{axis}` is not a positive integer"));
        }
    }
    found
}

/// One item: one occurrence of one Appendix A key, on a declared size (ADR 0018).
fn check_item(item: &Json, at: &str, scales: usize) -> Vec<String> {
    let Some(members) = item.as_object() else {
        return vec![format!(
            "`{at}` is {kind}, not an object",
            kind = item.kind()
        )];
    };
    let mut found = prefix_all(at, check_keys(members, ITEM_REQUIRED, ITEM_OPTIONAL));
    if item
        .get("advance")
        .is_some_and(|advance| advance.as_integer().is_none_or(|value| value < 0))
    {
        found.push(format!("`{at}.advance` is not a length"));
    }
    let scale = item.get("scale").map(Json::as_integer);
    if let Some(scale) = scale {
        let stated = scale.filter(|ordinal| *ordinal >= 0);
        let known = stated.and_then(|ordinal| usize::try_from(ordinal).ok());
        if known.is_none_or(|ordinal| ordinal >= scales) {
            found.push(format!(
                "`{at}.scale` does not name one of the {scales} declared size(s)"
            ));
        }
    }
    if item
        .get("frame")
        .is_some_and(|frame| frame.as_text().is_none_or(|name| !FRAMES.contains(&name)))
    {
        found.push(format!(
            "`{at}.frame` is not a declared character frame (字幅): \
             one of full-em, half-em, third-em, quarter-em, proportional, unstated"
        ));
    }
    found
}

/// The caller's break candidates, which are byte offsets into the running text.
///
/// A candidate at offset zero or at the end of the text names the paragraph's own edges
/// and is accepted, because every UAX #14 implementation an adopter already runs emits the
/// second (ADR 0018).
fn check_candidates(input: &Json) -> Vec<String> {
    let text = input.get("text").and_then(Json::as_text).unwrap_or("");
    let Some(candidates) = input.get("candidates") else {
        return Vec::new();
    };
    let Some(candidates) = candidates.as_array() else {
        return vec!["`input.candidates` is not an array".to_owned()];
    };
    let mut found = Vec::new();
    let mut previous: Option<i64> = None;
    for (index, candidate) in candidates.iter().enumerate() {
        let at = format!("input.candidates[{index}]");
        if let Some(members) = candidate.as_object() {
            found.extend(prefix_all(&at, check_keys(members, &["at"], &[])));
        }
        found.extend(check_offset(
            candidate.get("at"),
            text,
            &mut previous,
            &format!("{at}.at"),
            true,
        ));
    }
    found
}

/// One byte offset into a stream: on a character boundary, in range, and past the last.
fn check_offset(
    offset: Option<&Json>,
    text: &str,
    previous: &mut Option<i64>,
    at: &str,
    edge_allowed: bool,
) -> Vec<String> {
    let Some(offset) = offset else {
        return Vec::new();
    };
    let Some(value) = offset.as_integer() else {
        return vec![format!("`{at}` is not a byte offset")];
    };
    let mut found = Vec::new();
    if previous.is_some_and(|last| value <= last) {
        found.push(format!("`{at}` does not advance past the offset before it"));
    }
    *previous = Some(value);
    let placed = usize::try_from(value)
        .ok()
        .filter(|byte| text.is_char_boundary(*byte))
        .filter(|byte| edge_allowed || *byte < text.len());
    if placed.is_none() {
        found.push(format!(
            "`{at}` is {value}, which is not a character boundary inside the {length}-byte text",
            length = text.len()
        ));
    }
    found
}

/// The constructs declared over the streams (ADR 0016).
fn check_constructs(input: &Json) -> Vec<String> {
    let Some(constructs) = input.get("constructs") else {
        return Vec::new();
    };
    let Some(members) = constructs.as_object() else {
        return vec!["`input.constructs` is not an object".to_owned()];
    };
    let mut found = check_keys(members, &[], CONSTRUCT_KINDS)
        .into_iter()
        .map(|message| format!("`input.constructs` {message}"))
        .collect::<Vec<_>>();
    for (kind, value) in members {
        let Some(entries) = value.as_array() else {
            found.push(format!("`input.constructs.{kind}` is not an array"));
            continue;
        };
        for (index, entry) in entries.iter().enumerate() {
            let at = format!("input.constructs.{kind}[{index}]");
            match kind.as_str() {
                "ruby" => found.extend(check_ruby(entry, input, &at)),
                "emphasis" => found.extend(check_emphasis(entry, input, &at)),
                _ => {},
            }
        }
    }
    found
}

/// One ruby construct, whose ordinals index two different streams (ADR 0016).
fn check_ruby(ruby: &Json, input: &Json, at: &str) -> Vec<String> {
    let Some(members) = ruby.as_object() else {
        return vec![format!(
            "`{at}` is {kind}, not an object",
            kind = ruby.kind()
        )];
    };
    let mut found = prefix_all(at, check_keys(members, RUBY_REQUIRED, RUBY_OPTIONAL));
    let base_items = items_of(input);
    found.extend(check_range(
        ruby.get("base"),
        base_items,
        &format!("{at}.base"),
    ));
    let annotations = annotations_of(input);
    let stream = ruby
        .get("annotation")
        .and_then(Json::as_integer)
        .and_then(|ordinal| usize::try_from(ordinal).ok())
        .filter(|ordinal| *ordinal < annotations.len());
    if ruby.get("annotation").is_some() && stream.is_none() {
        found.push(format!(
            "`{at}.annotation` does not name one of the {count} annotation stream(s) \
             the input declares",
            count = annotations.len()
        ));
    }
    let annotation_items = stream
        .and_then(|ordinal| annotations.get(ordinal))
        .map_or(0, items_of);
    for (index, run) in ruby
        .get("runs")
        .and_then(Json::as_array)
        .unwrap_or_default()
        .iter()
        .enumerate()
    {
        let at = format!("{at}.runs[{index}]");
        if let Some(members) = run.as_object() {
            found.extend(prefix_all(&at, check_keys(members, RUN_REQUIRED, &[])));
        }
        found.extend(check_range(
            run.get("base"),
            base_items,
            &format!("{at}.base"),
        ));
        found.extend(check_range(
            run.get("annotation"),
            annotation_items,
            &format!("{at}.annotation"),
        ));
    }
    found
}

/// One emphasis construct. §3.3.9 fixes the size and the side, so it carries no stream.
fn check_emphasis(emphasis: &Json, input: &Json, at: &str) -> Vec<String> {
    let Some(members) = emphasis.as_object() else {
        return vec![format!(
            "`{at}` is {kind}, not an object",
            kind = emphasis.kind()
        )];
    };
    let mut found = prefix_all(at, check_keys(members, EMPHASIS_REQUIRED, &[]));
    found.extend(check_range(
        emphasis.get("base"),
        items_of(input),
        &format!("{at}.base"),
    ));
    if emphasis
        .get("symbol")
        .is_some_and(|symbol| symbol.as_text().is_none_or(str::is_empty))
    {
        found.push(format!("`{at}.symbol` is not a code point"));
    }
    found
}

/// How many items one stream declares.
fn items_of(stream: &Json) -> usize {
    stream
        .get("items")
        .and_then(Json::as_array)
        .map_or(0, <[Json]>::len)
}

/// A half-open ordinal range into one stream.
///
/// This is the check ADR 0016 asks the runner to hold: an ordinal indexes the stream the
/// surrounding object names, so a swapped base and annotation is a gate failure here
/// exactly as it is a compile error inside kumihan.
fn check_range(range: Option<&Json>, length: usize, at: &str) -> Vec<String> {
    let Some(range) = range else {
        return Vec::new();
    };
    let Some(bounds) = range.as_array() else {
        return vec![format!("`{at}` is not a `[start, end]` range")];
    };
    let (Some(start), Some(end)) = (
        bounds.first().and_then(Json::as_integer),
        bounds.get(1).and_then(Json::as_integer),
    ) else {
        return vec![format!("`{at}` is not a `[start, end]` range")];
    };
    let last = i64::try_from(length).unwrap_or(i64::MAX);
    if bounds.len() != 2 || start < 0 || end < start || end > last {
        return vec![format!(
            "`{at}` is [{start}, {end}), which is not a range of the {length} item(s) \
             of the stream it names (ADR 0016)"
        )];
    }
    Vec::new()
}

/// Check `permitted`: every reading JLReq allows, each tied to the policy that selects it.
///
/// The selection rule is made unique by a static check on the case rather than by a
/// run-time tie-break, which is what makes an "any of these" expectation impossible to
/// write: an entry carries the overlay that selects it, no two entries carry the same
/// overlay, and the entries' key sets are ordered by inclusion so the most specific
/// applying entry always exists and is always unique.
fn check_permitted(permitted: Option<&Json>, reference: Reference) -> Vec<String> {
    let Some(permitted) = permitted else {
        return Vec::new();
    };
    let Some(entries) = permitted.as_array() else {
        return vec!["`permitted` is not an array".to_owned()];
    };
    if entries.is_empty() {
        return vec![
            "`permitted` is empty; a case records at least one permitted outcome".to_owned(),
        ];
    }
    let mut found = Vec::new();
    let mut overlays = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let at = format!("permitted[{index}]");
        let Some(members) = entry.as_object() else {
            found.push(format!(
                "`{at}` is {kind}, not an object",
                kind = entry.kind()
            ));
            continue;
        };
        found.extend(prefix_all(
            &at,
            check_keys(members, &["source", "expect"], &["policy"]),
        ));
        if entry
            .get("expect")
            .is_some_and(|expect| expect.as_object().is_none_or(<[(String, Json)]>::is_empty))
        {
            found.push(format!("`{at}.expect` states nothing"));
        }
        let Some(policy) = entry.get("policy") else {
            found.push(format!(
                "`{at}` has no `policy`; `permitted` is a list of (policy, expectation) pairs \
                 rather than a list of bare expectations, so an implementation is told which \
                 reading applies to it"
            ));
            continue;
        };
        let (messages, overlay) = check_overlay(policy, reference, &at);
        found.extend(messages);
        overlays.push((at, overlay));
    }
    found.extend(check_selection(&overlays));
    found
}

/// One `policy` overlay: a partial map from question path to choice name.
fn check_overlay(
    policy: &Json,
    reference: Reference,
    at: &str,
) -> (Vec<String>, BTreeMap<String, String>) {
    let mut found = Vec::new();
    let mut overlay = BTreeMap::new();
    let Some(members) = policy.as_object() else {
        found.push(format!("`{at}.policy` is not an object"));
        return (found, overlay);
    };
    for (question, choice) in members {
        let permits = reference.questions.map(|known| known.get(question));
        if !is_path(question) {
            found.push(format!(
                "`{at}.policy` names `{question}`, which is not a question path"
            ));
        } else if permits == Some(None) {
            found.push(format!(
                "`{at}.policy` names `{question}`, which the generated policy space does \
                 not contain"
            ));
        }
        match choice.as_text() {
            Some(name) if !name.is_empty() => {
                if let Some(Some(permitted)) = permits {
                    if !permitted.contains(name) {
                        found.push(format!(
                            "`{at}.policy.{question}` is `{name}`, which is not one of the \
                             answer(s) spec/derived/questions.tsv records `{question}` as \
                             permitting: {permitted:?}. A choice the policy space does not \
                             have applies to nothing, so the entry would be unreachable \
                             rather than wrong"
                        ));
                    }
                }
                overlay.insert(question.clone(), name.to_owned());
            },
            _ => found.push(format!(
                "`{at}.policy.{question}` is not the name of a permitted choice"
            )),
        }
    }
    (found, overlay)
}

/// Whether a string is a stable dotted policy path, as `Question::path` renders one.
fn is_path(text: &str) -> bool {
    !text.is_empty()
        && text.split('.').all(|segment| {
            !segment.is_empty()
                && segment
                    .chars()
                    .all(|character| character.is_ascii_lowercase() || character == '_')
        })
}

/// The selection rule has exactly one answer for every policy an implementation declares.
fn check_selection(overlays: &[(String, BTreeMap<String, String>)]) -> Vec<String> {
    let mut found = Vec::new();
    for (index, (at, left)) in overlays.iter().enumerate() {
        for (other, right) in overlays.iter().skip(index.saturating_add(1)) {
            let mine: BTreeSet<&String> = left.keys().collect();
            let theirs: BTreeSet<&String> = right.keys().collect();
            if !mine.is_subset(&theirs) && !theirs.is_subset(&mine) {
                found.push(format!(
                    "`{at}` and `{other}` name question sets that are not ordered by inclusion, \
                     so a policy setting both selects neither"
                ));
            } else if left == right {
                found.push(format!(
                    "`{at}` and `{other}` declare the same overlay, so no policy selects \
                     one of them over the other"
                ));
            }
        }
    }
    found
}

/// Check `forbidden`: the outcomes the specification excludes between two permitted ones.
fn check_forbidden(forbidden: Option<&Json>) -> Vec<String> {
    let Some(forbidden) = forbidden else {
        return Vec::new();
    };
    let Some(entries) = forbidden.as_array() else {
        return vec!["`forbidden` is not an array".to_owned()];
    };
    let mut found = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let at = format!("forbidden[{index}]");
        let Some(members) = entry.as_object() else {
            found.push(format!(
                "`{at}` is {kind}, not an object",
                kind = entry.kind()
            ));
            continue;
        };
        found.extend(prefix_all(
            &at,
            check_keys(members, FORBIDDEN_REQUIRED, &[]),
        ));
        if entry
            .get("expect")
            .is_some_and(|expect| expect.as_object().is_none_or(<[(String, Json)]>::is_empty))
        {
            found.push(format!("`{at}.expect` states nothing"));
        }
        if entry
            .get("why")
            .is_some_and(|why| why.as_text().is_none_or(|text| text.trim().is_empty()))
        {
            found.push(format!(
                "`{at}.why` is empty; a forbidden outcome names the sentence that excludes it"
            ));
        }
    }
    found
}

/// Check `disagreements`: what another implementation does, and what we read instead.
///
/// The field is data rather than prose so a disagreement appears in a report, and every
/// part of it is needed there: the report names the implementation and its version, states
/// both readings, and cites the evidence.
fn check_disagreements(disagreements: Option<&Json>) -> Vec<String> {
    let Some(disagreements) = disagreements else {
        return Vec::new();
    };
    let Some(entries) = disagreements.as_array() else {
        return vec!["`disagreements` is not an array".to_owned()];
    };
    let mut found = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let at = format!("disagreements[{index}]");
        let Some(members) = entry.as_object() else {
            found.push(format!(
                "`{at}` is {kind}, not an object",
                kind = entry.kind()
            ));
            continue;
        };
        found.extend(prefix_all(
            &at,
            check_keys(members, DISAGREEMENT_REQUIRED, &[]),
        ));
        for &field in DISAGREEMENT_REQUIRED {
            if entry
                .get(field)
                .is_some_and(|value| value.as_text().is_none_or(|text| text.trim().is_empty()))
            {
                found.push(format!("`{at}.{field}` is empty"));
            }
        }
    }
    found
}

/// Walk every value of a case, checking the two things the format states about numbers
/// wherever they appear, and every `trims` array on the way.
///
/// Shape-independent on purpose: a boundary expectation and a line expectation state their
/// amounts the same way, so this holds for the kinds of case whose expectation shape a
/// later milestone pins as much as for the composition case worked out today.
fn walk_amounts(
    value: &Json,
    at: &str,
    inside_forbidden: bool,
    reference: Reference,
    found: &mut Vec<String>,
) {
    match value {
        Json::Object(members) => {
            if members.iter().any(|(key, _)| key == "em") {
                found.extend(check_amount(members, at, inside_forbidden, reference));
            }
            for (key, child) in members {
                let below = if at.is_empty() {
                    key.clone()
                } else {
                    format!("{at}.{key}")
                };
                if key == "trims" {
                    found.extend(check_trims(child, &below, inside_forbidden));
                }
                if key == "rules" && !at.is_empty() {
                    found.extend(check_addresses(value, key, false));
                }
                walk_amounts(
                    child,
                    &below,
                    inside_forbidden || key == "forbidden",
                    reference,
                    found,
                );
            }
        },
        Json::Array(entries) => {
            for (index, entry) in entries.iter().enumerate() {
                walk_amounts(
                    entry,
                    &format!("{at}[{index}]"),
                    inside_forbidden,
                    reference,
                    found,
                );
            }
        },
        _ => {},
    }
}

/// One amount: a fraction of the em and the unit count it resolves to.
///
/// The two are written together and checked against each other, which is what makes the
/// published suite denominator independent: a future change to ADR 0007's unit is a
/// mechanical re-derivation of the unit counts rather than a rewrite of the cases. An
/// amount inside `forbidden` is a pattern rather than a value, so there the unit count is
/// optional and only an outright disagreement is reported.
fn check_amount(
    members: &[(String, Json)],
    at: &str,
    inside_forbidden: bool,
    reference: Reference,
) -> Vec<String> {
    let Some(em) = members
        .iter()
        .find(|(key, _)| key == "em")
        .map(|(_, value)| value)
    else {
        return Vec::new();
    };
    let (Some(numerator), Some(denominator)) = (
        em.as_array()
            .and_then(<[Json]>::first)
            .and_then(Json::as_integer),
        em.as_array()
            .and_then(|parts| parts.get(1))
            .and_then(Json::as_integer),
    ) else {
        return vec![format!(
            "`{at}.em` is not a fraction written `[numerator, denominator]`"
        )];
    };
    if denominator == 0 {
        return vec![format!("`{at}.em` has a zero denominator")];
    }
    let Some(units) = members.iter().find(|(key, _)| key == "units") else {
        if inside_forbidden {
            return Vec::new();
        }
        return vec![format!(
            "`{at}` states `em` and no `units`; an amount is written as a fraction and a \
             unit count so the two can be checked against each other"
        )];
    };
    let Some(units) = units.1.as_integer() else {
        return vec![format!("`{at}.units` is not an integer")];
    };
    let Some(per_em) = reference.units_per_em else {
        return vec![format!(
            "`{at}` states an amount, and the em denominator could not be read from \
             {UNIT_CRATE}, so the two cannot be checked against each other"
        )];
    };
    match exact_units(numerator, denominator, per_em) {
        Some(exact) if exact == units => Vec::new(),
        Some(exact) => vec![format!(
            "`{at}` states {numerator}/{denominator} em and {units} units, \
             but {numerator}/{denominator} of an em is {exact} units"
        )],
        None => vec![format!(
            "`{at}` states {numerator}/{denominator} em, which a unit of 1/{per_em} em cannot \
             state exactly (ADR 0007)"
        )],
    }
}

/// The unit count of a fraction of the em, when the unit states it exactly.
fn exact_units(numerator: i64, denominator: i64, per_em: i64) -> Option<i64> {
    let total = numerator.checked_mul(per_em)?;
    if total.checked_rem(denominator)? != 0 {
        return None;
    }
    total.checked_div(denominator)
}

/// Every trim names the sentence that took the unit out of a supplied advance.
///
/// Without this an implementation could discharge an overlong line by subtracting an
/// arbitrary quantity from a caller's advance and calling it a trim, which is the one way
/// ADR 0002 can be evaded while still reporting everything. §3.1.2 states the frame, and
/// Table 1's cells state the conditional spaces; nothing else states a trim.
fn check_trims(trims: &Json, at: &str, inside_forbidden: bool) -> Vec<String> {
    let Some(entries) = trims.as_array() else {
        return vec![format!("`{at}` is not an array")];
    };
    let mut found = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let at = format!("{at}[{index}]");
        match entry.get("rule").map(Json::as_text) {
            Some(Some(rule)) if is_trim_rule(rule) => {},
            None if inside_forbidden => {},
            None => found.push(format!(
                "`{at}` names no `rule`; a unit taken out of a supplied advance is reported \
                 with the sentence that took it (ADR 0002, ADR 0017)"
            )),
            Some(rule) => found.push(format!(
                "`{at}.rule` is `{rule}`; a trim is stated by §3.1.2 or by a Table 1 cell \
                 such as `B.1@cl-05,cl-05`",
                rule = rule.unwrap_or("not a string")
            )),
        }
        if entry.get("referent").is_some_and(|referent| {
            referent
                .as_text()
                .is_none_or(|name| !REFERENTS.contains(&name))
        }) {
            found.push(format!(
                "`{at}.referent` is neither `preceding` nor `trailing`, which are \
                 Appendix B's `be` and `af`"
            ));
        }
    }
    found
}

/// Whether an address states a trim: §3.1.2, or a cell of Table 1.
fn is_trim_rule(rule: &str) -> bool {
    let Some(address) = parse_address(rule) else {
        return false;
    };
    match (address.section.as_str(), address.cell.as_ref()) {
        ("3.1.2", None) => address.note.is_none(),
        ("B.1", Some(_)) => true,
        _ => false,
    }
}

/// One rule address, in the grammar ADR 0013 fixes.
///
/// `section := digit+ ('.' digit+)* | [A-G] ('.' digit+)*`, and
/// `address := section ('#' note)? | section '@' cell`. The `#` is kumihan's separator for
/// JLReq's "note N", which the published document gives no machine-readable identifier.
#[derive(Debug, PartialEq, Eq)]
struct RuleAddress {
    /// The section path, as the document renders it.
    section: String,
    /// The note ordinal, for an appendix note.
    note: Option<String>,
    /// The two coordinates of a table cell, which is a rule because most cells implement
    /// no note.
    cell: Option<(String, String)>,
}

/// Read an address, accepting `*` as a cell coordinate only in a family pattern.
fn parse_address(text: &str) -> Option<RuleAddress> {
    if let Some((section, cell)) = text.split_once('@') {
        let (before, after) = cell.split_once(',')?;
        return Some(RuleAddress {
            section: valid_section(section)?.to_owned(),
            note: None,
            cell: Some((
                valid_before(before)?.to_owned(),
                valid_after(after)?.to_owned(),
            )),
        });
    }
    if let Some((section, note)) = text.split_once('#') {
        return Some(RuleAddress {
            section: valid_section(section)?.to_owned(),
            note: Some(valid_note(note)?.to_owned()),
            cell: None,
        });
    }
    Some(RuleAddress {
        section: valid_section(text)?.to_owned(),
        note: None,
        cell: None,
    })
}

/// A section path: JLReq's own numbering, or one of its appendix letters.
///
/// The four artifacts ADR 0013 names have to agree about what an address *is*, so this
/// gate reads `shared`'s grammar rather than a second one of its own. A gate that
/// accepted a wider language than the library would let a case file name a cell no
/// inventory row can ever carry, and the coverage subtraction would still close.
fn valid_section(text: &str) -> Option<&str> {
    shared::section(text).map(|_| text)
}

/// A note ordinal: JLReq numbers its appendix notes from one.
fn valid_note(text: &str) -> Option<&str> {
    shared::number(text).map(|_| text)
}

/// A row coordinate of a table cell, or the `*` of a family pattern.
///
/// The wildcard is the one thing this gate adds to the grammar, because a family pattern
/// is written in the address space and is not itself an address.
fn valid_before(text: &str) -> Option<&str> {
    (text == "*" || shared::before(text).is_some()).then_some(text)
}

/// A column coordinate of a table cell, or the `*` of a family pattern.
fn valid_after(text: &str) -> Option<&str> {
    (text == "*" || shared::after(text).is_some()).then_some(text)
}

/// Whether a family pattern names one inventoried address.
///
/// A row is `B.1@cl-05,*`, a column is `B.1@*,cl-05`, and a class-pair set is a list of
/// either. The pattern is written in the address space itself, so a family credits exactly
/// the rules it names and a pattern that names none is dead rather than silently generous.
fn pattern_names(pattern: &RuleAddress, address: &RuleAddress) -> bool {
    if pattern.section != address.section || pattern.note != address.note {
        return false;
    }
    match (pattern.cell.as_ref(), address.cell.as_ref()) {
        (None, None) => true,
        (Some((before, after)), Some((row, column))) => {
            (before == "*" || before == row) && (after == "*" || after == column)
        },
        _ => false,
    }
}

/// Every case id is unique across the suite.
fn unique_ids(cases: &[Case]) -> Vec<String> {
    let mut seen: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for case in cases {
        seen.entry(&case.id).or_default().push(&case.file);
    }
    seen.into_iter()
        .filter(|(_, files)| files.len() > 1)
        .map(|(id, files)| {
            format!(
                "the id `{id}` is used {count} times, in {files}; a case id is unique across \
                 the suite",
                count = files.len(),
                files = files.join(", ")
            )
        })
        .collect()
}

/// The §3.1.2 frame pair states one geometry from two directions, and that is asserted
/// here rather than left to a reader to notice.
///
/// The two cases declare the same text on the half-em and the full-em frame. Composition
/// normalizes to the specification's geometry and reports the trim, so their expectations
/// are identical except for `trims`, and their inputs are identical except for each item's
/// declared frame and the advance that frame describes. An implementation that adds Table
/// 1's half em to an advance that already contains it fails the second case on `extent`;
/// one that shortens the caller's advance without saying so fails on `trims`.
///
/// It is ADR 0017's assertion about *composition*, so it is asked of composition cases and
/// of nothing else. A classification pair may name the same two variants and mean the
/// opposite thing — the frame is what separates §A.1's listing of a key from §A.27's, so
/// there the expectations differ precisely because the frames do — and holding such a pair
/// to "identical except `trims`" would fail it for the wrong reason. §A.1 avoids the
/// collision today by naming its parenthesis pair `japanese-context-half-em` and
/// `western-context-proportional`, which is a naming convention and not a check; this is
/// the check.
fn frame_pairs(cases: &[Case]) -> Vec<String> {
    let mut halves: BTreeMap<&str, &Case> = BTreeMap::new();
    let mut fulls: BTreeMap<&str, &Case> = BTreeMap::new();
    for case in cases.iter().filter(|case| case.asks("compose")) {
        let Some((subject, variant)) = case.id.rsplit_once('/') else {
            continue;
        };
        match variant {
            "half-em-frame" => {
                halves.insert(subject, case);
            },
            "full-em-frame" => {
                fulls.insert(subject, case);
            },
            _ => {},
        }
    }
    let mut found = Vec::new();
    for (subject, half) in &halves {
        match fulls.get(subject) {
            Some(full) => found.extend(compare_frames(half, full)),
            None => found.push(format!(
                "`{id}` has no `{subject}/full-em-frame` beside it; the frame pair is one \
                 assertion and is checked as one (ADR 0017)",
                id = half.id
            )),
        }
    }
    for (subject, full) in &fulls {
        if !halves.contains_key(subject) {
            found.push(format!(
                "`{id}` has no `{subject}/half-em-frame` beside it; the frame pair is one \
                 assertion and is checked as one (ADR 0017)",
                id = full.id
            ));
        }
    }
    found
}

/// Compare the two cases of one frame pair.
fn compare_frames(half: &Case, full: &Case) -> Vec<String> {
    let mut found = Vec::new();
    let inputs = difference_between(
        half.body.get("input"),
        full.body.get("input"),
        &["frame", "advance"],
        "input",
    );
    if let Some(at) = inputs {
        found.push(format!(
            "`{id}` and `{other}` differ at `{at}`; the two cases of a frame pair declare \
             the same input apart from each item's frame and the advance it describes",
            id = half.id,
            other = full.id
        ));
    }
    let expectations = difference_between(
        half.body.get("permitted"),
        full.body.get("permitted"),
        &["trims"],
        "permitted",
    );
    if let Some(at) = expectations {
        found.push(format!(
            "`{id}` and `{other}` differ at `{at}`; the two cases of a frame pair expect \
             the same geometry and differ only in `trims` (ADR 0017)",
            id = half.id,
            other = full.id
        ));
    }
    found
}

/// Where two values first differ, ignoring the named members at any depth.
fn difference_between(
    left: Option<&Json>,
    right: Option<&Json>,
    ignoring: &[&str],
    at: &str,
) -> Option<String> {
    match (left, right) {
        (None, None) => None,
        (Some(left), Some(right)) => difference(left, right, ignoring, at),
        _ => Some(at.to_owned()),
    }
}

/// Where two present values first differ.
fn difference(left: &Json, right: &Json, ignoring: &[&str], at: &str) -> Option<String> {
    match (left, right) {
        (Json::Object(left), Json::Object(right)) => object_difference(left, right, ignoring, at),
        (Json::Array(left), Json::Array(right)) => {
            if left.len() != right.len() {
                return Some(at.to_owned());
            }
            left.iter()
                .zip(right.iter())
                .enumerate()
                .find_map(|(index, (left, right))| {
                    difference(left, right, ignoring, &format!("{at}[{index}]"))
                })
        },
        _ => (left != right).then(|| at.to_owned()),
    }
}

/// Where two objects first differ, comparing by name rather than by order.
fn object_difference(
    left: &[(String, Json)],
    right: &[(String, Json)],
    ignoring: &[&str],
    at: &str,
) -> Option<String> {
    let names = |members: &[(String, Json)]| -> BTreeSet<String> {
        members
            .iter()
            .map(|(key, _)| key.clone())
            .filter(|key| !ignoring.contains(&key.as_str()))
            .collect()
    };
    if names(left) != names(right) {
        return Some(at.to_owned());
    }
    for (key, value) in left {
        if ignoring.contains(&key.as_str()) {
            continue;
        }
        let other = right
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value);
        let Some(other) = other else {
            return Some(at.to_owned());
        };
        if let Some(found) = difference(value, other, ignoring, &format!("{at}.{key}")) {
            return Some(found);
        }
    }
    None
}

/// Every rule address a case names, and every address its families credit.
///
/// A family is what keeps the coverage requirement honest rather than abandoned: table
/// cells are rules, so the inventory is several thousand, and one case that exercises a
/// whole row credits the row. The dynamic half of the gate is what keeps a family from
/// crediting a cell it never reaches.
fn declared_addresses<'a>(cases: &'a [Case], inventory: &'a BTreeSet<String>) -> BTreeSet<&'a str> {
    let mut declared: BTreeSet<&str> = cases.iter().flat_map(Case::rules).collect();
    let patterns: Vec<RuleAddress> = cases
        .iter()
        .flat_map(Case::covers)
        .filter_map(parse_address)
        .collect();
    if patterns.is_empty() {
        return declared;
    }
    for rule in inventory {
        let Some(address) = parse_address(rule) else {
            continue;
        };
        if patterns
            .iter()
            .any(|pattern| pattern_names(pattern, &address))
        {
            declared.insert(rule.as_str());
        }
    }
    declared
}

/// Every address a case names that the inventory does not have.
///
/// Both halves matter and they fail in opposite directions: a rule with no case is an
/// untested claim, and a case naming an address the inventory does not have is a citation
/// to nothing. A `covers` pattern that credits nothing is the same failure in the family
/// form, so it is reported the same way.
/// Every `rules` entry below the top level of one case, in document order.
///
/// `inside` says whether the value being walked is already under an expectation, so the
/// case's own top-level `rules` — which is coverage and is subtracted — is not collected
/// twice under a different name.
fn cited_rules<'a>(value: &'a Json, inside: bool, found: &mut Vec<&'a str>) {
    match value {
        Json::Object(members) => {
            for (key, child) in members {
                if inside && key == "rules" {
                    found.extend(
                        child
                            .as_array()
                            .unwrap_or_default()
                            .iter()
                            .filter_map(Json::as_text),
                    );
                }
                cited_rules(child, inside || key == "expect", found);
            }
        },
        Json::Array(entries) => {
            for entry in entries {
                cited_rules(entry, inside, found);
            }
        },
        _ => {},
    }
}

fn unresolved_addresses(
    cases: &[Case],
    inventory: &BTreeSet<String>,
    anchors: Option<&BTreeSet<String>>,
) -> Vec<String> {
    let mut found = Vec::new();
    for case in cases {
        for rule in case.rules() {
            if !inventory.contains(rule) {
                found.push(format!(
                    "{file}: {id}: names `{rule}`, which the rule inventory does not contain",
                    file = case.file,
                    id = case.id
                ));
            }
        }
        for rule in case.cited() {
            let resolves =
                inventory.contains(rule) || anchors.is_none_or(|sections| sections.contains(rule));
            if !resolves {
                found.push(format!(
                    "{file}: {id}: cites `{rule}` inside an expectation, and it is neither an \
                     inventoried rule nor a section the rendering numbers; the field is the \
                     provenance a reader adjudicates by, and an address nothing resolves is \
                     worse there than nowhere",
                    file = case.file,
                    id = case.id
                ));
            }
        }
        for pattern in case.covers() {
            let names_something = parse_address(pattern).is_some_and(|pattern| {
                inventory
                    .iter()
                    .filter_map(|rule| parse_address(rule))
                    .any(|address| pattern_names(&pattern, &address))
            });
            if !names_something {
                found.push(format!(
                    "{file}: {id}: covers `{pattern}`, which names no inventoried rule",
                    file = case.file,
                    id = case.id
                ));
            }
        }
    }
    found
}

/// The committed schema says the same thing this gate enforces.
///
/// The schema is published so nobody else has to use our reader, which makes it a second
/// statement of one contract. This compares the two where they are mechanically
/// comparable: every field this gate requires must be required by the schema as well, so a
/// requirement added on one side and not the other fails here rather than in somebody
/// else's harness.
fn check_schema(schema: Option<&str>, has_cases: bool) -> Vec<String> {
    let Some(schema) = schema else {
        if has_cases {
            return vec![format!(
                "the suite has cases but {SCHEMA_FILE} is not committed; the schema is \
                 published so that nobody else has to use our reader"
            )];
        }
        return Vec::new();
    };
    let schema = match Json::parse(schema) {
        Ok(schema) => schema,
        Err(error) => {
            return vec![format!(
                "{SCHEMA_FILE}: {message}",
                message = error.message()
            )];
        },
    };
    let mut required = BTreeSet::new();
    collect_required(&schema, &mut required);
    let mut found = Vec::new();
    for (shape, fields) in REQUIRED_BY_SHAPE {
        for field in *fields {
            if !required.contains(*field) {
                found.push(format!(
                    "{SCHEMA_FILE} does not require `{field}` anywhere, and `conform --check` \
                     requires it of every {shape}; the published schema and this gate state \
                     one contract"
                ));
            }
        }
    }
    found
}

/// Every name any `required` array of the schema holds.
fn collect_required(value: &Json, found: &mut BTreeSet<String>) {
    match value {
        Json::Object(members) => {
            for (key, child) in members {
                if key == "required" {
                    found.extend(
                        child
                            .as_array()
                            .unwrap_or_default()
                            .iter()
                            .filter_map(Json::as_text)
                            .map(str::to_owned),
                    );
                }
                collect_required(child, found);
            }
        },
        Json::Array(entries) => {
            for entry in entries {
                collect_required(entry, found);
            }
        },
        _ => {},
    }
}

/// The largest integer an IEEE-754 double holds exactly.
///
/// Checked rather than assumed: the format promises a harness parsing a case with doubles
/// reads every number exactly, and that promise is only worth having if it is enforced.
const EXACT_INTEGER_CEILING: i64 = 1 << 53;
/// The same bound below zero.
const EXACT_INTEGER_FLOOR: i64 = -(1 << 53);
/// How deep a case file may nest. Far above anything the format needs; it exists so a
/// malformed file cannot recurse this reader past the stack.
const MAX_DEPTH: u8 = 32;

/// A JSON value, over the subset the case format uses.
///
/// A number is an integer because ADR 0005 guarantees every number in a case is one, so a
/// fraction or an exponent is a reading error naming that guarantee rather than a value
/// this type can hold.
///
/// `pub(crate)`, along with [`JsonError`] and the reading methods below: `xtask` declares no
/// dependencies (this module's own `EXACT_INTEGER_CEILING` doc states why), and
/// `xtask::attest`'s own `conformance-cases-agree-with-the-cells` checker reads the
/// published cases too, under the identical "a case file holds one object" grammar this
/// reader already enforces. Reusing this type is what keeps that a fact checked once rather
/// than a second hand-rolled reader that could disagree with the first about what a case
/// file is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Json {
    /// `null`.
    Nothing,
    /// `true` or `false`.
    Truth(bool),
    /// An integer inside 2^53.
    Integer(i64),
    /// A string.
    Text(String),
    /// An array.
    Array(Vec<Json>),
    /// An object, in the order the file writes it, with no repeated name.
    Object(Vec<(String, Json)>),
}

impl Json {
    /// Read one value, and nothing after it.
    pub(crate) fn parse(source: &str) -> Result<Self, JsonError> {
        let mut reader = Reader {
            bytes: source.as_bytes(),
            at: 0,
            line: 1,
        };
        let value = reader.value(0)?;
        reader.skip_space();
        if reader.peek().is_some() {
            return Err(reader.fault("a case file holds one object"));
        }
        Ok(value)
    }

    /// The value under `name`, when this is an object that has one.
    pub(crate) fn get(&self, name: &str) -> Option<&Self> {
        self.as_object()?
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
    }

    /// The members, when this is an object.
    pub(crate) fn as_object(&self) -> Option<&[(String, Self)]> {
        match self {
            Self::Object(members) => Some(members),
            _ => None,
        }
    }

    /// The entries, when this is an array.
    pub(crate) fn as_array(&self) -> Option<&[Self]> {
        match self {
            Self::Array(entries) => Some(entries),
            _ => None,
        }
    }

    /// The string, when this is one.
    pub(crate) fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }

    /// The integer, when this is one.
    pub(crate) fn as_integer(&self) -> Option<i64> {
        match self {
            Self::Integer(value) => Some(*value),
            _ => None,
        }
    }

    /// What kind of value this is, for a message.
    fn kind(&self) -> &'static str {
        match self {
            Self::Nothing => "null",
            Self::Truth(_) => "a boolean",
            Self::Integer(_) => "a number",
            Self::Text(_) => "a string",
            Self::Array(_) => "an array",
            Self::Object(_) => "an object",
        }
    }
}

/// Why a file is not the JSON this format accepts, and where.
///
/// `pub(crate)` because [`Json::parse`] is: an error type less visible than the function
/// that returns it is rejected by `private_interfaces` under `-D warnings`, not a choice
/// this module makes freely.
#[derive(Debug)]
pub(crate) struct JsonError {
    /// The line the reader stopped on, counted from one.
    line: usize,
    /// What was wrong there.
    reason: String,
}

impl JsonError {
    /// The message a report carries.
    fn message(&self) -> String {
        format!(
            "line {line}: {reason}",
            line = self.line,
            reason = self.reason
        )
    }
}

/// The hand-rolled reader over the subset the case format uses.
#[derive(Debug)]
struct Reader<'a> {
    /// The whole file.
    bytes: &'a [u8],
    /// How far the reader has come.
    at: usize,
    /// Which line that is, counted from one.
    line: usize,
}

impl Reader<'_> {
    /// The byte under the cursor.
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    /// Step over one byte, counting lines.
    fn bump(&mut self) {
        if self.peek() == Some(b'\n') {
            self.line = self.line.saturating_add(1);
        }
        self.at = self.at.saturating_add(1);
    }

    /// Step over the whitespace JSON allows between values.
    fn skip_space(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
            self.bump();
        }
    }

    /// A reading error at the cursor.
    fn fault(&self, reason: &str) -> JsonError {
        JsonError {
            line: self.line,
            reason: reason.to_owned(),
        }
    }

    /// Read one value.
    fn value(&mut self, depth: u8) -> Result<Json, JsonError> {
        if depth > MAX_DEPTH {
            return Err(self.fault("nests deeper than a case ever does"));
        }
        self.skip_space();
        match self.peek() {
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => self.text().map(Json::Text),
            Some(b't') => self.word("true", Json::Truth(true)),
            Some(b'f') => self.word("false", Json::Truth(false)),
            Some(b'n') => self.word("null", Json::Nothing),
            Some(b'-' | b'0'..=b'9') => self.number(),
            _ => Err(self.fault("expected a value")),
        }
    }

    /// Read one of the three bare words.
    fn word(&mut self, word: &str, value: Json) -> Result<Json, JsonError> {
        let end = self.at.saturating_add(word.len());
        if self.bytes.get(self.at..end) != Some(word.as_bytes()) {
            return Err(self.fault("expected a value"));
        }
        for _ in 0..word.len() {
            self.bump();
        }
        Ok(value)
    }

    /// Read an object, rejecting a repeated name.
    fn object(&mut self, depth: u8) -> Result<Json, JsonError> {
        self.bump();
        let mut members: Vec<(String, Json)> = Vec::new();
        self.skip_space();
        if self.peek() == Some(b'}') {
            self.bump();
            return Ok(Json::Object(members));
        }
        loop {
            self.skip_space();
            let name = self.text()?;
            if members.iter().any(|(key, _)| *key == name) {
                return Err(self.fault(&format!("names `{name}` twice")));
            }
            self.skip_space();
            if self.peek() != Some(b':') {
                return Err(self.fault("expected `:` after a name"));
            }
            self.bump();
            let value = self.value(depth.saturating_add(1))?;
            members.push((name, value));
            self.skip_space();
            match self.peek() {
                Some(b',') => self.bump(),
                Some(b'}') => {
                    self.bump();
                    return Ok(Json::Object(members));
                },
                _ => return Err(self.fault("expected `,` or `}`")),
            }
        }
    }

    /// Read an array.
    fn array(&mut self, depth: u8) -> Result<Json, JsonError> {
        self.bump();
        let mut entries = Vec::new();
        self.skip_space();
        if self.peek() == Some(b']') {
            self.bump();
            return Ok(Json::Array(entries));
        }
        loop {
            entries.push(self.value(depth.saturating_add(1))?);
            self.skip_space();
            match self.peek() {
                Some(b',') => self.bump(),
                Some(b']') => {
                    self.bump();
                    return Ok(Json::Array(entries));
                },
                _ => return Err(self.fault("expected `,` or `]`")),
            }
        }
    }

    /// Read a string, decoding the escapes JSON defines.
    fn text(&mut self) -> Result<String, JsonError> {
        if self.peek() != Some(b'"') {
            return Err(self.fault("expected a string"));
        }
        self.bump();
        let mut out = Vec::new();
        loop {
            match self.peek() {
                None => return Err(self.fault("a string is not closed")),
                Some(b'"') => {
                    self.bump();
                    return String::from_utf8(out).map_err(|_| self.fault("is not UTF-8"));
                },
                Some(b'\\') => {
                    self.bump();
                    self.escape(&mut out)?;
                },
                Some(byte) if byte < 0x20 => {
                    return Err(self.fault("a string holds a raw control character"));
                },
                Some(byte) => {
                    out.push(byte);
                    self.bump();
                },
            }
        }
    }

    /// Decode one escape, including a surrogate pair.
    fn escape(&mut self, out: &mut Vec<u8>) -> Result<(), JsonError> {
        let escape = self
            .peek()
            .ok_or_else(|| self.fault("an escape is cut off"))?;
        self.bump();
        let plain = match escape {
            b'"' => Some(b'"'),
            b'\\' => Some(b'\\'),
            b'/' => Some(b'/'),
            b'b' => Some(0x08),
            b'f' => Some(0x0C),
            b'n' => Some(b'\n'),
            b'r' => Some(b'\r'),
            b't' => Some(b'\t'),
            _ => None,
        };
        if let Some(byte) = plain {
            out.push(byte);
            return Ok(());
        }
        if escape != b'u' {
            return Err(self.fault("is not an escape JSON defines"));
        }
        let point = self.code_point()?;
        let mut buffer = [0_u8; 4];
        out.extend_from_slice(point.encode_utf8(&mut buffer).as_bytes());
        Ok(())
    }

    /// Read one `\u` escape, pairing surrogates.
    fn code_point(&mut self) -> Result<char, JsonError> {
        let first = self.hex4()?;
        if !(0xD800..0xDC00).contains(&first) {
            return char::from_u32(first).ok_or_else(|| self.fault("is not a code point"));
        }
        if self.peek() != Some(b'\\') {
            return Err(self.fault("a leading surrogate is unpaired"));
        }
        self.bump();
        if self.peek() != Some(b'u') {
            return Err(self.fault("a leading surrogate is unpaired"));
        }
        self.bump();
        let second = self.hex4()?;
        if !(0xDC00..0xE000).contains(&second) {
            return Err(self.fault("a leading surrogate is unpaired"));
        }
        let high = first
            .checked_sub(0xD800)
            .and_then(|part| part.checked_mul(0x400));
        let point = high
            .and_then(|high| high.checked_add(second.wrapping_sub(0xDC00)))
            .and_then(|part| part.checked_add(0x1_0000));
        point
            .and_then(char::from_u32)
            .ok_or_else(|| self.fault("is not a code point"))
    }

    /// Read four hexadecimal digits.
    fn hex4(&mut self) -> Result<u32, JsonError> {
        let mut value: u32 = 0;
        for _ in 0..4_u8 {
            let digit = self
                .peek()
                .and_then(|byte| char::from(byte).to_digit(16))
                .ok_or_else(|| self.fault("an escape needs four hexadecimal digits"))?;
            value = value
                .checked_mul(16)
                .and_then(|shifted| shifted.checked_add(digit))
                .ok_or_else(|| self.fault("an escape needs four hexadecimal digits"))?;
            self.bump();
        }
        Ok(value)
    }

    /// Read a number, which this format guarantees is an integer inside 2^53.
    fn number(&mut self) -> Result<Json, JsonError> {
        let start = self.at;
        if self.peek() == Some(b'-') {
            self.bump();
        }
        let digits = self.at;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.bump();
        }
        if self.at == digits {
            return Err(self.fault("expected a number"));
        }
        if matches!(self.peek(), Some(b'.' | b'e' | b'E')) {
            return Err(self.fault(
                "states a fraction or an exponent; every number in a case is an integer, \
                 which is what lets a case be compared exactly (ADR 0005)",
            ));
        }
        let literal = self
            .bytes
            .get(start..self.at)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .ok_or_else(|| self.fault("expected a number"))?;
        self.finish_number(literal)
    }

    /// Turn a number's literal text into a value, holding it to the format's guarantees.
    fn finish_number(&self, literal: &str) -> Result<Json, JsonError> {
        let digits = literal.strip_prefix('-').unwrap_or(literal);
        if digits.len() > 1 && digits.starts_with('0') {
            return Err(self.fault("states a number with a leading zero"));
        }
        let value: i64 = literal
            .parse()
            .map_err(|_| self.fault("states a number outside 2^53"))?;
        if !(EXACT_INTEGER_FLOOR..=EXACT_INTEGER_CEILING).contains(&value) {
            return Err(self.fault(
                "states a number outside 2^53, which a harness reading the case with \
                 doubles would not hold exactly",
            ));
        }
        Ok(Json::Integer(value))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::Path;

    use super::{
        Case, Json, Ledger, Milestones, REQUIRED_BY_SHAPE, RULES_INVENTORY, Reference, Suite,
        accept_arguments, check_input, check_permitted, check_question, check_schema, check_trims,
        check_widow_threshold, declared_addresses, declared_units_per_em, deferral, examine_file,
        examine_protocol_suite, frame_pairs, inventory_column, is_path, parse_address, unique_ids,
        unresolved_addresses,
    };
    use crate::shared;

    /// The worked case of `docs/design/conformance.md`, verbatim.
    ///
    /// It is the design document's own statement of what a correct case looks like, so it
    /// is the one fixture that must pass every check unchanged, and it is carried here
    /// rather than read from a file so that this gate's meaning cannot drift from the
    /// document that fixed it. Every negative case below is one mutation away from a shape
    /// this permits.
    const WORKED_CASE: &str = r#"{
  "$schema": "../cases.schema.json",
  "section": "3.1.9",
  "heading_en": "Positioning of Closing Brackets, Full Stops, Commas and Middle Dots at Line End",
  "heading_ja": "行末に配置する終わり括弧類，句点類，読点類及び中点類の配置方法",
  "cases": [
    {
      "id": "3.1.9/closing-bracket-at-line-end/half-em-frame",
      "rules": ["3.1.2", "3.1.9", "B.2#2"],
      "standing": "alternative",
      "quote": "In principle, closing brackets (cl-02), commas (cl-07) or full stops (cl-06) at the line end have half em spacing after them. This half em spacing can be removed for line adjustment. However, the possibilities are only half em spacing or solid. Other spacing, such as quarter em spacing should not be used.",
      "rationale": "The caller declares the half em frame, so the conditional space after the bracket is added to the supplied advance. JLReq keeps it at the line end and JIS X 4051 sets solid; both are permitted and the intermediate quarter em is forbidden by the quoted sentence, which states the prohibition twice.",
      "input": {
        "kind": "compose",
        "text": "あい」",
        "direction": "horizontal",
        "scales": [{ "inline_em": 1000, "block_em": 1000 }],
        "items": [
          { "start": 0, "advance": 1000, "frame": "full-em", "scale": 0 },
          { "start": 3, "advance": 1000, "frame": "full-em", "scale": 0 },
          { "start": 6, "advance":  500, "frame": "half-em", "scale": 0 }
        ],
        "candidates": [{ "at": 0 }, { "at": 3 }, { "at": 6 }, { "at": 9 }],
        "measure": 3000
      },
      "permitted": [
        {
          "policy": {},
          "source": "JLReq preferred (B.2#2)",
          "expect": {
            "lines": [
              {
                "placements": [0, 1000, 2000],
                "trims": [],
                "trailing": { "em": [1, 2], "units": 360, "resolved": 500 },
                "extent": 3000
              }
            ],
            "violations": []
          }
        },
        {
          "policy": { "spacing.line_end_punctuation": "solid" },
          "source": "JIS X 4051 (3.1.9, Figure 77)",
          "expect": {
            "lines": [
              {
                "placements": [0, 1000, 2000],
                "trims": [],
                "trailing": { "em": [0, 1], "units": 0, "resolved": 0 },
                "extent": 2500
              }
            ],
            "violations": []
          }
        }
      ],
      "forbidden": [
        {
          "expect": { "lines": [{ "trailing": { "em": [1, 4] } }] },
          "why": "3.1.9: 'the possibilities are only half em spacing or solid. Other spacing, such as quarter em spacing should not be used.'"
        }
      ],
      "disagreements": []
    },
    {
      "id": "3.1.9/closing-bracket-at-line-end/full-em-frame",
      "rules": ["3.1.2", "3.1.9", "B.2#2"],
      "standing": "normative",
      "quote": "The character advance of commas (cl-07), full stops (cl-06), opening brackets (cl-01), closing brackets (cl-02) and middle dots (cl-05) is half-width (half em). But when those punctuation marks are placed side-by-side with ideographic (cl-19), hiragana (cl-15), or katakana (cl-16) characters, in principle, a given amount of spacing will be inserted before or after the symbols, which makes them appear as if they were intrinsically full-width (one em).",
      "rationale": "The identical text, with the bracket declared on the ideographic frame — the advance a modern OpenType font reports. The conditional space is already inside that advance, so composition trims it out and reports the trim, and the line is stated in the same normalized geometry as the half-em case. Every expected value below is therefore byte-identical to the case above and only `trims` differs, which is the whole assertion: an implementation that adds the Table 1 amount to a full-em advance overshoots by half an em at the commonest adjacency in Japanese text, and one that shortens the advance silently produces the right extent with no evidence.",
      "input": {
        "kind": "compose",
        "text": "あい」",
        "direction": "horizontal",
        "scales": [{ "inline_em": 1000, "block_em": 1000 }],
        "items": [
          { "start": 0, "advance": 1000, "frame": "full-em", "scale": 0 },
          { "start": 3, "advance": 1000, "frame": "full-em", "scale": 0 },
          { "start": 6, "advance": 1000, "frame": "full-em", "scale": 0 }
        ],
        "candidates": [{ "at": 0 }, { "at": 3 }, { "at": 6 }, { "at": 9 }],
        "measure": 3000
      },
      "permitted": [
        {
          "policy": {},
          "source": "JLReq preferred (B.2#2)",
          "expect": {
            "lines": [
              {
                "placements": [0, 1000, 2000],
                "trims": [
                  { "item": 2, "em": [1, 2], "units": 360, "resolved": 500, "referent": "preceding", "rule": "3.1.2" }
                ],
                "trailing": { "em": [1, 2], "units": 360, "resolved": 500 },
                "extent": 3000
              }
            ],
            "violations": []
          }
        },
        {
          "policy": { "spacing.line_end_punctuation": "solid" },
          "source": "JIS X 4051 (3.1.9, Figure 77)",
          "expect": {
            "lines": [
              {
                "placements": [0, 1000, 2000],
                "trims": [
                  { "item": 2, "em": [1, 2], "units": 360, "resolved": 500, "referent": "preceding", "rule": "3.1.2" }
                ],
                "trailing": { "em": [0, 1], "units": 0, "resolved": 0 },
                "extent": 2500
              }
            ],
            "violations": []
          }
        }
      ],
      "forbidden": [
        {
          "expect": { "lines": [{ "extent": 3500 }] },
          "why": "Adding the Table 1 half em to an advance that already contains it. 3.1.2 states the bracket's own advance is half-width and that the amount is what makes it appear full-width, so it cannot be both."
        },
        {
          "expect": { "lines": [{ "extent": 2500, "trims": [] }] },
          "why": "Trimming the caller's advance and not saying so. ADR-0002 makes the supplied advance the caller's; a unit taken out of one is reported with the sentence that took it."
        }
      ],
      "disagreements": []
    }
  ]
}"#;

    /// The smallest case this gate accepts, so a negative test mutates exactly one thing.
    const MINIMAL: &str = r#"{
      "id": "3.1.9/minimal/one",
      "rules": ["3.1.9"],
      "standing": "normative",
      "quote": "In principle, closing brackets at the line end have half em spacing after them.",
      "rationale": "The smallest well-formed case, so a test can malform exactly one thing.",
      "input": {
        "kind": "compose",
        "text": "あ",
        "direction": "horizontal",
        "scales": [{ "inline_em": 1000, "block_em": 1000 }],
        "items": [{ "start": 0, "advance": 1000, "frame": "full-em", "scale": 0 }],
        "candidates": [{ "at": 0 }, { "at": 3 }],
        "measure": 1000
      },
      "permitted": [
        {
          "policy": {},
          "source": "JLReq preferred",
          "expect": {
            "lines": [
              {
                "placements": [0],
                "trims": [],
                "trailing": { "em": [0, 1], "units": 0, "resolved": 0 },
                "extent": 1000
              }
            ],
            "violations": []
          }
        }
      ]
    }"#;

    /// The smallest `feasible` case this gate accepts, so the census test below mutates
    /// exactly one thing: `input.kind`.
    const MINIMAL_FEASIBLE: &str = r#"{
      "id": "3.1.9/minimal-feasible/one",
      "rules": ["C.2#13"],
      "standing": "normative",
      "quote": "There is no line break opportunity between two consecutive characters belonging to the same set of characters in tate-chu-yoko (cl-30).",
      "rationale": "The smallest well-formed feasible case, so the census test mutates exactly one thing.",
      "input": {
        "kind": "feasible",
        "text": "あ",
        "direction": "horizontal",
        "scales": [{ "inline_em": 1000, "block_em": 1000 }],
        "items": [{ "start": 0, "advance": 1000, "frame": "full-em", "scale": 0 }],
        "candidates": [{ "at": 0 }]
      },
      "permitted": [
        {
          "policy": {},
          "source": "JLReq preferred",
          "expect": {
            "feasible": { "candidate": 0, "breakable": true }
          }
        }
      ]
    }"#;

    /// The smallest `lower` case this gate accepts, so the census test below mutates
    /// exactly one thing: `input.kind`.
    const MINIMAL_LOWER: &str = r#"{
      "id": "3.3.5/minimal-lower/one",
      "rules": ["3.3.5"],
      "standing": "normative",
      "quote": "This positioning of a ruby character is called 'nakatsuki' (center-alignment).",
      "rationale": "The smallest well-formed lower case, so the census test mutates exactly one thing.",
      "input": {
        "kind": "lower",
        "text": "鬼",
        "direction": "horizontal",
        "scales": [{ "inline_em": 1000, "block_em": 1000 }],
        "items": [{ "start": 0, "advance": 1000, "frame": "full-em", "scale": 0 }],
        "annotations": [
          {
            "text": "き",
            "scales": [{ "inline_em": 500, "block_em": 500 }],
            "items": [{ "start": 0, "advance": 500, "frame": "full-em", "scale": 0 }]
          }
        ],
        "constructs": {
          "ruby": [
            {
              "base": [0, 1],
              "annotation": 0,
              "style": "mono",
              "runs": [{ "base": [0, 1], "annotation": [0, 1] }]
            }
          ]
        }
      },
      "permitted": [
        {
          "policy": {},
          "source": "JLReq preferred",
          "expect": {
            "lower": { "construct": 0, "alignment": "nakatsuki" }
          }
        }
      ]
    }"#;

    /// The trim the full-em case of a frame pair reports.
    const FULL_EM_TRIMS: &str = r#"[{ "item": 1, "em": [1, 2], "units": 360,
        "resolved": 500, "referent": "preceding", "rule": "3.1.2" }]"#;

    /// The em denominator ADR 0007 fixes, and no generated policy space.
    fn plain() -> Reference<'static> {
        Reference {
            units_per_em: Some(720),
            questions: None,
        }
    }

    /// Read one fixture as the case file of §3.1.9.
    fn examine(source: &str) -> Vec<String> {
        examine_file("cases/3.1.9.json", source, plain()).0
    }

    /// Wrap one case body in the file that holds it.
    fn file_with(case: &str) -> String {
        format!("{{ \"section\": \"3.1.9\", \"cases\": [{case}] }}")
    }

    /// Read a fixture fragment, which is JSON by construction.
    fn json(source: &str) -> Json {
        Json::parse(source).expect("the fixture is JSON")
    }

    /// An inventory of three rules: one section and one Table 1 row.
    fn inventory() -> BTreeSet<String> {
        ["3.1.9", "B.1@cl-05,cl-05", "B.1@cl-05,cl-19"]
            .iter()
            .map(|address| (*address).to_owned())
            .collect()
    }

    /// A suite with nothing generated behind it and nothing deferred.
    fn bare_suite() -> Suite {
        Suite {
            directory_exists: false,
            files: Vec::new(),
            protocol_cases: None,
            protocol_schema: None,
            schema: None,
            rules: None,
            questions: None,
            anchors: None,
            units_per_em: Some(720),
            deferrals: Ledger::default(),
            milestones: Milestones::default(),
        }
    }

    /// A ledger deferring one rule of `inventory` to a milestone.
    fn deferring(rule: &str, milestone: &str) -> Ledger {
        Ledger::of(&format!(
            "[[deferred]]\nrule = \"{rule}\"\nmilestone = \"{milestone}\"\n\
             why = \"The fixture's reason.\"\n"
        ))
    }

    /// The cases of one fixture file.
    fn cases_of(source: &str) -> Vec<Case> {
        examine_file("cases/3.1.9.json", source, plain()).1
    }

    #[test]
    fn the_worked_case_of_the_design_document_validates() {
        let (found, cases) = examine_file("cases/3.1.9.json", WORKED_CASE, plain());
        assert!(found.is_empty(), "{found:#?}");
        assert_eq!(cases.len(), 2, "the worked case is a pair");
        assert!(unique_ids(&cases).is_empty());
        assert!(frame_pairs(&cases).is_empty(), "the pair agrees");
    }

    #[test]
    fn the_minimal_case_validates() {
        assert!(examine(&file_with(MINIMAL)).is_empty());
    }

    #[test]
    fn protocol_suite_rules_enter_the_same_coverage_set() {
        let source = r#"{"protocol":"kumihan.conformance/1","spec":"jlreq-2020-08-11+unicode-17.0.0","id":"3.3.9/emphasis","rules":["3.3.9"],"request":{},"expected":{}}"#;
        let (found, cases) = examine_protocol_suite("suite.ndjson", source);
        assert!(found.is_empty(), "{found:#?}");
        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].rules(), ["3.3.9"]);

        let missing = source.replace(r#","rules":["3.3.9"]"#, "");
        let (found, _) = examine_protocol_suite("suite.ndjson", &missing);
        assert!(
            found
                .iter()
                .any(|message| message.contains("non-empty rules")),
            "{found:#?}"
        );
    }

    #[test]
    fn protocol_suite_rejects_carriage_returns() {
        let source = concat!(
            "{\"protocol\":\"kumihan.conformance/1\",",
            "\"spec\":\"jlreq-2020-08-11+unicode-17.0.0\",",
            "\"id\":\"external/crlf\",\"rules\":[\"3.1.9\"],",
            "\"request\":{},\"expected\":{}}\r\n",
        );
        let (found, _) = examine_protocol_suite("suite.ndjson", source);
        assert!(
            found.iter().any(|message| message.contains("LF only")),
            "a published byte-exact suite must reject CRLF: {found:#?}"
        );
    }

    #[test]
    fn protocol_coverage_requires_every_observable_rule_in_the_external_suite() {
        let legacy = cases_of(&file_with(MINIMAL));
        let source = r#"{"protocol":"kumihan.conformance/1","spec":"jlreq-2020-08-11+unicode-17.0.0","id":"external/one","rules":["3.1.9"],"request":{},"expected":{}}"#;
        let (found, protocol) = examine_protocol_suite("suite.ndjson", source);
        assert!(found.is_empty(), "{found:#?}");

        let mut suite = bare_suite();
        suite.rules = Some(inventory());
        suite.protocol_cases = Some(source.to_owned());
        let cases: Vec<_> = legacy.into_iter().chain(protocol).collect();
        let found = suite.protocol_coverage(&cases);
        assert!(
            found
                .iter()
                .any(|message| message.contains("2 observable inventoried rule(s)")),
            "the retained Rust-only case must not satisfy the external contract: {found:#?}"
        );

        let complete = source.replace(
            r#"["3.1.9"]"#,
            r#"["3.1.9","B.1@cl-05,cl-05","B.1@cl-05,cl-19"]"#,
        );
        let (found, protocol) = examine_protocol_suite("suite.ndjson", &complete);
        assert!(found.is_empty(), "{found:#?}");
        suite.protocol_cases = Some(complete);
        assert!(suite.protocol_coverage(&protocol).is_empty());
    }

    #[test]
    fn a_permitted_entry_without_a_policy_is_rejected() {
        let broken = MINIMAL.replace("\"policy\": {},", "");
        let found = examine(&file_with(&broken));
        assert!(
            found
                .iter()
                .any(|message| message.contains("has no `policy`")),
            "{found:#?}"
        );
    }

    #[test]
    fn two_permitted_entries_that_are_not_ordered_by_inclusion_are_rejected() {
        let permitted = json(
            r#"[
              { "policy": { "kinsoku.level": "strict" }, "source": "a",
                "expect": { "lines": [] } },
              { "policy": { "spacing.line_end_punctuation": "solid" }, "source": "b",
                "expect": { "lines": [] } }
            ]"#,
        );
        let found = check_permitted(Some(&permitted), plain());
        assert!(
            found
                .iter()
                .any(|message| message.contains("not ordered by inclusion")),
            "{found:#?}"
        );
    }

    #[test]
    fn two_permitted_entries_with_one_overlay_are_rejected_and_a_chain_is_not() {
        let same = json(
            r#"[
              { "policy": { "kinsoku.level": "strict" }, "source": "a",
                "expect": { "lines": [] } },
              { "policy": { "kinsoku.level": "strict" }, "source": "b",
                "expect": { "lines": [] } }
            ]"#,
        );
        let found = check_permitted(Some(&same), plain());
        assert!(
            found
                .iter()
                .any(|message| message.contains("declare the same overlay")),
            "{found:#?}"
        );
        let chain = json(
            r#"[
              { "policy": {}, "source": "a", "expect": { "lines": [] } },
              { "policy": { "kinsoku.level": "strict" }, "source": "b",
                "expect": { "lines": [] } },
              { "policy": { "kinsoku.level": "loose" }, "source": "c",
                "expect": { "lines": [] } }
            ]"#,
        );
        assert!(
            check_permitted(Some(&chain), plain()).is_empty(),
            "two readings of one question are a chain, not an ambiguity"
        );
    }

    #[test]
    fn a_case_recording_a_silence_with_one_reading_is_rejected() {
        for standing in ["alternative", "unstated", "adjudicated"] {
            let broken = MINIMAL.replace(
                "\"standing\": \"normative\"",
                &format!("\"standing\": \"{standing}\""),
            );
            let found = examine(&file_with(&broken));
            assert!(
                found
                    .iter()
                    .any(|message| message.contains("records 1 reading(s)")),
                "{standing}: {found:#?}"
            );
        }
    }

    #[test]
    fn a_fraction_that_disagrees_with_its_unit_count_is_rejected() {
        let broken = MINIMAL.replace(
            "\"em\": [0, 1], \"units\": 0",
            "\"em\": [1, 2], \"units\": 300",
        );
        let found = examine(&file_with(&broken));
        assert!(
            found
                .iter()
                .any(|message| message.contains("1/2 of an em is 360 units")),
            "{found:#?}"
        );
        let unstatable = MINIMAL.replace(
            "\"em\": [0, 1], \"units\": 0",
            "\"em\": [1, 7], \"units\": 103",
        );
        let found = examine(&file_with(&unstatable));
        assert!(
            found
                .iter()
                .any(|message| message.contains("cannot state exactly")),
            "{found:#?}"
        );
    }

    #[test]
    fn an_amount_needs_its_unit_count_outside_forbidden_and_not_inside() {
        let bare = MINIMAL.replace("\"em\": [0, 1], \"units\": 0, ", "\"em\": [0, 1], ");
        let found = examine(&file_with(&bare));
        assert!(
            found
                .iter()
                .any(|message| message.contains("states `em` and no `units`")),
            "{found:#?}"
        );
        let pattern = MINIMAL.replace(
            "\"permitted\": [",
            "\"forbidden\": [{ \"expect\": { \"lines\": [{ \"trailing\": { \"em\": [1, 4] } }] }, \
             \"why\": \"3.1.9 permits only a half em or solid.\" }], \"permitted\": [",
        );
        assert!(
            examine(&file_with(&pattern)).is_empty(),
            "a forbidden expectation is a pattern rather than a value"
        );
    }

    #[test]
    fn a_number_the_format_does_not_permit_is_refused_by_the_reader() {
        let fraction = Json::parse("{ \"a\": 1.5 }")
            .err()
            .map(|error| error.message());
        assert!(
            fraction
                .as_ref()
                .is_some_and(|message| message.contains("integer")),
            "{fraction:?}"
        );
        let huge = Json::parse("{ \"a\": 9007199254740993 }")
            .err()
            .map(|error| error.message());
        assert!(
            huge.as_ref()
                .is_some_and(|message| message.contains("2^53")),
            "{huge:?}"
        );
        assert!(Json::parse("{ \"a\": 9007199254740992 }").is_ok());
    }

    #[test]
    fn an_id_that_does_not_name_its_own_file_is_rejected() {
        let broken = MINIMAL.replace("3.1.9/minimal/one", "3.1.2/minimal/one");
        let found = examine(&file_with(&broken));
        assert!(
            found
                .iter()
                .any(|message| message.contains("is not the file's `3.1.9`")),
            "{found:#?}"
        );
        let shapeless = MINIMAL.replace("3.1.9/minimal/one", "3.1.9/minimal");
        let found = examine(&file_with(&shapeless));
        assert!(
            found
                .iter()
                .any(|message| message.contains("<section>/<subject>/<variant>")),
            "{found:#?}"
        );
    }

    #[test]
    fn one_id_used_twice_is_rejected() {
        let mut cases = cases_of(&file_with(MINIMAL));
        cases.extend(examine_file("cases/other.json", &file_with(MINIMAL), plain()).1);
        let found = unique_ids(&cases);
        assert!(
            found
                .iter()
                .any(|message| message.contains("is used 2 times")),
            "{found:#?}"
        );
    }

    #[test]
    fn an_address_outside_the_grammar_is_rejected() {
        assert!(parse_address("3.1.9").is_some());
        assert!(parse_address("B.2#3").is_some());
        assert!(parse_address("B.1@cl-05,cl-05").is_some());
        assert!(parse_address("B.1@cl-05,line-end").is_some());
        assert!(
            parse_address("H.1").is_none(),
            "JLReq's appendices are A to G"
        );
        assert!(parse_address("3.1.9#").is_none(), "a note has an ordinal");
        assert!(
            parse_address("B.2#0").is_none(),
            "JLReq numbers notes from one"
        );
        assert!(
            parse_address("B.1@cl-05").is_none(),
            "a cell has two coordinates"
        );
        assert!(parse_address("3..9").is_none());
        assert!(parse_address("").is_none());
        // The grammar this gate reads is the library's, so the shapes `jlreq-spec` and
        // `spec-links` refuse are refused here too. Accepting a wider language would let
        // a case file name a cell no inventory row can carry while the coverage
        // subtraction still closed (ADR 0013).
        assert!(
            parse_address("B.1@cl-5,cl-05").is_none(),
            "JLReq pads a class to two digits"
        );
        assert!(
            parse_address("B.1@cl-31,cl-05").is_none(),
            "§3.9.2 closes the set at thirty"
        );
        assert!(
            parse_address("B.1@cl-00,cl-05").is_none(),
            "there is no class zero"
        );
        assert!(parse_address("B.1@banana,cl-05").is_none());
        assert!(parse_address("3.01").is_none(), "no leading zero");
        assert!(
            parse_address("B.1@cl-02,line-head").is_none(),
            "the line head is a row and the line end is a column"
        );
        assert!(parse_address("B.1@line-end,cl-05").is_none());
        assert!(
            parse_address("B.1@cl-05,*").is_some(),
            "a family pattern is written in the address space and adds only the wildcard"
        );
        assert!(parse_address("B.1@*,cl-05").is_some());
        let broken = MINIMAL.replace("[\"3.1.9\"]", "[\"H.9\"]");
        let found = examine(&file_with(&broken));
        assert!(
            found
                .iter()
                .any(|message| message.contains("is not a specification address")),
            "{found:#?}"
        );
    }

    #[test]
    fn a_cells_entry_names_a_real_table_and_axis() {
        let with_cells = |cells: &str| {
            MINIMAL.replacen(
                "\"rules\": [\"3.1.9\"],",
                &format!("\"rules\": [\"3.1.9\"], \"cells\": {cells},"),
                1,
            )
        };
        assert!(
            examine(&file_with(&with_cells(
                r#"[{ "table": 1, "before": "cl-02", "after": "line-end" }]"#
            )))
            .is_empty(),
            "a well-formed coordinate is not this gate's own business to reject"
        );

        let bad_table = examine(&file_with(&with_cells(
            r#"[{ "table": 7, "before": "cl-02", "after": "line-end" }]"#,
        )));
        assert!(
            bad_table
                .iter()
                .any(|line| line.contains("the matrices are 1 through 6")),
            "{bad_table:#?}"
        );

        let bad_axis = examine(&file_with(&with_cells(
            r#"[{ "table": 1, "before": "cl-99", "after": "line-end" }]"#,
        )));
        assert!(
            bad_axis
                .iter()
                .any(|line| line.contains("`before` is `cl-99`")),
            "{bad_axis:#?}"
        );

        let wrong_side = examine(&file_with(&with_cells(
            r#"[{ "table": 1, "before": "line-end", "after": "cl-02" }]"#,
        )));
        assert!(
            wrong_side
                .iter()
                .any(|line| line.contains("`before` is `line-end`")),
            "line-end is the column axis's own label, not the row's: {wrong_side:#?}"
        );

        let no_line_edges = examine(&file_with(&with_cells(
            r#"[{ "table": 2, "before": "line-head", "after": "cl-02" }]"#,
        )));
        assert!(
            no_line_edges
                .iter()
                .any(|line| line.contains("has no `line-head` row")),
            "Table 2 has no line-edge axis at all (§C.1): {no_line_edges:#?}"
        );

        let empty = examine(&file_with(&with_cells("[]")));
        assert!(
            empty.iter().any(|line| line.contains("`cells` is empty")),
            "{empty:#?}"
        );
    }

    #[test]
    fn an_item_off_a_character_boundary_is_rejected() {
        let broken = MINIMAL.replace("\"start\": 0", "\"start\": 1");
        let found = examine(&file_with(&broken));
        assert!(
            found
                .iter()
                .any(|message| message.contains("not a character boundary")),
            "{found:#?}"
        );
    }

    #[test]
    fn a_boundary_case_must_name_the_same_edge_in_every_expectation() {
        // The hole `check_question`'s own ordinal check already closes for `before`, opened
        // a second time the moment `edge` existed: without this, an entry naming `edge:
        // "end"` and a later one naming no edge at all would both be measured against the
        // one answer the first entry's `before` and `edge` select, silently agreeing with a
        // line-end reading its own text never asked about.
        let consistent = json(
            r#"{ "input": { "kind": "boundary" }, "permitted": [
                { "expect": { "boundary": { "before": 0, "edge": "end" } } },
                { "expect": { "boundary": { "before": 0, "edge": "end" } } }
            ] }"#,
        );
        assert!(check_question(&consistent).is_empty(), "{consistent:?}");

        let mismatched = json(
            r#"{ "input": { "kind": "boundary" }, "permitted": [
                { "expect": { "boundary": { "before": 0, "edge": "end" } } },
                { "expect": { "boundary": { "before": 0 } } }
            ] }"#,
        );
        let found = check_question(&mismatched);
        assert!(
            found
                .iter()
                .any(|message| message.contains("expect.boundary.edge")),
            "{found:#?}"
        );
    }

    #[test]
    fn an_ordinal_that_indexes_the_wrong_stream_is_rejected() {
        let sound = json(&ruby_input("[0, 1]"));
        assert!(
            check_input(Some(&sound)).is_empty(),
            "the ruby is well formed"
        );
        let swapped = json(&ruby_input("[0, 2]"));
        let found = check_input(Some(&swapped));
        assert!(
            found.iter().any(|message| message.contains("runs[0].base")),
            "{found:#?}"
        );
    }

    #[test]
    fn a_widow_threshold_is_bounded_at_a_u16() {
        // Mirrors `check_search`'s own bound on `tolerance`: a threshold this reader cannot
        // hold as a `u16` — negative, or past `u16::MAX` — is refused rather than silently
        // declined by `kumihan.rs`'s own `u16::try_from(threshold).ok()?` at runtime, the
        // failure mode this shape check exists to close (round 22's own brief).
        let negative = json(r#"{ "widow_threshold": -1 }"#);
        let found = check_widow_threshold(&negative);
        assert!(
            found
                .iter()
                .any(|message| message.contains("widow_threshold")),
            "{found:#?}"
        );
        let past_u16 = json(r#"{ "widow_threshold": 65536 }"#);
        let found = check_widow_threshold(&past_u16);
        assert!(
            found
                .iter()
                .any(|message| message.contains("widow_threshold")),
            "{found:#?}"
        );
        // `0` is `widow_threshold`'s own documented no-op (`cases.schema.json`'s own
        // description) and not a value this check refuses, unlike `check_positive`'s own
        // bound on `measure`.
        let zero = json(r#"{ "widow_threshold": 0 }"#);
        assert!(
            check_widow_threshold(&zero).is_empty(),
            "0 is the documented no-op"
        );
        let max = json(r#"{ "widow_threshold": 65535 }"#);
        assert!(
            check_widow_threshold(&max).is_empty(),
            "u16::MAX is the top of the range `Paragraph::with_widow_threshold` can hold"
        );
        let absent = json(r"{}");
        assert!(
            check_widow_threshold(&absent).is_empty(),
            "a case stating no threshold at all asserts nothing to check"
        );
    }

    /// One base item with a two-item reading attached, and the run's base range stated.
    fn ruby_input(base_run: &str) -> String {
        format!(
            r#"{{
              "kind": "compose",
              "text": "漢",
              "scales": [{{ "inline_em": 1000, "block_em": 1000 }}],
              "items": [{{ "start": 0, "advance": 1000, "scale": 0 }}],
              "measure": 1000,
              "candidates": [{{ "at": 0 }}],
              "annotations": [
                {{ "text": "かん", "scales": [{{ "inline_em": 500, "block_em": 500 }}],
                   "items": [{{ "start": 0, "advance": 500, "scale": 0 }},
                             {{ "start": 3, "advance": 500, "scale": 0 }}] }}
              ],
              "constructs": {{
                "ruby": [{{ "base": [0, 1], "annotation": 0, "style": "mono",
                            "runs": [{{ "base": {base_run}, "annotation": [0, 2] }}] }}]
              }}
            }}"#
        )
    }

    #[test]
    fn a_trim_names_the_sentence_that_took_the_unit() {
        let stated = json(
            r#"[{ "item": 1, "em": [1, 2], "units": 360, "referent": "preceding",
                  "rule": "3.1.2" }]"#,
        );
        assert!(check_trims(&stated, "trims", false).is_empty());
        let celled = json(r#"[{ "item": 1, "referent": "trailing", "rule": "B.1@cl-05,cl-05" }]"#);
        assert!(check_trims(&celled, "trims", false).is_empty());
        let invented = json(r#"[{ "item": 1, "referent": "preceding", "rule": "3.8.1" }]"#);
        let found = check_trims(&invented, "trims", false);
        assert!(
            found.iter().any(|message| message.contains("is `3.8.1`")),
            "{found:#?}"
        );
        let anonymous = json(r#"[{ "item": 1, "referent": "preceding" }]"#);
        assert!(!check_trims(&anonymous, "trims", false).is_empty());
        assert!(
            check_trims(&anonymous, "trims", true).is_empty(),
            "a forbidden expectation states only what it forbids"
        );
        let owned_by_nobody = json(r#"[{ "item": 1, "referent": "either", "rule": "3.1.2" }]"#);
        assert!(!check_trims(&owned_by_nobody, "trims", false).is_empty());
    }

    #[test]
    fn a_field_the_format_does_not_have_is_rejected() {
        let broken = MINIMAL.replace("\"rules\":", "\"rule\":");
        let found = examine(&file_with(&broken));
        assert!(
            found
                .iter()
                .any(|message| message.contains("unknown field `rule`")),
            "{found:#?}"
        );
        assert!(
            found
                .iter()
                .any(|message| message.contains("has no `rules`")),
            "{found:#?}"
        );
    }

    #[test]
    fn a_case_file_written_with_crlf_is_rejected() {
        let found = examine("{ \"section\": \"3.1.9\",\r\n \"cases\": [] }");
        assert!(
            found.iter().any(|message| message.contains("holds a CR")),
            "{found:#?}"
        );
        assert!(
            found
                .iter()
                .any(|message| message.contains("holds no case")),
            "{found:#?}"
        );
    }

    #[test]
    fn a_frame_pair_differing_outside_trims_is_rejected() {
        let agreeing = frame_pair(2000, FULL_EM_TRIMS);
        let (found, cases) = examine_file("cases/3.1.2.json", &agreeing, plain());
        assert!(found.is_empty(), "{found:#?}");
        assert!(frame_pairs(&cases).is_empty(), "the pair agrees");

        let differing = frame_pair(2400, FULL_EM_TRIMS);
        let found = frame_pairs(&examine_file("cases/3.1.2.json", &differing, plain()).1);
        assert!(
            found
                .iter()
                .any(|message| message.contains("permitted[0].expect.lines[0].extent")),
            "{found:#?}"
        );
    }

    #[test]
    fn a_frame_case_without_its_pair_is_rejected() {
        let lone = frame_pair(2000, FULL_EM_TRIMS).replace("full-em-frame", "solid-frame");
        let found = frame_pairs(&examine_file("cases/3.1.2.json", &lone, plain()).1);
        assert!(
            found
                .iter()
                .any(|message| message.contains("has no `3.1.2/bracket-at-line-end/full-em-frame`")),
            "{found:#?}"
        );
    }

    /// The two cases of a §3.1.2 frame pair, as one file.
    fn frame_pair(full_extent: i64, full_trims: &str) -> String {
        format!(
            "{{ \"section\": \"3.1.2\", \"cases\": [{half}, {full}] }}",
            half = frame_case("half-em-frame", 500, "half-em", 2000, "[]"),
            full = frame_case("full-em-frame", 1000, "full-em", full_extent, full_trims)
        )
    }

    /// One case of a frame pair. The two differ in the frame, the advance it describes,
    /// and the trims that follow from it, and in nothing else.
    fn frame_case(variant: &str, advance: i64, frame: &str, extent: i64, trims: &str) -> String {
        format!(
            r#"{{
              "id": "3.1.2/bracket-at-line-end/{variant}",
              "rules": ["3.1.2"],
              "standing": "normative",
              "quote": "The character advance of closing brackets (cl-02) is half-width.",
              "rationale": "The same geometry reached from two directions (ADR 0017).",
              "input": {{
                "kind": "compose",
                "text": "あ」",
                "scales": [{{ "inline_em": 1000, "block_em": 1000 }}],
                "items": [
                  {{ "start": 0, "advance": 1000, "frame": "full-em", "scale": 0 }},
                  {{ "start": 3, "advance": {advance}, "frame": "{frame}", "scale": 0 }}
                ],
                "candidates": [{{ "at": 0 }}, {{ "at": 3 }}, {{ "at": 6 }}],
                "measure": 2000
              }},
              "permitted": [
                {{
                  "policy": {{}},
                  "source": "JLReq preferred (B.2#2)",
                  "expect": {{
                    "lines": [
                      {{
                        "placements": [0, 1000],
                        "trims": {trims},
                        "trailing": {{ "em": [1, 2], "units": 360, "resolved": 500 }},
                        "extent": {extent}
                      }}
                    ],
                    "violations": []
                  }}
                }}
              ]
            }}"#
        )
    }

    #[test]
    fn historical_empty_inputs_do_not_impersonate_the_protocol_suite() {
        let suite = bare_suite();
        assert!(suite.coverage(&[]).is_empty());
        let census = suite.census(&[]);
        assert!(
            census
                .iter()
                .any(|line| line.contains("has not been generated")),
            "{census:#?}"
        );
        assert!(
            census
                .iter()
                .any(|line| line.contains("protocol-v1 is the authoritative suite")),
            "{census:#?}"
        );
    }

    #[test]
    fn a_case_written_before_the_inventory_cannot_be_checked_and_fails() {
        let found = bare_suite().coverage(&cases_of(&file_with(MINIMAL)));
        assert!(
            found
                .iter()
                .any(|message| message.contains("has not been generated")),
            "{found:#?}"
        );
    }

    #[test]
    fn coverage_subtracts_in_both_directions() {
        let mut suite = bare_suite();
        suite.rules = Some(inventory());
        let cases = cases_of(&file_with(MINIMAL));
        let found = suite.coverage(&cases);
        assert!(
            found
                .iter()
                .any(|message| message
                    .contains("2 inventoried rule(s) have neither a conformance case")),
            "{found:#?}"
        );
        let unknown = MINIMAL.replace("[\"3.1.9\"]", "[\"3.1.9\", \"B.2#4\"]");
        let found = unresolved_addresses(&cases_of(&file_with(&unknown)), &inventory(), None);
        assert!(
            found
                .iter()
                .any(|message| message.contains("the rule inventory does not contain")),
            "{found:#?}"
        );
    }

    #[test]
    fn a_suite_that_has_not_been_started_defers_coverage_and_names_it() {
        let mut suite = bare_suite();
        suite.rules = Some(inventory());
        assert!(
            suite.coverage(&[]).is_empty(),
            "an inventory subtracted from a suite that does not exist is not a finding"
        );
        let census = suite.census(&[]);
        assert!(
            census
                .iter()
                .any(|line| line.contains("declared coverage: did not run")
                    && line.contains("3 inventoried rule(s)")),
            "and the run says which check it could not make, and over how many rules: \
             {census:#?}"
        );
    }

    #[test]
    fn the_kind_census_line_counts_a_feasible_case_by_its_own_kind() {
        // Round 20's `optimal_search_census` precedent, applied to the kind this round adds:
        // a case naming `"kind": "feasible"` is counted under `feasible` in the same line
        // `classify`, `boundary`, `compose`, `align` and `tab` already are, not left silently
        // uncounted the way an unrecognized kind used to be able to.
        let cases = cases_of(&file_with(MINIMAL_FEASIBLE));
        assert_eq!(cases.len(), 1, "the fixture is one case");
        let census = bare_suite().census(&cases);
        assert!(
            census.iter().any(|line| line.contains("1 feasible")),
            "{census:#?}"
        );
    }

    #[test]
    fn the_kind_census_line_counts_a_lower_case_by_its_own_kind() {
        // The identical precedent applied to this round's own addition: a case naming
        // `"kind": "lower"` is counted under `lower` in the same line, not left silently
        // uncounted.
        let cases = cases_of(&file_with(MINIMAL_LOWER));
        assert_eq!(cases.len(), 1, "the fixture is one case");
        let census = bare_suite().census(&cases);
        assert!(
            census.iter().any(|line| line.contains("1 lower")),
            "{census:#?}"
        );
    }

    #[test]
    fn an_unrecognized_kind_is_a_violation_rather_than_a_silent_compose_reading() {
        // The gap this round closes in `check_input`'s own `match`: before, a `kind` this
        // format does not have fell through to the wildcard arm and was checked only
        // against `compose`'s own required fields, exactly the failure mode `ask`'s own doc
        // in `crates/jlreq-conform/src/run.rs` already names for a forgotten dispatcher arm,
        // now closed on the reading side as well.
        let mutated = MINIMAL.replace("\"kind\": \"compose\"", "\"kind\": \"composed\"");
        let found = examine(&file_with(&mutated));
        assert!(
            found
                .iter()
                .any(|message| message.contains("input.kind") && message.contains("composed")),
            "{found:#?}"
        );
    }

    #[test]
    fn a_deferred_rule_is_not_uncovered_and_is_counted_in_the_census() {
        let mut suite = bare_suite();
        suite.rules = Some(inventory());
        suite.directory_exists = true;
        suite.deferrals = Ledger::of(
            "[[deferred]]\nrule = \"B.1@cl-05,cl-05\"\nmilestone = \"M2\"\nwhy = \"a\"\n\
             [[deferred]]\nrule = \"B.1@cl-05,cl-19\"\nmilestone = \"M2\"\nwhy = \"b\"\n",
        );
        let cases = cases_of(&file_with(MINIMAL));
        assert!(
            suite.coverage(&cases).is_empty(),
            "every rule now has a case or a deferral: {found:#?}",
            found = suite.coverage(&cases)
        );
        let census = suite.census(&cases);
        assert!(
            census.iter().any(
                |line| line.contains("defers 2 of the 3 inventoried rule(s)")
                    && line.contains("M2 2")
            ),
            "the debt is counted out loud on a green run: {census:#?}"
        );
        assert!(
            census.iter().any(|line| line.contains(
                "1 of 3 inventoried rule(s) have a case, 2 are deferred to a later \
                 milestone, 0 are classified editorial/non-observable, 0 are none of these"
            )),
            "{census:#?}"
        );
    }

    #[test]
    fn a_rule_that_is_neither_covered_nor_deferred_fails_and_a_stale_deferral_does_too() {
        let mut suite = bare_suite();
        suite.rules = Some(inventory());
        suite.directory_exists = true;
        suite.deferrals = deferring("B.1@cl-05,cl-05", "M2");
        let cases = cases_of(&file_with(MINIMAL));
        let found = suite.coverage(&cases);
        assert!(
            found.iter().any(|message| message
                .contains("1 inventoried rule(s) have neither a conformance case, a deferral")),
            "deferring one of the two uncovered rules leaves the other uncovered: {found:#?}"
        );
        suite.deferrals = deferring("3.1.9", "M2");
        let found = suite.deferrals.examine(deferral::Reference {
            inventory: suite.rules.as_ref(),
            covered: Some(&suite.covered(&cases)),
            milestones: &suite.milestones,
        });
        assert!(
            found
                .iter()
                .any(|message| message.contains("already covers it")),
            "and deferring a rule the suite covers is stale rather than harmless: {found:#?}"
        );
    }

    #[test]
    fn a_suite_directory_that_exists_is_subtracted_from_even_when_it_is_empty() {
        let mut suite = bare_suite();
        suite.rules = Some(inventory());
        suite.directory_exists = true;
        let found = suite.coverage(&[]);
        assert!(
            found
                .iter()
                .any(|message| message
                    .contains("3 inventoried rule(s) have neither a conformance case")),
            "creating the directory is what turns the subtraction on: {found:#?}"
        );
        assert!(
            !suite
                .census(&[])
                .iter()
                .any(|line| line.contains("did not run")),
            "and then nothing is deferred"
        );
    }

    #[test]
    fn the_column_line_is_not_read_as_a_row_under_the_comment_block_derive_writes() {
        let source = concat!(
            "# spec/derived/rules.tsv\n",
            "#\n",
            "# Generated by `cargo run -p xtask -- derive`. Do not edit.\n",
            "\n",
            "address\tname\tstanding\n",
            "# A comment below the column line is a comment too.\n",
            "3.1.9\tPOSITIONING\tNormative\n",
        );
        let read = inventory_column(source, "address", Path::new(RULES_INVENTORY))
            .expect("the fixture names the column");
        assert_eq!(
            read,
            BTreeSet::from(["3.1.9".to_owned()]),
            "`address` is the header, not a rule addressed `address`"
        );
    }

    #[test]
    fn a_family_credits_the_cells_it_names_and_a_dead_one_is_reported() {
        let row = MINIMAL.replace(
            "\"rules\": [\"3.1.9\"],",
            "\"rules\": [\"3.1.9\"], \"covers\": [\"B.1@cl-05,*\"],",
        );
        let cases = cases_of(&file_with(&row));
        assert!(unresolved_addresses(&cases, &inventory(), None).is_empty());
        assert_eq!(
            declared_addresses(&cases, &inventory()).len(),
            3,
            "a row credits every cell in it"
        );
        let dead = MINIMAL.replace(
            "\"rules\": [\"3.1.9\"],",
            "\"rules\": [\"3.1.9\"], \"covers\": [\"B.1@cl-99,*\"],",
        );
        let found = unresolved_addresses(&cases_of(&file_with(&dead)), &inventory(), None);
        assert!(
            found
                .iter()
                .any(|message| message.contains("names no inventoried rule")),
            "{found:#?}"
        );
    }

    #[test]
    fn the_committed_schema_states_the_same_contract() {
        assert!(check_schema(None, false).is_empty());
        let missing = check_schema(None, true);
        assert!(
            missing
                .iter()
                .any(|message| message.contains("not committed")),
            "{missing:#?}"
        );
        let thin = check_schema(Some("{ \"required\": [\"id\"] }"), true);
        assert!(
            thin.iter()
                .any(|message| message.contains("does not require `rules`")),
            "{thin:#?}"
        );
        let names: Vec<String> = REQUIRED_BY_SHAPE
            .iter()
            .flat_map(|(_, fields)| fields.iter().map(|field| format!("\"{field}\"")))
            .collect();
        let whole = format!("{{ \"required\": [{fields}] }}", fields = names.join(", "));
        assert!(check_schema(Some(&whole), true).is_empty());
    }

    #[test]
    fn the_reader_reads_the_subset_the_format_uses() {
        assert_eq!(
            json(r#"{ "a": "あ😀\n" }"#)
                .get("a")
                .and_then(Json::as_text),
            Some("あ😀\n")
        );
        assert!(
            Json::parse(r#"{ "a": 1, "a": 2 }"#).is_err(),
            "a name is written once"
        );
        assert!(
            Json::parse(r#"{ "a": 1 } {}"#).is_err(),
            "one object per file"
        );
        assert!(
            Json::parse(r#"{ "a": 01 }"#).is_err(),
            "a leading zero is not JSON"
        );
        assert!(Json::parse("[").is_err());
        assert!(
            Json::parse(&"[".repeat(64)).is_err(),
            "a case never nests that deep"
        );
        assert_eq!(json("[]"), Json::Array(Vec::new()));
    }

    #[test]
    fn the_em_denominator_comes_from_the_crate_that_declares_it() {
        assert_eq!(
            declared_units_per_em("pub const UNITS_PER_EM: i32 = 720;"),
            Some(720)
        );
        assert_eq!(
            declared_units_per_em("const UNITS_PER_EM_HALF: i32 = 360;"),
            None
        );
        assert_eq!(declared_units_per_em("let units_per_em = 720;"), None);
    }

    #[test]
    fn a_policy_path_is_the_generated_dotted_path() {
        assert!(is_path("spacing.line_end_punctuation"));
        assert!(!is_path("spacing..level"));
        assert!(!is_path("Spacing.level"));
        assert!(!is_path(""));
        let unknown: BTreeMap<String, BTreeSet<String>> = [(
            "kinsoku.level".to_owned(),
            ["strict".to_owned(), "loose".to_owned()]
                .into_iter()
                .collect(),
        )]
        .into_iter()
        .collect();
        let policy = json(
            r#"[{ "policy": { "spacing.line_end_punctuation": "solid" }, "source": "a",
                  "expect": { "lines": [] } }]"#,
        );
        let reference = Reference {
            units_per_em: Some(720),
            questions: Some(&unknown),
        };
        let found = check_permitted(Some(&policy), reference);
        assert!(
            found
                .iter()
                .any(|message| message.contains("policy space does not contain")),
            "{found:#?}"
        );
    }

    #[test]
    fn a_choice_the_specification_does_not_permit_is_refused_by_name() {
        // The other half of the pair. A choice name was checked for being a non-empty
        // string and nothing else, so `kinsoku.level = banana` passed the gate and then
        // applied to nothing at run time — the entry became unreachable rather than wrong,
        // which is the failure the stable dotted paths exist to prevent.
        let known: BTreeMap<String, BTreeSet<String>> = [(
            "kinsoku.level".to_owned(),
            ["strict".to_owned(), "loose".to_owned()]
                .into_iter()
                .collect(),
        )]
        .into_iter()
        .collect();
        let reference = Reference {
            units_per_em: Some(720),
            questions: Some(&known),
        };
        let invented = json(
            r#"[{ "policy": {}, "source": "a", "expect": { "lines": [] } },
                { "policy": { "kinsoku.level": "banana" }, "source": "b",
                  "expect": { "lines": [] } }]"#,
        );
        let found = check_permitted(Some(&invented), reference);
        assert!(
            found.iter().any(|message| message.contains(
                "is `banana`, which is not one of the answer(s) spec/derived/questions.tsv"
            )),
            "{found:#?}"
        );
        let permitted = json(
            r#"[{ "policy": {}, "source": "a", "expect": { "lines": [] } },
                { "policy": { "kinsoku.level": "loose" }, "source": "b",
                  "expect": { "lines": [] } }]"#,
        );
        assert!(
            check_permitted(Some(&permitted), reference).is_empty(),
            "and an answer the row records is accepted"
        );
    }

    #[test]
    fn the_gate_takes_the_spelling_the_design_uses_and_nothing_else() {
        assert!(accept_arguments(&[]).is_ok());
        assert!(accept_arguments(&["--check".to_owned()]).is_ok());
        assert!(accept_arguments(&["--fix".to_owned()]).is_err());
    }

    #[test]
    fn the_repository_as_it_stands_has_nothing_to_object_to() {
        let root = shared::workspace_root().expect("the workspace root is locatable");
        let suite = Suite::read(&root).expect("the suite and its inventories are readable");
        let (census, violations) = suite.examine();
        assert!(violations.is_empty(), "{violations:#?}");
        assert_eq!(census.len(), 10, "{census:#?}");
        assert!(
            census
                .iter()
                .any(|line| line.contains("rule(s) to a later milestone: ")),
            "the deferral census is printed on every run, green or not: {census:#?}"
        );
        assert!(
            census.iter().any(
                |line| line.starts_with("declared coverage: ") && !line.contains("did not run")
            ),
            "the cases directory exists, so the subtraction runs and the census states it \
             in numbers; the `did not run` wording this assertion carried until M0-b was \
             the M0-a state, when creating the directory was still ahead: {census:#?}"
        );
        assert!(
            census
                .iter()
                .any(|line| line.contains("case(s) naming") && line.contains("rule address")),
            "the suite is read and counted, not merely found: {census:#?}"
        );
    }
}
