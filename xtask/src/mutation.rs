// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Cross-platform validation of generated-source and equivalent-mutant exclusions.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::Path;

use crate::generate::sha256;
use crate::shared::{Gate, array_header, before_comment, workspace_root};

const LEDGER: &str = "docs/mutation-ledger.toml";
const CONFIG: &str = ".cargo/mutants.toml";
const GENERATED: &str = "crates/jlreq-core/src/generated";

pub(crate) const GATE: Gate = Gate {
    name: "mutation-ledger",
    purpose: "every generated exclusion and equivalent mutant is individually provenance- and SHA-256-bound",
    reference: LEDGER,
    run,
};

#[derive(Debug, Default)]
struct Exclusion {
    path: String,
    sha256: String,
    kind: String,
    provenance: String,
    reason: String,
}

impl Exclusion {
    fn complete(&self) -> bool {
        !self.path.is_empty()
            && !self.sha256.is_empty()
            && !self.kind.is_empty()
            && !self.provenance.is_empty()
            && !self.reason.is_empty()
    }
}

#[derive(Debug, Default)]
struct Equivalent {
    mutant: String,
    exclude_re: String,
    source_sha256: String,
    proof: String,
}

impl Equivalent {
    fn complete(&self) -> bool {
        !self.mutant.is_empty()
            && !self.exclude_re.is_empty()
            && !self.source_sha256.is_empty()
            && !self.proof.is_empty()
    }
}

#[derive(Debug)]
enum OpenTable {
    Exclusion(Exclusion),
    Equivalent(Equivalent),
    Other,
}

#[derive(Debug, Default)]
struct Ledger {
    exclusions: Vec<Exclusion>,
    equivalents: Vec<Equivalent>,
    violations: Vec<String>,
}

fn run(arguments: &[String]) -> io::Result<Vec<String>> {
    if let Some(first) = arguments.first() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("mutation-ledger takes no arguments, observed `{first}`"),
        ));
    }
    let root = workspace_root()?;
    let ledger_source = fs::read_to_string(root.join(LEDGER))?;
    let config = fs::read_to_string(root.join(CONFIG))?;
    let mut ledger = parse_ledger(&ledger_source);
    validate_exclusions(&root, &ledger.exclusions, &mut ledger.violations)?;
    validate_config(&root, &config, &ledger.equivalents, &mut ledger.violations)?;
    Ok(ledger.violations)
}

fn parse_ledger(source: &str) -> Ledger {
    let mut ledger = Ledger::default();
    let mut open = OpenTable::Other;
    for (index, raw) in source.lines().enumerate() {
        let line_number = index.saturating_add(1);
        let line = before_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        if let Some(header) = array_header(line) {
            finish_table(&mut open, &mut ledger, line_number);
            open = match header {
                "exclusion" => OpenTable::Exclusion(Exclusion::default()),
                "equivalent" => OpenTable::Equivalent(Equivalent::default()),
                _ => OpenTable::Other,
            };
            continue;
        }
        let Some((key, raw_value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        match &mut open {
            OpenTable::Exclusion(value) => {
                let Some(parsed) = quoted_value(raw_value) else {
                    ledger.violations.push(format!(
                        "{LEDGER}:{line_number}: `{key}` is not a one-line quoted string"
                    ));
                    continue;
                };
                match key {
                    "path" => value.path = parsed,
                    "sha256" => value.sha256 = parsed,
                    "kind" => value.kind = parsed,
                    "provenance" => value.provenance = parsed,
                    "reason" => value.reason = parsed,
                    "glob" => ledger.violations.push(format!(
                        "{LEDGER}:{line_number}: generated exclusions must name individual paths, not a broad glob"
                    )),
                    _ => ledger.violations.push(format!(
                        "{LEDGER}:{line_number}: unknown exclusion field `{key}`"
                    )),
                }
            },
            OpenTable::Equivalent(value) => {
                let Some(parsed) = quoted_value(raw_value) else {
                    ledger.violations.push(format!(
                        "{LEDGER}:{line_number}: `{key}` is not a one-line quoted string"
                    ));
                    continue;
                };
                match key {
                    "mutant" => value.mutant = parsed,
                    "exclude_re" => value.exclude_re = parsed,
                    "source_sha256" => value.source_sha256 = parsed,
                    "proof" => value.proof = parsed,
                    _ => ledger.violations.push(format!(
                        "{LEDGER}:{line_number}: unknown equivalent field `{key}`"
                    )),
                }
            },
            OpenTable::Other => {},
        }
    }
    finish_table(
        &mut open,
        &mut ledger,
        source.lines().count().saturating_add(1),
    );
    ledger
}

fn finish_table(open: &mut OpenTable, ledger: &mut Ledger, line_number: usize) {
    match std::mem::replace(open, OpenTable::Other) {
        OpenTable::Exclusion(value) if value.complete() => ledger.exclusions.push(value),
        OpenTable::Exclusion(_) => ledger
            .violations
            .push(format!("{LEDGER}:{line_number}: incomplete [[exclusion]]")),
        OpenTable::Equivalent(value) if value.complete() => ledger.equivalents.push(value),
        OpenTable::Equivalent(_) => ledger
            .violations
            .push(format!("{LEDGER}:{line_number}: incomplete [[equivalent]]")),
        OpenTable::Other => {},
    }
}

fn quoted_value(raw: &str) -> Option<String> {
    let value = raw.trim();
    if let Some(inner) = value
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
    {
        return Some(inner.to_owned());
    }
    value
        .strip_prefix('\'')
        .and_then(|rest| rest.strip_suffix('\''))
        .map(str::to_owned)
}

fn validate_exclusions(
    root: &Path,
    exclusions: &[Exclusion],
    violations: &mut Vec<String>,
) -> io::Result<()> {
    let generated = generated_sources(root)?;
    let documented: BTreeSet<_> = exclusions.iter().map(|item| item.path.clone()).collect();
    for path in generated.difference(&documented) {
        violations.push(format!(
            "{LEDGER}: generated source `{path}` is not excluded"
        ));
    }
    for path in documented.difference(&generated) {
        violations.push(format!(
            "{LEDGER}: exclusion `{path}` is not a generated table source"
        ));
    }
    report_duplicates(
        exclusions.iter().map(|item| item.path.as_str()),
        "generated exclusion",
        violations,
    );

    for exclusion in exclusions {
        if exclusion.kind != "generated" {
            violations.push(format!(
                "{} has non-generated exclusion kind `{}`",
                exclusion.path, exclusion.kind
            ));
        }
        if exclusion.provenance != "data/manifest.toml" {
            violations.push(format!(
                "{} is not anchored to data/manifest.toml",
                exclusion.path
            ));
        }
        validate_digest(
            root,
            &exclusion.path,
            &exclusion.sha256,
            "generated exclusion",
            violations,
        )?;
    }
    Ok(())
}

fn generated_sources(root: &Path) -> io::Result<BTreeSet<String>> {
    let directory = root.join(GENERATED);
    let mut paths = BTreeSet::new();
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_file() && path.extension().is_some_and(|extension| extension == "rs") {
            paths.insert(repository_path(root, &path));
        }
    }
    Ok(paths)
}

fn validate_config(
    root: &Path,
    config: &str,
    equivalents: &[Equivalent],
    violations: &mut Vec<String>,
) -> io::Result<()> {
    let exact_glob = "exclude_globs = [\"crates/jlreq-core/src/generated/**\"]";
    if !config.lines().any(|line| line == exact_glob) {
        violations.push(format!(
            "{CONFIG} must exclude the generated table directory exactly"
        ));
    }
    if config.contains("\"crates/jlreq-core/src/generated.rs\"") {
        violations.push(format!(
            "{CONFIG} excludes the handwritten generated.rs integrity checks"
        ));
    }
    report_duplicates(
        equivalents.iter().map(|item| item.mutant.as_str()),
        "equivalent mutant",
        violations,
    );
    report_duplicates(
        equivalents.iter().map(|item| item.exclude_re.as_str()),
        "equivalent-mutant regex",
        violations,
    );

    for equivalent in equivalents {
        let Some(source) = mutant_source(&equivalent.mutant) else {
            violations.push(format!(
                "equivalent mutant does not start with a Rust source path: {}",
                equivalent.mutant
            ));
            continue;
        };
        validate_digest(
            root,
            &source,
            &equivalent.source_sha256,
            "equivalent mutant",
            violations,
        )?;
        let configured = format!("  '{}',", equivalent.exclude_re);
        if !config.lines().any(|line| line == configured) {
            violations.push(format!(
                "{} has no exact regex in {CONFIG}",
                equivalent.mutant
            ));
        }
    }
    let configured = config
        .lines()
        .filter(|line| line.starts_with("  '") && line.ends_with("',"))
        .count();
    if configured != equivalents.len() {
        violations.push(format!(
            "{CONFIG} has {configured} equivalent regex(es), but {LEDGER} documents {}",
            equivalents.len()
        ));
    }
    Ok(())
}

fn mutant_source(mutant: &str) -> Option<String> {
    let (prefix, _) = mutant.split_once(".rs:")?;
    Some(format!("{prefix}.rs"))
}

fn validate_digest(
    root: &Path,
    relative: &str,
    expected: &str,
    label: &str,
    violations: &mut Vec<String>,
) -> io::Result<()> {
    if expected.len() != 64 || !expected.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        violations.push(format!("{relative} has a malformed SHA-256 for {label}"));
        return Ok(());
    }
    if expected.bytes().any(|byte| byte.is_ascii_uppercase()) {
        violations.push(format!(
            "{relative} has a non-lowercase SHA-256 for {label}"
        ));
        return Ok(());
    }
    let path = root.join(relative);
    if !path.is_file() {
        violations.push(format!("{relative} does not exist"));
        return Ok(());
    }
    let actual = sha256(&fs::read(path)?);
    if actual != expected {
        violations.push(format!(
            "{relative} changed: expected {expected}, observed {actual}"
        ));
    }
    Ok(())
}

fn report_duplicates<'a>(
    values: impl Iterator<Item = &'a str>,
    label: &str,
    violations: &mut Vec<String>,
) {
    let mut seen = BTreeSet::new();
    let mut reported = BTreeSet::new();
    for value in values {
        if !seen.insert(value) && reported.insert(value) {
            violations.push(format!("duplicate {label}: {value}"));
        }
    }
}

fn repository_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_ledger_tables_parse_both_toml_string_styles() {
        let parsed = parse_ledger(
            "[[exclusion]]\npath = \"a.rs\"\nsha256 = \"00\"\nkind = \"generated\"\nprovenance = \"data/manifest.toml\"\nreason = \"why\"\n\n[[equivalent]]\nmutant = \"a.rs:1\"\nexclude_re = '^a$'\nsource_sha256 = \"11\"\nproof = \"same\"\n",
        );
        assert!(parsed.violations.is_empty());
        assert_eq!(parsed.exclusions.len(), 1);
        assert_eq!(parsed.equivalents.len(), 1);
        assert_eq!(parsed.equivalents[0].exclude_re, "^a$");
    }

    #[test]
    fn incomplete_unknown_and_glob_fields_are_findings() {
        let parsed = parse_ledger(
            "[[exclusion]]\nglob = \"generated/**\"\nunknown = \"x\"\n\n[[equivalent]]\nmutant = \"a.rs:1\"\n",
        );
        assert!(
            parsed
                .violations
                .iter()
                .any(|item| item.contains("broad glob"))
        );
        assert!(
            parsed
                .violations
                .iter()
                .any(|item| item.contains("unknown exclusion"))
        );
        assert_eq!(
            parsed
                .violations
                .iter()
                .filter(|item| item.contains("incomplete"))
                .count(),
            2
        );
    }

    #[test]
    fn mutant_paths_and_repository_paths_are_platform_independent() {
        assert_eq!(
            mutant_source("crates/a/src/lib.rs:1: replace"),
            Some("crates/a/src/lib.rs".into())
        );
        let root = std::path::PathBuf::from("workspace");
        assert_eq!(
            repository_path(&root, &root.join("crates").join("a").join("lib.rs")),
            "crates/a/lib.rs"
        );
    }

    #[test]
    fn the_repository_ledger_holds_without_posix_tools() {
        assert!(
            run(&[])
                .expect("the mutation ledger is readable")
                .is_empty()
        );
    }
}
