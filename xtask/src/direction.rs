// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `direction` gate: what keeps vertical writing a direction rather than a second
//! implementation.
//!
//! `docs/adr/0004` decided that horizontal and vertical composition run one code path, and
//! `docs/adr/0011` decided how that is held. JLReq conditions exactly three rules on the
//! writing direction — §3.1.3, §3.2.5 and §3.3.5 — and everything else it states twice is
//! exact axis mapping, expressed with `Side` and `InlineEdge`. This module is the
//! mechanical form of that sentence, so "there is no second code path" is falsifiable
//! rather than aspirational.
//!
//! # The mechanism
//!
//! The subject is a *named variant* of `Direction`, and nothing else. Naming the type is
//! unrestricted, because the direction is threaded by signature through most of the
//! workspace and passing a value through a signature is not a branch. A lexical ban on the
//! word would be worse than useless: the generated rule inventory has to carry JLReq's own
//! heading "Major Differences between Vertical Writing Mode and Horizontal Writing Mode".
//! Only a named variant can branch, so only a named variant is checked.
//!
//! Three checks, over comments-and-string-literals-stripped sources:
//!
//! 1. In a hand-written core source a variant may be named only inside an item
//!    `docs/direction-sites.toml` lists.
//! 2. In a generated core source a variant may be named only as the argument of a
//!    `Predicate::InDirection` row.
//! 3. The union of the rules those two name equals the set the rule inventory marks
//!    direction-conditional.
//!
//! Check 3 is an equality between what this workspace reads and what the specification
//! conditions, and one side of it is written milestone by milestone. A marked rule whose
//! reader has not been written yet is therefore neither passed over nor reported as broken:
//! `docs/direction-sites.toml` carries a `[[pending]]` table naming the rule, the crate
//! whose first item closes it, and why that is where it will be read, and the census names
//! every rule so deferred. The entry expires by itself — it is a violation once anything
//! reads the rule, and a violation once the crate it waits on declares an item — so it can
//! neither rot into a permanent exemption nor be written for a rule that has a reader today.
//! Without it this gate could only choose between two false sentences: that ADR 0011's
//! equality is broken, when what is true is that half of it has no subject yet, or that the
//! equality holds, by comparing an empty set with an empty set.
//!
//! Four spellings would let a variant be named where this gate could not attribute it —
//! a glob import of the variants, a brace import of one, renaming the type in a `use`, and
//! aliasing it with `type` — so each is rejected wherever it appears rather than being left
//! as a hole the mechanism does not cover.
//!
//! # Where each fact is read from
//!
//! The variants are read from the `enum Direction` declaration itself rather than written
//! down here, so a variant added to the type is covered the moment it is added, and a
//! second crate declaring the same type is a finding rather than a silent second subject
//! (`docs/adr/0019`).
//!
//! The set of direction-conditional rules is read from `spec/derived/rules.tsv`, the rule
//! inventory in the form `docs/design/generation.md` publishes it: the addresses are
//! canonical strings there, which is the same key space `docs/direction-sites.toml` uses,
//! and reading them there rather than out of the emitted Rust keeps this gate off
//! `jlreq-spec`'s private representation of an address. The emitted table is that same
//! inventory, and `generate --check` holds the two byte-identical.
//!
//! Because that is an argument and not a proof, the gate carries a tripwire against its own
//! blind spot: the number of rows the *emitted* inventory marks `direction_conditional:
//! true` must equal the number the TSV marks, so a reader of this gate that has gone stale
//! fails loudly instead of comparing against a set it silently read as empty.
//!
//! # What it examines today
//!
//! Every check above runs now. Check 1 constrains every core source in the workspace
//! today: no item is allowlisted, so naming a variant anywhere in hand-written core code is
//! a failure until an entry is written and reviewed. Check 2 has an empty subject — no
//! generated override table exists — and it constrains that subject from the commit that
//! creates it: a `Predicate::InDirection` row whose rule the gate cannot resolve fails.
//!
//! Check 3 now has one populated side. `spec/derived/rules.tsv` marks §3.1.3, §3.2.5 and
//! §3.3.5, and each of the three is read in a crate that is still a module comment and
//! `#![no_std]`: §3.1.3 reaches the boundary evaluator as data in `jlreq-spacing`, and
//! §3.2.5 and §3.3.5 are read where the tate-chu-yoko (縦中横) segment is built and where
//! ruby is lowered, both in `jlreq-inline`. All three are deferred by `[[pending]]` entries
//! naming those crates, so the first item either of them declares is what turns the last
//! half of check 3 back on, one crate at a time rather than all at once.
//!
//! # What it cannot see, named rather than glossed
//!
//! A branch that reads the direction indirectly, through a boolean some other function
//! derived from it, is invisible here. That residue is the parity gate's: every conformance
//! case not marked direction-specific is composed both ways and the inline results must be
//! bit-identical, so an indirect branch fails a case. Neither gate is claimed to do the
//! other's work (`docs/adr/0011`).
//!
//! A doc example is a comment, and comments are stripped before the scan, so a variant
//! named in a doc test is not a site. That follows from the ADR's own wording, and it is
//! stated here rather than discovered.
//!
//! Non-core crates are outside the scan. `jlreq-conform` composes every case both ways by
//! construction, which is the parity gate rather than a violation of this one.
//!
//! A `#[cfg(test)]` module inside a core crate is *not* outside it. The core crates have no
//! dev-dependencies, so their unit tests live in the sources this gate reads, and ADR 0011
//! draws the line at hand-written core sources rather than at library code. A core test that
//! needs to name a variant is therefore an allowlist entry like any other site — and until
//! the inventory marks a rule, check 3 refuses that entry, so a core test cannot name one
//! either. That is the intended reading and not an oversight: the place a direction is
//! exercised without naming a variant is the conformance suite, which composes each case
//! both ways from data.
//!
//! See `docs/design/api-spine.md`, `docs/adr/0004` and `docs/adr/0011`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::Path;

use crate::shared::{self, CoreCrate, Gate};

/// The `direction` gate, as the dispatcher sees it.
pub(crate) const GATE: Gate = Gate {
    name: "direction",
    purpose: concat!(
        "the rules that read the writing direction are exactly the rules the inventory ",
        "marks direction-conditional, less the ones the allowlist defers to a crate that ",
        "has not started — each named above"
    ),
    reference: concat!(
        "docs/adr/0011-typed-axes-and-direction-as-a-datum.md ",
        "and docs/direction-sites.toml"
    ),
    run,
};

/// The allowlist of hand-written sites, relative to the workspace root.
const ALLOWLIST: &str = "docs/direction-sites.toml";

/// The rule inventory, relative to the workspace root, in the form stage 1 emits it.
const INVENTORY: &str = "spec/derived/rules.tsv";

/// The type whose variants are the subject of this gate.
const TYPE: &str = "Direction";

/// The predicate a generated source may name a direction inside.
const PREDICATE: &str = "InDirection";

/// The enum that predicate belongs to, when the row spells the qualifier.
const PREDICATE_ENUM: &str = "Predicate";

/// The type a generated row addresses its rule with.
const RULE_TYPE: &str = "RuleId";

/// The inventory column, and the generated field, carrying the direction mark.
const FLAG_COLUMN: &str = "direction_conditional";

/// The inventory column carrying the specification address.
const ADDRESS_COLUMN: &str = "address";

/// The inventory column carrying the name of the generated constant for a rule.
const NAME_COLUMN: &str = "name";

/// The directory name that makes a core source generated rather than hand-written.
const GENERATED: &str = "generated";

/// The four keys a `[[site]]` table carries, and no others.
const SITE_KEYS: [&str; 4] = ["crate", "item", "rule", "why"];

/// The three keys a `[[pending]]` table carries, and no others.
const PENDING_KEYS: [&str; 3] = ["rule", "crate", "why"];

/// The keywords whose presence makes a crate one that declares something.
///
/// A `[[pending]]` entry expires when the crate it names starts, so "started" needs a
/// definition a scan can apply. A crate whose sources hold none of these declares nothing
/// that could contain a branch: `jlreq-spacing` today is a module comment and `#![no_std]`.
const ITEM_KEYWORDS: [&str; 10] = [
    "fn",
    "struct",
    "enum",
    "trait",
    "impl",
    "type",
    "const",
    "static",
    "union",
    "macro_rules",
];

/// Check every core source, the allowlist and the inventory. Takes no arguments.
fn run(arguments: &[String]) -> io::Result<Vec<String>> {
    if !arguments.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "the direction gate takes no arguments; got `{given}`",
                given = arguments.join(" ")
            ),
        ));
    }
    let root = shared::workspace_root()?;
    let core = shared::core_crates()?;
    let sources = core_sources(&root, &core)?;
    let variants = subject(&sources)?;

    let mut violations = Vec::new();
    let (allowlist, problems) = parse_allowlist(&read_allowlist(&root.join(ALLOWLIST))?);
    violations.extend(problems);
    let inventory = read_inventory(&root.join(INVENTORY), &mut violations)?;
    check_named_crates(&allowlist, &core, &mut violations);

    let mut census = Census::default();
    let mut predicate_rules = BTreeSet::new();
    let mut allowed_items = BTreeSet::new();
    for source in &sources {
        let found = occurrences(&source.code, &variants);
        if source.generated {
            census.generated = census.generated.saturating_add(1);
            check_generated(
                source,
                &found,
                &inventory,
                &mut predicate_rules,
                &mut census,
                &mut violations,
            );
        } else {
            census.hand_written = census.hand_written.saturating_add(1);
            check_hand_written(
                source,
                &found,
                &allowlist.sites,
                &mut allowed_items,
                &mut violations,
            );
        }
    }

    let deferred = check_pending(
        &allowlist,
        &predicate_rules,
        &inventory,
        &sources,
        &mut violations,
    );
    check_stale_sites(&allowlist.sites, &allowed_items, &mut violations);
    check_union(
        &allowlist.sites,
        &predicate_rules,
        &inventory,
        &deferred,
        &mut violations,
    );
    check_emitted_agrees(&sources, &inventory, &mut violations);
    report_census(&census, &allowlist, &inventory, &deferred);
    Ok(violations)
}

/// What the gate looked at, so that a run finding nothing says what it examined.
#[derive(Debug, Default)]
struct Census {
    /// Hand-written core sources scanned.
    hand_written: usize,
    /// Generated core sources scanned.
    generated: usize,
    /// Direction predicate rows found in generated data.
    rows: usize,
}

/// State what was examined, whether or not anything was found.
///
/// Printed by the check itself rather than carried in the gate's purpose line, because a
/// census is a count and the purpose line is a sentence. A gate whose subject does not
/// exist yet has to say so in numbers, or its silence reads as a pass.
fn report_census(
    census: &Census,
    allowlist: &Allowlist,
    inventory: &Inventory,
    deferred: &BTreeSet<String>,
) {
    let note = if inventory.present {
        String::new()
    } else {
        format!(" ({INVENTORY} has not been generated, so that set is empty)")
    };
    println!(
        "direction: examined {hand} hand-written and {generated} generated core source(s) \
         for a named variant of `{TYPE}`; {ALLOWLIST} names {count} site(s), generated data \
         carries {rows} direction predicate row(s), and the inventory marks {marked} rule(s) \
         direction-conditional{note}",
        hand = census.hand_written,
        generated = census.generated,
        count = allowlist.sites.len(),
        rows = census.rows,
        marked = inventory.conditional.len(),
    );
    if deferred.is_empty() {
        return;
    }
    for rule in deferred {
        let waiting = allowlist
            .pending
            .iter()
            .find(|entry| entry.rule == *rule)
            .map_or("a crate this gate could not name", |entry| {
                entry.crate_name.as_str()
            });
        println!(
            "direction: rule `{rule}` is marked {FLAG_COLUMN} and nothing reads it; \
             {ALLOWLIST} defers it until `{waiting}` declares an item, so that half of the \
             union did not run over it (ADR 0011)"
        );
    }
}

/// One `.rs` file of a core crate, with its comments and string literals blanked out.
#[derive(Debug)]
struct Source {
    /// The package name of the crate it belongs to.
    crate_name: String,
    /// Its path from the workspace root, with forward slashes on every platform.
    shown: String,
    /// Whether it lives under a `generated` directory.
    generated: bool,
    /// The file, with every comment and string literal replaced by blanks.
    code: String,
}

/// Every `.rs` file of every core crate, stripped and labeled.
fn core_sources(root: &Path, core: &[CoreCrate]) -> io::Result<Vec<Source>> {
    let mut sources = Vec::new();
    for each in core {
        let directory = each.directory.join("src");
        for path in shared::rust_sources(&directory)? {
            let relative = path.strip_prefix(&directory).unwrap_or(&path);
            let generated = relative
                .components()
                .any(|component| component.as_os_str() == GENERATED);
            sources.push(Source {
                crate_name: each.name.clone(),
                shown: shown_path(&path, root),
                generated,
                code: strip(&fs::read_to_string(&path)?),
            });
        }
    }
    Ok(sources)
}

/// Name a file the way a reader can open it, on either platform.
fn shown_path(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
        .replace('\\', "/")
}

/// Read the allowlist, or say that the control this gate reads is not there.
///
/// An absent allowlist is not an empty one. The file is what makes a branch on the
/// direction a reviewed decision rather than an edit in passing, so a run without it has
/// checked nothing about check 1 and says so instead of passing everything it permits.
fn read_allowlist(path: &Path) -> io::Result<String> {
    if !path.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "{ALLOWLIST} is missing; it is the control this gate reads, and an absent \
                 allowlist is not an empty one"
            ),
        ));
    }
    fs::read_to_string(path)
}

/// The variants of `Direction`, read from the declaration itself.
///
/// Derived rather than written down, for the reason the workspace member list is: a variant
/// added to the type is covered the moment it is added, and a subject that has moved fails
/// loudly rather than narrowing the check in silence.
fn subject(sources: &[Source]) -> io::Result<Vec<String>> {
    let mut declarations: Vec<(&str, Vec<String>)> = Vec::new();
    for source in sources {
        if let Some(variants) = declared_variants(&source.code) {
            declarations.push((source.shown.as_str(), variants));
        }
    }
    let mut found = declarations.into_iter();
    let Some((_, variants)) = found.next() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("no core crate declares `enum {TYPE}`; this gate has no subject"),
        ));
    };
    let elsewhere: Vec<&str> = found.map(|(shown, _)| shown).collect();
    if !elsewhere.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "`enum {TYPE}` is declared more than once, also in {elsewhere}; one fact has \
                 one carrier (ADR 0019) and this gate cannot tell which declaration is the \
                 subject",
                elsewhere = elsewhere.join(", ")
            ),
        ));
    }
    if variants.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("`enum {TYPE}` declares no variant; this gate has no subject"),
        ));
    }
    Ok(variants)
}

/// The variant names of the `Direction` declaration in one stripped source, if it has one.
fn declared_variants(code: &str) -> Option<Vec<String>> {
    let tokens = paired(code);
    let mut names = Vec::new();
    let mut found = false;
    for (index, token) in tokens.iter().enumerate() {
        if token.text != "enum" || text_at(&tokens, index.saturating_add(1)) != TYPE {
            continue;
        }
        let open = index.saturating_add(2);
        if text_at(&tokens, open) != "{" {
            continue;
        }
        let end = tokens.get(open).and_then(|brace| brace.mate)?;
        found = true;
        names.extend(variant_names(&tokens, open, end));
    }
    found.then_some(names)
}

/// The names declared at the top level of an enum body.
fn variant_names(tokens: &[Token<'_>], open: usize, close: usize) -> Vec<String> {
    let mut names = Vec::new();
    let mut index = open.saturating_add(1);
    while index < close {
        let Some(token) = tokens.get(index) else {
            break;
        };
        match token.text {
            "#" | "[" | "(" | "{" => {
                // An attribute or a variant's own body: skip it whole.
                let group = if token.text == "#" {
                    index = index.saturating_add(1);
                    tokens.get(index).and_then(|next| next.mate)
                } else {
                    token.mate
                };
                index = group.map_or(close, |end| end.saturating_add(1));
            },
            "," => index = index.saturating_add(1),
            text if is_name(text) => {
                names.push(text.to_owned());
                index = skip_to_comma(tokens, index, close);
            },
            _ => index = index.saturating_add(1),
        }
    }
    names
}

/// The index just past the next top-level comma of a group.
fn skip_to_comma(tokens: &[Token<'_>], from: usize, close: usize) -> usize {
    let mut index = from;
    while index < close {
        let Some(token) = tokens.get(index) else {
            break;
        };
        match token.text {
            "," => return index.saturating_add(1),
            "(" | "[" | "{" => {
                index = token.mate.map_or(close, |end| end.saturating_add(1));
            },
            _ => index = index.saturating_add(1),
        }
    }
    close
}

/// What a source does with the type, in the forms a token scan can attribute.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Naming {
    /// `Direction::Vertical`, or `Self::Vertical` inside an `impl Direction`. The one form
    /// that is a branch.
    Variant(String),
    /// `use ..::Direction::*`: every variant, unqualified and unattributable.
    Glob,
    /// `use ..::Direction::{Vertical}`: one variant, unqualified and unattributable.
    Import(String),
    /// `use ..::Direction as Whatever`: the type under a second name.
    Renamed(String),
    /// `type Whatever = Direction`: the same, spelled as an alias.
    Aliased(String),
}

/// Which rule a generated row addresses, as far as the row itself says.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Row {
    /// The row names one rule by its generated constant.
    Named(String),
    /// The row names one rule by its ordinal in the inventory.
    Ordinal(usize),
    /// No enclosing group names a rule at all.
    Absent,
    /// The smallest enclosing group that names a rule names more than one.
    Ambiguous(usize),
}

/// One place a core source names something of `Direction`.
#[derive(Debug)]
struct Occurrence {
    /// The one-based line it sits on.
    line: usize,
    /// What the source did.
    naming: Naming,
    /// The item it sits in, when it sits in one the allowlist could name.
    item: Option<String>,
    /// Whether it is written as the argument of a direction predicate.
    in_predicate: bool,
    /// The rule the enclosing generated row addresses.
    row: Row,
}

/// A brace scope, named by the header that opened it.
#[derive(Debug, Clone)]
enum Scope {
    /// A function body, by the name it is declared with.
    Function(String),
    /// An `impl` block, by the type it targets.
    Implementation(String),
    /// Anything else with braces: a module, a type, a match arm, a block.
    Other,
}

/// Every naming of `Direction` in one stripped source, with what encloses it.
fn occurrences(code: &str, variants: &[String]) -> Vec<Occurrence> {
    let tokens = paired(code);
    let mut found = Vec::new();
    let mut scopes: Vec<Scope> = Vec::new();
    let mut groups: Vec<usize> = Vec::new();
    let mut header: Vec<&str> = Vec::new();

    for (index, token) in tokens.iter().enumerate() {
        for naming in namings(&tokens, index, variants, &scopes) {
            found.push(Occurrence {
                line: token.line,
                naming,
                item: enclosing_item(&scopes),
                in_predicate: in_predicate(&tokens, index),
                row: row_of(&tokens, &groups),
            });
        }
        match token.text {
            "{" => {
                scopes.push(classify(&header));
                groups.push(index);
                header.clear();
            },
            "(" | "[" => groups.push(index),
            "}" => {
                scopes.pop();
                groups.pop();
                header.clear();
            },
            ")" | "]" => {
                groups.pop();
            },
            ";" => header.clear(),
            text => header.push(text),
        }
    }
    found
}

/// What the token at `index` names, if it names anything of `Direction`.
fn namings(
    tokens: &[Token<'_>],
    index: usize,
    variants: &[String],
    scopes: &[Scope],
) -> Vec<Naming> {
    let current = text_at(tokens, index);
    let next = index.saturating_add(1);
    let after = index.saturating_add(2);

    if current == "Self" && text_at(tokens, next) == "::" {
        let named = text_at(tokens, after).to_owned();
        if implementing(scopes) == Some(TYPE) && variants.contains(&named) {
            return vec![Naming::Variant(named)];
        }
        return Vec::new();
    }
    if current == "type" {
        return alias(tokens, index);
    }
    if current != TYPE {
        return Vec::new();
    }
    if text_at(tokens, next) == "as" {
        return vec![Naming::Renamed(text_at(tokens, after).to_owned())];
    }
    if text_at(tokens, next) != "::" {
        return Vec::new();
    }
    match text_at(tokens, after) {
        "*" => vec![Naming::Glob],
        "{" => imported(tokens, after, variants),
        named if variants.iter().any(|each| each == named) => {
            vec![Naming::Variant(named.to_owned())]
        },
        _ => Vec::new(),
    }
}

/// The variants a `use ..::Direction::{..}` list brings into scope unqualified.
fn imported(tokens: &[Token<'_>], open: usize, variants: &[String]) -> Vec<Naming> {
    let Some(close) = tokens.get(open).and_then(|brace| brace.mate) else {
        return Vec::new();
    };
    let mut names = Vec::new();
    let mut index = open.saturating_add(1);
    while index < close {
        let text = text_at(tokens, index);
        if variants.iter().any(|each| each == text) {
            names.push(Naming::Import(text.to_owned()));
        }
        index = index.saturating_add(1);
    }
    names
}

/// A `type Whatever = ..Direction;` alias, which would hide a later named variant.
///
/// Only an alias *of* the type hides one. `type Sites = BTreeMap<Direction, u8>;` names the
/// type inside another and no variant of it is reachable through the alias, so the test is
/// that the aliased path ends in the type rather than that the alias mentions it.
fn alias(tokens: &[Token<'_>], index: usize) -> Vec<Naming> {
    let name = text_at(tokens, index.saturating_add(1));
    if !is_name(name) {
        return Vec::new();
    }
    let mut cursor = index.saturating_add(2);
    let mut assigned = false;
    let mut last = "";
    while let Some(token) = tokens.get(cursor) {
        match token.text {
            "{" | "}" => return Vec::new(),
            ";" => break,
            "=" => assigned = true,
            text if assigned => last = text,
            _ => {},
        }
        cursor = cursor.saturating_add(1);
    }
    if assigned && last == TYPE {
        return vec![Naming::Aliased(name.to_owned())];
    }
    Vec::new()
}

/// Whether the token at `index` is the argument of a direction predicate.
///
/// The row form is `Predicate::InDirection(Direction::Vertical)`, and the path in front of
/// the variant may be spelled out, so the walk steps back over it before asking.
fn in_predicate(tokens: &[Token<'_>], index: usize) -> bool {
    let mut start = index;
    while start >= 2 && text_at(tokens, start.saturating_sub(1)) == "::" {
        start = start.saturating_sub(2);
    }
    if text_at(tokens, start.saturating_sub(1)) != "(" || start < 2 {
        return false;
    }
    let name = start.saturating_sub(2);
    if text_at(tokens, name) != PREDICATE {
        return false;
    }
    // A qualifier is optional, and when written it is the predicate's own enum.
    name < 2 || text_at(tokens, name.saturating_sub(1)) != "::" || {
        text_at(tokens, name.saturating_sub(2)) == PREDICATE_ENUM
    }
}

/// Which rule the smallest enclosing group that addresses one addresses.
///
/// The shape of a generated override row is not frozen by any design document, so the row
/// is found rather than assumed: the innermost enclosing group naming a `RuleId` is the
/// row, and a group naming two is reported as unreadable rather than guessed at.
fn row_of(tokens: &[Token<'_>], groups: &[usize]) -> Row {
    for start in groups.iter().rev() {
        let Some(end) = tokens.get(*start).and_then(|open| open.mate) else {
            continue;
        };
        let addressed = rules_in(tokens, *start, end);
        match addressed.len() {
            0 => {},
            1 => return addressed.first().cloned().unwrap_or(Row::Absent),
            count => return Row::Ambiguous(count),
        }
    }
    Row::Absent
}

/// Every `RuleId` a token range addresses.
fn rules_in(tokens: &[Token<'_>], open: usize, close: usize) -> Vec<Row> {
    let mut addressed = Vec::new();
    let mut index = open.saturating_add(1);
    while index < close {
        if text_at(tokens, index) == RULE_TYPE {
            let next = index.saturating_add(1);
            let after = index.saturating_add(2);
            if text_at(tokens, next) == "::" && is_name(text_at(tokens, after)) {
                addressed.push(Row::Named(text_at(tokens, after).to_owned()));
            } else if text_at(tokens, next) == "(" {
                if let Ok(ordinal) = text_at(tokens, after).parse::<usize>() {
                    addressed.push(Row::Ordinal(ordinal));
                }
            }
        }
        index = index.saturating_add(1);
    }
    addressed
}

/// The item an occurrence sits in, spelled the way the allowlist spells one.
fn enclosing_item(scopes: &[Scope]) -> Option<String> {
    let (depth, name) = scopes
        .iter()
        .enumerate()
        .rev()
        .find_map(|(depth, scope)| match scope {
            Scope::Function(name) => Some((depth, name.clone())),
            _ => None,
        })?;
    let target = scopes
        .iter()
        .take(depth)
        .rev()
        .find_map(|scope| match scope {
            Scope::Implementation(target) => Some(target.clone()),
            _ => None,
        });
    Some(target.map_or(name.clone(), |target| format!("{target}::{name}")))
}

/// The type the innermost enclosing `impl` block targets.
fn implementing(scopes: &[Scope]) -> Option<&str> {
    scopes.iter().rev().find_map(|scope| match scope {
        Scope::Implementation(target) => Some(target.as_str()),
        _ => None,
    })
}

/// Name a brace scope from the tokens that opened it.
fn classify(header: &[&str]) -> Scope {
    let header = match header.iter().position(|token| *token == "where") {
        Some(end) => header.get(..end).unwrap_or(header),
        None => header,
    };
    if let Some(position) = header.iter().position(|token| *token == "fn") {
        let name = header
            .get(position.saturating_add(1))
            .copied()
            .unwrap_or("");
        if is_name(name) {
            return Scope::Function(name.to_owned());
        }
    }
    if header.first().copied() == Some("impl") || header.contains(&"impl") {
        return Scope::Implementation(impl_target(header));
    }
    Scope::Other
}

/// The type an `impl` header targets: what follows `for`, or what follows `impl`.
fn impl_target(header: &[&str]) -> String {
    let outer = without_generics(header);
    let tail = match outer.iter().rposition(|token| *token == "for") {
        Some(position) => outer.get(position.saturating_add(1)..).unwrap_or(&outer),
        None => match outer.iter().position(|token| *token == "impl") {
            Some(position) => outer.get(position.saturating_add(1)..).unwrap_or(&outer),
            None => &outer,
        },
    };
    tail.iter()
        .rev()
        .find(|token| is_name(token))
        .map_or_else(String::new, |name| (*name).to_owned())
}

/// A header with every `<..>` group removed, so a path is the last name in it.
fn without_generics<'a>(header: &[&'a str]) -> Vec<&'a str> {
    let mut outer = Vec::new();
    let mut depth = 0usize;
    for token in header {
        match *token {
            "<" => depth = depth.saturating_add(1),
            ">" => depth = depth.saturating_sub(1),
            text if depth == 0 => outer.push(text),
            _ => {},
        }
    }
    outer
}

/// Reject every hand-written naming of a variant outside an allowlisted item.
fn check_hand_written(
    source: &Source,
    found: &[Occurrence],
    sites: &[Site],
    allowed: &mut BTreeSet<(String, String)>,
    violations: &mut Vec<String>,
) {
    for occurrence in found {
        let Naming::Variant(variant) = &occurrence.naming else {
            violations.push(structural(source, occurrence));
            continue;
        };
        let Some(item) = &occurrence.item else {
            violations.push(format!(
                "{shown}:{line}: `{TYPE}::{variant}` sits in no function, and {ALLOWLIST} \
                 names items, so this naming can never be allowlisted (ADR 0011)",
                shown = source.shown,
                line = occurrence.line,
            ));
            continue;
        };
        let listed = sites
            .iter()
            .any(|site| site.crate_name == source.crate_name && site.item == *item);
        if listed {
            allowed.insert((source.crate_name.clone(), item.clone()));
        } else {
            violations.push(format!(
                "{shown}:{line}: `{TYPE}::{variant}` in `{item}`, which {ALLOWLIST} does not \
                 list; a variant may be named only inside an allowlisted item (ADR 0011)",
                shown = source.shown,
                line = occurrence.line,
            ));
        }
    }
}

/// Reject every generated naming of a variant outside a direction predicate, and collect
/// the rules the surviving ones address.
fn check_generated(
    source: &Source,
    found: &[Occurrence],
    inventory: &Inventory,
    rules: &mut BTreeSet<String>,
    census: &mut Census,
    violations: &mut Vec<String>,
) {
    for occurrence in found {
        let Naming::Variant(variant) = &occurrence.naming else {
            violations.push(structural(source, occurrence));
            continue;
        };
        let at = format!(
            "{shown}:{line}",
            shown = source.shown,
            line = occurrence.line
        );
        if !occurrence.in_predicate {
            violations.push(format!(
                "{at}: `{TYPE}::{variant}` outside `{PREDICATE_ENUM}::{PREDICATE}`; generated \
                 data may name a direction only in that predicate (ADR 0011)"
            ));
            continue;
        }
        census.rows = census.rows.saturating_add(1);
        match &occurrence.row {
            Row::Named(name) => match inventory.by_name.get(name) {
                Some(address) => {
                    rules.insert(address.clone());
                },
                None => violations.push(format!(
                    "{at}: a direction predicate row addresses `{RULE_TYPE}::{name}`, which \
                     {INVENTORY} names no rule for"
                )),
            },
            Row::Ordinal(ordinal) => match inventory.by_ordinal.get(*ordinal) {
                Some(address) => {
                    rules.insert(address.clone());
                },
                None => violations.push(format!(
                    "{at}: a direction predicate row addresses rule {ordinal}, which is past \
                     the end of {INVENTORY}"
                )),
            },
            Row::Absent => violations.push(format!(
                "{at}: a direction predicate row addresses no `{RULE_TYPE}`, so this gate \
                 cannot tell which rule reads the direction"
            )),
            Row::Ambiguous(count) => violations.push(format!(
                "{at}: a direction predicate row addresses {count} rules, so this gate cannot \
                 tell which one reads the direction"
            )),
        }
    }
}

/// The message for a naming that defeats attribution wherever it appears.
fn structural(source: &Source, occurrence: &Occurrence) -> String {
    let at = format!(
        "{shown}:{line}",
        shown = source.shown,
        line = occurrence.line
    );
    match &occurrence.naming {
        Naming::Glob => format!(
            "{at}: `use ..{TYPE}::*` brings every variant into scope unqualified, so a branch \
             on the direction would be invisible to this gate (ADR 0011)"
        ),
        Naming::Import(variant) => format!(
            "{at}: `{TYPE}::{variant}` is imported unqualified; write the qualified path, so \
             that naming a variant stays attributable to an item (ADR 0011)"
        ),
        Naming::Renamed(name) => format!(
            "{at}: `{TYPE}` is renamed to `{name}`; a variant of it would then be named where \
             this gate could not see it (ADR 0011)"
        ),
        Naming::Aliased(name) => format!(
            "{at}: `type {name}` aliases `{TYPE}`; a variant of it would then be named where \
             this gate could not see it (ADR 0011)"
        ),
        Naming::Variant(variant) => format!("{at}: `{TYPE}::{variant}`"),
    }
}

/// Reject an allowlist entry naming a crate that is not part of the layout core.
fn check_named_crates(allowlist: &Allowlist, core: &[CoreCrate], violations: &mut Vec<String>) {
    let core_names: BTreeSet<&str> = core.iter().map(|each| each.name.as_str()).collect();
    let cited = allowlist
        .sites
        .iter()
        .map(|site| (site.line, site.crate_name.as_str()))
        .chain(
            allowlist
                .pending
                .iter()
                .map(|entry| (entry.line, entry.crate_name.as_str())),
        );
    for (line, name) in cited {
        if !core_names.contains(name) {
            violations.push(format!(
                "{ALLOWLIST}:{line}: `{name}` is not a core crate; the key is the package \
                 name, so `jlreq-inline` and never `jlreq_inline`"
            ));
        }
    }
}

/// Reject an allowlist entry that no longer names a site.
///
/// An entry naming an item that names no variant is rot: it permits nothing, and it keeps a
/// rule in the union of check 3 that nothing reads. The same reasoning as the workspace
/// exemption list in `shared.rs`, and the reason the file itself says an entry naming an
/// item that does not exist is an entry no gate can check.
fn check_stale_sites(
    sites: &[Site],
    allowed: &BTreeSet<(String, String)>,
    violations: &mut Vec<String>,
) {
    for site in sites {
        let named = (site.crate_name.clone(), site.item.clone());
        if !allowed.contains(&named) {
            violations.push(format!(
                "{ALLOWLIST}:{line}: `{crate_name}`'s `{item}` names no variant of `{TYPE}`; \
                 the entry has gone stale, and an entry that permits nothing still carries a \
                 rule into the union this gate checks",
                line = site.line,
                crate_name = site.crate_name,
                item = site.item,
            ));
        }
    }
}

/// Which marked rules the allowlist validly defers, and the four ways an entry is wrong.
///
/// A `[[pending]]` entry is the only thing that keeps a marked rule out of check 3's last
/// half, and it is written to expire rather than to be maintained. It is a violation when it
/// defers a rule the inventory does not mark (nothing to wait for), when something already
/// reads that rule (the wait is over and the entry now hides a real answer), and when the
/// crate it names has declared its first item (the reader could have been written, so the
/// gate goes back to demanding one). Only the surviving entries defer, and the census names
/// each of them, so the half that did not run is stated and never claimed.
///
/// This is what keeps the gate from having to choose between two false sentences: that a
/// rule is read when the layer that would read it is an empty crate, or that ADR 0011's
/// equality is broken when what is actually true is that half of it has no subject yet.
///
/// Without the inventory nothing here can be judged — whether an entry defers a marked rule
/// is a question only the inventory answers — so no entry is read and none defers. That
/// costs nothing: with no inventory the marked set is empty and check 3 demands nothing, and
/// the census already reports that the inventory has not been generated.
fn check_pending(
    allowlist: &Allowlist,
    predicate_rules: &BTreeSet<String>,
    inventory: &Inventory,
    sources: &[Source],
    violations: &mut Vec<String>,
) -> BTreeSet<String> {
    if !inventory.present {
        return BTreeSet::new();
    }
    let read: BTreeSet<&str> = allowlist
        .sites
        .iter()
        .map(|site| site.rule.as_str())
        .chain(predicate_rules.iter().map(String::as_str))
        .collect();
    let mut deferred = BTreeSet::new();
    for entry in &allowlist.pending {
        let Pending {
            rule,
            crate_name,
            line,
        } = entry;
        if !inventory.conditional.contains(rule) {
            violations.push(format!(
                "{ALLOWLIST}:{line}: defers rule `{rule}`, which {INVENTORY} does not mark \
                 {FLAG_COLUMN}; there is nothing for it to wait for (ADR 0011)"
            ));
            continue;
        }
        if read.contains(rule.as_str()) {
            violations.push(format!(
                "{ALLOWLIST}:{line}: defers rule `{rule}`, which is already read by an \
                 allowlisted item or a generated predicate row; the entry has served its \
                 purpose and now hides an answer this gate has (ADR 0011)"
            ));
            continue;
        }
        if declares_items(sources, crate_name) {
            violations.push(format!(
                "{ALLOWLIST}:{line}: defers rule `{rule}` until `{crate_name}` declares an \
                 item, and `{crate_name}` now declares one; either the rule is read there \
                 and this file says where, or the entry goes (ADR 0011)"
            ));
            continue;
        }
        deferred.insert(rule.clone());
    }
    deferred
}

/// Whether a core crate declares anything at all yet.
///
/// The question a `[[pending]]` entry expires on. A crate whose sources hold no item
/// keyword is a module comment and an attribute: there is no item for an allowlist to name
/// and no row for a table to carry, so the rule waiting on it could not be read there by
/// anyone. The moment one appears the entry is stale, whether or not it is the item that
/// will do the reading, because from then on the question is answerable.
fn declares_items(sources: &[Source], crate_name: &str) -> bool {
    sources
        .iter()
        .filter(|source| source.crate_name == crate_name)
        .any(|source| {
            paired(&source.code)
                .iter()
                .any(|token| ITEM_KEYWORDS.contains(&token.text))
        })
}

/// The union of the allowlisted rules and the generated predicate rows equals the set the
/// inventory marks direction-conditional.
///
/// The last loop is the half that closes over readers, and `deferred` is the set of marked
/// rules `docs/direction-sites.toml` states have none yet. Those are reported by the census
/// as a check that did not run over them rather than passed on: `check_pending` above is
/// what makes each one a reviewed, self-expiring statement instead of a silence.
fn check_union(
    sites: &[Site],
    predicate_rules: &BTreeSet<String>,
    inventory: &Inventory,
    deferred: &BTreeSet<String>,
    violations: &mut Vec<String>,
) {
    for site in sites {
        if !inventory.conditional.contains(&site.rule) {
            violations.push(format!(
                "{ALLOWLIST}:{line}: names rule `{rule}`, which {INVENTORY} does not mark \
                 {FLAG_COLUMN}; a site reads a rule the inventory marks, or it is not a \
                 direction site (ADR 0011)",
                line = site.line,
                rule = site.rule,
            ));
        }
    }
    for rule in predicate_rules {
        if !inventory.conditional.contains(rule) {
            violations.push(format!(
                "a direction predicate row addresses rule `{rule}`, which {INVENTORY} does \
                 not mark {FLAG_COLUMN} (ADR 0011)"
            ));
        }
    }
    let union: BTreeSet<&String> = sites
        .iter()
        .map(|site| &site.rule)
        .chain(predicate_rules)
        .collect();
    for rule in &inventory.conditional {
        if !union.contains(rule) && !deferred.contains(rule) {
            violations.push(format!(
                "{INVENTORY} marks rule `{rule}` {FLAG_COLUMN} and nothing reads it: no item \
                 in {ALLOWLIST} and no generated predicate row names it, and no `[[pending]]` \
                 entry says which crate's arrival will (ADR 0011)"
            ));
        }
    }
}

/// The emitted inventory and the inventory this gate reads agree about the mark.
///
/// A tripwire against this gate's own blind spot rather than a second reading of the fact:
/// if the emitted table marks rules that the source this gate reads does not, then the
/// reader has gone stale and every set comparison above was made against a set that is
/// silently too small.
fn check_emitted_agrees(sources: &[Source], inventory: &Inventory, violations: &mut Vec<String>) {
    let mut emitted = 0usize;
    for source in sources.iter().filter(|source| source.generated) {
        emitted = emitted.saturating_add(marked_rows(&source.code));
    }
    let marked = inventory.conditional.len();
    if emitted != marked {
        violations.push(format!(
            "the emitted rule inventory marks {emitted} row(s) `{FLAG_COLUMN}: true` and \
             {INVENTORY} marks {marked}; the two forms of one inventory disagree, so this \
             gate is reading a stale source (run `cargo run -p xtask -- generate`)"
        ));
    }
}

/// How many rows of an emitted table carry the direction mark.
fn marked_rows(code: &str) -> usize {
    let tokens = paired(code);
    tokens
        .iter()
        .enumerate()
        .filter(|(index, token)| {
            token.text == FLAG_COLUMN
                && text_at(&tokens, index.saturating_add(1)) == ":"
                && text_at(&tokens, index.saturating_add(2)) == "true"
        })
        .count()
}

/// One `[[site]]` entry of `docs/direction-sites.toml`.
#[derive(Debug)]
struct Site {
    /// The crate the item is declared in, as a package name.
    crate_name: String,
    /// The item, from its crate root with the crate name omitted.
    item: String,
    /// The direction-conditional rule the item reads, as a canonical address.
    rule: String,
    /// The line the entry opens on, so a finding names it.
    line: usize,
}

/// One `[[pending]]` entry of `docs/direction-sites.toml`.
///
/// A marked rule whose reader has not been written yet, and the crate whose first item
/// closes it. This is the only thing that keeps a marked rule out of check 3's "and nothing
/// reads it", and it expires by itself: the entry is a violation the moment its crate
/// declares an item, and a violation the moment anything does read the rule.
#[derive(Debug)]
struct Pending {
    /// The direction-conditional rule nothing reads yet, as a canonical address.
    rule: String,
    /// The crate whose arrival closes it, as a package name.
    crate_name: String,
    /// The line the entry opens on, so a finding names it.
    line: usize,
}

/// Everything `docs/direction-sites.toml` states.
#[derive(Debug, Default)]
struct Allowlist {
    /// The items that may name a variant of the direction.
    sites: Vec<Site>,
    /// The marked rules whose reader has not arrived.
    pending: Vec<Pending>,
}

/// Which of the two tables a draft is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Table {
    /// A `[[site]]`: an item that may name a variant.
    Site,
    /// A `[[pending]]`: a marked rule whose reader has not been written.
    Pending,
}

impl Table {
    /// The table's header, for a message that names what it read.
    fn header(self) -> &'static str {
        match self {
            Self::Site => "[[site]]",
            Self::Pending => "[[pending]]",
        }
    }

    /// The keys the table carries, and no others.
    fn keys(self) -> &'static [&'static str] {
        match self {
            Self::Site => &SITE_KEYS,
            Self::Pending => &PENDING_KEYS,
        }
    }
}

/// One table of the allowlist under construction.
#[derive(Debug)]
struct Draft {
    /// Which table it is.
    table: Table,
    /// The line the table header sits on.
    line: usize,
    /// The keys read so far.
    values: BTreeMap<String, String>,
}

/// Read the allowlist, and complain about anything the schema does not allow.
///
/// Hand-rolled for the reason the manifest scan is: the tool that enforces "the layout core
/// has no outside dependencies" declares none itself. It reads the two forms this file is
/// written in — `[[site]]` and `[[pending]]` tables of one-line basic strings — and rejects
/// everything else rather than skipping it, because a key this reader passed over in silence
/// would be a key no reviewer was told about.
fn parse_allowlist(text: &str) -> (Allowlist, Vec<String>) {
    let mut allowlist = Allowlist::default();
    let mut problems = Vec::new();
    let mut draft: Option<Draft> = None;

    for (offset, raw) in text.lines().enumerate() {
        let line = offset.saturating_add(1);
        let content = before_comment(raw).trim();
        if content.is_empty() {
            continue;
        }
        if content.starts_with('[') {
            close_draft(draft.take(), &mut allowlist, &mut problems);
            draft = match array_header(content) {
                Some("site") => Some(Draft {
                    table: Table::Site,
                    line,
                    values: BTreeMap::new(),
                }),
                Some("pending") => Some(Draft {
                    table: Table::Pending,
                    line,
                    values: BTreeMap::new(),
                }),
                _ => {
                    problems.push(format!(
                        "{ALLOWLIST}:{line}: `{content}` is not a table this file has; the \
                         schema is `[[site]]`, `[[pending]]` and nothing else"
                    ));
                    None
                },
            };
            continue;
        }
        read_key(content, line, draft.as_mut(), &mut problems);
    }
    close_draft(draft.take(), &mut allowlist, &mut problems);
    (allowlist, problems)
}

/// Read one `key = "value"` line into the table it belongs to.
fn read_key(content: &str, line: usize, draft: Option<&mut Draft>, problems: &mut Vec<String>) {
    let Some((key, rest)) = content.split_once('=') else {
        problems.push(format!(
            "{ALLOWLIST}:{line}: `{content}` is neither a table header nor a `key = \"value\"` \
             line"
        ));
        return;
    };
    let key = key.trim();
    let Some(draft) = draft else {
        problems.push(format!(
            "{ALLOWLIST}:{line}: `{key}` sits outside a `[[site]]` table or a `[[pending]]` \
             one; this file has no top-level keys"
        ));
        return;
    };
    let allowed = draft.table.keys();
    if !allowed.contains(&key) {
        problems.push(format!(
            "{ALLOWLIST}:{line}: `{key}` is not a key of `{header}`; the schema is {allowed:?} \
             and nothing else",
            header = draft.table.header()
        ));
        return;
    }
    let Some(value) = basic_string(rest) else {
        problems.push(format!(
            "{ALLOWLIST}:{line}: `{key}` is not a one-line basic string; this reader accepts \
             no other form, so that a value it cannot read is a finding rather than a silence"
        ));
        return;
    };
    if draft
        .values
        .insert(key.to_owned(), value.to_owned())
        .is_some()
    {
        problems.push(format!(
            "{ALLOWLIST}:{line}: `{key}` is written twice in one `{header}` table",
            header = draft.table.header()
        ));
    }
}

/// Turn a finished draft into an entry, or say which key it is missing.
fn close_draft(draft: Option<Draft>, allowlist: &mut Allowlist, problems: &mut Vec<String>) {
    let Some(draft) = draft else { return };
    let line = draft.line;
    let mut missing = Vec::new();
    for key in draft.table.keys() {
        if draft.values.get(*key).is_none_or(String::is_empty) {
            missing.push(*key);
        }
    }
    if !missing.is_empty() {
        problems.push(match draft.table {
            Table::Site => format!(
                "{ALLOWLIST}:{line}: this `[[site]]` table has no {missing:?}; every entry \
                 carries the crate, the item, the rule it reads and why the branch is \
                 unavoidable"
            ),
            Table::Pending => format!(
                "{ALLOWLIST}:{line}: this `[[pending]]` table has no {missing:?}; every entry \
                 carries the rule nothing reads yet, the crate whose first item closes it, \
                 and why that is where it will be read"
            ),
        });
        return;
    }
    let read = |key: &str| draft.values.get(key).cloned().unwrap_or_default();
    match draft.table {
        Table::Site => close_site(
            read("crate"),
            read("item"),
            read("rule"),
            line,
            allowlist,
            problems,
        ),
        Table::Pending => close_pending(read("rule"), read("crate"), line, allowlist, problems),
    }
}

/// Add a finished `[[site]]`, unless the file already carries it.
fn close_site(
    crate_name: String,
    item: String,
    rule: String,
    line: usize,
    allowlist: &mut Allowlist,
    problems: &mut Vec<String>,
) {
    let site = Site {
        crate_name,
        item,
        rule,
        line,
    };
    if let Some(twin) = allowlist
        .sites
        .iter()
        .find(|each| each.crate_name == site.crate_name && each.item == site.item)
        .filter(|each| each.rule == site.rule)
    {
        problems.push(format!(
            "{ALLOWLIST}:{line}: `{crate_name}`'s `{item}` is already listed for rule \
             `{rule}` on line {first}; the file carries one table per item and rule",
            crate_name = site.crate_name,
            item = site.item,
            rule = site.rule,
            first = twin.line,
        ));
        return;
    }
    allowlist.sites.push(site);
}

/// Add a finished `[[pending]]`, unless the file already defers that rule.
fn close_pending(
    rule: String,
    crate_name: String,
    line: usize,
    allowlist: &mut Allowlist,
    problems: &mut Vec<String>,
) {
    if let Some(twin) = allowlist.pending.iter().find(|each| each.rule == rule) {
        problems.push(format!(
            "{ALLOWLIST}:{line}: rule `{rule}` is already deferred on line {first}; one rule \
             waits on one crate, or the file states two answers to the question of where it \
             will be read",
            first = twin.line
        ));
        return;
    }
    allowlist.pending.push(Pending {
        rule,
        crate_name,
        line,
    });
}

/// The name inside an `[[array]]` header, if the line is one.
fn array_header(line: &str) -> Option<&str> {
    line.strip_prefix("[[")
        .and_then(|rest| rest.strip_suffix("]]"))
        .map(str::trim)
}

/// Everything before the first `#` that is not inside a string.
fn before_comment(line: &str) -> &str {
    let mut inside = false;
    let mut escaped = false;
    for (index, character) in line.char_indices() {
        match character {
            '\\' if inside => escaped = !escaped,
            '"' if !escaped => inside = !inside,
            '#' if !inside => return line.get(..index).unwrap_or(line),
            _ => escaped = false,
        }
    }
    line
}

/// The contents of a one-line basic string, when that is all the rest of the line holds.
fn basic_string(rest: &str) -> Option<&str> {
    let opened = rest.trim_start().strip_prefix('"')?;
    let mut escaped = false;
    for (index, character) in opened.char_indices() {
        match character {
            '\\' if !escaped => escaped = true,
            '"' if !escaped => {
                let value = opened.get(..index)?;
                let tail = opened.get(index.saturating_add(1)..)?;
                return tail.trim().is_empty().then_some(value);
            },
            _ => escaped = false,
        }
    }
    None
}

/// The rule inventory, as much of it as this gate reads.
#[derive(Debug, Default)]
struct Inventory {
    /// Whether the inventory has been generated at all.
    present: bool,
    /// Every address in inventory order, which is the ordinal a `RuleId` carries.
    by_ordinal: Vec<String>,
    /// The address of each rule the inventory gives a generated constant a name for.
    by_name: BTreeMap<String, String>,
    /// The addresses the inventory marks direction-conditional.
    conditional: BTreeSet<String>,
}

/// Read the inventory, or record that it has not been generated.
fn read_inventory(path: &Path, violations: &mut Vec<String>) -> io::Result<Inventory> {
    if !path.is_file() {
        return Ok(Inventory::default());
    }
    let (inventory, problems) = parse_inventory(&fs::read_to_string(path)?);
    violations.extend(problems);
    Ok(inventory)
}

/// Read the address, the name and the direction mark out of the inventory.
///
/// Columns are found by name and never by position, and a column this gate needs and cannot
/// find is a finding: a reader that fell back to "no rule is marked" would turn the whole of
/// check 3 into a silent pass.
fn parse_inventory(text: &str) -> (Inventory, Vec<String>) {
    let mut inventory = Inventory {
        present: true,
        ..Inventory::default()
    };
    let mut problems = Vec::new();
    let mut columns: Option<Vec<&str>> = None;

    for (offset, raw) in text.lines().enumerate() {
        let line = offset.saturating_add(1);
        if raw.trim().is_empty() || raw.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = raw.split('\t').map(str::trim).collect();
        let Some(header) = &columns else {
            for required in [ADDRESS_COLUMN, FLAG_COLUMN] {
                if !fields.contains(&required) {
                    problems.push(format!(
                        "{INVENTORY}:{line}: has no `{required}` column; this gate reads the \
                         inventory by column name, and cannot read this one"
                    ));
                }
            }
            columns = Some(fields);
            continue;
        };
        read_rule(header, &fields, line, &mut inventory, &mut problems);
    }
    (inventory, problems)
}

/// Read one inventory row into the three indexes this gate keeps.
fn read_rule(
    header: &[&str],
    fields: &[&str],
    line: usize,
    inventory: &mut Inventory,
    problems: &mut Vec<String>,
) {
    let column = |name: &str| {
        header
            .iter()
            .position(|each| *each == name)
            .and_then(|at| fields.get(at).copied())
    };
    let (Some(address), Some(flag)) = (column(ADDRESS_COLUMN), column(FLAG_COLUMN)) else {
        problems.push(format!(
            "{INVENTORY}:{line}: has fewer fields than the header has columns"
        ));
        return;
    };
    if inventory.by_ordinal.iter().any(|each| each == address) {
        problems.push(format!(
            "{INVENTORY}:{line}: addresses rule `{address}` a second time; one rule has one row"
        ));
        return;
    }
    match flag {
        "true" => {
            inventory.conditional.insert(address.to_owned());
        },
        "false" => {},
        other => problems.push(format!(
            "{INVENTORY}:{line}: `{FLAG_COLUMN}` reads `{other}`; the mark is `true` or \
             `false`, spelled as the emitted field spells it"
        )),
    }
    if let Some(name) = column(NAME_COLUMN).filter(|name| !name.is_empty()) {
        inventory
            .by_name
            .insert(name.to_owned(), address.to_owned());
    }
    inventory.by_ordinal.push(address.to_owned());
}

/// One token of a stripped source: a name, a number, a delimiter, or punctuation.
#[derive(Debug)]
struct Token<'a> {
    /// The token's own text.
    text: &'a str,
    /// The one-based line it sits on.
    line: usize,
    /// For an opening delimiter, the index of the token that closes it.
    mate: Option<usize>,
}

/// Tokenize a stripped source and pair its delimiters.
fn paired(code: &str) -> Vec<Token<'_>> {
    let mut tokens = tokenize(code);
    let mut stack: Vec<usize> = Vec::new();
    let mut mates: Vec<(usize, usize)> = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        match token.text {
            "(" | "[" | "{" => stack.push(index),
            ")" | "]" | "}" => {
                if let Some(open) = stack.pop() {
                    mates.push((open, index));
                }
            },
            _ => {},
        }
    }
    for (open, close) in mates {
        if let Some(token) = tokens.get_mut(open) {
            token.mate = Some(close);
        }
    }
    tokens
}

/// Split stripped code into tokens, keeping the line each sits on.
fn tokenize(code: &str) -> Vec<Token<'_>> {
    let characters: Vec<(usize, char)> = code.char_indices().collect();
    let mut tokens = Vec::new();
    let mut line = 1usize;
    let mut index = 0usize;

    while let Some(&(offset, character)) = characters.get(index) {
        if character == '\n' {
            line = line.saturating_add(1);
            index = index.saturating_add(1);
            continue;
        }
        if character.is_whitespace() {
            index = index.saturating_add(1);
            continue;
        }
        if is_word(character) {
            let mut end = code.len();
            while let Some(&(next, following)) = characters.get(index) {
                if is_word(following) {
                    index = index.saturating_add(1);
                } else {
                    end = next;
                    break;
                }
            }
            tokens.push(Token {
                text: code.get(offset..end).unwrap_or_default(),
                line,
                mate: None,
            });
            continue;
        }
        let mut end = offset.saturating_add(character.len_utf8());
        index = index.saturating_add(1);
        if character == ':' && matches!(characters.get(index), Some(&(_, ':'))) {
            end = end.saturating_add(1);
            index = index.saturating_add(1);
        }
        tokens.push(Token {
            text: code.get(offset..end).unwrap_or_default(),
            line,
            mate: None,
        });
    }
    tokens
}

/// The text of one token, or the empty string past the end.
fn text_at<'a>(tokens: &[Token<'a>], index: usize) -> &'a str {
    tokens.get(index).map_or("", |token| token.text)
}

/// Whether a character can sit inside a name.
fn is_word(character: char) -> bool {
    character == '_' || character.is_alphanumeric()
}

/// Whether a token is a name rather than a number or punctuation.
fn is_name(text: &str) -> bool {
    let mut characters = text.chars();
    characters
        .next()
        .is_some_and(|first| first == '_' || first.is_alphabetic())
        && characters.all(is_word)
}

/// Blank out every comment and string literal, keeping every line where it was.
///
/// ADR 0011 defines the scan over code with comments and string literals stripped, so this
/// is that definition rather than an approximation of it: block comments nest, raw strings
/// carry their hashes, and a character literal holding a quote does not open a string. What
/// is removed is replaced by blanks rather than deleted, so a finding names the line the
/// reader will find it on.
fn strip(source: &str) -> String {
    let characters: Vec<char> = source.chars().collect();
    let mut stripped = String::with_capacity(source.len());
    let mut index = 0usize;
    let mut previous = ' ';

    while let Some(&character) = characters.get(index) {
        let next = characters.get(index.saturating_add(1)).copied();
        let step = match (character, next) {
            ('/', Some('/')) => line_comment(&characters, index, &mut stripped),
            ('/', Some('*')) => block_comment(&characters, index, &mut stripped),
            ('\'', _) if !is_word(previous) => character_literal(&characters, index, &mut stripped),
            ('"', _) => text_literal(&characters, index, &mut stripped),
            ('r' | 'b', _) if !is_word(previous) => raw_literal(&characters, index, &mut stripped),
            _ => 0,
        };
        if step > 0 {
            index = index.saturating_add(step);
            previous = ' ';
            continue;
        }
        stripped.push(character);
        previous = character;
        index = index.saturating_add(1);
    }
    stripped
}

/// Blank a `//` comment through to the end of its line.
fn line_comment(characters: &[char], from: usize, stripped: &mut String) -> usize {
    let mut length = 0usize;
    while let Some(&character) = characters.get(from.saturating_add(length)) {
        if character == '\n' {
            break;
        }
        stripped.push(' ');
        length = length.saturating_add(1);
    }
    length
}

/// Blank a `/* */` comment, which nests.
fn block_comment(characters: &[char], from: usize, stripped: &mut String) -> usize {
    let mut length = 2usize;
    let mut depth = 1usize;
    stripped.push_str("  ");
    while depth > 0 {
        let at = from.saturating_add(length);
        let Some(&character) = characters.get(at) else {
            break;
        };
        let following = characters.get(at.saturating_add(1)).copied();
        let (width, text) = match (character, following) {
            ('/', Some('*')) => {
                depth = depth.saturating_add(1);
                (2usize, "  ")
            },
            ('*', Some('/')) => {
                depth = depth.saturating_sub(1);
                (2usize, "  ")
            },
            ('\n', _) => (1usize, "\n"),
            _ => (1usize, " "),
        };
        stripped.push_str(text);
        length = length.saturating_add(width);
    }
    length
}

/// Blank a `"` string, or a `b"` byte string, honoring backslash escapes.
fn text_literal(characters: &[char], from: usize, stripped: &mut String) -> usize {
    let mut length = 1usize;
    stripped.push(' ');
    let mut escaped = false;
    while let Some(&character) = characters.get(from.saturating_add(length)) {
        length = length.saturating_add(1);
        stripped.push(if character == '\n' { '\n' } else { ' ' });
        match character {
            '\\' if !escaped => escaped = true,
            '"' if !escaped => break,
            _ => escaped = false,
        }
    }
    length
}

/// Blank an `r"…"`, `r#"…"#` or `br#"…"#` literal, or nothing when the prefix opens none.
fn raw_literal(characters: &[char], from: usize, stripped: &mut String) -> usize {
    let mut length = 1usize;
    if characters.get(from) == Some(&'b') && characters.get(from.saturating_add(1)) == Some(&'r') {
        length = 2;
    }
    if characters.get(from) == Some(&'b') && length == 1 {
        // A byte string carries no hashes; the ordinary string reader handles it.
        return if characters.get(from.saturating_add(1)) == Some(&'"') {
            stripped.push(' ');
            text_literal(characters, from.saturating_add(1), stripped).saturating_add(1)
        } else {
            0
        };
    }
    let mut hashes = 0usize;
    while characters.get(from.saturating_add(length)) == Some(&'#') {
        hashes = hashes.saturating_add(1);
        length = length.saturating_add(1);
    }
    if characters.get(from.saturating_add(length)) != Some(&'"') {
        return 0;
    }
    length = length.saturating_add(1);
    for _ in 0..length {
        stripped.push(' ');
    }
    length.saturating_add(raw_body(
        characters,
        from.saturating_add(length),
        hashes,
        stripped,
    ))
}

/// Blank the body of a raw literal, which ends at a quote followed by its own hashes.
fn raw_body(characters: &[char], from: usize, hashes: usize, stripped: &mut String) -> usize {
    let mut length = 0usize;
    loop {
        let at = from.saturating_add(length);
        let Some(&character) = characters.get(at) else {
            break;
        };
        if character == '"' {
            let closed =
                (1..=hashes).all(|step| characters.get(at.saturating_add(step)) == Some(&'#'));
            if closed {
                for _ in 0..hashes.saturating_add(1) {
                    stripped.push(' ');
                }
                return length.saturating_add(hashes).saturating_add(1);
            }
        }
        stripped.push(if character == '\n' { '\n' } else { ' ' });
        length = length.saturating_add(1);
    }
    length
}

/// Blank a `'x'` literal, and leave a `'a` lifetime where it is.
fn character_literal(characters: &[char], from: usize, stripped: &mut String) -> usize {
    let first = characters.get(from.saturating_add(1)).copied();
    let second = characters.get(from.saturating_add(2)).copied();
    if first != Some('\\') && second != Some('\'') {
        return 0;
    }
    let mut length = 1usize;
    stripped.push(' ');
    let mut escaped = false;
    while let Some(&character) = characters.get(from.saturating_add(length)) {
        length = length.saturating_add(1);
        stripped.push(if character == '\n' { '\n' } else { ' ' });
        match character {
            '\\' if !escaped => escaped = true,
            '\'' if !escaped && length > 2 => break,
            _ => escaped = false,
        }
    }
    length
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::{
        Allowlist, Census, CoreCrate, Inventory, Naming, Occurrence, Pending, Row, Site, Source,
        check_emitted_agrees, check_generated, check_hand_written, check_named_crates,
        check_pending, check_stale_sites, check_union, declared_variants, occurrences,
        parse_allowlist, parse_inventory, read_allowlist, run, strip,
    };

    /// The declaration the workspace actually carries, as a fixture.
    const DECLARATION: &str = "pub enum Direction {\n    Horizontal,\n    Vertical,\n}\n";

    /// The two variants, as every scan test needs them.
    fn variants() -> Vec<String> {
        vec!["Horizontal".to_owned(), "Vertical".to_owned()]
    }

    /// A hand-written source of `jlreq-inline`, for the check that reads one.
    fn hand_written(code: &str) -> Source {
        Source {
            crate_name: "jlreq-inline".to_owned(),
            shown: "crates/jlreq-inline/src/lower.rs".to_owned(),
            generated: false,
            code: strip(code),
        }
    }

    /// A generated source of `jlreq-spacing`, for the check that reads one.
    fn generated(code: &str) -> Source {
        Source {
            crate_name: "jlreq-spacing".to_owned(),
            shown: "crates/jlreq-spacing/src/generated/notes.rs".to_owned(),
            generated: true,
            code: strip(code),
        }
    }

    /// One allowlist entry, without going through the file.
    fn site(item: &str, rule: &str) -> Site {
        Site {
            crate_name: "jlreq-inline".to_owned(),
            item: item.to_owned(),
            rule: rule.to_owned(),
            line: 60,
        }
    }

    /// One deferral, without going through the file.
    fn pending(rule: &str, crate_name: &str) -> Pending {
        Pending {
            rule: rule.to_owned(),
            crate_name: crate_name.to_owned(),
            line: 80,
        }
    }

    /// An allowlist holding only deferrals, which is this repository's shape today.
    fn deferring(entries: Vec<Pending>) -> Allowlist {
        Allowlist {
            sites: Vec::new(),
            pending: entries,
        }
    }

    /// A crate that has not started: a module comment and an attribute, nothing else.
    fn unstarted(crate_name: &str) -> Source {
        Source {
            crate_name: crate_name.to_owned(),
            shown: format!("crates/{crate_name}/src/lib.rs"),
            generated: false,
            code: strip("//! Mojikumi.\n#![no_std]\n"),
        }
    }

    /// An inventory marking one rule, without going through the file.
    fn marking(rules: &[&str]) -> Inventory {
        Inventory {
            present: true,
            by_ordinal: rules.iter().map(|rule| (*rule).to_owned()).collect(),
            by_name: BTreeMap::from([("KATATSUKI".to_owned(), "3.3.5".to_owned())]),
            conditional: rules.iter().map(|rule| (*rule).to_owned()).collect(),
        }
    }

    /// Run the hand-written check over one fixture and return what it found.
    fn scan_hand_written(code: &str, sites: &[Site]) -> Vec<String> {
        let source = hand_written(code);
        let found = occurrences(&source.code, &variants());
        let mut violations = Vec::new();
        let mut allowed = BTreeSet::new();
        check_hand_written(&source, &found, sites, &mut allowed, &mut violations);
        violations
    }

    /// Run the generated check over one fixture and return what it found and addressed.
    fn scan_generated(code: &str, inventory: &Inventory) -> (Vec<String>, BTreeSet<String>) {
        let source = generated(code);
        let found = occurrences(&source.code, &variants());
        let mut violations = Vec::new();
        let mut rules = BTreeSet::new();
        let mut census = Census::default();
        check_generated(
            &source,
            &found,
            inventory,
            &mut rules,
            &mut census,
            &mut violations,
        );
        (violations, rules)
    }

    #[test]
    fn the_variants_are_read_from_the_declaration() {
        assert_eq!(
            declared_variants(&strip(DECLARATION)),
            Some(variants()),
            "the subject is derived, so a variant added to the type is covered at once"
        );
        assert_eq!(
            declared_variants(&strip("pub enum Side { BlockStart, BlockEnd }")),
            None,
            "another two-valued enum is not this gate's subject"
        );
    }

    #[test]
    fn a_variant_with_an_attribute_or_a_body_is_still_one_variant() {
        let declaration = "enum Direction {\n    #[cfg(feature = \"x\")]\n    Horizontal,\n    \
                           Vertical(Axis),\n}";
        assert_eq!(declared_variants(&strip(declaration)), Some(variants()));
    }

    #[test]
    fn naming_the_type_is_not_naming_a_variant() {
        let code = "pub fn lower(direction: Direction) -> Direction { direction }";
        assert!(
            occurrences(&strip(code), &variants()).is_empty(),
            "the direction is threaded by signature through most of the workspace"
        );
    }

    #[test]
    fn an_associated_item_is_not_a_variant() {
        let code = "fn lower() { let _ = Direction::default(); }";
        assert!(occurrences(&strip(code), &variants()).is_empty());
    }

    #[test]
    fn a_variant_outside_an_allowlisted_item_is_a_violation() {
        let code = "fn place() {\n    let solid = Direction::Vertical;\n}";
        let violations = scan_hand_written(code, &[]);
        assert_eq!(violations.len(), 1, "found {violations:?}");
        let first = violations.first().map(String::as_str).unwrap_or_default();
        assert!(first.contains(":2:"), "the line is reported: {first}");
        assert!(first.contains("`place`"), "the item is reported: {first}");
    }

    #[test]
    fn a_variant_inside_an_allowlisted_item_is_permitted() {
        let code = "fn lower() {\n    let _ = Direction::Horizontal;\n}";
        assert!(
            scan_hand_written(code, &[site("lower", "3.3.5")]).is_empty(),
            "the allowlist is what makes the branch reviewable rather than absent"
        );
    }

    #[test]
    fn an_item_is_named_by_its_type_when_it_has_one() {
        let code = "impl Segment {\n    fn new() {\n        let _ = Direction::Vertical;\n    }\n}";
        assert!(
            scan_hand_written(code, &[site("Segment::new", "3.2.5")]).is_empty(),
            "an inherent method is `Type::method`, as the schema spells it"
        );
        let violations = scan_hand_written(code, &[site("new", "3.2.5")]);
        assert_eq!(
            violations.len(),
            1,
            "`new` alone does not name `Segment::new`: {violations:?}"
        );
    }

    #[test]
    fn a_closure_is_named_by_the_item_containing_it() {
        let code = "fn lower() {\n    let f = |x| { Direction::Vertical };\n}";
        assert!(
            scan_hand_written(code, &[site("lower", "3.3.5")]).is_empty(),
            "the schema says a nested closure or block is named by its item"
        );
    }

    #[test]
    fn a_variant_outside_every_function_can_never_be_allowlisted() {
        let code = "const DEFAULT: Direction = Direction::Horizontal;";
        let violations = scan_hand_written(code, &[site("DEFAULT", "3.3.5")]);
        assert_eq!(violations.len(), 1, "found {violations:?}");
        let first = violations.first().map(String::as_str).unwrap_or_default();
        assert!(first.contains("sits in no function"), "{first}");
    }

    #[test]
    fn self_is_a_variant_inside_the_types_own_impl() {
        let branching = "impl Direction {\n    fn solid(self) -> bool {\n        \
                         matches!(self, Self::Vertical)\n    }\n}";
        let violations = scan_hand_written(branching, &[]);
        assert_eq!(
            violations.len(),
            1,
            "a predicate over the direction is the mode flag under another name: {violations:?}"
        );
        let elsewhere =
            "impl Side {\n    fn flip(self) -> Self {\n        Self::BlockEnd\n    }\n}";
        assert!(
            scan_hand_written(elsewhere, &[]).is_empty(),
            "`Self` in another type's impl is another type's variant"
        );
    }

    #[test]
    fn the_three_spellings_that_defeat_attribution_are_refused() {
        for (code, expected) in [
            ("use jlreq_unit::Direction::*;", "into scope unqualified"),
            (
                "use jlreq_unit::Direction::{Horizontal, Vertical};",
                "imported unqualified",
            ),
            ("use jlreq_unit::Direction as Way;", "is renamed to `Way`"),
            ("type Way = jlreq_unit::Direction;", "aliases"),
        ] {
            let violations = scan_hand_written(code, &[site("lower", "3.3.5")]);
            assert!(
                violations
                    .first()
                    .is_some_and(|first| first.contains(expected)),
                "`{code}` is refused with `{expected}`: {violations:?}"
            );
        }
    }

    #[test]
    fn naming_the_type_inside_another_is_not_aliasing_it() {
        let code = "type Sites = BTreeMap<Direction, u8>;\nfn lower(sites: &Sites) {}";
        assert!(
            occurrences(&strip(code), &variants()).is_empty(),
            "no variant is reachable through `Sites`, so this hides nothing"
        );
    }

    #[test]
    fn importing_the_type_itself_is_not_importing_a_variant() {
        let code = "use jlreq_unit::{Direction, Side};\nuse jlreq_unit::Direction;";
        assert!(
            occurrences(&strip(code), &variants()).is_empty(),
            "naming the type is unrestricted"
        );
    }

    #[test]
    fn a_variant_in_a_comment_or_a_string_is_not_a_naming() {
        let code = "//! Composition reads Direction::Vertical only in `lower`.\n\
                    /* Direction::Horizontal */\n\
                    fn lower() { let note = \"Direction::Vertical\"; }";
        assert!(
            occurrences(&strip(code), &variants()).is_empty(),
            "ADR 0011 defines the scan over stripped sources"
        );
    }

    #[test]
    fn stripping_keeps_every_line_where_it_was() {
        let code = "fn a() {}\n/* one\n   two */\nfn b() { let _ = Direction::Vertical; }";
        let violations = scan_hand_written(code, &[]);
        let first = violations.first().map(String::as_str).unwrap_or_default();
        assert!(
            first.contains(":4:"),
            "the line survives stripping: {first}"
        );
    }

    #[test]
    fn a_quote_inside_a_character_literal_does_not_open_a_string() {
        let code = "fn lower() { let quote = '\"'; let _ = Direction::Vertical; }";
        assert_eq!(
            scan_hand_written(code, &[]).len(),
            1,
            "a naming after a character literal is still seen"
        );
    }

    #[test]
    fn a_lifetime_is_not_a_character_literal() {
        let code = "fn lower<'a>(text: &'a str) -> &'a str {\n    let _ = Direction::Vertical;\n    \
                    text\n}";
        assert_eq!(
            scan_hand_written(code, &[]).len(),
            1,
            "a lifetime does not swallow the rest of the file"
        );
    }

    #[test]
    fn a_raw_string_holding_a_variant_is_still_a_string() {
        let code = "fn lower() { let note = r#\"Direction::Vertical\"#; }";
        assert!(scan_hand_written(code, &[]).is_empty());
    }

    #[test]
    fn generated_data_may_name_a_direction_only_in_the_predicate() {
        let row = "static NOTES: &[Override] = &[Override { rule: RuleId::KATATSUKI, predicate: \
                   Predicate::InDirection(Direction::Horizontal) }];";
        let (violations, rules) = scan_generated(row, &marking(&["3.3.5"]));
        assert!(violations.is_empty(), "found {violations:?}");
        assert_eq!(
            rules,
            BTreeSet::from(["3.3.5".to_owned()]),
            "the row's rule is what check 3 unions"
        );

        let loose = "static NOTES: &[Override] = &[Override { rule: RuleId::KATATSUKI, amount: \
                     Direction::Horizontal }];";
        let (violations, rules) = scan_generated(loose, &marking(&["3.3.5"]));
        assert_eq!(violations.len(), 1, "found {violations:?}");
        assert!(rules.is_empty(), "a refused row addresses nothing");
    }

    #[test]
    fn a_predicate_row_that_addresses_no_rule_is_unreadable_rather_than_ignored() {
        let row = "static NOTES: &[Predicate] = &[Predicate::InDirection(Direction::Vertical)];";
        let (violations, rules) = scan_generated(row, &marking(&["3.2.5"]));
        assert_eq!(violations.len(), 1, "found {violations:?}");
        assert!(rules.is_empty());
        let first = violations.first().map(String::as_str).unwrap_or_default();
        assert!(first.contains("addresses no `RuleId`"), "{first}");
    }

    #[test]
    fn a_predicate_row_addressing_two_rules_is_refused() {
        let row = "static NOTES: &[Override] = &[Override { rule: RuleId::A, then: RuleId::B, \
                   predicate: Predicate::InDirection(Direction::Vertical) }];";
        let (violations, _) = scan_generated(row, &marking(&["3.2.5"]));
        assert!(
            violations
                .first()
                .is_some_and(|first| first.contains("addresses 2 rules")),
            "found {violations:?}"
        );
    }

    #[test]
    fn a_predicate_row_addressing_a_rule_by_ordinal_reads_the_inventory_by_ordinal() {
        let row = "static NOTES: &[Override] = &[Override(RuleId(1), \
                   Predicate::InDirection(Direction::Vertical))];";
        let (violations, rules) = scan_generated(row, &marking(&["3.1.3", "3.2.5"]));
        assert!(violations.is_empty(), "found {violations:?}");
        assert_eq!(rules, BTreeSet::from(["3.2.5".to_owned()]));
    }

    #[test]
    fn a_predicate_row_naming_a_rule_the_inventory_has_not_got_is_refused() {
        let row = "static NOTES: &[Override] = &[Override { rule: RuleId::UNKNOWN, predicate: \
                   Predicate::InDirection(Direction::Vertical) }];";
        let (violations, rules) = scan_generated(row, &marking(&["3.2.5"]));
        assert_eq!(violations.len(), 1, "found {violations:?}");
        assert!(rules.is_empty());
    }

    #[test]
    fn the_union_must_equal_the_marked_set() {
        let mut violations = Vec::new();
        check_union(
            &[site("lower", "3.3.5")],
            &BTreeSet::from(["3.2.5".to_owned()]),
            &marking(&["3.2.5", "3.3.5"]),
            &BTreeSet::new(),
            &mut violations,
        );
        assert!(violations.is_empty(), "found {violations:?}");
    }

    #[test]
    fn a_site_for_a_rule_the_inventory_does_not_mark_is_a_violation() {
        let mut violations = Vec::new();
        check_union(
            &[site("lower", "3.1.7")],
            &BTreeSet::new(),
            &marking(&["3.3.5"]),
            &BTreeSet::new(),
            &mut violations,
        );
        assert_eq!(violations.len(), 2, "found {violations:?}");
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains("does not mark")),
            "the site names a rule that is not direction-conditional: {violations:?}"
        );
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains("nothing reads it")),
            "and the rule that is has no site: {violations:?}"
        );
    }

    #[test]
    fn a_marked_rule_nothing_reads_is_a_violation() {
        let mut violations = Vec::new();
        check_union(
            &[],
            &BTreeSet::new(),
            &marking(&["3.2.5"]),
            &BTreeSet::new(),
            &mut violations,
        );
        assert_eq!(violations.len(), 1, "found {violations:?}");
    }

    #[test]
    fn an_empty_inventory_and_an_empty_allowlist_agree() {
        let mut violations = Vec::new();
        check_union(
            &[],
            &BTreeSet::new(),
            &Inventory::default(),
            &BTreeSet::new(),
            &mut violations,
        );
        assert!(
            violations.is_empty(),
            "nothing marked and nothing reading it is the honest state today: {violations:?}"
        );
    }

    #[test]
    fn a_deferred_rule_is_not_reported_as_unread_and_nothing_else_is_deferred() {
        let mut violations = Vec::new();
        check_union(
            &[],
            &BTreeSet::new(),
            &marking(&["3.2.5", "3.3.5"]),
            &BTreeSet::from(["3.2.5".to_owned()]),
            &mut violations,
        );
        assert_eq!(
            violations.len(),
            1,
            "the deferred rule is the census's business and the other is still unread: \
             {violations:?}"
        );
        assert!(
            violations
                .first()
                .is_some_and(|violation| violation.contains("`3.3.5`")),
            "found {violations:?}"
        );
    }

    #[test]
    fn a_deferral_holds_only_while_its_crate_declares_nothing() {
        let sources = [unstarted("jlreq-spacing")];
        let allowlist = deferring(vec![pending("3.1.3", "jlreq-spacing")]);
        let mut violations = Vec::new();
        let deferred = check_pending(
            &allowlist,
            &BTreeSet::new(),
            &marking(&["3.1.3"]),
            &sources,
            &mut violations,
        );
        assert!(violations.is_empty(), "found {violations:?}");
        assert_eq!(deferred, BTreeSet::from(["3.1.3".to_owned()]));

        let started = [Source {
            crate_name: "jlreq-spacing".to_owned(),
            shown: "crates/jlreq-spacing/src/lib.rs".to_owned(),
            generated: false,
            code: strip("#![no_std]\npub struct Boundary;\n"),
        }];
        let mut violations = Vec::new();
        let deferred = check_pending(
            &allowlist,
            &BTreeSet::new(),
            &marking(&["3.1.3"]),
            &started,
            &mut violations,
        );
        assert!(deferred.is_empty(), "the entry has expired");
        assert_eq!(violations.len(), 1, "found {violations:?}");
        assert!(
            violations
                .first()
                .is_some_and(|violation| violation.contains("now declares one")),
            "found {violations:?}"
        );
    }

    #[test]
    fn a_deferral_of_a_rule_that_is_read_or_is_not_marked_is_a_violation() {
        let sources = [unstarted("jlreq-inline")];
        let allowlist = deferring(vec![pending("3.3.5", "jlreq-inline")]);

        let mut violations = Vec::new();
        let deferred = check_pending(
            &allowlist,
            &BTreeSet::from(["3.3.5".to_owned()]),
            &marking(&["3.3.5"]),
            &sources,
            &mut violations,
        );
        assert!(deferred.is_empty(), "a rule with a reader is not deferred");
        assert!(
            violations
                .first()
                .is_some_and(|violation| violation.contains("already read")),
            "found {violations:?}"
        );

        let mut violations = Vec::new();
        let deferred = check_pending(
            &allowlist,
            &BTreeSet::new(),
            &marking(&["3.1.3"]),
            &sources,
            &mut violations,
        );
        assert!(deferred.is_empty(), "an unmarked rule waits for nothing");
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains("nothing for it to wait for")),
            "found {violations:?}"
        );
    }

    #[test]
    fn without_the_inventory_a_deferral_is_neither_judged_nor_honored() {
        let sources = [unstarted("jlreq-inline")];
        let allowlist = deferring(vec![pending("3.3.5", "jlreq-inline")]);
        let mut violations = Vec::new();
        let deferred = check_pending(
            &allowlist,
            &BTreeSet::new(),
            &Inventory::default(),
            &sources,
            &mut violations,
        );
        assert!(
            violations.is_empty(),
            "which rules are marked is a question only the inventory answers, and it has \
             not been generated: {violations:?}"
        );
        assert!(
            deferred.is_empty(),
            "and nothing is deferred, which costs nothing because the marked set is empty too"
        );
    }

    #[test]
    fn an_allowlist_entry_naming_something_that_is_not_a_core_crate_is_a_violation() {
        let core = [CoreCrate {
            name: "jlreq-inline".to_owned(),
            directory: std::path::PathBuf::from("crates/jlreq-inline"),
        }];
        let allowlist = Allowlist {
            sites: vec![site("lower", "3.3.5")],
            pending: vec![pending("3.1.3", "jlreq-inline")],
        };
        let mut violations = Vec::new();
        check_named_crates(&allowlist, &core, &mut violations);
        assert!(violations.is_empty(), "found {violations:?}");

        let mut misspelled = site("lower", "3.3.5");
        misspelled.crate_name = "jlreq_inline".to_owned();
        let allowlist = Allowlist {
            sites: vec![misspelled],
            pending: vec![pending("3.1.3", "jlreq_spacing")],
        };
        let mut violations = Vec::new();
        check_named_crates(&allowlist, &core, &mut violations);
        assert_eq!(
            violations.len(),
            2,
            "the key is the package name in both tables, and the two spellings are not the \
             same crate"
        );
    }

    #[test]
    fn a_stale_allowlist_entry_is_a_violation() {
        let mut violations = Vec::new();
        check_stale_sites(&[site("lower", "3.3.5")], &BTreeSet::new(), &mut violations);
        assert_eq!(violations.len(), 1, "found {violations:?}");
        let allowed = BTreeSet::from([("jlreq-inline".to_owned(), "lower".to_owned())]);
        let mut none = Vec::new();
        check_stale_sites(&[site("lower", "3.3.5")], &allowed, &mut none);
        assert!(none.is_empty(), "found {none:?}");
    }

    #[test]
    fn the_emitted_inventory_and_its_source_must_agree_about_the_mark() {
        let emitted = generated(
            "const RULES: &[Rule] = &[Rule { direction_conditional: true, }, Rule { \
             direction_conditional: false, }];",
        );
        let mut violations = Vec::new();
        check_emitted_agrees(&[emitted], &marking(&["3.2.5"]), &mut violations);
        assert!(violations.is_empty(), "found {violations:?}");

        let emitted = generated("const RULES: &[Rule] = &[Rule { direction_conditional: true, }];");
        let mut violations = Vec::new();
        check_emitted_agrees(&[emitted], &Inventory::default(), &mut violations);
        assert_eq!(
            violations.len(),
            1,
            "a reader that had gone stale would otherwise compare against an empty set"
        );
    }

    #[test]
    fn the_allowlist_reads_the_four_keys_and_refuses_the_rest() {
        let text = "[[site]]\ncrate = \"jlreq-inline\"\nitem = \"lower\"\nrule = \"3.3.5\"\n\
                    why = \"katatsuki is resolved here\"\n";
        let (allowlist, problems) = parse_allowlist(text);
        assert!(problems.is_empty(), "found {problems:?}");
        assert_eq!(allowlist.sites.len(), 1);
        assert_eq!(
            allowlist.sites.first().map(|site| site.rule.as_str()),
            Some("3.3.5")
        );
    }

    #[test]
    fn an_allowlist_entry_missing_a_key_is_refused() {
        let text = "[[site]]\ncrate = \"jlreq-inline\"\nitem = \"lower\"\nrule = \"3.3.5\"\n";
        let (allowlist, problems) = parse_allowlist(text);
        assert!(
            allowlist.sites.is_empty(),
            "an incomplete entry permits nothing"
        );
        assert_eq!(problems.len(), 1, "found {problems:?}");
        assert!(
            problems
                .first()
                .is_some_and(|problem| problem.contains("why")),
            "the missing key is named: {problems:?}"
        );
    }

    #[test]
    fn an_unknown_key_or_table_in_the_allowlist_is_refused() {
        let text = "[[site]]\ncrate = \"jlreq-inline\"\nitem = \"lower\"\nrule = \"3.3.5\"\n\
                    why = \"because\"\nsince = \"today\"\n\n[[exception]]\ncrate = \"x\"\n";
        let (_, problems) = parse_allowlist(text);
        assert_eq!(problems.len(), 3, "found {problems:?}");
        assert!(problems.iter().any(|problem| problem.contains("`since`")));
        assert!(
            problems
                .iter()
                .any(|problem| problem.contains("[[exception]]"))
        );
        assert!(
            problems
                .iter()
                .any(|problem| problem.contains("sits outside a `[[site]]` table or a")),
            "a table this file has no schema for holds no keys either: {problems:?}"
        );
    }

    #[test]
    fn a_repeated_item_and_rule_in_the_allowlist_is_refused() {
        let entry = "[[site]]\ncrate = \"jlreq-inline\"\nitem = \"lower\"\nrule = \"3.3.5\"\n\
                     why = \"because\"\n";
        let (allowlist, problems) = parse_allowlist(&format!("{entry}{entry}"));
        assert_eq!(
            allowlist.sites.len(),
            1,
            "the second is not a second permission"
        );
        assert_eq!(problems.len(), 1, "found {problems:?}");
    }

    #[test]
    fn one_item_reading_two_rules_is_two_entries_and_not_a_repetition() {
        let text = "[[site]]\ncrate = \"jlreq-inline\"\nitem = \"lower\"\nrule = \"3.3.5\"\n\
                    why = \"because\"\n\n[[site]]\ncrate = \"jlreq-inline\"\nitem = \"lower\"\n\
                    rule = \"3.2.5\"\nwhy = \"and because\"\n";
        let (allowlist, problems) = parse_allowlist(text);
        assert!(problems.is_empty(), "found {problems:?}");
        assert_eq!(
            allowlist.sites.len(),
            2,
            "the union in check 3 is this column"
        );
    }

    #[test]
    fn a_comment_in_the_allowlist_is_not_an_entry() {
        let text = "# [[site]]\n# crate = \"jlreq-inline\"\n";
        let (allowlist, problems) = parse_allowlist(text);
        assert!(
            allowlist.sites.is_empty(),
            "the file has no site today by design"
        );
        assert!(problems.is_empty(), "found {problems:?}");
    }

    #[test]
    fn the_allowlist_this_repository_ships_parses() {
        let text = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join(super::ALLOWLIST),
        )
        .expect("the allowlist is readable");
        let (allowlist, problems) = parse_allowlist(&text);
        assert!(problems.is_empty(), "found {problems:?}");
        assert!(
            allowlist.sites.is_empty(),
            "the file names no site until a composition source names a variant"
        );
        assert_eq!(
            allowlist
                .pending
                .iter()
                .map(|entry| entry.rule.as_str())
                .collect::<Vec<&str>>(),
            vec!["3.1.3", "3.2.5", "3.3.5"],
            "and it defers the whole marked set, because no crate that reads one has started"
        );
    }

    #[test]
    fn an_absent_allowlist_is_not_an_empty_one() {
        let missing = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("no-such-file.toml");
        assert!(
            read_allowlist(&missing).is_err(),
            "a run without the control has checked nothing, and says so"
        );
    }

    #[test]
    fn the_inventory_is_read_by_column_name() {
        let text = "address\tname\tdirection_conditional\tstanding\n\
                    3.1.3\tIDEOGRAPHIC_NUMERALS\ttrue\tAdjudicated\n\
                    3.1.7\tLINE_START_PROHIBITION\tfalse\tNormative\n";
        let (inventory, problems) = parse_inventory(text);
        assert!(problems.is_empty(), "found {problems:?}");
        assert_eq!(inventory.conditional, BTreeSet::from(["3.1.3".to_owned()]));
        assert_eq!(
            inventory.by_name.get("LINE_START_PROHIBITION").cloned(),
            Some("3.1.7".to_owned())
        );
        assert_eq!(
            inventory.by_ordinal.first().cloned(),
            Some("3.1.3".to_owned())
        );
    }

    #[test]
    fn an_inventory_without_the_mark_column_is_unreadable_rather_than_empty() {
        let text = "address\tstanding\n3.1.3\tAdjudicated\n";
        let (inventory, problems) = parse_inventory(text);
        assert_eq!(problems.len(), 2, "found {problems:?}");
        assert!(
            problems
                .first()
                .is_some_and(|problem| problem.contains("direction_conditional")),
            "the column this gate reads is named: {problems:?}"
        );
        assert!(inventory.conditional.is_empty());
    }

    #[test]
    fn an_inventory_mark_that_is_neither_true_nor_false_is_refused() {
        let text = "address\tdirection_conditional\n3.1.3\tyes\n";
        let (inventory, problems) = parse_inventory(text);
        assert_eq!(problems.len(), 1, "found {problems:?}");
        assert!(inventory.conditional.is_empty(), "and it is not marked");
    }

    #[test]
    fn an_inventory_addressing_one_rule_twice_is_refused() {
        let text = "address\tdirection_conditional\n3.1.3\ttrue\n3.1.3\ttrue\n";
        let (_, problems) = parse_inventory(text);
        assert_eq!(problems.len(), 1, "found {problems:?}");
    }

    #[test]
    fn a_missing_inventory_marks_nothing_and_says_so() {
        let inventory = Inventory::default();
        assert!(!inventory.present, "and the census reports that");
        assert!(inventory.conditional.is_empty());
    }

    #[test]
    fn the_gate_holds_over_this_repository() {
        let violations = run(&[]).expect("the gate can read the workspace");
        assert!(violations.is_empty(), "found {violations:?}");
    }

    #[test]
    fn the_gate_takes_no_arguments() {
        assert!(
            run(&["--check".to_owned()]).is_err(),
            "a caller passing a flag would otherwise believe a second check had run"
        );
    }

    /// A shape assertion over the occurrence record, so the fields a violation reads are
    /// the fields the scan fills.
    #[test]
    fn an_occurrence_carries_where_and_what_and_which_item() {
        let code = "fn lower() {\n    let _ = Direction::Vertical;\n}";
        let found = occurrences(&strip(code), &variants());
        let first: Option<&Occurrence> = found.first();
        assert!(first.is_some_and(|occurrence| {
            occurrence.line == 2
                && occurrence.naming == Naming::Variant("Vertical".to_owned())
                && occurrence.item.as_deref() == Some("lower")
                && !occurrence.in_predicate
                && occurrence.row == Row::Absent
        }));
    }
}
