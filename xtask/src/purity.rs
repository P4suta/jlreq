// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `purity` gate.
//!
//! Checks the invariants around the sole public library and its conformance product:
//!
//! - `jlreq` has no normal dependencies and remains `#![no_std]` plus `alloc`;
//! - `jlreq-conformance` depends only on the library and its product/test tooling;
//! - core source names neither floating-point types nor floating-point literals;
//! - manifests match the two-node product graph in `docs/design/api-spine.md`.
//!
//! The private module-layer direction is checked separately by `xtask direction`, while
//! `just no-std` and `just wasm` compile the portability half of this guarantee. The seam
//! scanner retained below has an empty production roster and is exercised only by fixtures
//! so restoring a retired cross-crate seam cannot silently pass.

use std::collections::BTreeSet;
use std::fs;
use std::io;

use crate::shared::{self, CoreCrate, Gate};

/// The `purity` gate, as the dispatcher sees it.
pub(crate) const GATE: Gate = Gate {
    name: "purity",
    purpose: concat!(
        "every crate declares only what its row of the crate graph permits, the layout ",
        "core stays no_std and writes no floating point"
    ),
    reference: concat!(
        "docs/adr/0001-no-std-no-io-no-font-in-core.md, ",
        "docs/adr/0005-integer-layout-units.md, ",
        "docs/adr/0015-the-crate-graph-and-the-inline-line-seam.md ",
        "and the crate graph in docs/design/api-spine.md"
    ),
    run,
};

/// One row of the crate-graph table in `docs/design/api-spine.md`.
#[derive(Debug)]
struct Adjacency {
    /// The package name, as the crate's own manifest declares it.
    crate_name: &'static str,
    /// Every crate this one may declare a dependency on, and no other. The rows are in
    /// dependency order, so a row never names a crate below it in this list.
    may_depend_on: &'static [&'static str],
}

impl Adjacency {
    /// The permitted dependencies, as a violation message should read them.
    fn permitted(&self) -> String {
        if self.may_depend_on.is_empty() {
            return "nothing at all".to_owned();
        }
        self.may_depend_on
            .iter()
            .map(|name| format!("`{name}`"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// The product graph, transcribed rather than derived from the manifests it checks.
const CRATE_GRAPH: &[Adjacency] = &[
    Adjacency {
        crate_name: "jlreq",
        may_depend_on: &[],
    },
    Adjacency {
        crate_name: "jlreq-conformance",
        may_depend_on: &["jlreq", "serde_json", "harfrust", "icu_segmenter"],
    },
];

/// Crates in the graph that `shared::core_crates` does not yield, with the member path
/// `Cargo.toml` spells for each.
///
/// The conformance suite is deliberately not core — it is a `std` program that reads its own
/// case files (`docs/adr/0006`) — but it is a row of the crate graph all the same, and the
/// spine states that no crate anywhere in that table has an outside dependency, naming it
/// explicitly. Its manifest is therefore read here. The name is verified against the
/// manifest rather than assumed from the path, so this pair cannot rot into applying one
/// crate's row to another.
const NON_CORE_GRAPH_MEMBERS: &[(&str, &str)] =
    &[("jlreq-conformance", "crates/jlreq-conformance")];

/// One type in the retired cross-crate seam model, retained for parser regression tests.
#[derive(Debug)]
struct SeamType {
    /// The type's name, as `docs/design/api-spine.md` spells it.
    name: &'static str,
    /// The crate whose functions build one.
    producer: &'static str,
    /// The crate whose functions read one.
    consumer: &'static str,
    /// The milestone the spine's file map places the type in.
    ///
    /// A seam type absent at or past its own milestone is a violation, and one that is
    /// not due yet is reported without failing. That distinction is data in this table
    /// rather than an early return, because a gate commissioned to prove "the seam is
    /// connected" (ADR 0015) must not print that sentence over a type nobody wrote.
    milestone: &'static str,
}

/// Historical fixture milestone.
const REACHED: &str = "M0";

/// Historical fixture owner.
const SEAM_OWNER: &str = "jlreq-unit";

/// The crate root, as `shared::relative_name` spells it.
const LIB: &str = "lib.rs";

/// Production has no cross-crate seam: all layout layers are private modules of `jlreq`.
const SEAM: &[SeamType] = &[];

/// One core crate's sources, read once and stripped to code.
#[derive(Debug)]
struct Sources {
    /// The package name, as its own manifest declares it.
    crate_name: String,
    /// Every source file, named relative to `src`, with comments and literals removed.
    files: Vec<(String, String)>,
}

/// Check every crate and gather the findings. Takes no arguments.
fn run(_arguments: &[String]) -> io::Result<Vec<String>> {
    let core = shared::core_crates()?;
    let mut violations = Vec::new();
    let mut notes = Vec::new();
    check_crate_graph(&core, &mut violations)?;

    let sources = read_sources(&core)?;
    for each in &sources {
        check_files(each, &mut violations);
    }
    check_seam(&sources, &mut violations, &mut notes);

    // The census names what was reached, so a green run is not read as a claim about a
    // subject nobody wrote. Printed before the findings, in the order the seam is listed.
    println!(
        "purity: examined {crates} core crate(s), {files} source file(s), and {seam} \
         seam type(s)",
        crates = sources.len(),
        files = sources.iter().map(|each| each.files.len()).sum::<usize>(),
        seam = SEAM.len(),
    );
    for note in &notes {
        println!("purity: {note}");
    }
    Ok(violations)
}

/// Reject any dependency the crate graph does not permit, and any crate it does not
/// describe.
fn check_crate_graph(core: &[CoreCrate], violations: &mut Vec<String>) -> io::Result<()> {
    let root = shared::workspace_root()?;
    let mut described = BTreeSet::new();

    for each in core {
        described.insert(each.name.clone());
        let manifest = fs::read_to_string(each.directory.join("Cargo.toml"))?;
        check_row(&each.name, &manifest, violations);
    }

    for (name, member) in NON_CORE_GRAPH_MEMBERS {
        let manifest = fs::read_to_string(root.join(member).join("Cargo.toml"))?;
        if package_name(&manifest) != Some(*name) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "`{member}/Cargo.toml` does not declare the package `{name}`; the \
                     crate-graph member list in xtask/src/purity.rs has gone stale"
                ),
            ));
        }
        described.insert((*name).to_owned());
        check_row(name, &manifest, violations);
    }

    for row in CRATE_GRAPH {
        if !described.contains(row.crate_name) {
            violations.push(format!(
                "{name}: the crate graph names it but the workspace has no such crate; the \
                 table in xtask/src/purity.rs has gone stale",
                name = row.crate_name
            ));
        }
    }
    Ok(())
}

/// Reject any dependency the crate's own row of the graph does not permit.
///
/// A crate the graph does not describe is itself a violation rather than an unchecked
/// crate, so adding a workspace member without a row fails the gate instead of quietly
/// escaping it.
fn check_row(crate_name: &str, manifest: &str, violations: &mut Vec<String>) {
    let Some(row) = CRATE_GRAPH.iter().find(|row| row.crate_name == crate_name) else {
        violations.push(format!(
            "{crate_name}: is a workspace crate that the crate graph in \
             xtask/src/purity.rs does not describe; the table has gone stale (see the crate \
             graph in docs/design/api-spine.md)"
        ));
        return;
    };
    for dependency in declared_dependencies(manifest) {
        if !row.may_depend_on.contains(&dependency.as_str()) {
            violations.push(format!(
                "{crate_name}: declares `{dependency}`; its row of the crate graph permits \
                 {permitted} (ADR 0015)",
                permitted = row.permitted()
            ));
        }
    }
}

/// Read every core crate's sources once, stripped to code.
fn read_sources(core: &[CoreCrate]) -> io::Result<Vec<Sources>> {
    let mut all = Vec::new();
    for each in core {
        let directory = each.directory.join("src");
        let mut files = Vec::new();
        for source in shared::rust_sources(&directory)? {
            files.push((
                shared::relative_name(&source, &directory),
                code_only(&fs::read_to_string(&source)?),
            ));
        }
        all.push(Sources {
            crate_name: each.name.clone(),
            files,
        });
    }
    Ok(all)
}

/// Require `#![no_std]` and reject floating point in every source file of one crate.
fn check_files(sources: &Sources, violations: &mut Vec<String>) {
    let crate_name = &sources.crate_name;
    if !sources.files.iter().any(|(name, _)| name == LIB) {
        violations.push(format!("{crate_name}: has no src/lib.rs"));
    }

    for (name, code) in &sources.files {
        if name == LIB && !code.lines().any(|line| line.trim() == "#![no_std]") {
            violations.push(format!(
                "{crate_name}: {name} does not declare `#![no_std]` (ADR 0001)"
            ));
        }
        if let Some(token) = float_token(code) {
            violations.push(format!(
                "{crate_name}: {name} uses `{token}`; layout arithmetic is integer (ADR 0005)"
            ));
        }
        if let Some(literal) = float_literal(code) {
            violations.push(format!(
                "{crate_name}: {name} writes the floating-point literal `{literal}`; layout \
                 arithmetic is integer (ADR 0005)"
            ));
        }
    }
}

/// Check that every seam type is owned by `jlreq-unit` and has both of its ends.
fn check_seam(sources: &[Sources], violations: &mut Vec<String>, notes: &mut Vec<String>) {
    let code: Vec<(&str, String)> = sources
        .iter()
        .map(|each| {
            let joined = each
                .files
                .iter()
                .map(|(_, code)| code.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            (each.crate_name.as_str(), joined)
        })
        .collect();

    for seam in SEAM {
        check_seam_type(seam, &code, violations, notes);
    }
}

/// Check one seam type against the code of every core crate.
///
/// A type no crate declares is a violation once its own milestone has been reached, and a
/// note before then. The absent case is the one this check was commissioned for — a seam
/// with nothing on the far end — so it cannot be the case the gate says nothing about.
fn check_seam_type(
    seam: &SeamType,
    code: &[(&str, String)],
    violations: &mut Vec<String>,
    notes: &mut Vec<String>,
) {
    let declaring: Vec<(&str, Declared)> = code
        .iter()
        .filter_map(|(crate_name, code)| {
            declaration_of(code, seam.name).map(|kind| (*crate_name, kind))
        })
        .collect();

    if declaring.is_empty() {
        let absent = format!(
            "`{name}` is declared by no crate; the seam carries it from `{producer}` to \n             `{consumer}`",
            name = seam.name,
            producer = seam.producer,
            consumer = seam.consumer
        );
        if seam.milestone <= REACHED {
            violations.push(format!(
                "{SEAM_OWNER}: {absent}, and the API spine places it in {milestone}, which \n                 this workspace has reached (ADR 0015)",
                milestone = seam.milestone
            ));
        } else {
            notes.push(format!(
                "{absent}; it arrives at {milestone}, so neither end was examined",
                milestone = seam.milestone
            ));
        }
        return;
    }

    for (crate_name, _) in &declaring {
        if *crate_name != SEAM_OWNER {
            violations.push(format!(
                "{crate_name}: declares `{name}`; every type crossing the seam is owned by \
                 `{SEAM_OWNER}`, so that neither `{producer}` nor `{consumer}` names a type \
                 the other owns (ADR 0015)",
                name = seam.name,
                producer = seam.producer,
                consumer = seam.consumer
            ));
        }
    }

    let opaque = declaring
        .iter()
        .any(|(crate_name, kind)| *crate_name == SEAM_OWNER && *kind == Declared::Struct);
    if !opaque {
        return;
    }
    let Some((_, owning_code)) = code.iter().find(|(name, _)| *name == SEAM_OWNER) else {
        return;
    };

    let functions = inherent_public_functions(owning_code, seam.name);
    if !functions.iter().any(Signature::is_constructor) {
        violations.push(format!(
            "{SEAM_OWNER}: `{name}` has no public constructor, so its producer `{producer}` \
             cannot build one: a seam readable at one end and not writable at the other is a \
             seam with nothing on the far end (ADR 0012, ADR 0015)",
            name = seam.name,
            producer = seam.producer
        ));
    }
    if !functions.iter().any(Signature::is_accessor) {
        violations.push(format!(
            "{SEAM_OWNER}: `{name}` has no public accessor, so its consumer `{consumer}` \
             cannot read one: a seam writable at one end and not readable at the other is a \
             seam with nothing on the far end (ADR 0012, ADR 0015)",
            name = seam.name,
            consumer = seam.consumer
        ));
    }
}

/// Collect the dependency names a manifest declares.
///
/// This is a deliberate hand-rolled scan rather than a TOML parser. The tool that
/// enforces "the layout core has no outside dependencies" should not itself have any, and
/// it only ever reads manifests this repository writes in one documented style. It
/// understands both `[dependencies]` tables and `[dependencies.name]` sub-tables, and
/// treats `dev-` and `build-` dependencies and `[target.'cfg(..)'.dependencies]` the same
/// way, because a core crate should not acquire those either.
fn declared_dependencies(manifest: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut inside_dependency_table = false;

    for line in manifest.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(header) = line
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            let header = header.trim();
            if let Some(name) = dependency_subtable_name(header) {
                names.insert(name.to_owned());
                inside_dependency_table = false;
            } else {
                inside_dependency_table = is_dependency_table(header);
            }
            continue;
        }

        if inside_dependency_table {
            if let Some((key, _)) = line.split_once('=') {
                let key = key.trim().trim_matches('"');
                if !key.is_empty() {
                    names.insert(key.to_owned());
                }
            }
        }
    }

    names
}

/// Whether a table header names a dependency table.
fn is_dependency_table(header: &str) -> bool {
    ["dependencies", "dev-dependencies", "build-dependencies"]
        .iter()
        .any(|kind| header == *kind || header.ends_with(&format!(".{kind}")))
}

/// Extract `name` from a `[dependencies.name]` style header.
fn dependency_subtable_name(header: &str) -> Option<&str> {
    let (prefix, name) = header.rsplit_once('.')?;
    is_dependency_table(prefix).then(|| name.trim_matches('"'))
}

/// The package name a crate manifest declares.
///
/// A second, minimal reader: `shared` has one and does not publish it, and this gate needs
/// a package name for the one crate in the graph that `shared::core_crates` deliberately
/// does not yield.
fn package_name(manifest: &str) -> Option<&str> {
    let mut inside_package = false;
    for line in manifest.lines() {
        let line = line.trim();
        if let Some(header) = line
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            inside_package = header.trim() == "package";
            continue;
        }
        if inside_package {
            if let Some((key, value)) = line.split_once('=') {
                if key.trim() == "name" {
                    return value.split('"').nth(1);
                }
            }
        }
    }
    None
}

/// Find the first floating-point type named in `code`, as a whole token.
fn float_token(code: &str) -> Option<&'static str> {
    code.split(|character: char| !character.is_alphanumeric() && character != '_')
        .find_map(|token| match token {
            "f32" => Some("f32"),
            "f64" => Some("f64"),
            _ => None,
        })
}

/// Find the first floating-point literal written in `code`.
///
/// The other half of ADR 0005, and the half no other tool holds: `let r = 0.5; r * 2.0;`
/// names neither `f32` nor `f64`, so inference alone makes it floating point and every
/// name-based check passes it. Reads the literal grammar rather than searching for a dot,
/// so `self.0` is a tuple field, `0..10` is a range, `0x1f` is an integer, and `1e5` and
/// `1f32` are floats even though neither writes one.
fn float_literal(code: &str) -> Option<String> {
    let text: Vec<char> = code.chars().collect();
    let mut index = 0;
    while let Some(&character) = text.get(index) {
        if !character.is_ascii_digit() {
            index = index.saturating_add(1);
            continue;
        }
        if !starts_a_literal(&text, index) {
            index = run_end(&text, index, |character| {
                character.is_alphanumeric() || character == '_'
            });
            continue;
        }
        let (end, is_float) = numeric_literal(&text, index);
        if is_float {
            return text.get(index..end).map(|literal| literal.iter().collect());
        }
        index = end;
    }
    None
}

/// Whether the digit at `index` starts a numeric literal.
///
/// A digit inside an identifier does not, and neither does a tuple field: `self.0` names a
/// field and `value.0.1` names two. A digit after `..` does, because `0..1.5` is a range
/// whose end is a float.
fn starts_a_literal(text: &[char], index: usize) -> bool {
    let Some(before) = index.checked_sub(1) else {
        return true;
    };
    match text.get(before) {
        Some(&previous) if previous.is_alphanumeric() || previous == '_' => false,
        Some('.') => matches!(
            before.checked_sub(1).and_then(|earlier| text.get(earlier)),
            Some('.')
        ),
        None | Some(_) => true,
    }
}

/// The end of the numeric literal starting at `start`, and whether it is a float.
///
/// Follows the Rust reference: a fraction whose dot is followed by neither another dot nor
/// an identifier, an exponent with or without a fraction, and the `f32` and `f64` suffixes,
/// which make a literal floating point without writing a dot.
fn numeric_literal(text: &[char], start: usize) -> (usize, bool) {
    let after_prefix = start.saturating_add(2);
    if text.get(start) == Some(&'0')
        && matches!(text.get(start.saturating_add(1)), Some('x' | 'o' | 'b'))
    {
        return (
            run_end(text, after_prefix, |character| {
                character.is_ascii_alphanumeric() || character == '_'
            }),
            false,
        );
    }

    let mut end = run_end(text, start, is_digit_part);
    let mut float = false;

    if text.get(end) == Some(&'.') {
        let next = text.get(end.saturating_add(1)).copied();
        let continues_something_else =
            matches!(next, Some('.' | '_')) || next.is_some_and(char::is_alphabetic);
        if !continues_something_else {
            float = true;
            end = run_end(text, end.saturating_add(1), is_digit_part);
        }
    }

    if matches!(text.get(end), Some('e' | 'E')) {
        let signed = matches!(text.get(end.saturating_add(1)), Some('+' | '-'));
        let digits = if signed {
            end.saturating_add(2)
        } else {
            end.saturating_add(1)
        };
        if text.get(digits).is_some_and(char::is_ascii_digit) {
            float = true;
            end = run_end(text, digits, is_digit_part);
        }
    }

    let suffix_end = run_end(text, end, |character| {
        character.is_ascii_alphanumeric() || character == '_'
    });
    let suffix: String = text
        .get(end..suffix_end)
        .unwrap_or_default()
        .iter()
        .collect();
    (suffix_end, float || suffix == "f32" || suffix == "f64")
}

/// Whether a character continues the digits of a numeric literal.
fn is_digit_part(character: char) -> bool {
    character.is_ascii_digit() || character == '_'
}

/// The index just past the run of characters satisfying `keep`, starting at `from`.
fn run_end(text: &[char], from: usize, keep: impl Fn(char) -> bool) -> usize {
    let mut end = from;
    while text.get(end).is_some_and(|&character| keep(character)) {
        end = end.saturating_add(1);
    }
    end
}

/// Everything a source says as code, with what the shared reader cannot see removed first.
///
/// `shared::code_only` drops `//` comments and says plainly that it tracks neither string
/// literals nor block comments. That approximation is adequate for a scan looking for type
/// *names*; it is not adequate for one looking for *literals*, where a `{:.3}` in a format
/// string is a precision, a `0.5` in a block comment is prose, and a `//` inside a string
/// literal is not the start of a comment. So the text it cannot see is blanked here first,
/// and it then removes line comments from a source in which no `//` can be quoted.
fn code_only(source: &str) -> String {
    shared::code_only(&without_hidden_text(source))
}

/// The source with every block comment and every string, byte-string and character literal
/// replaced by blanks.
///
/// Blanks rather than deletions, and newlines are kept, so that the line structure the
/// `#![no_std]` check reads survives and two tokens either side of a comment do not become
/// one. Line comments are left as they are for the shared reader to remove; they are
/// recognized here only so that a quote inside one cannot open a string.
fn without_hidden_text(source: &str) -> String {
    let text: Vec<char> = source.chars().collect();
    let mut kept = String::with_capacity(source.len());
    let mut index = 0;
    while let Some(&character) = text.get(index) {
        if let Some(end) = line_comment_end(&text, index) {
            kept.extend(text.get(index..end).unwrap_or_default());
            index = end;
        } else if let Some(end) = hidden_end(&text, index) {
            blank(&text, index, end, &mut kept);
            index = end;
        } else {
            kept.push(character);
            index = index.saturating_add(1);
        }
    }
    kept
}

/// Append `text[from..to]` as blanks, keeping the newlines.
fn blank(text: &[char], from: usize, to: usize, kept: &mut String) {
    for &character in text.get(from..to).unwrap_or_default() {
        kept.push(if character == '\n' { '\n' } else { ' ' });
    }
}

/// The index just past the non-code text starting at `index`, if any starts there.
fn hidden_end(text: &[char], index: usize) -> Option<usize> {
    block_comment_end(text, index)
        .or_else(|| raw_string_end(text, index))
        .or_else(|| string_end(text, index))
        .or_else(|| character_end(text, index))
}

/// The index just past a `//` comment, which ends at the newline it does not include.
fn line_comment_end(text: &[char], index: usize) -> Option<usize> {
    (text.get(index) == Some(&'/') && text.get(index.saturating_add(1)) == Some(&'/'))
        .then(|| run_end(text, index, |character| character != '\n'))
}

/// The index just past a `/* */` comment, which Rust allows to nest.
fn block_comment_end(text: &[char], index: usize) -> Option<usize> {
    if text.get(index) != Some(&'/') || text.get(index.saturating_add(1)) != Some(&'*') {
        return None;
    }
    let mut depth = 0_usize;
    let mut cursor = index;
    while let Some(&character) = text.get(cursor) {
        let next = text.get(cursor.saturating_add(1));
        if character == '/' && next == Some(&'*') {
            depth = depth.saturating_add(1);
            cursor = cursor.saturating_add(2);
        } else if character == '*' && next == Some(&'/') {
            depth = depth.saturating_sub(1);
            cursor = cursor.saturating_add(2);
            if depth == 0 {
                return Some(cursor);
            }
        } else {
            cursor = cursor.saturating_add(1);
        }
    }
    Some(text.len())
}

/// The index just past a `"..."` literal, escapes included.
fn string_end(text: &[char], index: usize) -> Option<usize> {
    if text.get(index) != Some(&'"') {
        return None;
    }
    let mut cursor = index.saturating_add(1);
    while let Some(&character) = text.get(cursor) {
        match character {
            '\\' => cursor = cursor.saturating_add(2),
            '"' => return Some(cursor.saturating_add(1)),
            _ => cursor = cursor.saturating_add(1),
        }
    }
    Some(text.len())
}

/// The index just past an `r"..."`, `r#"..."#` or `br#"..."#` literal, which honors no
/// escapes and ends only at a quote followed by its own number of hashes.
fn raw_string_end(text: &[char], index: usize) -> Option<usize> {
    if index
        .checked_sub(1)
        .and_then(|before| text.get(before))
        .is_some_and(|&previous| previous.is_alphanumeric() || previous == '_')
    {
        return None;
    }
    let after_marker = match (text.get(index), text.get(index.saturating_add(1))) {
        (Some('r'), _) => index.saturating_add(1),
        (Some('b'), Some('r')) => index.saturating_add(2),
        _ => return None,
    };
    let quote = run_end(text, after_marker, |character| character == '#');
    if text.get(quote) != Some(&'"') {
        return None;
    }
    let hashes = quote.checked_sub(after_marker)?;

    let mut cursor = quote.saturating_add(1);
    while let Some(&character) = text.get(cursor) {
        if character == '"' {
            let after_quote = cursor.saturating_add(1);
            let closed = run_end(text, after_quote, |each| each == '#');
            if closed
                .checked_sub(after_quote)
                .is_some_and(|count| count >= hashes)
            {
                return Some(after_quote.saturating_add(hashes));
            }
        }
        cursor = cursor.saturating_add(1);
    }
    Some(text.len())
}

/// The index just past a `'x'` literal, or `None` when the quote opens a lifetime.
///
/// The two are told apart the way the language tells them apart: `'\n'` and `'a'` close,
/// and `'a` in `&'a str` does not.
fn character_end(text: &[char], index: usize) -> Option<usize> {
    if text.get(index) != Some(&'\'') {
        return None;
    }
    if text.get(index.saturating_add(1)) == Some(&'\\') {
        let mut cursor = index.saturating_add(2);
        while let Some(&character) = text.get(cursor) {
            match character {
                '\\' => cursor = cursor.saturating_add(2),
                '\'' => return Some(cursor.saturating_add(1)),
                _ => cursor = cursor.saturating_add(1),
            }
        }
        return Some(text.len());
    }
    (text.get(index.saturating_add(2)) == Some(&'\'')).then(|| index.saturating_add(3))
}

/// What kind of item a public declaration introduces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Declared {
    /// A `pub struct`, whose body this repository keeps private.
    Struct,
    /// A `pub enum`, whose public variants are its own two ends.
    Enum,
}

/// How `name` is declared in `code`, if it is declared publicly there.
fn declaration_of(code: &str, name: &str) -> Option<Declared> {
    if declares(code, "pub struct", name) {
        return Some(Declared::Struct);
    }
    declares(code, "pub enum", name).then_some(Declared::Enum)
}

/// Whether `code` introduces `name` with `keyword`.
fn declares(code: &str, keyword: &str, name: &str) -> bool {
    code.match_indices(keyword).any(|(index, _)| {
        starts_a_word(code, index)
            && after(code, index, keyword).is_some_and(|rest| identifier_at(rest) == Some(name))
    })
}

/// Whether the byte at `index` begins a word rather than continuing one.
fn starts_a_word(code: &str, index: usize) -> bool {
    !code
        .get(..index)
        .and_then(|before| before.chars().next_back())
        .is_some_and(|previous| previous.is_alphanumeric() || previous == '_')
}

/// The text after `keyword` at `index`, when a word ends there.
fn after<'a>(code: &'a str, index: usize, keyword: &str) -> Option<&'a str> {
    let rest = code.get(index.saturating_add(keyword.len())..)?;
    rest.starts_with(|character: char| character.is_whitespace() || character == '<')
        .then_some(rest)
}

/// The identifier at the start of `text`, after any spacing.
fn identifier_at(text: &str) -> Option<&str> {
    let text = text.trim_start();
    let end = text
        .char_indices()
        .find(|(_, character)| !character.is_alphanumeric() && *character != '_')
        .map_or(text.len(), |(index, _)| index);
    if end == 0 { None } else { text.get(..end) }
}

/// `text` with a leading `keyword` and the spacing after it removed.
fn strip_keyword<'a>(text: &'a str, keyword: &str) -> Option<&'a str> {
    let rest = text.strip_prefix(keyword)?;
    rest.starts_with(|character: char| character.is_whitespace())
        .then(|| rest.trim_start())
}

/// One public function's signature, as far as this gate reads one.
#[derive(Debug)]
struct Signature<'a> {
    /// The parameter list, without its parentheses.
    parameters: &'a str,
    /// The return type, or `None` when the function returns `()`.
    returns: Option<&'a str>,
}

impl Signature<'_> {
    /// Whether this is a named constructor, under the definition the API spine pins: an
    /// associated function returning `Self`, `Result<Self, _>` or `Option<Self>`.
    fn is_constructor(&self) -> bool {
        let Some(returns) = self.returns else {
            return false;
        };
        let compact: String = returns
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect();
        compact == "Self" || compact == "Option<Self>" || compact.starts_with("Result<Self,")
    }

    /// Whether this reads the value it is called on: a method that answers something other
    /// than another one of itself.
    fn is_accessor(&self) -> bool {
        self.returns.is_some() && self.has_receiver() && !self.is_constructor()
    }

    /// Whether the first parameter is a receiver.
    fn has_receiver(&self) -> bool {
        let first = self.parameters.split(',').next().unwrap_or_default().trim();
        let first = first.strip_prefix('&').map_or(first, str::trim_start);
        let first = strip_lifetime(first);
        let first = strip_keyword(first, "mut").unwrap_or(first);
        identifier_at(first) == Some("self")
    }
}

/// `text` with a leading `'lifetime` and the spacing after it removed, if it has one.
fn strip_lifetime(text: &str) -> &str {
    let Some(rest) = text.strip_prefix('\'') else {
        return text;
    };
    let Some(name) = identifier_at(rest) else {
        return text;
    };
    rest.get(name.len()..).map_or(text, str::trim_start)
}

/// Every public function declared in an inherent `impl` block for `type_name`.
fn inherent_public_functions<'a>(code: &'a str, type_name: &str) -> Vec<Signature<'a>> {
    let mut functions = Vec::new();
    for body in inherent_impl_bodies(code, type_name) {
        functions.extend(public_functions(body));
    }
    functions
}

/// The body of every inherent `impl` block for `type_name`.
///
/// Trait implementations are not inherent and declare nothing public of their own, so an
/// `impl <Trait> for <Type>` is skipped.
fn inherent_impl_bodies<'a>(code: &'a str, type_name: &str) -> Vec<&'a str> {
    let mut bodies = Vec::new();
    for (index, _) in code.match_indices("impl") {
        if !starts_a_word(code, index) {
            continue;
        }
        let Some(rest) = after(code, index, "impl") else {
            continue;
        };
        let Some(open) = rest.find('{') else {
            continue;
        };
        let Some(header) = rest.get(..open) else {
            continue;
        };
        if header.contains(" for ") || impl_self_type(header) != Some(type_name) {
            continue;
        }
        if let Some(body) = rest
            .get(open.saturating_add(1)..)
            .and_then(balanced_brace_body)
        {
            bodies.push(body);
        }
    }
    bodies
}

/// The name of the type an `impl` header implements.
fn impl_self_type(header: &str) -> Option<&str> {
    identifier_at(skip_generics(header.trim_start())?)
}

/// `text` with a leading balanced `<...>` removed, if it has one.
///
/// A `>` closing a `->` inside the generics closes nothing, which is why the previous
/// character is looked at.
fn skip_generics(text: &str) -> Option<&str> {
    if !text.starts_with('<') {
        return Some(text);
    }
    let mut depth = 0_usize;
    let mut previous = ' ';
    for (index, character) in text.char_indices() {
        match character {
            '<' => depth = depth.saturating_add(1),
            '>' if previous != '-' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return text.get(index.saturating_add(1)..);
                }
            },
            _ => {},
        }
        previous = character;
    }
    None
}

/// The text a `{` opens, up to its matching `}`.
fn balanced_brace_body(text: &str) -> Option<&str> {
    let mut depth = 1_usize;
    for (index, character) in text.char_indices() {
        match character {
            '{' => depth = depth.saturating_add(1),
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return text.get(..index);
                }
            },
            _ => {},
        }
    }
    None
}

/// Every public function declared directly in an `impl` body.
fn public_functions(body: &str) -> Vec<Signature<'_>> {
    let mut found = Vec::new();
    for (index, _) in body.match_indices("pub") {
        if !starts_a_word(body, index) {
            continue;
        }
        let Some(rest) = body.get(index.saturating_add("pub".len())..) else {
            continue;
        };
        if let Some(signature) = after_fn_keyword(rest).and_then(signature_of) {
            found.push(signature);
        }
    }
    found
}

/// Modifiers that may sit between `pub` and `fn`.
const FUNCTION_MODIFIERS: &[&str] = &["const", "unsafe", "async", "extern", "default"];

/// The text just after the `fn` keyword of a public function whose `pub` ends where `text`
/// begins.
///
/// `pub(crate)` is not public and `public` is not `pub`, so both answer `None`.
fn after_fn_keyword(text: &str) -> Option<&str> {
    if text.starts_with(|character: char| {
        character.is_alphanumeric() || character == '_' || character == '('
    }) {
        return None;
    }
    let mut rest = text.trim_start();
    for _ in 0..FUNCTION_MODIFIERS.len() {
        if let Some(after) = strip_keyword(rest, "fn") {
            return Some(after);
        }
        let stripped = FUNCTION_MODIFIERS
            .iter()
            .find_map(|modifier| strip_keyword(rest, modifier))?;
        rest = stripped;
    }
    strip_keyword(rest, "fn")
}

/// Read a signature from the text just after a `fn` keyword.
fn signature_of(text: &str) -> Option<Signature<'_>> {
    let open = text.find('(')?;
    let inside = text.get(open.saturating_add(1)..)?;
    let close = balanced_paren_end(inside)?;
    let parameters = inside.get(..close)?;
    let tail = inside.get(close.saturating_add(1)..)?;
    let tail = tail
        .find(['{', ';'])
        .and_then(|end| tail.get(..end))
        .unwrap_or(tail);
    let returns = tail
        .split_once("->")
        .map(|(_, returned)| returned.split(" where ").next().unwrap_or(returned).trim());
    Some(Signature {
        parameters,
        returns,
    })
}

/// The offset of the `)` closing a `(` that opened just before `text`.
fn balanced_paren_end(text: &str) -> Option<usize> {
    let mut depth = 1_usize;
    for (index, character) in text.char_indices() {
        match character {
            '(' => depth = depth.saturating_add(1),
            ')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(index);
                }
            },
            _ => {},
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        CRATE_GRAPH, SEAM, SEAM_OWNER, SeamType, check_row, check_seam_type, code_only,
        declaration_of, declared_dependencies, dependency_subtable_name, float_literal,
        float_token, is_dependency_table, package_name,
    };

    /// One crate's fixture code, in the shape the seam check reads.
    fn workspace<'a>(crate_name: &'a str, code: &str) -> Vec<(&'a str, String)> {
        vec![(crate_name, code.to_owned())]
    }

    /// The seam type of that name, so a test names what it is about.
    fn seam(name: &'static str) -> SeamType {
        SeamType {
            name,
            producer: "fixture-producer",
            consumer: "fixture-consumer",
            milestone: "M0",
        }
    }

    #[test]
    fn reads_dependencies_from_a_plain_table() {
        let manifest = "[package]\nname = \"x\"\n\n[dependencies]\nserde = \"1\"\nlog = { version = \"0.4\" }\n";
        let names = declared_dependencies(manifest);
        assert!(names.contains("serde"));
        assert!(names.contains("log"));
        assert!(!names.contains("name"), "package keys are not dependencies");
    }

    #[test]
    fn reads_dependencies_from_a_subtable_header() {
        let names = declared_dependencies("[dependencies.serde]\nversion = \"1\"\n");
        assert!(names.contains("serde"));
        assert!(
            !names.contains("version"),
            "keys inside a dependency subtable describe it, they are not further dependencies"
        );
    }

    #[test]
    fn treats_dev_build_and_target_dependencies_as_dependencies() {
        assert!(is_dependency_table("dev-dependencies"));
        assert!(is_dependency_table("build-dependencies"));
        assert!(is_dependency_table("target.'cfg(windows)'.dependencies"));
        assert!(!is_dependency_table("package"));
        assert!(!is_dependency_table("lints.clippy"));
    }

    #[test]
    fn names_a_dependency_subtable_only_under_a_dependency_table() {
        assert_eq!(
            dependency_subtable_name("dependencies.serde"),
            Some("serde")
        );
        assert_eq!(
            dependency_subtable_name("target.'cfg(windows)'.dependencies"),
            None,
            "this is a dependency table, not a single dependency"
        );
        assert_eq!(dependency_subtable_name("workspace.package"), None);
    }

    #[test]
    fn an_empty_manifest_declares_nothing() {
        assert!(declared_dependencies("").is_empty());
        assert!(declared_dependencies("[dependencies]\n").is_empty());
    }

    #[test]
    fn finds_floating_point_types_as_whole_tokens() {
        assert_eq!(float_token("let advance: f32 = 0;"), Some("f32"));
        assert_eq!(float_token("value as f64"), Some("f64"));
        assert_eq!(float_token("let buf32 = 0; let f6 = 0;"), None);
        assert_eq!(float_token("struct Cf32;"), None);
    }

    #[test]
    fn prose_mentioning_a_forbidden_token_is_not_a_violation() {
        let source = "//! Callers converting from an f64 pipeline convert once.\nlet em: i32 = 0;";
        assert_eq!(float_token(&code_only(source)), None);
    }

    #[test]
    fn code_before_a_trailing_comment_is_still_checked() {
        let source = "let advance: f64 = 0; // measured upstream";
        assert_eq!(float_token(&code_only(source)), Some("f64"));
    }

    #[test]
    fn the_no_std_attribute_is_recognized_only_as_a_whole_line() {
        assert!(
            code_only("#![no_std]\n")
                .lines()
                .any(|line| line.trim() == "#![no_std]")
        );
        assert!(
            !code_only("//! mentions #![no_std] in prose\n")
                .lines()
                .any(|line| line.trim() == "#![no_std]"),
            "a doc comment describing the attribute does not apply it"
        );
        assert!(
            !code_only("/*\n#![no_std]\n*/\n")
                .lines()
                .any(|line| line.trim() == "#![no_std]"),
            "a block comment quoting the attribute does not apply it either"
        );
    }

    #[test]
    fn a_crate_may_declare_only_what_its_row_permits() {
        let mut violations = Vec::new();
        check_row("jlreq", "[dependencies]\n", &mut violations);
        assert!(violations.is_empty(), "found {violations:?}");
    }

    #[test]
    fn the_unified_products_have_explicit_dependency_rows() {
        let library = CRATE_GRAPH
            .iter()
            .find(|row| row.crate_name == "jlreq")
            .expect("the unified public library has a graph row");
        assert!(
            library.may_depend_on.is_empty(),
            "the no_std public library has no external dependencies"
        );

        let runner = CRATE_GRAPH
            .iter()
            .find(|row| row.crate_name == "jlreq-conformance")
            .expect("the binary-only conformance product has a graph row");
        assert!(runner.may_depend_on.contains(&"jlreq"));
        assert!(runner.may_depend_on.contains(&"serde_json"));
        assert!(runner.may_depend_on.contains(&"harfrust"));
        assert!(runner.may_depend_on.contains(&"icu_segmenter"));
    }

    #[test]
    fn a_dependency_the_row_omits_is_a_violation_even_when_it_is_core() {
        let mut violations = Vec::new();
        check_row("jlreq", "[dependencies]\nserde = \"1\"\n", &mut violations);
        assert_eq!(violations.len(), 1, "found {violations:?}");
        assert!(
            violations[0].contains("serde"),
            "the message names the edge: {violations:?}"
        );
    }

    #[test]
    fn neither_seam_crate_may_reach_the_other() {
        let mut violations = Vec::new();
        check_row(
            "jlreq-inline",
            "[dependencies]\njlreq-line = \"0\"\n",
            &mut violations,
        );
        check_row(
            "jlreq-line",
            "[dev-dependencies]\njlreq-inline = \"0\"\n",
            &mut violations,
        );
        assert_eq!(
            violations.len(),
            2,
            "§3.4.3 forbids one edge and §3.3.8 rule 3 the other: {violations:?}"
        );
    }

    #[test]
    fn a_crate_the_graph_does_not_describe_is_a_violation() {
        let mut violations = Vec::new();
        check_row("jlreq-page", "[dependencies]\n", &mut violations);
        assert_eq!(violations.len(), 1, "found {violations:?}");
        assert!(
            violations[0].contains("stale"),
            "a member without a row means the table rotted: {violations:?}"
        );
    }

    #[test]
    fn the_crate_graph_is_ordered_and_closed() {
        let mut earlier = Vec::new();
        for row in CRATE_GRAPH {
            for permitted in row.may_depend_on {
                if !CRATE_GRAPH
                    .iter()
                    .any(|candidate| candidate.crate_name == *permitted)
                {
                    continue;
                }
                assert!(
                    earlier.contains(permitted),
                    "{crate_name} may depend on {permitted}, which is not a crate the graph \
                     already described; the graph must stay acyclic and in dependency order",
                    crate_name = row.crate_name
                );
            }
            earlier.push(row.crate_name);
        }
    }

    #[test]
    fn the_seam_is_owned_by_one_crate_and_named_by_two() {
        for seam in SEAM {
            assert_ne!(seam.producer, seam.consumer, "a seam has two ends");
            for end in [seam.producer, seam.consumer] {
                assert!(
                    CRATE_GRAPH.iter().any(|row| row.crate_name == end),
                    "{end} is not a crate the graph describes"
                );
                assert_ne!(
                    end, SEAM_OWNER,
                    "the owner of a seam type is not an end of it"
                );
            }
        }
    }

    #[test]
    fn the_retired_inter_crate_seam_roster_is_empty() {
        let mut violations = Vec::new();
        let mut notes = Vec::new();
        let code = workspace(SEAM_OWNER, "pub struct Something;");
        for seam in SEAM {
            check_seam_type(seam, &code, &mut violations, &mut notes);
        }
        assert_eq!(
            violations.len().saturating_add(notes.len()),
            SEAM.len(),
            "the absent case is the one this check exists for, so it is never silent:              {violations:?} {notes:?}"
        );
        assert!(SEAM.is_empty());
        assert!(violations.is_empty());
        assert!(notes.is_empty());
    }

    #[test]
    fn a_seam_type_not_due_yet_is_reported_without_failing() {
        let later = SeamType {
            name: "Ladder",
            producer: "jlreq-inline",
            consumer: "jlreq-line",
            milestone: "M9",
        };
        let mut violations = Vec::new();
        let mut notes = Vec::new();
        check_seam_type(
            &later,
            &workspace(SEAM_OWNER, "pub struct Something;"),
            &mut violations,
            &mut notes,
        );
        assert!(violations.is_empty(), "found {violations:?}");
        assert_eq!(notes.len(), 1, "found {notes:?}");
        assert!(notes[0].contains("M9"), "found {notes:?}");
    }

    #[test]
    fn a_seam_type_with_both_ends_is_accepted() {
        let source = "pub struct Separation { after: u32 }\n\
                      impl Separation {\n\
                      pub const fn new(after: u32) -> Self { Self { after } }\n\
                      pub const fn after(self) -> u32 { self.after }\n\
                      }\n";
        let mut violations = Vec::new();
        check_seam_type(
            &seam("Separation"),
            &workspace(SEAM_OWNER, source),
            &mut violations,
            &mut Vec::new(),
        );
        assert!(violations.is_empty(), "found {violations:?}");
    }

    #[test]
    fn a_seam_type_that_cannot_be_read_is_a_violation() {
        let source = "pub struct Separation { after: u32 }\n\
                      impl Separation {\n\
                      pub const fn new(after: u32) -> Self { Self { after } }\n\
                      const fn after(self) -> u32 { self.after }\n\
                      }\n";
        let mut violations = Vec::new();
        check_seam_type(
            &seam("Separation"),
            &workspace(SEAM_OWNER, source),
            &mut violations,
            &mut Vec::new(),
        );
        assert_eq!(violations.len(), 1, "found {violations:?}");
        assert!(
            violations[0].contains("no public accessor"),
            "found {violations:?}"
        );
    }

    #[test]
    fn a_seam_type_that_cannot_be_built_is_a_violation() {
        let source = "pub struct Separation { after: u32 }\n\
                      impl Separation {\n\
                      pub const fn after(self) -> u32 { self.after }\n\
                      }\n";
        let mut violations = Vec::new();
        check_seam_type(
            &seam("Separation"),
            &workspace(SEAM_OWNER, source),
            &mut violations,
            &mut Vec::new(),
        );
        assert_eq!(violations.len(), 1, "found {violations:?}");
        assert!(
            violations[0].contains("no public constructor"),
            "found {violations:?}"
        );
    }

    #[test]
    fn a_seam_type_declared_by_an_end_is_a_violation() {
        let source = "pub struct Segment { items: u32 }\n";
        let mut violations = Vec::new();
        check_seam_type(
            &seam("Segment"),
            &workspace("jlreq-line", source),
            &mut violations,
            &mut Vec::new(),
        );
        assert_eq!(violations.len(), 1, "found {violations:?}");
        assert!(
            violations[0].contains(SEAM_OWNER),
            "the message names the crate that owns it: {violations:?}"
        );
    }

    #[test]
    fn a_trait_implementation_supplies_neither_end() {
        let source = "pub struct Separation;\n\
                      impl Default for Separation {\n\
                      fn default() -> Self { Self }\n\
                      }\n";
        let mut violations = Vec::new();
        check_seam_type(
            &seam("Separation"),
            &workspace(SEAM_OWNER, source),
            &mut violations,
            &mut Vec::new(),
        );
        assert_eq!(
            violations.len(),
            2,
            "a trait impl declares nothing public of its own: {violations:?}"
        );
    }

    #[test]
    fn an_enum_is_its_own_two_ends() {
        let source = "pub enum Straddle { Forbidden, Permitted }\n";
        let mut violations = Vec::new();
        check_seam_type(
            &seam("Straddle"),
            &workspace(SEAM_OWNER, source),
            &mut violations,
            &mut Vec::new(),
        );
        assert!(
            violations.is_empty(),
            "public variants are readable and writable by construction: {violations:?}"
        );
    }

    #[test]
    fn a_declaration_is_read_as_a_whole_word() {
        assert!(declaration_of("pub struct Runs<'a> {}", "Runs").is_some());
        assert!(declaration_of("pub enum Straddle {}", "Straddle").is_some());
        assert_eq!(
            declaration_of("pub struct RunsError;", "Runs"),
            None,
            "a longer name is a different type"
        );
        assert_eq!(
            declaration_of("pub(crate) struct Runs;", "Runs"),
            None,
            "a crate-visible type is not a seam"
        );
    }

    #[test]
    fn finds_a_bare_floating_point_literal() {
        assert_eq!(
            float_literal("let ratio = 0.5;"),
            Some("0.5".to_owned()),
            "the measured hole: this names neither f32 nor f64"
        );
        assert_eq!(
            float_literal("let doubled = r * 2.0;"),
            Some("2.0".to_owned())
        );
        assert_eq!(float_literal("let big = 1e5;"), Some("1e5".to_owned()));
        assert_eq!(
            float_literal("let small = 2.5e-3;"),
            Some("2.5e-3".to_owned())
        );
        assert_eq!(
            float_literal("let typed = 1f32;"),
            Some("1f32".to_owned()),
            "a suffix makes a literal floating point without writing a dot"
        );
        assert_eq!(float_literal("let truncated = 1.;"), Some("1.".to_owned()));
    }

    #[test]
    fn a_tuple_field_access_is_not_a_literal() {
        assert_eq!(float_literal("self.0"), None);
        assert_eq!(float_literal("let units = self.0;"), None);
        assert_eq!(
            float_literal("let inner = value.0.1;"),
            None,
            "a tuple inside a tuple is two field accesses, not a fraction"
        );
        assert_eq!(
            float_literal("let end = 0..1.5;"),
            Some("1.5".to_owned()),
            "but a range end really is a literal, and the two dots say so"
        );
    }

    #[test]
    fn a_format_precision_is_not_a_literal() {
        assert_eq!(
            float_literal(&code_only("write!(f, \"{:.3}\", value)")),
            None,
            "a precision inside a format string is not a number in the code"
        );
        assert_eq!(
            float_literal(&code_only("let text = \"0.5\";")),
            None,
            "a string literal is text"
        );
        assert_eq!(
            float_literal(&code_only("let raw = r#\"0.5\"#;")),
            None,
            "a raw string literal is text too"
        );
    }

    #[test]
    fn a_comment_is_not_a_literal() {
        assert_eq!(
            float_literal(&code_only("// half is 0.5\nlet half = 1;")),
            None
        );
        assert_eq!(float_literal(&code_only("/* 0.5 */ let half = 1;")), None);
        assert_eq!(
            float_literal(&code_only("/* /* 0.5 */ */ let half = 1;")),
            None,
            "block comments nest in Rust"
        );
        assert_eq!(
            float_literal(&code_only("/// The ratio 0.5.\npub const HALF: u8 = 1;")),
            None
        );
    }

    #[test]
    fn an_integer_is_not_a_literal_this_gate_objects_to() {
        assert_eq!(float_literal("for index in 0..10 {}"), None);
        assert_eq!(float_literal("let mask = 0x1f;"), None);
        assert_eq!(float_literal("let bits = 0b1010_1010;"), None);
        assert_eq!(float_literal("let big = 1_000_000;"), None);
        assert_eq!(float_literal("let range = 0..=9;"), None);
        assert_eq!(float_literal("let units = i32::from(2);"), None);
    }

    #[test]
    fn a_lifetime_does_not_open_a_character_literal() {
        assert_eq!(
            float_literal(&code_only("fn of<'a>(text: &'a str) { let ratio = 0.5; }")),
            Some("0.5".to_owned()),
            "reading `'a` as a character literal would have hidden everything after it"
        );
        assert_eq!(
            float_literal(&code_only("let dot = '.'; let quote = '\\'';")),
            None
        );
    }

    #[test]
    fn a_comment_marker_inside_a_string_does_not_end_the_line() {
        assert_eq!(
            float_literal(&code_only(
                "let url = \"https://example.test\";\nlet r = 0.5;"
            )),
            Some("0.5".to_owned()),
            "cutting the line at the first `//` would have hidden the literal below it"
        );
    }

    #[test]
    fn reads_a_package_name_only_from_the_package_table() {
        assert_eq!(
            package_name("[package]\nname = \"jlreq-conform\"\n"),
            Some("jlreq-conform")
        );
        assert_eq!(package_name("[lints]\nname = \"decoy\"\n"), None);
        assert_eq!(package_name(""), None);
    }
}
