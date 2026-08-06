// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `spec-links` gate.
//!
//! Every answer this library gives is attributed to a sentence of JLReq, and the
//! attribution is worth exactly as much as the checking of it. `docs/adr/0013` makes that
//! checkable by giving four artifacts one identifier space — the generated tables, the doc
//! comment on every public item, the conformance case files, and this gate — and the chain
//! it ties has four links:
//!
//! 1. every public item of the layout core carries a `JLReq:` line,
//! 2. every address such a line names is the canonical spelling of the rule-address
//!    grammar,
//! 3. every address resolves: its section is one the specification publishes, and an
//!    address naming a note or a matrix cell is a rule the generated inventory names,
//! 4. every rule an item cites has a conformance case.
//!
//! Links 1 and 2 hold today. Links 3 and 4 read `spec/derived/anchors.tsv`,
//! `spec/derived/rules.tsv` and `crates/jlreq-conform/cases/`, none of which exists yet:
//! the first two arrive with stage 1 of the generation pipeline
//! (`docs/design/generation.md`) and the third with the conformance suite
//! (`docs/design/conformance.md`). Each reports itself as *not run*, naming the file it
//! waited for, rather than reporting that it passed. Nothing is switched on by hand when
//! those files land: the checks are written, and each starts constraining the moment the
//! file it reads exists.
//!
//! ## What must cite
//!
//! Every item the layout core declares `pub`, wherever it is declared: in a module, in an
//! inherent `impl`, or in a macro that writes one. A private module is no exemption,
//! because `pub use` carries its items into the public surface anyway. The core is the
//! crate list `shared` derives, so `jlreq-conform` is outside it: its public types
//! describe the case format rather than implementing a rule, and the rules its cases name
//! are `conform --check`'s subject rather than this gate's.
//!
//! Four kinds of declaration are outside the requirement, each for a reason rather than
//! for convenience. A `pub use` re-export names an item that carries its own citation. A
//! `pub mod` is a namespace and states nothing; every item it exposes is checked one by
//! one. An item inside `impl Trait for Type` belongs to the trait, which carries the
//! citation at its own declaration. And a `#[cfg(test)]` module is not the public surface.
//! Conversely a trait *definition*'s items are required, because a trait's methods are
//! public without saying `pub`, and in a generated source every variant of a public enum
//! is required too, which is the contract `docs/design/generation.md` puts on the emitter.
//!
//! ## What a citation may say
//!
//! An item that implements no statement of the specification says exactly that, in the
//! same line: `JLReq: n/a (arithmetic)`. The escape hatch is the point rather than a
//! weakness — a length type implements no sentence of JLReq, and the alternative is a
//! reader unable to tell an item that implements nothing from one whose citation was
//! forgotten. What is refused is the shape between the two: a line naming only an ADR
//! names no sentence of the specification and does not admit to naming none, and that is
//! the citation habit decaying into decoration.
//!
//! A citation is one line. Its references are separated by a comma and a space, and the
//! canonical rendering of a matrix cell writes `B.1@cl-05,cl-05` without one, so the
//! spelling itself says which comma is a separator and a citation never has to be
//! reassembled across two lines to be read.
//!
//! ## Hand-rolled, on purpose
//!
//! The source scan and the two data readers are written out here for the reason stated on
//! `purity`'s manifest scan: `xtask` declares no dependencies, because it is the tool that
//! enforces the layout core declaring none. The address grammar was written out here too
//! and is not any more: it is `crate::shared`'s, so that this gate, `conform` and `attest`
//! read one language rather than three. It is still stated twice in the repository —
//! there, and in `jlreq-spec`'s `Address::parse`, which no gate may depend on — and
//! `docs/design/address-corpus.tsv` is what holds those two equal.
//!
//! See `docs/adr/0013-rules-are-addressed-by-specification-address.md`,
//! `docs/design/api-spine.md`, `docs/design/generation.md` and
//! `docs/design/conformance.md`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::Path;

use crate::shared::{self, Address, CoreCrate, Detail, Gate, Section, address, number};

/// The `spec-links` gate, as the dispatcher sees it.
pub(crate) const GATE: Gate = Gate {
    name: "spec-links",
    purpose: concat!(
        "every public item of the layout core cites the specification, and every address ",
        "it names is well formed, resolved and tested as far as the generated inventory ",
        "and the conformance cases exist"
    ),
    reference: concat!(
        "docs/adr/0013-rules-are-addressed-by-specification-address.md ",
        "and docs/design/api-spine.md"
    ),
    run,
};

/// The generated section inventory: the specification's own *rendered* section numbers,
/// which is what an address is validated against rather than the published anchor slugs,
/// because those are off by one (`docs/design/generation.md`).
const SECTIONS: &str = "spec/derived/anchors.tsv";

/// The generated rule inventory: one row per rule (`docs/design/generation.md`).
const INVENTORY: &str = "spec/derived/rules.tsv";

/// The conformance cases (`docs/design/conformance.md`).
const CASES: &str = "crates/jlreq-conform/cases";

/// The marker that opens a citation.
const MARKER: &str = "JLReq:";

/// The sign every specification reference is written with.
const SIGN: char = '§';

/// The dash a range of references is written with: an en dash, as the workspace writes it.
const RANGE: char = '–';

/// The item kinds that must cite.
const CITED: &[&str] = &[
    "fn", "struct", "enum", "trait", "type", "const", "static", "union",
];

/// Every item kind the scanner recognizes. `mod` and `use` are here so that the scanner
/// knows a module body from a block and a re-export from a declaration, not because either
/// must cite.
const KINDS: &[&str] = &[
    "fn", "struct", "enum", "trait", "type", "const", "static", "union", "mod", "use",
];

/// Check the citation chain and gather the findings. Takes no arguments.
///
/// A slot that silently accepted an argument would let a caller believe a mode had run, so
/// one is refused rather than ignored.
fn run(arguments: &[String]) -> io::Result<Vec<String>> {
    if let Some(first) = arguments.first() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("spec-links takes no arguments; got `{first}`"),
        ));
    }
    let root = shared::workspace_root()?;
    let mut violations = Vec::new();
    let surface = read_surface(&shared::core_crates()?, &mut violations)?;
    let cited = read_addresses(&surface, &mut violations);

    let mut examined = vec![format!(
        "examined: {items} public items in {crates} core crates, {lines} citation lines, \
         {addresses} distinct addresses",
        items = surface.items,
        crates = surface.crates,
        lines = surface.citations.len(),
        addresses = cited.len(),
    )];
    let sections = read_table(&root.join(SECTIONS), SECTIONS, &mut violations)?;
    let inventory = read_table(&root.join(INVENTORY), INVENTORY, &mut violations)?;
    let cases = read_cases(&root.join(CASES))?;

    check_sections(sections.as_ref(), &cited, &mut examined, &mut violations);
    let rules = check_rules(inventory.as_ref(), &cited, &mut examined, &mut violations);
    check_cases(
        cases.as_ref(),
        rules.as_ref(),
        &cited,
        &mut examined,
        &mut violations,
    );

    for line in &examined {
        println!("{name}: {line}", name = GATE.name);
    }
    Ok(violations)
}

// ---------------------------------------------------------------------------------------
// The public surface
// ---------------------------------------------------------------------------------------

/// What the layout core's sources hold: its public items, and every citation written in
/// them.
#[derive(Debug, Default)]
struct Surface {
    /// How many crates were read.
    crates: usize,
    /// How many public items were examined.
    items: usize,
    /// Every citation, in source order.
    citations: Vec<Citation>,
}

/// One `JLReq:` line, and where it was written.
#[derive(Debug)]
struct Citation {
    /// Where a report names it: `jlreq-unit: item.rs:234`.
    place: String,
    /// Everything the line says after the marker.
    text: String,
}

/// Read every core crate, requiring a citation of every public item.
fn read_surface(core: &[CoreCrate], violations: &mut Vec<String>) -> io::Result<Surface> {
    let mut surface = Surface {
        crates: core.len(),
        ..Surface::default()
    };
    for each in core {
        let directory = each.directory.join("src");
        for source in shared::rust_sources(&directory)? {
            let name = shared::relative_name(&source, &directory);
            let scan = scan(&fs::read_to_string(&source)?, is_generated(&source));
            for item in &scan.items {
                surface.items = surface.items.saturating_add(1);
                if !item.cites {
                    violations.push(format!(
                        "{crate_name}: {name}:{line}: `{item_name}` is public and carries no \
                         `JLReq:` line (ADR 0013)",
                        crate_name = each.name,
                        line = item.line,
                        item_name = item.name,
                    ));
                }
            }
            for (line, text) in scan.citations {
                surface.citations.push(Citation {
                    place: format!("{crate_name}: {name}:{line}", crate_name = each.name),
                    text,
                });
            }
        }
    }
    Ok(surface)
}

/// Whether a source is emitted rather than written, which the emitter's stricter contract
/// keys on (`docs/design/generation.md`).
fn is_generated(source: &Path) -> bool {
    source
        .components()
        .any(|component| component.as_os_str() == "generated")
}

// ---------------------------------------------------------------------------------------
// Scanning one source
// ---------------------------------------------------------------------------------------

/// One public item, as the scanner met it.
#[derive(Debug)]
struct Item {
    /// How a report names it: `Em::ZERO`, `distribute`, `Frame::ThirdEm`.
    name: String,
    /// The line its declaration begins on, counted from one.
    line: usize,
    /// Whether its own doc comment carries a citation.
    cites: bool,
}

/// What one source file holds.
#[derive(Debug, Default)]
struct Scan {
    /// Its public items, in source order.
    items: Vec<Item>,
    /// Every citation written in it, by line.
    citations: Vec<(usize, String)>,
}

/// A block the scanner is inside.
#[derive(Debug)]
enum Scope {
    /// An inherent `impl`, whose items are named after its type.
    Inherent(String),
    /// A trait definition, whose items are public without saying so.
    Definition(String),
    /// An `impl Trait for Type`, whose items belong to the trait.
    Implementation,
    /// A public enum, whose variants a generated source must cite.
    Enumeration(String),
    /// A `#[cfg(test)]` module: not the public surface.
    Tests,
    /// Anything else with a body.
    Other,
}

/// The scanner's position in one file.
#[derive(Debug, Default)]
struct Cursor {
    /// Whether the documentation read since the last code line carries a citation.
    cites: bool,
    /// Whether the attributes read since the last code line include `cfg(test)`.
    testing: bool,
    /// Unclosed brackets of an attribute spanning several lines.
    attribute: usize,
    /// The current brace depth.
    depth: usize,
    /// The blocks the scanner is inside, innermost last, each with the depth it opened at.
    scopes: Vec<(Scope, usize)>,
}

/// Read one source file.
///
/// `generated` selects the emitter's contract, under which every variant of a public enum
/// cites as well.
fn scan(source: &str, generated: bool) -> Scan {
    let mut scan = Scan::default();
    let mut cursor = Cursor::default();
    for (offset, raw) in source.lines().enumerate() {
        cursor.read(offset.saturating_add(1), raw.trim(), generated, &mut scan);
    }
    scan
}

impl Cursor {
    /// Read one line.
    fn read(&mut self, number: usize, line: &str, generated: bool, scan: &mut Scan) {
        if let Some(text) = comment(line) {
            if let Some(claim) = claim_of(text) {
                scan.citations.push((number, claim.to_owned()));
                self.cites = self.cites || line.starts_with("///");
            }
            return;
        }
        if self.attribute > 0 || line.starts_with('#') {
            self.testing = self.testing || line.contains("cfg(test)");
            self.attribute = brackets(self.attribute, line);
            return;
        }
        if line.is_empty() {
            self.forget();
            return;
        }
        self.code(number, &code_of(line), generated, scan);
    }

    /// Read one line of code: what it declares, what it opens, and what it closes.
    fn code(&mut self, number: usize, code: &str, generated: bool, scan: &mut Scan) {
        let opens = code.matches('{').count();
        let closes = code.matches('}').count();
        match declaration(code) {
            Some(declared) => self.declare(number, &declared, scan),
            None if generated => self.variant(number, code, scan),
            None => {},
        }
        if opens > closes {
            let scope = self.opening(code);
            self.scopes.push((scope, self.depth));
        }
        self.depth = self.depth.saturating_add(opens).saturating_sub(closes);
        while self.scopes.last().is_some_and(|(_, at)| self.depth <= *at) {
            self.scopes.pop();
        }
        self.forget();
    }

    /// Record a declaration when it is one of the public surface.
    fn declare(&mut self, number: usize, declared: &Declared<'_>, scan: &mut Scan) {
        if !CITED.contains(&declared.kind) || self.exempt() {
            return;
        }
        // A trait's items are public without saying so.
        let inherited = matches!(self.innermost(), Some(Scope::Definition(_)));
        if !declared.public && !inherited {
            return;
        }
        let name = match self.innermost() {
            Some(Scope::Inherent(owner) | Scope::Definition(owner)) => {
                format!("{owner}::{name}", name = declared.name)
            },
            _ => declared.name.to_owned(),
        };
        scan.items.push(Item {
            name,
            line: number,
            cites: self.cites,
        });
    }

    /// Record an enum variant of a generated source.
    fn variant(&mut self, number: usize, code: &str, scan: &mut Scan) {
        let Some(Scope::Enumeration(owner)) = self.innermost() else {
            return;
        };
        if !code.starts_with(|character: char| character.is_ascii_uppercase()) {
            return;
        }
        let name = identifier(code);
        if name.is_empty() {
            return;
        }
        scan.items.push(Item {
            name: format!("{owner}::{name}"),
            line: number,
            cites: self.cites,
        });
    }

    /// The block a code line opens.
    fn opening(&self, code: &str) -> Scope {
        if let Some(rest) = code.strip_prefix("impl") {
            return if rest.contains(" for ") {
                Scope::Implementation
            } else {
                Scope::Inherent(after_generics(rest).to_owned())
            };
        }
        match declaration(code) {
            Some(declared) if declared.kind == "trait" => {
                Scope::Definition(declared.name.to_owned())
            },
            Some(declared) if declared.kind == "enum" && declared.public => {
                Scope::Enumeration(declared.name.to_owned())
            },
            Some(declared) if declared.kind == "mod" && self.testing => Scope::Tests,
            _ => Scope::Other,
        }
    }

    /// The innermost block, if the scanner is inside one.
    fn innermost(&self) -> Option<&Scope> {
        self.scopes.last().map(|(scope, _)| scope)
    }

    /// Whether the scanner is anywhere inside a block the requirement does not reach.
    fn exempt(&self) -> bool {
        self.scopes
            .iter()
            .any(|(scope, _)| matches!(scope, Scope::Tests | Scope::Implementation))
    }

    /// Forget the documentation and attributes a code line has consumed.
    fn forget(&mut self) {
        self.cites = false;
        self.testing = false;
    }
}

/// An item declaration, as a line spells it.
#[derive(Debug)]
struct Declared<'a> {
    /// `fn`, `struct`, `enum`, and the rest of [`KINDS`].
    kind: &'a str,
    /// The name it declares.
    name: &'a str,
    /// Whether it is public: `pub`, and not `pub(crate)` or anything narrower.
    public: bool,
}

/// The item a code line declares, when it declares one.
fn declaration(code: &str) -> Option<Declared<'_>> {
    let (public, rest) = match code.strip_prefix("pub ") {
        Some(rest) => (true, rest.trim_start()),
        None => match code.strip_prefix("pub(") {
            Some(rest) => (false, rest.split_once(')')?.1.trim_start()),
            None => (false, code),
        },
    };
    let mut words = rest.split_whitespace().peekable();
    let mut kind = words.next()?;
    loop {
        match kind {
            "unsafe" | "async" | "default" => kind = words.next()?,
            "extern" => {
                if words.peek().is_some_and(|word| word.starts_with('"')) {
                    words.next();
                }
                kind = words.next()?;
            },
            // `const fn` declares a function; `const NAME` declares a constant.
            "const" if words.peek() == Some(&"fn") => kind = words.next()?,
            _ => break,
        }
    }
    if !KINDS.contains(&kind) {
        return None;
    }
    let name = identifier(words.next()?);
    (!name.is_empty()).then_some(Declared { kind, name, public })
}

/// The leading identifier of a word: `Em` of `Em(i32);`, `units` of `units(self)`.
fn identifier(word: &str) -> &str {
    let end = word
        .find(|character: char| !character.is_alphanumeric() && character != '_')
        .unwrap_or(word.len());
    word.get(..end).unwrap_or(word)
}

/// The type an `impl` names, past any generic parameters.
fn after_generics(rest: &str) -> &str {
    let rest = rest.trim_start();
    let Some(inner) = rest.strip_prefix('<') else {
        return identifier_or_word(rest);
    };
    let mut depth = 1usize;
    for (index, character) in inner.char_indices() {
        match character {
            '<' => depth = depth.saturating_add(1),
            '>' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let tail = inner.get(index.saturating_add(1)..).unwrap_or("");
                    return identifier_or_word(tail.trim_start());
                }
            },
            _ => {},
        }
    }
    identifier_or_word(rest)
}

/// The leading identifier of a word, or the whole first word when it has none — a macro
/// body writes `impl $type`, and naming it `$type` in a report is better than naming it
/// nothing.
fn identifier_or_word(text: &str) -> &str {
    let word = text.split_whitespace().next().unwrap_or("");
    let name = identifier(word);
    if name.is_empty() { word } else { name }
}

/// The text of a whole-line comment, when the line is one.
fn comment(line: &str) -> Option<&str> {
    ["///", "//!", "//"]
        .into_iter()
        .find_map(|prefix| line.strip_prefix(prefix))
}

/// The claim a comment line carries, when it carries one.
///
/// A marker inside a code span is prose about the convention rather than a citation. The
/// API spine's own sentence — every public item carries a `JLReq:` line — is written that
/// way, and a gate that read it as a citation could not describe itself.
fn claim_of(text: &str) -> Option<&str> {
    let index = text.find(MARKER)?;
    let before = text.get(..index)?;
    if before.matches('`').count() % 2 == 1 {
        return None;
    }
    Some(text.get(index.saturating_add(MARKER.len())..)?.trim())
}

/// The code on a line: its string literals emptied and any trailing comment dropped, so
/// that a brace inside either does not move the scanner's idea of where it is.
fn code_of(line: &str) -> String {
    let mut code = String::with_capacity(line.len());
    let mut characters = line.chars().peekable();
    let mut inside = false;
    while let Some(character) = characters.next() {
        match character {
            '\\' if inside => {
                characters.next();
            },
            '"' => inside = !inside,
            '/' if !inside && characters.peek() == Some(&'/') => break,
            _ if !inside => code.push(character),
            _ => {},
        }
    }
    code
}

/// The bracket balance an attribute line leaves behind it.
fn brackets(open: usize, line: &str) -> usize {
    open.saturating_add(line.matches('[').count())
        .saturating_sub(line.matches(']').count())
}

// ---------------------------------------------------------------------------------------
// The citation grammar
// ---------------------------------------------------------------------------------------

/// What one reference of a citation names.
#[derive(Debug)]
enum Reference {
    /// One or more specification addresses: a section, a note, a cell, or a range.
    Spec(Vec<Address>),
    /// One of this repository's architecture decision records.
    Record,
    /// A legend token or a recorded decision, continuing the reference before it.
    Continuation,
}

/// Read one citation line into the addresses it names.
///
/// The grammar, which `docs/adr/0013` fixes and the API spine writes throughout:
///
/// ```text
/// claim     := "n/a" "(" reason ")" | reference ("," reference)*
/// reference := "§" span qualifier? | "#" number qualifier? | "`" token "`" | "ADR-" dddd
/// span      := address ("–" ("§" address | "#" number))?
/// ```
///
/// A `#` reference continues the section of the reference before it, which is how the
/// workspace writes `§B.2#1, #2, #4`. A range must run upward between two addresses
/// differing only in their last number, so `§A.20–§A.23` names four rules and
/// `§A.23–§A.20` names none.
fn claim(text: &str) -> Result<Vec<Address>, String> {
    if let Some(rest) = text.strip_prefix("n/a") {
        return excuse(rest.trim()).map(|()| Vec::new());
    }
    let mut addresses: Vec<Address> = Vec::new();
    let mut previous: Option<Section> = None;
    let mut named = false;
    for (position, part) in references(text).into_iter().enumerate() {
        match reference(part, previous.as_ref())? {
            Reference::Spec(found) => {
                named = true;
                previous = found.last().map(|address| address.section.clone());
                addresses.extend(found);
            },
            Reference::Continuation if position == 0 => {
                return Err(format!(
                    "`{part}` continues a reference, but nothing comes before it"
                ));
            },
            Reference::Record | Reference::Continuation => {},
        }
    }
    if named {
        Ok(addresses)
    } else {
        Err(concat!(
            "names no statement of the specification; an item that implements none says ",
            "so as `n/a (<reason>)` (ADR 0013)"
        )
        .to_owned())
    }
}

/// Check the `n/a` form, which must state why the item implements no statement.
fn excuse(rest: &str) -> Result<(), String> {
    let Some(reason) = rest.strip_prefix('(').and_then(|it| it.strip_suffix(')')) else {
        return Err("`n/a` must state its reason as `n/a (<reason>)`".to_owned());
    };
    if reason.trim().is_empty() {
        return Err("`n/a ()` states no reason".to_owned());
    }
    Ok(())
}

/// Split a citation into its references: the commas outside backticks and parentheses.
fn references(text: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut quoted = false;
    let mut depth = 0usize;
    for (index, character) in text.char_indices() {
        match character {
            '`' => quoted = !quoted,
            '(' if !quoted => depth = depth.saturating_add(1),
            ')' if !quoted => depth = depth.saturating_sub(1),
            ',' if !quoted && depth == 0 && separates(text, index) => {
                if let Some(part) = text.get(start..index) {
                    parts.push(part.trim());
                }
                start = index.saturating_add(1);
            },
            _ => {},
        }
    }
    if let Some(part) = text.get(start..) {
        parts.push(part.trim());
    }
    parts
}

/// Whether the comma at `index` separates two references rather than the two coordinates
/// of a matrix cell.
///
/// The spelling itself says which comma it is: a citation separates its references with a
/// comma and a space, and the canonical rendering of a cell writes `B.1@cl-05,cl-05` with
/// no space at all. A citation is therefore also one line, and a trailing comma is the
/// empty reference reported below rather than a continuation onto the next.
fn separates(text: &str, index: usize) -> bool {
    match text.get(index.saturating_add(1)..) {
        Some(rest) => rest.is_empty() || rest.starts_with(' '),
        None => true,
    }
}

/// Read one reference, given the section the reference before it named.
fn reference(part: &str, previous: Option<&Section>) -> Result<Reference, String> {
    if part.is_empty() {
        return Err(
            "an empty reference: a citation is one line and ends with its last reference"
                .to_owned(),
        );
    }
    if let Some(rest) = part.strip_prefix(SIGN) {
        return span(rest).map(Reference::Spec);
    }
    if let Some(rest) = part.strip_prefix('#') {
        let Some(section) = previous else {
            return Err(format!(
                "`{part}` continues a section, but no reference before it names one"
            ));
        };
        return Ok(Reference::Spec(vec![Address {
            section: section.clone(),
            detail: Detail::Note(note(rest, part)?),
        }]));
    }
    if part.starts_with('`') {
        return if part.matches('`').count() % 2 == 0 {
            Ok(Reference::Continuation)
        } else {
            Err(format!("`{part}` opens a code span and does not close it"))
        };
    }
    if let Some(rest) = part.strip_prefix("ADR-") {
        let (digits, _) = token(rest);
        return if digits.len() == 4 && digits.bytes().all(|byte| byte.is_ascii_digit()) {
            Ok(Reference::Record)
        } else {
            Err(format!(
                "`{part}` is not a decision record: those are `ADR-nnnn`"
            ))
        };
    }
    Err(format!(
        "`{part}` is not a reference: write `§<address>`, `#<note>`, `ADR-nnnn`, or a \
         `token` continuing the reference before it"
    ))
}

/// Read a `§` reference: one address, or a range of them, and the prose that may follow.
fn span(text: &str) -> Result<Vec<Address>, String> {
    let (reference, _qualifier) = token(text);
    let (head, tail) = match reference.split_once(RANGE) {
        Some((head, tail)) => (head, Some(tail)),
        None => (reference, None),
    };
    let start = parsed(head)?;
    let Some(tail) = tail else {
        return Ok(vec![start]);
    };
    let end = match tail.strip_prefix('#') {
        Some(digits) => Address {
            section: start.section.clone(),
            detail: Detail::Note(note(digits, tail)?),
        },
        None => parsed(tail.strip_prefix(SIGN).unwrap_or(tail))?,
    };
    expand(&start, &end)
}

/// Every address a range covers, endpoints included.
fn expand(start: &Address, end: &Address) -> Result<Vec<Address>, String> {
    let refused = || {
        format!(
            "`{start}–{end}` is not a range: a range runs upward between two addresses \
             differing only in their last number"
        )
    };
    match (&start.detail, &end.detail) {
        (Detail::Note(from), Detail::Note(upto)) if start.section == end.section => {
            steps(*from, *upto).ok_or_else(refused).map(|ordinals| {
                ordinals
                    .map(|note| Address {
                        section: start.section.clone(),
                        detail: Detail::Note(note),
                    })
                    .collect()
            })
        },
        (Detail::Whole, Detail::Whole) if siblings(&start.section, &end.section) => {
            let (Some(from), Some(upto)) = (start.section.parts.last(), end.section.parts.last())
            else {
                return Err(refused());
            };
            steps(*from, *upto).ok_or_else(refused).map(|components| {
                components
                    .map(|component| Address {
                        section: replaced(&start.section, component),
                        detail: Detail::Whole,
                    })
                    .collect()
            })
        },
        _ => Err(refused()),
    }
}

/// The ordinals a range covers, or `None` when it does not run upward.
fn steps(from: u8, upto: u8) -> Option<std::ops::RangeInclusive<u8>> {
    (from <= upto).then_some(from..=upto)
}

/// Whether two sections differ in nothing but their last component.
fn siblings(left: &Section, right: &Section) -> bool {
    left.appendix == right.appendix
        && left.parts.len() == right.parts.len()
        && !left.parts.is_empty()
        && left.parts.split_last().map(|(_, head)| head)
            == right.parts.split_last().map(|(_, head)| head)
}

/// A section with its last component replaced.
fn replaced(section: &Section, component: u8) -> Section {
    let mut parts = section.parts.clone();
    if let Some(last) = parts.last_mut() {
        *last = component;
    }
    Section {
        appendix: section.appendix,
        parts,
    }
}

/// The reference at the head of a `§` span, and the prose that follows it.
fn token(text: &str) -> (&str, &str) {
    match text.split_once(' ') {
        Some((head, tail)) => (head, tail),
        None => (text, ""),
    }
}

/// Parse an address, naming the text when it is not one.
fn parsed(text: &str) -> Result<Address, String> {
    address(text).ok_or_else(|| format!("`{text}` is not a specification address (ADR 0013)"))
}

/// Parse a note ordinal, naming the reference when it is not one.
fn note(digits: &str, part: &str) -> Result<u8, String> {
    let (digits, _) = token(digits);
    number(digits).ok_or_else(|| format!("`{part}` is not a note ordinal"))
}

// ---------------------------------------------------------------------------------------
// The four checks
// ---------------------------------------------------------------------------------------

/// One address the layout core cites, and what the later checks need of it.
#[derive(Debug)]
struct Mention {
    /// Its section, canonically rendered: the `B.2` of `B.2#3`.
    section: String,
    /// The first place it was cited.
    place: String,
    /// Whether it names something inside its section — a note, or a matrix cell.
    detailed: bool,
}

/// Every address the layout core cites, canonically rendered.
type Cited = BTreeMap<String, Mention>;

/// Read every citation, refusing the ones the grammar does not accept.
fn read_addresses(surface: &Surface, violations: &mut Vec<String>) -> Cited {
    let mut cited = Cited::new();
    for citation in &surface.citations {
        match claim(&citation.text) {
            Err(reason) => violations.push(format!(
                "{place}: `{MARKER} {text}`: {reason}",
                place = citation.place,
                text = citation.text,
            )),
            Ok(addresses) => {
                for address in addresses {
                    cited.entry(address.to_string()).or_insert_with(|| Mention {
                        section: address.section.to_string(),
                        place: citation.place.clone(),
                        detailed: address.detail != Detail::Whole,
                    });
                }
            },
        }
    }
    cited
}

/// Hold every cited address against the sections the specification publishes.
fn check_sections(
    sections: Option<&BTreeSet<String>>,
    cited: &Cited,
    examined: &mut Vec<String>,
    violations: &mut Vec<String>,
) {
    let Some(sections) = sections else {
        examined.push(format!(
            "section resolution: did not run, `{SECTIONS}` does not exist yet \
             (docs/design/generation.md)"
        ));
        return;
    };
    let mut held = 0usize;
    for (address, mention) in cited {
        if sections.contains(&mention.section) {
            held = held.saturating_add(1);
        } else {
            violations.push(format!(
                "{place}: `{SIGN}{address}` names section {section}, which `{SECTIONS}` does \
                 not publish (ADR 0013)",
                place = mention.place,
                section = mention.section,
            ));
        }
    }
    examined.push(format!(
        "section resolution: {held} of {total} cited addresses name a section `{SECTIONS}` \
         publishes",
        total = cited.len(),
    ));
}

/// Hold every cited address against the rule inventory, and report which of them are rules.
///
/// `None` when the inventory does not exist, which is what keeps the closure check below
/// from passing over an empty set.
fn check_rules(
    inventory: Option<&BTreeSet<String>>,
    cited: &Cited,
    examined: &mut Vec<String>,
    violations: &mut Vec<String>,
) -> Option<BTreeSet<String>> {
    let Some(inventory) = inventory else {
        examined.push(format!(
            "rule resolution: did not run, `{INVENTORY}` does not exist yet \
             (docs/design/generation.md)"
        ));
        return None;
    };
    let mut rules = BTreeSet::new();
    for (address, mention) in cited {
        if inventory.contains(address) {
            rules.insert(address.clone());
        } else if mention.detailed {
            violations.push(format!(
                "{place}: `{SIGN}{address}` is not a rule `{INVENTORY}` names (ADR 0013)",
                place = mention.place,
            ));
        }
    }
    examined.push(format!(
        "rule resolution: {count} of {total} cited addresses are rules `{INVENTORY}` names",
        count = rules.len(),
        total = cited.len(),
    ));
    Some(rules)
}

/// Hold every cited rule against the conformance cases.
fn check_cases(
    cases: Option<&BTreeSet<String>>,
    rules: Option<&BTreeSet<String>>,
    cited: &Cited,
    examined: &mut Vec<String>,
    violations: &mut Vec<String>,
) {
    let (Some(cases), Some(rules)) = (cases, rules) else {
        let missing = if cases.is_none() { CASES } else { INVENTORY };
        examined.push(format!(
            "case closure: did not run, `{missing}` holds nothing yet \
             (docs/design/conformance.md)"
        ));
        return;
    };
    let mut tested = 0usize;
    for rule in rules {
        if cases.contains(rule) {
            tested = tested.saturating_add(1);
        } else {
            let place = cited
                .get(rule)
                .map_or("the layout core", |mention| mention.place.as_str());
            violations.push(format!(
                "{place}: `{SIGN}{rule}` is cited and no conformance case names it (ADR 0013)"
            ));
        }
    }
    examined.push(format!(
        "case closure: {tested} of {total} cited rules have a conformance case",
        total = rules.len(),
    ));
}

// ---------------------------------------------------------------------------------------
// The generated data
// ---------------------------------------------------------------------------------------

/// The first column of a generated table, or `None` when the table does not exist yet.
///
/// The first row may name the columns; every row after it carries an address in the
/// canonical rendering, and one that does not is a violation rather than a skipped line,
/// because a table this gate cannot read tells it nothing about the addresses it holds.
fn read_table(
    path: &Path,
    name: &str,
    violations: &mut Vec<String>,
) -> io::Result<Option<BTreeSet<String>>> {
    if !path.is_file() {
        return Ok(None);
    }
    Ok(Some(table_rows(
        &fs::read_to_string(path)?,
        name,
        violations,
    )))
}

/// The addresses a generated table's first column holds.
fn table_rows(text: &str, name: &str, violations: &mut Vec<String>) -> BTreeSet<String> {
    let mut rows = BTreeSet::new();
    let mut first = true;
    for (offset, line) in text.lines().enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let field = line.split('\t').next().unwrap_or("").trim();
        match address(field) {
            Some(parsed) if parsed.to_string() == field => {
                rows.insert(field.to_owned());
            },
            // A leading row that is not an address names the columns.
            _ if first => {},
            _ => violations.push(format!(
                "{name}:{line_number}: `{field}` is not a rule address in the canonical \
                 rendering (ADR 0013)",
                line_number = offset.saturating_add(1),
            )),
        }
        first = false;
    }
    rows
}

/// Every rule the conformance cases declare, or `None` when there are none yet.
fn read_cases(dir: &Path) -> io::Result<Option<BTreeSet<String>>> {
    if !dir.is_dir() {
        return Ok(None);
    }
    let mut files = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            files.push(path);
        }
    }
    files.sort();
    if files.is_empty() {
        return Ok(None);
    }
    let mut rules = BTreeSet::new();
    for file in files {
        let text = fs::read_to_string(&file)?;
        let declared = declared_rules(&text).map_err(|reason| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{path}: {reason}", path = file.display()),
            )
        })?;
        rules.extend(declared);
    }
    Ok(Some(rules))
}

/// Every string in a `rules` or `covers` array of one case file.
///
/// A deliberate minimal reader rather than a JSON parser, for the reason `xtask` declares
/// no dependencies at all. It reads keys by the colon that follows them, so a prose value
/// spelling `rules` is not mistaken for the field, and it leaves every other string
/// undecoded — no rule address contains an escape, and every other string is read only to
/// be discarded.
fn declared_rules(text: &str) -> Result<Vec<String>, String> {
    let mut found = Vec::new();
    let mut characters = text.chars().peekable();
    let mut key: Option<String> = None;
    let mut depth = 0usize;
    let mut collecting: Option<usize> = None;
    while let Some(character) = characters.next() {
        match character {
            '"' => {
                let value = string(&mut characters)?;
                while characters.peek().is_some_and(|it| it.is_whitespace()) {
                    characters.next();
                }
                if characters.peek() == Some(&':') {
                    characters.next();
                    key = Some(value);
                } else if collecting.is_some() {
                    found.push(value);
                }
            },
            '[' => {
                depth = depth.saturating_add(1);
                if collecting.is_none() && matches!(key.as_deref(), Some("rules" | "covers")) {
                    collecting = Some(depth);
                }
                key = None;
            },
            ']' => {
                if collecting == Some(depth) {
                    collecting = None;
                }
                depth = depth.saturating_sub(1);
            },
            _ => {},
        }
    }
    Ok(found)
}

/// Read one JSON string, the opening quote already consumed.
fn string(characters: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Result<String, String> {
    let mut value = String::new();
    while let Some(character) = characters.next() {
        match character {
            '"' => return Ok(value),
            '\\' => match characters.next() {
                Some(escaped) => value.push(escaped),
                None => break,
            },
            _ => value.push(character),
        }
    }
    Err("a string is opened and never closed".to_owned())
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::Path;

    use super::{
        Citation, Cited, Detail, Mention, Surface, address, check_cases, check_rules,
        check_sections, claim, claim_of, declared_rules, is_generated, read_addresses, read_table,
        scan, table_rows,
    };

    /// Build a citation index the way `read_addresses` does, from addresses alone.
    fn cited(addresses: &[&str]) -> Cited {
        let mut cited = BTreeMap::new();
        for text in addresses {
            let parsed = address(text).expect("the fixture is an address");
            cited.insert(
                parsed.to_string(),
                Mention {
                    section: parsed.section.to_string(),
                    place: "jlreq-unit: item.rs:1".to_owned(),
                    detailed: parsed.detail != Detail::Whole,
                },
            );
        }
        cited
    }

    /// The set a generated table would have produced.
    fn table(rows: &[&str]) -> BTreeSet<String> {
        rows.iter().map(|row| (*row).to_owned()).collect()
    }

    #[test]
    fn every_shape_of_the_grammar_parses_and_renders_back() {
        for text in [
            "3",
            "3.1",
            "3.1.9",
            "3.1.12",
            "1.2.3.4",
            "3.255",
            "A",
            "G",
            "A.19",
            "B.1",
            "F.3.4",
            "B.2#3",
            "C.2#12",
            "3.1.9#1",
            "B.2#255",
            "B.1@cl-05,cl-05",
            "B.1@cl-01,line-end",
            "D.1@line-head,cl-30",
        ] {
            let parsed = address(text).unwrap_or_else(|| panic!("`{text}` is an address"));
            assert_eq!(parsed.to_string(), text, "`{text}` renders back unchanged");
        }
    }

    #[test]
    fn a_non_canonical_spelling_is_not_an_address() {
        for text in [
            "",                // nothing
            "3.",              // a trailing dot
            "3..1",            // an empty component
            "03.1",            // a leading zero
            "3.0",             // a zero component
            "+3",              // a sign
            "3.256",           // past the representation
            "1.2.3.4.5",       // deeper than the grammar holds
            "H.1",             // not one of the seven appendices
            "b.2",             // the letter is upper case
            "B2",              // the dot is not optional
            "B.2#0",           // notes are numbered from one
            "B.2#",            // no ordinal
            "B.1@cl-05",       // half a cell
            "B.1@cl-5,cl-05",  // JLReq pads a class to two digits
            "B.1@cl-31,cl-01", // past the thirty classes
            "B.1@cl-00,cl-01", // there is no class zero
            "B.1@row,column",  // not a coordinate
            // The line head is a row and the line end is a column; each transposition
            // names a cell no matrix has.
            "B.1@cl-02,line-head",
            "B.1@line-end,cl-05",
            "B.1@line-end,line-head",
        ] {
            assert!(address(text).is_none(), "`{text}` is not an address");
        }
    }

    #[test]
    fn the_citation_shapes_the_workspace_writes_are_all_read() {
        for (text, count) in [
            ("n/a (arithmetic)", 0),
            ("n/a (ADR-0002, ADR-0007)", 0),
            ("n/a (`decision:remainder`)", 0),
            ("§3.1.6", 1),
            ("§B.1, ADR-0007", 1),
            ("§3.1.2, §3.2.4, §3.2.6, §A Remarks", 4),
            ("§B.1 `1/2 be hang`, `1/4 af hang`", 1),
            ("§B.1 blank cell", 1),
            ("§3.8.3 step 6", 1),
            ("§C.3 (silence), `decision:adjustment-preference`", 1),
            ("§A.25 U+002F", 1),
            ("§A.20–§A.23, §A.30", 5),
            ("§B.2#9–#11, §C.2#6–#8, §C.2#13", 7),
            ("§B.2#1, #2, #4, #6, #7, #8, #17", 7),
            ("§B.1@cl-05,cl-05", 1),
            ("§B.1@cl-05,cl-05, §B.2#3", 2),
        ] {
            let addresses = claim(text).unwrap_or_else(|error| panic!("`{text}`: {error}"));
            assert_eq!(addresses.len(), count, "`{text}` names {count} addresses");
        }
    }

    #[test]
    fn a_range_names_every_address_between_its_ends() {
        let addresses = claim("§A.20–§A.23").expect("the range is well formed");
        let rendered: Vec<String> = addresses.iter().map(ToString::to_string).collect();
        assert_eq!(rendered, ["A.20", "A.21", "A.22", "A.23"]);

        let notes = claim("§B.2#9–#11").expect("the range is well formed");
        let rendered: Vec<String> = notes.iter().map(ToString::to_string).collect();
        assert_eq!(rendered, ["B.2#9", "B.2#10", "B.2#11"]);
    }

    #[test]
    fn a_citation_naming_no_statement_of_the_specification_is_refused() {
        for text in [
            "",                   // an empty claim
            "ADR-0002",           // this project's reasoning, not the specification's text
            "ADR-0002, ADR-0007", // more of the same
            "n/a",                // no reason
            "n/a ()",             // an empty reason
            "n/a arithmetic",     // the reason is parenthesized
            "see section 3.1.9",  // prose
            "3.1.9",              // the sign is not optional
        ] {
            assert!(claim(text).is_err(), "`{text}` is not a citation");
        }
    }

    #[test]
    fn a_citation_naming_something_the_grammar_refuses_is_refused() {
        for text in [
            "§3.1.x",            // not an address
            "§cl-05",            // a coordinate is not a section
            "§B.2#0",            // notes are numbered from one
            "§A.20–",            // a range with one end
            "§A.23–§A.20",       // a range running downward
            "§A.20–§B.1",        // a range across two appendices
            "§3.1.9–§3.2.1",     // a range differing in more than its last number
            "§B.2#9–§C.2#11",    // a note range across two sections
            "#3",                // a continuation with nothing before it
            "§B.2, #0",          // a continuation that is not an ordinal
            "`1/4 af hang`",     // a token with nothing before it
            "§B.1, `1/2",        // an unclosed code span
            "ADR-7",             // a record is four digits
            "§B.1@cl-05, cl-05", // a cell writes its coordinates without a space
            "§3.1.9, ",          // a trailing comma
        ] {
            assert!(claim(text).is_err(), "`{text}` is not a citation");
        }
    }

    #[test]
    fn a_marker_inside_a_code_span_is_prose_and_not_a_citation() {
        assert_eq!(claim_of(" JLReq: §3.1.9"), Some("§3.1.9"));
        assert_eq!(
            claim_of(" Solid setting. JLReq: §B.1 blank cell"),
            Some("§B.1 blank cell")
        );
        assert_eq!(claim_of(" every public item carries a `JLReq:` line"), None);
        assert_eq!(claim_of(" no marker here"), None);
    }

    #[test]
    fn a_public_item_without_a_citation_is_found() {
        let source = "\
/// A quantity the writing system states.
///
/// JLReq: §B.1
pub struct Em(i32);

/// An index into the scale table.
pub struct ScaleId(u8);

impl Em {
    /// JLReq: n/a (arithmetic)
    #[must_use]
    pub const fn units(self) -> i32 {
        self.0
    }

    /// Solid setting. JLReq: §B.1 blank cell
    pub const ZERO: Self = Self(0);

    /// The bound.
    pub const LIMIT: i32 = 1;
}
";
        let scanned = scan(source, false);
        let missing: Vec<&str> = scanned
            .items
            .iter()
            .filter(|item| !item.cites)
            .map(|item| item.name.as_str())
            .collect();
        assert_eq!(missing, ["ScaleId", "Em::LIMIT"]);
        assert_eq!(scanned.items.len(), 5, "every public item is examined");
        assert_eq!(scanned.citations.len(), 3);
    }

    #[test]
    fn what_is_not_the_public_surface_is_not_required_to_cite() {
        let source = "\
//! A module. JLReq: §B.1

pub use crate::length::Em;

pub mod axis;

pub(crate) const UPPER: i32 = 1;

mod private {
    pub struct Reachable;
}

impl fmt::Display for Em {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, \"{}\", self.0)
    }
}

#[cfg(test)]
mod tests {
    pub fn helper() {}
}
";
        let scanned = scan(source, false);
        let names: Vec<&str> = scanned
            .items
            .iter()
            .map(|item| item.name.as_str())
            .collect();
        assert_eq!(
            names,
            ["Reachable"],
            "a `pub` item inside a private module is still public through a re-export"
        );
        assert_eq!(
            scanned.citations.len(),
            1,
            "a module citation is read for its grammar and attributed to no item"
        );
    }

    #[test]
    fn a_trait_definitions_items_are_public_without_saying_so() {
        let source = "\
/// What an implementation supplies. JLReq: n/a (ADR-0006)
pub trait Compose {
    /// The class of one item. JLReq: §3.9.2, §A
    fn classify(&self) -> u8;

    /// The composed lines.
    fn compose(&self) -> u8;
}
";
        let scanned = scan(source, false);
        let missing: Vec<&str> = scanned
            .items
            .iter()
            .filter(|item| !item.cites)
            .map(|item| item.name.as_str())
            .collect();
        assert_eq!(missing, ["Compose::compose"]);
    }

    #[test]
    fn a_generated_enum_variant_cites_and_a_hand_written_one_need_not() {
        let source = "\
/// What kind of claim a rule makes. JLReq: n/a (addressing)
pub enum Standing {
    /// Normative specification text.
    Normative,
    /// A silence.
    Unstated,
}
";
        let written = scan(source, false);
        assert!(
            written.items.iter().all(|item| item.cites),
            "a hand-written variant is not an item the requirement reaches"
        );
        let emitted = scan(source, true);
        let missing: Vec<&str> = emitted
            .items
            .iter()
            .filter(|item| !item.cites)
            .map(|item| item.name.as_str())
            .collect();
        assert_eq!(missing, ["Standing::Normative", "Standing::Unstated"]);
    }

    #[test]
    fn a_generated_source_is_known_by_its_directory() {
        assert!(is_generated(Path::new(
            "crates/jlreq-spec/src/generated/rules.rs"
        )));
        assert!(!is_generated(Path::new("crates/jlreq-spec/src/rule.rs")));
    }

    #[test]
    fn an_address_the_section_inventory_does_not_publish_is_a_violation() {
        let sections = table(&["3.1.9", "B.2"]);
        let mut examined = Vec::new();
        let mut violations = Vec::new();
        check_sections(
            Some(&sections),
            &cited(&["3.1.9", "B.2#3", "9.9.9"]),
            &mut examined,
            &mut violations,
        );
        assert_eq!(violations.len(), 1, "found {violations:?}");
        assert!(violations[0].contains("9.9.9"), "found {violations:?}");
    }

    #[test]
    fn a_detailed_address_the_rule_inventory_does_not_name_is_a_violation() {
        let inventory = table(&["3.1.9", "B.2#3"]);
        let mut examined = Vec::new();
        let mut violations = Vec::new();
        let rules = check_rules(
            Some(&inventory),
            &cited(&["3.1.9", "B.2#3", "B.2#7", "B.1"]),
            &mut examined,
            &mut violations,
        );
        assert_eq!(violations.len(), 1, "found {violations:?}");
        assert!(violations[0].contains("B.2#7"), "found {violations:?}");
        assert_eq!(
            rules,
            Some(table(&["3.1.9", "B.2#3"])),
            "a section-level citation is held by the section inventory instead"
        );
    }

    #[test]
    fn a_cited_rule_with_no_conformance_case_is_a_violation() {
        let mut examined = Vec::new();
        let mut violations = Vec::new();
        check_cases(
            Some(&table(&["3.1.9"])),
            Some(&table(&["3.1.9", "B.2#3"])),
            &cited(&["3.1.9", "B.2#3"]),
            &mut examined,
            &mut violations,
        );
        assert_eq!(violations.len(), 1, "found {violations:?}");
        assert!(violations[0].contains("B.2#3"), "found {violations:?}");
    }

    #[test]
    fn a_check_whose_data_does_not_exist_says_so_rather_than_passing() {
        let mut examined = Vec::new();
        let mut violations = Vec::new();
        check_sections(None, &cited(&["3.1.9"]), &mut examined, &mut violations);
        let rules = check_rules(None, &cited(&["3.1.9"]), &mut examined, &mut violations);
        check_cases(
            Some(&table(&[])),
            rules.as_ref(),
            &cited(&["3.1.9"]),
            &mut examined,
            &mut violations,
        );
        assert!(violations.is_empty(), "found {violations:?}");
        assert_eq!(examined.len(), 3);
        assert!(examined.iter().all(|line| line.contains("did not run")));
        assert!(
            examined
                .iter()
                .all(|line| line.contains(".tsv") || line.contains("cases")),
            "each names the file it waited for: {examined:?}"
        );
    }

    #[test]
    fn the_case_reader_finds_the_rules_arrays_and_not_a_string_that_says_rules() {
        let cases = "\
{
  \"section\": \"3.1.9\",
  \"cases\": [
    {
      \"id\": \"3.1.9/closing-bracket-at-line-end/half-em-frame\",
      \"rules\": [\"3.1.2\", \"3.1.9\", \"B.2#2\"],
      \"covers\": [\"B.1@cl-05,cl-05\"],
      \"rationale\": \"the rules\",
      \"permitted\": [{ \"source\": \"rules\" }],
      \"forbidden\": [{ \"why\": \"a quoted \\\"rules\\\" is not a field\" }]
    }
  ]
}
";
        let found = declared_rules(cases).expect("the fixture is readable");
        assert_eq!(found, ["3.1.2", "3.1.9", "B.2#2", "B.1@cl-05,cl-05"]);
    }

    #[test]
    fn an_unclosed_string_stops_the_case_reader_rather_than_shortening_the_set() {
        assert!(declared_rules("{ \"rules\": [\"3.1.9").is_err());
    }

    #[test]
    fn a_generated_table_names_its_columns_once_and_carries_addresses_after() {
        let mut violations = Vec::new();
        let rows = table_rows(
            "address\tstanding\tstatement\n3.1.9\tnormative\tA sentence.\nB.2#3\tnormative\tMore.\n",
            "rules.tsv",
            &mut violations,
        );
        assert!(violations.is_empty(), "found {violations:?}");
        assert_eq!(rows, table(&["3.1.9", "B.2#3"]));
    }

    #[test]
    fn a_table_row_that_is_not_a_canonical_address_is_a_violation() {
        let mut violations = Vec::new();
        table_rows(
            "address\tstanding\n3.1.9\tnormative\ncl-5\tnormative\n",
            "rules.tsv",
            &mut violations,
        );
        assert_eq!(violations.len(), 1, "found {violations:?}");
        assert!(violations[0].contains("cl-5"), "found {violations:?}");
        assert!(
            violations[0].contains("rules.tsv:3"),
            "found {violations:?}"
        );
    }

    #[test]
    fn a_table_that_does_not_exist_is_absent_rather_than_empty() {
        let mut violations = Vec::new();
        let rows = read_table(
            Path::new("spec/derived/nothing-is-here.tsv"),
            "nothing-is-here.tsv",
            &mut violations,
        )
        .expect("a missing file is not an error");
        assert_eq!(rows, None);
        assert!(violations.is_empty());
    }

    #[test]
    fn a_refused_citation_is_reported_where_it_was_written() {
        let surface = Surface {
            crates: 1,
            items: 2,
            citations: vec![
                Citation {
                    place: "jlreq-unit: item.rs:12".to_owned(),
                    text: "§3.1.2, §A Remarks".to_owned(),
                },
                Citation {
                    place: "jlreq-unit: item.rs:234".to_owned(),
                    text: "ADR-0002".to_owned(),
                },
            ],
        };
        let mut violations = Vec::new();
        let cited = read_addresses(&surface, &mut violations);
        assert_eq!(
            cited.keys().collect::<Vec<_>>(),
            ["3.1.2", "A"],
            "a well-formed citation contributes its addresses"
        );
        assert_eq!(violations.len(), 1, "found {violations:?}");
        assert!(
            violations[0].starts_with("jlreq-unit: item.rs:234:"),
            "found {violations:?}"
        );
    }

    #[test]
    fn the_gate_examines_the_workspace_it_runs_in() {
        let core = crate::shared::core_crates().expect("the workspace manifest is readable");
        let mut violations = Vec::new();
        let surface =
            super::read_surface(&core, &mut violations).expect("the sources are readable");
        assert!(surface.items > 0, "the layout core has public items");
        assert!(
            !surface.citations.is_empty(),
            "the layout core carries citations"
        );
    }
}
