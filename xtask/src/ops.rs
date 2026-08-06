// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `ops` gate.
//!
//! Three tables of `docs/api-frozen.toml`, enforced over the sources of every workspace
//! member:
//!
//! - `[[no_impl]]` — no listed type implements or derives a listed trait. Traits are
//!   matched on the name written in `impl <Trait> for <Type>` and in `#[derive(..)]`, so
//!   `use core::ops::Add;` does not evade the check, and over an explicit type list, so a
//!   new length type is covered only once it is added to the control file.
//! - `[[no_public_constructor]]` — a listed type gives a caller no way to build one except
//!   the factory the entry names: no public field, no public variant, no public associated
//!   function returning `Self`, and no derived `Default`.
//! - `[[scalar_channel]]` — `new`, `units` and `get` are called on an axis type or on
//!   `Advance` only inside `jlreq-unit`'s own `axis` and `length` modules and inside an
//!   item listed in `docs/scalar-sites.toml`. This is the control that makes ADR 0011's
//!   axis separation real rather than a claim: those functions are a round-trip pair
//!   through `i32` that no arrangement of types removes, so the untyped channel is
//!   narrowed to a reviewed list instead of being denied.
//!
//! The allowlist is checked in both directions. An entry naming an item that does not
//! exist, or an item that makes no call the channel would otherwise reject, is a finding
//! rather than a no-op — a permission for nothing is a permission that has stopped being
//! read, and this file's own preamble says an entry no gate can check is not an entry.
//!
//! # Mechanism
//!
//! Sources are read as text with comments, string literals and character literals blanked
//! out, then tokenized, then walked once with a stack of `{ … }` frames. The frames are
//! what let a finding name the item it is in, which is the column `docs/scalar-sites.toml`
//! is keyed on. Nothing here parses Rust in general: it recognizes `impl` headers, `fn`
//! signatures, type declarations, attributes, `use` renames and `Type::member` paths, and
//! it reads nothing else. `xtask` declares no dependencies, for the reason stated on
//! `purity`'s manifest scan, so the scan is hand-rolled in that same style.
//!
//! A `use … as …` that renames a watched trait or type is itself a finding. Name matching
//! is the whole mechanism, and a rename is the one edit that would put a name beyond it.
//!
//! # Scope
//!
//! The scalar half reads composition code and not tests. `purity` forbids a core crate
//! dev-dependencies, so its unit tests live in `#[cfg(test)]` modules inside the crate and
//! have to build the values they exercise; requiring each of them in
//! `docs/scalar-sites.toml` would fill a reviewed list with the one kind of entry it
//! exists to keep out, and a test cannot hand a wrong number to a caller. An example
//! inside a doc comment is a test too, and is blanked with the prose it sits in. The trait
//! half reads test code as well, because an `impl` under `cfg(test)` still changes what
//! the type is in the build that runs the suite.
//!
//! # What this gate does not see
//!
//! A call written on a receiver — `extent.units()` — does not name the type it is called
//! on, and no token scan tells it from `em.units()`, which is permitted everywhere because
//! `Em` is not an axis type. So the scalar half matches the path form, `InlineExtent::new`
//! and `InlineOffset::units`, which is where the invariant actually lives: a cross-axis
//! assignment needs the *entry* half, `new` is an associated function, and an associated
//! function has no other call syntax. An exit with no matching entry hands a bare `i32` to
//! code that has no typed way to put it back onto an axis.
//!
//! The axis types are tuple structs with `pub(crate)` fields, so inside `jlreq-unit`
//! `InlineExtent(raw)` and `value.0` are a second channel that names no method and that
//! `[[scalar_channel]]`'s method list therefore does not reach. That residue is stated
//! rather than glossed; closing it is a change to the control file or to those fields, and
//! neither is this gate's to make.
//!
//! See `docs/design/api-spine.md` and
//! `docs/adr/0011-typed-axes-and-direction-as-a-datum.md`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::shared::{self, Gate};

/// The frozen API file, relative to the workspace root.
const FROZEN: &str = "docs/api-frozen.toml";

/// Traits whose derived form builds a value without a named constructor.
const CONSTRUCTING_DERIVES: &[&str] = &["Default"];

/// The `ops` gate, as the dispatcher sees it.
pub(crate) const GATE: Gate = Gate {
    name: "ops",
    purpose: concat!(
        "no listed type carries a forbidden operator or conversion trait, no sealed type ",
        "has a public constructor beyond its factory, and the axis types meet a plain ",
        "integer only at home and at the reviewed sites"
    ),
    reference: concat!(
        "docs/api-frozen.toml, docs/scalar-sites.toml ",
        "and docs/adr/0011-typed-axes-and-direction-as-a-datum.md"
    ),
    run,
};

/// Check every workspace member against the three tables. Takes no arguments.
fn run(_arguments: &[String]) -> io::Result<Vec<String>> {
    let root = shared::workspace_root()?;
    let control = Control::read(&root)?;
    let members = workspace_crates(&root)?;
    let mut violations = Vec::new();
    let mut reach = Reach::default();

    for member in &members {
        for (folder, only_tests) in [("src", false), ("tests", true)] {
            let directory = member.directory.join(folder);
            for source in shared::rust_sources(&directory)? {
                let place = Place::of(member, &source, &directory);
                let read = scan(&without_prose(&fs::read_to_string(&source)?), only_tests);
                check_source(&read, &control, &place, &mut violations);
                reach.record(&read, &place, &control);
            }
        }
    }

    check_control(&control, &members, &reach, &mut violations);
    Ok(violations)
}

/// One finding, kept beside its line so a file reports in the order it reads.
#[derive(Debug)]
struct Finding {
    /// The line the finding is on.
    line: usize,
    /// What there is to say about it.
    message: String,
}

/// Apply all three tables to one source file, in the order the file reads.
fn check_source(read: &Scan, control: &Control, place: &Place, violations: &mut Vec<String>) {
    let mut found = Vec::new();
    check_no_impl(read, control, place, &mut found);
    check_no_public_constructor(read, control, place, &mut found);
    check_scalar_channel(read, control, place, &mut found);
    found.sort_by_key(|finding| finding.line);
    violations.extend(found.into_iter().map(|finding| finding.message));
}

/// Reject an application of a forbidden trait to a listed type, however it is written.
fn check_no_impl(read: &Scan, control: &Control, place: &Place, violations: &mut Vec<Finding>) {
    for rule in &control.forbidden {
        for applied in &read.trait_impls {
            if rule.forbids(&applied.trait_name, &applied.type_name) {
                violations.push(Finding {
                    line: applied.line,
                    message: format!(
                        "{at}: `impl {trait_name} for {type_name}`; `[[no_impl]]` of {FROZEN} \
                         forbids {type_name} from carrying {trait_name}",
                        at = place.at(applied.line),
                        trait_name = applied.trait_name,
                        type_name = applied.type_name,
                    ),
                });
            }
        }
        for derived in &read.derives {
            if rule.forbids(&derived.trait_name, &derived.type_name) {
                violations.push(Finding {
                    line: derived.line,
                    message: format!(
                        "{at}: {type_name} derives {trait_name}; a derived trait is an \
                         implementation, and `[[no_impl]]` of {FROZEN} forbids this one",
                        at = place.at(derived.line),
                        trait_name = derived.trait_name,
                        type_name = derived.type_name,
                    ),
                });
            }
        }
    }
    for renamed in &read.renames {
        if control.watches(&renamed.original) {
            violations.push(Finding {
                line: renamed.line,
                message: format!(
                    "{at}: `{original} as {alias}`; this gate matches names as they are \
                     written, so renaming {original} on import would put it beyond every \
                     check {FROZEN} states about it",
                    at = place.at(renamed.line),
                    original = renamed.original,
                    alias = renamed.alias,
                ),
            });
        }
    }
}

/// Reject any way of building a sealed type other than its named factory.
fn check_no_public_constructor(
    read: &Scan,
    control: &Control,
    place: &Place,
    violations: &mut Vec<Finding>,
) {
    for sealed in &control.sealed {
        if !same_crate(&place.crate_name, &sealed.crate_name) {
            continue;
        }
        for declared in &read.declarations {
            let (true, Some(how)) = (declared.name == sealed.type_name, declared.open) else {
                continue;
            };
            violations.push(Finding {
                line: declared.line,
                message: format!(
                    "{at}: {type_name} is declared with {how}, which is a public constructor; \
                     `[[no_public_constructor]]` of {FROZEN} allows {factory}",
                    at = place.at(declared.line),
                    type_name = sealed.type_name,
                    factory = permitted(&sealed.factory),
                ),
            });
        }
        check_sealed_functions(read, sealed, place, violations);
    }
}

/// Reject a public associated function, or a derived `Default`, that returns a sealed type.
fn check_sealed_functions(
    read: &Scan,
    sealed: &Sealed,
    place: &Place,
    violations: &mut Vec<Finding>,
) {
    for made in &read.associated {
        if made.type_name != sealed.type_name || !made.builds() {
            continue;
        }
        if !sealed.factory.is_empty() && made.name == sealed.factory {
            continue;
        }
        violations.push(Finding {
            line: made.line,
            message: format!(
                "{at}: `{type_name}::{name}` is a public associated function returning the \
                 type; `[[no_public_constructor]]` of {FROZEN} allows {factory}",
                at = place.at(made.line),
                type_name = sealed.type_name,
                name = made.name,
                factory = permitted(&sealed.factory),
            ),
        });
    }
    for derived in &read.derives {
        if derived.type_name == sealed.type_name
            && CONSTRUCTING_DERIVES.contains(&derived.trait_name.as_str())
        {
            violations.push(Finding {
                line: derived.line,
                message: format!(
                    "{at}: {type_name} derives {trait_name}, whose associated function builds \
                     one; `[[no_public_constructor]]` of {FROZEN} allows {factory}",
                    at = place.at(derived.line),
                    type_name = sealed.type_name,
                    trait_name = derived.trait_name,
                    factory = permitted(&sealed.factory),
                ),
            });
        }
    }
}

/// Reject a raw-integer crossing outside the home modules and the reviewed sites.
fn check_scalar_channel(
    read: &Scan,
    control: &Control,
    place: &Place,
    violations: &mut Vec<Finding>,
) {
    for channel in &control.channels {
        for used in &read.paths {
            if used.test_only || !channel.crosses(&used.type_name, &used.member) {
                continue;
            }
            if channel.at_home(place) || control.allows(&place.crate_name, &used.item) {
                continue;
            }
            violations.push(Finding {
                line: used.line,
                message: format!(
                    "{at}: {item} calls `{type_name}::{member}`; the untyped channel is open \
                     in {home} and at the items listed in {allowlist}, and nowhere else \
                     (ADR 0011)",
                    at = place.at(used.line),
                    item = named(&used.item),
                    type_name = used.type_name,
                    member = used.member,
                    home = channel.home,
                    allowlist = channel.allowlist,
                ),
            });
        }
    }
}

/// Reject a control-file entry that names something the workspace does not have.
///
/// The tables are written before the code they govern, which is what keeps them from being
/// written to pass it. The cost is that an entry can name a crate that was renamed, a home
/// module that was moved, or a site that was deleted, and each of those is an entry that
/// silently stopped being a control. All three are findings here.
fn check_control(
    control: &Control,
    members: &[CrateSource],
    reach: &Reach,
    violations: &mut Vec<String>,
) {
    let names: Vec<&str> = members.iter().map(|member| member.name.as_str()).collect();
    for sealed in &control.sealed {
        if !names
            .iter()
            .any(|name| same_crate(name, &sealed.crate_name))
        {
            violations.push(format!(
                "{FROZEN}: `[[no_public_constructor]]` names `{crate_name}::{type_name}`, \
                 and `{crate_name}` is not a workspace member",
                crate_name = sealed.crate_name,
                type_name = sealed.type_name,
            ));
        }
    }
    for channel in &control.channels {
        check_homes(channel, reach, violations);
    }
    for site in &control.sites {
        check_site(site, &names, reach, violations);
    }
}

/// Reject a home module the workspace no longer has.
fn check_homes(channel: &Channel, reach: &Reach, violations: &mut Vec<String>) {
    for home in &channel.homes {
        if !reach.modules.contains(home) {
            violations.push(format!(
                "{FROZEN}: `[[scalar_channel]]` opens the channel in `{crate_name}::{module}`, \
                 which is not a module of this workspace",
                crate_name = home.0,
                module = home.1,
            ));
        }
    }
}

/// Reject an allowlist entry that permits nothing, in either of the two ways it can.
fn check_site(site: &Site, names: &[&str], reach: &Reach, violations: &mut Vec<String>) {
    let listed = format!(
        "{crate_name}::{item}",
        crate_name = site.crate_name,
        item = site.item
    );
    if !names.iter().any(|name| same_crate(name, &site.crate_name)) {
        violations.push(format!(
            "{allowlist}: lists `{listed}`, and `{crate_name}` is not a workspace member",
            allowlist = site.allowlist,
            crate_name = site.crate_name,
        ));
        return;
    }
    let declared = reach.items.iter().any(|(crate_name, items)| {
        same_crate(crate_name, &site.crate_name) && items.contains(&site.item)
    });
    if !declared {
        violations.push(format!(
            "{allowlist}: lists `{listed}`, which that crate declares no such item for; an \
             entry naming an item that does not exist is an entry no gate can check",
            allowlist = site.allowlist,
        ));
        return;
    }
    let used = reach
        .users
        .iter()
        .any(|(crate_name, item)| same_crate(crate_name, &site.crate_name) && *item == site.item);
    if !used {
        violations.push(format!(
            "{allowlist}: lists `{listed}`, which makes no call the channel would reject; \
             a permission for nothing is a permission that has stopped being read",
            allowlist = site.allowlist,
        ));
    }
}

/// How a violation names the one constructor a sealed type is allowed, if it has one.
fn permitted(factory: &str) -> String {
    if factory.is_empty() {
        return "none at all".to_owned();
    }
    format!("only `{factory}`")
}

/// How a violation names an item, including the one that is not inside any.
fn named(item: &str) -> String {
    if item.is_empty() {
        return "crate scope".to_owned();
    }
    format!("`{item}`")
}

/// Whether two crate names are the same crate, written either way.
///
/// `docs/api-frozen.toml` spells a crate as a Rust path, `jlreq_line`, and
/// `docs/scalar-sites.toml` spells it as the package name, `jlreq-line`. Both name one
/// crate, and the two spellings differ by exactly this substitution.
fn same_crate(one: &str, other: &str) -> bool {
    one.replace('-', "_") == other.replace('-', "_")
}

/// What the scan of every source leaves behind for the control-file checks.
#[derive(Debug, Default)]
struct Reach {
    /// Every item each crate declares outside its tests, named as an allowlist names one.
    items: BTreeMap<String, BTreeSet<String>>,
    /// Every crate and item that reaches into the untyped channel.
    users: BTreeSet<(String, String)>,
    /// Every crate and module the workspace has, so a home cannot name a moved one.
    modules: BTreeSet<(String, String)>,
}

impl Reach {
    /// Remember what one source contributes to the control-file checks.
    fn record(&mut self, read: &Scan, place: &Place, control: &Control) {
        self.modules
            .insert((place.crate_name.clone(), place.module.clone()));
        self.items
            .entry(place.crate_name.clone())
            .or_default()
            .extend(read.items.iter().cloned());
        for used in &read.paths {
            let crossing = control
                .channels
                .iter()
                .any(|channel| channel.crosses(&used.type_name, &used.member));
            if crossing && !used.test_only {
                self.users
                    .insert((place.crate_name.clone(), used.item.clone()));
            }
        }
    }
}

/// Where a finding is: the crate, the file, and the module the file is.
#[derive(Debug)]
struct Place {
    /// The package name of the crate the file belongs to.
    crate_name: String,
    /// The crate and the path inside it, with `/` on every platform so the report reads
    /// the same wherever it runs.
    file: String,
    /// The module path inside the crate, `axis` or `generated::rules` or the empty root.
    module: String,
}

impl Place {
    /// Name one source file of one crate.
    fn of(member: &CrateSource, source: &Path, directory: &Path) -> Self {
        let inside = shared::relative_name(source, &member.directory).replace('\\', "/");
        Self {
            crate_name: member.name.clone(),
            file: format!("{name}/{inside}", name = member.name),
            module: module_path(source, directory),
        }
    }

    /// Name one line of it.
    fn at(&self, line: usize) -> String {
        format!("{file}:{line}", file = self.file)
    }
}

/// The module path a source file is, relative to the directory that roots the crate.
fn module_path(source: &Path, directory: &Path) -> String {
    let inside = source.strip_prefix(directory).unwrap_or(source);
    let mut segments = Vec::new();
    for component in inside.components() {
        let Some(part) = component.as_os_str().to_str() else {
            continue;
        };
        let part = part.strip_suffix(".rs").unwrap_or(part);
        if matches!(part, "lib" | "main" | "mod") {
            continue;
        }
        segments.push(part);
    }
    segments.join("::")
}

/// A workspace member: what its manifest calls it, and where it lives.
#[derive(Debug)]
struct CrateSource {
    /// The package name, as its own manifest declares it.
    name: String,
    /// The directory holding that manifest.
    directory: PathBuf,
}

/// Every workspace member, in manifest order.
///
/// `shared::core_crates` answers the layout core, which is the wrong set here: the second
/// allowlist entry this design anticipates is `jlreq-conform`'s bridge to the case format,
/// and a channel call in `xtask` would be as much of a finding as one anywhere else. The
/// right home for this is `shared`, next to `core_crates`; it is written here because a
/// gate does not edit the module every other gate shares.
fn workspace_crates(root: &Path) -> io::Result<Vec<CrateSource>> {
    let manifest = fs::read_to_string(root.join("Cargo.toml"))?;
    let members = workspace_members(&manifest);
    if members.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Cargo.toml declares no workspace members",
        ));
    }
    let mut found = Vec::new();
    for member in members {
        let directory = root.join(&member);
        let manifest = fs::read_to_string(directory.join("Cargo.toml"))?;
        let name = package_name(&manifest).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{member}/Cargo.toml declares no package name"),
            )
        })?;
        found.push(CrateSource {
            name: name.to_owned(),
            directory,
        });
    }
    Ok(found)
}

/// Read the member paths out of a workspace manifest.
fn workspace_members(manifest: &str) -> Vec<String> {
    let mut members = Vec::new();
    let mut inside = false;
    let mut open = false;
    for raw in manifest.lines() {
        let line = without_comment(raw).trim();
        if open {
            members.extend(quoted_values(line).into_iter().map(str::to_owned));
            open = !line.contains(']');
            continue;
        }
        if let Some(header) = table_header(line) {
            inside = header == "workspace";
            continue;
        }
        if let (true, Some((key, value))) = (inside, line.split_once('=')) {
            if key.trim() == "members" {
                members.extend(quoted_values(value).into_iter().map(str::to_owned));
                open = !value.contains(']');
            }
        }
    }
    members
}

/// Read the package name out of a crate manifest.
fn package_name(manifest: &str) -> Option<&str> {
    let mut inside = false;
    for raw in manifest.lines() {
        let line = without_comment(raw).trim();
        if let Some(header) = table_header(line) {
            inside = header == "package";
            continue;
        }
        if let (true, Some((key, value))) = (inside, line.split_once('=')) {
            if key.trim() == "name" {
                return quoted_values(value).first().copied();
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------------------
// The control files.
// ---------------------------------------------------------------------------------------

/// The three tables this gate enforces, and the allowlist one of them names.
#[derive(Debug)]
struct Control {
    /// `[[no_impl]]`, one entry per table.
    forbidden: Vec<Forbidden>,
    /// `[[no_public_constructor]]`, one entry per table.
    sealed: Vec<Sealed>,
    /// `[[scalar_channel]]`, one entry per table.
    channels: Vec<Channel>,
    /// Every `[[site]]` of every allowlist the channels name.
    sites: Vec<Site>,
}

/// One `[[no_impl]]` entry: traits no listed type may carry.
#[derive(Debug)]
struct Forbidden {
    /// The types the entry closes.
    types: BTreeSet<String>,
    /// The traits it closes them against.
    traits: BTreeSet<String>,
}

impl Forbidden {
    /// Whether this entry forbids `trait_name` on `type_name`.
    fn forbids(&self, trait_name: &str, type_name: &str) -> bool {
        self.types.contains(type_name) && self.traits.contains(trait_name)
    }
}

/// One `[[no_public_constructor]]` entry.
#[derive(Debug)]
struct Sealed {
    /// The crate declaring the type, as `docs/api-frozen.toml` spells it.
    crate_name: String,
    /// The type itself.
    type_name: String,
    /// The one associated function permitted to build it; empty when none is.
    factory: String,
}

/// One `[[scalar_channel]]` entry.
#[derive(Debug)]
struct Channel {
    /// The types whose untyped channel is narrowed.
    types: BTreeSet<String>,
    /// The functions that are the channel.
    methods: BTreeSet<String>,
    /// The modules the channel is open in, as the entry states them.
    home: String,
    /// Those modules, as a crate and a module path.
    homes: Vec<(String, String)>,
    /// The file listing every other site, relative to the workspace root.
    allowlist: String,
}

impl Channel {
    /// Whether `type_name::member` is a crossing this entry governs.
    fn crosses(&self, type_name: &str, member: &str) -> bool {
        self.types.contains(type_name) && self.methods.contains(member)
    }

    /// Whether `place` is one of the modules the channel is open in.
    ///
    /// A submodule of a home is part of it: the entry names where the types are defined,
    /// and a definition split across a private submodule is the same home.
    fn at_home(&self, place: &Place) -> bool {
        self.homes.iter().any(|(crate_name, module)| {
            same_crate(crate_name, &place.crate_name)
                && (place.module == *module || place.module.starts_with(&format!("{module}::")))
        })
    }
}

/// One `[[site]]` of an allowlist: an item the channel is open at.
#[derive(Debug)]
struct Site {
    /// The crate the item is declared in, as the package name.
    crate_name: String,
    /// The item, named as the enclosing item a call is attributed to.
    item: String,
    /// The file this entry was read from, for the message when it has gone stale.
    allowlist: String,
}

impl Control {
    /// Read the tables and the allowlists they name.
    fn read(root: &Path) -> io::Result<Self> {
        let frozen = fs::read_to_string(root.join(FROZEN))?;
        let forbidden = read_forbidden(&frozen)?;
        let sealed = read_sealed(&frozen)?;
        let channels = read_channels(&frozen)?;
        let mut sites = Vec::new();
        for channel in &channels {
            let document = fs::read_to_string(root.join(&channel.allowlist))?;
            sites.extend(read_sites(&document, &channel.allowlist)?);
        }
        Ok(Self {
            forbidden,
            sealed,
            channels,
            sites,
        })
    }

    /// Whether a name is one this gate matches on, and so one a rename would hide.
    fn watches(&self, name: &str) -> bool {
        let forbidden = self
            .forbidden
            .iter()
            .any(|rule| rule.types.contains(name) || rule.traits.contains(name));
        forbidden
            || self
                .channels
                .iter()
                .any(|channel| channel.types.contains(name))
    }

    /// Whether an item is one the allowlist opens the channel at.
    fn allows(&self, crate_name: &str, item: &str) -> bool {
        !item.is_empty()
            && self
                .sites
                .iter()
                .any(|site| same_crate(&site.crate_name, crate_name) && site.item == item)
    }
}

/// Read `[[no_impl]]`, which must state both of its lists.
fn read_forbidden(document: &str) -> io::Result<Vec<Forbidden>> {
    let mut rules = Vec::new();
    for entry in required(document, "no_impl")? {
        rules.push(Forbidden {
            types: entry.set("no_impl", "types")?,
            traits: entry.set("no_impl", "traits")?,
        });
    }
    Ok(rules)
}

/// Read `[[no_public_constructor]]`, whose empty factory means "no constructor at all".
fn read_sealed(document: &str) -> io::Result<Vec<Sealed>> {
    let mut sealed = Vec::new();
    for entry in required(document, "no_public_constructor")? {
        let path = entry.text("no_public_constructor", "type")?;
        let (crate_name, type_name) = path.rsplit_once("::").ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{FROZEN}: `[[no_public_constructor]]` type `{path}` names no crate"),
            )
        })?;
        sealed.push(Sealed {
            crate_name: crate_name.to_owned(),
            type_name: type_name.to_owned(),
            factory: entry.text("no_public_constructor", "factory")?.to_owned(),
        });
    }
    Ok(sealed)
}

/// Read `[[scalar_channel]]`, including the home modules and the allowlist it names.
fn read_channels(document: &str) -> io::Result<Vec<Channel>> {
    let mut channels = Vec::new();
    for entry in required(document, "scalar_channel")? {
        let home = entry.text("scalar_channel", "home")?.to_owned();
        channels.push(Channel {
            types: entry.set("scalar_channel", "types")?,
            methods: entry.set("scalar_channel", "methods")?,
            homes: home_modules(&home),
            home,
            allowlist: entry.text("scalar_channel", "allowlist")?.to_owned(),
        });
    }
    Ok(channels)
}

/// Split `jlreq-unit::axis, jlreq-unit::length` into the crates and modules it names.
fn home_modules(home: &str) -> Vec<(String, String)> {
    home.split(',')
        .filter_map(|one| one.trim().split_once("::"))
        .map(|(crate_name, module)| (crate_name.to_owned(), module.to_owned()))
        .collect()
}

/// Read every `[[site]]` of an allowlist. An empty allowlist is a valid one.
fn read_sites(document: &str, allowlist: &str) -> io::Result<Vec<Site>> {
    let mut sites = Vec::new();
    for entry in array_of_tables(document, "site") {
        sites.push(Site {
            crate_name: entry.text(allowlist, "crate")?.to_owned(),
            item: entry.text(allowlist, "item")?.to_owned(),
            allowlist: allowlist.to_owned(),
        });
        entry.text(allowlist, "why")?;
    }
    Ok(sites)
}

/// Read an array of tables that must be there.
///
/// A table this gate enforces and cannot find is a gate that cannot run, which is a
/// failure and never a pass: an absent table says nothing about the invariant.
fn required(document: &str, name: &str) -> io::Result<Vec<Entry>> {
    let entries = array_of_tables(document, name);
    if entries.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{FROZEN} states no `[[{name}]]` table, which this gate enforces"),
        ));
    }
    Ok(entries)
}

/// One `[[table]]` entry: every key it states, each as the strings it holds.
///
/// A scalar is a list of one, because no key this gate reads means anything different
/// depending on which of the two it was written as.
#[derive(Debug, Default, PartialEq, Eq)]
struct Entry {
    /// The keys, in the order a `BTreeMap` keeps them.
    values: BTreeMap<String, Vec<String>>,
}

impl Entry {
    /// One key's single value, or an error naming the entry it is missing from.
    fn text(&self, whose: &str, key: &str) -> io::Result<&str> {
        self.values
            .get(key)
            .and_then(|values| values.first())
            .map(String::as_str)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{whose}: an entry states no `{key}`"),
                )
            })
    }

    /// One key's values as a set, which must not be empty.
    fn set(&self, whose: &str, key: &str) -> io::Result<BTreeSet<String>> {
        let values: BTreeSet<String> = self
            .values
            .get(key)
            .map(|values| values.iter().cloned().collect())
            .unwrap_or_default();
        if values.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{whose}: an entry states no `{key}`"),
            ));
        }
        Ok(values)
    }
}

/// Read every `[[name]]` entry out of a TOML document.
///
/// Hand-rolled for the reason stated on `purity`'s manifest scan, and it understands only
/// what this repository's control files are written in: array-of-table headers, string
/// values, and arrays of strings on one line or several.
fn array_of_tables(document: &str, name: &str) -> Vec<Entry> {
    let header = format!("[[{name}]]");
    let mut entries: Vec<Entry> = Vec::new();
    let mut inside = false;
    let mut open = String::new();

    for raw in document.lines() {
        let line = without_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        if !open.is_empty() {
            extend(entries.last_mut(), &open, line);
            if line.contains(']') {
                open.clear();
            }
            continue;
        }
        if line.starts_with('[') {
            inside = line == header;
            if inside {
                entries.push(Entry::default());
            }
            continue;
        }
        if inside {
            open = read_pair(entries.last_mut(), line);
        }
    }
    entries
}

/// Read one `key = value` line, answering the key when its array has not closed yet.
fn read_pair(entry: Option<&mut Entry>, line: &str) -> String {
    let Some((key, value)) = line.split_once('=') else {
        return String::new();
    };
    let key = key.trim().to_owned();
    extend(entry, &key, value);
    if value.trim().starts_with('[') && !value.contains(']') {
        return key;
    }
    String::new()
}

/// Add the strings written on `line` to one key of one entry.
fn extend(entry: Option<&mut Entry>, key: &str, line: &str) {
    let Some(entry) = entry else {
        return;
    };
    entry
        .values
        .entry(key.to_owned())
        .or_default()
        .extend(quoted_values(line).into_iter().map(str::to_owned));
}

/// The name inside a `[table]` header, if the line is one.
fn table_header(line: &str) -> Option<&str> {
    line.strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
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

// ---------------------------------------------------------------------------------------
// Reading a source file as code.
// ---------------------------------------------------------------------------------------

/// Blank out everything in a source that is not code, keeping every newline.
///
/// Comments — line, block, and nested block — string literals, raw strings, byte strings
/// and character literals become spaces, so prose naming a forbidden trait is prose and a
/// fixture holding a forbidden call is a fixture. Newlines survive, so a finding still
/// names the line it is on. A lifetime is not a character literal and stays: `&'a str` is
/// code. `shared::code_only` strips `//` and nothing else, which is not enough for a gate
/// whose own tests are fixtures full of the constructs it rejects.
fn without_prose(source: &str) -> String {
    let text: Vec<char> = source.chars().collect();
    let mut kept = String::with_capacity(source.len());
    let mut at = 0;
    while at < text.len() {
        let Some(end) = literal_end(&text, at) else {
            if let Some(character) = text.get(at) {
                kept.push(*character);
            }
            at = at.saturating_add(1);
            continue;
        };
        for character in text.get(at..end).unwrap_or_default() {
            kept.push(if *character == '\n' { '\n' } else { ' ' });
        }
        at = end;
    }
    kept
}

/// The end of the comment or literal starting at `at`, if one starts there.
fn literal_end(text: &[char], at: usize) -> Option<usize> {
    let here = *text.get(at)?;
    let next = text.get(at.saturating_add(1)).copied();
    let end = match (here, next) {
        ('/', Some('/')) => until_newline(text, at),
        ('/', Some('*')) => block_comment_end(text, at),
        ('"', _) => string_end(text, at.saturating_add(1)),
        ('\'', _) => character_end(text, at)?,
        _ => prefixed_string_end(text, at)?,
    };
    Some(end.min(text.len()))
}

/// The index of the newline ending a line comment, or the end of the source.
fn until_newline(text: &[char], at: usize) -> usize {
    let mut cursor = at;
    while let Some(&character) = text.get(cursor) {
        if character == '\n' {
            break;
        }
        cursor = cursor.saturating_add(1);
    }
    cursor
}

/// The index past the `*/` closing a block comment, which may nest.
fn block_comment_end(text: &[char], at: usize) -> usize {
    let mut cursor = at.saturating_add(2);
    let mut depth = 1usize;
    while depth > 0 {
        let Some(&character) = text.get(cursor) else {
            return cursor;
        };
        let next = text.get(cursor.saturating_add(1)).copied();
        match (character, next) {
            ('/', Some('*')) => depth = depth.saturating_add(1),
            ('*', Some('/')) => depth = depth.saturating_sub(1),
            _ => {
                cursor = cursor.saturating_add(1);
                continue;
            },
        }
        cursor = cursor.saturating_add(2);
    }
    cursor
}

/// The index past the `"` closing a string that opened at `from`.
fn string_end(text: &[char], from: usize) -> usize {
    let mut cursor = from;
    while let Some(&character) = text.get(cursor) {
        match character {
            '\\' => cursor = cursor.saturating_add(2),
            '"' => return cursor.saturating_add(1),
            _ => cursor = cursor.saturating_add(1),
        }
    }
    cursor
}

/// The index past a raw string's closing `"` and its hashes.
fn raw_string_end(text: &[char], from: usize, hashes: usize) -> usize {
    let mut cursor = from;
    while let Some(&character) = text.get(cursor) {
        cursor = cursor.saturating_add(1);
        if character != '"' {
            continue;
        }
        let closed = (0..hashes).all(|step| text.get(cursor.saturating_add(step)) == Some(&'#'));
        if closed {
            return cursor.saturating_add(hashes);
        }
    }
    cursor
}

/// The index past a character literal, or `None` for a lifetime.
fn character_end(text: &[char], at: usize) -> Option<usize> {
    let after = at.saturating_add(1);
    if text.get(after) == Some(&'\\') {
        let mut cursor = at.saturating_add(2);
        while let Some(&character) = text.get(cursor) {
            cursor = cursor.saturating_add(1);
            if character == '\'' {
                return Some(cursor);
            }
        }
        return Some(cursor);
    }
    let closing = at.saturating_add(2);
    if text.get(closing) == Some(&'\'') {
        return Some(closing.saturating_add(1));
    }
    None
}

/// The index past a raw or byte string, if one starts at `at`.
fn prefixed_string_end(text: &[char], at: usize) -> Option<usize> {
    let preceded = at
        .checked_sub(1)
        .and_then(|before| text.get(before))
        .is_some_and(|character| is_word(*character));
    if preceded {
        return None;
    }
    let mut cursor = at;
    if text.get(cursor) == Some(&'b') {
        cursor = cursor.saturating_add(1);
    }
    if text.get(cursor) == Some(&'r') {
        cursor = cursor.saturating_add(1);
        let mut hashes = 0usize;
        while text.get(cursor) == Some(&'#') {
            hashes = hashes.saturating_add(1);
            cursor = cursor.saturating_add(1);
        }
        if text.get(cursor) != Some(&'"') {
            return None;
        }
        return Some(raw_string_end(text, cursor.saturating_add(1), hashes));
    }
    if cursor > at && text.get(cursor) == Some(&'"') {
        return Some(string_end(text, cursor.saturating_add(1)));
    }
    None
}

/// Whether a character can appear inside an identifier.
fn is_word(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

/// Whether a token is an identifier rather than punctuation or a number.
fn is_identifier(token: &str) -> bool {
    token
        .chars()
        .next()
        .is_some_and(|character| character.is_alphabetic() || character == '_')
}

/// One lexical token: an identifier, a number, or one punctuation character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Token<'s> {
    /// The token itself.
    text: &'s str,
    /// The line it was written on, counting from one.
    line: usize,
}

/// Split code into tokens, remembering which line each was written on.
fn tokenize(code: &str) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();
    let mut line = 1usize;
    let mut word: Option<usize> = None;
    for (offset, character) in code.char_indices() {
        if is_word(character) {
            word = word.or(Some(offset));
            continue;
        }
        if let Some(start) = word.take() {
            push_token(code, start, offset, line, &mut tokens);
        }
        if character == '\n' {
            line = line.saturating_add(1);
            continue;
        }
        if character.is_whitespace() {
            continue;
        }
        let end = offset.saturating_add(character.len_utf8());
        push_token(code, offset, end, line, &mut tokens);
    }
    if let Some(start) = word {
        push_token(code, start, code.len(), line, &mut tokens);
    }
    tokens
}

/// Append one token, if the range names one.
fn push_token<'s>(
    code: &'s str,
    start: usize,
    end: usize,
    line: usize,
    tokens: &mut Vec<Token<'s>>,
) {
    if let Some(text) = code.get(start..end) {
        tokens.push(Token { text, line });
    }
}

// ---------------------------------------------------------------------------------------
// The scan.
// ---------------------------------------------------------------------------------------

/// A trait applied to a type, by an `impl` or by a `derive`.
#[derive(Debug, PartialEq, Eq)]
struct Applied {
    /// The trait, as the name written at the application.
    trait_name: String,
    /// The type it was applied to.
    type_name: String,
    /// Where.
    line: usize,
}

/// A `Type::member` path, wherever it is written.
#[derive(Debug, PartialEq, Eq)]
struct PathUse {
    /// The path segment before the `::`.
    type_name: String,
    /// The one after it.
    member: String,
    /// The item the use is inside, empty at crate scope.
    item: String,
    /// Where.
    line: usize,
    /// Whether it is inside a `#[cfg(test)]` item or an integration test.
    test_only: bool,
}

/// A `use … as …`, which renames a name this gate matches on.
#[derive(Debug, PartialEq, Eq)]
struct Rename {
    /// The name as it is declared.
    original: String,
    /// The name it was imported under.
    alias: String,
    /// Where.
    line: usize,
}

/// A declared `struct` or `enum`, and whether a caller elsewhere can build one.
#[derive(Debug, PartialEq, Eq)]
struct Declaration {
    /// The type's name.
    name: String,
    /// How a caller can build one, when one can.
    open: Option<&'static str>,
    /// Where.
    line: usize,
}

/// A function declared inside an `impl` block.
#[derive(Debug, PartialEq, Eq)]
struct Associated {
    /// The type the `impl` block is for.
    type_name: String,
    /// The function's name.
    name: String,
    /// Whether a caller outside the crate can call it.
    public: bool,
    /// Whether it takes no receiver, which is what makes it a constructor rather than a
    /// method (`docs/design/api-spine.md` pins both definitions).
    no_receiver: bool,
    /// Whether it answers `Self`, `Result<Self, _>` or `Option<Self>`.
    returns_self: bool,
    /// Where.
    line: usize,
}

impl Associated {
    /// Whether this function is a public constructor under the pinned definition.
    fn builds(&self) -> bool {
        self.public && self.no_receiver && self.returns_self
    }
}

/// Everything one source file says that any of the three tables is about.
#[derive(Debug, Default)]
struct Scan {
    /// `impl <Trait> for <Type>`.
    trait_impls: Vec<Applied>,
    /// `#[derive(<Trait>)]` on the type it precedes.
    derives: Vec<Applied>,
    /// Every `Type::member` path.
    paths: Vec<PathUse>,
    /// Every `use … as …`.
    renames: Vec<Rename>,
    /// Every `struct` and `enum` declaration.
    declarations: Vec<Declaration>,
    /// Every function declared inside an `impl` block.
    associated: Vec<Associated>,
    /// Every item outside tests, named as an allowlist names one.
    items: BTreeSet<String>,
}

/// What the `{` about to open names.
#[derive(Debug, Default, Clone)]
enum Names {
    /// A block, a match arm, a closure: nothing this gate attributes anything to.
    #[default]
    Nothing,
    /// `impl <Type>` or `impl <Trait> for <Type>`.
    Implementation {
        /// The type the block is for.
        type_name: String,
        /// Whether a trait is being implemented, which makes its functions public.
        of_a_trait: bool,
    },
    /// `fn <name>`.
    Function(String),
}

/// One `{ … }` block.
#[derive(Debug)]
struct Frame {
    /// What it names.
    names: Names,
    /// Whether it is compiled only for tests.
    test_only: bool,
}

impl Frame {
    /// The function this frame is, if it is one.
    fn function_name(&self) -> Option<&str> {
        match &self.names {
            Names::Function(name) => Some(name),
            _ => None,
        }
    }

    /// The type this frame implements, if it implements one.
    fn implementation(&self) -> Option<(&str, bool)> {
        match &self.names {
            Names::Implementation {
                type_name,
                of_a_trait,
            } => Some((type_name, *of_a_trait)),
            _ => None,
        }
    }
}

/// Read one source file as code.
fn scan(code: &str, only_tests: bool) -> Scan {
    let tokens = tokenize(code);
    Scanner {
        tokens: &tokens,
        at: 0,
        stack: Vec::new(),
        pending: Names::Nothing,
        pending_test: false,
        derives: Vec::new(),
        only_tests,
        out: Scan::default(),
    }
    .walk()
}

/// One pass over one file's tokens.
#[derive(Debug)]
struct Scanner<'s> {
    /// The tokens, in order.
    tokens: &'s [Token<'s>],
    /// The cursor.
    at: usize,
    /// The blocks currently open, outermost first.
    stack: Vec<Frame>,
    /// What the next `{` will name.
    pending: Names,
    /// Whether a `#[cfg(test)]` applies to the next block.
    pending_test: bool,
    /// The traits of a `#[derive(..)]` waiting for the type it is on.
    derives: Vec<String>,
    /// Whether the whole file is test-only.
    only_tests: bool,
    /// What has been found.
    out: Scan,
}

impl<'s> Scanner<'s> {
    /// Walk every token once.
    fn walk(mut self) -> Scan {
        while let Some(token) = self.tokens.get(self.at).copied() {
            let start = self.at;
            self.at = self.at.saturating_add(1);
            match token.text {
                "impl" => self.read_impl(),
                "fn" => self.read_fn(start, token.line),
                "struct" | "enum" | "union" => self.read_declaration(start, token),
                "use" => self.read_use(),
                "#" => self.read_attribute(),
                "{" => self.open(),
                "}" => self.close(),
                ";" => self.settle(),
                _ => self.read_path(start, token),
            }
        }
        self.out
    }

    /// Read an `impl` header up to the `{` that opens its block.
    fn read_impl(&mut self) {
        self.derives.clear();
        self.pending = Names::Nothing;
        self.skip_generics();
        let header = self.take_header();
        let (applied, subject) = split_on_for(&header);
        let Some(type_name) = last_path_name(subject) else {
            return;
        };
        let trait_name = applied.and_then(last_path_name);
        if let Some(trait_name) = trait_name.clone() {
            self.out.trait_impls.push(Applied {
                trait_name,
                type_name: type_name.clone(),
                line: header.first().map_or(0, |token| token.line),
            });
        }
        self.pending = Names::Implementation {
            type_name,
            of_a_trait: trait_name.is_some(),
        };
    }

    /// Read a `fn` signature up to the `{` or `;` that ends it.
    fn read_fn(&mut self, start: usize, line: usize) {
        self.derives.clear();
        self.pending = Names::Nothing;
        let Some(name) = self.identifier() else {
            return;
        };
        let public = self.declared_public(start);
        self.skip_generics();
        let parameters = self.take_balanced("(", ")");
        let returns = self.take_returns();
        let owner = self.enclosing_type();
        if let Some((type_name, of_a_trait)) = owner.clone() {
            self.out.associated.push(Associated {
                returns_self: returns
                    .iter()
                    .any(|token| token.text == "Self" || token.text == type_name),
                type_name,
                name: name.clone(),
                public: public || of_a_trait,
                no_receiver: !takes_a_receiver(&parameters),
                line,
            });
        }
        if !self.test_only() {
            let item = match owner {
                Some((type_name, _)) => format!("{type_name}::{name}"),
                None => name.clone(),
            };
            self.out.items.insert(item);
        }
        self.pending = Names::Function(name);
    }

    /// Read a `struct`, `enum` or `union` declaration and how open it is.
    fn read_declaration(&mut self, start: usize, keyword: Token<'s>) {
        let Some(name) = self.identifier() else {
            return;
        };
        let public = self.declared_public(start);
        let body = self.take_declaration();
        self.out.declarations.push(Declaration {
            open: if public {
                openness(keyword.text, &body)
            } else {
                None
            },
            name: name.clone(),
            line: keyword.line,
        });
        for trait_name in std::mem::take(&mut self.derives) {
            self.out.derives.push(Applied {
                trait_name,
                type_name: name.clone(),
                line: keyword.line,
            });
        }
    }

    /// Read a `use` item, recording every name it renames.
    fn read_use(&mut self) {
        self.derives.clear();
        let mut previous: Option<Token<'s>> = None;
        while let Some(token) = self.tokens.get(self.at).copied() {
            self.at = self.at.saturating_add(1);
            if token.text == ";" {
                return;
            }
            if token.text == "as" {
                self.record_rename(previous, token.line);
            }
            previous = Some(token);
        }
    }

    /// Record one `<original> as <alias>`.
    fn record_rename(&mut self, previous: Option<Token<'s>>, line: usize) {
        let Some(original) = previous.filter(|token| is_identifier(token.text)) else {
            return;
        };
        let alias = self
            .tokens
            .get(self.at)
            .map_or("_", |token| token.text)
            .to_owned();
        self.out.renames.push(Rename {
            original: original.text.to_owned(),
            alias,
            line,
        });
    }

    /// Read an attribute, which may name derived traits or the test configuration.
    fn read_attribute(&mut self) {
        if self.peek_is("!") {
            self.at = self.at.saturating_add(1);
        }
        if !self.peek_is("[") {
            return;
        }
        let inside = self.take_balanced("[", "]");
        let Some(first) = inside.first() else {
            return;
        };
        match first.text {
            "derive" => self.derives.extend(
                inside
                    .iter()
                    .skip(1)
                    .filter(|token| is_identifier(token.text))
                    .map(|token| token.text.to_owned()),
            ),
            "cfg" if is_test_configuration(&inside) => self.pending_test = true,
            _ => {},
        }
    }

    /// Record a `Type::member` path, wherever it is written.
    fn read_path(&mut self, start: usize, token: Token<'s>) {
        if !is_identifier(token.text) {
            return;
        }
        let ahead = usize::from(self.is_qualified(start));
        let colons = (self.token_at(ahead), self.token_at(ahead.saturating_add(1)))
            == (Some(":"), Some(":"));
        let Some(member) = self.token_at(ahead.saturating_add(2)).filter(|_| colons) else {
            return;
        };
        if !is_identifier(member) {
            return;
        }
        self.out.paths.push(PathUse {
            type_name: token.text.to_owned(),
            member: member.to_owned(),
            item: self.item(),
            line: token.line,
            test_only: self.test_only(),
        });
    }

    /// Whether the identifier at `index` is the type of a `<Type>::member` path.
    ///
    /// The angle brackets of a qualified path are punctuation the scan would otherwise
    /// read straight past, which would leave `<BlockExtent>::new` one rewrite away from
    /// every check here. `Vec<BlockExtent>::new` is not that shape: there the `<` follows
    /// a name, and the type of the path is that name.
    fn is_qualified(&self, index: usize) -> bool {
        if self.token_at(0) != Some(">") {
            return false;
        }
        let Some(opening) = index.checked_sub(1) else {
            return false;
        };
        if self.tokens.get(opening).map(|token| token.text) != Some("<") {
            return false;
        }
        !opening
            .checked_sub(1)
            .and_then(|before| self.tokens.get(before))
            .is_some_and(|token| is_identifier(token.text))
    }

    /// Open a block, giving it whatever the last declaration named it.
    fn open(&mut self) {
        let inherited = self.stack.last().is_some_and(|frame| frame.test_only);
        self.stack.push(Frame {
            names: std::mem::take(&mut self.pending),
            test_only: inherited || self.pending_test,
        });
        self.pending_test = false;
        self.derives.clear();
    }

    /// Close a block.
    fn close(&mut self) {
        self.stack.pop();
    }

    /// Forget a declaration that ended without opening a block.
    fn settle(&mut self) {
        self.pending = Names::Nothing;
        self.pending_test = false;
        self.derives.clear();
    }

    /// The item a finding here belongs to, named as an allowlist names one.
    fn item(&self) -> String {
        let function = self.stack.iter().rev().find_map(Frame::function_name);
        let owner = self.stack.iter().rev().find_map(Frame::implementation);
        match (owner, function) {
            (Some((type_name, _)), Some(name)) => format!("{type_name}::{name}"),
            (None, Some(name)) => name.to_owned(),
            _ => String::new(),
        }
    }

    /// The type of the innermost `impl` block, and whether it implements a trait.
    fn enclosing_type(&self) -> Option<(String, bool)> {
        self.stack
            .iter()
            .rev()
            .find_map(Frame::implementation)
            .map(|(type_name, of_a_trait)| (type_name.to_owned(), of_a_trait))
    }

    /// Whether the cursor is inside test-only code.
    fn test_only(&self) -> bool {
        self.only_tests || self.stack.last().is_some_and(|frame| frame.test_only)
    }

    /// The text of the token `ahead` places past the cursor.
    fn token_at(&self, ahead: usize) -> Option<&'s str> {
        self.tokens
            .get(self.at.saturating_add(ahead))
            .map(|token| token.text)
    }

    /// Whether the token at the cursor is `text`.
    fn peek_is(&self, text: &str) -> bool {
        self.token_at(0) == Some(text)
    }

    /// Consume and answer the identifier at the cursor.
    fn identifier(&mut self) -> Option<String> {
        let text = self.token_at(0).filter(|text| is_identifier(text))?;
        self.at = self.at.saturating_add(1);
        Some(text.to_owned())
    }

    /// Whether the item whose keyword is at `index` is declared `pub` without restriction.
    ///
    /// `pub(crate)` is not public: a caller outside the crate cannot reach it, and every
    /// table here is about what a caller can do.
    fn declared_public(&self, index: usize) -> bool {
        let mut back = index;
        loop {
            let Some(previous) = back.checked_sub(1) else {
                return false;
            };
            let Some(token) = self.tokens.get(previous) else {
                return false;
            };
            match token.text {
                "pub" => return true,
                "const" | "async" | "unsafe" | "extern" | "default" | "static" => back = previous,
                _ => return false,
            }
        }
    }

    /// Consume a `<…>` list if one starts at the cursor.
    fn skip_generics(&mut self) {
        if !self.peek_is("<") {
            return;
        }
        let mut depth = 0usize;
        let mut previous = "";
        while let Some(token) = self.tokens.get(self.at).copied() {
            self.at = self.at.saturating_add(1);
            match token.text {
                "<" => depth = depth.saturating_add(1),
                ">" if previous != "-" => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return;
                    }
                },
                _ => {},
            }
            previous = token.text;
        }
    }

    /// Consume the tokens up to the `{`, `;` or `where` that ends a header.
    fn take_header(&mut self) -> Vec<Token<'s>> {
        let mut taken = Vec::new();
        while let Some(token) = self.tokens.get(self.at).copied() {
            if matches!(token.text, "{" | ";" | "where") {
                break;
            }
            self.at = self.at.saturating_add(1);
            taken.push(token);
        }
        taken
    }

    /// Consume a balanced `open … close` group at the cursor and answer what was inside.
    fn take_balanced(&mut self, open: &str, close: &str) -> Vec<Token<'s>> {
        let mut taken = Vec::new();
        if !self.peek_is(open) {
            return taken;
        }
        let mut depth = 0usize;
        while let Some(token) = self.tokens.get(self.at).copied() {
            self.at = self.at.saturating_add(1);
            if token.text == open {
                depth = depth.saturating_add(1);
                if depth == 1 {
                    continue;
                }
            }
            if token.text == close {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return taken;
                }
            }
            taken.push(token);
        }
        taken
    }

    /// Consume a return type, if the signature states one.
    fn take_returns(&mut self) -> Vec<Token<'s>> {
        if (self.token_at(0), self.token_at(1)) != (Some("-"), Some(">")) {
            return Vec::new();
        }
        self.at = self.at.saturating_add(2);
        self.take_header()
    }

    /// Consume the rest of a type declaration: a body, a tuple, or nothing.
    fn take_declaration(&mut self) -> Vec<Token<'s>> {
        let mut taken = Vec::new();
        let mut depth = 0usize;
        while let Some(token) = self.tokens.get(self.at).copied() {
            self.at = self.at.saturating_add(1);
            match token.text {
                "{" | "(" | "[" => depth = depth.saturating_add(1),
                "}" | ")" | "]" => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        taken.push(token);
                        return taken;
                    }
                },
                ";" if depth == 0 => return taken,
                _ => {},
            }
            taken.push(token);
        }
        taken
    }
}

/// How a caller outside the crate can build a public type, when it can.
fn openness(keyword: &str, body: &[Token<'_>]) -> Option<&'static str> {
    if keyword == "enum" {
        return Some("public variants");
    }
    if !body.iter().any(|token| matches!(token.text, "{" | "(")) {
        return Some("no fields, so its name alone is a value");
    }
    let public_field = body
        .windows(2)
        .any(|pair| matches!(pair, [first, second] if first.text == "pub" && second.text != "("));
    public_field.then_some("a public field")
}

/// Whether a parameter list starts with a receiver, which makes it a method.
fn takes_a_receiver(parameters: &[Token<'_>]) -> bool {
    parameters
        .split(|token| token.text == ",")
        .next()
        .is_some_and(|first| first.iter().any(|token| token.text == "self"))
}

/// Whether an attribute is `cfg(test)` rather than a configuration that excludes it.
fn is_test_configuration(inside: &[Token<'_>]) -> bool {
    let mentions_test = inside.iter().any(|token| token.text == "test");
    mentions_test && !inside.iter().any(|token| token.text == "not")
}

/// Split an `impl` header on the `for` that separates the trait from the type.
fn split_on_for<'t, 's>(header: &'t [Token<'s>]) -> (Option<&'t [Token<'s>]>, &'t [Token<'s>]) {
    let mut depth = 0usize;
    let mut previous = "";
    for (index, token) in header.iter().enumerate() {
        match token.text {
            "<" | "(" => depth = depth.saturating_add(1),
            ">" if previous == "-" => {},
            ">" | ")" => depth = depth.saturating_sub(1),
            "for" if depth == 0 => {
                let after = index.saturating_add(1);
                return (header.get(..index), header.get(after..).unwrap_or_default());
            },
            _ => {},
        }
        previous = token.text;
    }
    (None, header)
}

/// The name a path names: its last segment outside any `<…>` or `(…)`.
///
/// `core::ops::Add<i32>` is `Add` and `Vec<Em>` is `Vec`, which is what makes the match a
/// match on the name as written rather than on the path it was imported by.
fn last_path_name(tokens: &[Token<'_>]) -> Option<String> {
    let mut depth = 0usize;
    let mut previous = "";
    let mut found = None;
    for token in tokens {
        match token.text {
            "<" | "(" => depth = depth.saturating_add(1),
            ">" if previous == "-" => {},
            ">" | ")" => depth = depth.saturating_sub(1),
            text if depth == 0 && is_identifier(text) => found = Some(text.to_owned()),
            _ => {},
        }
        previous = token.text;
    }
    found
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{
        Control, Place, Scan, array_of_tables, home_modules, is_test_configuration, last_path_name,
        module_path, package_name, run, same_crate, scan, tokenize, without_prose,
        workspace_crates, workspace_members,
    };
    use crate::shared;

    /// The control files as the repository actually has them.
    fn control() -> Control {
        let root = shared::workspace_root().expect("the workspace root is locatable");
        Control::read(&root).expect("the control files are readable")
    }

    /// Read a fixture as one file of one crate.
    fn read(source: &str) -> Scan {
        scan(&without_prose(source), false)
    }

    /// A place inside a crate, for the checks that are about where a finding is.
    fn place(crate_name: &str, module: &str) -> Place {
        Place {
            crate_name: crate_name.to_owned(),
            file: format!("{crate_name}/src/{module}.rs"),
            module: module.to_owned(),
        }
    }

    /// Run one file's checks against the repository's own control files.
    fn findings(source: &str, at: &Place) -> Vec<String> {
        let mut violations = Vec::new();
        super::check_source(&read(source), &control(), at, &mut violations);
        violations
    }

    #[test]
    fn the_gate_holds_on_this_repository() {
        let violations = run(&[]).expect("the gate can read the workspace");
        assert!(violations.is_empty(), "found {violations:?}");
    }

    #[test]
    fn the_frozen_tables_still_say_what_this_gate_enforces() {
        let control = control();
        let forbidden = control.forbidden.first().expect("[[no_impl]] is stated");
        assert!(forbidden.traits.contains("Add"), "arithmetic is forbidden");
        assert!(forbidden.traits.contains("From"), "conversion is forbidden");
        assert!(forbidden.types.contains("Em"), "over the length types");
        let channel = control
            .channels
            .first()
            .expect("[[scalar_channel]] is stated");
        assert!(channel.types.contains("Advance"), "over the axis types");
        assert!(channel.methods.contains("new"), "the entry half");
        assert!(channel.methods.contains("units"), "and the exit half");
        assert_eq!(
            channel.homes.len(),
            2,
            "two home modules, {home:?}",
            home = channel.homes
        );
    }

    #[test]
    fn an_operator_on_a_listed_type_is_a_finding() {
        let at = place("jlreq-unit", "arith");
        let found = findings("impl Add for Em {\n    fn add(self) {}\n}\n", &at);
        assert_eq!(found.len(), 1, "found {found:?}");
        assert!(found[0].contains("jlreq-unit/src/arith.rs:1"), "{found:?}");
        assert!(found[0].contains("impl Add for Em"), "{found:?}");
    }

    #[test]
    fn an_operator_reached_by_its_path_is_the_same_finding() {
        let at = place("jlreq-unit", "arith");
        let found = findings("impl core::ops::Mul<i32> for InlineExtent {}\n", &at);
        assert_eq!(
            found.len(),
            1,
            "an import cannot evade a name match: {found:?}"
        );
    }

    #[test]
    fn an_ordering_derived_on_a_listed_type_is_a_finding() {
        let at = place("jlreq-unit", "length");
        let found = findings(
            "#[derive(Debug, Ord)]\n#[non_exhaustive]\npub struct Em(i32);\n",
            &at,
        );
        assert_eq!(
            found.len(),
            1,
            "an attribute between the two does not hide it: {found:?}"
        );
        assert!(found[0].contains("derives Ord"), "{found:?}");
    }

    #[test]
    fn a_trait_that_is_not_listed_and_a_type_that_is_not_listed_are_not_findings() {
        let at = place("jlreq-unit", "length");
        assert!(
            findings("impl Iterator for Em {}\n", &at).is_empty(),
            "Iterator is not listed"
        );
        assert!(
            findings("#[derive(PartialOrd, Ord)]\npub struct ScaleId(u8);\n", &at).is_empty(),
            "a type outside the list is covered only once the control file names it"
        );
        assert!(
            findings("impl Em {\n    pub const fn add_sat(self) {}\n}\n", &at).is_empty(),
            "an inherent block implements nothing"
        );
    }

    #[test]
    fn renaming_a_watched_name_on_import_is_a_finding() {
        let at = place("jlreq-line", "badness");
        let found = findings("use core::ops::Add as Plus;\n", &at);
        assert_eq!(found.len(), 1, "found {found:?}");
        assert!(found[0].contains("Add as Plus"), "{found:?}");
        assert!(
            findings("use core::fmt::Display as _;\n", &at).is_empty(),
            "a name no table mentions is not watched"
        );
    }

    #[test]
    fn building_an_axis_value_outside_the_home_is_a_finding() {
        let at = place("jlreq-line", "badness");
        let found = findings("fn of() {\n    let x = BlockExtent::new(3);\n}\n", &at);
        assert_eq!(found.len(), 1, "found {found:?}");
        assert!(
            found[0].contains("jlreq-line/src/badness.rs:2"),
            "{found:?}"
        );
        assert!(
            found[0].contains("`of` calls `BlockExtent::new`"),
            "{found:?}"
        );
    }

    #[test]
    fn reading_one_back_out_by_its_path_is_a_finding() {
        let at = place("jlreq-line", "badness");
        let found = findings(
            "impl Badness {\n    fn of(x: T) {\n        x.map(InlineOffset::units);\n    }\n}\n",
            &at,
        );
        assert_eq!(found.len(), 1, "found {found:?}");
        assert!(
            found[0].contains("`Badness::of`"),
            "the item is the one an allowlist names: {found:?}"
        );
    }

    #[test]
    fn the_same_call_at_home_is_not_a_finding() {
        for module in ["axis", "length"] {
            let at = place("jlreq-unit", module);
            assert!(
                findings("fn make() {\n    BlockExtent::new(3);\n}\n", &at).is_empty(),
                "{module} is where the channel lives"
            );
        }
    }

    #[test]
    fn the_same_call_in_another_module_of_the_same_crate_is_a_finding() {
        let at = place("jlreq-unit", "arith");
        let found = findings("fn make() {\n    BlockExtent::new(3);\n}\n", &at);
        assert_eq!(
            found.len(),
            1,
            "the home is a module and not a crate: {found:?}"
        );
    }

    #[test]
    fn a_qualified_path_is_the_same_call_written_with_brackets() {
        let at = place("jlreq-line", "badness");
        let found = findings("fn of() {\n    let x = <BlockExtent>::new(3);\n}\n", &at);
        assert_eq!(found.len(), 1, "the brackets are not an escape: {found:?}");
        assert!(found[0].contains("`BlockExtent::new`"), "{found:?}");
        assert!(
            findings("fn of() {\n    let v: Wrapper<BlockExtent> = w;\n}\n", &at).is_empty(),
            "a type argument is not the type a path is on"
        );
    }

    #[test]
    fn a_length_that_is_not_an_axis_type_is_not_on_the_channel() {
        let at = place("jlreq-spacing", "table");
        assert!(
            findings("fn amount() {\n    Em::units(Em::HALF);\n}\n", &at).is_empty(),
            "Em is a writing-system fraction and has no axis to be put on the wrong one of"
        );
    }

    #[test]
    fn a_call_in_a_test_module_is_not_a_finding() {
        let at = place("jlreq-line", "badness");
        let source =
            "#[cfg(test)]\nmod tests {\n    fn case() {\n        BlockExtent::new(3);\n    }\n}\n";
        assert!(
            findings(source, &at).is_empty(),
            "a unit test must build the values it exercises, and cannot hand one to a caller"
        );
        let source = "#[cfg(test)]\nmod tests {\n    impl Add for Em {}\n}\n";
        assert_eq!(
            findings(source, &at).len(),
            1,
            "an impl under cfg(test) still changes what the type is where the suite runs"
        );
    }

    #[test]
    fn a_public_constructor_on_a_sealed_type_is_a_finding() {
        let at = place("jlreq-line", "feasible");
        let found = findings("impl Feasible {\n    pub fn new() -> Self {}\n}\n", &at);
        assert_eq!(found.len(), 1, "found {found:?}");
        assert!(found[0].contains("`Feasible::new`"), "{found:?}");
        assert!(found[0].contains("only `compute`"), "{found:?}");
    }

    #[test]
    fn the_named_factory_is_not_a_finding() {
        let at = place("jlreq-line", "feasible");
        assert!(
            findings(
                "impl Feasible {\n    pub fn compute(x: T) -> Option<Self> {}\n}\n",
                &at
            )
            .is_empty(),
            "the factory is the one way in"
        );
        assert!(
            findings("impl Feasible {\n    fn new() -> Self {}\n}\n", &at).is_empty(),
            "a private associated function is not a public constructor"
        );
        assert!(
            findings(
                "impl Feasible {\n    pub fn breaks(self) -> Self {}\n}\n",
                &at
            )
            .is_empty(),
            "a method takes a receiver, so a caller already had one"
        );
    }

    #[test]
    fn a_type_sealed_with_no_factory_admits_no_constructor_at_all() {
        let at = place("jlreq-line", "feasible");
        let found = findings(
            "impl FeasibleBreak {\n    pub fn compute() -> Self {}\n}\n",
            &at,
        );
        assert_eq!(found.len(), 1, "an empty factory names nothing: {found:?}");
    }

    #[test]
    fn a_public_field_and_a_derived_default_are_constructors_too() {
        let at = place("jlreq-line", "feasible");
        let found = findings("pub struct Feasible {\n    pub cost: i32,\n}\n", &at);
        assert_eq!(found.len(), 1, "a public field builds one: {found:?}");
        let found = findings("#[derive(Default)]\npub struct Feasible(i32);\n", &at);
        assert_eq!(found.len(), 1, "and so does a derived Default: {found:?}");
        assert!(
            findings("pub struct Feasible(pub(crate) i32);\n", &at).is_empty(),
            "a crate-visible field is not reachable by a caller"
        );
    }

    #[test]
    fn a_sealed_type_is_checked_only_in_the_crate_that_declares_it() {
        let source = "impl Feasible {\n    pub fn new() -> Self {}\n}\n";
        assert!(
            findings(source, &place("jlreq-inline", "lower")).is_empty(),
            "another crate's identically named type is another type"
        );
    }

    #[test]
    fn prose_and_fixtures_naming_a_forbidden_construct_are_not_findings() {
        let at = place("jlreq-unit", "arith");
        let sources = [
            "//! No `impl Add for Em` is written anywhere.\nfn f() {}\n",
            "/* impl Add for Em */\nfn f() {}\n",
            "/* outer /* inner impl Add for Em */ still a comment */\nfn f() {}\n",
            "fn f() {\n    let fixture = \"impl Add for Em\";\n}\n",
            "fn f() {\n    let fixture = r#\"impl Add for Em\"#;\n}\n",
            "fn f() {\n    let fixture = \"a quote \\\" and impl Add for Em\";\n}\n",
        ];
        for source in sources {
            assert!(findings(source, &at).is_empty(), "still code: {source}");
        }
    }

    #[test]
    fn code_after_prose_on_the_same_line_is_still_read() {
        let at = place("jlreq-unit", "arith");
        let found = findings("impl Add for Em {} // permitted upstream\n", &at);
        assert_eq!(
            found.len(),
            1,
            "a trailing comment hides nothing: {found:?}"
        );
    }

    #[test]
    fn a_file_reports_its_findings_in_the_order_it_reads() {
        let at = place("jlreq-line", "badness");
        let found = findings(
            "fn of() {\n    BlockExtent::new(3);\n}\n\nimpl Add for Em {}\n",
            &at,
        );
        assert_eq!(found.len(), 2, "found {found:?}");
        assert!(
            found[0].contains(":2:"),
            "the earlier line comes first: {found:?}"
        );
        assert!(found[1].contains(":5:"), "{found:?}");
    }

    #[test]
    fn a_lifetime_is_not_a_character_literal() {
        let code = without_prose("fn f<'a>(x: &'a str) -> &'a str { 'q' }\n");
        assert!(code.contains("&'a str"), "a lifetime is code: {code}");
        assert!(!code.contains("'q'"), "a character literal is not: {code}");
    }

    #[test]
    fn blanking_prose_keeps_every_line_where_it_was() {
        let source = "//! one\n/* two\n   three */\nfn four() {}\n";
        let code = without_prose(source);
        assert_eq!(code.lines().count(), source.lines().count(), "{code:?}");
        assert_eq!(
            code.lines().nth(3).map(str::trim),
            Some("fn four() {}"),
            "the code is still on its own line: {code:?}"
        );
    }

    #[test]
    fn an_item_is_named_the_way_an_allowlist_names_one() {
        let read = read(concat!(
            "pub fn compose(\n    total: InlineExtent,\n) -> Line {\n",
            "    InlineExtent::new(1);\n}\n",
            "impl Badness {\n    fn of(self) {\n        Advance::get(a);\n    }\n}\n",
        ));
        let items: Vec<&str> = read.paths.iter().map(|used| used.item.as_str()).collect();
        assert!(
            items.contains(&"compose"),
            "a signature over several lines still names its item: {items:?}"
        );
        assert!(items.contains(&"Badness::of"), "found {items:?}");
        assert!(
            read.items.contains("Badness::of") && read.items.contains("compose"),
            "both are declared items: {declared:?}",
            declared = read.items
        );
    }

    #[test]
    fn a_call_outside_every_item_is_named_as_one() {
        let at = place("jlreq-line", "badness");
        let found = findings("const START: X = InlineOffset::new(0);\n", &at);
        assert_eq!(found.len(), 1, "found {found:?}");
        assert!(found[0].contains("crate scope"), "{found:?}");
    }

    #[test]
    fn an_allowlist_entry_naming_nothing_is_a_finding() {
        let mut violations = Vec::new();
        let reach = super::Reach::default();
        let site = super::Site {
            crate_name: "jlreq-line".to_owned(),
            item: "Badness::of".to_owned(),
            allowlist: "docs/scalar-sites.toml".to_owned(),
        };
        super::check_site(&site, &["jlreq-line"], &reach, &mut violations);
        assert_eq!(violations.len(), 1, "found {violations:?}");
        assert!(
            violations[0].contains("no gate can check"),
            "{violations:?}"
        );

        let mut reach = super::Reach::default();
        reach
            .items
            .entry("jlreq-line".to_owned())
            .or_default()
            .insert("Badness::of".to_owned());
        let mut violations = Vec::new();
        super::check_site(&site, &["jlreq-line"], &reach, &mut violations);
        assert_eq!(violations.len(), 1, "found {violations:?}");
        assert!(violations[0].contains("makes no call"), "{violations:?}");

        reach
            .users
            .insert(("jlreq-line".to_owned(), "Badness::of".to_owned()));
        let mut violations = Vec::new();
        super::check_site(&site, &["jlreq-line"], &reach, &mut violations);
        assert!(
            violations.is_empty(),
            "an entry that permits a real call: {violations:?}"
        );
    }

    #[test]
    fn an_allowlist_entry_naming_no_crate_is_a_finding() {
        let mut violations = Vec::new();
        let site = super::Site {
            crate_name: "jlreq-typography".to_owned(),
            item: "compose".to_owned(),
            allowlist: "docs/scalar-sites.toml".to_owned(),
        };
        super::check_site(
            &site,
            &["jlreq-line"],
            &super::Reach::default(),
            &mut violations,
        );
        assert_eq!(violations.len(), 1, "found {violations:?}");
        assert!(
            violations[0].contains("not a workspace member"),
            "{violations:?}"
        );
    }

    #[test]
    fn a_home_module_the_workspace_does_not_have_is_a_finding() {
        let mut violations = Vec::new();
        let channel = super::Channel {
            types: BTreeSet::new(),
            methods: BTreeSet::new(),
            home: "jlreq-unit::geometry".to_owned(),
            homes: home_modules("jlreq-unit::geometry"),
            allowlist: "docs/scalar-sites.toml".to_owned(),
        };
        super::check_homes(&channel, &super::Reach::default(), &mut violations);
        assert_eq!(violations.len(), 1, "found {violations:?}");
        assert!(
            violations[0].contains("not a module of this workspace"),
            "{violations:?}"
        );
    }

    #[test]
    fn an_array_of_tables_reads_both_array_layouts() {
        let document = concat!(
            "[[other]]\ntypes = [\"decoy\"]\n\n",
            "[[no_impl]]\ntypes = [\n  \"Em\",\n  \"Advance\",\n]\n",
            "traits = [\"Add\", \"Sub\"]\nwhy = \"because\"\n",
        );
        let entries = array_of_tables(document, "no_impl");
        assert_eq!(entries.len(), 1, "one entry, not the decoy: {entries:?}");
        assert_eq!(
            entries[0].set("t", "types").expect("types are stated"),
            ["Advance".to_owned(), "Em".to_owned()]
                .into_iter()
                .collect()
        );
        assert_eq!(
            entries[0].text("t", "why").expect("a reason is stated"),
            "because"
        );
        assert!(
            entries[0].text("t", "home").is_err(),
            "a key nobody wrote is an error"
        );
    }

    #[test]
    fn a_control_file_that_lost_a_table_this_gate_enforces_cannot_be_read() {
        assert!(
            super::read_forbidden("[[other]]\ntypes = [\"Em\"]\n").is_err(),
            "a table that is gone says nothing about the invariant, so the gate fails"
        );
        assert!(
            super::read_channels("").is_err(),
            "and so does an empty file"
        );
        assert!(
            super::read_forbidden("[[no_impl]]\ntypes = [\"Em\"]\n").is_err(),
            "an entry stating types and no traits closes nothing"
        );
        assert!(
            super::read_sealed("[[no_public_constructor]]\ntype = \"Feasible\"\nfactory = \"\"\n")
                .is_err(),
            "a type naming no crate cannot be looked for in one"
        );
    }

    #[test]
    fn a_commented_out_entry_is_not_an_entry() {
        let document = "# [[site]]\n# crate = \"jlreq-line\"\n[[site]]\ncrate = \"jlreq-conform\"\nitem = \"c\"\nwhy = \"w\"\n";
        let entries = array_of_tables(document, "site");
        assert_eq!(entries.len(), 1, "found {entries:?}");
        assert_eq!(
            entries[0].text("t", "crate").expect("a crate is stated"),
            "jlreq-conform"
        );
    }

    #[test]
    fn the_home_is_read_as_the_modules_it_names() {
        assert_eq!(
            home_modules("jlreq-unit::axis, jlreq-unit::length"),
            [
                ("jlreq-unit".to_owned(), "axis".to_owned()),
                ("jlreq-unit".to_owned(), "length".to_owned()),
            ]
        );
    }

    #[test]
    fn a_crate_is_the_same_crate_however_it_is_spelled() {
        assert!(
            same_crate("jlreq-line", "jlreq_line"),
            "one crate, two spellings"
        );
        assert!(
            !same_crate("jlreq-line", "jlreq_inline"),
            "and two crates are two"
        );
    }

    #[test]
    fn a_module_is_named_by_its_path_below_the_crate_root() {
        let root = std::path::Path::new("crates/jlreq-spec/src");
        assert_eq!(module_path(&root.join("lib.rs"), root), "");
        assert_eq!(module_path(&root.join("rule.rs"), root), "rule");
        assert_eq!(
            module_path(&root.join("generated").join("mod.rs"), root),
            "generated"
        );
        assert_eq!(
            module_path(&root.join("generated").join("rules.rs"), root),
            "generated::rules"
        );
    }

    #[test]
    fn a_path_is_named_by_its_last_segment_outside_its_arguments() {
        let tokens = tokenize("core::ops::Add<i32>");
        assert_eq!(last_path_name(&tokens), Some("Add".to_owned()));
        let tokens = tokenize("Vec<Em>");
        assert_eq!(
            last_path_name(&tokens),
            Some("Vec".to_owned()),
            "an argument is not the type"
        );
        let tokens = tokenize("Distribution<'_>");
        assert_eq!(last_path_name(&tokens), Some("Distribution".to_owned()));
    }

    #[test]
    fn a_negated_test_configuration_is_not_a_test_configuration() {
        assert!(is_test_configuration(&tokenize("cfg(test)")));
        assert!(!is_test_configuration(&tokenize("cfg(not(test))")));
        assert!(!is_test_configuration(&tokenize("cfg(feature = \"std\")")));
    }

    #[test]
    fn the_members_are_read_from_the_workspace_table_alone() {
        let manifest = "[workspace.metadata]\nmembers = [\"decoy\"]\n\n[workspace]\nmembers = [\n  \"crates/a\",\n  \"xtask\",\n]\n";
        assert_eq!(workspace_members(manifest), ["crates/a", "xtask"]);
        assert_eq!(package_name("[package]\nname = \"x\"\n"), Some("x"));
    }

    #[test]
    fn every_workspace_member_is_examined_and_not_only_the_core() {
        let root = shared::workspace_root().expect("the workspace root is locatable");
        let members = workspace_crates(&root).expect("every member has a manifest");
        let names: Vec<&str> = members.iter().map(|member| member.name.as_str()).collect();
        for expected in ["jlreq-unit", "jlreq-conform", "xtask"] {
            assert!(
                names.contains(&expected),
                "{expected} is examined: {names:?}"
            );
        }
    }
}
