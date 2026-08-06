// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! What every task shares: where the repository is, how to read it, and how a finding is
//! reported.
//!
//! A gate is four things — the name it is invoked by, the sentence its holding justifies,
//! the document to read when it does not hold, and the check itself. Reporting is written
//! once here so that every task speaks with the same voice and so that adding a task is
//! writing a module and one line in the dispatcher's table.
//!
//! The list of core crates is *derived* rather than kept. `Cargo.toml` already names the
//! workspace members; this module subtracts the members that are deliberately not core.
//! A crate added to the workspace and forgotten here is therefore checked as core and
//! fails the gate, where a hand-maintained list would have skipped it in silence.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Workspace members that are not part of the layout core.
///
/// A denylist rather than an allowlist, because the two fail in opposite directions and
/// only one of them fails safely. Each entry is a member path exactly as `Cargo.toml`
/// spells it, and an entry naming something that is no longer a member is an error rather
/// than a no-op, so this list cannot rot unnoticed.
const NON_CORE_MEMBERS: &[&str] = &[
    // The conformance suite is a std program that reads its own case files (ADR 0006).
    "crates/jlreq-conform",
    // The repository's own tooling, which is this program.
    "xtask",
];

/// One repository gate: a subcommand of `xtask`.
#[derive(Debug)]
pub(crate) struct Gate {
    /// The name the gate is invoked by, after `cargo run -p xtask --`.
    pub(crate) name: &'static str,
    /// What holding this gate means, in one line. Printed when the check finds nothing,
    /// and listed in the usage message, so it states what was actually checked.
    pub(crate) purpose: &'static str,
    /// Where to read about the invariant. Printed after the violations when it fails.
    pub(crate) reference: &'static str,
    /// The check itself. It receives the arguments following the subcommand and returns
    /// one message per violation, or an error when it could not run at all.
    pub(crate) run: fn(&[String]) -> io::Result<Vec<String>>,
}

impl Gate {
    /// Run the gate and turn its findings into output and an exit code.
    ///
    /// A gate that cannot run is a failure, not a pass: an unreadable manifest tells us
    /// nothing about the invariant, and reporting success there would be the one failure
    /// mode a policy check must not have.
    pub(crate) fn report(&self, arguments: &[String]) -> ExitCode {
        let Self {
            name,
            purpose,
            reference,
            run,
        } = self;
        match run(arguments) {
            Err(error) => {
                eprintln!("xtask: {name} could not run: {error}");
                ExitCode::FAILURE
            },
            Ok(violations) if violations.is_empty() => {
                println!("{name}: {purpose}");
                ExitCode::SUCCESS
            },
            Ok(violations) => {
                for violation in &violations {
                    eprintln!("{name}: {violation}");
                }
                eprintln!(
                    "{name}: {count} violation(s). See {reference}",
                    count = violations.len()
                );
                ExitCode::FAILURE
            },
        }
    }
}

/// A crate that makes up the layout core.
#[derive(Debug)]
pub(crate) struct CoreCrate {
    /// The package name, as its own manifest declares it.
    pub(crate) name: String,
    /// The directory holding that manifest.
    pub(crate) directory: PathBuf,
}

/// Locate the workspace root relative to this crate.
pub(crate) fn workspace_root() -> io::Result<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "the xtask manifest directory has no parent",
            )
        })
}

/// The crates that make up the layout core, in workspace order.
///
/// Every workspace member except the exempted ones, named by the package name
/// its own manifest declares rather than by its directory, because a dependency is written
/// with the package name and the two are free to differ.
pub(crate) fn core_crates() -> io::Result<Vec<CoreCrate>> {
    let root = workspace_root()?;
    let manifest = fs::read_to_string(root.join("Cargo.toml"))?;
    let members = workspace_members(&manifest);
    if members.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Cargo.toml declares no workspace members",
        ));
    }

    for excluded in NON_CORE_MEMBERS {
        if !members.iter().any(|member| member == excluded) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "`{excluded}` is exempted from the layout core but is not a workspace \
                     member; the exemption list in xtask/src/shared.rs has gone stale"
                ),
            ));
        }
    }

    let mut crates = Vec::new();
    for member in members {
        if NON_CORE_MEMBERS.contains(&member.as_str()) {
            continue;
        }
        let directory = root.join(&member);
        let manifest = fs::read_to_string(directory.join("Cargo.toml"))?;
        let name = package_name(&manifest).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{member}/Cargo.toml declares no package name"),
            )
        })?;
        crates.push(CoreCrate {
            name: name.to_owned(),
            directory,
        });
    }
    Ok(crates)
}

/// Read the member paths out of a workspace manifest.
///
/// Hand-rolled for the reason stated on the `purity` module's manifest scan: the tool that
/// enforces "the layout core has no outside dependencies" declares none itself. It
/// understands the one form this repository writes — a `members` array under
/// `[workspace]`, on one line or several — and reads nothing else.
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
            members.extend(quoted_values(line).into_iter().map(str::to_owned));
            inside_members = !line.contains(']');
            continue;
        }

        if let Some(header) = table_header(line) {
            inside_workspace = header == "workspace";
            continue;
        }

        if inside_workspace {
            if let Some((key, value)) = line.split_once('=') {
                if key.trim() == "members" {
                    members.extend(quoted_values(value).into_iter().map(str::to_owned));
                    inside_members = !value.contains(']');
                }
            }
        }
    }

    members
}

/// Read the package name out of a crate manifest.
fn package_name(manifest: &str) -> Option<&str> {
    let mut inside_package = false;
    for line in manifest.lines() {
        let line = without_comment(line).trim();
        if let Some(header) = table_header(line) {
            inside_package = header == "package";
            continue;
        }
        if inside_package {
            if let Some((key, value)) = line.split_once('=') {
                if key.trim() == "name" {
                    return quoted_values(value).first().copied();
                }
            }
        }
    }
    None
}

/// The name inside a `[table]` header, if the line is one.
fn table_header(line: &str) -> Option<&str> {
    line.strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .map(str::trim)
}

/// Everything before the first `#` that is not inside a string.
fn without_comment(line: &str) -> &str {
    let mut inside_string = false;
    for (index, character) in line.char_indices() {
        match character {
            '"' => inside_string = !inside_string,
            '#' if !inside_string => return line.get(..index).unwrap_or(line),
            _ => {},
        }
    }
    line
}

/// The string literals on a line, in order, without their quotes.
fn quoted_values(line: &str) -> Vec<&str> {
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

/// Gather every `.rs` file under `dir`, recursively, in a stable order.
pub(crate) fn rust_sources(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    collect_rust_sources(dir, &mut found)?;
    found.sort();
    Ok(found)
}

/// Walk one directory, appending in whatever order the filesystem reports.
fn collect_rust_sources(dir: &Path, found: &mut Vec<PathBuf>) -> io::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_rust_sources(&path, found)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
    Ok(())
}

/// Name a source file relative to the directory it was found under.
pub(crate) fn relative_name(source: &Path, dir: &Path) -> String {
    source
        .strip_prefix(dir)
        .unwrap_or(source)
        .display()
        .to_string()
}

/// Drop `//` comments so prose that names a forbidden token is not itself a violation.
///
/// This is a line-oriented approximation that does not track string literals or block
/// comments. It is adequate because it reads only this repository's own sources, and the
/// worst case is ignoring a token inside a string literal, which is not a violation.
pub(crate) fn code_only(source: &str) -> String {
    source
        .lines()
        .map(|line| line.split_once("//").map_or(line, |(code, _)| code))
        .collect::<Vec<_>>()
        .join("\n")
}

// ---------------------------------------------------------------------------------------
// The address grammar
// ---------------------------------------------------------------------------------------

// One carrier for the canonical rendering docs/adr/0013 fixes, read by `spec-links` from
// doc comments, by `conform` from case files, and by `attest` from the captured matrices.
// Three hand-rolled copies accepted three different languages, and the coverage gate was
// the lax one: a case file could name a cell no inventory row can ever carry and the
// subtraction still closed. That is the defect docs/adr/0019 names, committed inside the
// tool that enforces it.
//
// `jlreq-spec` states the same grammar a second time, as a `const fn` over bytes, because
// xtask declares no dependencies and a core crate is not one it could declare. Those two
// are two carriers by necessity, and they are held equal by the corpus in
// docs/design/address-corpus.tsv, which both of them read.

/// The deepest section path an address holds: `1.2.3.4`.
const MAX_PARTS: usize = 4;

/// The number of character classes §3.9.2 closes the set at.
const CLASSES: u8 = 30;

/// A specification address: a section, and what it names inside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Address {
    /// The section path.
    pub(crate) section: Section,
    /// What it names within that section.
    pub(crate) detail: Detail,
}

/// What an address names within a section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Detail {
    /// The section itself: `3.1.9`.
    Whole,
    /// One numbered note: the `#3` of `B.2#3`.
    Note(u8),
    /// One matrix cell, row then column: the `@cl-05,cl-05` of `B.1@cl-05,cl-05`.
    Cell(Before, After),
}

/// The row coordinate of a matrix cell: a character class, or the line head.
///
/// The two coordinates are two types and not one, because the matrices are not symmetric
/// in them: Tables 1 and 3 through 5 carry one line-head *row* and one line-end *column*
/// and nothing else, which is the frozen reason `jlreq_spacing::Before` and
/// `jlreq_spacing::After` are two types in docs/api-frozen.toml. A symmetric coordinate
/// makes `B.1@cl-02,line-head` — a cell no matrix has — a well-formed address, in a space
/// docs/adr/0013 calls a one-way door.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Before {
    /// One of the thirty classes, `cl-01` through `cl-30`.
    Class(u8),
    /// The line head row.
    LineHead,
}

/// The column coordinate of a matrix cell: a character class, or the line end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum After {
    /// One of the thirty classes, `cl-01` through `cl-30`.
    Class(u8),
    /// The line end column.
    LineEnd,
}

/// A section path: an optional appendix letter and its numbered components.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Section {
    /// The appendix, when the path is one of the seven lettered ones.
    pub(crate) appendix: Option<char>,
    /// The numbered components: `3.1.9`'s three, `B.2`'s one.
    pub(crate) parts: Vec<u8>,
}

impl fmt::Display for Address {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let section = &self.section;
        match &self.detail {
            Detail::Whole => write!(formatter, "{section}"),
            Detail::Note(note) => write!(formatter, "{section}#{note}"),
            Detail::Cell(row, column) => write!(formatter, "{section}@{row},{column}"),
        }
    }
}

impl fmt::Display for Section {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(appendix) = self.appendix {
            write!(formatter, "{appendix}")?;
        }
        for (index, part) in self.parts.iter().enumerate() {
            if index > 0 || self.appendix.is_some() {
                write!(formatter, ".")?;
            }
            write!(formatter, "{part}")?;
        }
        Ok(())
    }
}

impl fmt::Display for Before {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Class(number) => write!(formatter, "cl-{number:02}"),
            Self::LineHead => formatter.write_str("line-head"),
        }
    }
}

impl fmt::Display for After {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Class(number) => write!(formatter, "cl-{number:02}"),
            Self::LineEnd => formatter.write_str("line-end"),
        }
    }
}

/// Parse the canonical rendering of an address. `None` when the text is not one.
pub(crate) fn address(text: &str) -> Option<Address> {
    if let Some((head, tail)) = text.split_once('#') {
        return Some(Address {
            section: section(head)?,
            detail: Detail::Note(number(tail)?),
        });
    }
    if let Some((head, tail)) = text.split_once('@') {
        let (row, column) = tail.split_once(',')?;
        return Some(Address {
            section: section(head)?,
            detail: Detail::Cell(before(row)?, after(column)?),
        });
    }
    Some(Address {
        section: section(text)?,
        detail: Detail::Whole,
    })
}

/// Parse a section path: an optional appendix letter, then dot-separated numbers.
pub(crate) fn section(text: &str) -> Option<Section> {
    let appendix = match text.chars().next() {
        Some(letter @ 'A'..='G') => Some(letter),
        _ => None,
    };
    let rest = match appendix {
        Some(_) => text.get(1..)?,
        None => text,
    };
    let mut parts = Vec::new();
    if !rest.is_empty() {
        let numbers = match appendix {
            Some(_) => rest.strip_prefix('.')?,
            None => rest,
        };
        for component in numbers.split('.') {
            parts.push(number(component)?);
            if parts.len() > MAX_PARTS {
                return None;
            }
        }
    }
    if appendix.is_none() && parts.is_empty() {
        return None;
    }
    Some(Section { appendix, parts })
}

/// Parse one number of an address, rejecting every non-canonical spelling: an empty run of
/// digits, a sign, a leading zero, a zero, and a value this representation cannot hold.
///
/// The specification numbers its sections, its tables and its notes from one, so a zero
/// addresses nothing.
pub(crate) fn number(digits: &str) -> Option<u8> {
    if digits.is_empty()
        || digits.starts_with('0')
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    digits.parse().ok()
}

/// Parse a row coordinate: `cl-05` or `line-head`, and never `line-end`.
pub(crate) fn before(text: &str) -> Option<Before> {
    match text {
        "line-head" => Some(Before::LineHead),
        "line-end" => None,
        _ => class(text).map(Before::Class),
    }
}

/// Parse a column coordinate: `cl-05` or `line-end`, and never `line-head`.
pub(crate) fn after(text: &str) -> Option<After> {
    match text {
        "line-end" => Some(After::LineEnd),
        "line-head" => None,
        _ => class(text).map(After::Class),
    }
}

/// Parse a class coordinate.
///
/// JLReq pads a class to two digits — 302 occurrences of `cl-01` and none of `cl-1` — so
/// the padding is the canonical spelling rather than a courtesy.
pub(crate) fn class(text: &str) -> Option<u8> {
    let digits = text.strip_prefix("cl-")?;
    if digits.len() != 2 || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let number: u8 = digits.parse().ok()?;
    (1..=CLASSES).contains(&number).then_some(number)
}

#[cfg(test)]
mod tests {
    use super::{
        NON_CORE_MEMBERS, address, core_crates, package_name, quoted_values, table_header,
        without_comment, workspace_members, workspace_root,
    };

    /// The shared address corpus, relative to the workspace root.
    const CORPUS: &str = "docs/design/address-corpus.tsv";

    #[test]
    fn the_address_grammar_agrees_with_the_corpus_jlreq_spec_reads() {
        // The grammar has two carriers and cannot have one: this one is `std` and declares
        // no dependencies, and `jlreq_spec::Address::parse` is `no_std` and `const`, so
        // neither can call the other. Both read this file, so a spelling one accepts and
        // the other refuses fails here rather than reaching a published case file
        // (docs/adr/0013, docs/adr/0019).
        let path = workspace_root().expect("the workspace root").join(CORPUS);
        let corpus = std::fs::read_to_string(&path).expect("the address corpus is readable");
        let mut rows = 0_usize;
        for line in corpus.lines().skip(1) {
            let mut fields = line.split('\t');
            let (Some(text), Some(verdict)) = (fields.next(), fields.next()) else {
                continue;
            };
            let accepted = match verdict {
                "yes" => true,
                "no" => false,
                other => panic!("`{text}` is neither accepted nor rejected but `{other}`"),
            };
            assert_eq!(
                address(text).is_some(),
                accepted,
                "`{text}`: the corpus says accepted={accepted}"
            );
            if accepted {
                assert_eq!(
                    address(text).map(|parsed| parsed.to_string()).as_deref(),
                    Some(text),
                    "`{text}` renders as itself"
                );
            }
            rows = rows.saturating_add(1);
        }
        assert!(rows > 60, "the corpus was read: {rows} rows");
    }

    #[test]
    fn reads_members_from_a_multi_line_array() {
        let manifest = "[workspace]\nresolver = \"3\"\nmembers = [\n  \"crates/a\",\n  \"xtask\",\n]\nexclude = [\"fuzz\"]\n";
        assert_eq!(workspace_members(manifest), ["crates/a", "xtask"]);
    }

    #[test]
    fn reads_members_from_a_single_line_array() {
        assert_eq!(
            workspace_members("[workspace]\nmembers = [\"crates/a\", \"crates/b\"]\n"),
            ["crates/a", "crates/b"]
        );
    }

    #[test]
    fn reads_members_only_from_the_workspace_table() {
        let manifest = "[workspace.metadata]\nmembers = [\"decoy\"]\n\n[workspace]\nmembers = [\"crates/a\"]\n";
        assert_eq!(workspace_members(manifest), ["crates/a"]);
    }

    #[test]
    fn a_commented_out_member_is_not_a_member() {
        let manifest = "[workspace]\nmembers = [\n  \"crates/a\",\n  # \"crates/b\",\n]\n";
        assert_eq!(workspace_members(manifest), ["crates/a"]);
    }

    #[test]
    fn reads_the_package_name_and_not_another_tables_name() {
        let manifest =
            "[package]\nname = \"jlreq-unit\"\nedition = \"2024\"\n\n[lints]\nname = \"decoy\"\n";
        assert_eq!(package_name(manifest), Some("jlreq-unit"));
        assert_eq!(package_name("[lints]\nname = \"decoy\"\n"), None);
    }

    #[test]
    fn a_table_header_is_a_whole_line() {
        assert_eq!(table_header("[package]"), Some("package"));
        assert_eq!(table_header("members = [\"crates/a\"]"), None);
        assert_eq!(quoted_values("name = \"a\" # \"b\""), ["a", "b"]);
        assert_eq!(
            quoted_values(without_comment("name = \"a\" # \"b\"")),
            ["a"]
        );
    }

    #[test]
    fn the_core_is_every_member_that_is_not_exempted() {
        let core = core_crates().expect("the workspace manifest is readable");
        let names: Vec<&str> = core.iter().map(|each| each.name.as_str()).collect();
        assert!(names.contains(&"jlreq-unit"), "found {names:?}");
        assert!(names.contains(&"jlreq"), "found {names:?}");
        for exempted in NON_CORE_MEMBERS {
            let last = exempted.rsplit('/').next().unwrap_or(exempted);
            assert!(!names.contains(&last), "{exempted} is not core");
        }
        for each in &core {
            assert!(
                each.directory.join("Cargo.toml").is_file(),
                "{name} has a manifest",
                name = each.name
            );
        }
    }
}
