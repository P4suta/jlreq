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
    assigned: BTreeSet<String>,
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
    assigned: BTreeSet<String>,
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
        let mut findings = Vec::new();
        match &mut open {
            OpenTable::Exclusion(value) => {
                let Some(parsed) = quoted_value(raw_value) else {
                    ledger.violations.push(format!(
                        "{LEDGER}:{line_number}: `{key}` is not a one-line quoted string"
                    ));
                    continue;
                };
                match key {
                    "path" => assign_once(
                        &mut value.path,
                        &mut value.assigned,
                        &parsed,
                        key,
                        line_number,
                        &mut findings,
                    ),
                    "sha256" => {
                        assign_once(
                            &mut value.sha256,
                            &mut value.assigned,
                            &parsed,
                            key,
                            line_number,
                            &mut findings,
                        );
                    },
                    "kind" => assign_once(
                        &mut value.kind,
                        &mut value.assigned,
                        &parsed,
                        key,
                        line_number,
                        &mut findings,
                    ),
                    "provenance" => assign_once(
                        &mut value.provenance,
                        &mut value.assigned,
                        &parsed,
                        key,
                        line_number,
                        &mut findings,
                    ),
                    "reason" => {
                        assign_once(
                            &mut value.reason,
                            &mut value.assigned,
                            &parsed,
                            key,
                            line_number,
                            &mut findings,
                        );
                    },
                    "glob" => findings.push(format!(
                        "{LEDGER}:{line_number}: generated exclusions must name individual paths, not a broad glob"
                    )),
                    _ => findings.push(format!(
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
                    "mutant" => {
                        assign_once(
                            &mut value.mutant,
                            &mut value.assigned,
                            &parsed,
                            key,
                            line_number,
                            &mut findings,
                        );
                    },
                    "exclude_re" => assign_once(
                        &mut value.exclude_re,
                        &mut value.assigned,
                        &parsed,
                        key,
                        line_number,
                        &mut findings,
                    ),
                    "source_sha256" => assign_once(
                        &mut value.source_sha256,
                        &mut value.assigned,
                        &parsed,
                        key,
                        line_number,
                        &mut findings,
                    ),
                    "proof" => {
                        assign_once(
                            &mut value.proof,
                            &mut value.assigned,
                            &parsed,
                            key,
                            line_number,
                            &mut findings,
                        );
                    },
                    _ => findings.push(format!(
                        "{LEDGER}:{line_number}: unknown equivalent field `{key}`"
                    )),
                }
            },
            OpenTable::Other => {},
        }
        ledger.violations.extend(findings);
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

fn assign_once(
    field: &mut String,
    assigned: &mut BTreeSet<String>,
    parsed: &str,
    key: &str,
    line_number: usize,
    findings: &mut Vec<String>,
) {
    if assigned.insert(key.to_owned()) {
        field.push_str(parsed);
    } else {
        findings.push(format!(
            "{LEDGER}:{line_number}: `{key}` is stated more than once in one table"
        ));
    }
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
    let generated_glob = "crates/jlreq-core/src/generated/**";
    let configured_globs = match config_string_array(config, "exclude_globs") {
        Ok(values) => values,
        Err(finding) => {
            violations.push(format!("{CONFIG}: {finding}"));
            Vec::new()
        },
    };
    if configured_globs != [generated_glob] {
        violations.push(format!(
            "{CONFIG} must exclude the generated table directory exactly"
        ));
    }
    if configured_globs
        .iter()
        .any(|glob| glob == "crates/jlreq-core/src/generated.rs")
    {
        violations.push(format!(
            "{CONFIG} excludes the handwritten generated.rs integrity checks"
        ));
    }
    let configured_regexes = match config_string_array(config, "exclude_re") {
        Ok(values) => values,
        Err(finding) => {
            violations.push(format!("{CONFIG}: {finding}"));
            Vec::new()
        },
    };
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
    report_duplicates(
        configured_regexes.iter().map(String::as_str),
        "configured equivalent-mutant regex",
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
        if !configured_regexes.contains(&equivalent.exclude_re) {
            violations.push(format!(
                "{} has no exact regex in {CONFIG}",
                equivalent.mutant
            ));
        }
    }
    let documented: BTreeSet<_> = equivalents
        .iter()
        .map(|item| item.exclude_re.as_str())
        .collect();
    for unexpected in configured_regexes
        .iter()
        .map(String::as_str)
        .filter(|regex| !documented.contains(regex))
    {
        violations.push(format!(
            "{CONFIG} has undocumented equivalent-mutant regex `{unexpected}`"
        ));
    }
    if configured_regexes.len() != equivalents.len() {
        violations.push(format!(
            "{CONFIG} has {} equivalent regex(es), but {LEDGER} documents {}",
            configured_regexes.len(),
            equivalents.len()
        ));
    }
    Ok(())
}

fn config_string_array(config: &str, key: &str) -> Result<Vec<String>, String> {
    let mut offsets = Vec::new();
    let mut line_start = 0_usize;
    for line in config.split_inclusive('\n') {
        let content = line.strip_suffix('\n').unwrap_or(line);
        let visible = before_comment(content);
        match (visible.split_once('='), content.find('=')) {
            (Some((candidate, _)), Some(equals)) if candidate.trim() == key => {
                offsets.push(line_start.saturating_add(equals).saturating_add(1));
            },
            _ => {},
        }
        line_start = line_start.saturating_add(line.len());
    }
    match offsets.as_slice() {
        [] => Err(format!("missing `{key}` string array")),
        [offset] => parse_toml_string_array(&config[*offset..])
            .map_err(|finding| format!("invalid `{key}` string array: {finding}")),
        _ => Err(format!("`{key}` is assigned more than once")),
    }
}

fn parse_toml_string_array(source: &str) -> Result<Vec<String>, String> {
    let characters: Vec<char> = source.chars().collect();
    let mut cursor = 0_usize;
    skip_toml_trivia(&characters, &mut cursor);
    if characters.get(cursor) != Some(&'[') {
        return Err("value is not an array".to_owned());
    }
    cursor = cursor.saturating_add(1);
    let mut values = Vec::new();
    loop {
        skip_toml_trivia(&characters, &mut cursor);
        if characters.get(cursor) == Some(&']') {
            return Ok(values);
        }
        let quote = characters
            .get(cursor)
            .copied()
            .filter(|quote| matches!(quote, '\'' | '"'))
            .ok_or_else(|| "array element is not a quoted string".to_owned())?;
        cursor = cursor.saturating_add(1);
        let mut value = String::new();
        loop {
            let character = characters
                .get(cursor)
                .copied()
                .ok_or_else(|| "quoted string is not terminated".to_owned())?;
            cursor = cursor.saturating_add(1);
            if character == quote {
                break;
            }
            if quote == '"' && character == '\\' {
                let escaped = characters
                    .get(cursor)
                    .copied()
                    .ok_or_else(|| "basic string escape is not terminated".to_owned())?;
                cursor = cursor.saturating_add(1);
                value.push(match escaped {
                    'b' => '\u{0008}',
                    't' => '\t',
                    'n' => '\n',
                    'f' => '\u{000c}',
                    'r' => '\r',
                    '"' => '"',
                    '\\' => '\\',
                    _ => return Err(format!("unsupported basic string escape `\\{escaped}`")),
                });
            } else {
                value.push(character);
            }
        }
        values.push(value);
        skip_toml_trivia(&characters, &mut cursor);
        match characters.get(cursor) {
            Some(',') => cursor = cursor.saturating_add(1),
            Some(']') => return Ok(values),
            Some(_) => return Err("array elements are not comma-separated".to_owned()),
            None => return Err("array is not terminated".to_owned()),
        }
    }
}

fn skip_toml_trivia(characters: &[char], cursor: &mut usize) {
    loop {
        while characters
            .get(*cursor)
            .is_some_and(|value| value.is_whitespace())
        {
            *cursor = cursor.saturating_add(1);
        }
        if characters.get(*cursor) != Some(&'#') {
            return;
        }
        while characters.get(*cursor).is_some_and(|value| *value != '\n') {
            *cursor = cursor.saturating_add(1);
        }
    }
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
    fn repeated_ledger_fields_are_findings_and_keep_the_first_value() {
        let parsed = parse_ledger(
            "[[exclusion]]\npath = \"first.rs\"\npath = \"second.rs\"\nsha256 = \"00\"\nkind = \"generated\"\nprovenance = \"data/manifest.toml\"\nreason = \"why\"\n\n[[equivalent]]\nmutant = \"first.rs:1\"\nexclude_re = '^first$'\nexclude_re = '^second$'\nsource_sha256 = \"11\"\nproof = \"same\"\n",
        );
        assert_eq!(
            parsed
                .violations
                .iter()
                .filter(|finding| finding.contains("stated more than once"))
                .count(),
            2
        );
        assert_eq!(parsed.exclusions[0].path, "first.rs");
        assert_eq!(parsed.equivalents[0].exclude_re, "^first$");
    }

    #[test]
    fn repeated_ledger_fields_are_findings_after_an_empty_first_value() {
        let parsed = parse_ledger(
            "[[exclusion]]\npath = \"\"\npath = \"replacement.rs\"\nsha256 = \"00\"\nkind = \"generated\"\nprovenance = \"data/manifest.toml\"\nreason = \"why\"\n",
        );
        assert!(
            parsed
                .violations
                .iter()
                .any(|finding| finding.contains("`path` is stated more than once"))
        );
        assert!(
            parsed
                .violations
                .iter()
                .any(|finding| finding.contains("incomplete [[exclusion]]"))
        );
        assert!(parsed.exclusions.is_empty());
    }

    #[test]
    fn config_arrays_ignore_formatting_and_reject_undocumented_regexes() {
        let expanded = "exclude_globs = [\n  \"crates/jlreq-core/src/generated/**\", # generated tables\n]\nexclude_re = [\n]\n";
        let mut valid = Vec::new();
        validate_config(Path::new("."), expanded, &[], &mut valid).unwrap();
        assert!(valid.is_empty(), "{valid:?}");

        let mismatched =
            "exclude_globs = [\"crates/jlreq-core/src/generated/**\"]\nexclude_re = ['^extra$']\n";
        let mut invalid = Vec::new();
        validate_config(Path::new("."), mismatched, &[], &mut invalid).unwrap();
        assert!(
            invalid
                .iter()
                .any(|finding| finding.contains("undocumented equivalent-mutant regex"))
        );
        assert!(
            invalid
                .iter()
                .any(|finding| finding.contains("has 1 equivalent regex"))
        );
    }

    #[test]
    fn digest_validation_rejects_every_unbound_state() {
        let root =
            std::env::temp_dir().join(format!("jlreq-xtask-mutation-{}", std::process::id()));
        fs::create_dir(&root).expect("create isolated digest fixture");

        let cases = [
            ("bad".to_owned(), "malformed SHA-256"),
            ("A".repeat(64), "non-lowercase SHA-256"),
        ];
        for (digest, expected) in cases {
            let mut findings = Vec::new();
            validate_digest(&root, "fixture.rs", &digest, "fixture", &mut findings).unwrap();
            assert!(
                findings.iter().any(|finding| finding.contains(expected)),
                "{findings:?}"
            );
        }

        let mut missing = Vec::new();
        validate_digest(
            &root,
            "missing.rs",
            &"0".repeat(64),
            "fixture",
            &mut missing,
        )
        .unwrap();
        assert!(
            missing
                .iter()
                .any(|finding| finding.contains("does not exist"))
        );

        fs::write(root.join("fixture.rs"), b"actual").expect("write digest fixture");
        let mut mismatch = Vec::new();
        validate_digest(
            &root,
            "fixture.rs",
            &"0".repeat(64),
            "fixture",
            &mut mismatch,
        )
        .unwrap();
        assert!(mismatch.iter().any(|finding| finding.contains("changed")));

        fs::remove_dir_all(root).expect("remove isolated digest fixture");
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
