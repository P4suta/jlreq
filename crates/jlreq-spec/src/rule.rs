// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Rule addressing: the specification's own identifier space, and the inventory it indexes.
//!
//! Four artifacts have to agree about what a rule is — the generated tables, the doc
//! comment on every public item, the conformance case files, and the coverage gate that
//! subtracts one set from the other. They agree because all four spell a rule the way the
//! specification spells it: a section path, optionally with a note ordinal, or one cell of
//! a matrix (see `docs/adr/0013`). An identifier invented here would have made a failure
//! report unreadable to the browser engineers and Typst maintainers the conformance suite
//! is written for, which is the whole point of the suite.
//!
//! [`Address`] is that spelling, parsed. One rendering is canonical and the parser accepts
//! nothing else, so an address that appears in a generated table, in a doc comment and in
//! a case file is the same bytes in all three.

use core::fmt;

/// The deepest section path an [`Address`] holds.
///
/// The published document's deepest section is three components and its longest appendix
/// path is a letter and two, so four is the specification's depth with headroom for a
/// revision that numbers one level deeper. The bound exists because this crate allocates
/// nothing, and it is part of what `parse` checks.
///
/// Crate-visible because the generated inventory writes an address as its components: the
/// emitter cannot call [`Address::parse`] and refuse a spelling at run time, so it states
/// the representation and the assertion below reads it back.
pub(crate) const MAX_PARTS: usize = 4;

/// The number of character classes §3.9.2 closes the set at.
const CLASS_COUNT: u8 = 30;

/// The width of a class coordinate, `cl-05`: JLReq zero-pads to two digits.
const CLASS_ID_LEN: usize = 5;

/// The largest ordinal a `u16` identifier can carry, which is `u16::MAX`.
///
/// Shared with the policy space, whose [`Question`](crate::Question) is the same shape for
/// the same reason: an identifier into generated data, one machine word wide at most.
pub(crate) const LARGEST_ORDINAL: usize = 65_535;

// A rule is addressed by a `u16` ordinal into the generated inventory, so an inventory
// that outgrew one would alias two rules onto one identifier. Several thousand table cells
// are rules (docs/adr/0013), so the headroom is worth stating mechanically rather than
// assuming.
const _: () = assert!(RULES.len() <= LARGEST_ORDINAL);

// Every generated address is canonical, so the bytes in a table, in a doc comment and in a
// case file are the same bytes. A generated file that fails this does not compile, which
// is the cheapest place to catch an emitter that learned a second spelling.
const _: () = {
    let mut index = 0;
    while index < RULES.len() {
        assert!(
            RULES[index].address.is_canonical(),
            "a generated rule address is not in the canonical form `Address::parse` accepts"
        );
        index = index.saturating_add(1);
    }
};

/// A stable identifier for one normative statement of JLReq.
///
/// The address is the specification's own, so a failure report is readable by someone who
/// has never seen this code (ADR-0013). Generated from the rule inventory.
///
/// JLReq: n/a (addressing)
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct RuleId(pub(crate) u16);

impl RuleId {
    // The named constants — `LINE_START_PROHIBITION` (3.1.7), `MIDDLE_DOT_SUM` (B.2#3),
    // `INSEPARABLE_PAIRS` (C.2#5), and one per inventoried rule — are emitted beside
    // `RULES` rather than written here, because a hand-written constant is a
    // transcription of the section number it names (docs/adr/0009). They arrive with the
    // inventory, in a second `impl RuleId` block.

    /// Every rule kumihan implements. The coverage gate subtracts from this.
    ///
    /// JLReq: n/a (addressing)
    pub const ALL: &'static [Self] = &identifiers();

    /// The canonical rendering: `3.1.9`, `B.2#3`, `B.1@cl-05,cl-05`.
    ///
    /// JLReq: n/a (addressing)
    #[must_use]
    pub const fn address(self) -> Address {
        RULES[self.ordinal()].address
    }

    /// The sentence, quoted from the specification.
    ///
    /// JLReq: n/a (addressing)
    #[must_use]
    pub const fn statement(self) -> &'static str {
        RULES[self.ordinal()].statement
    }

    /// Whether this rule reads the writing direction (ADR-0011). Exactly three
    /// do.
    ///
    /// JLReq: §3.1.3, §3.2.5, §3.3.5
    #[must_use]
    pub const fn is_direction_conditional(self) -> bool {
        RULES[self.ordinal()].direction_conditional
    }

    /// What kind of claim this rule is.
    ///
    /// JLReq: n/a (addressing)
    #[must_use]
    pub const fn standing(self) -> Standing {
        RULES[self.ordinal()].standing
    }

    /// The rule at `address`, or `None` when the inventory has no such rule.
    ///
    /// A well-formed address that names nothing is `None` like a malformed one, because
    /// both are the same failure to the caller reading a case file: a citation this
    /// library cannot answer for.
    ///
    /// JLReq: n/a (addressing)
    pub fn parse(address: &str) -> Option<Self> {
        let address = Address::parse(address)?;
        let ordinal = RULES.iter().position(|rule| rule.address == address)?;
        u16::try_from(ordinal).ok().map(Self)
    }

    /// This rule's position in the generated inventory.
    const fn ordinal(self) -> usize {
        self.0 as usize
    }
}

/// One identifier per inventoried rule, in the inventory's own order.
///
/// Derived rather than emitted: the inventory states each rule once, and its identifiers
/// are that statement counted (ADR-0019).
const fn identifiers() -> [RuleId; RULES.len()] {
    let mut all = [RuleId(0); RULES.len()];
    let mut index = 0;
    let mut ordinal = 0u16;
    while index < RULES.len() {
        all[index] = RuleId(ordinal);
        index = index.saturating_add(1);
        ordinal = ordinal.saturating_add(1);
    }
    all
}

/// A parsed specification address. Byte-identical in the tables, in doc comments, and in
/// the conformance case files.
///
/// Grammar: `section := digit+ ('.' digit+)* | [A-G] ('.' digit+)*`,
/// `address := section ('#' note)? | section '@' cell`.
/// The `#` is kumihan's separator for JLReq's "note N", which the published document
/// gives no machine-readable identifier; that is recorded rather than glossed over.
///
/// One rendering is canonical and [`Address::parse`] accepts nothing else. A number
/// carries no leading zero and starts at one, because the specification numbers its
/// sections, its tables and its notes from one; a class coordinate is padded to two digits
/// because that is what JLReq writes — 302 occurrences of `cl-01` and none of `cl-1`. So
/// parsing and rendering round-trip in both directions, and one rule has one spelling
/// wherever it appears.
///
/// A cell coordinate is a class or one of the two line edges that §B.1 and §D.1 carry as
/// an extra row and column, written `line-head` and `line-end`. That spelling is this
/// project's, for the same reason the `#` is: the published matrices print those two axes
/// as prose labels and give them no identifier. It is used byte-identically wherever the
/// two edges appear — in the captured transcription, in the generated inventory, in a doc
/// comment and in a case file — so nothing anywhere translates between two forms of it.
///
/// The row and the column are not interchangeable. A matrix carries one line-head row and
/// one line-end column, so `B.1@cl-02,line-head` and `B.1@line-end,cl-05` address cells no
/// matrix has and are not addresses.
///
/// The section part is the specification's *rendered* section number and never its
/// published anchor slug, which is off by one — `legend_of_table_2` renders "B.1 Legend of
/// Table 1", so an address keyed on the anchor would misnumber every matrix (see
/// `docs/design/generation.md`).
///
/// JLReq: n/a (addressing)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Address(pub(crate) Detail);

impl Address {
    /// Parse the canonical rendering. `None` when `text` is not one.
    ///
    /// This is the only way to build an address, which is what makes the canonical form
    /// the only form: a spelling the parser rejects has no value to reach a table, a
    /// report or a case file with.
    ///
    /// # Examples
    ///
    /// ```
    /// use jlreq_spec::Address;
    ///
    /// let cell = Address::parse("B.1@cl-05,cl-05");
    /// assert_eq!(cell.map(|cell| cell.to_string()).as_deref(), Some("B.1@cl-05,cl-05"));
    ///
    /// // JLReq writes `cl-05` and never `cl-5`, so only one of the two is an address.
    /// assert_eq!(Address::parse("B.1@cl-5,cl-05"), None);
    /// ```
    ///
    /// JLReq: n/a (addressing)
    #[must_use]
    pub const fn parse(text: &str) -> Option<Self> {
        let bytes = text.as_bytes();
        let (head, tail) = bytes.split_at(separator(bytes));
        let Some(section) = parse_section(head) else {
            return None;
        };
        if tail.is_empty() {
            return Some(Self(Detail::Section(section)));
        }
        let (marker, rest) = tail.split_at(1);
        match marker[0] {
            b'#' => {
                let Some(note) = parse_number(rest) else {
                    return None;
                };
                Some(Self(Detail::Note(section, note)))
            },
            b'@' => {
                let Some((before, after)) = parse_cell(rest) else {
                    return None;
                };
                Some(Self(Detail::Cell(section, before, after)))
            },
            _ => None,
        }
    }

    /// The address a generated row states, assembled from its rendered components.
    ///
    /// The emitter writes the representation directly rather than calling [`parse`], because
    /// a generated table is a `const` and a parse that could answer `None` at run time is
    /// not one. Nothing is trusted for it: the compile-time assertion over `RULES` reads
    /// every assembled address back through [`is_canonical`], so a component the grammar
    /// refuses is a build failure rather than a rule nobody can cite.
    ///
    /// A `note` of zero is the absence of one, which is unambiguous because JLReq numbers
    /// its notes from one and `is_canonical` refuses a zeroth note.
    ///
    /// [`parse`]: Address::parse
    /// [`is_canonical`]: Address::is_canonical
    pub(crate) const fn assembled(
        appendix: Option<Appendix>,
        values: [u8; MAX_PARTS],
        depth: u8,
        note: u8,
    ) -> Self {
        let section = Section {
            appendix,
            parts: Parts { values, depth },
        };
        if note == 0 {
            Self(Detail::Section(section))
        } else {
            Self(Detail::Note(section, note))
        }
    }

    /// Whether this address names a numbered note of its section rather than the whole of
    /// it.
    ///
    /// Crate-visible because the two are what the inventory partitions into and the size of
    /// each partition is asserted against the published document.
    pub(crate) const fn is_note(self) -> bool {
        matches!(self.0, Detail::Note(_, _))
    }

    /// Whether this address is the one form `parse` accepts and `Display` writes.
    ///
    /// Held over generated data by a compile-time assertion, because the emitter writes
    /// the representation directly and equality is only the rendering's equality while
    /// every value is canonical.
    pub(crate) const fn is_canonical(self) -> bool {
        match self.0 {
            Detail::Section(section) => section.is_canonical(),
            // Note ordinals are one-based: JLReq has no note 0 to cite.
            Detail::Note(section, note) => section.is_canonical() && note >= 1,
            Detail::Cell(section, before, after) => {
                section.is_canonical() && before.is_canonical() && after.is_canonical()
            },
        }
    }
}

impl fmt::Display for Address {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Detail::Section(section) => write!(formatter, "{section}"),
            Detail::Note(section, note) => write!(formatter, "{section}#{note}"),
            Detail::Cell(section, before, after) => {
                write!(formatter, "{section}@{before},{after}")
            },
        }
    }
}

/// What kind of claim a rule makes.
///
/// The last two exist because the specification contradicts itself in places and leaves
/// holes in others. A library that quietly filled them would publish invention as
/// requirement (ADR-0013).
///
/// JLReq: n/a (addressing)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Standing {
    /// Normative specification text.
    Normative,
    /// JLReq permits several answers here; the choice is a [`Question`](crate::Question).
    Alternative,
    /// JLReq says nothing. This is kumihan's published reading, recorded in
    /// `docs/decisions/`. Examples: an unlisted code point (§3.9.2), emphasis dots
    /// (圏点) having no class or table row (§3.3.9), warichu (割注) line adjustment
    /// (§3.8.3).
    Unstated,
    /// The specification says two incompatible things. Both are recorded; the case
    /// carries both readings. Examples: §D.2 note 5 against notes 1–3 on a priority
    /// ordinal; §3.1.3's Note reading "vertical" in English against 横組 in Japanese;
    /// §3.8.3 numbering Appendix D's tables one higher than Appendix D does.
    Adjudicated,
}

impl Standing {
    /// Whether the specification decides an answer of this standing.
    ///
    /// Normative text decides, and so does an alternative: JLReq states the permitted set
    /// and the caller's policy picks within it. A silence decides nothing, and neither
    /// does a contradiction — there the specification spoke twice and this project chose,
    /// which is exactly the case a caller needs to be able to tell apart from a
    /// requirement.
    pub(crate) const fn is_specified(self) -> bool {
        matches!(self, Self::Normative | Self::Alternative)
    }

    /// The weaker of two standings, as a chain of rules is only as specified as its least
    /// specified step.
    ///
    /// The order runs Normative, Alternative, Adjudicated, Unstated: an adjudication is at
    /// least an answer the specification states somewhere, while a silence is one it never
    /// states at all. No public ordering exists, because the order is this reading and not
    /// the specification's.
    pub(crate) const fn weakest(self, other: Self) -> Self {
        if other.distance() > self.distance() {
            other
        } else {
            self
        }
    }

    /// How far an answer of this standing is from the specification's own word.
    const fn distance(self) -> u8 {
        match self {
            Self::Normative => 0,
            Self::Alternative => 1,
            Self::Adjudicated => 2,
            Self::Unstated => 3,
        }
    }
}

/// Every inventoried rule, in the specification's own reading order.
///
/// Generated. Stage 1 of the pipeline reads the vendored snapshot of the published
/// specification into `spec/derived/rules.tsv`, and stage 2 turns that file into
/// `src/generated/inventory.rs`, which holds these rows and one named [`RuleId`] constant
/// per row (see `docs/design/generation.md`). A hand edit to either is a bug even when it
/// is correct, because the next revision of the specification will not carry it forward
/// (ADR-0009).
///
/// What the inventory covers is §3 and Appendices B through F: every section that states
/// something in its own words, and every note of the four `Notes` sections. The matrix
/// cells `Detail::Cell` addresses are transcribed rather than derived and join this table
/// with the captured matrices, so a cell address parses today and resolves to no rule.
pub(crate) use crate::generated::inventory::RULES;

/// One row of the rule inventory.
///
/// The columns are `spec/derived/rules.tsv`'s, one row per normative statement, per
/// appendix note, and per matrix cell (ADR-0013).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Rule {
    /// Where the specification states it.
    pub(crate) address: Address,
    /// The sentence, quoted.
    pub(crate) statement: &'static str,
    /// Whether evaluating it consults the writing direction (ADR-0011).
    pub(crate) direction_conditional: bool,
    /// What kind of claim it makes.
    pub(crate) standing: Standing,
}

/// What an address addresses within a section.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Detail {
    /// The section itself: `3.1.9`.
    Section(Section),
    /// One numbered note of the section: `B.2#3`.
    Note(Section, u8),
    /// One cell of the matrix the section publishes, row then column:
    /// `B.1@cl-05,cl-05`.
    Cell(Section, Before, After),
}

/// A section path: an optional appendix letter and its numbered components.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Section {
    /// The appendix, when the path is one of the seven lettered ones.
    pub(crate) appendix: Option<Appendix>,
    /// The numbered components, `3.1.9`'s three or `B.2`'s one.
    pub(crate) parts: Parts,
}

impl Section {
    /// Whether this path is the one form `parse` accepts.
    ///
    /// A path with neither a letter nor a number renders as nothing, which is not an
    /// address; everything else about a path renders and re-parses unchanged.
    const fn is_canonical(self) -> bool {
        self.parts.is_canonical() && (self.appendix.is_some() || self.parts.depth >= 1)
    }
}

impl fmt::Display for Section {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(appendix) = self.appendix {
            formatter.write_str(appendix.letter())?;
        }
        for (index, component) in self.parts.components().iter().enumerate() {
            if index > 0 || self.appendix.is_some() {
                formatter.write_str(".")?;
            }
            write!(formatter, "{component}")?;
        }
        Ok(())
    }
}

/// The numbered components of a section path, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Parts {
    /// The components, zero beyond `depth` so that equality is the rendering's equality.
    pub(crate) values: [u8; MAX_PARTS],
    /// How many of `values` are components.
    pub(crate) depth: u8,
}

impl Parts {
    /// The components, without the unused tail.
    ///
    /// The depth is brought inside the array rather than trusted, so this is total for
    /// every value of the type. A depth past `MAX_PARTS` is refused by `is_canonical` and
    /// therefore cannot reach the inventory; a `split_at` that could panic on it would
    /// still be a panic path in a crate whose selling point is that it has none.
    const fn components(&self) -> &[u8] {
        let depth = if self.depth as usize > MAX_PARTS {
            MAX_PARTS
        } else {
            self.depth as usize
        };
        let (used, _) = self.values.split_at(depth);
        used
    }

    /// Whether this is the one representation of the path it renders.
    ///
    /// Three conditions. The path fits; every component is a number the specification
    /// could have written, which is one or more; and nothing is left behind the depth,
    /// which is the one an emitter can get wrong, because a stray component is invisible
    /// to the rendering and visible to equality and would make two equal addresses compare
    /// unequal.
    const fn is_canonical(self) -> bool {
        if self.depth as usize > MAX_PARTS {
            return false;
        }
        let mut index = 0;
        while index < MAX_PARTS {
            let inside = index < self.depth as usize;
            let component = self.values[index];
            if inside && component == 0 {
                return false;
            }
            if !inside && component != 0 {
                return false;
            }
            index = index.saturating_add(1);
        }
        true
    }
}

/// One of the seven appendices JLReq publishes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Appendix {
    /// Character classes.
    A,
    /// Spacing between characters.
    B,
    /// Line breaking.
    C,
    /// Inter-character space reduction.
    D,
    /// Inter-character space expansion.
    E,
    /// Jukugo (熟語) ruby.
    F,
    /// The seventh, so the grammar's `[A-G]` is complete.
    G,
}

impl Appendix {
    /// The letter, as the specification renders it.
    const fn letter(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
            Self::E => "E",
            Self::F => "F",
            Self::G => "G",
        }
    }

    /// The appendix a leading byte names, or `None` when it names no appendix.
    const fn of(letter: u8) -> Option<Self> {
        match letter {
            b'A' => Some(Self::A),
            b'B' => Some(Self::B),
            b'C' => Some(Self::C),
            b'D' => Some(Self::D),
            b'E' => Some(Self::E),
            b'F' => Some(Self::F),
            b'G' => Some(Self::G),
            _ => None,
        }
    }
}

/// The row coordinate of a matrix cell: a character class, or the line head.
///
/// The two coordinates are two types and not one, because the matrices are not symmetric
/// in them. Table 1 and Tables 3 through 5 carry **one line-head row and one line-end
/// column** and nothing else — the frozen reason `jlreq_spacing::Before` and
/// `jlreq_spacing::After` are two types in `docs/api-frozen.toml` — so a line-head column
/// addresses a cell no matrix has, and a symmetric coordinate would make that address
/// well formed and canonical. The address space is a one-way door, so the restriction is
/// the type rather than a sentence beside it (ADR-0013, ADR-0016).
///
/// Table 2 and Table 6 have no line-edge axes at all, which is a property of those
/// matrices and not of this vocabulary (see `docs/design/generation.md`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Before {
    /// One of the thirty classes, `cl-01` through `cl-30`.
    Class(u8),
    /// The line head row.
    LineHead,
}

/// The column coordinate of a matrix cell: a character class, or the line end.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum After {
    /// One of the thirty classes, `cl-01` through `cl-30`.
    Class(u8),
    /// The line end column.
    LineEnd,
}

impl Before {
    /// Whether this coordinate names a class the specification defines.
    const fn is_canonical(self) -> bool {
        match self {
            Self::Class(number) => is_class(number),
            Self::LineHead => true,
        }
    }
}

impl After {
    /// Whether this coordinate names a class the specification defines.
    const fn is_canonical(self) -> bool {
        match self {
            Self::Class(number) => is_class(number),
            Self::LineEnd => true,
        }
    }
}

/// Whether a number names one of the classes §3.9.2 closes the set at.
const fn is_class(number: u8) -> bool {
    number >= 1 && number <= CLASS_COUNT
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

/// The offset of the first `#` or `@`, or the whole length when there is neither.
const fn separator(bytes: &[u8]) -> usize {
    let mut index = 0;
    while index < bytes.len() {
        if matches!(bytes[index], b'#' | b'@') {
            return index;
        }
        index = index.saturating_add(1);
    }
    index
}

/// Parse a section path: an optional appendix letter, then dot-separated numbers.
const fn parse_section(bytes: &[u8]) -> Option<Section> {
    if bytes.is_empty() {
        return None;
    }
    let appendix = Appendix::of(bytes[0]);
    let mut index = if appendix.is_some() { 1 } else { 0 };
    let mut values = [0u8; MAX_PARTS];
    let mut depth = 0u8;
    let mut first = true;

    while index < bytes.len() {
        // A body section opens with a number; every other component follows a dot.
        if appendix.is_some() || !first {
            if bytes[index] != b'.' {
                return None;
            }
            index = index.saturating_add(1);
        }
        first = false;
        let start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index = index.saturating_add(1);
        }
        let (upto, _) = bytes.split_at(index);
        let (_, digits) = upto.split_at(start);
        let Some(value) = parse_number(digits) else {
            return None;
        };
        if depth as usize >= MAX_PARTS {
            return None;
        }
        values[depth as usize] = value;
        depth = depth.saturating_add(1);
    }

    if appendix.is_none() && depth == 0 {
        return None;
    }
    Some(Section {
        appendix,
        parts: Parts { values, depth },
    })
}

/// Parse one number of an address — a section component or a note ordinal — rejecting
/// every non-canonical spelling of it: an empty run of digits, a leading zero, a zero, and
/// a value this representation cannot hold.
///
/// The specification numbers its sections, its tables and its notes from one, so a zero
/// addresses nothing and is refused here rather than left to the inventory to fail on.
const fn parse_number(digits: &[u8]) -> Option<u8> {
    if digits.is_empty() || digits[0] == b'0' {
        return None;
    }
    let mut value: u8 = 0;
    let mut index = 0;
    while index < digits.len() {
        let digit = digits[index];
        if !digit.is_ascii_digit() {
            return None;
        }
        let Some(scaled) = value.checked_mul(10) else {
            return None;
        };
        let Some(sum) = scaled.checked_add(digit.wrapping_sub(b'0')) else {
            return None;
        };
        value = sum;
        index = index.saturating_add(1);
    }
    Some(value)
}

/// Parse a cell coordinate pair, row first.
const fn parse_cell(bytes: &[u8]) -> Option<(Before, After)> {
    let mut index = 0;
    while index < bytes.len() && bytes[index] != b',' {
        index = index.saturating_add(1);
    }
    if index == bytes.len() {
        return None;
    }
    let (row, rest) = bytes.split_at(index);
    let (_, column) = rest.split_at(1);
    let Some(before) = parse_before(row) else {
        return None;
    };
    let Some(after) = parse_after(column) else {
        return None;
    };
    Some((before, after))
}

/// Parse a row coordinate: `cl-05` or `line-head`, and never `line-end`.
const fn parse_before(bytes: &[u8]) -> Option<Before> {
    if equals(bytes, b"line-head") {
        return Some(Before::LineHead);
    }
    match parse_class(bytes) {
        Some(number) => Some(Before::Class(number)),
        None => None,
    }
}

/// Parse a column coordinate: `cl-05` or `line-end`, and never `line-head`.
const fn parse_after(bytes: &[u8]) -> Option<After> {
    if equals(bytes, b"line-end") {
        return Some(After::LineEnd);
    }
    match parse_class(bytes) {
        Some(number) => Some(After::Class(number)),
        None => None,
    }
}

/// Parse a class coordinate, `cl-05`. JLReq zero-pads to two digits, so `cl-5` is not one.
const fn parse_class(bytes: &[u8]) -> Option<u8> {
    if bytes.len() != CLASS_ID_LEN {
        return None;
    }
    if bytes[0] != b'c' || bytes[1] != b'l' || bytes[2] != b'-' {
        return None;
    }
    let (_, digits) = bytes.split_at(3);
    if !digits[0].is_ascii_digit() || !digits[1].is_ascii_digit() {
        return None;
    }
    let tens = digits[0].wrapping_sub(b'0');
    let ones = digits[1].wrapping_sub(b'0');
    let Some(scaled) = tens.checked_mul(10) else {
        return None;
    };
    let Some(number) = scaled.checked_add(ones) else {
        return None;
    };
    if is_class(number) { Some(number) } else { None }
}

/// Whether two byte strings are the same, in a constant context.
const fn equals(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut index = 0;
    while index < left.len() {
        if left[index] != right[index] {
            return false;
        }
        index = index.saturating_add(1);
    }
    true
}

#[cfg(test)]
mod tests {
    use core::fmt::{self, Write as _};

    use super::{
        Address, After, Appendix, Before, Detail, Parts, Rule, RuleId, Section, Standing, equals,
    };

    /// The shared address corpus: one row per spelling, and whether it is an address.
    ///
    /// The grammar has two carriers in this repository and cannot have one — this parser
    /// is `no_std` and `const`, and `xtask`'s is `std` and declares no dependencies, so
    /// neither can call the other. This file is what holds them equal: both read it, both
    /// assert their own answer against the `accepted` column, and a spelling one accepts
    /// and the other refuses fails a test rather than reaching a case file (ADR-0013,
    /// ADR-0019).
    const CORPUS: &str = include_str!("../../../docs/design/address-corpus.tsv");

    /// One row of the corpus: the spelling, and whether it is an address.
    fn corpus() -> impl Iterator<Item = (&'static str, bool)> {
        CORPUS.lines().skip(1).filter_map(|line| {
            let mut fields = line.split('\t');
            let text = fields.next()?;
            let accepted = match fields.next()? {
                "yes" => true,
                "no" => false,
                other => panic!("`{text}` is neither accepted nor rejected but `{other}`"),
            };
            Some((text, accepted))
        })
    }

    /// A fixed-capacity sink, so a rendering can be compared without an allocator.
    struct Rendered {
        bytes: [u8; 32],
        len: usize,
    }

    impl Rendered {
        /// Render one address.
        fn of(address: Address) -> Self {
            let mut rendered = Self {
                bytes: [0; 32],
                len: 0,
            };
            write!(&mut rendered, "{address}").expect("an address fits in the buffer");
            rendered
        }

        /// The rendering, as text.
        fn as_str(&self) -> &str {
            let (written, _) = self.bytes.split_at(self.len);
            core::str::from_utf8(written).expect("an address renders as ASCII")
        }
    }

    impl fmt::Write for Rendered {
        fn write_str(&mut self, text: &str) -> fmt::Result {
            let end = self.len.checked_add(text.len()).ok_or(fmt::Error)?;
            let slot = self.bytes.get_mut(self.len..end).ok_or(fmt::Error)?;
            slot.copy_from_slice(text.as_bytes());
            self.len = end;
            Ok(())
        }
    }

    #[test]
    fn every_address_of_the_corpus_round_trips_through_both_directions() {
        let mut accepted = 0_usize;
        for (text, _) in corpus().filter(|(_, accepted)| *accepted) {
            let address = Address::parse(text).unwrap_or_else(|| panic!("`{text}` is canonical"));
            assert_eq!(Rendered::of(address).as_str(), text, "rendering `{text}`");
            assert_eq!(
                Address::parse(Rendered::of(address).as_str()),
                Some(address),
                "re-parsing `{text}`"
            );
            assert!(address.is_canonical(), "`{text}` is canonical");
            accepted = accepted.saturating_add(1);
        }
        assert!(accepted > 20, "the corpus was read: {accepted} addresses");
    }

    #[test]
    fn no_second_spelling_of_an_address_is_accepted() {
        let mut rejected = 0_usize;
        for (text, _) in corpus().filter(|(_, accepted)| !*accepted) {
            assert_eq!(Address::parse(text), None, "`{text}` is not an address");
            rejected = rejected.saturating_add(1);
        }
        assert!(rejected > 40, "the corpus was read: {rejected} refusals");
    }

    #[test]
    fn acceptance_is_exactly_agreement_with_the_rendering() {
        // The property the corpus is instances of: a string is an address when it is what
        // the address it names renders as, and never otherwise.
        for (text, _) in corpus() {
            let round_trip = Address::parse(text).map(Rendered::of);
            let agrees = round_trip.is_some_and(|rendered| rendered.as_str() == text);
            assert_eq!(
                agrees,
                Address::parse(text).is_some(),
                "`{text}` is accepted if and only if it is its own rendering"
            );
        }
    }

    #[test]
    fn two_addresses_of_the_corpus_are_equal_only_when_they_render_alike() {
        for (text, _) in corpus().filter(|(_, accepted)| *accepted) {
            let address = Address::parse(text).unwrap_or_else(|| panic!("`{text}` is canonical"));
            for (other_text, _) in corpus().filter(|(_, accepted)| *accepted) {
                if other_text == text {
                    continue;
                }
                let other =
                    Address::parse(other_text).unwrap_or_else(|| panic!("`{other_text}` parses"));
                assert_ne!(address, other, "`{text}` and `{other_text}` are two rules");
            }
        }
    }

    #[test]
    fn a_representation_the_parser_cannot_produce_is_not_canonical() {
        // Each of these is what an emitter writing the representation directly could get
        // wrong, and each is what the compile-time assertion over `RULES` refuses.
        let stray_component = Address(Detail::Section(Section {
            appendix: None,
            parts: Parts {
                values: [3, 1, 9, 7],
                depth: 3,
            },
        }));
        assert!(
            !stray_component.is_canonical(),
            "a component behind the depth renders invisibly and compares visibly"
        );

        let empty_path = Address(Detail::Section(Section {
            appendix: None,
            parts: Parts {
                values: [0, 0, 0, 0],
                depth: 0,
            },
        }));
        assert!(!empty_path.is_canonical(), "a path with nothing in it");

        let zeroth_component = Address(Detail::Section(Section {
            appendix: None,
            parts: Parts {
                values: [3, 0, 0, 0],
                depth: 2,
            },
        }));
        assert!(
            !zeroth_component.is_canonical(),
            "the specification numbers its sections from one"
        );

        let whole_appendix = Address(Detail::Section(Section {
            appendix: Some(Appendix::B),
            parts: Parts {
                values: [0, 0, 0, 0],
                depth: 0,
            },
        }));
        assert!(whole_appendix.is_canonical(), "`B` is an address");

        let zeroth_note = Address(Detail::Note(
            Section {
                appendix: Some(Appendix::B),
                parts: Parts {
                    values: [2, 0, 0, 0],
                    depth: 1,
                },
            },
            0,
        ));
        assert!(!zeroth_note.is_canonical(), "JLReq numbers notes from one");

        let unknown_class = Address(Detail::Cell(
            Section {
                appendix: Some(Appendix::B),
                parts: Parts {
                    values: [1, 0, 0, 0],
                    depth: 1,
                },
            },
            Before::Class(31),
            After::LineEnd,
        ));
        assert!(
            !unknown_class.is_canonical(),
            "§3.9.2 closes the set at thirty"
        );
    }

    #[test]
    fn a_note_and_a_cell_of_one_section_are_different_rules() {
        let section = Address::parse("B.1").expect("a section");
        let note = Address::parse("B.1#1").expect("a note");
        let cell = Address::parse("B.1@cl-05,cl-05").expect("a cell");
        assert_ne!(section, note);
        assert_ne!(section, cell);
        assert_ne!(note, cell);
    }

    #[test]
    fn a_cell_is_ordered_row_then_column() {
        let row = Address::parse("B.1@cl-01,cl-02").expect("a cell");
        let column = Address::parse("B.1@cl-02,cl-01").expect("a cell");
        assert_ne!(row, column, "the row and the column are different axes");
        assert_eq!(Rendered::of(row).as_str(), "B.1@cl-01,cl-02");
    }

    #[test]
    fn a_line_edge_belongs_to_one_axis_and_not_the_other() {
        assert!(
            Address::parse("B.1@line-head,cl-02").is_some(),
            "the line head is a row"
        );
        assert!(
            Address::parse("B.1@cl-02,line-end").is_some(),
            "the line end is a column"
        );
        assert_eq!(
            Address::parse("B.1@cl-02,line-head"),
            None,
            "a line-head column addresses a cell no matrix has, and the address space is              a one-way door"
        );
        assert_eq!(
            Address::parse("B.1@line-end,cl-02"),
            None,
            "and neither does a line-end row"
        );
    }

    #[test]
    fn a_rule_is_reached_by_the_address_the_specification_gives_it() {
        let closing = RuleId::parse("3.1.9").expect("§3.1.9 states something in its own words");
        assert_eq!(
            Rendered::of(closing.address()).as_str(),
            "3.1.9",
            "the address parsed and the address reported are the same spelling"
        );
        assert_eq!(closing.standing(), Standing::Normative);
        assert!(
            closing.statement().contains("closing brackets"),
            "the sentence is quoted from the published document"
        );
    }

    #[test]
    fn an_address_the_inventory_does_not_name_is_no_rule() {
        assert_eq!(
            RuleId::parse("B.1@cl-05,cl-05"),
            None,
            "a matrix cell is transcribed rather than derived and joins the inventory with \
             the captured matrices"
        );
        assert_eq!(
            RuleId::parse("2.1.1"),
            None,
            "the inventory covers §3 and Appendices B through F: a rule no layer here \
             answers for could never gain the case ADR 0013 requires of every one"
        );
        assert_eq!(RuleId::parse("not an address"), None);
    }

    #[test]
    fn every_identifier_of_the_inventory_addresses_its_own_row() {
        for (ordinal, rule) in RuleId::ALL.iter().enumerate() {
            let address = Rendered::of(rule.address());
            assert_eq!(
                RuleId::parse(address.as_str()),
                Some(*rule),
                "identifier {ordinal} and its address name one rule"
            );
            assert!(!rule.statement().is_empty());
        }
    }

    #[test]
    fn exactly_three_rules_read_the_writing_direction() {
        let marked = RuleId::ALL
            .iter()
            .filter(|rule| rule.is_direction_conditional())
            .map(|rule| Rendered::of(rule.address()));
        assert!(
            marked
                .into_iter()
                .map(|rendered| rendered.as_str() == "3.1.3"
                    || rendered.as_str() == "3.2.5"
                    || rendered.as_str() == "3.3.5")
                .eq([true, true, true]),
            "ADR 0011 fixes §3.1.3, §3.2.5 and §3.3.5, and a fourth is a change to generated \
             data plus a code-owner review rather than an incidental branch"
        );
    }

    #[test]
    fn a_standing_is_specified_when_the_specification_decides_it() {
        assert!(Standing::Normative.is_specified());
        assert!(
            Standing::Alternative.is_specified(),
            "JLReq states the permitted set; the policy picks within it"
        );
        assert!(!Standing::Unstated.is_specified());
        assert!(
            !Standing::Adjudicated.is_specified(),
            "a specification that says two incompatible things decides neither"
        );
    }

    #[test]
    fn the_weakest_standing_wins_a_chain() {
        assert_eq!(
            Standing::Normative.weakest(Standing::Unstated),
            Standing::Unstated
        );
        assert_eq!(
            Standing::Unstated.weakest(Standing::Normative),
            Standing::Unstated,
            "the combination does not depend on the order of the chain"
        );
        assert_eq!(
            Standing::Alternative.weakest(Standing::Adjudicated),
            Standing::Adjudicated
        );
        assert_eq!(
            Standing::Normative.weakest(Standing::Normative),
            Standing::Normative
        );
    }

    #[test]
    fn a_rule_reads_its_row_of_the_inventory() {
        // The inventory is generated and is empty today, so this exercises the row shape
        // against a row of the same type rather than against `RULES`.
        let row = Rule {
            address: Address::parse("3.1.3").expect("a section"),
            statement: "In vertical writing mode, ideographic numerals are set solid.",
            direction_conditional: true,
            standing: Standing::Adjudicated,
        };
        assert_eq!(Rendered::of(row.address).as_str(), "3.1.3");
        assert!(row.direction_conditional);
        assert!(!row.standing.is_specified());
        assert!(!row.statement.is_empty());
    }

    #[test]
    fn byte_comparison_is_length_sensitive() {
        assert!(equals(b"line-end", b"line-end"));
        assert!(!equals(b"line-end", b"line-ends"));
        assert!(!equals(b"line-end", b"line-han"));
        assert!(equals(b"", b""));
    }
}
