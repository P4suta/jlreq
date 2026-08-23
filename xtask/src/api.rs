// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `api` gate.
//!
//! Holds the unified candidate surface exactly to `docs/api-1.0.toml`, including the
//! bidirectional mapping from all 22 specification questions to dedicated Style enums.
//! The retired pre-1.0 crate surfaces are deliberately outside this gate. Their structural
//! parser remains unit-tested for repository archaeology, but is inactive unless somebody
//! restores the deleted `docs/api-frozen.toml` control.
//!
//! That historical parser checked four things, each against its control file rather than against a list kept
//! here, so that relaxing any of them is a reviewed edit to a code-owned file, and a fifth
//! is checked about the gate itself:
//!
//! - every public type is `#[non_exhaustive]` unless `[[exempt]]` names it — and an
//!   `[[exempt]]` entry whose type turns out to be `#[non_exhaustive]` after all is a
//!   relaxation nobody needs, reported so the list cannot rot into permission for a
//!   decision the code has already reversed;
//! - every `[[frozen]]` projection still exists, is public, is inherent to the type it
//!   projects, takes a receiver, and answers with a set that cannot grow — `bool`, or a
//!   type `[[exempt]]` records the specification as closing. A projection whose answer set
//!   grows is exactly the outcome ADR 0012 forbids, and it is the part of that decision a
//!   signature can be read for;
//! - every `#[non_exhaustive]` type an adopter has to pass in can still be obtained;
//! - no `[[forbidden]]` shape appears, matched against declared item identifiers only;
//! - nothing in the published surface is a name this reader failed to resolve, because a
//!   gate blind to part of what it governs must say so rather than pass.
//!
//! # The two pinned definitions, and one stated widening
//!
//! *Input position* and *named constructor* are pinned in `docs/design/api-spine.md`
//! rather than settled by whatever the code turns out to do. A public type is in an input
//! position when it appears in the parameter list of a public function anywhere in the
//! workspace other than as the receiver, including inside a reference, a slice, a range,
//! an `Option` or a `Result`. A named constructor is an associated function returning
//! `Self`, `Result<Self, _>` or `Option<Self>`.
//!
//! This gate reads "associated function" as one *without* a receiver, because the same
//! decision puts consuming builder methods in a separate category, and a builder needing a
//! `Self` to start from is not how a caller obtains the first one.
//!
//! It then checks something wider than the pinned sentence, and says so here and in its own
//! output rather than taking the widening silently. ADR 0012 states the purpose of this
//! check as "otherwise the type is unconstructible and the compatibility regime has quietly
//! made the API unusable", and three types of the published design are unconstructible
//! under the sentence while being perfectly obtainable in fact: `Policy` is named
//! (`Policy::JLREQ`), `Question` is enumerated (`Question::ALL`, `Question::KINSOKU_LEVEL`),
//! and `Choice` is reached through `Question::permits`. None of the three may gain a
//! constructor — a public `Question::new(u16)` would hand a caller an ordinal with no row in
//! the generated policy space — so the sentence applied to the letter fails on the design it
//! protects, which is the one thing ADR 0012 says a gate must not do.
//!
//! So the gate asks the question the decision asks: starting from nothing, can a caller
//! outside the crate get one? A type is obtainable when some public item of its own crate
//! hands it over and the caller can reach that item — an associated constant, a function
//! with no receiver, or a method on a type that is itself obtainable — iterated to a
//! fixed point. Every named constructor satisfies it, a builder method alone does not, and
//! a sealed input with no public producer anywhere still fails, which is the failure the
//! check exists for. The API spine's sentence is what should gain the clause; until it
//! does, this note and the gate's own output carry it.
//!
//! The requirement itself falls only on the types `#[non_exhaustive]` genuinely seals: a
//! struct or a union, whose literal a caller outside the crate cannot write, and an enum
//! every variant of which is itself `#[non_exhaustive]`. A plain `#[non_exhaustive]` enum
//! leaves every variant nameable, so the failure the check exists to prevent cannot occur
//! there, and demanding a constructor for `Frame` or `Role` would be the same gate failing
//! on the same design.
//!
//! # How the surface is read
//!
//! Hand-rolled, for the reason stated on `purity`'s manifest scan: the tool that enforces
//! "the layout core declares no outside dependencies" declares none itself. Sources are
//! stripped of comments and of string and character literals — so prose naming a forbidden
//! word is not a finding — then read as items: types with their attributes, functions with
//! their parameter and result types, associated constants with their declared type, and
//! macro-generated surfaces, whose expansions are instantiated at each invocation.
//!
//! Visibility is read literally: a type is public when it is declared `pub`, whether or not
//! a `pub use` re-exports it. That is the outer bound and it fails closed — a `pub` type is
//! one export line away from an adopter's hands, so holding it to the frozen shape now is
//! what keeps the shape from being decided by that line. The 1.0 surface governed by the
//! active path is the sole `jlreq` library and the `jlreq::style` namespace, compared
//! exactly with `docs/api-1.0.toml`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::Path;

use crate::shared::{self, Gate};

/// The `api` gate, as the dispatcher sees it.
pub(crate) const GATE: Gate = Gate {
    name: "api",
    purpose: "jlreq matches docs/api-1.0.toml and its 22 typed Style mappings exactly",
    reference: concat!(
        "docs/adr/0012-outcome-and-detail-compatibility.md ",
        "and docs/design/api-spine.md"
    ),
    run,
};

/// Read the control file and the workspace, report what was examined, and check it. Takes
/// no arguments.
fn run(arguments: &[String]) -> io::Result<Vec<String>> {
    if let Some(first) = arguments.first() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("the api gate takes no arguments; got `{first}`"),
        ));
    }
    let root = shared::workspace_root()?;
    let mut violations = Vec::new();
    // Keep the historical parser executable only for repository archaeology: the 1.0
    // workspace does not ship this file, so it is never part of the release contract.
    if root.join("docs").join("api-frozen.toml").is_file() {
        let control = Control::read(&root)?;
        let surface = Surface::read(&root)?;
        control.resolve_against(&surface)?;
        report(&control, &surface);
        check_readability(&surface, &mut violations);
        check_exhaustiveness(&control, &surface, &mut violations);
        check_projections(&control, &surface, &mut violations);
        check_construction(&surface, &mut violations);
        check_forbidden_names(&control, &surface, &mut violations);
        check_forbidden_signatures(&control, &surface, &mut violations);
        if let Some(derived) = derived_questions(&root)? {
            violations.extend(check_closed_choices(&control, &derived));
        }
    }
    check_policy_space(&root, &mut violations)?;
    check_one_point_zero_allowlist(&root, &mut violations)?;
    println!("api: checked the sole public Rust crate against docs/api-1.0.toml");
    Ok(violations)
}

// -------------------------------------------------------------------------------------
// The jlreq 1.0 allowlist
// -------------------------------------------------------------------------------------

/// One public module whose directly exported item names are frozen for 1.0.
#[derive(Debug)]
struct AllowedModule {
    /// `jlreq` for the crate root, or a public module path such as `jlreq::style`.
    path: String,
    /// Every directly nameable item in that module, in no significant order.
    items: BTreeSet<String>,
}

/// One specification choice mapped onto its dedicated public Rust enum.
#[derive(Debug)]
struct StyleChoiceMapping {
    /// Stable dotted path from `spec/derived/questions.tsv`.
    question: String,
    /// Public enum in `jlreq::style`.
    rust_type: String,
    /// Number of choices the dated specification records.
    count: usize,
}

/// Read the explicit 1.0 surface and reject missing, extra, or duplicated module rows.
fn allowed_modules(root: &Path) -> io::Result<Vec<AllowedModule>> {
    let path = root.join("docs").join("api-1.0.toml");
    let text = fs::read_to_string(path)?;
    let entries = entries_of(&text);
    let mut modules = Vec::new();
    let mut paths = BTreeSet::new();
    for entry in entries.iter().filter(|entry| entry.table == "module") {
        let module_path = entry.single("path").ok_or_else(|| {
            malformed("a `[[module]]` entry in docs/api-1.0.toml has no path".to_owned())
        })?;
        let items: BTreeSet<String> = entry.list("items").iter().cloned().collect();
        if items.is_empty() {
            return Err(malformed(format!(
                "the `[[module]]` entry for `{module_path}` in docs/api-1.0.toml has no items"
            )));
        }
        if !paths.insert(module_path.to_owned()) {
            return Err(malformed(format!(
                "docs/api-1.0.toml lists the module `{module_path}` more than once"
            )));
        }
        modules.push(AllowedModule {
            path: module_path.to_owned(),
            items,
        });
    }
    if modules.is_empty() {
        return Err(malformed(
            "docs/api-1.0.toml has no `[[module]]` entries".to_owned(),
        ));
    }
    Ok(modules)
}

/// Read the complete mapping from specification questions to typed public enums.
fn style_choice_mappings(root: &Path) -> io::Result<Vec<StyleChoiceMapping>> {
    let path = root.join("docs").join("api-1.0.toml");
    let text = fs::read_to_string(path)?;
    let entries = entries_of(&text);
    let mut mappings = Vec::new();
    for entry in entries.iter().filter(|entry| entry.table == "style_choice") {
        let question = entry
            .single("question")
            .ok_or_else(|| malformed("a `[[style_choice]]` entry has no `question`".to_owned()))?;
        let rust_type = entry
            .single("type")
            .ok_or_else(|| malformed(format!("the style choice `{question}` has no `type`")))?;
        let count = entry
            .single("count")
            .and_then(|value| value.parse().ok())
            .or_else(|| entry.count("count"))
            .ok_or_else(|| {
                malformed(format!(
                    "the style choice `{question}` has no integer `count`"
                ))
            })?;
        mappings.push(StyleChoiceMapping {
            question: question.to_owned(),
            rust_type: rust_type.to_owned(),
            count,
        });
    }
    if mappings.is_empty() {
        return Err(malformed(
            "docs/api-1.0.toml has no `[[style_choice]]` entries".to_owned(),
        ));
    }
    Ok(mappings)
}

/// Compare the typed public choices with generated specification data in both directions.
fn check_style_choice_mappings(
    mappings: &[StyleChoiceMapping],
    derived: &[DerivedQuestion],
    style_items: &BTreeSet<String>,
) -> Vec<String> {
    let mut violations = Vec::new();
    let mut seen_questions = BTreeSet::new();
    let mut seen_types = BTreeSet::new();
    for mapping in mappings {
        if !seen_questions.insert(&mapping.question) {
            violations.push(format!(
                "docs/api-1.0.toml maps `{}` more than once",
                mapping.question
            ));
        }
        if !seen_types.insert(&mapping.rust_type) {
            violations.push(format!(
                "docs/api-1.0.toml maps more than one question to `{}`",
                mapping.rust_type
            ));
        }
        if !style_items.contains(&mapping.rust_type) {
            violations.push(format!(
                "docs/api-1.0.toml maps `{}` to `{}`, which jlreq::style does not export",
                mapping.question, mapping.rust_type
            ));
        }
        let Some(question) = derived
            .iter()
            .find(|question| question.path == mapping.question)
        else {
            violations.push(format!(
                "docs/api-1.0.toml maps `{}`, which {POLICY_SPACE} does not record",
                mapping.question
            ));
            continue;
        };
        if mapping.count != question.answers {
            violations.push(format!(
                "`{rust_type}` records {mapped} choices for `{path}`, but {POLICY_SPACE} row `{constant}` records {derived}",
                rust_type = mapping.rust_type,
                mapped = mapping.count,
                path = mapping.question,
                constant = question.constant,
                derived = question.answers
            ));
        }
    }
    for question in derived {
        if !seen_questions.contains(&question.path) {
            violations.push(format!(
                "{POLICY_SPACE} records `{}`, but docs/api-1.0.toml maps it to no typed enum",
                question.path
            ));
        }
    }
    violations
}

/// Compare one module source with its exact allowlist in both directions.
fn check_allowed_items(allowed: &AllowedModule, source: &str) -> Vec<String> {
    let actual: BTreeSet<String> = declarations_of(source, "src/lib.rs")
        .into_iter()
        .filter(|declaration| {
            declaration.visibility == Visibility::Public && declaration.owner.is_none()
        })
        .map(|declaration| declaration.name)
        .collect();
    let mut violations = Vec::new();
    for missing in allowed.items.difference(&actual) {
        violations.push(format!(
            "docs/api-1.0.toml allows `{path}::{missing}`, but that item is not exported",
            path = allowed.path
        ));
    }
    for extra in actual.difference(&allowed.items) {
        violations.push(format!(
            "`{path}::{extra}` is exported but absent from docs/api-1.0.toml",
            path = allowed.path
        ));
    }
    violations
}

/// Hold the only public Rust crate to the root and style-module names frozen for 1.0.
fn check_one_point_zero_allowlist(root: &Path, violations: &mut Vec<String>) -> io::Result<()> {
    let modules = allowed_modules(root)?;
    for module in &modules {
        let relative = match module.path.as_str() {
            "jlreq" => "lib.rs".to_owned(),
            path if path.starts_with("jlreq::") => {
                format!(
                    "{}.rs",
                    path.trim_start_matches("jlreq::").replace("::", "/")
                )
            },
            path => {
                return Err(malformed(format!(
                    "docs/api-1.0.toml names `{path}`; the only public Rust crate is `jlreq`"
                )));
            },
        };
        let source_path = root.join("crates").join("jlreq").join("src").join(relative);
        let source = fs::read_to_string(source_path)?;
        violations.extend(check_allowed_items(module, &source));
    }
    println!(
        "api: docs/api-1.0.toml freezes {items} item name(s) across {modules} public module(s).",
        items = modules
            .iter()
            .map(|module| module.items.len())
            .sum::<usize>(),
        modules = modules.len()
    );
    Ok(())
}

// -------------------------------------------------------------------------------------
// The control file
// -------------------------------------------------------------------------------------

/// One `[[table]]` entry of `docs/api-frozen.toml`.
///
/// A value is kept as the string literals it contains, in order, which is what four of the
/// five tables this gate reads are made of. The fifth, `[[closed_choices]]`, states a
/// `count` that holds no string literal, so a bare integer is kept beside the strings —
/// without it the one table whose whole content is a number would read as empty and the
/// check over it would pass over nothing.
#[derive(Debug, Default)]
struct Entry {
    /// The table name, without its brackets.
    table: String,
    /// The keys of this entry, each with the strings its value holds.
    values: BTreeMap<String, Vec<String>>,
    /// The keys of this entry whose value is a bare integer.
    counts: BTreeMap<String, usize>,
}

impl Entry {
    /// The strings a key holds, or an empty slice when it has none.
    fn list(&self, key: &str) -> &[String] {
        self.values.get(key).map_or(&[], Vec::as_slice)
    }

    /// The single string a key holds.
    fn single(&self, key: &str) -> Option<&str> {
        self.list(key).first().map(String::as_str)
    }

    /// The integer a key holds, when its value is a bare number.
    fn count(&self, key: &str) -> Option<usize> {
        self.counts.get(key).copied()
    }
}

/// A type named by the control file, split into the crate that declares it and its name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct TypePath {
    /// The crate's name as a Rust path segment: `jlreq_spec`.
    crate_path: String,
    /// The type's own name: `Provenance`.
    name: String,
}

impl TypePath {
    /// Split `jlreq_spec::Provenance`.
    fn parse(text: &str) -> Option<Self> {
        let (crate_path, name) = text.split_once("::")?;
        (!crate_path.is_empty() && !name.is_empty() && !name.contains("::")).then(|| Self {
            crate_path: crate_path.to_owned(),
            name: name.to_owned(),
        })
    }
}

impl std::fmt::Display for TypePath {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { crate_path, name } = self;
        write!(formatter, "{crate_path}::{name}")
    }
}

/// `docs/api-frozen.toml`, in the four tables this gate reads.
#[derive(Debug)]
struct Control {
    /// Each open output type and the total accessors that project it.
    frozen: Vec<(TypePath, Vec<String>)>,
    /// Each type allowed to be exhaustive, with the sentence that closes it.
    exempt: Vec<(TypePath, String)>,
    /// Each shape that may never appear, over the crates it is forbidden in.
    forbidden: Vec<Forbidden>,
    /// Each policy question whose answer set may never grow, with the size it is fixed at.
    closed_choices: Vec<ClosedChoices>,
}

/// One `[[closed_choices]]` entry: a question path and the number of answers it may permit.
#[derive(Debug)]
struct ClosedChoices {
    /// The stable dotted path, as `spec/derived/questions.tsv` writes it.
    question: String,
    /// How many answers the specification closes the set at.
    count: usize,
}

/// One `[[forbidden]]` entry: a name guard or a signature guard, over named crates.
#[derive(Debug)]
struct Forbidden {
    /// The crates the guard covers, by package name.
    crates: Vec<String>,
    /// Item identifiers that may not be declared publicly there.
    item_names: Vec<String>,
    /// A whole signature that may not appear there, as `(type, type) -> type`.
    signature: Option<String>,
}

impl Control {
    /// Read and validate `docs/api-frozen.toml`.
    ///
    /// A missing file, an unreadable entry, or a table with no entries at all is an error
    /// rather than a pass: an empty `[[forbidden]]` table would switch a guard off in
    /// silence, which is the one failure mode a control may not have.
    fn read(root: &Path) -> io::Result<Self> {
        let path = root.join("docs").join("api-frozen.toml");
        let text = fs::read_to_string(&path)?;
        let entries = entries_of(&text);
        let control = Self {
            frozen: frozen_entries(&entries)?,
            exempt: exempt_entries(&entries)?,
            forbidden: forbidden_entries(&entries)?,
            closed_choices: closed_choice_entries(&entries)?,
        };
        for (table, count) in [
            ("frozen", control.frozen.len()),
            ("exempt", control.exempt.len()),
            ("forbidden", control.forbidden.len()),
            ("closed_choices", control.closed_choices.len()),
        ] {
            if count == 0 {
                return Err(malformed(format!(
                    "docs/api-frozen.toml has no `[[{table}]]` entries; this gate reads that \
                     table and an empty one would disable a check in silence"
                )));
            }
        }
        Ok(control)
    }

    /// Every crate the control file names must be a workspace member.
    ///
    /// An entry naming a crate that is not one is an error rather than a no-op, so the
    /// file cannot rot into a guard over nothing.
    fn resolve_against(&self, surface: &Surface) -> io::Result<()> {
        for path in self.frozen.iter().map(|(each, _)| each) {
            surface
                .by_path(&path.crate_path)
                .ok_or_else(|| unknown(path))?;
        }
        for path in self.exempt.iter().map(|(each, _)| each) {
            surface
                .by_path(&path.crate_path)
                .ok_or_else(|| unknown(path))?;
        }
        for entry in &self.forbidden {
            for name in &entry.crates {
                if surface.by_name(name).is_none() {
                    return Err(malformed(format!(
                        "docs/api-frozen.toml forbids a shape in `{name}`, which is not a \
                         workspace member"
                    )));
                }
            }
        }
        Ok(())
    }

    /// The sentence closing a type, when the file exempts it from being open.
    fn exemption(&self, path: &TypePath) -> Option<&str> {
        self.exempt
            .iter()
            .find(|(each, _)| each == path)
            .map(|(_, why)| why.as_str())
    }

    /// The names of the exempt types, whose answer sets the specification closes.
    fn closed_type_names(&self) -> BTreeSet<&str> {
        self.exempt
            .iter()
            .map(|(path, _)| path.name.as_str())
            .collect()
    }
}

/// The error for a control file that cannot be read as one.
fn malformed(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

/// The error for a control entry naming a crate this workspace does not have.
fn unknown(path: &TypePath) -> io::Error {
    malformed(format!(
        "docs/api-frozen.toml names `{path}`, whose crate is not a workspace member"
    ))
}

/// The `[[frozen]]` entries: an open type and the projections that must stay total.
fn frozen_entries(entries: &[Entry]) -> io::Result<Vec<(TypePath, Vec<String>)>> {
    let mut frozen = Vec::new();
    for entry in entries.iter().filter(|entry| entry.table == "frozen") {
        let path = named_type(entry, "frozen")?;
        let projections = entry.list("projections");
        if projections.is_empty() {
            return Err(malformed(format!(
                "the `[[frozen]]` entry for `{path}` names no projections; an open type \
                 without one is what ADR 0012 forbids"
            )));
        }
        frozen.push((path, projections.to_vec()));
    }
    Ok(frozen)
}

/// The `[[exempt]]` entries: a closed type and the sentence that closes it.
fn exempt_entries(entries: &[Entry]) -> io::Result<Vec<(TypePath, String)>> {
    let mut exempt = Vec::new();
    for entry in entries.iter().filter(|entry| entry.table == "exempt") {
        let path = named_type(entry, "exempt")?;
        let why = entry.single("why").ok_or_else(|| {
            malformed(format!(
                "the `[[exempt]]` entry for `{path}` states no reason; an exemption without \
                 the sentence that closes the set is an opinion"
            ))
        })?;
        exempt.push((path, why.to_owned()));
    }
    Ok(exempt)
}

/// The `[[forbidden]]` entries: a name guard or a signature guard.
fn forbidden_entries(entries: &[Entry]) -> io::Result<Vec<Forbidden>> {
    let mut forbidden = Vec::new();
    for entry in entries.iter().filter(|entry| entry.table == "forbidden") {
        let crates = entry.list("crates").to_vec();
        let item_names = entry.list("item_names").to_vec();
        let signature = entry.single("signature").map(str::to_owned);
        if crates.is_empty() || (item_names.is_empty() && signature.is_none()) {
            return Err(malformed(
                "a `[[forbidden]]` entry names no crates, or neither `item_names` nor a \
                 `signature`; it would forbid nothing"
                    .to_owned(),
            ));
        }
        forbidden.push(Forbidden {
            crates,
            item_names,
            signature,
        });
    }
    Ok(forbidden)
}

/// The `[[closed_choices]]` entries: a question path and the size its answer set is fixed at.
///
/// The comment in the control file calls a count that may never grow a control only if it is
/// written down before the data that could grow it, which is exactly the position this table
/// was in until the policy space was derived. It is read now, so the sentence is true.
fn closed_choice_entries(entries: &[Entry]) -> io::Result<Vec<ClosedChoices>> {
    let mut closed = Vec::new();
    for entry in entries
        .iter()
        .filter(|entry| entry.table == "closed_choices")
    {
        let question = entry.single("question").ok_or_else(|| {
            malformed(
                "a `[[closed_choices]]` entry names no `question`; a closed set belongs to \
                 one place JLReq permits more than one answer"
                    .to_owned(),
            )
        })?;
        let count = entry.count("count").ok_or_else(|| {
            malformed(format!(
                "the `[[closed_choices]]` entry for `{question}` states no `count`; the \
                 whole content of the entry is the number the set may never exceed"
            ))
        })?;
        if entry.single("why").is_none() {
            return Err(malformed(format!(
                "the `[[closed_choices]]` entry for `{question}` states no reason; a closed \
                 set without the sentence that closes it is an opinion"
            )));
        }
        closed.push(ClosedChoices {
            question: question.to_owned(),
            count,
        });
    }
    Ok(closed)
}

/// The `type` key of an entry, as a crate and a name.
fn named_type(entry: &Entry, table: &str) -> io::Result<TypePath> {
    let text = entry
        .single("type")
        .ok_or_else(|| malformed(format!("a `[[{table}]]` entry has no `type` key")))?;
    TypePath::parse(text).ok_or_else(|| {
        malformed(format!(
            "the `[[{table}]]` entry `{text}` is not a `crate_path::Type`"
        ))
    })
}

/// Read a TOML document as a list of array-table entries.
///
/// Hand-rolled for the reason the module states, and it understands exactly the shape this
/// repository writes: `[[table]]` headers, `key = "value"` and `key = [...]` on one line or
/// several. Bracket counting is done after the string literals are removed, so a bracket
/// inside a quoted sentence does not open an array.
fn entries_of(text: &str) -> Vec<Entry> {
    let mut entries: Vec<Entry> = Vec::new();
    let mut open: Option<String> = None;

    for raw in text.lines() {
        let line = without_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        if let Some(name) = array_table(line) {
            entries.push(Entry {
                table: name.to_owned(),
                values: BTreeMap::new(),
                counts: BTreeMap::new(),
            });
            open = None;
            continue;
        }
        let Some(entry) = entries.last_mut() else {
            continue;
        };
        if let Some(key) = open.clone() {
            entry
                .values
                .entry(key)
                .or_default()
                .extend(quoted(line).into_iter().map(str::to_owned));
            if outside_strings(line).contains(']') {
                open = None;
            }
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim().to_owned();
            entry
                .values
                .entry(key.clone())
                .or_default()
                .extend(quoted(value).into_iter().map(str::to_owned));
            let bare = outside_strings(value);
            if let Ok(number) = bare.trim().parse::<usize>() {
                entry.counts.insert(key.clone(), number);
            }
            if bare.contains('[') && !bare.contains(']') {
                open = Some(key);
            }
        }
    }
    entries
}

/// The name inside a `[[table]]` header, if the line is one.
fn array_table(line: &str) -> Option<&str> {
    line.strip_prefix("[[")
        .and_then(|rest| rest.strip_suffix("]]"))
        .map(str::trim)
}

/// Everything before the first `#` that is not inside a string.
fn without_comment(line: &str) -> &str {
    let mut inside = false;
    for (index, character) in line.char_indices() {
        match character {
            '"' => inside = !inside,
            '#' if !inside => return line.get(..index).unwrap_or(line),
            _ => {},
        }
    }
    line
}

/// The string literals on a line, in order, without their quotes.
fn quoted(line: &str) -> Vec<&str> {
    let mut values = Vec::new();
    let mut rest = line;
    while let Some((_, after)) = rest.split_once('"') {
        let Some((value, remainder)) = after.split_once('"') else {
            break;
        };
        values.push(value);
        rest = remainder;
    }
    values
}

/// The line with every quoted string removed, so its brackets can be counted.
fn outside_strings(line: &str) -> String {
    let mut bare = String::new();
    let mut inside = false;
    for character in line.chars() {
        match character {
            '"' => inside = !inside,
            _ if !inside => bare.push(character),
            _ => {},
        }
    }
    bare
}

// -------------------------------------------------------------------------------------
// The workspace surface
// -------------------------------------------------------------------------------------

/// Whether ADR 0012's promise covers a crate, which its own manifest decides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Publication {
    /// The crate is published, so its surface is an adopter's.
    Published,
    /// The crate states `publish = false`, so it has no adopters to keep.
    Internal,
}

/// One workspace member and everything it declares.
#[derive(Debug)]
struct Member {
    /// The package name its own manifest declares: `jlreq-unit`.
    name: String,
    /// That name as a Rust path segment: `jlreq_unit`.
    path: String,
    /// Whether the crate is published.
    publication: Publication,
    /// How many source files were read, which the report states.
    files: usize,
    /// Every item the crate declares.
    declarations: Vec<Declaration>,
}

impl Member {
    /// The public types this crate declares.
    fn public_types(&self) -> impl Iterator<Item = &Declaration> {
        self.declarations
            .iter()
            .filter(|each| each.visibility == Visibility::Public && each.kind.is_type())
    }

    /// The declaration of a type this crate declares, public or not.
    fn declared_type(&self, name: &str) -> Option<&Declaration> {
        self.declarations
            .iter()
            .find(|each| each.kind.is_type() && each.name == name)
    }

    /// The public associated items of one type.
    fn associated(&self, type_name: &str) -> impl Iterator<Item = &Declaration> {
        self.declarations.iter().filter(move |each| {
            each.visibility == Visibility::Public
                && each.owner.as_ref().is_some_and(|owner| {
                    owner.association == Association::Inherent && owner.type_name == type_name
                })
        })
    }

    /// The public functions this crate declares, free and associated alike.
    fn public_functions(&self) -> impl Iterator<Item = &Declaration> {
        self.declarations
            .iter()
            .filter(|each| each.kind == Kind::Function && each.visibility == Visibility::Public)
    }
}

/// Every workspace member, read once.
#[derive(Debug)]
struct Surface {
    /// The members, in the order the workspace manifest lists them.
    members: Vec<Member>,
}

impl Surface {
    /// Read every workspace member's sources.
    fn read(root: &Path) -> io::Result<Self> {
        let manifest = fs::read_to_string(root.join("Cargo.toml"))?;
        let listed = workspace_members(&manifest);
        if listed.is_empty() {
            return Err(malformed(
                "Cargo.toml declares no workspace members".to_owned(),
            ));
        }
        let mut members = Vec::new();
        for directory in listed {
            members.push(read_member(&root.join(&directory), &directory)?);
        }
        Ok(Self { members })
    }

    /// Published members plus the blocked jlreq release candidate.
    fn published(&self) -> impl Iterator<Item = &Member> {
        self.members
            .iter()
            .filter(|member| member.publication == Publication::Published || member.name == "jlreq")
    }

    /// A member by its Rust path segment.
    fn by_path(&self, path: &str) -> Option<&Member> {
        self.members.iter().find(|member| member.path == path)
    }

    /// A member by its package name.
    fn by_name(&self, name: &str) -> Option<&Member> {
        self.members.iter().find(|member| member.name == name)
    }

    /// Every type name appearing in an input position anywhere in the workspace.
    ///
    /// The receiver is excluded at parse time and `Self` is resolved to the type it is
    /// written in, so a method taking `rhs: Self` puts its own type in an input position,
    /// which it does. Names are matched without their crate, because a parameter is written
    /// with the name and two crates are free to declare the same one; that can only add an
    /// obligation, never drop one.
    fn input_positions(&self) -> BTreeSet<&str> {
        let mut names = BTreeSet::new();
        for member in &self.members {
            for declaration in member.public_functions() {
                for parameter in &declaration.signature.parameters {
                    names.extend(
                        parameter
                            .iter()
                            .filter(|token| is_name(token))
                            .map(String::as_str),
                    );
                }
            }
        }
        names
    }
}

/// Read one member: its manifest, then every source file under `src`.
fn read_member(directory: &Path, listed: &str) -> io::Result<Member> {
    let manifest = fs::read_to_string(directory.join("Cargo.toml"))?;
    let name = package_name(&manifest)
        .ok_or_else(|| malformed(format!("{listed}/Cargo.toml declares no package name")))?
        .to_owned();
    let publication = if publishes(&manifest) {
        Publication::Published
    } else {
        Publication::Internal
    };
    let sources = shared::rust_sources(&directory.join("src"))?;
    let mut declarations = Vec::new();
    for source in &sources {
        let file = shared::relative_name(source, directory).replace('\\', "/");
        let text = fs::read_to_string(source)?;
        declarations.extend(declarations_of(&text, &file));
    }
    Ok(Member {
        path: name.replace('-', "_"),
        name,
        publication,
        files: sources.len(),
        declarations,
    })
}

/// Read the member paths out of the workspace manifest.
///
/// The same one-form scan `shared` performs for the layout core, repeated here because
/// this gate needs every member and not only the core ones, and because a gate module owns
/// its own reading rather than widening what another gate shares.
fn workspace_members(manifest: &str) -> Vec<String> {
    let mut members = Vec::new();
    let mut inside_workspace = false;
    let mut inside_members = false;

    for line in manifest.lines() {
        let line = without_comment(line).trim();
        if line.is_empty() {
            continue;
        }
        if inside_members {
            members.extend(quoted(line).into_iter().map(str::to_owned));
            inside_members = !line.contains(']');
            continue;
        }
        if let Some(header) = line
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            inside_workspace = header.trim() == "workspace";
            continue;
        }
        if !inside_workspace {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() == "members" {
            members.extend(quoted(value).into_iter().map(str::to_owned));
            inside_members = !value.contains(']');
        }
    }
    members
}

/// The package name a crate manifest declares.
fn package_name(manifest: &str) -> Option<&str> {
    package_value(manifest, "name").and_then(|value| quoted(value).first().copied())
}

/// Whether a crate manifest leaves itself publishable.
fn publishes(manifest: &str) -> bool {
    package_value(manifest, "publish").is_none_or(|value| value.trim() != "false")
}

/// The raw value of one key of the `[package]` table.
fn package_value<'m>(manifest: &'m str, key: &str) -> Option<&'m str> {
    let mut inside_package = false;
    for line in manifest.lines() {
        let line = without_comment(line).trim();
        if let Some(header) = line
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            inside_package = header.trim() == "package";
            continue;
        }
        if !inside_package {
            continue;
        }
        let Some((found, value)) = line.split_once('=') else {
            continue;
        };
        if found.trim() == key {
            return Some(value);
        }
    }
    None
}

// -------------------------------------------------------------------------------------
// Reading a source file
// -------------------------------------------------------------------------------------

/// What kind of item a declaration is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// `struct`.
    Struct,
    /// `enum`.
    Enum,
    /// `union`.
    Union,
    /// `trait`.
    Trait,
    /// `type`.
    Alias,
    /// `const`.
    Constant,
    /// `static`.
    Static,
    /// `mod`.
    Module,
    /// `fn`.
    Function,
    /// A name a `pub use` publishes, which is how this workspace exports its items.
    Reexport,
}

impl Kind {
    /// Whether this kind declares a type, which is what `#[non_exhaustive]` governs.
    const fn is_type(self) -> bool {
        matches!(self, Self::Struct | Self::Enum | Self::Union)
    }
}

/// Whether an item is written `pub`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Visibility {
    /// Declared `pub`, with no restriction.
    Public,
    /// Private, or restricted to a scope: `pub(crate)`, `pub(super)`, `pub(in ..)`.
    Restricted,
}

/// Whether a type carries `#[non_exhaustive]`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Openness {
    /// No `#[non_exhaustive]`: the caller may match it exhaustively and build it.
    #[default]
    Exhaustive,
    /// `#[non_exhaustive]`: jlreq may still add detail here.
    Open,
}

/// Whether `#[non_exhaustive]` leaves a caller any way to name a value.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Construction {
    /// A value can still be named from outside: any exhaustive type, and an open enum
    /// whose variants are not themselves open.
    #[default]
    Nameable,
    /// Nothing outside the crate can name a value, so an entry point is required.
    Sealed,
}

/// Whether an item is gated to test builds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Gating {
    /// Compiled always.
    #[default]
    Always,
    /// `#[cfg(test)]`: not part of any surface.
    TestOnly,
}

/// How an associated item reaches its type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Association {
    /// An inherent `impl Type`.
    Inherent,
    /// An `impl Trait for Type`, which is the trait's surface rather than the type's.
    Trait,
}

/// Whether a function takes a receiver.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Receiver {
    /// An associated function: no `self`.
    #[default]
    Absent,
    /// A method.
    Present,
}

/// The type an associated item belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Owner {
    /// The type's name, without its generic arguments.
    type_name: String,
    /// Whether the item is inherent to it.
    association: Association,
}

/// A function's parameter and result types, with `Self` resolved.
///
/// A constant or a static carries its declared type in `result` and nothing in
/// `parameters`, because the question asked of both is the same one: what type does this
/// item hand the caller.
#[derive(Debug, Clone, Default)]
struct Signature {
    /// Whether a receiver was written.
    receiver: Receiver,
    /// One entry per parameter other than the receiver, as its type's tokens.
    parameters: Vec<Vec<String>>,
    /// The result type's tokens; empty when there is none.
    result: Vec<String>,
}

/// Where a declaration is written.
#[derive(Debug, Clone)]
struct Origin {
    /// The file, relative to the crate directory, with forward slashes.
    file: String,
    /// The line it starts on, counting from one.
    line: usize,
}

/// One item a source file declares.
#[derive(Debug, Clone)]
struct Declaration {
    /// What kind of item it is.
    kind: Kind,
    /// Its identifier.
    name: String,
    /// Whether it is written `pub`.
    visibility: Visibility,
    /// Whether it carries `#[non_exhaustive]`.
    openness: Openness,
    /// Whether a caller outside the crate can name a value of it.
    construction: Construction,
    /// The type it is associated with, when it is an associated item.
    owner: Option<Owner>,
    /// Its types.
    signature: Signature,
    /// Where it is written.
    origin: Origin,
}

impl Declaration {
    /// `jlreq-unit/src/axis.rs:98`, the way a reader finds it.
    fn at(&self, member: &Member) -> String {
        let Origin { file, line } = &self.origin;
        let name = &member.name;
        format!("{name}/{file}:{line}")
    }
}

/// Read every declaration of one source file.
fn declarations_of(source: &str, file: &str) -> Vec<Declaration> {
    let tokens = tokenize(&code_only(source));
    let mut cursor = Cursor::new(&tokens);
    let scope = Scope {
        owner: None,
        default_visibility: Visibility::Restricted,
        file: file.to_owned(),
    };
    let mut found = Found::default();
    parse_block(&mut cursor, &scope, &mut found);
    found.declarations
}

/// What one file's parse produces: its declarations, and the macros it defines.
#[derive(Debug, Default)]
struct Found {
    /// Every declaration read so far, including macro expansions.
    declarations: Vec<Declaration>,
    /// The `macro_rules!` surfaces defined so far, in definition order.
    macros: Vec<MacroRules>,
}

/// A `macro_rules!` definition whose expansion declares items.
#[derive(Debug)]
struct MacroRules {
    /// The macro's name.
    name: String,
    /// Its metavariables, in the order the rule's pattern binds them.
    parameters: Vec<String>,
    /// The declarations its expansion makes, still written in metavariables.
    declarations: Vec<Declaration>,
}

/// What a block's items are associated with.
#[derive(Debug, Clone)]
struct Scope {
    /// The type an associated item belongs to.
    owner: Option<Owner>,
    /// The visibility an item takes when it states none: a trait's items are as public as
    /// the trait, and everything else is private until it says otherwise.
    default_visibility: Visibility,
    /// The file being read.
    file: String,
}

// -------------------------------------------------------------------------------------
// Stripping and tokenizing
// -------------------------------------------------------------------------------------

/// Replace every comment, string literal and character literal with spaces.
///
/// Line structure survives exactly — each removed character becomes a space and every
/// newline stays — so a finding still names the line it is on. Nested block comments, raw
/// strings and byte strings are all handled, and a lifetime is not mistaken for a
/// character literal. `shared::code_only` strips only `//`, which is enough for a token
/// scan and not enough for a parse.
fn code_only(source: &str) -> String {
    let characters: Vec<char> = source.chars().collect();
    let mut cleaned = String::with_capacity(source.len());
    let mut index = 0_usize;
    while let Some(&character) = characters.get(index) {
        index = match character {
            '/' if characters.get(index.saturating_add(1)) == Some(&'/') => {
                blank_line_comment(&characters, index, &mut cleaned)
            },
            '/' if characters.get(index.saturating_add(1)) == Some(&'*') => {
                blank_block_comment(&characters, index, &mut cleaned)
            },
            '"' => blank_quoted(&characters, index, '"', &mut cleaned),
            '\'' if is_character_literal(&characters, index) => {
                blank_quoted(&characters, index, '\'', &mut cleaned)
            },
            'r' | 'b' if starts_raw_string(&characters, index) => {
                blank_raw_string(&characters, index, &mut cleaned)
            },
            _ => {
                cleaned.push(character);
                index.saturating_add(1)
            },
        };
    }
    cleaned
}

/// The blank that replaces one removed character: a newline stays a newline.
const fn blank_of(character: char) -> char {
    if character == '\n' { '\n' } else { ' ' }
}

/// Blank a `//` comment up to, but not including, the end of its line.
fn blank_line_comment(characters: &[char], start: usize, cleaned: &mut String) -> usize {
    let mut index = start;
    while let Some(&character) = characters.get(index) {
        if character == '\n' {
            break;
        }
        cleaned.push(' ');
        index = index.saturating_add(1);
    }
    index
}

/// Blank a `/* */` comment, honoring Rust's nesting.
fn blank_block_comment(characters: &[char], start: usize, cleaned: &mut String) -> usize {
    let mut index = start;
    let mut depth = 0_usize;
    while let Some(&character) = characters.get(index) {
        let next = characters.get(index.saturating_add(1)).copied();
        let pair = match (character, next) {
            ('/', Some('*')) => {
                depth = depth.saturating_add(1);
                true
            },
            ('*', Some('/')) => {
                depth = depth.saturating_sub(1);
                true
            },
            _ => false,
        };
        if pair {
            cleaned.push(' ');
            cleaned.push(' ');
            index = index.saturating_add(2);
            if depth == 0 {
                break;
            }
            continue;
        }
        cleaned.push(blank_of(character));
        index = index.saturating_add(1);
    }
    index
}

/// Blank a literal delimited by `quote`, honoring backslash escapes.
fn blank_quoted(characters: &[char], start: usize, quote: char, cleaned: &mut String) -> usize {
    let mut index = start.saturating_add(1);
    cleaned.push(' ');
    while let Some(&character) = characters.get(index) {
        cleaned.push(blank_of(character));
        index = index.saturating_add(1);
        if character == quote {
            break;
        }
        if character != '\\' {
            continue;
        }
        if let Some(&escaped) = characters.get(index) {
            cleaned.push(blank_of(escaped));
            index = index.saturating_add(1);
        }
    }
    index
}

/// Whether the quote at `start` opens a character literal rather than a lifetime.
fn is_character_literal(characters: &[char], start: usize) -> bool {
    match characters.get(start.saturating_add(1)) {
        Some('\\') => true,
        Some(_) => characters.get(start.saturating_add(2)) == Some(&'\''),
        None => false,
    }
}

/// Whether a raw string literal starts here: `r"`, `r#"`, `br##"`, and so on.
fn starts_raw_string(characters: &[char], start: usize) -> bool {
    let mut index = start;
    if characters.get(index) == Some(&'b') {
        index = index.saturating_add(1);
    }
    if characters.get(index) != Some(&'r') {
        return false;
    }
    index = index.saturating_add(1);
    while characters.get(index) == Some(&'#') {
        index = index.saturating_add(1);
    }
    characters.get(index) == Some(&'"')
}

/// Blank a raw string literal, matching its own hash count.
fn blank_raw_string(characters: &[char], start: usize, cleaned: &mut String) -> usize {
    let mut index = start;
    let mut hashes = 0_usize;
    while let Some(&character) = characters.get(index) {
        cleaned.push(' ');
        index = index.saturating_add(1);
        match character {
            '#' => hashes = hashes.saturating_add(1),
            '"' => break,
            _ => {},
        }
    }
    while let Some(&character) = characters.get(index) {
        cleaned.push(blank_of(character));
        index = index.saturating_add(1);
        if character == '"' && closes_raw_string(characters, index, hashes) {
            for _ in 0..hashes {
                cleaned.push(' ');
                index = index.saturating_add(1);
            }
            break;
        }
    }
    index
}

/// Whether `hashes` hashes follow, closing a raw string.
fn closes_raw_string(characters: &[char], start: usize, hashes: usize) -> bool {
    let mut index = start;
    for _ in 0..hashes {
        if characters.get(index) != Some(&'#') {
            return false;
        }
        index = index.saturating_add(1);
    }
    true
}

/// One token: an identifier, a metavariable, a number, or punctuation.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Token {
    /// The token's text.
    text: String,
    /// The line it starts on, counting from one.
    line: usize,
}

/// Split cleaned source into tokens.
///
/// `::`, `->` and `=>` are single tokens because the parse reads them as one thing;
/// everything else that is not a name is one character, which keeps bracket counting
/// honest.
fn tokenize(cleaned: &str) -> Vec<Token> {
    let characters: Vec<char> = cleaned.chars().collect();
    let mut tokens = Vec::new();
    let mut index = 0_usize;
    let mut line = 1_usize;
    while let Some(&character) = characters.get(index) {
        if character == '\n' {
            line = line.saturating_add(1);
            index = index.saturating_add(1);
            continue;
        }
        if character.is_whitespace() {
            index = index.saturating_add(1);
            continue;
        }
        let (text, next) = if character == '$' || is_name_start(character) {
            read_name(&characters, index)
        } else {
            read_punctuation(&characters, index)
        };
        tokens.push(Token { text, line });
        index = next;
    }
    tokens
}

/// Whether a character can start a name.
fn is_name_start(character: char) -> bool {
    character.is_alphabetic() || character == '_'
}

/// Whether a token is a name rather than punctuation.
fn is_name(token: &str) -> bool {
    token.chars().next().is_some_and(is_name_start)
}

/// Whether a token can name a declared item.
///
/// A `macro_rules!` expansion writes the name it declares as a metavariable — `impl $type`
/// — and the invocation binds it. Reading `$type` as a name here is what lets the
/// expansion be instantiated later; a token that is still a metavariable when a check runs
/// would mean an invocation was never matched to its definition, and no check accepts one.
fn is_item_name(token: &str) -> bool {
    is_name(token) || token.starts_with('$')
}

/// Read an identifier, or a `$name` metavariable, from `start`.
fn read_name(characters: &[char], start: usize) -> (String, usize) {
    let mut text = String::new();
    let mut index = start;
    if characters.get(index) == Some(&'$') {
        text.push('$');
        index = index.saturating_add(1);
    }
    while let Some(&character) = characters.get(index) {
        if !character.is_alphanumeric() && character != '_' {
            break;
        }
        text.push(character);
        index = index.saturating_add(1);
    }
    (text, index)
}

/// Read one punctuation token from `start`.
fn read_punctuation(characters: &[char], start: usize) -> (String, usize) {
    let next = start.saturating_add(1);
    let pair: Option<String> = characters
        .get(start)
        .zip(characters.get(next))
        .map(|(first, second)| [*first, *second].iter().collect::<String>())
        .filter(|pair| ["::", "->", "=>"].contains(&pair.as_str()));
    match pair {
        Some(text) => (text, start.saturating_add(2)),
        None => (characters.get(start).copied().into_iter().collect(), next),
    }
}

// -------------------------------------------------------------------------------------
// Parsing items
// -------------------------------------------------------------------------------------

/// A position in a token stream that only moves forward.
#[derive(Debug, Clone, Copy)]
struct Cursor<'t> {
    /// The tokens not yet read.
    rest: &'t [Token],
}

impl<'t> Cursor<'t> {
    /// Start at the first token.
    const fn new(tokens: &'t [Token]) -> Self {
        Self { rest: tokens }
    }

    /// The next token, unread.
    const fn peek(self) -> Option<&'t Token> {
        self.rest.first()
    }

    /// The token after the next one, unread.
    fn peek_second(self) -> Option<&'t Token> {
        self.rest.get(1)
    }

    /// Whether the next token has this text.
    fn at(self, text: &str) -> bool {
        self.peek().is_some_and(|token| token.text == text)
    }

    /// Read the next token.
    fn take(&mut self) -> Option<&'t Token> {
        let (head, tail) = self.rest.split_first()?;
        self.rest = tail;
        Some(head)
    }

    /// Read the next token when it has this text, and report whether it did.
    fn eat(&mut self, text: &str) -> bool {
        let found = self.at(text);
        if found {
            self.take();
        }
        found
    }

    /// Read a name, if one is next.
    fn take_name(&mut self) -> Option<String> {
        let name = self
            .peek()
            .filter(|token| is_item_name(&token.text))?
            .text
            .clone();
        self.take();
        Some(name)
    }

    /// Read a balanced group starting at the opening delimiter, returning what is inside
    /// it. Nothing is read when the opening delimiter is not next.
    fn take_group(&mut self, open: &str, close: &str) -> Vec<Token> {
        let mut inside = Vec::new();
        if !self.eat(open) {
            return inside;
        }
        let mut depth = 1_usize;
        while let Some(token) = self.take() {
            if token.text == open {
                depth = depth.saturating_add(1);
            } else if token.text == close {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    break;
                }
            }
            inside.push(token.clone());
        }
        inside
    }

    /// Read up to, but not including, the next token with one of these texts that is not
    /// nested inside a delimiter.
    ///
    /// The nesting matters: the result type of
    /// `const fn identifiers() -> [RuleId; RULES.len()]` contains a semicolon, and a scan
    /// that stopped at it would read the rest of the file as the inside of an array and
    /// lose every declaration after it — which is exactly what an earlier draft of this
    /// gate did, silently, while reporting that it had examined the crate.
    fn take_until(&mut self, stops: &[&str]) -> Vec<Token> {
        let mut taken = Vec::new();
        let mut depth = 0_usize;
        while let Some(token) = self.peek() {
            let text = token.text.as_str();
            if depth == 0 && stops.contains(&text) {
                break;
            }
            match text {
                "(" | "[" | "{" | "<" => depth = depth.saturating_add(1),
                ")" | "]" | "}" | ">" => depth = depth.saturating_sub(1),
                _ => {},
            }
            taken.push(token.clone());
            self.take();
        }
        taken
    }

    /// Skip a balanced brace group, or one token when no group is next.
    fn skip_body(&mut self) {
        if self.at("{") {
            self.take_group("{", "}");
        } else {
            self.take();
        }
    }

    /// Skip to the end of a statement, ignoring semicolons inside nested groups.
    fn skip_statement(&mut self) {
        let mut depth = 0_usize;
        while let Some(token) = self.take() {
            match token.text.as_str() {
                "{" | "(" | "[" => depth = depth.saturating_add(1),
                "}" | ")" | "]" => depth = depth.saturating_sub(1),
                ";" if depth == 0 => break,
                _ => {},
            }
        }
    }
}

/// The attributes written on the item about to be read.
#[derive(Debug, Default)]
struct Attributes {
    /// Whether `#[non_exhaustive]` is among them.
    openness: Openness,
    /// Whether `#[cfg(test)]` is among them.
    gating: Gating,
}

/// Read every item of one block, stopping after the closing brace or at the last token.
fn parse_block(cursor: &mut Cursor, scope: &Scope, found: &mut Found) {
    let mut attributes = Attributes::default();
    while let Some(token) = cursor.peek() {
        match token.text.as_str() {
            "}" => {
                cursor.take();
                return;
            },
            "#" => {
                read_attribute(cursor, &mut attributes);
                continue;
            },
            ";" => {
                cursor.take();
                attributes = Attributes::default();
                continue;
            },
            _ => {},
        }
        let visibility = read_visibility(cursor, scope.default_visibility);
        parse_item(cursor, &attributes, visibility, scope, found);
        attributes = Attributes::default();
    }
}

/// Read one `#[..]` attribute and record the two that matter.
fn read_attribute(cursor: &mut Cursor, attributes: &mut Attributes) {
    cursor.eat("#");
    cursor.eat("!");
    let inside = cursor.take_group("[", "]");
    let texts: Vec<&str> = inside.iter().map(|token| token.text.as_str()).collect();
    if texts.first() == Some(&"non_exhaustive") {
        attributes.openness = Openness::Open;
    }
    if texts.first() == Some(&"cfg") && texts.contains(&"test") {
        attributes.gating = Gating::TestOnly;
    }
}

/// Read the visibility an item states, falling back to its block's.
fn read_visibility(cursor: &mut Cursor, default: Visibility) -> Visibility {
    if !cursor.eat("pub") {
        return default;
    }
    if cursor.at("(") {
        cursor.take_group("(", ")");
        return Visibility::Restricted;
    }
    Visibility::Public
}

/// Read one item, whatever kind it is. Always consumes at least one token.
fn parse_item(
    cursor: &mut Cursor,
    attributes: &Attributes,
    visibility: Visibility,
    scope: &Scope,
    found: &mut Found,
) {
    let Some(token) = cursor.peek() else {
        return;
    };
    let line = token.line;
    match token.text.as_str() {
        "const" if cursor.peek_second().is_some_and(|next| next.text == "fn") => {
            cursor.take();
            parse_item(cursor, attributes, visibility, scope, found);
        },
        "unsafe" | "async" | "extern" | "default" => {
            cursor.take();
            parse_item(cursor, attributes, visibility, scope, found);
        },
        "use" => parse_use(cursor, visibility, scope, line, found),
        "mod" => parse_module(cursor, attributes, visibility, scope, line, found),
        "struct" | "union" => parse_struct(cursor, attributes, visibility, scope, line, found),
        "enum" => parse_enum(cursor, attributes, visibility, scope, line, found),
        "trait" => parse_trait(cursor, visibility, scope, line, found),
        "impl" => parse_impl(cursor, scope, found),
        "fn" => parse_function(cursor, visibility, scope, line, found),
        "const" | "static" => parse_constant(cursor, visibility, scope, line, found),
        "type" => parse_alias(cursor, visibility, scope, line, found),
        "macro_rules" => parse_macro_rules(cursor, scope, found),
        _ if cursor.peek_second().is_some_and(|next| next.text == "!") => {
            parse_invocation(cursor, scope, line, found);
        },
        _ => {
            cursor.take();
        },
    }
}

/// Build one declaration in the current scope.
fn declare(
    kind: Kind,
    name: String,
    visibility: Visibility,
    scope: &Scope,
    line: usize,
) -> Declaration {
    Declaration {
        kind,
        name,
        visibility,
        openness: Openness::Exhaustive,
        construction: Construction::Nameable,
        owner: scope.owner.clone(),
        signature: Signature::default(),
        origin: Origin {
            file: scope.file.clone(),
            line,
        },
    }
}

/// Read a `use` item, recording what a `pub use` publishes.
fn parse_use(
    cursor: &mut Cursor,
    visibility: Visibility,
    scope: &Scope,
    line: usize,
    found: &mut Found,
) {
    cursor.take();
    let path = cursor.take_until(&[";"]);
    cursor.eat(";");
    if visibility != Visibility::Public {
        return;
    }
    for name in reexported_names(&path) {
        found
            .declarations
            .push(declare(Kind::Reexport, name, visibility, scope, line));
    }
}

/// The names a `use` path publishes: each leaf, and each `as` alias.
fn reexported_names(path: &[Token]) -> Vec<String> {
    let texts: Vec<&str> = path.iter().map(|token| token.text.as_str()).collect();
    let mut names = Vec::new();
    let mut renaming = false;
    for (position, text) in texts.iter().enumerate() {
        if *text == "as" {
            names.pop();
            renaming = true;
            continue;
        }
        if !is_name(text) {
            continue;
        }
        if renaming {
            names.push((*text).to_owned());
            renaming = false;
            continue;
        }
        let followed_by_path = texts.get(position.saturating_add(1)) == Some(&"::");
        let structural = matches!(*text, "crate" | "self" | "super");
        if !followed_by_path && !structural {
            names.push((*text).to_owned());
        }
    }
    names
}

/// Read a `mod` item, and its body when it has one that is not test-only.
fn parse_module(
    cursor: &mut Cursor,
    attributes: &Attributes,
    visibility: Visibility,
    scope: &Scope,
    line: usize,
    found: &mut Found,
) {
    cursor.take();
    let Some(name) = cursor.take_name() else {
        return;
    };
    found
        .declarations
        .push(declare(Kind::Module, name, visibility, scope, line));
    if cursor.eat(";") {
        return;
    }
    if attributes.gating == Gating::TestOnly {
        cursor.skip_body();
        return;
    }
    if cursor.eat("{") {
        let inner = Scope {
            owner: None,
            default_visibility: Visibility::Restricted,
            file: scope.file.clone(),
        };
        parse_block(cursor, &inner, found);
    }
}

/// Read a `struct` or `union` item.
fn parse_struct(
    cursor: &mut Cursor,
    attributes: &Attributes,
    visibility: Visibility,
    scope: &Scope,
    line: usize,
    found: &mut Found,
) {
    let kind = if cursor.at("union") {
        Kind::Union
    } else {
        Kind::Struct
    };
    cursor.take();
    let Some(name) = cursor.take_name() else {
        return;
    };
    let mut declaration = declare(kind, name, visibility, scope, line);
    declaration.openness = attributes.openness;
    declaration.construction = match attributes.openness {
        Openness::Open => Construction::Sealed,
        Openness::Exhaustive => Construction::Nameable,
    };
    found.declarations.push(declaration);
    skip_type_header(cursor);
    if cursor.at("(") {
        cursor.take_group("(", ")");
    }
    if cursor.at("{") {
        cursor.take_group("{", "}");
    }
    cursor.eat(";");
}

/// Read an `enum` item, including whether its variants are themselves sealed.
fn parse_enum(
    cursor: &mut Cursor,
    attributes: &Attributes,
    visibility: Visibility,
    scope: &Scope,
    line: usize,
    found: &mut Found,
) {
    cursor.take();
    let Some(name) = cursor.take_name() else {
        return;
    };
    let mut declaration = declare(Kind::Enum, name, visibility, scope, line);
    declaration.openness = attributes.openness;
    skip_type_header(cursor);
    let body = cursor.take_group("{", "}");
    declaration.construction = match (attributes.openness, variants_are_sealed(&body)) {
        (Openness::Open, true) => Construction::Sealed,
        _ => Construction::Nameable,
    };
    found.declarations.push(declaration);
}

/// Whether an enum body has variants and every one carries `#[non_exhaustive]`.
///
/// That is the only shape in which `#[non_exhaustive]` leaves a caller no way to name a
/// value of an enum; on any other, naming a variant still works and the constructor
/// requirement would be a demand the language does not make.
fn variants_are_sealed(body: &[Token]) -> bool {
    let mut cursor = Cursor::new(body);
    let mut variants = 0_usize;
    let mut sealed = 0_usize;
    let mut attributes = Attributes::default();
    while let Some(token) = cursor.peek() {
        if token.text == "#" {
            read_attribute(&mut cursor, &mut attributes);
            continue;
        }
        if is_name(&token.text) {
            variants = variants.saturating_add(1);
            if attributes.openness == Openness::Open {
                sealed = sealed.saturating_add(1);
            }
            attributes = Attributes::default();
            cursor.take();
            skip_variant_body(&mut cursor);
            continue;
        }
        cursor.take();
    }
    variants > 0 && variants == sealed
}

/// Skip one variant's fields, discriminant and separator.
fn skip_variant_body(cursor: &mut Cursor) {
    if cursor.at("(") {
        cursor.take_group("(", ")");
    } else if cursor.at("{") {
        cursor.take_group("{", "}");
    }
    if cursor.at("=") {
        cursor.take_until(&[","]);
    }
    cursor.eat(",");
}

/// Skip a type's generic parameters and where clause, up to its body.
fn skip_type_header(cursor: &mut Cursor) {
    if cursor.at("<") {
        cursor.take_group("<", ">");
    }
    if cursor.at("where") {
        cursor.take_until(&["{", ";", "("]);
    }
}

/// Read a `trait` item; its members are as public as the trait itself.
fn parse_trait(
    cursor: &mut Cursor,
    visibility: Visibility,
    scope: &Scope,
    line: usize,
    found: &mut Found,
) {
    cursor.take();
    let Some(name) = cursor.take_name() else {
        return;
    };
    found
        .declarations
        .push(declare(Kind::Trait, name, visibility, scope, line));
    cursor.take_until(&["{", ";"]);
    if cursor.eat("{") {
        let inner = Scope {
            owner: None,
            default_visibility: visibility,
            file: scope.file.clone(),
        };
        parse_block(cursor, &inner, found);
    } else {
        cursor.eat(";");
    }
}

/// Read an `impl` block and everything associated with it.
fn parse_impl(cursor: &mut Cursor, scope: &Scope, found: &mut Found) {
    cursor.take();
    if cursor.at("<") {
        cursor.take_group("<", ">");
    }
    let head = cursor.take_until(&["{", "where"]);
    if cursor.at("where") {
        cursor.take_until(&["{"]);
    }
    let owner = impl_owner(&head);
    if !cursor.eat("{") {
        return;
    }
    let inner = Scope {
        owner,
        default_visibility: Visibility::Restricted,
        file: scope.file.clone(),
    };
    parse_block(cursor, &inner, found);
}

/// The type an `impl` head names, and whether the block is inherent.
fn impl_owner(head: &[Token]) -> Option<Owner> {
    let texts: Vec<&str> = head.iter().map(|token| token.text.as_str()).collect();
    let (association, target) = match texts.iter().position(|text| *text == "for") {
        Some(position) => (
            Association::Trait,
            texts.get(position.saturating_add(1)..).unwrap_or_default(),
        ),
        None => (Association::Inherent, texts.as_slice()),
    };
    let before_generics = match target.iter().position(|text| *text == "<") {
        Some(position) => target.get(..position).unwrap_or_default(),
        None => target,
    };
    before_generics
        .iter()
        .rev()
        .find(|text| is_item_name(text))
        .map(|name| Owner {
            type_name: (*name).to_owned(),
            association,
        })
}

/// Read a `fn` item, keeping its parameter and result types.
fn parse_function(
    cursor: &mut Cursor,
    visibility: Visibility,
    scope: &Scope,
    line: usize,
    found: &mut Found,
) {
    cursor.take();
    let Some(name) = cursor.take_name() else {
        return;
    };
    if cursor.at("<") {
        cursor.take_group("<", ">");
    }
    let parameters = cursor.take_group("(", ")");
    let result = if cursor.eat("->") {
        cursor.take_until(&["{", ";", "where"])
    } else {
        Vec::new()
    };
    if cursor.at("where") {
        cursor.take_until(&["{", ";"]);
    }
    if cursor.at("{") {
        cursor.take_group("{", "}");
    } else {
        cursor.eat(";");
    }
    let mut declaration = declare(Kind::Function, name, visibility, scope, line);
    declaration.signature = signature_of(&parameters, &result, scope.owner.as_ref());
    found.declarations.push(declaration);
}

/// Read a `const` or `static` item, keeping its declared type.
fn parse_constant(
    cursor: &mut Cursor,
    visibility: Visibility,
    scope: &Scope,
    line: usize,
    found: &mut Found,
) {
    let kind = if cursor.at("static") {
        Kind::Static
    } else {
        Kind::Constant
    };
    cursor.take();
    cursor.eat("mut");
    let Some(name) = cursor.take_name() else {
        return;
    };
    let declared = if cursor.eat(":") {
        cursor.take_until(&["=", ";"])
    } else {
        Vec::new()
    };
    cursor.skip_statement();
    let mut declaration = declare(kind, name, visibility, scope, line);
    declaration.signature.result = resolve(&declared, scope.owner.as_ref());
    found.declarations.push(declaration);
}

/// Read a `type` alias.
fn parse_alias(
    cursor: &mut Cursor,
    visibility: Visibility,
    scope: &Scope,
    line: usize,
    found: &mut Found,
) {
    cursor.take();
    let Some(name) = cursor.take_name() else {
        return;
    };
    cursor.skip_statement();
    found
        .declarations
        .push(declare(Kind::Alias, name, visibility, scope, line));
}

/// Split a parameter list and a result into a signature, resolving `Self`.
fn signature_of(parameters: &[Token], result: &[Token], owner: Option<&Owner>) -> Signature {
    let mut signature = Signature {
        result: resolve(result, owner),
        ..Signature::default()
    };
    for parameter in split_top_level(parameters, ",") {
        match parameter_type(&parameter) {
            Some(declared) => signature.parameters.push(resolve(&declared, owner)),
            None => signature.receiver = Receiver::Present,
        }
    }
    signature
}

/// One parameter's declared type, or nothing when it is the receiver.
fn parameter_type(parameter: &[Token]) -> Option<Vec<Token>> {
    let position = split_top_level(parameter, ":").first()?.len();
    let (pattern, rest) = parameter.split_at(position);
    if pattern.iter().any(|token| token.text == "self") {
        return None;
    }
    let declared: Vec<Token> = rest.iter().skip(1).cloned().collect();
    (!declared.is_empty()).then_some(declared)
}

/// Replace `Self` with the type the item is written in.
fn resolve(tokens: &[Token], owner: Option<&Owner>) -> Vec<String> {
    tokens
        .iter()
        .map(|token| match owner {
            Some(owner) if token.text == "Self" => owner.type_name.clone(),
            _ => token.text.clone(),
        })
        .collect()
}

/// Split tokens on a separator that is not nested inside a delimiter.
fn split_top_level(tokens: &[Token], separator: &str) -> Vec<Vec<Token>> {
    let mut parts = vec![Vec::new()];
    let mut depth = 0_usize;
    for token in tokens {
        match token.text.as_str() {
            "(" | "[" | "{" | "<" => depth = depth.saturating_add(1),
            ")" | "]" | "}" | ">" => depth = depth.saturating_sub(1),
            text if text == separator && depth == 0 => {
                parts.push(Vec::new());
                continue;
            },
            _ => {},
        }
        if let Some(last) = parts.last_mut() {
            last.push(token.clone());
        }
    }
    parts.retain(|part| !part.is_empty());
    parts
}

// -------------------------------------------------------------------------------------
// Parsing the two macro surfaces
// -------------------------------------------------------------------------------------

/// Read a `macro_rules!` definition, keeping the items its expansion declares.
///
/// `jlreq-unit` generates the axis channel and the closed arithmetic this way rather than
/// writing six copies, so a gate that could not see through the expansion would report
/// `InlineExtent::new` missing while it is right there. Each rule's pattern gives the
/// metavariables in order; each invocation binds them positionally.
fn parse_macro_rules(cursor: &mut Cursor, scope: &Scope, found: &mut Found) {
    cursor.take();
    cursor.eat("!");
    let Some(name) = cursor.take_name() else {
        return;
    };
    let body = cursor.take_group("{", "}");
    let mut rules = Cursor::new(&body);
    let mut parameters = Vec::new();
    let mut declarations = Vec::new();
    while !rules.rest.is_empty() {
        let pattern = rules.take_group("(", ")");
        parameters = metavariables(&pattern);
        if !rules.eat("=>") {
            rules.take();
            continue;
        }
        let expansion = rules.take_group("{", "}");
        let mut inner = Cursor::new(&expansion);
        let mut expanded = Found::default();
        parse_block(&mut inner, scope, &mut expanded);
        declarations = expanded.declarations;
        rules.eat(";");
    }
    found.macros.push(MacroRules {
        name,
        parameters,
        declarations,
    });
}

/// The metavariables a rule's pattern binds, in order and without repetition.
fn metavariables(pattern: &[Token]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for token in pattern {
        if token.text.starts_with('$') && !names.contains(&token.text) {
            names.push(token.text.clone());
        }
    }
    names
}

/// Read a macro invocation at item position, instantiating what the macro declares.
fn parse_invocation(cursor: &mut Cursor, scope: &Scope, line: usize, found: &mut Found) {
    let Some(name) = cursor.take_name() else {
        return;
    };
    cursor.eat("!");
    let arguments = if cursor.at("[") {
        cursor.take_group("[", "]")
    } else if cursor.at("{") {
        cursor.take_group("{", "}")
    } else {
        cursor.take_group("(", ")")
    };
    cursor.eat(";");

    let Some(definition) = found.macros.iter().find(|each| each.name == name) else {
        return;
    };
    let bindings = bind(&definition.parameters, &arguments);
    let instantiated: Vec<Declaration> = definition
        .declarations
        .iter()
        .map(|declaration| instantiate(declaration, &bindings, scope, line))
        .collect();
    found.declarations.extend(instantiated);
}

/// Bind a macro's metavariables to the first token of each positional argument.
fn bind(parameters: &[String], arguments: &[Token]) -> BTreeMap<String, String> {
    let mut bindings = BTreeMap::new();
    for (parameter, argument) in parameters.iter().zip(split_top_level(arguments, ",")) {
        if let Some(first) = argument.first() {
            bindings.insert(parameter.clone(), first.text.clone());
        }
    }
    bindings
}

/// Rewrite one expanded declaration with the invocation's bindings.
fn instantiate(
    declaration: &Declaration,
    bindings: &BTreeMap<String, String>,
    scope: &Scope,
    line: usize,
) -> Declaration {
    let substitute = |text: &String| bindings.get(text).cloned().unwrap_or_else(|| text.clone());
    Declaration {
        kind: declaration.kind,
        name: substitute(&declaration.name),
        visibility: declaration.visibility,
        openness: declaration.openness,
        construction: declaration.construction,
        owner: declaration.owner.as_ref().map(|owner| Owner {
            type_name: substitute(&owner.type_name),
            association: owner.association,
        }),
        signature: Signature {
            receiver: declaration.signature.receiver,
            parameters: declaration
                .signature
                .parameters
                .iter()
                .map(|parameter| parameter.iter().map(substitute).collect())
                .collect(),
            result: declaration
                .signature
                .result
                .iter()
                .map(substitute)
                .collect(),
        },
        origin: Origin {
            file: scope.file.clone(),
            line,
        },
    }
}

// -------------------------------------------------------------------------------------
// The checks
// -------------------------------------------------------------------------------------

/// Nothing in the published surface is a name this gate failed to read.
///
/// A declaration whose name is still a `macro_rules!` metavariable is an expansion no
/// invocation bound, which means the gate is blind to whatever that macro declares. Blind
/// and passing is the one outcome a control may not have, so it is reported here rather
/// than skipped.
fn check_readability(surface: &Surface, violations: &mut Vec<String>) {
    for member in surface.published() {
        for declaration in &member.declarations {
            if !declaration.name.starts_with('$') {
                continue;
            }
            let at = declaration.at(member);
            let name = &declaration.name;
            violations.push(format!(
                "{at}: an expansion declares `{name}`, which no invocation bound; this gate \
                 cannot read what that macro publishes and will not report success over it"
            ));
        }
    }
}

/// Every public type is open unless the frozen file records what closes it.
fn check_exhaustiveness(control: &Control, surface: &Surface, violations: &mut Vec<String>) {
    for member in surface.published() {
        for declaration in member.public_types() {
            let path = TypePath {
                crate_path: member.path.clone(),
                name: declaration.name.clone(),
            };
            let at = declaration.at(member);
            match (declaration.openness, control.exemption(&path)) {
                (Openness::Exhaustive, None) => violations.push(format!(
                    "{at}: `{path}` is public and exhaustive; add `#[non_exhaustive]`, or an \
                     `[[exempt]]` entry naming the sentence that closes it (ADR 0012)"
                )),
                (Openness::Open, Some(_)) => violations.push(format!(
                    "{at}: `{path}` is `#[non_exhaustive]`, so its `[[exempt]]` entry in \
                     docs/api-frozen.toml permits a decision the code has reversed; remove the \
                     entry or the attribute"
                )),
                _ => {},
            }
        }
    }
}

/// Every frozen projection still exists and still answers with a set that cannot grow.
fn check_projections(control: &Control, surface: &Surface, violations: &mut Vec<String>) {
    let closed = control.closed_type_names();
    for (path, projections) in &control.frozen {
        let Some(member) = surface.by_path(&path.crate_path) else {
            continue;
        };
        let Some(declaration) = member.declared_type(&path.name) else {
            continue;
        };
        let at = declaration.at(member);
        for projection in projections {
            let found = member
                .associated(&path.name)
                .find(|each| each.kind == Kind::Function && &each.name == projection);
            let Some(function) = found else {
                violations.push(format!(
                    "{at}: `{path}` has no public inherent `{projection}`; the frozen \
                     projection is what keeps a new variant detail rather than an outcome \
                     (ADR 0012)"
                ));
                continue;
            };
            if function.signature.receiver == Receiver::Absent {
                violations.push(format!(
                    "{at}: `{path}::{projection}` takes no receiver, so it projects nothing"
                ));
            }
            if !answers_a_closed_set(&function.signature.result, &closed) {
                let result = function.signature.result.concat();
                violations.push(format!(
                    "{at}: `{path}::{projection}` answers with `{result}`, whose set is not \
                     frozen; a projection answers `bool` or a type `[[exempt]]` closes"
                ));
            }
        }
    }
}

/// Whether a result's answer set is closed forever.
fn answers_a_closed_set(result: &[String], closed: &BTreeSet<&str>) -> bool {
    match result {
        [single] => single == "bool" || closed.contains(single.as_str()),
        _ => false,
    }
}

/// Every sealed type an adopter must pass in can still be obtained.
fn check_construction(surface: &Surface, violations: &mut Vec<String>) {
    let inputs = surface.input_positions();
    for member in surface.published() {
        let obtainable = obtainable_types(member);
        for declaration in member.public_types() {
            if declaration.construction != Construction::Sealed
                || !inputs.contains(declaration.name.as_str())
                || obtainable.contains(declaration.name.as_str())
            {
                continue;
            }
            let at = declaration.at(member);
            let path = &member.path;
            let name = &declaration.name;
            violations.push(format!(
                "{at}: `{path}::{name}` is sealed by `#[non_exhaustive]` and appears in an \
                 input position, but nothing a caller can reach hands one over; give it a named \
                 constructor (ADR 0012)"
            ));
        }
    }
}

/// The crate's own types a caller outside it can get hold of, starting from nothing.
///
/// A public item hands a type over when its result names it. The caller can reach that item
/// when it needs no receiver, or when the receiver's type is itself obtainable — so the set
/// is the fixed point of that step, which terminates because it only grows and the crate
/// declares finitely many types. Every named constructor is in it by construction; a
/// consuming builder alone is not, because reaching it already requires the value.
fn obtainable_types(member: &Member) -> BTreeSet<&str> {
    let declared: BTreeSet<&str> = member
        .declarations
        .iter()
        .filter(|each| each.kind.is_type())
        .map(|each| each.name.as_str())
        .collect();
    let producers: Vec<(Option<&str>, Vec<&str>)> = member
        .declarations
        .iter()
        .filter(|each| each.visibility == Visibility::Public)
        .filter(|each| matches!(each.kind, Kind::Function | Kind::Constant | Kind::Static))
        .map(|each| (already_held(each), handed_over(each, &declared)))
        .filter(|(_, produced)| !produced.is_empty())
        .collect();

    let mut obtained: BTreeSet<&str> = BTreeSet::new();
    loop {
        let mut grew = false;
        for (required, produced) in &producers {
            if required.is_some_and(|type_name| !obtained.contains(type_name)) {
                continue;
            }
            for name in produced {
                grew |= obtained.insert(name);
            }
        }
        if !grew {
            break;
        }
    }
    obtained
}

/// The type a caller must already hold to reach an item: its receiver's.
fn already_held(declaration: &Declaration) -> Option<&str> {
    if declaration.signature.receiver == Receiver::Absent {
        return None;
    }
    declaration
        .owner
        .as_ref()
        .map(|owner| owner.type_name.as_str())
}

/// The crate's own types an item's result hands the caller.
fn handed_over<'m>(declaration: &'m Declaration, declared: &BTreeSet<&'m str>) -> Vec<&'m str> {
    declaration
        .signature
        .result
        .iter()
        .map(String::as_str)
        .filter(|name| declared.contains(name))
        .collect()
}

/// Whether a result is `target`, `Option<target>`, or `Result<target, _>`.
///
/// The target is compared after `Self` has been resolved, so `-> Self` and `-> InlineExtent`
/// are the same answer written two ways.
fn results_in(result: &[String], target: &str) -> bool {
    let inner = match result {
        [single] => return single == target,
        [wrapper, open, rest @ ..] if open == "<" && wrapper == "Option" => {
            match rest.split_last() {
                Some((last, body)) if last == ">" => body.to_vec(),
                _ => return false,
            }
        },
        [wrapper, open, rest @ ..] if open == "<" && wrapper == "Result" => {
            split_first_argument(rest)
        },
        _ => return false,
    };
    matches!(inner.as_slice(), [single] if single == target)
}

/// The first type argument of a generic list, up to the first top-level comma.
fn split_first_argument(tokens: &[String]) -> Vec<String> {
    let mut argument = Vec::new();
    let mut depth = 0_usize;
    for token in tokens {
        match token.as_str() {
            "<" | "(" | "[" => depth = depth.saturating_add(1),
            ">" | ")" | "]" if depth > 0 => depth = depth.saturating_sub(1),
            ">" | ")" | "]" | "," if depth == 0 => break,
            _ => {},
        }
        argument.push(token.clone());
    }
    argument
}

/// No forbidden identifier is declared publicly in the crates the guard covers.
fn check_forbidden_names(control: &Control, surface: &Surface, violations: &mut Vec<String>) {
    for entry in &control.forbidden {
        if entry.item_names.is_empty() {
            continue;
        }
        for member in entry.crates.iter().filter_map(|name| surface.by_name(name)) {
            for declaration in &member.declarations {
                if declaration.visibility != Visibility::Public
                    || !entry.item_names.contains(&declaration.name)
                {
                    continue;
                }
                let at = declaration.at(member);
                let name = &declaration.name;
                violations.push(format!(
                    "{at}: `{name}` is a declared public item name the frozen file forbids in \
                     this crate; the library never asks how wide a character is (ADR 0002)"
                ));
            }
        }
    }
}

/// No forbidden signature is declared publicly in the crates the guard covers.
fn check_forbidden_signatures(control: &Control, surface: &Surface, violations: &mut Vec<String>) {
    for entry in &control.forbidden {
        let Some(text) = entry.signature.as_deref() else {
            continue;
        };
        let Some((parameters, result)) = parse_signature_spec(text) else {
            continue;
        };
        for member in entry.crates.iter().filter_map(|name| surface.by_name(name)) {
            for declaration in &member.declarations {
                if declaration.kind != Kind::Function
                    || declaration.visibility != Visibility::Public
                    || !matches_spec(&declaration.signature, &parameters, &result)
                {
                    continue;
                }
                let at = declaration.at(member);
                let name = &declaration.name;
                violations.push(format!(
                    "{at}: `{name}` has the shape `{text}`, which the frozen file forbids; a \
                     class is a function of an occurrence and not of a code point (ADR 0008)"
                ));
            }
        }
    }
}

/// Read a `(type, type) -> type` shape out of the control file.
fn parse_signature_spec(text: &str) -> Option<(Vec<String>, String)> {
    let (parameters, result) = text.split_once("->")?;
    let inside = parameters.trim().strip_prefix('(')?.strip_suffix(')')?;
    let parameters = inside
        .split(',')
        .map(|each| {
            tokenize(each)
                .iter()
                .map(|token| token.text.clone())
                .collect()
        })
        .filter(|each: &String| !each.is_empty())
        .collect();
    Some((parameters, result.split_whitespace().collect()))
}

/// Whether a signature is the forbidden shape.
///
/// The parameter list must match exactly, so a function taking more than the code point is
/// not caught — it is asking a different question. The result matches `Class` and the two
/// wrappers around one class, because an adopter reaching for the function ADR 0008
/// forbids reaches for all three; a function answering with a *set* of classes is the
/// honest shape and is deliberately not matched.
fn matches_spec(signature: &Signature, parameters: &[String], result: &str) -> bool {
    let declared: Vec<String> = signature
        .parameters
        .iter()
        .map(|parameter| parameter.concat())
        .collect();
    declared == parameters && results_in(&signature.result, result)
}

// -------------------------------------------------------------------------------------
// The report
// -------------------------------------------------------------------------------------

/// Print what was examined, and what the frozen file lists that does not exist yet.
///
/// The absent half is the point: it makes this gate a milestone view of the frozen API
/// while the milestones are still arriving, and it is stated as reporting rather than as
/// passing, because a listed type nobody has written is not a type that conforms.
fn report(control: &Control, surface: &Surface) {
    let published: Vec<&Member> = surface.published().collect();
    let files: usize = published.iter().map(|member| member.files).sum();
    let types: usize = published
        .iter()
        .map(|member| member.public_types().count())
        .sum();
    let functions: usize = published
        .iter()
        .map(|member| member.public_functions().count())
        .sum();
    let crates = published.len();
    let listed = control.frozen.len().saturating_add(control.exempt.len());
    println!(
        "api: read docs/api-frozen.toml: {frozen} frozen projections, {exempt} exempt types, \
         {forbidden} forbidden shapes.",
        frozen = control.frozen.len(),
        exempt = control.exempt.len(),
        forbidden = control.forbidden.len()
    );
    println!(
        "api: examined {crates} published crates, {files} source files, {types} public types, \
         {functions} public functions."
    );

    let absent = absent_types(control, surface);
    let present = listed.saturating_sub(absent.len());
    println!(
        "api: {present} of {listed} listed types exist and were checked; {count} arrive with a \
         later milestone and are reported, not failed on:",
        count = absent.len()
    );
    for line in &absent {
        println!("api:   {line}");
    }
    println!(
        "api: every sealed public input remains obtainable from an entry point a caller can reach."
    );
}

// -------------------------------------------------------------------------------------
// The policy space
// -------------------------------------------------------------------------------------

/// The derived policy space those constants are generated from.
const POLICY_SPACE: &str = "spec/derived/questions.tsv";

/// Hold the derived policy space equal to the dedicated typed enums 1.0 publishes.
///
/// `docs/api-1.0.toml` maps every derived question path to one public enum and its closed
/// choice count. The subtraction runs in both directions, so an unmapped specification row,
/// an invented public setting, or a changed answer count is a failure.
///
/// The `[[closed_choices]]` table is checked here too, because it is a claim about the same
/// data: a question whose answer set the specification closes may not be derived with more
/// answers than the specification closes it at. `docs/api-frozen.toml` says a count that may
/// never grow is only a control if it is written down before the data that could grow it,
/// which is what makes reading it now — rather than when the emitter lands — the point.
///
/// Neither file being absent is treated as a pass: the census says which side was missing and
/// what could therefore not be subtracted.
fn check_policy_space(root: &Path, violations: &mut Vec<String>) -> io::Result<()> {
    let mappings = style_choice_mappings(root)?;
    let modules = allowed_modules(root)?;
    let style_items = modules
        .iter()
        .find(|module| module.path == "jlreq::style")
        .map(|module| &module.items)
        .ok_or_else(|| malformed("docs/api-1.0.toml has no `jlreq::style` module".to_owned()))?;
    let derived = derived_questions(root)?.ok_or_else(|| {
        malformed(format!(
            "{POLICY_SPACE} does not exist, so the typed Style mapping cannot be checked"
        ))
    })?;
    violations.extend(check_style_choice_mappings(
        &mappings,
        &derived,
        style_items,
    ));
    println!(
        "api: docs/api-1.0.toml maps {mappings} typed Style choice(s) onto {rows} generated JLReq question(s).",
        mappings = mappings.len(),
        rows = derived.len()
    );
    Ok(())
}

/// One row of the derived policy space, in the two columns this gate reads.
#[derive(Debug)]
struct DerivedQuestion {
    /// The stable dotted path a conformance case file names it by.
    path: String,
    /// The `Question` constant the spine publishes for it.
    constant: String,
    /// How many answers it permits.
    answers: usize,
}

/// Every `[[closed_choices]]` claim, held against the answer set that was derived.
fn check_closed_choices(control: &Control, derived: &[DerivedQuestion]) -> Vec<String> {
    let mut found = Vec::new();
    for closed in &control.closed_choices {
        let Some(question) = derived
            .iter()
            .find(|question| question.path == closed.question)
        else {
            found.push(format!(
                "docs/api-frozen.toml closes the answer set of `{question}`, which \
                 {POLICY_SPACE} does not record; a control over a question that does not \
                 exist guards nothing",
                question = closed.question
            ));
            continue;
        };
        if question.answers != closed.count {
            found.push(format!(
                "docs/api-frozen.toml closes `{path}` at {count} answer(s) and {POLICY_SPACE} \
                 permits {answers}; the count was written down before the data so that this \
                 could not happen quietly",
                path = closed.question,
                count = closed.count,
                answers = question.answers
            ));
        }
    }
    found
}

/// The policy space `xtask derive` wrote, in the three columns this gate reads.
///
/// Read as a table rather than through `jlreq-spec`, because this program declares no
/// dependencies; `generate --check` is what holds the file and the emitted Rust in step.
fn derived_questions(root: &Path) -> io::Result<Option<Vec<DerivedQuestion>>> {
    let path = root.join(POLICY_SPACE);
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path)?;
    let mut rows = text
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'));
    let Some(header) = rows.next() else {
        return Ok(Some(Vec::new()));
    };
    let column = |name: &str| header.split('\t').position(|each| each.trim() == name);
    let (Some(path_at), Some(constant_at), Some(permits_at)) =
        (column("question"), column("constant"), column("permits"))
    else {
        return Err(malformed(format!(
            "{POLICY_SPACE} has no `question`, `constant` or `permits` column; \
             docs/design/generation.md names all three"
        )));
    };
    let mut found = Vec::new();
    for row in rows {
        let fields: Vec<&str> = row.split('\t').collect();
        let (Some(path), Some(constant), Some(permits)) = (
            fields.get(path_at),
            fields.get(constant_at),
            fields.get(permits_at),
        ) else {
            return Err(malformed(format!(
                "{POLICY_SPACE} states a row short of a column"
            )));
        };
        found.push(DerivedQuestion {
            path: (*path).to_owned(),
            constant: (*constant).to_owned(),
            answers: permits.split_whitespace().count(),
        });
    }
    Ok(Some(found))
}

/// The listed types that do not exist yet, with what waits on each.
fn absent_types(control: &Control, surface: &Surface) -> Vec<String> {
    let mut absent = Vec::new();
    for (path, projections) in &control.frozen {
        if declared(surface, path) {
            continue;
        }
        let wanted = projections.join(", ");
        absent.push(format!("{path} — frozen projections {wanted}"));
    }
    for (path, _) in &control.exempt {
        if declared(surface, path) {
            continue;
        }
        absent.push(format!("{path} — exempt from being open"));
    }
    absent
}

/// Whether a listed type is declared in the crate that is supposed to declare it.
fn declared(surface: &Surface, path: &TypePath) -> bool {
    surface
        .by_path(&path.crate_path)
        .and_then(|member| member.declared_type(&path.name))
        .is_some()
}

#[cfg(test)]
mod tests {
    use super::{
        AllowedModule, ClosedChoices, Construction, Control, DerivedQuestion, Forbidden, Kind,
        Openness, Receiver, StyleChoiceMapping, Surface, TypePath, Visibility,
        answers_a_closed_set, check_allowed_items, check_closed_choices, check_construction,
        check_exhaustiveness, check_forbidden_names, check_forbidden_signatures, check_projections,
        check_readability, check_style_choice_mappings, code_only, declarations_of, entries_of,
        obtainable_types, parse_signature_spec, reexported_names, results_in, tokenize,
    };
    use std::collections::BTreeSet;

    /// Read one fixture source as if it were a crate's only file.
    fn read(source: &str) -> Vec<super::Declaration> {
        declarations_of(source, "src/lib.rs")
    }

    #[test]
    fn one_point_zero_allowlist_is_exact_in_both_directions() {
        let allowed = AllowedModule {
            path: "jlreq".to_owned(),
            items: ["Style".to_owned(), "compose".to_owned()]
                .into_iter()
                .collect(),
        };
        assert!(check_allowed_items(&allowed, "pub struct Style; pub fn compose() {}").is_empty());

        let violations = check_allowed_items(
            &allowed,
            "pub struct Style; pub struct RuleId; pub fn hidden_detail() {}",
        );
        assert_eq!(violations.len(), 3, "found {violations:?}");
        assert!(violations.iter().any(|line| line.contains("compose")));
        assert!(violations.iter().any(|line| line.contains("RuleId")));
        assert!(violations.iter().any(|line| line.contains("hidden_detail")));
    }

    #[test]
    fn typed_style_mapping_replaces_the_public_question_vocabulary() {
        let mappings = vec![StyleChoiceMapping {
            question: "kinsoku.level".to_owned(),
            rust_type: "KinsokuLevel".to_owned(),
            count: 4,
        }];
        let recorded = vec![DerivedQuestion {
            path: "kinsoku.level".to_owned(),
            constant: "KINSOKU_LEVEL".to_owned(),
            answers: 4,
        }];
        let style_items = ["KinsokuLevel".to_owned()].into_iter().collect();
        assert!(check_style_choice_mappings(&mappings, &recorded, &style_items).is_empty());

        let grown = vec![DerivedQuestion {
            path: "kinsoku.level".to_owned(),
            constant: "KINSOKU_LEVEL".to_owned(),
            answers: 5,
        }];
        let violations = check_style_choice_mappings(&mappings, &grown, &style_items);
        assert_eq!(violations.len(), 1, "found {violations:?}");
        assert!(violations[0].contains("KinsokuLevel"));
    }

    /// One member holding exactly this fixture, published under this crate name.
    fn member(name: &str, source: &str) -> super::Member {
        super::Member {
            path: name.replace('-', "_"),
            name: name.to_owned(),
            publication: super::Publication::Published,
            files: 1,
            declarations: read(source),
        }
    }

    /// One explicitly non-publishable fixture member.
    fn internal_member(name: &str, source: &str) -> super::Member {
        let mut member = member(name, source);
        member.publication = super::Publication::Internal;
        member
    }

    /// A surface of one fixture crate.
    fn surface(name: &str, source: &str) -> Surface {
        Surface {
            members: vec![member(name, source)],
        }
    }

    /// A compact historical fixture for the parser checks below. The retired control file
    /// is intentionally not part of the repository or the production gate.
    fn control() -> Control {
        Control {
            frozen: vec![(
                TypePath::parse("jlreq_spec::Provenance").expect("fixture type path"),
                vec!["is_specified".to_owned()],
            )],
            exempt: vec![(
                TypePath::parse("jlreq_unit::Direction").expect("fixture type path"),
                "the fixture closes the two directions".to_owned(),
            )],
            forbidden: vec![
                Forbidden {
                    crates: vec!["jlreq-line".to_owned()],
                    item_names: vec!["measure".to_owned()],
                    signature: None,
                },
                Forbidden {
                    crates: vec!["jlreq-class".to_owned()],
                    item_names: Vec::new(),
                    signature: Some("(char) -> Class".to_owned()),
                },
            ],
            closed_choices: vec![ClosedChoices {
                question: "kinsoku.level".to_owned(),
                count: 4,
            }],
        }
    }

    #[test]
    fn the_gate_takes_no_arguments() {
        assert!(super::run(&["--check".to_owned()]).is_err());
    }

    #[test]
    fn the_jlreq_release_candidate_is_checked_before_publication() {
        let surface = Surface {
            members: vec![
                internal_member("jlreq", "pub struct PublicApi;\n"),
                internal_member("jlreq-unit", "pub struct LegacyApi;\n"),
            ],
        };
        let checked: Vec<_> = surface
            .published()
            .map(|member| member.name.as_str())
            .collect();
        assert_eq!(
            checked,
            ["jlreq"],
            "publish=false keeps the release blocked, not the API gate blind"
        );
    }

    #[test]
    fn prose_and_strings_are_not_code() {
        let source = "//! A doc comment naming pub struct Sneaky and a font.\n\
                      /* nested /* block */ comment with pub enum Hidden */\n\
                      const NAME: &str = \"pub struct InString\";\n";
        let cleaned = code_only(source);
        assert!(!cleaned.contains("Sneaky"), "{cleaned}");
        assert!(!cleaned.contains("Hidden"), "{cleaned}");
        assert!(!cleaned.contains("InString"), "{cleaned}");
        assert_eq!(
            cleaned.lines().count(),
            source.lines().count(),
            "a finding must still name its own line"
        );
    }

    #[test]
    fn a_lifetime_is_not_a_character_literal() {
        let cleaned = code_only("pub struct Runs<'a> { slots: &'a [char] }\nlet quote = '\"';\n");
        assert!(cleaned.contains("Runs<'a>"), "{cleaned}");
        assert!(cleaned.contains("&'a [char]"), "{cleaned}");
        let declarations = read("pub struct Runs<'a> { c: char }\npub struct After;\n");
        assert_eq!(declarations.len(), 2, "the quote did not swallow the file");
    }

    #[test]
    fn a_raw_string_ends_at_its_own_hashes() {
        let cleaned = code_only("let text = r#\"a \" quote\"#;\npub enum Seen { One }\n");
        assert!(cleaned.contains("pub enum Seen"), "{cleaned}");
    }

    #[test]
    fn a_public_type_is_read_with_its_attribute() {
        let declarations = read(
            "#[derive(Debug)]\n#[non_exhaustive]\npub struct Item { start: u32 }\n\
             pub enum Side { Start, End }\npub(crate) struct Private;\n",
        );
        let item = declarations
            .iter()
            .find(|each| each.name == "Item")
            .expect("Item is declared");
        assert_eq!(item.kind, Kind::Struct);
        assert_eq!(item.openness, Openness::Open);
        assert_eq!(item.construction, Construction::Sealed);
        let side = declarations
            .iter()
            .find(|each| each.name == "Side")
            .expect("Side is declared");
        assert_eq!(side.openness, Openness::Exhaustive);
        let private = declarations
            .iter()
            .find(|each| each.name == "Private")
            .expect("Private is declared");
        assert_eq!(private.visibility, Visibility::Restricted);
    }

    #[test]
    fn an_open_enum_still_lets_a_caller_name_a_variant() {
        let declarations = read(
            "#[non_exhaustive]\npub enum Frame { Solid, FullEm }\n\
             #[non_exhaustive]\npub enum Sealed { #[non_exhaustive] One, #[non_exhaustive] Two }\n",
        );
        let frame = declarations
            .iter()
            .find(|each| each.name == "Frame")
            .expect("Frame is declared");
        assert_eq!(
            frame.construction,
            Construction::Nameable,
            "`#[non_exhaustive]` on an enum does not hide its variants"
        );
        let sealed = declarations
            .iter()
            .find(|each| each.name == "Sealed")
            .expect("Sealed is declared");
        assert_eq!(sealed.construction, Construction::Sealed);
    }

    #[test]
    fn a_signature_keeps_its_parameters_and_resolves_self() {
        let declarations = read(
            "pub struct Cursor;\nimpl Cursor {\n  pub const fn new() -> Self { Self }\n\
             pub fn advance(self, by: InlineExtent, and: &mut Carry) -> Self { self }\n}\n",
        );
        let new = declarations
            .iter()
            .find(|each| each.name == "new")
            .expect("new is declared");
        assert_eq!(new.signature.receiver, Receiver::Absent);
        assert_eq!(new.signature.result, ["Cursor"], "Self is resolved");
        let advance = declarations
            .iter()
            .find(|each| each.name == "advance")
            .expect("advance is declared");
        assert_eq!(advance.signature.receiver, Receiver::Present);
        assert_eq!(
            advance.signature.parameters.len(),
            2,
            "the receiver is not a parameter"
        );
    }

    #[test]
    fn a_macro_generated_constructor_is_seen_where_it_is_written() {
        let source = "macro_rules! axis_scalar {\n  ($type:ident, $what:literal) => {\n\
                      impl $type {\n  pub const ZERO: Self = Self(0);\n\
                      pub const fn new(units: i32) -> Option<Self> { None }\n  }\n  };\n}\n\
                      #[non_exhaustive]\npub struct InlineExtent(i32);\n\
                      axis_scalar!(InlineExtent, \"extent along the inline axis\");\n";
        let holder = member("jlreq-unit", source);
        assert!(
            obtainable_types(&holder).contains("InlineExtent"),
            "the expansion declares `InlineExtent::new`"
        );
    }

    #[test]
    fn an_expansion_no_invocation_bound_is_reported_rather_than_skipped() {
        let source = "macro_rules! declare {\n  ($name:ident) => {\n\
                      #[non_exhaustive]\n  pub struct $name(i32);\n  };\n}\n\
                      declare!();\n";
        let mut violations = Vec::new();
        check_readability(&surface("jlreq-unit", source), &mut violations);
        assert_eq!(
            violations.len(),
            1,
            "a surface the gate cannot read is not a surface it passes: {violations:?}"
        );

        let mut bound = Vec::new();
        check_readability(
            &surface(
                "jlreq-unit",
                "macro_rules! declare {\n  ($name:ident) => {\n\
                 #[non_exhaustive]\n  pub struct $name(i32);\n  };\n}\ndeclare!(Advance);\n",
            ),
            &mut bound,
        );
        assert!(bound.is_empty(), "{bound:?}");
    }

    #[test]
    fn a_reexport_publishes_the_names_it_names() {
        let declarations = read(
            "pub use crate::axis::{Direction, Side};\npub use crate::length::Em as Fraction;\n\
             use crate::hidden::NotPublished;\n",
        );
        let names: Vec<&str> = declarations
            .iter()
            .filter(|each| each.kind == Kind::Reexport)
            .map(|each| each.name.as_str())
            .collect();
        assert_eq!(names, ["Direction", "Side", "Fraction"]);
        assert_eq!(
            reexported_names(&tokenize("crate :: a :: b")),
            ["b"],
            "only the leaf is published"
        );
    }

    #[test]
    fn a_test_module_is_not_a_surface() {
        let declarations = read("#[cfg(test)]\nmod tests {\n  pub struct Fixture;\n}\n");
        assert!(
            !declarations.iter().any(|each| each.name == "Fixture"),
            "a test fixture is not published"
        );
    }

    #[test]
    fn an_exhaustive_public_type_is_a_violation_unless_it_is_exempt() {
        let mut violations = Vec::new();
        check_exhaustiveness(
            &control(),
            &surface("jlreq-unit", "pub struct Leaked { pub bytes: u32 }\n"),
            &mut violations,
        );
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert!(
            violations[0].contains("jlreq_unit::Leaked"),
            "{violations:?}"
        );

        let mut permitted = Vec::new();
        check_exhaustiveness(
            &control(),
            &surface(
                "jlreq-unit",
                "pub enum Direction { Horizontal, Vertical }\n",
            ),
            &mut permitted,
        );
        assert!(permitted.is_empty(), "{permitted:?}");
    }

    #[test]
    fn an_exemption_the_code_has_reversed_is_a_violation() {
        let mut violations = Vec::new();
        check_exhaustiveness(
            &control(),
            &surface(
                "jlreq-unit",
                "#[non_exhaustive]\npub enum Direction { Horizontal, Vertical }\n",
            ),
            &mut violations,
        );
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert!(violations[0].contains("remove the"), "{violations:?}");
    }

    #[test]
    fn a_missing_or_growing_projection_is_a_violation() {
        let mut missing = Vec::new();
        check_projections(
            &control(),
            &surface("jlreq-spec", "#[non_exhaustive]\npub struct Provenance;\n"),
            &mut missing,
        );
        assert_eq!(missing.len(), 1, "{missing:?}");
        assert!(missing[0].contains("is_specified"), "{missing:?}");

        let mut growing = Vec::new();
        check_projections(
            &control(),
            &surface(
                "jlreq-spec",
                "#[non_exhaustive]\npub struct Provenance;\nimpl Provenance {\n\
                 pub const fn is_specified(self) -> Reason { Reason::Stated }\n}\n",
            ),
            &mut growing,
        );
        assert_eq!(growing.len(), 1, "{growing:?}");
        assert!(growing[0].contains("not frozen"), "{growing:?}");

        let mut held = Vec::new();
        check_projections(
            &control(),
            &surface(
                "jlreq-spec",
                "#[non_exhaustive]\npub struct Provenance;\nimpl Provenance {\n\
                 pub const fn is_specified(self) -> bool { true }\n}\n",
            ),
            &mut held,
        );
        assert!(held.is_empty(), "{held:?}");
    }

    #[test]
    fn a_projection_on_a_type_that_does_not_exist_yet_is_reported_not_failed_on() {
        let mut violations = Vec::new();
        check_projections(&control(), &surface("jlreq-line", ""), &mut violations);
        assert!(
            violations.is_empty(),
            "`jlreq_line::Fit` arrives at M1; its absence is a milestone, not a breach"
        );
    }

    #[test]
    fn a_sealed_input_nobody_can_build_is_a_violation() {
        let mut violations = Vec::new();
        check_construction(
            &surface(
                "jlreq-unit",
                "#[non_exhaustive]\npub struct Demand { start: u32 }\n\
                 pub fn compose(demand: Demand) -> u32 { 0 }\n",
            ),
            &mut violations,
        );
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert!(violations[0].contains("hands one over"), "{violations:?}");
    }

    #[test]
    fn every_shape_the_design_obtains_a_value_by_satisfies_the_check() {
        for entry in [
            "impl Demand { pub const fn new(start: u32) -> Self { Self { start } } }",
            "impl Demand { pub fn parse(text: &str) -> Option<Self> { None } }",
            "impl Demand { pub fn build(text: &str) -> Result<Self, Error> { Err(Error) } }",
            "impl Demand { pub const ALL: &'static [Self] = &[]; }",
            "pub struct Book;\nimpl Book { pub const ONE: Self = Book; \
             pub fn demands(self) -> &'static [Demand] { &[] } }",
        ] {
            let source = format!(
                "#[non_exhaustive]\npub struct Demand {{ start: u32 }}\n\
                 pub fn compose(demand: Demand) -> u32 {{ 0 }}\n{entry}\n"
            );
            let mut violations = Vec::new();
            check_construction(&surface("jlreq-unit", &source), &mut violations);
            assert!(violations.is_empty(), "{entry}: {violations:?}");
        }
    }

    #[test]
    fn a_builder_method_alone_is_not_a_way_in() {
        let source = "#[non_exhaustive]\npub struct Demand { start: u32 }\n\
                      pub fn compose(demand: Demand) -> u32 { 0 }\n\
                      impl Demand { pub const fn with_start(mut self, start: u32) -> Self { self } }\n";
        let mut violations = Vec::new();
        check_construction(&surface("jlreq-unit", source), &mut violations);
        assert_eq!(
            violations.len(),
            1,
            "reaching a builder already requires the value: {violations:?}"
        );
    }

    #[test]
    fn a_producer_a_caller_cannot_reach_is_not_a_way_in() {
        let source = "#[non_exhaustive]\npub struct Demand { start: u32 }\n\
                      pub fn compose(demand: Demand) -> u32 { 0 }\n\
                      #[non_exhaustive]\npub struct Sealed { held: u32 }\n\
                      impl Sealed { pub fn demand(self) -> Demand { todo } }\n";
        let mut violations = Vec::new();
        check_construction(&surface("jlreq-unit", source), &mut violations);
        assert_eq!(
            violations.len(),
            1,
            "nothing hands over the `Sealed` the producer needs: {violations:?}"
        );
    }

    #[test]
    fn a_sealed_type_no_caller_passes_in_needs_no_way_in() {
        let mut violations = Vec::new();
        check_construction(
            &surface(
                "jlreq-unit",
                "#[non_exhaustive]\npub struct Distribution { next: usize }\n\
                 pub fn distribute() -> Distribution { Distribution { next: 0 } }\n",
            ),
            &mut violations,
        );
        assert!(
            violations.is_empty(),
            "an output nobody hands back in is not an input: {violations:?}"
        );
    }

    #[test]
    fn a_forbidden_name_is_caught_only_where_it_is_declared() {
        let mut declared = Vec::new();
        check_forbidden_names(
            &control(),
            &surface("jlreq-line", "pub fn measure(text: &str) -> u32 { 0 }\n"),
            &mut declared,
        );
        assert_eq!(declared.len(), 1, "{declared:?}");

        let mut parameter = Vec::new();
        check_forbidden_names(
            &control(),
            &surface(
                "jlreq-line",
                "//! Composing to a measure. See the font note.\n\
                 pub fn compose(text: &str, measure: u32, width: u32) -> u32 { 0 }\n",
            ),
            &mut parameter,
        );
        assert!(
            parameter.is_empty(),
            "a parameter name and prose are not declarations: {parameter:?}"
        );

        let mut elsewhere = Vec::new();
        check_forbidden_names(
            &control(),
            &surface("jlreq-unit", "pub fn measure() -> u32 { 0 }\n"),
            &mut elsewhere,
        );
        assert!(
            elsewhere.is_empty(),
            "the guard covers the crates the file names: {elsewhere:?}"
        );
    }

    #[test]
    fn a_reexported_forbidden_name_is_still_declared_publicly() {
        let mut violations = Vec::new();
        check_forbidden_names(
            &control(),
            &surface("jlreq-line", "pub use crate::inner::measure;\n"),
            &mut violations,
        );
        assert_eq!(
            violations.len(),
            1,
            "a re-export is how this workspace publishes an item: {violations:?}"
        );
    }

    #[test]
    fn the_total_classification_function_is_caught_in_three_shapes() {
        for result in ["Class", "Option<Class>", "Result<Class, Error>"] {
            let source = format!("pub fn classify(point: char) -> {result} {{ todo }}\n");
            let mut violations = Vec::new();
            check_forbidden_signatures(
                &control(),
                &surface("jlreq-class", &source),
                &mut violations,
            );
            assert_eq!(violations.len(), 1, "{result}: {violations:?}");
        }
    }

    #[test]
    fn an_honest_classification_signature_is_not_the_forbidden_one() {
        for source in [
            "pub fn classify(text: Text<'_>, at: ItemIndex) -> Classified { todo }\n",
            "pub fn candidates(point: char) -> &'static [Class] { &[] }\n",
            "pub fn classify(point: char, frame: Frame) -> Class { todo }\n",
        ] {
            let mut violations = Vec::new();
            check_forbidden_signatures(
                &control(),
                &surface("jlreq-class", source),
                &mut violations,
            );
            assert!(violations.is_empty(), "{source}: {violations:?}");
        }
    }

    #[test]
    fn the_control_file_is_read_as_entries() {
        let entries = entries_of(
            "[[frozen]]\ntype = \"a::B\"\nprojections = [\n  \"is_one\",\n  \"is_two\",\n]\n\
             [[exempt]]\ntype = \"a::C\"\nwhy = \"the specification closes it\"\n",
        );
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].list("projections"), ["is_one", "is_two"]);
        assert_eq!(entries[1].single("type"), Some("a::C"));
    }

    #[test]
    fn a_bracket_inside_a_sentence_does_not_open_an_array() {
        let entries =
            entries_of("[[forbidden]]\nwhy = \"a [bracketed] aside\"\ncrates = [\"jlreq\"]\n");
        assert_eq!(entries[0].list("crates"), ["jlreq"]);
        assert_eq!(entries[0].list("why"), ["a [bracketed] aside"]);
    }

    #[test]
    fn a_control_file_missing_a_table_cannot_be_read() {
        assert!(TypePath::parse("jlreq_spec::Provenance").is_some());
        assert!(TypePath::parse("Provenance").is_none());
        assert!(super::frozen_entries(&entries_of("[[frozen]]\ntype = \"a::B\"\n")).is_err());
        assert!(super::exempt_entries(&entries_of("[[exempt]]\ntype = \"a::B\"\n")).is_err());
        assert!(
            super::forbidden_entries(&entries_of("[[forbidden]]\nitem_names = [\"font\"]\n"))
                .is_err()
        );
    }

    #[test]
    fn a_result_is_recognized_through_one_wrapper_only() {
        let owned = |texts: &[&str]| -> Vec<String> {
            texts.iter().map(|each| (*each).to_owned()).collect()
        };
        assert!(results_in(&owned(&["Class"]), "Class"));
        assert!(results_in(&owned(&["Option", "<", "Class", ">"]), "Class"));
        assert!(results_in(
            &owned(&["Result", "<", "Class", ",", "E", ">"]),
            "Class"
        ));
        assert!(!results_in(&owned(&["&", "[", "Class", "]"]), "Class"));
        assert!(!results_in(&owned(&["Option", "<", "Other", ">"]), "Class"));
    }

    #[test]
    fn a_closed_answer_is_a_boolean_or_a_type_the_specification_closes() {
        let closed: BTreeSet<&str> = ["Class"].into_iter().collect();
        assert!(answers_a_closed_set(&["bool".to_owned()], &closed));
        assert!(answers_a_closed_set(&["Class".to_owned()], &closed));
        assert!(!answers_a_closed_set(&["Reason".to_owned()], &closed));
        assert!(!answers_a_closed_set(
            &[
                "Option".to_owned(),
                "<".to_owned(),
                "bool".to_owned(),
                ">".to_owned()
            ],
            &closed
        ));
    }

    #[test]
    fn a_signature_shape_is_read_from_the_control_file() {
        let (parameters, result) =
            parse_signature_spec("(char) -> Class").expect("the shipped shape parses");
        assert_eq!(parameters, ["char"]);
        assert_eq!(result, "Class");
        assert!(parse_signature_spec("nonsense").is_none());
    }

    #[test]
    fn a_closed_answer_set_that_grew_is_refused() {
        let derived = |answers: usize| {
            vec![DerivedQuestion {
                path: "kinsoku.level".to_owned(),
                constant: "KINSOKU_LEVEL".to_owned(),
                answers,
            }]
        };
        let control = Control {
            frozen: Vec::new(),
            exempt: Vec::new(),
            forbidden: Vec::new(),
            closed_choices: vec![ClosedChoices {
                question: "kinsoku.level".to_owned(),
                count: 4,
            }],
        };
        assert!(check_closed_choices(&control, &derived(4)).is_empty());
        let grown = check_closed_choices(&control, &derived(5));
        assert_eq!(grown.len(), 1, "{grown:?}");
        assert!(grown[0].contains("closes `kinsoku.level` at 4 answer(s)"));
        let absent = check_closed_choices(&control, &[]);
        assert_eq!(absent.len(), 1, "{absent:?}");
        assert!(absent[0].contains("a control over a question that does not exist"));
    }

    #[test]
    fn the_repository_itself_holds_every_check() {
        let violations = super::run(&[]).expect("the 1.0 API gate runs");
        assert!(violations.is_empty(), "{violations:#?}");
    }
}
