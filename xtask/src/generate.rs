// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `generate` gate: stage 2 of the specification data pipeline.
//!
//! Stage 1 is the `derive` gate, which reads the vendored snapshot and emits
//! `spec/derived/*.tsv`. Stage 2 is this module: it turns the tab-separated files under
//! `spec/derived/` and `spec/captured/` into the Rust modules committed at
//! `crates/jlreq/src/generated/*.rs`. Tab-separated text needs no dependency to read, which is
//! what keeps this program's empty dependency table intact, and the SHA-256 below is
//! hand-written for the same reason.
//!
//! `generate` writes every generated file and then checks its own work; `generate --check`
//! checks without writing. The comparison is byte identity rather than semantic
//! equivalence, so a hand edit to a generated table is caught the moment it is committed,
//! and it is a bug even when it is correct, because the next revision of the specification
//! will not carry it forward (ADR 0009).
//!
//! Three things are checked, and the second and third bind the repository as it stands
//! today:
//!
//! - every declared unit's output is exactly what its input generates, byte for byte, and
//!   a unit whose output file is absent is as much a failure as one whose output differs;
//! - every file under a `generated` directory is claimed by some unit, so a hand-written
//!   one cannot hide among the machine-written ones — every file and not every `.rs` file,
//!   because a binary table pulled in with `include_bytes!` is generated data too;
//! - `data/manifest.toml` records the SHA-256 of every file the pipeline reads or writes,
//!   and of nothing else, so a hand edit to an input that was never regenerated from is
//!   visible too. That is wider than the units' own outputs and inputs: it covers the
//!   derived tables that `attest`, `api` and `conform` read, the vendored documents behind
//!   all of them, and the modules
//!   of this program that emit them. A ledger that records part of a chain records nothing
//!   about the rest of it.
//!
//! The gate reports how much it examined rather than claiming to have proved something,
//! because `UNITS` covers the tables written so far and not the ones still to arrive.
//! Adding a generated table is adding one entry to `UNITS`; nothing else in this module
//! changes, and the generator itself lives in the module that owns the subject.
//!
//! Stage 1 is `derive`, which reads the vendored snapshot and writes the tab-separated
//! files this gate reads. The two together are the pipeline, and both are byte-identity
//! gates.
//!
//! Three conventions are pinned here, because a generated file has to agree with the tool
//! that writes it:
//!
//! - the module declaration for a `generated` directory belongs in `src/generated.rs`
//!   beside the directory, never in `src/generated/mod.rs`, because every Rust file
//!   *inside* the directory is machine-written and this gate says so;
//! - `spec/PROVENANCE.toml` states `specification-date` at its top level, above the first
//!   table, naming the revision of JLReq every generated file was generated from;
//! - `data/manifest.toml` is an array of `[[file]]` tables sorted by path, each holding a
//!   `path` and a `sha256`, written by this gate and read back by it.
//!
//! See `docs/design/generation.md` and `docs/adr/0009`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::Path;

use crate::classes;
use crate::derive;
use crate::inventory;
use crate::policy;
use crate::shared::{self, Gate};
use crate::spacing;

/// The name this gate is invoked by.
const NAME: &str = "generate";

/// The `generate` gate, as the dispatcher sees it.
pub(crate) const GATE: Gate = Gate {
    name: NAME,
    purpose: concat!(
        "every generated module is the byte-identical output of a generation unit, ",
        "and data/manifest.toml records what each one was generated from"
    ),
    reference: "docs/design/generation.md",
    run,
};

/// The directory holding the crates a generated module may live in.
const CRATES_DIRECTORY: &str = "crates";

/// The directory name that marks machine-written Rust inside a crate.
const GENERATED_DIRECTORY: &str = "generated";

/// The directory holding the tab-separated inputs, derived and captured alike.
const SPEC_DIRECTORY: &str = "spec";

/// Where the digest of every generated file and every input is recorded.
const MANIFEST_PATH: &str = "data/manifest.toml";

/// Where the snapshot records what it is a snapshot of.
const PROVENANCE_PATH: &str = "spec/PROVENANCE.toml";

/// The key naming the revision of JLReq the snapshot was taken from.
const SPECIFICATION_DATE_KEY: &str = "specification-date";

/// The module every generation unit is driven by, and therefore a generator of every one of
/// them: the header below is written here, so a change to it changes every emitted file.
const FRAME: &str = "xtask/src/generate.rs";

/// The copyright line every emitted file carries.
///
/// A constant rather than the clock: the output must be byte-identical on every run, so
/// nothing this gate writes may depend on the day it ran.
const COPYRIGHT: &str = "2026 jlreq contributors";

/// The license identifier every emitted file carries.
const LICENSE: &str = "MIT OR Apache-2.0";

/// What the subcommand was asked to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Write every generated file, then check the result like `Check` does.
    Emit,
    /// Write nothing and report every file regeneration would change.
    Check,
}

/// One generated file: where its data comes from, where its Rust goes, and how the one
/// becomes the other.
///
/// The input is a tab-separated file under `spec/`, the output a Rust module in a crate's
/// `generated` directory, and the generator a function from the records of the first to
/// the items of the second. Everything a generated file states about itself — its license,
/// its source, that source's digest, the specification revision, the generator and its
/// digest, and the entry count — is written by this module and not by the generator, so
/// every generated file in the repository carries the same header whoever wrote the unit.
#[derive(Debug)]
pub(crate) struct Unit {
    /// The tab-separated input, relative to the workspace root, with forward slashes.
    pub(crate) input: &'static str,
    /// The `xtask` modules whose bytes decide what this unit writes, beside the frame in
    /// this file, which is added to every one of them.
    ///
    /// A generated file states the digest of its input so that a reader can tell which data
    /// it was generated from. It states the digest of its generator for the same reason and
    /// against the same failure: the semantic columns of a generated table are the
    /// generator's reading of the source rather than a column of it — which frame a Remarks
    /// cell permits, which role it names — so a file that named only its input could change
    /// the meaning of every row with every recorded provenance byte unchanged.
    pub(crate) generator: &'static [&'static str],
    /// The Rust module this unit writes, relative to the workspace root.
    pub(crate) output: &'static str,
    /// The first line of the emitted module's documentation, which is the one sentence a
    /// reader of the generated file meets before the header explains where it came from.
    pub(crate) summary: &'static str,
    /// Turns the records of the input into the items of the output.
    pub(crate) emit: fn(&Table) -> Result<Emission, String>,
}

/// Every generated file, in the order `docs/design/generation.md` lists them.
///
/// A unit's generator lives in the module that owns the subject rather than here, so that
/// adding a generated table touches this list and nothing else in this file. The units
/// still to arrive are the appendix notes and the captured matrices.
const UNITS: &[Unit] = &[
    classes::UNIFIED_APPENDIX_A_TABLE,
    classes::UNIFIED_IDEOGRAPH_TABLE,
    classes::UNIFIED_FOLDING_TABLE,
    classes::UNIFIED_SCRIPT_TABLE,
    spacing::UNIFIED_TABLE1,
    spacing::UNIFIED_TABLE2,
    spacing::UNIFIED_TABLE3,
    spacing::UNIFIED_TABLE4,
    spacing::UNIFIED_TABLE5,
    spacing::UNIFIED_TABLE6,
];

/// Superseded Rust outputs whose declarations remain compiled but are no longer rendered.
/// The derivation code they share with the unified outputs remains part of `xtask`.
const HISTORICAL_UNITS: &[Unit] = &[
    classes::APPENDIX_A_TABLE,
    classes::CLASS_TABLE,
    classes::IDEOGRAPH_TABLE,
    classes::FOLDING_TABLE,
    classes::SCRIPT_TABLE,
    inventory::RULE_INVENTORY,
    policy::POLICY_SPACE_UNIT,
    spacing::TABLE1,
    spacing::TABLE2,
    spacing::TABLE3,
    spacing::TABLE4,
    spacing::TABLE5,
    spacing::TABLE6,
];

/// A tab-separated input, as a generator sees it.
#[derive(Debug)]
pub(crate) struct Table {
    /// Repository-relative name of the file the records were read from, for messages.
    pub(crate) source: String,
    /// The names on the header line, in order.
    pub(crate) columns: Vec<String>,
    /// Every record under the header line, in file order.
    pub(crate) records: Vec<Record>,
}

/// One record of a tab-separated input.
#[derive(Debug)]
pub(crate) struct Record {
    /// The one-based line the record was read from, so a rejection names it.
    pub(crate) line: usize,
    /// The fields, one per column of the header line.
    pub(crate) fields: Vec<String>,
}

/// What a generator produced.
#[derive(Debug)]
pub(crate) struct Emission {
    /// The items of the module, without the header this module writes.
    pub(crate) items: String,
    /// How many entries those items define.
    ///
    /// Stated by the generator rather than counted from the input, because the two differ
    /// wherever the published table repeats a key: §A.19 lists 465 rows for 464 members,
    /// which is a recorded defect and not a license to conflate the two counts.
    pub(crate) entries: usize,
}

/// One unit, rendered: the exact bytes its output file must contain.
#[derive(Debug)]
struct Rendered {
    /// Repository-relative path of the input the contents were rendered from.
    input: String,
    /// Repository-relative path of the file the contents belong in.
    output: String,
    /// The complete file: the header this module writes, then the generator's items.
    contents: String,
    /// The SHA-256 of the input, in lowercase hexadecimal.
    input_digest: String,
}

/// The modules that decided a generated file's bytes, and their digest together.
#[derive(Debug)]
struct Generator {
    /// The modules, sorted, each named the way this repository names a path.
    modules: Vec<String>,
    /// One digest over all of them.
    digest: String,
}

/// The digest of the modules that decide what one unit writes.
///
/// The frame in this file is added to whatever the unit declares. The digest is taken over
/// the paths as well as the bytes, so renaming a generator is a change and not a
/// coincidence, and it replaces the version string an earlier revision of this header
/// carried: `env!("CARGO_PKG_VERSION")` is the shared workspace version, which moves on a
/// release and never on a change to a generator, so it was churn where information was
/// wanted (`docs/design/generation.md`).
fn generator_digest(
    root: &Path,
    modules: &[&'static str],
    violations: &mut Vec<String>,
) -> io::Result<Option<Generator>> {
    let mut named: BTreeSet<&str> = modules.iter().copied().collect();
    named.insert(FRAME);
    let mut ledger = String::new();
    let mut listed = Vec::new();
    for module in named {
        let path = root.join(module);
        if !path.is_file() {
            violations.push(format!(
                "{module}: declared as a generator and not present, so no digest states what \
                 emitted the file it writes"
            ));
            return Ok(None);
        }
        ledger.push_str(module);
        ledger.push(' ');
        ledger.push_str(&sha256(&fs::read(&path)?));
        ledger.push('\n');
        listed.push(module.to_owned());
    }
    Ok(Some(Generator {
        digest: sha256(ledger.as_bytes()),
        modules: listed,
    }))
}

/// Render every unit, write when asked to, and report what does not agree.
fn run(arguments: &[String]) -> io::Result<Vec<String>> {
    let mode = mode(arguments)?;
    let root = shared::workspace_root()?;

    let mut violations = check_declarations(UNITS);
    violations.extend(check_declarations(HISTORICAL_UNITS));
    let rendered = render(&root, UNITS, &mut violations)?;
    let digests = digests(&root, &rendered, &mut violations)?;

    if mode == Mode::Emit && violations.is_empty() {
        emit(&root, &rendered, &digests)?;
    }

    let found = generated_files(&root)?;
    let recorded = read_manifest(&root, &mut violations)?;
    check_orphans(UNITS, &found, &mut violations);
    check_outputs(&root, &rendered, &mut violations)?;
    check_manifest(&digests, &recorded, &mut violations);

    println!(
        "{NAME}: examined {units} active generation unit(s), {modules} module(s) under \
         crates/jlreq/src/{GENERATED_DIRECTORY}/ and {entries} digest(s) recorded \
         in {MANIFEST_PATH}",
        units = UNITS.len(),
        modules = found.len(),
        entries = recorded.len(),
    );
    Ok(violations)
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
                "usage: `{NAME}` to write the generated files, `{NAME} --check` to prove \
                 regeneration would change none of them; got `{given}`",
                given = arguments.join(" ")
            ),
        )),
    }
}

/// Reject a unit whose declaration the rest of this gate could not police.
///
/// The scan for unclaimed modules reads `crates/*/src/generated/`, so a unit writing
/// anywhere else would be byte-checked while a hand-written file beside it went unnoticed,
/// and two units claiming one path would let the order they are listed in decide what that
/// path ends up holding.
fn check_declarations(units: &[Unit]) -> Vec<String> {
    let mut violations = Vec::new();
    let mut claimed = BTreeSet::new();
    for unit in units {
        if unit.input.split('/').next() != Some(SPEC_DIRECTORY) {
            violations.push(format!(
                "{input}: a generation unit reads only the vendored specification data \
                 under `{SPEC_DIRECTORY}/`",
                input = unit.input
            ));
        }
        if !is_generated_module(unit.output) {
            violations.push(format!(
                "{output}: a generation unit writes only \
                 `{CRATES_DIRECTORY}/<crate>/src/{GENERATED_DIRECTORY}/<module>.rs`",
                output = unit.output
            ));
        }
        for module in unit.generator {
            if !is_xtask_module(module) {
                violations.push(format!(
                    "{module}: a unit's generator is an `xtask` module, and its digest is \
                     what makes `{output}` state which generator emitted it",
                    output = unit.output
                ));
            }
        }
        if unit.generator.is_empty() {
            violations.push(format!(
                "{output}: names no generator, so its header could not state what emitted it",
                output = unit.output
            ));
        }
        if !claimed.insert(unit.output) {
            violations.push(format!(
                "{output}: claimed by two generation units, so which one writes it would \
                 depend on the order they are declared in",
                output = unit.output
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

/// Whether a repository path names a Rust module directly inside a crate's `generated`
/// directory.
fn is_generated_module(path: &str) -> bool {
    let parts: Vec<&str> = path.split('/').collect();
    match parts.as_slice() {
        [crates, _package, "src", generated, module] => {
            *crates == CRATES_DIRECTORY
                && *generated == GENERATED_DIRECTORY
                && module
                    .rsplit_once('.')
                    .is_some_and(|(stem, extension)| !stem.is_empty() && extension == "rs")
        },
        _ => false,
    }
}

/// Render every unit into the bytes its output file must hold.
///
/// A unit that cannot be rendered — an absent input, an input that is not tab-separated
/// text, a generator that refuses — is a violation and not an aborted run, so one broken
/// table does not hide the state of the others.
fn render(root: &Path, units: &[Unit], violations: &mut Vec<String>) -> io::Result<Vec<Rendered>> {
    if units.is_empty() {
        // Nothing to render, and so no reason to demand the snapshot's provenance: the
        // first unit arrives in the same commit as the snapshot that provenance describes.
        return Ok(Vec::new());
    }
    let specification_date = read_specification_date(root)?;
    let mut rendered = Vec::new();
    for unit in units {
        if let Some(one) = render_one(root, unit, &specification_date, violations)? {
            rendered.push(one);
        }
    }
    Ok(rendered)
}

/// Render one unit, or explain why it could not be rendered.
fn render_one(
    root: &Path,
    unit: &Unit,
    specification_date: &str,
    violations: &mut Vec<String>,
) -> io::Result<Option<Rendered>> {
    let path = root.join(unit.input);
    if !path.is_file() {
        violations.push(format!(
            "{input}: declared as the source of `{output}` and not present",
            input = unit.input,
            output = unit.output
        ));
        return Ok(None);
    }

    let bytes = fs::read(&path)?;
    let input_digest = sha256(&bytes);
    let Ok(text) = String::from_utf8(bytes) else {
        violations.push(format!("{input}: is not UTF-8", input = unit.input));
        return Ok(None);
    };

    let table = match read_table(unit.input.to_owned(), &text) {
        Ok(table) => table,
        Err(violation) => {
            violations.push(violation);
            return Ok(None);
        },
    };

    let Some(generator) = generator_digest(root, unit.generator, violations)? else {
        return Ok(None);
    };

    let emission = match (unit.emit)(&table) {
        Ok(emission) => emission,
        Err(reason) => {
            violations.push(format!("{input}: {reason}", input = unit.input));
            return Ok(None);
        },
    };
    if emission.entries == 0 || emission.items.trim().is_empty() {
        violations.push(format!(
            "{input}: generated no entries for `{output}`",
            input = unit.input,
            output = unit.output
        ));
        return Ok(None);
    }

    let contents = compose(
        unit,
        &input_digest,
        specification_date,
        &generator,
        &emission,
    );
    if contents.contains('\r') {
        violations.push(format!(
            "{output}: holds a carriage return; generated Rust is LF, because \
             `rustfmt.toml` sets `newline_style = \"Unix\"` and a CRLF file fails the \
             format check outright",
            output = unit.output
        ));
        return Ok(None);
    }

    Ok(Some(Rendered {
        input: unit.input.to_owned(),
        output: unit.output.to_owned(),
        contents,
        input_digest,
    }))
}

/// Read a tab-separated input.
///
/// The format is the one `docs/design/generation.md` writes: a header line naming the
/// columns, one record per line under it, `#` comments and blank lines ignored. A record
/// whose field count differs from the header's is rejected rather than padded, because a
/// short row in a provenance-bearing file is a cell with no provenance; and a carriage
/// return is rejected rather than absorbed, because these files are compared byte for byte
/// and `.gitattributes` keeps the whole tree LF.
///
/// Crate-visible so that a unit's own tests can run its generator over the committed file it
/// reads, rather than over a fixture that agrees with the generator by construction.
pub(crate) fn read_table(source: String, text: &str) -> Result<Table, String> {
    let mut columns: Vec<String> = Vec::new();
    let mut records = Vec::new();

    for (index, line) in text.split('\n').enumerate() {
        let number = index.wrapping_add(1);
        if line.contains('\r') {
            return Err(format!(
                "{source}:{number}: holds a carriage return; the specification data is LF"
            ));
        }
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let fields: Vec<String> = line.split('\t').map(str::to_owned).collect();
        if columns.is_empty() {
            columns = fields;
            continue;
        }
        records.push(Record {
            line: number,
            fields,
        });
    }

    let table = Table {
        source,
        columns,
        records,
    };
    if table.columns.is_empty() {
        return Err(format!(
            "{source}: names no columns; the first line that is neither blank nor a \
             comment is the header",
            source = table.source
        ));
    }
    if table.records.is_empty() {
        return Err(format!(
            "{source}: a header line and no records",
            source = table.source
        ));
    }
    for record in &table.records {
        if record.fields.len() != table.columns.len() {
            return Err(format!(
                "{source}:{line}: {found} field(s) under {expected} column(s)",
                source = table.source,
                line = record.line,
                found = record.fields.len(),
                expected = table.columns.len()
            ));
        }
    }
    Ok(table)
}

/// Compose the complete generated file: the header every one of them carries, then the
/// generator's items.
///
/// The header is what makes the file auditable without running anything — it names the
/// source, that source's digest, the specification revision, the generator, and how many
/// entries the items below define — and it says outright that the file is not to be
/// edited. Nothing in it comes from the clock or the environment, so two runs of the same
/// generator over the same input produce the same bytes.
fn compose(
    unit: &Unit,
    input_digest: &str,
    specification_date: &str,
    generator: &Generator,
    emission: &Emission,
) -> String {
    // REUSE-IgnoreStart
    // The template below writes an SPDX header into another file. `reuse lint` reads any
    // `SPDX-License-Identifier:` it finds as a statement about the file it is reading, so
    // without these markers the emitter would be reported as declaring `{LICENSE}` for
    // itself. This file's own header, at the top and outside the markers, is still read.
    format!(
        "// SPDX-FileCopyrightText: {COPYRIGHT}\n\
         //\n\
         // SPDX-License-Identifier: {LICENSE}\n\
         \n\
         //! {summary}\n\
         //!\n\
         //! Do not edit. `cargo run -p xtask -- generate` writes this file, and\n\
         //! `generate --check` fails when regenerating it would change a byte. A hand\n\
         //! edit is a bug even when it is correct, because the next revision of the\n\
         //! specification will not carry it forward (ADR 0009).\n\
         //!\n\
         //! - Source: `{input}`\n\
         //! - Source SHA-256: `{input_digest}`\n\
         //! - Specification: JLReq, {specification_date}\n\
         //! - Generator: {modules}\n\
         //! - Generator SHA-256: `{generator}`\n\
         //! - Entries: {entries}\n\
         \n\
         {items}\n",
        summary = unit.summary,
        input = unit.input,
        modules = generator
            .modules
            .iter()
            .map(|module| format!("`{module}`"))
            .collect::<Vec<String>>()
            .join(", "),
        generator = generator.digest,
        entries = emission.entries,
        items = emission.items.trim_end_matches('\n'),
    )
    // REUSE-IgnoreEnd
}

/// The digest of every file the pipeline reads or writes.
///
/// Keyed by repository path and therefore sorted, so the manifest this becomes is
/// byte-identical wherever it was written.
///
/// Not only the units' own outputs and inputs. Several derived tables — the document
/// skeleton, the appendix notes and the policy space among them — are consumed by no
/// generation unit yet and are load-bearing all the same: `attest`, `api`, and `conform`
/// read them as release controls. The vendored documents and this
/// program's own reading and emitting modules are recorded for the same reason. The manifest
/// is this repository's one ledger of what produced what, and a ledger that records part of
/// a chain records nothing about the rest of it.
fn digests(
    root: &Path,
    rendered: &[Rendered],
    violations: &mut Vec<String>,
) -> io::Result<BTreeMap<String, String>> {
    let mut entries = BTreeMap::new();
    for one in rendered {
        entries.insert(one.output.clone(), sha256(one.contents.as_bytes()));
        entries.insert(one.input.clone(), one.input_digest.clone());
    }
    if rendered.is_empty() {
        return Ok(entries);
    }
    for path in ledger() {
        if entries.contains_key(&path) {
            // A unit's own render is the authority for its own two files: it hashed the
            // input it actually read and the bytes it actually produced. This pass fills in
            // the rest of the chain and never restates either.
            continue;
        }
        let full = root.join(&path);
        if !full.is_file() {
            violations.push(format!(
                "{path}: read or written by the specification data pipeline and not present, \
                 so {MANIFEST_PATH} could not record a digest for it"
            ));
            continue;
        }
        entries.insert(path, sha256(&fs::read(&full)?));
    }
    Ok(entries)
}

/// Every path the manifest records beside the units' own outputs and inputs.
fn ledger() -> BTreeSet<String> {
    let mut paths: BTreeSet<String> = BTreeSet::new();
    for derivation in derive::DERIVATIONS {
        paths.insert(derivation.output.to_owned());
    }
    for source in derive::distinct_sources(derive::DERIVATIONS) {
        paths.insert(source.to_owned());
    }
    for module in derive::distinct_readers(derive::DERIVATIONS) {
        paths.insert(module.to_owned());
    }
    for unit in UNITS {
        for module in unit.generator {
            paths.insert((*module).to_owned());
        }
    }
    paths.insert(FRAME.to_owned());
    // The provenance record is an input of this stage too: `specification-date` is copied
    // into the header of every emitted file.
    paths.insert(PROVENANCE_PATH.to_owned());
    for path in [
        "spec/captured/table1.ja.tsv",
        "spec/captured/table2.ja.tsv",
        "spec/captured/table3.ja.tsv",
        "spec/captured/table4.ja.tsv",
        "spec/captured/table5.ja.tsv",
        "spec/captured/table6.ja.tsv",
        "spec/captured/invariants.tsv",
        "xtask/src/attest.rs",
        "docs/public-api.toml",
        "xtask/src/api.rs",
        "crates/jlreq-conformance/suite.ndjson",
        "crates/jlreq-conformance/protocol.schema.json",
        "docs/conformance-deferrals.toml",
        "ROADMAP.md",
        "xtask/src/conform.rs",
        "xtask/src/deferral.rs",
    ] {
        paths.insert(path.to_owned());
    }
    paths
}

/// Write every rendered file and the manifest recording them.
///
/// Called only when every unit rendered, so a run that could not render one table never
/// leaves a half-written tree behind for someone to commit.
fn emit(root: &Path, rendered: &[Rendered], entries: &BTreeMap<String, String>) -> io::Result<()> {
    for one in rendered {
        let path = root.join(&one.output);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, one.contents.as_bytes())?;
    }
    if entries.is_empty() {
        // Nothing was generated, so there is nothing to record and no file to write. An
        // empty manifest would be a record of nothing that still had to be maintained.
        return Ok(());
    }
    let path = root.join(MANIFEST_PATH);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, render_manifest(entries).as_bytes())
}

/// Every file under a `generated` directory anywhere in `crates/`.
///
/// Discovered rather than declared: a file in a directory no unit knows about is exactly
/// what this gate is looking for, so the scan cannot be driven by the unit list.
///
/// Every file and not only every `.rs` one. A binary table beside the modules, pulled into a
/// hand-written `generated.rs` with `include_bytes!`, is generated data no unit writes, in
/// the one directory this scan exists to close.
fn generated_files(root: &Path) -> io::Result<BTreeSet<String>> {
    let mut found = BTreeSet::new();
    collect_generated(&root.join(CRATES_DIRECTORY), root, false, &mut found)?;
    Ok(found)
}

/// Walk one directory, collecting every file at or below a `generated` component.
fn collect_generated(
    directory: &Path,
    root: &Path,
    inside: bool,
    found: &mut BTreeSet<String>,
) -> io::Result<()> {
    if !directory.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            let generated = inside
                || path
                    .file_name()
                    .is_some_and(|name| name == GENERATED_DIRECTORY);
            collect_generated(&path, root, generated, found)?;
        } else if inside {
            found.insert(repository_path(root, &path));
        }
    }
    Ok(())
}

/// Report every generated file no unit writes.
fn check_orphans(units: &[Unit], found: &BTreeSet<String>, violations: &mut Vec<String>) {
    let claimed: BTreeSet<&str> = units.iter().map(|unit| unit.output).collect();
    for path in found {
        if !claimed.contains(path.as_str()) {
            violations.push(format!(
                "{path}: no generation unit writes this file, and everything under a \
                 `{GENERATED_DIRECTORY}` directory is generated; a hand-written module \
                 declaring the others belongs in `src/{GENERATED_DIRECTORY}.rs` beside the \
                 directory"
            ));
        }
    }
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
        if let Some(violation) =
            difference(&one.output, &one.input, on_disk.as_deref(), &one.contents)
        {
            violations.push(violation);
        }
    }
    Ok(())
}

/// What one output file earns against the bytes regenerating it produces, if anything.
///
/// Byte identity, not semantic equivalence: a reordered table, a re-wrapped doc comment and
/// a changed line ending are all differences, because all three mean the file in the
/// repository is not the file the generator writes.
fn difference(
    output: &str,
    input: &str,
    on_disk: Option<&[u8]>,
    regenerated: &str,
) -> Option<String> {
    let Some(on_disk) = on_disk else {
        return Some(format!(
            "{output}: `{input}` generates this file and it is not present; run \
             `cargo run -p xtask -- generate`"
        ));
    };
    if on_disk == regenerated.as_bytes() {
        return None;
    }
    Some(format!(
        "{output}: regenerating it from `{input}` would change it. On disk {before_length} \
         byte(s), SHA-256 {before}; regenerated {after_length} byte(s), SHA-256 {after}. A \
         generated file is never hand-edited (ADR 0009); run \
         `cargo run -p xtask -- generate`",
        before_length = on_disk.len(),
        before = sha256(on_disk),
        after_length = regenerated.len(),
        after = sha256(regenerated.as_bytes()),
    ))
}

/// Read what `data/manifest.toml` records, reporting whatever it holds that this gate does
/// not write.
///
/// A manifest that is not there records nothing, which is the right reading when no unit is
/// declared and a reported failure the moment one is.
fn read_manifest(
    root: &Path,
    violations: &mut Vec<String>,
) -> io::Result<BTreeMap<String, String>> {
    let path = root.join(MANIFEST_PATH);
    if !path.is_file() {
        return Ok(BTreeMap::new());
    }
    Ok(manifest_entries(&fs::read_to_string(&path)?, violations))
}

/// Read the `[[file]]` tables of a manifest.
///
/// A hand-rolled scan rather than a TOML parser, for the reason stated on
/// `declared_dependencies` in the `purity` module: the program that enforces the layout
/// core's empty dependency table declares none itself. It understands the one shape this
/// gate writes, and it reports anything else rather than skipping it, because a manifest
/// holding a table this gate cannot read is a manifest that is no longer a record.
fn manifest_entries(text: &str, violations: &mut Vec<String>) -> BTreeMap<String, String> {
    let mut entries = BTreeMap::new();
    let mut path: Option<String> = None;
    let mut digest: Option<String> = None;
    let mut inside_file = false;

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            commit(&mut path, &mut digest, &mut entries, violations);
            inside_file = line == "[[file]]";
            if !inside_file {
                violations.push(format!(
                    "{MANIFEST_PATH}: holds `{line}`; this gate writes `[[file]]` tables \
                     and reads nothing else"
                ));
            }
            continue;
        }
        if !inside_file {
            // A key belonging to a table this gate does not write is not a record of a
            // file, whatever it happens to be called.
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            match key.trim() {
                "path" => path = quoted(value).map(str::to_owned),
                "sha256" => digest = quoted(value).map(str::to_owned),
                _ => {},
            }
        }
    }
    commit(&mut path, &mut digest, &mut entries, violations);
    entries
}

/// Finish the `[[file]]` table just read, reporting one that is missing a key.
fn commit(
    path: &mut Option<String>,
    digest: &mut Option<String>,
    entries: &mut BTreeMap<String, String>,
    violations: &mut Vec<String>,
) {
    match (path.take(), digest.take()) {
        (Some(path), Some(digest)) => {
            entries.insert(path, digest);
        },
        (Some(path), None) => violations.push(format!(
            "{MANIFEST_PATH}: records `{path}` with no `sha256`"
        )),
        (None, Some(digest)) => violations.push(format!(
            "{MANIFEST_PATH}: records SHA-256 {digest} with no `path`"
        )),
        (None, None) => {},
    }
}

/// Report every disagreement between the digests the units account for and the digests the
/// manifest records.
fn check_manifest(
    expected: &BTreeMap<String, String>,
    recorded: &BTreeMap<String, String>,
    violations: &mut Vec<String>,
) {
    for (path, digest) in expected {
        match recorded.get(path) {
            None => violations.push(format!(
                "{MANIFEST_PATH}: records no digest for `{path}`; run \
                 `cargo run -p xtask -- generate`"
            )),
            Some(recorded) if recorded != digest => violations.push(format!(
                "{MANIFEST_PATH}: records SHA-256 {recorded} for `{path}`, which is \
                 {digest}; run `cargo run -p xtask -- generate`"
            )),
            Some(_) => {},
        }
    }
    for path in recorded.keys() {
        if !expected.contains_key(path) {
            violations.push(format!(
                "{MANIFEST_PATH}: records `{path}`, which no generation unit writes or \
                 reads"
            ));
        }
    }
}

/// Render the manifest: one `[[file]]` table per path, in sorted order.
fn render_manifest(entries: &BTreeMap<String, String>) -> String {
    let mut tables = String::new();
    for (path, digest) in entries {
        tables.push_str("\n[[file]]\npath = \"");
        tables.push_str(path);
        tables.push_str("\"\nsha256 = \"");
        tables.push_str(digest);
        tables.push_str("\"\n");
    }
    // REUSE-IgnoreStart
    // As in `compose`: the header below belongs to the manifest this writes, not to this
    // file, and `reuse lint` cannot tell the two apart without these markers.
    format!(
        "# SPDX-FileCopyrightText: {COPYRIGHT}\n\
         #\n\
         # SPDX-License-Identifier: {LICENSE}\n\
         #\n\
         # Generated by `cargo run -p xtask -- generate`. Do not edit.\n\
         #\n\
         # The SHA-256 of every generated file and of every input it was generated from.\n\
         # `cargo run -p xtask -- generate --check` fails when a digest recorded here is\n\
         # not the digest of the file it names, so an edit to an input that was never\n\
         # regenerated from is as visible as an edit to the output.\n\
         {tables}"
    )
    // REUSE-IgnoreEnd
}

/// Read the revision of JLReq the vendored snapshot was taken from.
fn read_specification_date(root: &Path) -> io::Result<String> {
    let path = root.join(PROVENANCE_PATH);
    let text = fs::read_to_string(&path).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!(
                "{PROVENANCE_PATH}: {error}; every generated file states the revision of \
                 JLReq it was generated from"
            ),
        )
    })?;
    specification_date(&text).map(str::to_owned).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{PROVENANCE_PATH}: states no `{SPECIFICATION_DATE_KEY}`"),
        )
    })
}

/// The `specification-date` at the top level of a provenance file.
///
/// Top level means above the first table, so a key of the same name inside a table
/// describing one upstream document is not mistaken for the snapshot's own.
fn specification_date(text: &str) -> Option<&str> {
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            return None;
        }
        if let Some((key, value)) = line.split_once('=') {
            if key.trim() == SPECIFICATION_DATE_KEY {
                return quoted(value);
            }
        }
    }
    None
}

/// The first string literal on a line, without its quotes.
fn quoted(value: &str) -> Option<&str> {
    let (_, after) = value.split_once('"')?;
    let (inside, _) = after.split_once('"')?;
    Some(inside)
}

/// Name a path the way this repository writes one: relative to the workspace root, with
/// forward slashes, so a manifest written on Windows is byte-identical to one written on
/// Linux.
fn repository_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// The size of one SHA-256 block, in bytes.
const BLOCK: usize = 64;

/// Where the message length is written inside the final block.
const LENGTH_OFFSET: usize = 56;

/// How many rounds one block is compressed in.
const ROUNDS: usize = 64;

/// The digits a digest is written in, low to high.
const HEXADECIMAL: [char; 16] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
];

/// How many characters a SHA-256 digest is written in: two per byte.
const DIGEST_CHARACTERS: usize = 64;

/// The initial hash value: the fractional parts of the square roots of the first eight
/// primes (FIPS 180-4 §5.3.3).
const INITIAL_STATE: [u32; 8] = [
    0x6a09_e667,
    0xbb67_ae85,
    0x3c6e_f372,
    0xa54f_f53a,
    0x510e_527f,
    0x9b05_688c,
    0x1f83_d9ab,
    0x5be0_cd19,
];

/// The round constants: the fractional parts of the cube roots of the first sixty-four
/// primes (FIPS 180-4 §4.2.2).
const ROUND_CONSTANTS: [u32; ROUNDS] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

/// The SHA-256 of a byte string, in lowercase hexadecimal (FIPS 180-4 §6.2).
///
/// Hand-written for the reason this whole program is: `xtask` declares no dependencies,
/// because it is what enforces that the layout core declares none. A hundred lines of
/// arithmetic is a smaller price than a dependency in the tool that polices dependencies,
/// and the published test vectors below say whether the price bought the right answer.
///
/// Shared with `derive`, which states the digest of every vendored source inside the file
/// it reads out of it, so that the chain from the published document to the emitted Rust
/// is digest-linked at every step and computed by one implementation.
pub(crate) fn sha256(bytes: &[u8]) -> String {
    let mut padded = Vec::with_capacity(bytes.len().saturating_add(BLOCK.saturating_mul(2)));
    padded.extend_from_slice(bytes);
    padded.push(0x80);
    while padded.len() % BLOCK != LENGTH_OFFSET {
        padded.push(0);
    }
    // The length field is 64 bits. `usize` is at most 64 bits wide on every target this
    // program is built for, so the conversion is exact, and the saturation below is
    // unreachable rather than approximate: no file is 2^61 bytes long.
    let bit_length = u64::try_from(bytes.len())
        .unwrap_or(u64::MAX)
        .saturating_mul(8);
    padded.extend_from_slice(&bit_length.to_be_bytes());

    let mut state = INITIAL_STATE;
    for block in padded.chunks_exact(BLOCK) {
        compress(&mut state, block);
    }

    let mut hexadecimal = String::with_capacity(DIGEST_CHARACTERS);
    for byte in state.iter().flat_map(|word| word.to_be_bytes()) {
        hexadecimal.push(HEXADECIMAL[usize::from(byte >> 4)]);
        hexadecimal.push(HEXADECIMAL[usize::from(byte & 0x0f)]);
    }
    hexadecimal
}

/// Fold one 64-byte block into the hash state (FIPS 180-4 §6.2.2).
fn compress(state: &mut [u32; 8], block: &[u8]) {
    let mut schedule = [0_u32; ROUNDS];
    for (word, chunk) in schedule.iter_mut().zip(block.chunks_exact(4)) {
        *word = chunk
            .iter()
            .fold(0_u32, |built, byte| (built << 8) | u32::from(*byte));
    }
    for index in 16..ROUNDS {
        // The loop starts at sixteen, so none of these subtractions reaches below zero;
        // the wrapping form is written because the workspace denies bare integer
        // arithmetic, and a hash is the one place where wrapping is the definition.
        let recent = schedule[index.wrapping_sub(2)];
        let older = schedule[index.wrapping_sub(15)];
        let mixed = older.rotate_right(7) ^ older.rotate_right(18) ^ (older >> 3);
        let spread = recent.rotate_right(17) ^ recent.rotate_right(19) ^ (recent >> 10);
        schedule[index] = schedule[index.wrapping_sub(16)]
            .wrapping_add(mixed)
            .wrapping_add(schedule[index.wrapping_sub(7)])
            .wrapping_add(spread);
    }

    // The eight working variables are held as an array rather than as eight names, so the
    // round's shift is one rotation rather than seven assignments.
    let mut working = *state;
    for (word, constant) in schedule.iter().zip(ROUND_CONSTANTS) {
        let summed =
            working[4].rotate_right(6) ^ working[4].rotate_right(11) ^ working[4].rotate_right(25);
        let choice = (working[4] & working[5]) ^ (!working[4] & working[6]);
        let first = working[7]
            .wrapping_add(summed)
            .wrapping_add(choice)
            .wrapping_add(constant)
            .wrapping_add(*word);
        let mingled =
            working[0].rotate_right(2) ^ working[0].rotate_right(13) ^ working[0].rotate_right(22);
        let majority =
            (working[0] & working[1]) ^ (working[0] & working[2]) ^ (working[1] & working[2]);
        let second = mingled.wrapping_add(majority);
        let carried = working[3].wrapping_add(first);
        working.rotate_right(1);
        working[0] = first.wrapping_add(second);
        working[4] = carried;
    }
    for (total, part) in state.iter_mut().zip(working) {
        *total = total.wrapping_add(part);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::{
        Emission, Generator, Mode, Rendered, Table, UNITS, Unit, check_declarations,
        check_manifest, check_orphans, compose, difference, digests, is_generated_module,
        manifest_entries, mode, quoted, read_table, render_manifest, sha256, specification_date,
    };
    use crate::shared;

    /// A tab-separated input in the shape `docs/design/generation.md` writes.
    const CAPTURED: &str = "# spec/captured/table1.en.tsv\nsource\ttable\tbefore\tafter\ttoken\tnote\ntable_en2.pdf\t1\tcl-05\tcl-05\t1/4 be + 1/4 af\tB.2#3\ntable_en2.pdf\t1\tcl-02\tline-end\t1/2 be\tB.2#2\n";

    /// A unit declaration whose generator is never reached by these tests.
    fn unit(input: &'static str, output: &'static str) -> Unit {
        Unit {
            input,
            generator: &["xtask/src/classes.rs"],
            output,
            summary: "A fixture.",
            emit: |_| Err("a fixture generator emits nothing".to_owned()),
        }
    }

    /// The generator a composed fixture states.
    fn generator() -> Generator {
        Generator {
            modules: vec![
                "xtask/src/classes.rs".to_owned(),
                "xtask/src/generate.rs".to_owned(),
            ],
            digest: "0f0f0f".to_owned(),
        }
    }

    /// The workspace root, for the checks that read the tree they ship with.
    fn root() -> std::path::PathBuf {
        shared::workspace_root().expect("the workspace root")
    }

    #[test]
    fn the_gate_holds_over_this_repository() {
        // The gate run over the tree it ships with, so `just test` exercises the real
        // pipeline against the real derived tables rather than against fixtures that agree
        // with the generator by construction.
        let violations = super::run(&["--check".to_owned()]).expect("the gate can run");
        assert!(
            violations.is_empty(),
            "regenerating the specification data changes a committed file: {violations:?}"
        );
    }

    #[test]
    fn every_declared_unit_names_a_generator_of_this_program() {
        assert!(
            check_declarations(UNITS).is_empty(),
            "the units this repository declares must satisfy the gate that reads them"
        );
        for unit in UNITS {
            assert!(
                !unit.generator.is_empty(),
                "{output} states which generator emitted it",
                output = unit.output
            );
        }
    }

    #[test]
    fn sha256_matches_the_published_test_vectors() {
        assert_eq!(
            sha256(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
            "fifty-six bytes: the padding spills into a second block"
        );
        assert_eq!(
            sha256(
                b"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu"
            ),
            "cf5b16a778af8380036ce59e7b0492370b249b11e8f07a51afac45037afee9d1"
        );
    }

    #[test]
    fn one_changed_byte_changes_the_digest() {
        assert_ne!(sha256(b"cl-05"), sha256(b"cl-06"));
    }

    #[test]
    fn a_tab_separated_input_reads_its_header_and_its_records() {
        let table = read_table("spec/captured/table1.en.tsv".to_owned(), CAPTURED)
            .expect("the fixture is well formed");
        assert_eq!(table.columns.len(), 6);
        assert_eq!(table.columns.first().map(String::as_str), Some("source"));
        assert_eq!(table.records.len(), 2, "the comment line is not a record");
        assert_eq!(
            table.records.first().map(|record| record.line),
            Some(3),
            "a record names the line it was read from, comments included in the count"
        );
        assert_eq!(
            table
                .records
                .first()
                .and_then(|record| record.fields.get(4)),
            Some(&"1/4 be + 1/4 af".to_owned())
        );
        assert_eq!(table.source, "spec/captured/table1.en.tsv");
    }

    #[test]
    fn a_record_with_the_wrong_number_of_fields_is_rejected() {
        let short = "before\tafter\ttoken\ncl-05\tcl-05\n";
        let violation = read_table("spec/captured/table1.en.tsv".to_owned(), short)
            .expect_err("a short record has a cell with no provenance");
        assert!(violation.contains(":2:"), "names the line: {violation}");
        assert!(
            violation.contains("2 field(s) under 3 column(s)"),
            "names both counts: {violation}"
        );
    }

    #[test]
    fn a_carriage_return_in_an_input_is_rejected() {
        let violation = read_table("spec/derived/anchors.tsv".to_owned(), "a\tb\r\n1\t2\n")
            .expect_err("the specification data is LF");
        assert!(violation.contains("carriage return"), "{violation}");
    }

    #[test]
    fn an_input_with_a_header_and_no_records_is_rejected() {
        let violation = read_table("spec/derived/notes.tsv".to_owned(), "a\tb\n# nothing\n")
            .expect_err("a generator with no records emits nothing");
        assert!(violation.contains("no records"), "{violation}");
    }

    #[test]
    fn an_input_with_no_header_is_rejected() {
        let violation = read_table("spec/derived/rules.tsv".to_owned(), "# only a comment\n")
            .expect_err("the first line that is not a comment is the header");
        assert!(violation.contains("names no columns"), "{violation}");
    }

    #[test]
    fn a_generated_module_states_where_it_came_from_and_forbids_editing() {
        let declared = unit(
            "spec/derived/appendix-a.tsv",
            "crates/jlreq-class/src/generated/appendix_a.rs",
        );
        let emission = Emission {
            items: "/// The keys.\npub static KEYS: [u32; 0] = [];".to_owned(),
            entries: 1133,
        };
        let file = compose(&declared, "0123abc", "2020-08-11", &generator(), &emission);

        // REUSE-IgnoreStart
        // The header this asserts on is the emitted file's, not this one's.
        assert!(file.starts_with("// SPDX-FileCopyrightText: 2026 jlreq contributors\n"));
        assert!(file.contains("// SPDX-License-Identifier: MIT OR Apache-2.0\n"));
        // REUSE-IgnoreEnd
        assert!(file.contains("//! A fixture.\n"));
        assert!(file.contains("Do not edit."), "{file}");
        assert!(file.contains("//! - Source: `spec/derived/appendix-a.tsv`\n"));
        assert!(file.contains("//! - Source SHA-256: `0123abc`\n"));
        assert!(file.contains("//! - Specification: JLReq, 2020-08-11\n"));
        assert!(file.contains("//! - Entries: 1133\n"));
        assert!(
            file.contains(
                "//! - Generator: `xtask/src/classes.rs`, `xtask/src/generate.rs`\n\
                 //! - Generator SHA-256: `0f0f0f`\n"
            ),
            "the generator is provenance too: the semantic columns of a generated table are \
             its reading of the source rather than a column of it, so a file naming only its \
             input overstates what its digests cover. A version string cannot do this work — \
             it is the shared workspace version, which moves on a release and never on a \
             change to a generator: {file}"
        );
        assert!(
            file.ends_with("pub static KEYS: [u32; 0] = [];\n"),
            "{file}"
        );
        assert!(!file.contains('\r'), "generated Rust is LF");
    }

    #[test]
    fn the_same_input_composes_the_same_bytes_twice() {
        let declared = unit(
            "spec/derived/rules.tsv",
            "crates/jlreq-spec/src/generated/rule.rs",
        );
        let emission = Emission {
            items: "pub static RULES: [u32; 1] = [1];".to_owned(),
            entries: 1,
        };
        assert_eq!(
            compose(&declared, "abc", "2020-08-11", &generator(), &emission),
            compose(&declared, "abc", "2020-08-11", &generator(), &emission),
            "nothing in a generated file may come from the clock or the environment"
        );
    }

    #[test]
    fn a_generated_module_sits_directly_in_a_crates_generated_directory() {
        assert!(is_generated_module(
            "crates/jlreq-class/src/generated/appendix_a.rs"
        ));
        assert!(
            !is_generated_module("crates/jlreq-class/src/generated.rs"),
            "the module declaring the directory is hand-written and lives beside it"
        );
        assert!(
            !is_generated_module("crates/jlreq-class/src/generated/deeper/table.rs"),
            "a nested module would escape the scan that claims files"
        );
        assert!(!is_generated_module("crates/jlreq-class/src/class.rs"));
        assert!(!is_generated_module("spec/derived/appendix-a.tsv"));
    }

    #[test]
    fn a_unit_writing_outside_a_generated_directory_is_rejected() {
        let violations = check_declarations(&[unit(
            "spec/derived/appendix-a.tsv",
            "crates/jlreq-class/src/appendix_a.rs",
        )]);
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert!(
            violations
                .first()
                .is_some_and(|violation| violation.contains("generated")),
            "{violations:?}"
        );
    }

    #[test]
    fn a_unit_reading_outside_the_specification_data_is_rejected() {
        let violations = check_declarations(&[unit(
            "docs/design/generation.md",
            "crates/jlreq-class/src/generated/appendix_a.rs",
        )]);
        assert_eq!(violations.len(), 1, "{violations:?}");
    }

    #[test]
    fn two_units_writing_one_file_are_rejected() {
        let output = "crates/jlreq-line/src/generated/figures.rs";
        let violations = check_declarations(&[
            unit("spec/captured/figures.tsv", output),
            unit("spec/derived/notes.tsv", output),
        ]);
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert!(
            violations
                .first()
                .is_some_and(|violation| violation.contains("two generation units")),
            "{violations:?}"
        );
    }

    #[test]
    fn the_declared_units_are_well_formed() {
        assert!(
            check_declarations(UNITS).is_empty(),
            "the units this repository declares must satisfy the gate that reads them"
        );
    }

    #[test]
    fn a_generated_module_no_unit_writes_is_reported() {
        let found: BTreeSet<String> = [
            "crates/jlreq-class/src/generated/appendix_a.rs".to_owned(),
            "crates/jlreq-class/src/generated/mod.rs".to_owned(),
        ]
        .into_iter()
        .collect();
        let mut violations = Vec::new();
        check_orphans(
            &[unit(
                "spec/derived/appendix-a.tsv",
                "crates/jlreq-class/src/generated/appendix_a.rs",
            )],
            &found,
            &mut violations,
        );
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert!(
            violations
                .first()
                .is_some_and(|violation| violation.contains("mod.rs")),
            "the hand-written module in the generated directory is the one reported: \
             {violations:?}"
        );
    }

    #[test]
    fn nothing_is_reported_when_every_generated_module_is_claimed() {
        let mut violations = Vec::new();
        check_orphans(&[], &BTreeSet::new(), &mut violations);
        assert!(violations.is_empty(), "{violations:?}");
    }

    #[test]
    fn a_byte_of_difference_is_a_violation() {
        let regenerated = "pub static A: u8 = 1;\n";
        assert_eq!(
            difference(
                "out.rs",
                "in.tsv",
                Some(regenerated.as_bytes()),
                regenerated
            ),
            None
        );

        let edited = "pub static A: u8 = 2;\n";
        let violation = difference("out.rs", "in.tsv", Some(edited.as_bytes()), regenerated)
            .expect("one byte apart is a difference");
        assert!(violation.contains("would change it"), "{violation}");
        assert!(
            violation.contains(&sha256(edited.as_bytes())),
            "{violation}"
        );
        assert!(
            violation.contains(&sha256(regenerated.as_bytes())),
            "{violation}"
        );

        let line_endings = "pub static A: u8 = 1;\r\n";
        assert!(
            difference(
                "out.rs",
                "in.tsv",
                Some(line_endings.as_bytes()),
                regenerated
            )
            .is_some(),
            "byte identity, so a CRLF file is a difference"
        );
    }

    #[test]
    fn a_unit_whose_output_is_absent_is_a_violation() {
        let violation = difference("out.rs", "in.tsv", None, "pub static A: u8 = 1;\n")
            .expect("a unit with no file is as much a failure as a file with no unit");
        assert!(violation.contains("not present"), "{violation}");
    }

    #[test]
    fn the_manifest_records_every_output_and_every_input() {
        let rendered = vec![Rendered {
            input: "spec/derived/appendix-a.tsv".to_owned(),
            output: "crates/jlreq-class/src/generated/appendix_a.rs".to_owned(),
            contents: "pub static KEYS: [u32; 0] = [];\n".to_owned(),
            input_digest: sha256(b"a\tb\n"),
        }];
        let mut violations = Vec::new();
        let entries = digests(&root(), &rendered, &mut violations).expect("the tree is readable");
        assert!(violations.is_empty(), "{violations:?}");
        assert!(
            entries.len() > 2,
            "the manifest records the whole chain and not only this unit's two files, and \
             this run recorded {count}",
            count = entries.len()
        );
        for path in [
            "spec/derived/anchors.tsv",
            "spec/derived/notes.tsv",
            "spec/snapshot/index.html",
            "spec/captured/table1.ja.tsv",
            "spec/captured/invariants.tsv",
            "docs/public-api.toml",
            "crates/jlreq-conformance/suite.ndjson",
            "crates/jlreq-conformance/protocol.schema.json",
            "docs/conformance-deferrals.toml",
            "xtask/src/classes.rs",
            "xtask/src/attest.rs",
            "xtask/src/api.rs",
            "xtask/src/conform.rs",
            "xtask/src/derive.rs",
            "xtask/src/generate.rs",
            "xtask/src/inventory.rs",
        ] {
            assert!(
                entries.contains_key(path),
                "{path} is read or written by the pipeline and the manifest records it"
            );
        }
        assert_eq!(
            entries.get("spec/derived/appendix-a.tsv"),
            Some(&sha256(b"a\tb\n"))
        );
        assert_eq!(
            entries.get("crates/jlreq-class/src/generated/appendix_a.rs"),
            Some(&sha256(b"pub static KEYS: [u32; 0] = [];\n"))
        );
    }

    #[test]
    fn a_manifest_survives_being_written_and_read_back() {
        let mut entries = BTreeMap::new();
        entries.insert("spec/derived/appendix-a.tsv".to_owned(), sha256(b"one"));
        entries.insert(
            "crates/jlreq-class/src/generated/appendix_a.rs".to_owned(),
            sha256(b"two"),
        );
        let mut violations = Vec::new();
        let read = manifest_entries(&render_manifest(&entries), &mut violations);
        assert!(violations.is_empty(), "{violations:?}");
        assert_eq!(read, entries);
    }

    #[test]
    fn an_empty_manifest_is_read_as_recording_nothing() {
        let mut violations = Vec::new();
        assert!(manifest_entries("", &mut violations).is_empty());
        assert!(violations.is_empty(), "{violations:?}");
    }

    #[test]
    fn a_manifest_table_missing_a_key_is_reported() {
        let mut violations = Vec::new();
        let entries = manifest_entries("[[file]]\npath = \"a.rs\"\n", &mut violations);
        assert!(entries.is_empty(), "{entries:?}");
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert!(
            violations
                .first()
                .is_some_and(|violation| violation.contains("no `sha256`")),
            "{violations:?}"
        );
    }

    #[test]
    fn a_manifest_table_this_gate_does_not_write_is_reported() {
        let mut violations = Vec::new();
        let entries = manifest_entries(
            "[[document]]\nurl = \"https://example.invalid\"\npath = \"a.rs\"\nsha256 = \"00\"\n",
            &mut violations,
        );
        assert!(
            entries.is_empty(),
            "a key under a table this gate does not write is not a record of a file: \
             {entries:?}"
        );
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert!(
            violations
                .first()
                .is_some_and(|violation| violation.contains("[[document]]")),
            "{violations:?}"
        );
    }

    #[test]
    fn a_stale_or_missing_or_surplus_digest_is_reported() {
        let mut expected = BTreeMap::new();
        expected.insert("a.rs".to_owned(), sha256(b"a"));
        expected.insert("b.tsv".to_owned(), sha256(b"b"));

        let mut recorded = BTreeMap::new();
        recorded.insert("a.rs".to_owned(), sha256(b"stale"));
        recorded.insert("c.rs".to_owned(), sha256(b"c"));

        let mut violations = Vec::new();
        check_manifest(&expected, &recorded, &mut violations);
        assert_eq!(violations.len(), 3, "{violations:?}");
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains("`a.rs`") && violation.contains("which is")),
            "the stale digest: {violations:?}"
        );
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains("records no digest for `b.tsv`")),
            "the missing entry: {violations:?}"
        );
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains("`c.rs`")
                    && violation.contains("no generation unit")),
            "the surplus entry: {violations:?}"
        );
    }

    #[test]
    fn agreement_between_the_manifest_and_the_units_is_silent() {
        let mut entries = BTreeMap::new();
        entries.insert("a.rs".to_owned(), sha256(b"a"));
        let mut violations = Vec::new();
        check_manifest(&entries, &entries.clone(), &mut violations);
        assert!(violations.is_empty(), "{violations:?}");
    }

    #[test]
    fn the_specification_date_is_read_from_the_top_level_only() {
        let provenance = "# spec/PROVENANCE.toml\nspecification-date = \"2020-08-11\"\n\n[[document]]\nurl = \"https://www.w3.org/TR/jlreq/\"\n";
        assert_eq!(specification_date(provenance), Some("2020-08-11"));
        assert_eq!(
            specification_date("[[document]]\nspecification-date = \"2020-08-11\"\n"),
            None,
            "a date describing one upstream document is not the snapshot's own"
        );
        assert_eq!(specification_date("# nothing here\n"), None);
    }

    #[test]
    fn a_quoted_value_is_read_without_its_quotes() {
        assert_eq!(quoted(" = \"2020-08-11\""), Some("2020-08-11"));
        assert_eq!(quoted(" = 2020"), None);
    }

    #[test]
    fn the_gate_takes_check_and_refuses_anything_else() {
        assert_eq!(mode(&[]).ok(), Some(Mode::Emit));
        assert_eq!(mode(&["--check".to_owned()]).ok(), Some(Mode::Check));
        assert!(
            mode(&["--dry-run".to_owned()]).is_err(),
            "an unrecognized argument must not be read as a pass"
        );
        assert!(mode(&["--check".to_owned(), "--check".to_owned()]).is_err());
    }

    #[test]
    fn a_generator_sees_its_records_by_column() {
        let table: Table = read_table("spec/captured/table1.en.tsv".to_owned(), CAPTURED)
            .expect("the fixture is well formed");
        let token = table
            .columns
            .iter()
            .position(|column| column == "token")
            .expect("the fixture names a token column");
        let tokens: Vec<&str> = table
            .records
            .iter()
            .filter_map(|record| record.fields.get(token).map(String::as_str))
            .collect();
        assert_eq!(tokens, ["1/4 be + 1/4 af", "1/2 be"]);
    }
}
