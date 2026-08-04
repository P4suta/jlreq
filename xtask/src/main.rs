// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Repository maintenance tasks for the kumihan workspace.
//!
//! Run with `cargo run -p xtask -- <task>`, or through the `Justfile` recipes.
//!
//! # `purity`
//!
//! Checks that the layout core still satisfies the invariants it was designed around:
//!
//! - it declares no dependency outside the core itself (`docs/adr/0001`),
//! - every core crate declares `#![no_std]` (`docs/adr/0001`),
//! - no core source uses `f32` or `f64` (`docs/adr/0005`).
//!
//! These are the invariants that decide whether this library can run in a browser, in a
//! game engine, or on a target without a floating-point unit, and whether its conformance
//! suite can assert exact values. They are cheap to hold from an empty repository and
//! expensive to restore once violated, which is why the gate exists before the code does.
//!
//! `just no-std` and `just wasm` are the compile-time half of the same guarantee.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// The crates that make up the layout core.
///
/// Mirrors `core_crates` in the `Justfile`. A new core crate belongs in both.
const CORE_CRATES: &[&str] = &[
    "jlreq-class",
    "jlreq-spacing",
    "jlreq-line",
    "jlreq-inline",
    "jlreq",
];

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        Some("purity") => run_purity(),
        Some(task) => {
            eprintln!("xtask: unknown task `{task}`");
            print_usage();
            ExitCode::FAILURE
        },
        None => {
            print_usage();
            ExitCode::FAILURE
        },
    }
}

/// Print the available tasks.
fn print_usage() {
    eprintln!("usage: cargo run -p xtask -- purity");
}

/// Report every purity violation, or confirm there are none.
fn run_purity() -> ExitCode {
    match collect_violations() {
        Err(error) => {
            eprintln!("xtask: purity could not run: {error}");
            ExitCode::FAILURE
        },
        Ok(violations) if violations.is_empty() => {
            println!(
                "purity: the layout core declares no outside dependencies, stays no_std, \
                 and uses no floating point"
            );
            ExitCode::SUCCESS
        },
        Ok(violations) => {
            for violation in &violations {
                eprintln!("purity: {violation}");
            }
            eprintln!(
                "purity: {count} violation(s). See docs/adr/0001-no-std-no-io-no-font-in-core.md \
                 and docs/adr/0005-integer-layout-units.md",
                count = violations.len()
            );
            ExitCode::FAILURE
        },
    }
}

/// Check every core crate and gather the findings.
fn collect_violations() -> io::Result<Vec<String>> {
    let root = workspace_root()?.join("crates");
    let mut violations = Vec::new();
    for crate_name in CORE_CRATES {
        let each = root.join(crate_name);
        check_manifest(&each.join("Cargo.toml"), crate_name, &mut violations)?;
        check_sources(&each.join("src"), crate_name, &mut violations)?;
    }
    Ok(violations)
}

/// Locate the workspace root relative to this crate.
fn workspace_root() -> io::Result<PathBuf> {
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

/// Reject any declared dependency that is not itself part of the core.
fn check_manifest(path: &Path, crate_name: &str, violations: &mut Vec<String>) -> io::Result<()> {
    let manifest = fs::read_to_string(path)?;
    for dependency in declared_dependencies(&manifest) {
        if !CORE_CRATES.contains(&dependency.as_str()) {
            violations.push(format!(
                "{crate_name}: declares `{dependency}`; the layout core may depend only on \
                 other core crates (ADR 0001)"
            ));
        }
    }
    Ok(())
}

/// Require `#![no_std]` and reject floating point in every source file.
fn check_sources(dir: &Path, crate_name: &str, violations: &mut Vec<String>) -> io::Result<()> {
    let mut sources = Vec::new();
    collect_rust_sources(dir, &mut sources)?;
    sources.sort();

    let lib = dir.join("lib.rs");
    if !sources.contains(&lib) {
        violations.push(format!("{crate_name}: has no src/lib.rs"));
    }

    for source in &sources {
        let code = code_only(&fs::read_to_string(source)?);
        let name = relative_name(source, dir);
        if *source == lib && !code.lines().any(|line| line.trim() == "#![no_std]") {
            violations.push(format!(
                "{crate_name}: {name} does not declare `#![no_std]` (ADR 0001)"
            ));
        }
        if let Some(token) = float_token(&code) {
            violations.push(format!(
                "{crate_name}: {name} uses `{token}`; layout arithmetic is integer (ADR 0005)"
            ));
        }
    }
    Ok(())
}

/// Gather every `.rs` file under `dir`, recursively.
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

/// Name a source file relative to its crate's `src` directory.
fn relative_name(source: &Path, dir: &Path) -> String {
    source
        .strip_prefix(dir)
        .unwrap_or(source)
        .display()
        .to_string()
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

/// Drop `//` comments so prose that names a forbidden token is not itself a violation.
///
/// This is a line-oriented approximation that does not track string literals or block
/// comments. It is adequate because it reads only this repository's own sources, and the
/// worst case is ignoring a token inside a string literal, which is not a violation.
fn code_only(source: &str) -> String {
    source
        .lines()
        .map(|line| line.split_once("//").map_or(line, |(code, _)| code))
        .collect::<Vec<_>>()
        .join("\n")
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

#[cfg(test)]
mod tests {
    use super::{
        code_only, declared_dependencies, dependency_subtable_name, float_token,
        is_dependency_table,
    };

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
    }
}
