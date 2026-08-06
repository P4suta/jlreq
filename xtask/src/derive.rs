// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `derive` gate: stage 1 of the specification data pipeline.
//!
//! Stage 1 reads the vendored specification snapshot under `spec/snapshot/` and writes the
//! tab-separated files under `spec/derived/`. Stage 2 is `generate`, which turns those
//! files into the Rust committed under `crates/*/src/generated/`. The two stages are two
//! gates rather than one because the intermediate table is itself a deliverable: a JLReq
//! reader can review `spec/derived/appendix-a.tsv` against Appendix A without reading a
//! line of Rust, which is the audience `docs/adr/0006` is written for.
//!
//! Both halves of the pipeline are byte-identity gates. `derive` writes every derived file
//! and then checks its own work; `derive --check` checks without writing. A hand edit to a
//! derived file is a bug even when it is correct, because the next revision of the
//! specification will not carry it forward (ADR 0009).
//!
//! Each derived file states, in its own comment header, every source it was read from and
//! that source's SHA-256. `spec/PROVENANCE.toml` records the same digests for the vendored
//! files, `data/manifest.toml` records the digest of every derived file as an input of
//! stage 2, and `attest --digests` verifies the vendored files against `PROVENANCE.toml`.
//! The chain from the published document to the emitted Rust is therefore digest-linked
//! end to end, with no step taken on trust.
//!
//! # Why the scanner lives here rather than in an excluded workspace
//!
//! An earlier revision of `docs/design/generation.md` put stage 1 in `tools/jlreq-gen`, a
//! workspace excluded from the root, on the reasoning that stage 1 "may parse HTML,
//! because it is not published and its dependency tree does not tax the crates that are".
//! The scanner written for this milestone parses the snapshot with `std` alone, in the
//! hand-rolled style the rest of this program is written in, so there is no dependency
//! tree to keep out of the workspace — and everything outside the workspace escapes
//! Clippy, `rustfmt`, `cargo-msrv` and, decisively, `cargo nextest`. Excluding the scanner
//! would have bought nothing and cost the tests that prove it reads the document's actual
//! shape. `docs/design/generation.md` records the change and the reasoning.
//!
//! Adding a derived file is adding one entry to `DERIVATIONS`; nothing else in this module
//! changes.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::Path;

use crate::classes;
use crate::defects;
use crate::generate::sha256;
use crate::inventory;
use crate::policy;
use crate::shared::{self, Gate};

/// The name this gate is invoked by.
const NAME: &str = "derive";

/// The `derive` gate, as the dispatcher sees it.
pub(crate) const GATE: Gate = Gate {
    name: NAME,
    purpose: concat!(
        "every file under spec/derived/ is the byte-identical reading of the vendored ",
        "specification snapshot, and states the digest of every source it was read from"
    ),
    reference: "docs/design/generation.md",
    run,
};

/// The directory holding the vendored upstream documents.
const SNAPSHOT_DIRECTORY: &str = "spec/snapshot";

/// The directory holding the tab-separated readings of them.
const DERIVED_DIRECTORY: &str = "spec/derived";

/// Where the provenance of every vendored document is recorded.
const PROVENANCE_PATH: &str = "spec/PROVENANCE.toml";

/// The path `spec/PROVENANCE.toml` records its own document paths relative to.
const PROVENANCE_ROOT: &str = "spec";

/// The module every derivation is driven by, and therefore a reader of every one of them.
///
/// A derived file's bytes are decided by the reader that produced its rows *and* by the
/// frame that writes its header, so both are named in that file's own provenance.
const FRAME: &str = "xtask/src/derive.rs";

/// The element the published rendering states its own publication date in.
const PUBLISHED_MARKER: &str = "<time class=\"dt-published\" datetime=\"";

/// What the subcommand was asked to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Write every derived file, then check the result like `Check` does.
    Emit,
    /// Write nothing and report every file regeneration would change.
    Check,
}

/// One derived file: the vendored sources it reads, where its rows go, and how the one
/// becomes the other.
///
/// Everything a derived file states about itself — that it is not to be edited, which
/// sources it was read from, and their digests — is written by this module and not by the
/// reader, so every derived file carries the same header whoever wrote the derivation.
#[derive(Debug)]
pub(crate) struct Derivation {
    /// The vendored sources, relative to the workspace root, with forward slashes, in the
    /// order the reader receives them.
    pub(crate) sources: &'static [&'static str],
    /// The `xtask` modules whose bytes decide what this derivation writes, beside the frame
    /// in this file, which is added to every one of them.
    ///
    /// A derived file states the digest of its sources so that a reader can tell which
    /// revision of the specification it is a reading of. It states the digest of its reader
    /// for the same reason and against the same failure: an editorial judgment lives in the
    /// reader rather than in the document — which `REMARKS` cell carries which frame, which
    /// heading closes a class name — so a file that named only its sources could change
    /// meaning entirely with every recorded provenance byte unchanged.
    pub(crate) reader: &'static [&'static str],
    /// The tab-separated file this derivation writes, relative to the workspace root.
    pub(crate) output: &'static str,
    /// One sentence naming what the rows below the header are, for a reader who opened the
    /// file rather than this module.
    pub(crate) caption: &'static str,
    /// Turns the text of every source, in `sources` order, into the header line and the
    /// records under it.
    pub(crate) read: fn(&[String]) -> Result<String, String>,
}

/// Every derived file, in the order `docs/design/generation.md` lists them.
pub(crate) const DERIVATIONS: &[Derivation] = &[
    classes::CLASSES,
    classes::APPENDIX_A,
    inventory::ANCHORS,
    inventory::NOTES,
    inventory::RULES,
    policy::QUESTIONS,
    defects::DEFECTS,
    classes::IDEOGRAPHS,
    classes::FOLDING,
    classes::SCRIPTS,
];

/// One derivation, rendered: the exact bytes its output file must contain.
#[derive(Debug)]
struct Rendered {
    /// Repository-relative path of the file the contents belong in.
    output: String,
    /// The complete file: the header this module writes, then the reader's rows.
    contents: String,
}

/// Read every derivation, write when asked to, and report what does not agree.
fn run(arguments: &[String]) -> io::Result<Vec<String>> {
    let mode = mode(arguments)?;
    let root = shared::workspace_root()?;
    let provenance = read_provenance(&root)?;

    let mut violations = check_declarations(DERIVATIONS, &provenance);
    check_snapshot(&root, &provenance, &mut violations)?;
    check_specification_date(&root, &provenance, &mut violations)?;
    let rendered = render(&root, DERIVATIONS, &mut violations)?;

    if mode == Mode::Emit && violations.is_empty() {
        emit(&root, &rendered)?;
    }

    check_outputs(&root, &rendered, &mut violations)?;
    check_strays(&root, DERIVATIONS, &mut violations)?;

    println!(
        "{NAME}: examined {count} derivation(s) reading {sources} vendored source(s) under \
         {SNAPSHOT_DIRECTORY}/, every one of them recorded in {PROVENANCE_PATH}",
        count = DERIVATIONS.len(),
        sources = distinct_sources(DERIVATIONS).len(),
    );
    Ok(violations)
}

/// One vendored document, as `spec/PROVENANCE.toml` records it.
#[derive(Debug)]
pub(crate) struct Recorded {
    /// The path, relative to the workspace root, with forward slashes.
    pub(crate) path: String,
    /// The date the upstream work itself carries, or the empty string when the record
    /// states none.
    published: String,
}

/// What `spec/PROVENANCE.toml` records.
#[derive(Debug)]
pub(crate) struct Provenance {
    /// The revision of JLReq every generated file states it was generated from.
    specification_date: String,
    /// One entry per `[[document]]` block, in file order.
    pub(crate) documents: Vec<Recorded>,
}

impl Provenance {
    /// Whether a repository path names a document this file records.
    fn records(&self, path: &str) -> bool {
        self.documents.iter().any(|document| document.path == path)
    }
}

/// Read the provenance record.
///
/// A hand-rolled scan for the reason `purity`'s manifest reader is one: the program that
/// enforces the layout core's empty dependency table declares none itself. It understands
/// the one shape this repository writes — a top-level `specification-date`, then one
/// `[[document]]` table per vendored file — and paths are recorded relative to `spec/`,
/// which is where this reader puts them back.
fn read_provenance(root: &Path) -> io::Result<Provenance> {
    let path = root.join(PROVENANCE_PATH);
    let text = fs::read_to_string(&path).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!(
                "{PROVENANCE_PATH}: {error}; every vendored document this repository reads is \
                 recorded there"
            ),
        )
    })?;

    let mut specification_date = String::new();
    let mut documents: Vec<Recorded> = Vec::new();
    let mut inside_document = false;
    let mut above_first_table = true;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            inside_document = line == "[[document]]";
            above_first_table = false;
            if inside_document {
                documents.push(Recorded {
                    path: String::new(),
                    published: String::new(),
                });
            }
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let (key, value) = (key.trim(), quoted(value).unwrap_or_default());
        if above_first_table && key == "specification-date" {
            specification_date.clear();
            specification_date.push_str(value);
        }
        if !inside_document {
            continue;
        }
        let Some(current) = documents.last_mut() else {
            continue;
        };
        match key {
            "path" => current.path = format!("{PROVENANCE_ROOT}/{value}"),
            "published" => {
                current.published.clear();
                current.published.push_str(value);
            },
            _ => {},
        }
    }

    if specification_date.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{PROVENANCE_PATH}: states no `specification-date` above its first table"),
        ));
    }
    Ok(Provenance {
        specification_date,
        documents,
    })
}

/// The first string literal on a line, without its quotes.
fn quoted(value: &str) -> Option<&str> {
    let (_, after) = value.split_once('"')?;
    let (inside, _) = after.split_once('"')?;
    Some(inside)
}

/// Report every file under `spec/snapshot/` that `spec/PROVENANCE.toml` does not record, and
/// every recorded snapshot document that is not there.
///
/// The two output directories of the pipeline are closed — `check_strays` reports a derived
/// file no derivation writes, and `generate`'s orphan scan reports a generated file no unit
/// writes — and the input directory is closed here, for the same reason. A vendored file no
/// digest covers is a source this repository could read while claiming, in every header
/// downstream, to have read only what `PROVENANCE.toml` records.
fn check_snapshot(
    root: &Path,
    provenance: &Provenance,
    violations: &mut Vec<String>,
) -> io::Result<()> {
    let directory = root.join(SNAPSHOT_DIRECTORY);
    let mut found: Vec<String> = Vec::new();
    walk(&directory, root, &mut found)?;
    found.sort();
    for path in &found {
        if !provenance.records(path) {
            violations.push(format!(
                "{path}: is vendored under `{SNAPSHOT_DIRECTORY}/` and \
                 `{PROVENANCE_PATH}` records no digest for it"
            ));
        }
    }
    for document in &provenance.documents {
        if document.path.starts_with(&format!("{SNAPSHOT_DIRECTORY}/"))
            && !found.contains(&document.path)
        {
            violations.push(format!(
                "{path}: recorded in `{PROVENANCE_PATH}` and not vendored",
                path = document.path
            ));
        }
    }
    Ok(())
}

/// Every file under one directory, recursively, named the way this repository names one.
fn walk(directory: &Path, root: &Path, found: &mut Vec<String>) -> io::Result<()> {
    if !directory.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            walk(&path, root, found)?;
        } else {
            found.push(repository_path(root, &path));
        }
    }
    Ok(())
}

/// Name a path relative to the workspace root, with forward slashes, so a run on Windows
/// reports what a run on Linux reports.
fn repository_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Require the revision every generated file claims to be generated from to be the one the
/// vendored rendering itself states.
///
/// `specification-date` is otherwise a free string copied into the header of every generated
/// file, checked against nothing. The published rendering carries its own publication date in
/// a `<time class="dt-published">` element, so the claim is checkable against the document it
/// is a claim about, which is what every other figure in this pipeline is held to.
fn check_specification_date(
    root: &Path,
    provenance: &Provenance,
    violations: &mut Vec<String>,
) -> io::Result<()> {
    let snapshot = format!("{SNAPSHOT_DIRECTORY}/index.html");
    let path = root.join(&snapshot);
    if !path.is_file() {
        return Ok(());
    }
    let text = fs::read_to_string(&path)?;
    let Some(rendered) = text
        .split_once(PUBLISHED_MARKER)
        .and_then(|(_, rest)| rest.split_once('"'))
        .map(|(date, _)| date)
    else {
        violations.push(format!(
            "{snapshot}: states no publication date of its own, so `specification-date` \
             could not be checked against the document it describes"
        ));
        return Ok(());
    };
    if rendered != provenance.specification_date {
        violations.push(format!(
            "{PROVENANCE_PATH}: states `specification-date = \"{stated}\"` where {snapshot} \
             publishes {rendered}; every generated file carries that string as the revision \
             of JLReq it was generated from",
            stated = provenance.specification_date,
        ));
    }
    for document in &provenance.documents {
        if document.path == snapshot && document.published != rendered {
            violations.push(format!(
                "{PROVENANCE_PATH}: records `published = \"{published}\"` for {snapshot}, \
                 which publishes {rendered}",
                published = document.published,
            ));
        }
    }
    Ok(())
}

/// Read the one argument this gate accepts.
///
/// An unrecognized argument is refused rather than ignored: a caller who wrote `--dry-run`
/// and saw a pass would believe nothing had been written.
fn mode(arguments: &[String]) -> io::Result<Mode> {
    match arguments {
        [] => Ok(Mode::Emit),
        [only] if only == "--check" => Ok(Mode::Check),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "usage: `{NAME}` to write the derived files, `{NAME} --check` to prove \
                 rereading the snapshot would change none of them; got `{given}`",
                given = arguments.join(" ")
            ),
        )),
    }
}

/// The distinct vendored sources every declared derivation reads.
pub(crate) fn distinct_sources(derivations: &[Derivation]) -> BTreeSet<&'static str> {
    derivations
        .iter()
        .flat_map(|derivation| derivation.sources.iter().copied())
        .collect()
}

/// The distinct `xtask` modules every declared derivation is read by, the frame included.
///
/// Read by `generate`, which records a digest for each of them in `data/manifest.toml`: the
/// manifest is this repository's one ledger of what produced what, and stage 1's readers
/// decide the bytes of stage 2's inputs.
pub(crate) fn distinct_readers(derivations: &[Derivation]) -> BTreeSet<&'static str> {
    let mut modules: BTreeSet<&'static str> = derivations
        .iter()
        .flat_map(|derivation| derivation.reader.iter().copied())
        .collect();
    modules.insert(FRAME);
    modules
}

/// Reject a derivation whose declaration the rest of this gate could not police.
///
/// The scan for stray files reads `spec/derived/`, so a derivation writing anywhere else
/// would be byte-checked while a hand-written file beside it went unnoticed; a derivation
/// reading outside `spec/snapshot/` would be reading something no digest covers; and two
/// derivations claiming one path would let the order they are listed in decide what that
/// path ends up holding.
fn check_declarations(derivations: &[Derivation], provenance: &Provenance) -> Vec<String> {
    let mut violations = Vec::new();
    let mut claimed = BTreeSet::new();
    for derivation in derivations {
        for source in derivation.sources {
            if !source.starts_with(&format!("{SNAPSHOT_DIRECTORY}/")) {
                violations.push(format!(
                    "{source}: a derivation reads only the vendored documents under \
                     `{SNAPSHOT_DIRECTORY}/`, which are the ones `spec/PROVENANCE.toml` \
                     records a digest for"
                ));
            } else if !provenance.records(source) {
                violations.push(format!(
                    "{source}: declared as a source of `{output}` and not recorded in \
                     `{PROVENANCE_PATH}`, so nothing states what it is or where it came from",
                    output = derivation.output
                ));
            }
        }
        for module in derivation.reader {
            if !is_xtask_module(module) {
                violations.push(format!(
                    "{module}: a derivation's reader is an `xtask` module, and its digest is \
                     what makes `{output}` state which reader produced it",
                    output = derivation.output
                ));
            }
        }
        if derivation.reader.is_empty() {
            violations.push(format!(
                "{output}: names no reader, so its header could not state what read it",
                output = derivation.output
            ));
        }
        if derivation.sources.is_empty() {
            violations.push(format!(
                "{output}: a derivation names no source, so nothing states what it is a \
                 reading of",
                output = derivation.output
            ));
        }
        if !is_derived_table(derivation.output) {
            violations.push(format!(
                "{output}: a derivation writes only `{DERIVED_DIRECTORY}/<name>.tsv`",
                output = derivation.output
            ));
        }
        if !claimed.insert(derivation.output) {
            violations.push(format!(
                "{output}: claimed by two derivations, so which one writes it would depend \
                 on the order they are declared in",
                output = derivation.output
            ));
        }
    }
    violations
}

/// Whether a repository path names a Rust module of this program.
fn is_xtask_module(path: &str) -> bool {
    path.starts_with("xtask/src/")
        && path
            .strip_suffix(".rs")
            .is_some_and(|stem| !stem.is_empty())
}

/// Whether a repository path names a tab-separated file directly inside `spec/derived/`.
fn is_derived_table(path: &str) -> bool {
    let parts: Vec<&str> = path.split('/').collect();
    match parts.as_slice() {
        ["spec", "derived", name] => name
            .rsplit_once('.')
            .is_some_and(|(stem, extension)| !stem.is_empty() && extension == "tsv"),
        _ => false,
    }
}

/// Render every derivation into the bytes its output file must hold.
///
/// A derivation that cannot be rendered — an absent source, a source that is not UTF-8, a
/// reader that refuses — is a violation and not an aborted run, so one unreadable table
/// does not hide the state of the others.
fn render(
    root: &Path,
    derivations: &[Derivation],
    violations: &mut Vec<String>,
) -> io::Result<Vec<Rendered>> {
    let mut rendered = Vec::new();
    for derivation in derivations {
        if let Some(one) = render_one(root, derivation, violations)? {
            rendered.push(one);
        }
    }
    Ok(rendered)
}

/// Render one derivation, or explain why it could not be rendered.
fn render_one(
    root: &Path,
    derivation: &Derivation,
    violations: &mut Vec<String>,
) -> io::Result<Option<Rendered>> {
    let mut texts = Vec::new();
    let mut digests = Vec::new();
    for source in derivation.sources {
        let path = root.join(source);
        if !path.is_file() {
            violations.push(format!(
                "{source}: declared as a source of `{output}` and not present",
                output = derivation.output
            ));
            return Ok(None);
        }
        let bytes = fs::read(&path)?;
        digests.push(sha256(&bytes));
        let Ok(text) = String::from_utf8(bytes) else {
            violations.push(format!("{source}: is not UTF-8"));
            return Ok(None);
        };
        texts.push(text);
    }

    let Some(reader) = reader_digest(root, derivation.reader, violations)? else {
        return Ok(None);
    };

    let rows = match (derivation.read)(&texts) {
        Ok(rows) => rows,
        Err(reason) => {
            violations.push(format!("{output}: {reason}", output = derivation.output));
            return Ok(None);
        },
    };

    let contents = compose(derivation, &digests, &reader, &rows);
    if contents.contains('\r') {
        violations.push(format!(
            "{output}: holds a carriage return; the specification data is LF, and \
             `.gitattributes` keeps the whole tree that way",
            output = derivation.output
        ));
        return Ok(None);
    }

    Ok(Some(Rendered {
        output: derivation.output.to_owned(),
        contents,
    }))
}

/// Compose the complete derived file: the header every one of them carries, then the rows.
///
/// The header is what makes the file auditable without running anything — it names every
/// source and that source's digest, and it says outright that the file is not to be
/// edited. Nothing in it comes from the clock or the environment, so two runs of the same
/// reader over the same snapshot produce the same bytes.
fn compose(derivation: &Derivation, digests: &[String], reader: &Reader, rows: &str) -> String {
    let mut header = format!(
        "# {output}\n\
         #\n\
         # Generated by `cargo run -p xtask -- derive`. Do not edit. `derive --check`\n\
         # fails when rereading the snapshot would change a byte, and a hand edit is a bug\n\
         # even when it is correct, because the next revision of the specification will\n\
         # not carry it forward (ADR 0009).\n\
         #\n\
         # {caption}\n\
         #\n",
        output = derivation.output,
        caption = derivation.caption,
    );
    let provenance: Vec<String> = derivation
        .sources
        .iter()
        .zip(digests)
        .map(|(source, digest)| format!("# Source: {source}\n# Source SHA-256: {digest}\n"))
        .collect();
    header.push_str(&provenance.concat());
    header.push_str("# Reader: ");
    header.push_str(&reader.modules.join(", "));
    header.push_str("\n# Reader SHA-256: ");
    header.push_str(&reader.digest);
    header.push_str("\n\n");
    header.push_str(rows.trim_end_matches('\n'));
    header.push('\n');
    header
}

/// The modules that decided a derived file's bytes, and their digest together.
#[derive(Debug)]
pub(crate) struct Reader {
    /// The modules, sorted, each named the way this repository names a path.
    pub(crate) modules: Vec<String>,
    /// One digest over all of them.
    pub(crate) digest: String,
}

/// The digest of the modules that decide what one derivation writes.
///
/// The frame in this file is added to whatever the derivation declares, because the header
/// is written here and a change to it changes every derived file. The digest is taken over
/// the paths as well as the bytes, so renaming a reader is a change and not a coincidence.
fn reader_digest(
    root: &Path,
    modules: &[&'static str],
    violations: &mut Vec<String>,
) -> io::Result<Option<Reader>> {
    let mut named: BTreeSet<&str> = modules.iter().copied().collect();
    named.insert(FRAME);
    let mut ledger = String::new();
    let mut listed = Vec::new();
    for module in named {
        let path = root.join(module);
        if !path.is_file() {
            violations.push(format!(
                "{module}: declared as a reader and not present, so no digest states what \
                 produced the file it reads into"
            ));
            return Ok(None);
        }
        ledger.push_str(module);
        ledger.push(' ');
        ledger.push_str(&sha256(&fs::read(&path)?));
        ledger.push('\n');
        listed.push(module.to_owned());
    }
    Ok(Some(Reader {
        digest: sha256(ledger.as_bytes()),
        modules: listed,
    }))
}

/// Write every rendered file.
///
/// Called only when every derivation rendered, so a run that could not read one table
/// never leaves a half-written tree behind for someone to commit.
fn emit(root: &Path, rendered: &[Rendered]) -> io::Result<()> {
    for one in rendered {
        let path = root.join(&one.output);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, one.contents.as_bytes())?;
    }
    Ok(())
}

/// Report every rendered output the repository does not hold, byte for byte.
fn check_outputs(
    root: &Path,
    rendered: &[Rendered],
    violations: &mut Vec<String>,
) -> io::Result<()> {
    for one in rendered {
        let path = root.join(&one.output);
        let on_disk = if path.is_file() {
            Some(fs::read(&path)?)
        } else {
            None
        };
        if let Some(violation) = difference(&one.output, on_disk.as_deref(), &one.contents) {
            violations.push(violation);
        }
    }
    Ok(())
}

/// What one output file earns against the bytes rereading the snapshot produces, if
/// anything.
fn difference(output: &str, on_disk: Option<&[u8]>, regenerated: &str) -> Option<String> {
    let Some(on_disk) = on_disk else {
        return Some(format!(
            "{output}: the snapshot generates this file and it is not present; run \
             `cargo run -p xtask -- derive`"
        ));
    };
    if on_disk == regenerated.as_bytes() {
        return None;
    }
    Some(format!(
        "{output}: rereading the snapshot would change it. On disk {before_length} byte(s), \
         SHA-256 {before}; regenerated {after_length} byte(s), SHA-256 {after}. A derived \
         file is never hand-edited (ADR 0009); run `cargo run -p xtask -- derive`",
        before_length = on_disk.len(),
        before = sha256(on_disk),
        after_length = regenerated.len(),
        after = sha256(regenerated.as_bytes()),
    ))
}

/// Report every file under `spec/derived/` no derivation writes.
///
/// Discovered rather than declared, for the reason the `generate` gate scans the
/// `generated` directories: a table nobody derives is exactly what this gate is looking
/// for, so the scan cannot be driven by the list of derivations.
fn check_strays(
    root: &Path,
    derivations: &[Derivation],
    violations: &mut Vec<String>,
) -> io::Result<()> {
    let directory = root.join(DERIVED_DIRECTORY);
    if !directory.is_dir() {
        return Ok(());
    }
    let claimed: BTreeSet<&str> = derivations
        .iter()
        .map(|derivation| derivation.output)
        .collect();
    let mut names: Vec<String> = Vec::new();
    for entry in fs::read_dir(&directory)? {
        let entry = entry?;
        names.push(entry.file_name().to_string_lossy().into_owned());
    }
    names.sort();
    for name in names {
        let path = format!("{DERIVED_DIRECTORY}/{name}");
        if !claimed.contains(path.as_str()) {
            violations.push(format!(
                "{path}: no derivation writes this file, and everything under \
                 `{DERIVED_DIRECTORY}/` is a reading of the vendored snapshot (ADR 0009)"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        DERIVATIONS, Derivation, Mode, Provenance, Reader, Recorded, check_declarations, compose,
        difference, distinct_readers, distinct_sources, is_derived_table, mode, read_provenance,
        sha256,
    };
    use crate::shared;
    use std::fs;

    /// A derivation declaration whose reader is never reached by these tests.
    fn derivation(sources: &'static [&'static str], output: &'static str) -> Derivation {
        Derivation {
            sources,
            reader: &["xtask/src/classes.rs"],
            output,
            caption: "A fixture.",
            read: |_| Err("a fixture reader reads nothing".to_owned()),
        }
    }

    /// A provenance record naming the vendored documents these tests refer to.
    fn provenance() -> Provenance {
        Provenance {
            specification_date: "2020-08-11".to_owned(),
            documents: ["spec/snapshot/index.html", "spec/snapshot/ucd/PropList.txt"]
                .into_iter()
                .map(|path| Recorded {
                    path: path.to_owned(),
                    published: "2020-08-11".to_owned(),
                })
                .collect(),
        }
    }

    #[test]
    fn the_gate_takes_check_and_refuses_anything_else() {
        assert_eq!(mode(&[]).ok(), Some(Mode::Emit));
        assert_eq!(mode(&["--check".to_owned()]).ok(), Some(Mode::Check));
        assert!(
            mode(&["--dry-run".to_owned()]).is_err(),
            "an unrecognized argument must not be read as a pass"
        );
    }

    #[test]
    fn a_derived_table_sits_directly_in_the_derived_directory() {
        assert!(is_derived_table("spec/derived/appendix-a.tsv"));
        assert!(!is_derived_table("spec/derived/nested/appendix-a.tsv"));
        assert!(!is_derived_table("spec/captured/table1.en.tsv"));
        assert!(
            !is_derived_table("spec/derived/appendix-a.csv"),
            "the format is tab-separated, which is what `generate` reads"
        );
    }

    #[test]
    fn a_derivation_reading_outside_the_snapshot_is_rejected() {
        let violations = check_declarations(
            &[derivation(
                &["docs/design/generation.md"],
                "spec/derived/appendix-a.tsv",
            )],
            &provenance(),
        );
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert!(
            violations
                .first()
                .is_some_and(|violation| violation.contains("spec/snapshot/")),
            "{violations:?}"
        );
    }

    #[test]
    fn a_derivation_reading_a_snapshot_file_no_provenance_records_is_rejected() {
        let violations = check_declarations(
            &[derivation(
                &["spec/snapshot/smuggled.txt"],
                "spec/derived/appendix-a.tsv",
            )],
            &provenance(),
        );
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert!(
            violations
                .first()
                .is_some_and(|violation| violation.contains("PROVENANCE.toml")),
            "a source no digest covers is one nothing states the origin of: {violations:?}"
        );
    }

    #[test]
    fn a_derivation_writing_outside_the_derived_directory_is_rejected() {
        let violations = check_declarations(
            &[derivation(
                &["spec/snapshot/index.html"],
                "crates/jlreq-class/src/generated/appendix_a.rs",
            )],
            &provenance(),
        );
        assert_eq!(violations.len(), 1, "{violations:?}");
    }

    #[test]
    fn a_derivation_naming_no_source_is_rejected() {
        let violations = check_declarations(
            &[derivation(&[], "spec/derived/appendix-a.tsv")],
            &provenance(),
        );
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert!(
            violations
                .first()
                .is_some_and(|violation| violation.contains("names no source")),
            "{violations:?}"
        );
    }

    #[test]
    fn a_derivation_naming_no_reader_is_rejected() {
        let mut declared = derivation(&["spec/snapshot/index.html"], "spec/derived/folding.tsv");
        declared.reader = &[];
        let violations = check_declarations(&[declared], &provenance());
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert!(
            violations
                .first()
                .is_some_and(|violation| violation.contains("names no reader")),
            "a file whose header could not state what read it is one whose meaning nothing \
             records: {violations:?}"
        );
    }

    #[test]
    fn a_reader_that_is_not_a_module_of_this_program_is_rejected() {
        let mut declared = derivation(&["spec/snapshot/index.html"], "spec/derived/folding.tsv");
        declared.reader = &["docs/design/generation.md"];
        let violations = check_declarations(&[declared], &provenance());
        assert_eq!(violations.len(), 1, "{violations:?}");
    }

    #[test]
    fn two_derivations_writing_one_file_are_rejected() {
        let output = "spec/derived/appendix-a.tsv";
        let violations = check_declarations(
            &[
                derivation(&["spec/snapshot/index.html"], output),
                derivation(&["spec/snapshot/ucd/PropList.txt"], output),
            ],
            &provenance(),
        );
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert!(
            violations
                .first()
                .is_some_and(|violation| violation.contains("two derivations")),
            "{violations:?}"
        );
    }

    #[test]
    fn the_declared_derivations_are_well_formed() {
        let root = shared::workspace_root().expect("the workspace root");
        let recorded = read_provenance(&root).expect("spec/PROVENANCE.toml is readable");
        assert!(
            check_declarations(DERIVATIONS, &recorded).is_empty(),
            "the derivations this repository declares must satisfy the gate that reads them"
        );
        assert!(
            !distinct_sources(DERIVATIONS).is_empty(),
            "every derivation reads the vendored snapshot"
        );
        assert!(
            distinct_readers(DERIVATIONS).contains(super::FRAME),
            "the frame writes every derived file's header, so it reads every one of them"
        );
    }

    /// The reader a composed fixture states.
    fn reader() -> Reader {
        Reader {
            modules: vec!["xtask/src/derive.rs".to_owned()],
            digest: "89abcdef".to_owned(),
        }
    }

    #[test]
    fn a_derived_file_states_every_source_and_its_digest_and_forbids_editing() {
        let declared = derivation(
            &["spec/snapshot/index.html", "spec/snapshot/ucd/PropList.txt"],
            "spec/derived/appendix-a.tsv",
        );
        let digests = ["0123abc".to_owned(), "4567def".to_owned()];
        let file = compose(&declared, &digests, &reader(), "class\tkey\ncl-01\t0028\n");

        assert!(file.starts_with("# spec/derived/appendix-a.tsv\n"));
        assert!(file.contains("Do not edit."), "{file}");
        assert!(file.contains("# A fixture.\n"), "{file}");
        assert!(file.contains("# Source: spec/snapshot/index.html\n"));
        assert!(file.contains("# Source SHA-256: 0123abc\n"));
        assert!(file.contains("# Source: spec/snapshot/ucd/PropList.txt\n"));
        assert!(file.contains("# Source SHA-256: 4567def\n"));
        assert!(
            file.contains("# Reader: xtask/src/derive.rs\n# Reader SHA-256: 89abcdef\n"),
            "the reader is provenance too: an editorial judgment lives in it rather than in \
             the document, so a file naming only its sources overstates what its digests \
             cover: {file}"
        );
        assert!(file.ends_with("class\tkey\ncl-01\t0028\n"), "{file}");
        assert!(!file.contains('\r'), "the specification data is LF");
    }

    #[test]
    fn the_same_snapshot_composes_the_same_bytes_twice() {
        let declared = derivation(&["spec/snapshot/index.html"], "spec/derived/folding.tsv");
        let digests = ["abc".to_owned()];
        assert_eq!(
            compose(&declared, &digests, &reader(), "a\tb\n1\t2\n"),
            compose(&declared, &digests, &reader(), "a\tb\n1\t2\n"),
            "nothing in a derived file may come from the clock or the environment"
        );
    }

    #[test]
    fn a_byte_of_difference_is_a_violation() {
        let rows = "class\tkey\ncl-01\t0028\n";
        assert_eq!(difference("out.tsv", Some(rows.as_bytes()), rows), None);
        let violation = difference("out.tsv", Some(b"class\tkey\ncl-02\t0029\n"), rows)
            .expect("one row apart is a difference");
        assert!(violation.contains("would change it"), "{violation}");
        let absent = difference("out.tsv", None, rows).expect("an absent output is a failure");
        assert!(absent.contains("not present"), "{absent}");
    }

    #[test]
    fn the_gate_holds_over_this_repository() {
        // The gate run over the tree it ships with, so `just test` exercises the real
        // pipeline against the real snapshot rather than against fixtures that agree with
        // the reader by construction. `derive --check` is the only check that binds
        // spec/derived/ to the vendored document, and a gate whose only exercise is its own
        // fixtures is a gate that cannot fail for the reason it exists.
        let violations = super::run(&["--check".to_owned()]).expect("the gate can run");
        assert!(
            violations.is_empty(),
            "rereading the vendored snapshot changes a derived file: {violations:?}"
        );
    }

    #[test]
    fn a_hand_edited_derived_table_is_refused_and_names_both_digests() {
        // The silent-drop proof. Every stage-2 gate agrees with a hand-edited derived table,
        // because the generated Rust and the table agree with each other; only rereading the
        // snapshot disagrees. This is that reading, over the committed file, one row apart.
        let root = shared::workspace_root().expect("the workspace root");
        let path = root.join("spec/derived/classes.tsv");
        let committed = fs::read_to_string(&path).expect("the derived table is readable");
        let edited = committed.replace(
            "cl-01\tOpening brackets",
            "cl-01\tFabricated name for a class",
        );
        assert_ne!(edited, committed, "the fixture edits the row it names");
        let violation = difference(
            "spec/derived/classes.tsv",
            Some(edited.as_bytes()),
            &committed,
        )
        .expect("a table the snapshot does not generate is a violation");
        assert!(
            violation.contains("rereading the snapshot would change it"),
            "{violation}"
        );
        assert!(
            violation.contains(&sha256(edited.as_bytes())),
            "the report names the digest on disk: {violation}"
        );
        assert!(
            violation.contains(&sha256(committed.as_bytes())),
            "and the digest rereading produces: {violation}"
        );
    }

    #[test]
    fn the_recorded_provenance_covers_the_vendored_snapshot() {
        let root = shared::workspace_root().expect("the workspace root");
        let recorded = read_provenance(&root).expect("spec/PROVENANCE.toml is readable");
        let mut violations = Vec::new();
        super::check_snapshot(&root, &recorded, &mut violations).expect("the scan can run");
        assert!(
            violations.is_empty(),
            "every file vendored under spec/snapshot/ is one PROVENANCE.toml records: \
             {violations:?}"
        );
        let mut dated = Vec::new();
        super::check_specification_date(&root, &recorded, &mut dated).expect("the check can run");
        assert!(
            dated.is_empty(),
            "the revision every generated file claims is the one the rendering publishes: \
             {dated:?}"
        );
    }

    #[test]
    fn a_vendored_file_no_provenance_records_is_reported() {
        let recorded = Provenance {
            specification_date: "2020-08-11".to_owned(),
            documents: Vec::new(),
        };
        let root = shared::workspace_root().expect("the workspace root");
        let mut violations = Vec::new();
        super::check_snapshot(&root, &recorded, &mut violations).expect("the scan can run");
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains("spec/snapshot/index.html")),
            "the input directory is closed the way both output directories are: {violations:?}"
        );
    }

    #[test]
    fn a_specification_date_the_document_does_not_publish_is_reported() {
        let recorded = Provenance {
            specification_date: "2099-01-01".to_owned(),
            documents: Vec::new(),
        };
        let root = shared::workspace_root().expect("the workspace root");
        let mut violations = Vec::new();
        super::check_specification_date(&root, &recorded, &mut violations)
            .expect("the check can run");
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains("2099-01-01")),
            "the revision every generated file states is held against the rendering that \
             publishes it: {violations:?}"
        );
    }
}
