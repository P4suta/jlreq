// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The conformance deferral ledger: `docs/conformance-deferrals.toml`.
//!
//! CONTRIBUTING.md states that a rule without a conformance case is incomplete, and [ADR
//! 0013](../../docs/adr/0013-rules-are-addressed-by-specification-address.md) makes that
//! arithmetic. Two gates do the subtraction: `conform` subtracts the rules the cases
//! declare from the rules `spec/derived/rules.tsv` inventories, and `spec-links` subtracts
//! them from the rules the layout core's doc comments cite. The inventory is generated
//! whole — every rule of §3 and of Appendices B through F at once — while the suite is
//! written milestone by milestone, so both subtractions have a remainder that is nothing
//! but the schedule.
//!
//! Without a place to write that down, a gate could only choose between two false
//! sentences: that the rules are covered, by weakening the subtraction until the remainder
//! disappeared, or that the workspace is broken, by failing over a milestone that has not
//! happened yet. So a rule is in exactly one of five states, and this module makes every
//! state explicit:
//!
//! - **covered** — a case names it, or a `covers` family credits it;
//! - **deferred** — a `[[deferred]]` table names it and the milestone that will cover it;
//! - **editorial** — evidence shows that it advises an editor, not the layout engine;
//! - **non-observable** — evidence shows that JLReq requires no distinguishable output;
//! - **uncovered** — none of these, which is a violation naming the rule.
//!
//! Deferred is not exempt, and three things keep it from becoming so. The count is reported
//! on every run, per milestone, so the debt is stated in numbers on a green run rather than
//! being a silence. The milestone must be one `ROADMAP.md` declares, so an entry cannot
//! name a schedule that does not exist; that document is what a milestone *is*, and this
//! one only names one (ADR 0019). And the entry expires by itself: the moment a case names
//! the rule, the rule is covered and the deferral is a violation, because a claim about the
//! future that the present has already answered is a stale claim. Deleting the entry is the
//! reviewable act that says the rule now has a case.
//!
//! # The other half of the subtraction
//!
//! A deferral says which milestone will cover a rule. Nothing said which milestone already
//! *does*, and that asymmetry was a hole a whole milestone's work could fall through: a rule
//! whose case this milestone owed could be given a `[[deferred]]` entry naming a later one
//! and every gate stayed green, because the only thing the subtraction knew was that the
//! rule was spoken for. The prose that named the rules M0 answers lived in the ledger's own
//! header, where no gate could read it.
//!
//! `[[owned]]` is that half written down. An entry names a rule whose case a milestone has
//! already written, and it is held to the opposite invariant from a deferral: the rule must
//! be **covered** now, and it must not be deferred by anything. Removing the case then fails
//! rather than passing, and parking the rule in a later milestone fails as well, so the
//! reviewable act is the same in both directions.
//!
//! What is deliberately not checked is that the milestone named is the *right* one. Nothing
//! mechanical can know that, which is why every entry carries a `why` a reviewer can
//! disagree with and why the file is owned by the code owners. Nothing here reads which
//! milestone the workspace is currently writing either, so this module can neither bless
//! nor refuse a deferral naming it; what makes such an entry visible is the census, and what
//! ends it is a case.
//!
//! The reader is hand-rolled for the reason `xtask` declares no dependencies at all, and
//! its line primitives are `shared`'s, so this file and `docs/direction-sites.toml` are
//! read in one language rather than two.
//!
//! See `docs/conformance-deferrals.toml`, `docs/design/conformance.md` and
//! `docs/adr/0013-rules-are-addressed-by-specification-address.md`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::Path;

use crate::shared::{self, array_header, basic_string, before_comment};

/// The ledger, relative to the workspace root.
pub(crate) const LEDGER: &str = "docs/conformance-deferrals.toml";

/// The document that declares what the milestones are, relative to the workspace root.
pub(crate) const ROADMAP: &str = "ROADMAP.md";

/// The table that says a later milestone will cover a rule.
const TABLE: &str = "deferred";

/// The table that says a milestone already has.
const OWNED: &str = "owned";

/// A specification statement that gives editorial guidance rather than a layout result.
const EDITORIAL: &str = "editorial";

/// A statement whose required distinction is absent from the black-box input or output.
const NON_OBSERVABLE: &str = "non-observable";

/// The four states the ledger can record, and no others.
const TABLES: [&str; 4] = [TABLE, OWNED, EDITORIAL, NON_OBSERVABLE];

/// The three keys a scheduled or owned rule carries.
const SCHEDULE_KEYS: [&str; 3] = ["rule", "milestone", "why"];

/// A final classification has evidence but deliberately no schedule.
const CLASSIFICATION_KEYS: [&str; 2] = ["rule", "why"];

/// The vendored primary specification a final classification identifies in its evidence.
const PRIMARY_SPEC: &str = "spec/snapshot/index.html";

/// The heading depth `ROADMAP.md` gives one milestone.
const HEADING: &str = "## ";

/// The letter a milestone name opens with.
const MILESTONE: char = 'M';

/// One entry of either table: which rule, which milestone, and where it is written.
#[derive(Debug)]
pub(crate) struct Deferral {
    /// Which table wrote it, which is what says whether the milestone is a promise or a
    /// record.
    table: &'static str,
    /// The rule, as a canonical address.
    rule: String,
    /// The milestone whose cases close it, or have closed it; empty for final classifications.
    milestone: String,
    /// The line the table opens on, so a finding names it.
    line: usize,
}

/// Everything `docs/conformance-deferrals.toml` states.
#[derive(Debug, Default)]
pub(crate) struct Ledger {
    /// Whether the file exists at all. An absent ledger is not an empty one.
    present: bool,
    /// The entries of both tables, in the order the file writes them.
    entries: Vec<Deferral>,
    /// What the reader could not read, one message per malformed thing.
    problems: Vec<String>,
}

/// The milestones `ROADMAP.md` declares, which is the vocabulary a deferral names one from.
#[derive(Debug, Default)]
pub(crate) struct Milestones {
    /// Whether the roadmap could be read.
    present: bool,
    /// Every milestone name it heads a section with.
    names: BTreeSet<String>,
}

/// What the cross-file checks compare the ledger against.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Reference<'a> {
    /// Every address the rule inventory names, when it has been generated.
    pub(crate) inventory: Option<&'a BTreeSet<String>>,
    /// Every rule the conformance suite covers today, families included, when the caller has
    /// read the suite. `None` is a check that did not run rather than a suite covering
    /// nothing, which is the same convention `inventory` uses: the two findings that compare
    /// against it — a stale deferral and an owned rule without a case — would otherwise be
    /// answered by a reader that never opened the cases directory.
    pub(crate) covered: Option<&'a BTreeSet<&'a str>>,
    /// The milestones `ROADMAP.md` declares.
    pub(crate) milestones: &'a Milestones,
}

impl Milestones {
    /// Read the roadmap, or record that it could not be read.
    pub(crate) fn read(root: &Path) -> io::Result<Self> {
        let path = root.join(ROADMAP);
        if !path.is_file() {
            return Ok(Self::default());
        }
        Ok(Self {
            present: true,
            names: declared(&fs::read_to_string(path)?),
        })
    }
}

/// Every milestone a roadmap heads a section with.
///
/// Read from the headings themselves rather than written down here, for the reason the
/// workspace member list is derived: a milestone added to the roadmap is nameable the
/// moment it is added, and one that is renamed fails every entry naming the old name
/// instead of narrowing the check in silence.
fn declared(roadmap: &str) -> BTreeSet<String> {
    roadmap
        .lines()
        .filter_map(|line| line.strip_prefix(HEADING))
        .filter_map(|heading| heading.split_whitespace().next())
        .filter(|name| is_milestone(name))
        .map(str::to_owned)
        .collect()
}

/// Whether a heading's first word names a milestone: `M` and a number.
///
/// The roadmap numbers its milestones from zero, so this is not the address grammar's
/// number, which numbers the specification's sections from one.
fn is_milestone(name: &str) -> bool {
    name.strip_prefix(MILESTONE).is_some_and(|digits| {
        !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
    })
}

impl Ledger {
    /// Read the ledger, or record that it is not there.
    pub(crate) fn read(root: &Path) -> io::Result<Self> {
        let path = root.join(LEDGER);
        if !path.is_file() {
            return Ok(Self::default());
        }
        Ok(Self::of(&fs::read_to_string(path)?))
    }

    /// The ledger one file's text states.
    ///
    /// Separate from `read` so that a fixture goes through the same reader the committed
    /// file does, here and in `conform`'s tests.
    pub(crate) fn of(text: &str) -> Self {
        let (entries, problems) = parse(text);
        Self {
            present: true,
            entries,
            problems,
        }
    }

    /// The rules the ledger defers, as far as an entry names one at all.
    ///
    /// An entry whose address is not in the grammar defers nothing, because it names no
    /// rule to defer; every other entry does, whether or not the cross-file checks below
    /// find something to say about it. Those findings are reported once, by the gate that
    /// owns this file, rather than turning into a second and misleading "uncovered" for the
    /// same rule.
    pub(crate) fn rules(&self) -> BTreeSet<&str> {
        self.entries
            .iter()
            .filter(|entry| entry.table == TABLE && is_address(&entry.rule))
            .map(|entry| entry.rule.as_str())
            .collect()
    }

    /// The rules the ledger records a milestone as already covering.
    ///
    /// Read by the gate that owns this file and by nothing else: an owned rule is covered by
    /// a case, so it is not a third state of the subtraction. What the entry adds is the
    /// obligation, which is checked against the suite rather than assumed.
    pub(crate) fn owned(&self) -> BTreeSet<&str> {
        self.entries
            .iter()
            .filter(|entry| entry.table == OWNED && is_address(&entry.rule))
            .map(|entry| entry.rule.as_str())
            .collect()
    }

    /// Rules resolved by an evidence-bearing editorial or non-observable classification.
    pub(crate) fn classified(&self) -> BTreeSet<&str> {
        self.entries
            .iter()
            .filter(|entry| {
                matches!(entry.table, EDITORIAL | NON_OBSERVABLE) && is_address(&entry.rule)
            })
            .map(|entry| entry.rule.as_str())
            .collect()
    }

    /// Rules which legitimately need no case yet: deferred or explicitly classified.
    pub(crate) fn accounted(&self) -> BTreeSet<&str> {
        self.rules().union(&self.classified()).copied().collect()
    }

    /// Every way this file can be wrong, in one pass.
    pub(crate) fn examine(&self, reference: Reference<'_>) -> Vec<String> {
        if !self.present {
            return vec![format!(
                "{LEDGER} is missing; it is where a rule a later milestone covers is \
                 declared, and an absent ledger is not an empty one — without it every \
                 inventoried rule without a case is uncovered (ADR 0013)"
            )];
        }
        let mut found = self.problems.clone();
        let scheduled = self
            .entries
            .iter()
            .filter(|entry| matches!(entry.table, TABLE | OWNED))
            .count();
        if !reference.milestones.present && scheduled != 0 {
            found.push(format!(
                "{ROADMAP} could not be read, so nothing says the {scheduled} milestone(s) this \
                 file defers to exist; the roadmap is what a milestone is and this file only \
                 names one (ADR 0019)"
            ));
        }
        for entry in &self.entries {
            found.extend(examine_entry(entry, reference));
        }
        found.extend(self.unrecorded(reference));
        found
    }

    /// Every rule the suite covers that this file does not say a milestone covers.
    ///
    /// The ledger tables are a total accounting of the inventory, and this is the half that
    /// keeps covered rules total. Without it a case could credit a rule to nobody: `kind` alone decides
    /// which layer a case asks, so publishing a boundary case for a rule no layer of this
    /// workspace answers credited the rule to the coverage gate and put nothing in front of
    /// a reviewer. Both routes now end at the same file and the same `why`.
    fn unrecorded(&self, reference: Reference<'_>) -> Vec<String> {
        let Some(covered) = reference.covered else {
            return Vec::new();
        };
        let owned = self.owned();
        let missing: Vec<&&str> = covered
            .iter()
            .filter(|rule| !owned.contains(*rule))
            .collect();
        if missing.is_empty() {
            return Vec::new();
        }
        vec![format!(
            "{LEDGER}: {count} rule(s) a conformance case covers have no `[[{OWNED}]]` entry, \
             so nothing says which milestone's cases cover them: {missing:?}",
            count = missing.len()
        )]
    }

    /// What the ledger states, whether or not anything was found.
    ///
    /// Printed on every run, because a deferral that is not counted out loud is an
    /// exemption. The per-milestone breakdown is what makes the debt legible as a schedule
    /// rather than as one number that only ever grows. It counts the entries that actually
    /// defer something, so the parts sum to the total and an entry naming no rule is a
    /// violation rather than a number.
    pub(crate) fn census(&self, inventory: Option<&BTreeSet<String>>) -> String {
        if !self.present {
            return format!("{LEDGER} does not exist, so no rule is deferred");
        }
        let deferred = self.rules();
        let mut counted: BTreeMap<&str, usize> = BTreeMap::new();
        for entry in self
            .entries
            .iter()
            .filter(|entry| entry.table == TABLE && deferred.contains(entry.rule.as_str()))
        {
            let count = counted.entry(entry.milestone.as_str()).or_default();
            *count = count.saturating_add(1);
        }
        let by_milestone = counted
            .into_iter()
            .map(|(milestone, count)| format!("{milestone} {count}"))
            .collect::<Vec<_>>()
            .join(", ");
        let of = match inventory {
            Some(inventory) => format!(" of the {total} inventoried", total = inventory.len()),
            None => String::new(),
        };
        let owned = self
            .entries
            .iter()
            .filter(|entry| entry.table == OWNED)
            .fold(BTreeMap::<&str, usize>::new(), |mut counted, entry| {
                let count = counted.entry(entry.milestone.as_str()).or_default();
                *count = count.saturating_add(1);
                counted
            })
            .into_iter()
            .map(|(milestone, count)| format!("{milestone} {count}"))
            .collect::<Vec<_>>()
            .join(", ");
        let editorial = self
            .entries
            .iter()
            .filter(|entry| entry.table == EDITORIAL)
            .count();
        let non_observable = self
            .entries
            .iter()
            .filter(|entry| entry.table == NON_OBSERVABLE)
            .count();
        format!(
            "{LEDGER} defers {count}{of} rule(s) to a later milestone: {by_milestone}; and \
             classifies {editorial} editorial and {non_observable} non-observable rule(s); \
             and records {owned_count} as covered by a milestone already written: {owned}",
            count = deferred.len(),
            owned_count = self.owned().len()
        )
    }
}

/// Check one entry against the inventory, the roadmap and the suite.
///
/// Four findings, and each of them is a way the entry has stopped being true rather than a
/// matter of taste: it defers a rule that is not in the address grammar, or a rule the
/// inventory does not carry, or to a milestone the roadmap does not declare, or a rule a
/// case already covers.
fn examine_entry(entry: &Deferral, reference: Reference<'_>) -> Vec<String> {
    let Deferral {
        table,
        rule,
        milestone,
        line,
    } = entry;
    let mut found = Vec::new();
    if !is_address(rule) {
        found.push(format!(
            "{LEDGER}:{line}: defers `{rule}`, which is not a specification address in the \
             canonical rendering (ADR 0013: `3.1.9`, `B.2#3`, `B.1@cl-05,cl-05`)"
        ));
    } else if reference
        .inventory
        .is_some_and(|inventory| !inventory.contains(rule.as_str()))
    {
        found.push(format!(
            "{LEDGER}:{line}: defers rule `{rule}`, which the rule inventory does not \
             contain; a deferral of a rule that does not exist defers nothing (ADR 0013)"
        ));
    }
    if matches!(*table, TABLE | OWNED)
        && reference.milestones.present
        && !reference.milestones.names.contains(milestone)
    {
        found.push(format!(
            "{LEDGER}:{line}: names rule `{rule}` under `{milestone}`, which {ROADMAP} does \
             not declare; an entry names a milestone that document heads a section with"
        ));
    }
    let Some(covered) = reference.covered else {
        return found;
    };
    if *table == TABLE && covered.contains(rule.as_str()) {
        found.push(format!(
            "{LEDGER}:{line}: defers rule `{rule}` to `{milestone}`, and a conformance case \
             already covers it; the deferral has gone stale and deleting it is what says the \
             rule now has a case (ADR 0013)"
        ));
    }
    if matches!(*table, EDITORIAL | NON_OBSERVABLE) && covered.contains(rule.as_str()) {
        found.push(format!(
            "{LEDGER}:{line}: classifies rule `{rule}` under `[[{table}]]`, but a \
             conformance case already observes it; remove or revise the contradicted \
             classification"
        ));
    }
    if *table == OWNED && !covered.contains(rule.as_str()) {
        found.push(format!(
            "{LEDGER}:{line}: records rule `{rule}` as one `{milestone}` covers, and no \
             conformance case names it; a rule a milestone owns is covered by a case of that \
             milestone, and moving it to a `[[{TABLE}]]` table is not what closes it (ADR 0013)"
        ));
    }
    found
}

/// Whether the text is a rule address in the canonical rendering ADR 0013 fixes.
fn is_address(text: &str) -> bool {
    shared::address(text).is_some_and(|parsed| parsed.to_string() == text)
}

/// One table of the ledger under construction.
#[derive(Debug)]
struct Draft {
    /// Which of the four tables it is.
    table: &'static str,
    /// The line the table header sits on.
    line: usize,
    /// The keys read so far.
    values: BTreeMap<String, String>,
}

/// Read the ledger, and complain about anything the schema does not allow.
///
/// It reads the four tables this file defines, all with one-line basic strings, and rejects
/// everything else rather than skipping it, because a key this reader passed over in
/// silence would be a key no reviewer was told about.
fn parse(text: &str) -> (Vec<Deferral>, Vec<String>) {
    let mut entries = Vec::new();
    let mut problems = Vec::new();
    let mut draft: Option<Draft> = None;

    for (offset, raw) in text.lines().enumerate() {
        let line = offset.saturating_add(1);
        let content = before_comment(raw).trim();
        if content.is_empty() {
            continue;
        }
        if content.starts_with('[') {
            close(draft.take(), &mut entries, &mut problems);
            draft = open(content, line, &mut problems);
            continue;
        }
        read_key(content, line, draft.as_mut(), &mut problems);
    }
    close(draft.take(), &mut entries, &mut problems);
    (entries, problems)
}

/// Open a table, or say that the file has no such table.
fn open(content: &str, line: usize, problems: &mut Vec<String>) -> Option<Draft> {
    if let Some(table) = TABLES
        .into_iter()
        .find(|table| array_header(content) == Some(table))
    {
        return Some(Draft {
            table,
            line,
            values: BTreeMap::new(),
        });
    }
    problems.push(format!(
        "{LEDGER}:{line}: `{content}` is not a table this file has; the schema is \
         `[[{TABLE}]]`, `[[{OWNED}]]`, `[[{EDITORIAL}]]`, `[[{NON_OBSERVABLE}]]` and \
         nothing else"
    ));
    None
}

/// Read one `key = "value"` line into the table it belongs to.
fn read_key(content: &str, line: usize, draft: Option<&mut Draft>, problems: &mut Vec<String>) {
    let Some((key, rest)) = content.split_once('=') else {
        problems.push(format!(
            "{LEDGER}:{line}: `{content}` is neither a table header nor a `key = \"value\"` line"
        ));
        return;
    };
    let key = key.trim();
    let Some(draft) = draft else {
        problems.push(format!(
            "{LEDGER}:{line}: `{key}` sits outside a recognized ledger table; \
             this file has no top-level keys"
        ));
        return;
    };
    let keys = keys_for(draft.table);
    if !keys.contains(&key) {
        problems.push(format!(
            "{LEDGER}:{line}: `{key}` is not a key of `[[{table}]]`; the schema is {keys:?} \
             and nothing else",
            table = draft.table
        ));
        return;
    }
    let Some(value) = basic_string(rest) else {
        problems.push(format!(
            "{LEDGER}:{line}: `{key}` is not a one-line basic string; this reader accepts no \
             other form, so that a value it cannot read is a finding rather than a silence"
        ));
        return;
    };
    if draft
        .values
        .insert(key.to_owned(), value.to_owned())
        .is_some()
    {
        problems.push(format!(
            "{LEDGER}:{line}: `{key}` is written twice in one `[[{TABLE}]]` table"
        ));
    }
}

/// Turn a finished draft into a deferral, or say which key it is missing.
fn close(draft: Option<Draft>, entries: &mut Vec<Deferral>, problems: &mut Vec<String>) {
    let Some(draft) = draft else { return };
    let line = draft.line;
    let missing: Vec<&str> = keys_for(draft.table)
        .iter()
        .copied()
        .filter(|key| draft.values.get(*key).is_none_or(String::is_empty))
        .collect();
    if !missing.is_empty() {
        problems.push(format!(
            "{LEDGER}:{line}: this `[[{table}]]` table has no {missing:?}; every entry carries \
             every entry carries the fields its table schema requires",
            table = draft.table
        ));
        return;
    }
    if matches!(draft.table, EDITORIAL | NON_OBSERVABLE)
        && !draft
            .values
            .get("why")
            .is_some_and(|why| why.contains(PRIMARY_SPEC))
    {
        problems.push(format!(
            concat!(
                "{}:{}: this `[[{}]]` table must cite `{}` in ",
                "`why`; a final classification is evidence-bearing only when it points to ",
                "the vendored primary text a reviewer can inspect"
            ),
            LEDGER, line, draft.table, PRIMARY_SPEC
        ));
        return;
    }
    let read = |key: &str| draft.values.get(key).cloned().unwrap_or_default();
    let rule = read("rule");
    if let Some(twin) = entries.iter().find(|entry| entry.rule == rule) {
        problems.push(format!(
            "{LEDGER}:{line}: rule `{rule}` is already named on line {first}, under \
             `[[{first_table}]]`; one rule is covered by one milestone or waits on one, and \
             two entries are two answers to the question of when it is covered",
            first = twin.line,
            first_table = twin.table
        ));
        return;
    }
    entries.push(Deferral {
        table: draft.table,
        rule,
        milestone: read("milestone"),
        line,
    });
}

fn keys_for(table: &str) -> &'static [&'static str] {
    if matches!(table, EDITORIAL | NON_OBSERVABLE) {
        &CLASSIFICATION_KEYS
    } else {
        &SCHEDULE_KEYS
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{Ledger, Milestones, Reference, declared, is_address, parse};
    use crate::shared;

    /// A well-formed ledger deferring two rules to two milestones.
    const LEDGER: &str = r#"
# A comment, and a `[[deferred]]` inside it is prose.
[[deferred]]
rule = "3.1.9"
milestone = "M2"
why = "The amount at the line end is jlreq-spacing's answer."

[[deferred]]
rule = "B.2#3"
milestone = "M1"
why = "Two conditional spaces rather than one amount."
"#;

    /// The roadmap's shape: milestone headings and a section that is not one.
    const ROADMAP: &str = "# Roadmap\n\n## M0 — Character classes\n\ntext\n\n\
                           ## M1 — Kinsoku and line adjustment\n\n## M2 — Mojikumi\n\n\
                           ## Non-goals\n";

    /// The milestones of the fixture roadmap.
    fn milestones() -> Milestones {
        Milestones {
            present: true,
            names: declared(ROADMAP),
        }
    }

    /// An inventory holding both fixture rules.
    fn inventory() -> BTreeSet<String> {
        ["3.1.9", "B.2#3"]
            .iter()
            .map(|address| (*address).to_owned())
            .collect()
    }

    /// Examine a ledger against the fixture inventory and roadmap, covering nothing.
    fn examine(ledger: &Ledger, covered: &BTreeSet<&str>) -> Vec<String> {
        let inventory = inventory();
        let milestones = milestones();
        ledger.examine(Reference {
            inventory: Some(&inventory),
            covered: Some(covered),
            milestones: &milestones,
        })
    }

    #[test]
    fn a_well_formed_ledger_defers_what_it_names() {
        let ledger = Ledger::of(LEDGER);
        let found = examine(&ledger, &BTreeSet::new());
        assert!(found.is_empty(), "{found:#?}");
        assert_eq!(ledger.rules(), BTreeSet::from(["3.1.9", "B.2#3"]));
        let inventory = inventory();
        let census = ledger.census(Some(&inventory));
        assert!(
            census.contains("defers 2 of the 2 inventoried rule(s)")
                && census.contains("M1 1, M2 1"),
            "the debt is counted per milestone on every run: {census}"
        );
    }

    #[test]
    fn a_deferred_rule_that_has_a_case_is_stale() {
        let covered = BTreeSet::from(["3.1.9"]);
        let found = examine(&Ledger::of(LEDGER), &covered);
        assert!(
            found
                .iter()
                .any(|message| message.contains("already covers it")),
            "a deferral the suite has answered is a violation rather than a duplicate: \
             {found:#?}"
        );
    }

    #[test]
    fn a_non_observable_rule_is_accounted_for_without_being_deferred() {
        let source = r#"
[[non-observable]]
rule = "3.1.9"
why = "The Note in spec/snapshot/index.html leaves the result implementation-defined, so no black-box output is required."
"#;
        let ledger = Ledger::of(source);
        let found = examine(&ledger, &BTreeSet::new());
        assert!(found.is_empty(), "{found:#?}");
        assert!(
            ledger.rules().is_empty(),
            "classification is not implementation debt"
        );
        assert_eq!(ledger.classified(), BTreeSet::from(["3.1.9"]));
        assert_eq!(ledger.accounted(), BTreeSet::from(["3.1.9"]));
        let census = ledger.census(Some(&inventory()));
        assert!(census.contains("1 non-observable"), "{census}");

        let stale = examine(&ledger, &BTreeSet::from(["3.1.9"]));
        assert!(
            stale
                .iter()
                .any(|message| message.contains("classifies") && message.contains("case")),
            "an observable case and a non-observable classification contradict: {stale:#?}"
        );
    }

    #[test]
    fn a_final_classification_cites_the_primary_specification() {
        let source = r#"
[[editorial]]
rule = "3.1.9"
why = "This is advice for the author rather than a layout result."
"#;
        let (_, problems) = parse(source);
        assert!(
            problems
                .iter()
                .any(|message| message.contains("must cite `spec/snapshot/index.html`")),
            "a conclusion without a primary-source locator is not evidence-bearing: {problems:#?}"
        );
    }

    #[test]
    fn a_deferral_names_an_inventoried_rule_and_a_declared_milestone() {
        let unknown = LEDGER.replace("\"B.2#3\"", "\"B.2#4\"");
        let found = examine(&Ledger::of(&unknown), &BTreeSet::new());
        assert!(
            found
                .iter()
                .any(|message| message.contains("the rule inventory does not contain")),
            "{found:#?}"
        );
        let ahead = LEDGER.replace("\"M2\"", "\"M9\"");
        let found = examine(&Ledger::of(&ahead), &BTreeSet::new());
        assert!(
            found
                .iter()
                .any(|message| message.contains("does not declare")),
            "{found:#?}"
        );
        let malformed = LEDGER.replace("\"3.1.9\"", "\"3.1.9#\"");
        let ledger = Ledger::of(&malformed);
        let found = examine(&ledger, &BTreeSet::new());
        assert!(
            found
                .iter()
                .any(|message| message.contains("not a specification address")),
            "{found:#?}"
        );
        assert_eq!(
            ledger.rules(),
            BTreeSet::from(["B.2#3"]),
            "an entry naming no rule defers none"
        );
    }

    #[test]
    fn the_schema_is_the_three_keys_and_nothing_else() {
        let cases = [
            ("rule = \"3.1.9\"\n", "sits outside"),
            ("[[deferred]]\nrule = \"3.1.9\"\n", "has no"),
            (
                "[[pending]]\nrule = \"3.1.9\"\nmilestone = \"M1\"\nwhy = \"a\"\n",
                "is not a table this file has",
            ),
            (
                "[[deferred]]\nrule = \"3.1.9\"\nmilestone = \"M1\"\nwhy = \"a\"\nnote = \"b\"\n",
                "is not a key of",
            ),
            (
                "[[deferred]]\nrule = \"3.1.9\"\nmilestone = \"M1\"\nwhy = 3\n",
                "not a one-line basic string",
            ),
            (
                "[[deferred]]\nrule = \"3.1.9\"\nrule = \"B.2#3\"\nmilestone = \"M1\"\nwhy = \"a\"\n",
                "written twice",
            ),
            (
                "[[deferred]]\nrule = \"3.1.9\"\nmilestone = \"M1\"\nwhy = \"a\"\n\
                 [[deferred]]\nrule = \"3.1.9\"\nmilestone = \"M2\"\nwhy = \"b\"\n",
                "is already named",
            ),
            (
                "[[owned]]\nrule = \"3.1.9\"\nmilestone = \"M0\"\nwhy = \"a\"\n\
                 [[deferred]]\nrule = \"3.1.9\"\nmilestone = \"M2\"\nwhy = \"b\"\n",
                "is already named on line 1, under `[[owned]]`",
            ),
        ];
        for (source, expected) in cases {
            let (_, problems) = parse(source);
            assert!(
                problems.iter().any(|message| message.contains(expected)),
                "`{expected}` was not reported for `{source}`: {problems:#?}"
            );
        }
    }

    #[test]
    fn a_rule_a_milestone_owns_is_one_a_case_covers() {
        // The half a deferral cannot state. An `[[owned]]` entry is a claim about the
        // present, so the suite is what answers it: delete the case and the entry fails,
        // which is what stops a rule this milestone owes from being parked in a later one.
        let owned = LEDGER.replace(
            "[[deferred]]\nrule = \"3.1.9\"",
            "[[owned]]\nrule = \"3.1.9\"",
        );
        let ledger = Ledger::of(&owned);
        assert_eq!(ledger.owned(), BTreeSet::from(["3.1.9"]));
        assert_eq!(
            ledger.rules(),
            BTreeSet::from(["B.2#3"]),
            "an owned rule is covered rather than deferred, so it is not a third state of \
             the subtraction"
        );
        let found = examine(&ledger, &BTreeSet::from(["B.2#3"]));
        assert!(
            found
                .iter()
                .any(|message| message.contains("and no conformance case names it")),
            "{found:#?}"
        );
        let found = examine(&ledger, &BTreeSet::from(["3.1.9"]));
        assert!(found.is_empty(), "{found:#?}");
    }

    #[test]
    fn a_covered_rule_no_milestone_owns_is_credited_to_nobody() {
        // The second deferral route, closed: a case may not credit a rule to the coverage
        // gate without an entry naming the milestone whose cases cover it and why.
        let found = examine(&Ledger::of(LEDGER), &BTreeSet::from(["B.1@cl-05,cl-05"]));
        assert!(
            found.iter().any(|message| message
                .contains("have no `[[owned]]` entry, so nothing says which milestone")),
            "{found:#?}"
        );
    }

    #[test]
    fn a_ledger_the_caller_has_read_no_cases_against_is_judged_on_its_own_terms() {
        let ledger = Ledger::of(&LEDGER.replace(
            "[[deferred]]\nrule = \"3.1.9\"",
            "[[owned]]\nrule = \"3.1.9\"",
        ));
        let inventory = inventory();
        let milestones = milestones();
        let found = ledger.examine(Reference {
            inventory: Some(&inventory),
            covered: None,
            milestones: &milestones,
        });
        assert!(
            found.is_empty(),
            "`None` is a check that did not run rather than a suite covering nothing, so \
             neither the stale half nor the owned half answers from a directory nobody \
             opened: {found:#?}"
        );
    }

    #[test]
    fn an_absent_ledger_is_not_an_empty_one() {
        let ledger = Ledger::default();
        let found = examine(&ledger, &BTreeSet::new());
        assert!(
            found.iter().any(|message| message.contains("is missing")),
            "{found:#?}"
        );
        assert!(ledger.census(None).contains("does not exist"));
    }

    #[test]
    fn the_milestones_are_the_roadmap_headings() {
        assert_eq!(
            declared(ROADMAP),
            BTreeSet::from(["M0".to_owned(), "M1".to_owned(), "M2".to_owned()]),
            "a section that is not a milestone declares none"
        );
        let milestones = Milestones::default();
        let inventory = inventory();
        let found = Ledger::of(LEDGER).examine(Reference {
            inventory: Some(&inventory),
            covered: Some(&BTreeSet::new()),
            milestones: &milestones,
        });
        assert_eq!(
            found.len(),
            1,
            "an unreadable roadmap leaves every milestone unjudged, and is said once: \
             {found:#?}"
        );
    }

    #[test]
    fn an_address_is_the_canonical_rendering() {
        assert!(is_address("3.1.9") && is_address("B") && is_address("B.2#3"));
        assert!(!is_address("3.01.9") && !is_address("§3.1.9") && !is_address(""));
    }

    #[test]
    fn the_repository_as_it_stands_has_nothing_to_object_to() {
        let root = shared::workspace_root().expect("the workspace root is locatable");
        let ledger = Ledger::read(&root).expect("the ledger is readable");
        let milestones = Milestones::read(&root).expect("the roadmap is readable");
        let found = ledger.examine(Reference {
            inventory: None,
            covered: None,
            milestones: &milestones,
        });
        assert!(found.is_empty(), "{found:#?}");
        assert!(
            !ledger.rules().is_empty(),
            "the suite is written milestone by milestone, so the ledger is not empty yet"
        );
    }
}
