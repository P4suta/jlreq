// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The rule inventory and the document skeleton: three derivations of the `derive` gate.
//!
//! `crate::derive` is stage 1 of the specification data pipeline and this module is three
//! entries in its registry, reading the ReSpec-rendered snapshot vendored at
//! `spec/snapshot/index.html`:
//!
//! - `spec/derived/anchors.tsv` — the document skeleton: every section the published
//!   rendering numbers, with its anchor id and its heading in both locales.
//! - `spec/derived/rules.tsv` — the rule inventory ADR 0013 addresses: one row per
//!   statement, with its standing, its direction mark, and the sentence quoted in both
//!   locales.
//! - `spec/derived/notes.tsv` — every note of §B.2, §C.2, §D.2 and §E.2, in both locales.
//!
//! Everything else about a derived file — that it is not to be edited, which sources it was
//! read from and their digests, byte identity on a second run — is the frame's, so this
//! module is the reading and nothing else.
//!
//! # Where each fact comes from
//!
//! **The section number is the document's own rendered numbering**, read from the
//! `bdi.secno` element of each heading and never from the anchor slug. ADR 0013 is explicit
//! about this, and `docs/design/generation.md` records why: the appendix legend anchors are
//! off by one from the tables they render — `legend_of_table_2` renders "B.1 Legend of Table
//! 1" — so a tool keyed on the anchor misnumbers a table. The anchor is emitted beside the
//! number as a second column, which is what lets that off-by-one be checked rather than
//! absorbed.
//!
//! **Both locales are read.** The published document is bilingual in one file: every
//! paragraph appears twice, as `its-locale-filter-list="en"` and as
//! `its-locale-filter-list="ja"`. A scan that does not select on that attribute interleaves
//! the two languages, and a cross reference nested inside a paragraph carries both again. So
//! every extracted string is taken from its own locale's span with the other locale's spans
//! removed first, both columns are emitted, and a statement present in one locale and absent
//! in the other is a finding rather than a column silently left empty.
//!
//! **Nothing is skipped in silence.** Every count this module produces is checked against a
//! figure declared below and measured against the vendored snapshot. A revision that
//! renumbers a section, drops a note, or adds an appendix fails the build instead of
//! regenerating quietly, which is the failure mode the whole apparatus exists to prevent.
//!
//! # What is a rule, and what is only a heading
//!
//! `anchors.tsv` holds every numbered section, because a citation to any of them has to
//! resolve. `rules.tsv` holds fewer, and the two criteria are stated here rather than left
//! to whatever the scan happens to do.
//!
//! **Scope.** §3 Line Composition and Appendices B through F. §1.4 states that the document
//! has four parts and says what each is; §3 is the one it describes as line composition, and
//! Appendices B, C, D and E hold the matrices §3 defers to for the amounts, the break
//! opportunities, the reduction and the expansion, with F the jukugo-ruby (熟語ルビ) detail
//! §3.3.7 defers to. `ARCHITECTURE.md` places this library at line composition only, so §2
//! (the kihon-hanmen 基本版面 and page design) and §4 (headings, notes, illustrations and
//! tables) state requirements no layer here answers for — and a rule nothing can answer for
//! could never gain the conformance case ADR 0013 requires of every inventoried rule.
//! Appendix A is the character-class data rather than a rule, which is `appendix-a.tsv`, and
//! G, H, I and J are terminology, references, acknowledgements and the revision log.
//!
//! **A section states a rule when it states something in its own words**: at least one
//! paragraph at the top level of its own body, before its first subsection. A container like
//! §3.1, which is a heading and then immediately §3.1.1, states nothing itself. A section
//! whose whole body is a list — §B.1's legend, §B.2's notes — states nothing in its own words
//! either; its rules are that list's items and, for the legends, the matrix cells the
//! captured stage transcribes.
//!
//! A note ordinal is written `B.2#3`. The `#` is ours (ADR 0013), and the sections it applies
//! to are derived rather than listed: a note-bearing section is one whose heading is exactly
//! "Notes" in English and 注記 in Japanese, which the published document uses for exactly
//! four sections. The ordinal is the item's position in that section's first ordered list.
//! Sections that number their own items in prose instead — §C.3's four strictness levels,
//! §3.8.3's six reduction steps — are cited with a qualifier (`§C.3 level 1`, `§3.8.3 step
//! 6`), which is the spelling the workspace already uses, so they stay one rule each.
//!
//! # The direction mark is a reading, and says so
//!
//! ADR 0011 and `ARCHITECTURE.md`'s fourth invariant state that JLReq conditions exactly
//! three rules on the writing direction — §3.1.3, §3.2.5 and §3.3.5 — and that everything
//! else the specification states twice is exact axis mapping. That is a reading of the
//! document and not a property a scanner can compute: §3.2.2 and §3.2.3 name a writing mode
//! in their own titles and are scoped by it rather than conditioned on it.
//!
//! So the document supplies the evidence and the decision record supplies the reading, and
//! this module holds the two against each other. It derives the *candidates* — every
//! inventoried rule whose own text names a writing mode, in either locale — and refuses to
//! emit unless every marked rule is one of them and the candidate set is the size measured
//! here. A revision that stopped conditioning §3.2.5 on the writing mode would fail the build
//! rather than leave a mark nothing supports.
//!
//! # Hand-rolled, on purpose
//!
//! The scanner is written out here for the reason stated on `purity`'s manifest scan and on
//! `generate`'s: `xtask` declares no dependencies, because it is the program that enforces
//! the layout core declaring none. It understands the shapes this one document is written in
//! and refuses everything else, because a scanner that skipped what it did not recognize
//! would be exactly the silent drop this gate exists to prevent.
//!
//! See `docs/design/generation.md`, `docs/adr/0009`, `docs/adr/0011` and `docs/adr/0013`.

use std::collections::{BTreeMap, BTreeSet};

use crate::derive::Derivation;
use crate::generate::{Emission, Record, Table, Unit};
use crate::shared::{Detail, address};

/// The vendored rendering of the published document.
const SNAPSHOT: &str = "spec/snapshot/index.html";

/// The document skeleton: rendered number, anchor id, heading in both locales.
pub(crate) const ANCHORS: Derivation = Derivation {
    sources: &[SNAPSHOT],
    reader: &["xtask/src/inventory.rs"],
    output: "spec/derived/anchors.tsv",
    caption: "Every section the published rendering numbers, in document order, addressed \
              by that rendering's own number rather than by the anchor slug beside it.",
    read: read_anchors,
};

/// The rule inventory ADR 0013 addresses.
pub(crate) const RULES: Derivation = Derivation {
    sources: &[SNAPSHOT],
    reader: &["xtask/src/inventory.rs"],
    output: "spec/derived/rules.tsv",
    caption: "One row per statement of JLReq this library answers for: every section of §3 \
              and of Appendices B through F that states something in its own words, and \
              every note of §B.2, §C.2, §D.2 and §E.2.",
    read: read_rules,
};

/// Every note of the four appendix `Notes` sections.
pub(crate) const NOTES: Derivation = Derivation {
    sources: &[SNAPSHOT],
    reader: &["xtask/src/inventory.rs"],
    output: "spec/derived/notes.tsv",
    caption: "Every note of the four sections the published document heads Notes / 注記, in \
              both locales, numbered by its position in that section's first ordered list.",
    read: read_notes,
};

/// The path `ANCHORS` writes, for the findings that name it.
const ANCHORS_FILE: &str = "spec/derived/anchors.tsv";

/// The path `RULES` writes, for the findings that name it.
const RULES_FILE: &str = "spec/derived/rules.tsv";

/// The path `NOTES` writes, for the findings that name it.
const NOTES_FILE: &str = "spec/derived/notes.tsv";

// ---------------------------------------------------------------------------------------
// The figures this repository declares, each measured against the vendored snapshot
// ---------------------------------------------------------------------------------------

/// Every heading element the published rendering holds, numbered or not.
const HEADINGS: usize = 185;

/// The headings the rendering gives no section number: the document title, the W3C
/// publication banner, Abstract, Status of This Document, and Table of Contents.
const UNNUMBERED_HEADINGS: usize = 5;

/// The sections the rendering numbers.
const NUMBERED_HEADINGS: usize = 180;

/// The sections whose rendered number the frozen address grammar spells.
const ADDRESSABLE_SECTIONS: usize = 177;

/// The sections whose rendered number the grammar does not spell, with the English heading
/// each carries.
///
/// `docs/design/address-corpus.tsv` fixes the grammar at seven appendices, A through G:
/// `H.1` is in it as a rejected spelling. The published document runs to J, so the three
/// past G are named here with what they are — a reference list, an acknowledgement and a
/// revision log, none of which states a requirement anything could cite. They are recorded
/// rather than dropped, and an appendix appearing past them, or one of these three gaining
/// normative content, fails this derivation.
const UNADDRESSABLE: [(&str, &str); 3] = [
    ("H", "References"),
    ("I", "Acknowledgements"),
    ("J", "Revision Log"),
];

/// The top-level parts of the document the rule inventory covers.
const SCOPE: [&str; 6] = ["3", "B", "C", "D", "E", "F"];

/// The sections in scope that state something in their own words.
const RULE_SECTIONS: usize = 59;

/// Each note-bearing section and how many notes it publishes.
const NOTE_SECTIONS: [(&str, usize); 4] = [("B.2", 17), ("C.2", 13), ("D.2", 5), ("E.2", 12)];

/// Every note of those four sections.
const NOTE_RULES: usize = 47;

/// Every row of the rule inventory.
const RULE_ROWS: usize = 106;

/// The heading that marks a note-bearing section, in each locale.
const NOTES_HEADING: (&str, &str) = ("Notes", "注記");

/// The rules ADR 0011 reads as conditioned on the writing direction.
///
/// A reading and not a derivation; see this module's own documentation. Every one of them
/// must be a rule this scan inventoried and a rule whose text names a writing mode, or
/// nothing is emitted.
const DIRECTION_CONDITIONAL: [&str; 3] = ["3.1.3", "3.2.5", "3.3.5"];

/// Every inventoried rule whose own text names a writing mode in either locale.
///
/// The candidate set is far wider than the marked set, which is the point: ADR 0011's claim
/// is that all but three of these are exact axis mapping rather than a condition.
const DIRECTION_CANDIDATES: usize = 30;

/// The names of the two writing modes in the English text (§2.3).
const DIRECTION_WORDS_EN: [&str; 2] = ["vertical", "horizontal"];

/// The names of the two writing modes, and of the construct that exists in only one of
/// them, in the Japanese text (§2.3, §3.2.5).
const DIRECTION_WORDS_JA: [&str; 5] = ["縦組", "横組", "縦中横", "縦書", "横書"];

/// The standing every row this stage derives carries.
///
/// The rows quote the specification, so each is normative text. The other three standings
/// arrive elsewhere and not from a scanner: an `Alternative` with the policy space, and an
/// `Unstated` or an `Adjudicated` from `docs/decisions/`, which are this project's published
/// readings and are written by people rather than derived.
const STANDING: &str = "Normative";

/// The container elements a paragraph is nested inside rather than stated at the top level
/// of a section.
const CONTAINERS: [&str; 8] = [
    "ol",
    "ul",
    "table",
    "figure",
    "aside",
    "div",
    "dl",
    "blockquote",
];

/// The character entities the published rendering uses, and their text.
///
/// A no-break space is decoded as a space, because a tab-separated field is one line of text
/// and the distinction survives nowhere downstream. Every other entity is refused, so a
/// rendering that gained one is a finding rather than a mangled string.
const ENTITIES: [(&str, &str); 5] = [
    ("&nbsp;", " "),
    ("&lt;", "<"),
    ("&gt;", ">"),
    ("&amp;", "&"),
    ("&quot;", "\""),
];

/// How much of an undecodable entity a finding quotes.
const ENTITY_SAMPLE: usize = 8;

/// Which locale a string was read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Locale {
    /// `its-locale-filter-list="en"`.
    English,
    /// `its-locale-filter-list="ja"`.
    Japanese,
}

impl Locale {
    /// Both of them, in the order the document writes them.
    const BOTH: [Self; 2] = [Self::English, Self::Japanese];

    /// The value the published rendering writes in both of a span's attributes.
    const fn tag(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Japanese => "ja",
        }
    }

    /// The other one.
    const fn other(self) -> Self {
        match self {
            Self::English => Self::Japanese,
            Self::Japanese => Self::English,
        }
    }
}

/// One tag of a fragment, as the scanner reads it.
#[derive(Debug, Clone, Copy)]
struct Tag<'a> {
    /// The element name, as the document writes it.
    name: &'a str,
    /// Everything after the name and before the closing angle bracket.
    attributes: &'a str,
    /// Whether this is a closing tag.
    closing: bool,
    /// Where the tag opens.
    start: usize,
    /// Where the tag ends, which is where its content begins.
    end: usize,
}

/// One heading element of the published rendering.
#[derive(Debug, Clone, Copy)]
struct Element<'a> {
    /// The heading's level, `1` through `6`.
    level: char,
    /// The opening tag's attributes.
    attributes: &'a str,
    /// Everything between the opening and closing tags.
    content: &'a str,
    /// Where the element opens, which is where the section before it stops.
    start: usize,
    /// Where the element closes, which is where its own body begins.
    end: usize,
}

/// One numbered section of the published document, with the body it opens.
#[derive(Debug)]
struct Heading {
    /// The rendered section number without its trailing period: `3.1.9`, `A`, `H`.
    rendered: String,
    /// The canonical address, when the frozen grammar spells the rendered number.
    address: Option<String>,
    /// The `id` the rendering gives the heading, which is what a URL fragment names.
    anchor: String,
    /// The English heading.
    title_en: String,
    /// The Japanese heading.
    title_ja: String,
    /// The section's own body: everything after this heading and before the next one.
    body: (usize, usize),
}

/// One paragraph of a section's body.
#[derive(Debug)]
struct Paragraph {
    /// How many container elements it sits inside. Zero is the section's own prose.
    depth: usize,
    /// The locale its attributes declare, if either.
    locale: Option<Locale>,
    /// The paragraph as one line of text.
    text: String,
}

/// One section that states something in its own words.
#[derive(Debug)]
struct Statement {
    /// The canonical address.
    address: String,
    /// The English heading, which is what the generated constant is named from.
    title_en: String,
    /// The first paragraph of the section's own prose, in English.
    lead_en: String,
    /// The same paragraph in Japanese.
    lead_ja: String,
    /// Whether any of the section's own text names a writing mode.
    names_a_direction: bool,
}

/// One row of the rule inventory.
#[derive(Debug)]
struct Rule {
    /// The canonical address.
    address: String,
    /// The name the generated constant for this rule carries.
    name: String,
    /// Whether evaluating it consults the writing direction.
    direction_conditional: bool,
    /// The sentence, quoted from the English text.
    statement_en: String,
    /// The sentence, quoted from the Japanese text.
    statement_ja: String,
}

/// One note of one of the four appendix `Notes` sections.
#[derive(Debug)]
struct Note {
    /// The canonical address: `B.2#3`.
    address: String,
    /// The section the note belongs to.
    section: String,
    /// Its position in that section's list, from one.
    ordinal: usize,
    /// The note, in English.
    text_en: String,
    /// The note, in Japanese.
    text_ja: String,
}

/// The published document, read once: its skeleton, its notes and its rules.
#[derive(Debug)]
struct Document {
    /// Every numbered section, in document order.
    headings: Vec<Heading>,
    /// Every note of the four `Notes` sections, in document order.
    notes: Vec<Note>,
    /// The rule inventory, in the specification's own reading order.
    rules: Vec<Rule>,
}

// ---------------------------------------------------------------------------------------
// The three readers
// ---------------------------------------------------------------------------------------

/// Read the document skeleton out of the published rendering.
fn read_anchors(sources: &[String]) -> Result<String, String> {
    let document = read_document(only(sources)?)?;
    let mut out = String::from("address\tanchor\ttitle_en\ttitle_ja\n");
    out.push_str(&explanation(&[
        "Three numbered sections are absent, deliberately. The address grammar",
        "docs/design/address-corpus.tsv fixes runs to seven appendices, A through G, while",
        "the published document runs to J; H References, I Acknowledgements and J Revision",
        "Log state no requirement anything could cite. An appendix past them, or normative",
        "content appearing in one of those three, fails this derivation.",
    ]));
    for heading in &document.headings {
        let Some(address) = heading.address.as_deref() else {
            continue;
        };
        out.push_str(&row(&[
            address,
            &heading.anchor,
            &heading.title_en,
            &heading.title_ja,
        ])?);
    }
    Ok(out)
}

/// Read the rule inventory out of the published rendering.
fn read_rules(sources: &[String]) -> Result<String, String> {
    let document = read_document(only(sources)?)?;
    let mut out = String::from(
        "address\tname\tstanding\tdirection_conditional\tstatement_en\tstatement_ja\n",
    );
    out.push_str(&explanation(&[
        "`standing` is Normative throughout, because every row quotes the specification. An",
        "Alternative arrives with the policy space, and an Unstated or an Adjudicated from",
        "docs/decisions/: those are this project's published readings, written rather than",
        "derived (docs/adr/0009).",
        "",
        "`direction_conditional` is ADR 0011's reading and not a property of the text. The",
        "three rules marked here are the three that decision fixes, and this derivation",
        "refuses to emit unless each one's own text still names a writing mode. Many more",
        "rules name one without being conditioned on it.",
    ]));
    for rule in &document.rules {
        out.push_str(&row(&[
            &rule.address,
            &rule.name,
            STANDING,
            if rule.direction_conditional {
                "true"
            } else {
                "false"
            },
            &rule.statement_en,
            &rule.statement_ja,
        ])?);
    }
    Ok(out)
}

/// Read the appendix notes out of the published rendering.
fn read_notes(sources: &[String]) -> Result<String, String> {
    let document = read_document(only(sources)?)?;
    let mut out = String::from("address\tsection\tordinal\ttext_en\ttext_ja\n");
    out.push_str(&explanation(&[
        "The ordinal counts only the items of the section's own first ordered list: a nested",
        "list inside one item is not four more notes. The `#` separating an ordinal from its",
        "section is this project's, because JLReq writes \"note 7\" in prose and gives its",
        "list items opaque ids (docs/adr/0013).",
    ]));
    for note in &document.notes {
        out.push_str(&row(&[
            &note.address,
            &note.section,
            &note.ordinal.to_string(),
            &note.text_en,
            &note.text_ja,
        ])?);
    }
    Ok(out)
}

/// Every in-scope section's own prose and every appendix note, in both locales.
///
/// Read by `crate::policy`, whose derivation is a reading of which sections state two
/// answers rather than one, and which holds every one of those readings against the
/// sentence it names (ADR 0009). The value is `[English, Japanese]`, in the order
/// [`Locale::BOTH`] declares.
///
/// The text is the *whole* of a section rather than the lead paragraph `rules.tsv` quotes,
/// because the sentence that permits an alternative is as often in a list item or a note as
/// in the opening paragraph: §C.3 states its four levels as list items, §3.3.5 states the
/// two ruby alignments three paragraphs in, and §3.9.2 concedes the ambiguous case in a note
/// nested four containers deep. A check against the lead alone would refuse most of the
/// policy space and would tempt the reading to be trimmed to fit the scan.
///
/// Keyed by the canonical address, so a question naming a section or a note the published
/// document does not have has nothing to be held against and fails the derivation that
/// names it.
pub(crate) fn prose(html: &str) -> Result<BTreeMap<String, [String; 2]>, String> {
    let headings = headings(html)?;
    let mut found = BTreeMap::new();
    for heading in &headings {
        let Some(address) = heading.address.as_deref().filter(|it| in_scope(it)) else {
            continue;
        };
        let (start, end) = heading.body;
        let read = paragraphs(html.get(start..end).unwrap_or(""))?;
        let joined = |locale: Locale| -> String {
            read.iter()
                .filter(|paragraph| paragraph.locale == Some(locale))
                .map(|paragraph| paragraph.text.as_str())
                .collect::<Vec<&str>>()
                .join(" ")
        };
        found.insert(
            address.to_owned(),
            [joined(Locale::English), joined(Locale::Japanese)],
        );
    }
    let mut violations = Vec::new();
    for note in read_notes_of(html, &headings, &mut violations) {
        found.insert(note.address, [note.text_en, note.text_ja]);
    }
    if !violations.is_empty() {
        return Err(violations.join("\n  "));
    }
    Ok(found)
}

/// The one source these derivations read.
fn only(sources: &[String]) -> Result<&str, String> {
    match sources {
        [one] => Ok(one.as_str()),
        _ => Err(format!(
            "this derivation reads {SNAPSHOT} and nothing else; it was handed {count} \
             source(s)",
            count = sources.len()
        )),
    }
}

/// A comment block, written under the column line rather than above it.
///
/// Every reader of a derived table skips a comment wherever it appears, and one of them —
/// `conform`, reading the rule inventory — takes the first line it does not skip as the
/// header and every line after that as a row. A comment block above the column line would
/// therefore hand it the column line itself as a rule address.
fn explanation(lines: &[&str]) -> String {
    let mut text = String::new();
    for line in lines {
        text.push('#');
        if !line.is_empty() {
            text.push(' ');
            text.push_str(line);
        }
        text.push('\n');
    }
    text
}

/// One row: the fields, tab separated.
///
/// A field holding a tab or a newline would silently become two rows or two fields, so it
/// is refused. Nothing this module extracts can hold either — every string is collapsed to
/// one line first — which is why this is a check rather than an escape.
fn row(fields: &[&str]) -> Result<String, String> {
    if let Some(broken) = fields
        .iter()
        .find(|field| field.contains('\t') || field.contains('\n'))
    {
        return Err(format!(
            "`{broken}` holds a tab or a newline, and a tab-separated field is one line"
        ));
    }
    let mut line = fields.join("\t");
    line.push('\n');
    Ok(line)
}

/// Read the whole document once, or report everything that stopped the reading.
///
/// One `Err` carries every finding rather than the first, so a revision that moved two
/// things is one report rather than two runs.
fn read_document(html: &str) -> Result<Document, String> {
    let headings = headings(html)?;
    let mut violations = Vec::new();
    check_headings(&headings, &mut violations);

    let sections = read_sections(html, &headings, &mut violations);
    let notes = read_notes_of(html, &headings, &mut violations);
    let rules = build_rules(&sections, &notes, &mut violations);
    if !violations.is_empty() {
        return Err(violations.join("\n  "));
    }
    Ok(Document {
        headings,
        notes,
        rules,
    })
}

// ---------------------------------------------------------------------------------------
// The scanner
// ---------------------------------------------------------------------------------------

/// Every tag of a fragment, in document order.
fn tags(fragment: &str) -> Vec<Tag<'_>> {
    let mut found = Vec::new();
    let mut cursor = 0usize;
    while let Some(rest) = fragment.get(cursor..) {
        let Some(at) = rest.find('<') else { break };
        let start = cursor.saturating_add(at);
        let Some(head) = fragment.get(start..) else {
            break;
        };
        let Some(length) = head.find('>') else { break };
        let inside = head.get(1..length).unwrap_or("");
        let closing = inside.starts_with('/');
        let body = inside.strip_prefix('/').unwrap_or(inside);
        let split = body
            .find(|character: char| character.is_whitespace())
            .unwrap_or(body.len());
        cursor = start.saturating_add(length).saturating_add(1);
        found.push(Tag {
            name: body.get(..split).unwrap_or(""),
            attributes: body.get(split..).unwrap_or(""),
            closing,
            start,
            end: cursor,
        });
    }
    found
}

/// Every heading element of the document, in document order.
///
/// A heading is read as an element rather than by searching for `class="secno"`, because
/// every cross reference in the body carries a rendered number of its own and a scan keyed
/// on the class would read those as headings.
fn heading_elements(html: &str) -> Result<Vec<Element<'_>>, String> {
    let mut found = Vec::new();
    let mut cursor = 0usize;
    while let Some(rest) = html.get(cursor..) {
        let Some(at) = rest.find("<h") else { break };
        let start = cursor.saturating_add(at);
        let after = start.saturating_add("<h".len());
        let Some(level) = html.get(after..).and_then(|tail| tail.chars().next()) else {
            break;
        };
        let Some(follows) = html
            .get(after.saturating_add(level.len_utf8())..)
            .and_then(|tail| tail.chars().next())
        else {
            break;
        };
        if !level.is_ascii_digit() || !(follows == '>' || follows.is_whitespace()) {
            cursor = after;
            continue;
        }
        let head = html.get(start..).unwrap_or("");
        let opened = head
            .find('>')
            .ok_or_else(|| format!("{SNAPSHOT}: an unclosed h{level} opening tag"))?;
        let content_start = start.saturating_add(opened).saturating_add(1);
        let closing = format!("</h{level}>");
        let length = html
            .get(content_start..)
            .and_then(|tail| tail.find(&closing))
            .ok_or_else(|| format!("{SNAPSHOT}: an unclosed h{level} element"))?;
        let content_end = content_start.saturating_add(length);
        cursor = content_end.saturating_add(closing.len());
        found.push(Element {
            level,
            attributes: head.get("<h".len().saturating_add(1)..opened).unwrap_or(""),
            content: html.get(content_start..content_end).unwrap_or(""),
            start,
            end: cursor,
        });
    }
    Ok(found)
}

/// Every numbered section, with the body it opens.
///
/// A heading whose content opens with the rendered number is a numbered section; one that
/// does not is front matter, and is counted rather than passed over.
fn headings(html: &str) -> Result<Vec<Heading>, String> {
    let elements = heading_elements(html)?;
    let mut found = Vec::new();
    let mut unnumbered = 0usize;
    for (index, element) in elements.iter().enumerate() {
        let marker = "<bdi class=\"secno\">";
        if !element.content.starts_with(marker) {
            if element.content.contains(marker) {
                return Err(format!(
                    "{SNAPSHOT}: an h{level} states a rendered number that does not open it",
                    level = element.level
                ));
            }
            unnumbered = unnumbered.saturating_add(1);
            continue;
        }
        let number = between(element.content, marker, "</bdi>").ok_or_else(|| {
            format!(
                "{SNAPSHOT}: an h{level} opens a rendered number and does not close it",
                level = element.level
            )
        })?;
        let anchor = attribute(element.attributes, "id").ok_or_else(|| {
            format!("{SNAPSHOT}: the heading numbered `{number}` carries no `id`")
        })?;
        let rendered = number.trim().trim_end_matches('.').to_owned();
        let end = elements
            .get(index.saturating_add(1))
            .map_or(html.len(), |next| next.start);
        found.push(Heading {
            address: canonical(&rendered),
            rendered,
            anchor: anchor.to_owned(),
            title_en: locale_text(element.content, Locale::English)?,
            title_ja: locale_text(element.content, Locale::Japanese)?,
            body: (element.end, end),
        });
    }
    if unnumbered != UNNUMBERED_HEADINGS {
        return Err(format!(
            "{SNAPSHOT}: {unnumbered} heading(s) carry no rendered section number, and this \
             repository declares {UNNUMBERED_HEADINGS}"
        ));
    }
    Ok(found)
}

/// The value of one attribute of an opening tag, or `None` when it carries none.
fn attribute<'a>(attributes: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!("{name}=\"");
    let at = attributes.find(&needle)?;
    let rest = attributes.get(at.saturating_add(needle.len())..)?;
    let end = rest.find('"')?;
    rest.get(..end)
}

/// The text between two markers, or `None` when either is absent.
fn between<'a>(text: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let at = text.find(open)?;
    let rest = text.get(at.saturating_add(open.len())..)?;
    let end = rest.find(close)?;
    rest.get(..end)
}

/// The opening tag of one locale's span.
fn span_of(locale: Locale) -> String {
    format!(
        "<span its-locale-filter-list=\"{tag}\" lang=\"{tag}\">",
        tag = locale.tag()
    )
}

/// The text of one locale's span inside a fragment that carries both.
///
/// Exactly one span per locale is expected. A fragment carrying two of one locale would make
/// "the English heading" ambiguous, so it is refused rather than resolved by position.
fn locale_text(fragment: &str, locale: Locale) -> Result<String, String> {
    let open = span_of(locale);
    let Some(at) = fragment.find(&open) else {
        return Err(format!(
            "{SNAPSHOT}: a heading states no `{tag}` span; the published document is \
             bilingual in one file and every heading carries both",
            tag = locale.tag()
        ));
    };
    let rest = fragment.get(at.saturating_add(open.len())..).unwrap_or("");
    if rest.contains(&open) {
        return Err(format!(
            "{SNAPSHOT}: a heading states two `{tag}` spans, so which one is the heading is \
             not decidable",
            tag = locale.tag()
        ));
    }
    let end = rest
        .find("</span>")
        .ok_or_else(|| format!("{SNAPSHOT}: an unclosed `{tag}` span", tag = locale.tag()))?;
    plain(rest.get(..end).unwrap_or(""))
}

/// A fragment as text: tags removed, the five published entities decoded, every run of
/// whitespace collapsed to one space.
///
/// An entity the document does not use is refused rather than passed through, because a
/// half-decoded string in a published table is worse than a failed build.
fn plain(markup: &str) -> Result<String, String> {
    let mut text = String::with_capacity(markup.len());
    let mut rest = markup;
    while !rest.is_empty() {
        let Some(at) = rest.find(['<', '&']) else {
            text.push_str(rest);
            break;
        };
        let (before, from) = rest.split_at(at);
        text.push_str(before);
        rest = if from.starts_with('<') {
            let end = from
                .find('>')
                .ok_or_else(|| format!("{SNAPSHOT}: an unclosed tag"))?;
            from.get(end.saturating_add(1)..).unwrap_or("")
        } else {
            let (entity, decoded) = ENTITIES
                .iter()
                .find(|(entity, _)| from.starts_with(entity))
                .ok_or_else(|| {
                    format!(
                        "{SNAPSHOT}: `{sample}` is an entity this scanner does not decode",
                        sample = from.get(..ENTITY_SAMPLE).unwrap_or(from)
                    )
                })?;
            text.push_str(decoded);
            from.get(entity.len()..).unwrap_or("")
        };
    }
    Ok(collapse(&text))
}

/// Every run of whitespace as one space, with the ends trimmed.
fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<&str>>().join(" ")
}

/// A fragment with the other locale's spans removed, so that a paragraph reads in one
/// language.
///
/// The nested case is the one that matters: a cross reference inside an English paragraph
/// carries the referenced section's heading in both languages, so an English statement that
/// did not do this would end in Japanese.
fn one_locale(fragment: &str, locale: Locale) -> Result<String, String> {
    let open = span_of(locale.other());
    let mut kept = String::with_capacity(fragment.len());
    let mut rest = fragment;
    while let Some(at) = rest.find(&open) {
        kept.push_str(rest.get(..at).unwrap_or(""));
        let inside = rest.get(at.saturating_add(open.len())..).unwrap_or("");
        let end = span_end(inside).ok_or_else(|| {
            format!(
                "{SNAPSHOT}: a `{tag}` span is never closed",
                tag = locale.other().tag()
            )
        })?;
        rest = inside.get(end..).unwrap_or("");
    }
    kept.push_str(rest);
    Ok(kept)
}

/// Where the span opened just before `text` closes, counting nested spans.
fn span_end(text: &str) -> Option<usize> {
    let mut depth = 1usize;
    let mut cursor = 0usize;
    loop {
        let rest = text.get(cursor..)?;
        let close = rest.find("</span>")?;
        match rest.find("<span") {
            Some(open) if open < close => {
                cursor = cursor.saturating_add(open).saturating_add("<span".len());
                depth = depth.saturating_add(1);
            },
            _ => {
                cursor = cursor.saturating_add(close).saturating_add("</span>".len());
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(cursor);
                }
            },
        }
    }
}

/// Every paragraph of a body, with the nesting depth each sits at.
///
/// Depth is counted over the container elements a paragraph can be nested inside, so a
/// section's own prose is the paragraphs at depth zero and a list item's or a note's
/// paragraphs are deeper. That distinction is what separates a section stating something in
/// its own words from one whose whole body is a list.
fn paragraphs(body: &str) -> Result<Vec<Paragraph>, String> {
    let mut found = Vec::new();
    let mut depth = 0usize;
    for tag in tags(body) {
        if CONTAINERS.contains(&tag.name) {
            depth = if tag.closing {
                depth.saturating_sub(1)
            } else {
                depth.saturating_add(1)
            };
            continue;
        }
        if tag.name != "p" || tag.closing {
            continue;
        }
        let locale = locale_of(tag.attributes);
        let rest = body.get(tag.end..).unwrap_or("");
        let end = rest
            .find("</p>")
            .ok_or_else(|| format!("{SNAPSHOT}: an unclosed paragraph"))?;
        let inner = rest.get(..end).unwrap_or("");
        let text = match locale {
            Some(locale) => plain(&one_locale(inner, locale)?)?,
            None => plain(inner)?,
        };
        found.push(Paragraph {
            depth,
            locale,
            text,
        });
    }
    Ok(found)
}

/// Which locale a paragraph's attributes declare, if either.
fn locale_of(attributes: &str) -> Option<Locale> {
    Locale::BOTH.into_iter().find(|locale| {
        attributes.contains(&format!(
            "its-locale-filter-list=\"{tag}\"",
            tag = locale.tag()
        ))
    })
}

/// The direct items of the first ordered list in a fragment.
///
/// Nesting is counted, because the published lists contain lists: §B.1's third legend item
/// opens a list of the notations a cell may hold, and a count that read those as items of
/// the list above would number everything after them wrongly.
fn items(body: &str) -> Result<Vec<&str>, String> {
    let mut found = Vec::new();
    let mut depth = 0usize;
    let mut list: Option<usize> = None;
    let mut opened: Option<usize> = None;
    for tag in tags(body) {
        if !matches!(tag.name, "ol" | "ul" | "table" | "li") {
            continue;
        }
        if tag.name == "li" {
            if !tag.closing && list == Some(depth) {
                push_item(body, &mut opened, tag.start, &mut found);
                opened = Some(tag.end);
            }
            continue;
        }
        if tag.closing {
            if list == Some(depth) && tag.name == "ol" {
                push_item(body, &mut opened, tag.start, &mut found);
                return Ok(found);
            }
            depth = depth.saturating_sub(1);
            continue;
        }
        depth = depth.saturating_add(1);
        if list.is_none() && tag.name == "ol" {
            list = Some(depth);
        }
    }
    if list.is_some() {
        return Err(format!("{SNAPSHOT}: an unclosed ordered list"));
    }
    Ok(found)
}

/// Close the item that was open, if one was.
fn push_item<'a>(body: &'a str, opened: &mut Option<usize>, at: usize, found: &mut Vec<&'a str>) {
    if let Some(start) = opened.take() {
        found.push(body.get(start..at.max(start)).unwrap_or(""));
    }
}

// ---------------------------------------------------------------------------------------
// The three tables
// ---------------------------------------------------------------------------------------

/// Every in-scope section that states a rule, and every finding along the way.
fn read_sections(html: &str, headings: &[Heading], violations: &mut Vec<String>) -> Vec<Statement> {
    let mut found = Vec::new();
    for heading in headings {
        let Some(address) = heading.address.as_deref().filter(|it| in_scope(it)) else {
            continue;
        };
        let (start, end) = heading.body;
        let body = html.get(start..end).unwrap_or("");
        match paragraphs(body) {
            Err(reason) => violations.push(format!("§{address}: {reason}")),
            Ok(read) => {
                if let Some(statement) = statement(address, heading, &read, violations) {
                    found.push(statement);
                }
            },
        }
    }
    if found.len() != RULE_SECTIONS {
        violations.push(format!(
            "{RULES_FILE}: {count} section(s) in scope state a rule, and this repository \
             declares {RULE_SECTIONS}",
            count = found.len()
        ));
    }
    found
}

/// One section's statement, or nothing when the section states nothing in its own words.
fn statement(
    address: &str,
    heading: &Heading,
    paragraphs: &[Paragraph],
    violations: &mut Vec<String>,
) -> Option<Statement> {
    let own: Vec<&Paragraph> = paragraphs
        .iter()
        .filter(|paragraph| paragraph.depth == 0)
        .collect();
    if own.is_empty() {
        return None;
    }
    if own.iter().any(|paragraph| paragraph.locale.is_none()) {
        violations.push(format!(
            "§{address}: states a paragraph of its own that declares no locale; every \
             paragraph of the published body carries `its-locale-filter-list`, and one that \
             does not is the divergence docs/design/generation.md refuses to absorb"
        ));
        return None;
    }
    let lead = |locale: Locale| -> Option<&str> {
        own.iter()
            .find(|paragraph| paragraph.locale == Some(locale))
            .map(|paragraph| paragraph.text.as_str())
    };
    let counted = |locale: Locale| -> usize {
        own.iter()
            .filter(|paragraph| paragraph.locale == Some(locale))
            .count()
    };
    if counted(Locale::English) != counted(Locale::Japanese) {
        violations.push(format!(
            "§{address}: states {en} paragraph(s) of its own in English and {ja} in \
             Japanese; the two locales are not in correspondence and no defect records it \
             (docs/design/generation.md)",
            en = counted(Locale::English),
            ja = counted(Locale::Japanese),
        ));
        return None;
    }
    let (Some(lead_en), Some(lead_ja)) = (lead(Locale::English), lead(Locale::Japanese)) else {
        return None;
    };
    Some(Statement {
        address: address.to_owned(),
        title_en: heading.title_en.clone(),
        lead_en: lead_en.to_owned(),
        lead_ja: lead_ja.to_owned(),
        names_a_direction: paragraphs
            .iter()
            .any(|paragraph| names_a_direction(paragraph.locale, &paragraph.text)),
    })
}

/// Whether a string names one of the two writing modes.
fn names_a_direction(locale: Option<Locale>, text: &str) -> bool {
    match locale {
        Some(Locale::English) => {
            let lowered = text.to_ascii_lowercase();
            DIRECTION_WORDS_EN.iter().any(|word| lowered.contains(word))
        },
        Some(Locale::Japanese) => DIRECTION_WORDS_JA.iter().any(|word| text.contains(word)),
        None => false,
    }
}

/// Whether an address belongs to the part of the document the rule inventory covers.
fn in_scope(address: &str) -> bool {
    SCOPE.contains(&address.split('.').next().unwrap_or(address))
}

/// Every note of the four `Notes` sections, and every finding along the way.
fn read_notes_of(html: &str, headings: &[Heading], violations: &mut Vec<String>) -> Vec<Note> {
    let mut found = Vec::new();
    let mut sections = Vec::new();
    for heading in headings {
        let Some(address) = heading.address.as_deref() else {
            continue;
        };
        if (heading.title_en.as_str(), heading.title_ja.as_str()) != NOTES_HEADING {
            continue;
        }
        sections.push(address.to_owned());
        let (start, end) = heading.body;
        match notes_of(address, html.get(start..end).unwrap_or("")) {
            Ok(notes) => found.extend(notes),
            Err(reason) => violations.push(reason),
        }
    }
    check_notes(&sections, &found, violations);
    found
}

/// Hold what was read against the four sections and the counts this repository declares.
fn check_notes(sections: &[String], found: &[Note], violations: &mut Vec<String>) {
    let declared: Vec<&str> = NOTE_SECTIONS.iter().map(|(name, _)| *name).collect();
    let read: Vec<&str> = sections.iter().map(String::as_str).collect();
    if read != declared {
        violations.push(format!(
            "{NOTES_FILE}: the sections headed `{en}` / `{ja}` are {read:?}, and this \
             repository declares {declared:?}",
            en = NOTES_HEADING.0,
            ja = NOTES_HEADING.1,
        ));
    }
    for (section, expected) in NOTE_SECTIONS {
        let count = found.iter().filter(|note| note.section == section).count();
        if count != expected {
            violations.push(format!(
                "{NOTES_FILE}: §{section} publishes {count} note(s), and this repository \
                 declares {expected}"
            ));
        }
    }
    if found.len() != NOTE_RULES {
        violations.push(format!(
            "{NOTES_FILE}: {count} note(s) in all, and this repository declares {NOTE_RULES}",
            count = found.len()
        ));
    }
}

/// The notes of one `Notes` section: the items of the first ordered list in its body.
fn notes_of(section: &str, body: &str) -> Result<Vec<Note>, String> {
    let mut notes = Vec::new();
    for (index, item) in items(body)?.into_iter().enumerate() {
        let ordinal = index.saturating_add(1);
        let address = format!("{section}#{ordinal}");
        let paragraphs = paragraphs(item)?;
        let joined = |locale: Locale| -> String {
            paragraphs
                .iter()
                .filter(|paragraph| paragraph.locale == Some(locale))
                .map(|paragraph| paragraph.text.as_str())
                .collect::<Vec<&str>>()
                .join(" ")
        };
        let text_en = joined(Locale::English);
        let text_ja = joined(Locale::Japanese);
        if text_en.is_empty() || text_ja.is_empty() {
            return Err(format!(
                "§{address}: is published in one locale only; the extractor emits both \
                 columns and fails on a divergence no defect records \
                 (docs/design/generation.md)"
            ));
        }
        notes.push(Note {
            address,
            section: section.to_owned(),
            ordinal,
            text_en,
            text_ja,
        });
    }
    Ok(notes)
}

/// The rule inventory: the sections that state a rule, then the notes, in reading order.
fn build_rules(sections: &[Statement], notes: &[Note], violations: &mut Vec<String>) -> Vec<Rule> {
    let mut rules = Vec::new();
    let mut candidates = 0usize;
    for section in sections {
        if section.names_a_direction {
            candidates = candidates.saturating_add(1);
        }
        rules.push(Rule {
            address: section.address.clone(),
            name: identifier(&section.title_en, violations),
            direction_conditional: DIRECTION_CONDITIONAL.contains(&section.address.as_str()),
            statement_en: section.lead_en.clone(),
            statement_ja: section.lead_ja.clone(),
        });
    }
    for note in notes {
        if names_a_direction(Some(Locale::English), &note.text_en)
            || names_a_direction(Some(Locale::Japanese), &note.text_ja)
        {
            candidates = candidates.saturating_add(1);
        }
        rules.push(Rule {
            address: note.address.clone(),
            name: format!(
                "{section}_NOTE_{ordinal}",
                section = note.section.replace('.', "_"),
                ordinal = note.ordinal
            ),
            direction_conditional: false,
            statement_en: note.text_en.clone(),
            statement_ja: note.text_ja.clone(),
        });
    }
    check_rules(&rules, candidates, sections, violations);
    rules
}

/// Hold the inventory against the counts, the names and the direction reading.
fn check_rules(
    rules: &[Rule],
    candidates: usize,
    sections: &[Statement],
    violations: &mut Vec<String>,
) {
    if rules.len() != RULE_ROWS {
        violations.push(format!(
            "{RULES_FILE}: {count} rule(s), and this repository declares {RULE_ROWS}",
            count = rules.len()
        ));
    }
    if candidates != DIRECTION_CANDIDATES {
        violations.push(format!(
            "{RULES_FILE}: {candidates} rule(s) name a writing mode, and this repository \
             declares {DIRECTION_CANDIDATES} (ADR 0011)"
        ));
    }
    let named: BTreeSet<&str> = rules.iter().map(|rule| rule.name.as_str()).collect();
    if named.len() != rules.len() {
        violations.push(format!(
            "{RULES_FILE}: {count} distinct constant name(s) over {rows} rule(s); a rule is \
             cited in code by name, so two rules may not share one (ADR 0019)",
            count = named.len(),
            rows = rules.len(),
        ));
    }
    let addressed: BTreeMap<&str, bool> = rules
        .iter()
        .map(|rule| (rule.address.as_str(), rule.direction_conditional))
        .collect();
    if addressed.len() != rules.len() {
        violations.push(format!(
            "{RULES_FILE}: {count} distinct address(es) over {rows} rule(s); one rule has \
             one row",
            count = addressed.len(),
            rows = rules.len(),
        ));
    }
    for marked in DIRECTION_CONDITIONAL {
        if addressed.get(marked) != Some(&true) {
            violations.push(format!(
                "{RULES_FILE}: §{marked} is read as direction-conditional and the inventory \
                 does not mark it; ADR 0011 fixes the set at {DIRECTION_CONDITIONAL:?}"
            ));
        }
        if !sections
            .iter()
            .any(|section| section.address == marked && section.names_a_direction)
        {
            violations.push(format!(
                "{RULES_FILE}: §{marked} is marked direction-conditional and nothing in its \
                 own text names a writing mode; the document supplies the evidence and ADR \
                 0011 supplies the reading, and here the two disagree"
            ));
        }
    }
}

/// The name a rule's generated constant carries, derived from the section's English heading.
///
/// The heading is the document's own name for what the section states, so the constant a
/// reader writes in code is the specification's word rather than one invented here. A
/// heading that will not spell a Rust identifier is a finding, because a generated file that
/// does not compile is worse than one that was never written.
fn identifier(title: &str, violations: &mut Vec<String>) -> String {
    let mut name = String::with_capacity(title.len());
    for character in title.chars() {
        if character.is_ascii_alphanumeric() {
            name.push(character.to_ascii_uppercase());
        } else if !name.ends_with('_') {
            name.push('_');
        }
    }
    let name = name.trim_matches('_').to_owned();
    if name.is_empty() || name.starts_with(|first: char| first.is_ascii_digit()) {
        violations.push(format!(
            "{RULES_FILE}: the heading `{title}` does not spell a Rust identifier"
        ));
    }
    name
}

/// The canonical rendering of a rendered section number, when the grammar spells it.
///
/// The grammar is `crate::shared`'s, the workspace's one carrier for it: a number this
/// rejects is one no citation, no case file and no generated table could ever hold.
fn canonical(rendered: &str) -> Option<String> {
    address(rendered)
        .map(|parsed| parsed.to_string())
        .filter(|parsed| parsed == rendered)
}

/// Hold the heading scan against the counts this repository declares.
fn check_headings(headings: &[Heading], violations: &mut Vec<String>) {
    let numbered = headings.len();
    if numbered != NUMBERED_HEADINGS {
        violations.push(format!(
            "{SNAPSHOT}: {numbered} numbered section(s), and this repository declares \
             {NUMBERED_HEADINGS}"
        ));
    }
    if numbered.saturating_add(UNNUMBERED_HEADINGS) != HEADINGS {
        violations.push(format!(
            "{SNAPSHOT}: {HEADINGS} heading(s) are declared and {numbered} numbered plus \
             {UNNUMBERED_HEADINGS} unnumbered were read"
        ));
    }
    let refused: Vec<(&str, &str)> = headings
        .iter()
        .filter(|heading| heading.address.is_none())
        .map(|heading| (heading.rendered.as_str(), heading.title_en.as_str()))
        .collect();
    if refused != UNADDRESSABLE {
        violations.push(format!(
            "{ANCHORS_FILE}: the address grammar does not spell {refused:?}, and this \
             repository declares {UNADDRESSABLE:?}; see docs/design/address-corpus.tsv"
        ));
    }
    let addressable = numbered.saturating_sub(refused.len());
    if addressable != ADDRESSABLE_SECTIONS {
        violations.push(format!(
            "{ANCHORS_FILE}: {addressable} addressable section(s), and this repository \
             declares {ADDRESSABLE_SECTIONS}"
        ));
    }
}

// ---------------------------------------------------------------------------------------
// Stage 2: the generation unit
// ---------------------------------------------------------------------------------------

/// The rule inventory, as `jlreq-spec` indexes it.
pub(crate) const RULE_INVENTORY: Unit = Unit {
    input: RULES_FILE,
    generator: &["xtask/src/inventory.rs"],
    output: "crates/jlreq-spec/src/generated/inventory.rs",
    summary: "The rule inventory: one row and one named identifier per statement of JLReq.",
    emit: emit_inventory,
};

/// The deepest section path an address holds, as `jlreq_spec::rule::MAX_PARTS` writes it.
///
/// The emitter writes an address as its components, so the padding it writes has to be the
/// width the type declares. The two are held equal by the compile-time canonicality
/// assertion over `RULES`: a path this padded to the wrong width would leave a component
/// behind the depth, which `Parts::is_canonical` refuses.
const ADDRESS_PARTS: usize = 4;

/// The standing every derived row carries, as the Rust enum spells it.
const NORMATIVE: &str = "Normative";

/// The column `rustfmt.toml` sets as the width of a line of this workspace's Rust.
const MAX_WIDTH: usize = 100;

/// One row of the inventory, read out of the derived table.
#[derive(Debug)]
struct Row {
    /// The canonical address, for the doc comment and for the messages.
    rendered: String,
    /// The appendix letter, when the path is a lettered one.
    appendix: Option<char>,
    /// The numbered components, padded to `ADDRESS_PARTS`.
    values: [u8; ADDRESS_PARTS],
    /// How many of `values` are components.
    depth: u8,
    /// The note ordinal, or zero when the address names a whole section.
    note: u8,
    /// The identifier the emitted constant is named by.
    name: String,
    /// Whether evaluating the rule consults the writing direction.
    direction_conditional: bool,
    /// The sentence, quoted from the published document.
    statement: String,
}

/// Emit the rule inventory.
fn emit_inventory(table: &Table) -> Result<Emission, String> {
    let rows = read_inventory(table)?;
    let mut items = String::new();
    items.push_str(&inventory_rows(&rows));
    items.push_str(&inventory_identifiers(&rows));
    Ok(Emission {
        entries: rows.len(),
        items,
    })
}

/// Read the derived inventory into the rows the emitted table holds.
fn read_inventory(table: &Table) -> Result<Vec<Row>, String> {
    let columns = [
        "address",
        "name",
        "standing",
        "direction_conditional",
        "statement_en",
        "statement_ja",
    ];
    if table.columns != columns {
        return Err(format!(
            "names the columns {found:?} where this generator reads {columns:?}",
            found = table.columns
        ));
    }

    let mut rows = Vec::new();
    for record in &table.records {
        rows.push(read_row(record)?);
    }
    check_inventory(&rows)?;
    Ok(rows)
}

/// Read one row, refusing everything the emitted table could not state.
fn read_row(record: &Record) -> Result<Row, String> {
    let rendered = field(record, 0)?.to_owned();
    let parsed = canonical(&rendered)
        .and_then(|_| address(&rendered))
        .ok_or_else(|| {
            at(
                record,
                &format!("`{rendered}` is not the canonical rendering of an address"),
            )
        })?;
    let note = match parsed.detail {
        Detail::Whole => 0,
        Detail::Note(ordinal) => ordinal,
        Detail::Cell(_, _) => {
            return Err(at(
                record,
                "a matrix cell is transcribed rather than derived and joins the inventory \
                 with the captured matrices (docs/adr/0009)",
            ));
        },
    };
    if parsed.section.parts.len() > ADDRESS_PARTS {
        return Err(at(
            record,
            &format!("`{rendered}` is deeper than the {ADDRESS_PARTS} components an address holds"),
        ));
    }
    let mut values = [0u8; ADDRESS_PARTS];
    for (slot, part) in values.iter_mut().zip(parsed.section.parts.iter()) {
        *slot = *part;
    }

    let name = field(record, 1)?.to_owned();
    if !is_constant_name(&name) {
        return Err(at(
            record,
            &format!("`{name}` will not spell a Rust constant in SCREAMING_SNAKE_CASE"),
        ));
    }
    let standing = field(record, 2)?;
    if standing != NORMATIVE {
        return Err(at(
            record,
            &format!(
                "states the standing `{standing}`; a derivation quotes the document, so a \
                 row that is not `{NORMATIVE}` is a reading published from docs/decisions/ \
                 rather than read from the snapshot (ADR 0009)"
            ),
        ));
    }
    let direction_conditional = match field(record, 3)? {
        "true" => true,
        "false" => false,
        other => {
            return Err(at(
                record,
                &format!("marks `direction_conditional` as `{other}`, which is not a truth"),
            ));
        },
    };
    let statement = field(record, 4)?.to_owned();
    if statement.is_empty() {
        return Err(at(
            record,
            "quotes nothing; a rule with no sentence cannot be reported to a reader who has \
             never seen this code (ADR 0013)",
        ));
    }

    Ok(Row {
        rendered,
        appendix: parsed.section.appendix,
        values,
        depth: u8::try_from(parsed.section.parts.len()).unwrap_or(u8::MAX),
        note,
        name,
        direction_conditional,
        statement,
    })
}

/// Hold the read inventory against the figures this repository declares.
fn check_inventory(rows: &[Row]) -> Result<(), String> {
    if rows.len() != RULE_ROWS {
        return Err(format!(
            "holds {found} row(s) where this repository was written against {RULE_ROWS}",
            found = rows.len()
        ));
    }
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for row in rows {
        if !seen.insert(&row.rendered) {
            return Err(format!(
                "addresses `{rendered}` twice, so two rules would share one identifier",
                rendered = row.rendered
            ));
        }
    }
    let mut named: BTreeSet<&str> = BTreeSet::new();
    for row in rows {
        if !named.insert(&row.name) {
            return Err(format!(
                "names two rules `{name}`, and a constant is declared once",
                name = row.name
            ));
        }
    }
    let marked: Vec<&str> = rows
        .iter()
        .filter(|row| row.direction_conditional)
        .map(|row| row.rendered.as_str())
        .collect();
    if marked != DIRECTION_CONDITIONAL {
        return Err(format!(
            "marks {marked:?} direction-conditional and docs/adr/0011 fixes \
             {DIRECTION_CONDITIONAL:?}; a fourth is a decision record and a code-owner \
             review, never an emitted row"
        ));
    }
    Ok(())
}

/// Whether a name is the `SCREAMING_SNAKE_CASE` identifier a constant is declared with.
fn is_constant_name(name: &str) -> bool {
    !name.is_empty()
        && name.starts_with(|first: char| first.is_ascii_uppercase())
        && !name.ends_with('_')
        && !name.contains("__")
        && name.chars().all(|character| {
            character.is_ascii_uppercase() || character.is_ascii_digit() || character == '_'
        })
}

/// The rows of the emitted inventory.
fn inventory_rows(rows: &[Row]) -> String {
    let written: Vec<String> = rows
        .iter()
        .map(|row| {
            format!(
                "    Rule {{\n\
                 \x20       address: Address::assembled({appendix}, {values:?}, {depth}, {note}),\n\
                 \x20       statement: {statement},\n\
                 \x20       direction_conditional: {mark},\n\
                 \x20       standing: Standing::Normative,\n\
                 \x20   }},\n",
                appendix = match row.appendix {
                    Some(letter) => format!("Some(Appendix::{letter})"),
                    None => "None".to_owned(),
                },
                values = row.values,
                depth = row.depth,
                note = row.note,
                statement = literal(&row.statement),
                mark = row.direction_conditional,
            )
        })
        .collect();
    format!("{HEAD}{body}];\n\n", body = written.concat())
}

/// What the emitted table says about itself, above its first row.
const HEAD: &str = "use crate::rule::{Address, Appendix, Rule, RuleId, Standing};\n\
                    \n\
                    /// Every inventoried rule, in the specification's own reading order.\n\
                    ///\n\
                    /// The address is written as its components rather than as text, because\n\
                    /// a table read at run time is not a `const`; `crates/jlreq-spec/src/\n\
                    /// rule.rs` reads every one of them back through the address grammar at\n\
                    /// compile time, so a component the grammar refuses is a build failure\n\
                    /// rather than a rule nobody can cite.\n\
                    ///\n\
                    /// JLReq: \u{a7}3, \u{a7}B, \u{a7}C, \u{a7}D, \u{a7}E, \u{a7}F\n\
                    pub(crate) const RULES: &[Rule] = &[\n";

/// The named identifier of every inventoried rule.
///
/// A rule is cited in code by a name and in a report by its address, which is what makes a
/// failure readable to someone who has never seen this code (ADR 0013). The ordinal is the
/// row's position in `RULES`, so the two tables cannot drift apart.
fn inventory_identifiers(rows: &[Row]) -> String {
    let written: Vec<String> = rows
        .iter()
        .enumerate()
        .map(|(ordinal, row)| {
            format!(
                "{gap}    /// The statement JLReq makes at \u{a7}{rendered}.\n\
                 \x20   ///\n\
                 \x20   /// JLReq: \u{a7}{rendered}\n\
                 {declaration}",
                gap = if ordinal == 0 { "" } else { "\n" },
                rendered = row.rendered,
                declaration = declaration(&row.name, ordinal),
            )
        })
        .collect();
    format!("impl RuleId {{\n{body}}}\n", body = written.concat())
}

/// One named identifier, wrapped where `rustfmt` would wrap it.
///
/// The emitted file has to be what `cargo fmt` would leave alone, because `just fmt-check`
/// and `generate --check` both run over it and a file only one of them accepts fails the
/// build whichever way it is written. JLReq's own section headings spell some of these
/// constants past the hundred columns `rustfmt.toml` sets, and there is nothing to shorten:
/// the name is the heading.
fn declaration(name: &str, ordinal: usize) -> String {
    let one_line = format!("    pub const {name}: Self = Self({ordinal});\n");
    if one_line.trim_end().chars().count() <= MAX_WIDTH {
        return one_line;
    }
    format!("    pub const {name}: Self =\n        Self({ordinal});\n")
}

/// One Rust string literal holding `text`.
///
/// Only the backslash and the double quote need escaping: every other byte of a derived
/// field is printable UTF-8, because the derivation collapsed each statement onto one line
/// and refused a tab.
fn literal(text: &str) -> String {
    let mut out = String::with_capacity(text.len().saturating_add(2));
    out.push('"');
    for character in text.chars() {
        match character {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

/// One field of one record.
fn field(record: &Record, index: usize) -> Result<&str, String> {
    record
        .fields
        .get(index)
        .map(String::as_str)
        .ok_or_else(|| format!("line {line}: has no field {index}", line = record.line))
}

/// A rejection, with the line it belongs to.
fn at(record: &Record, reason: &str) -> String {
    format!("line {line}: {reason}", line = record.line)
}

#[cfg(test)]
mod tests {
    use super::{
        ADDRESSABLE_SECTIONS, ANCHORS, DIRECTION_CONDITIONAL, HEADINGS, Heading, Locale, MAX_WIDTH,
        NOTE_RULES, NOTE_SECTIONS, NOTES, NUMBERED_HEADINGS, RULE_ROWS, RULE_SECTIONS, RULES,
        RULES_FILE, SNAPSHOT, UNADDRESSABLE, UNNUMBERED_HEADINGS, canonical, collapse, declaration,
        emit_inventory, headings, identifier, in_scope, is_constant_name, items, literal,
        names_a_direction, one_locale, paragraphs, plain, read_document, row,
    };
    use crate::shared;

    /// A heading of the published rendering, copied out of `spec/snapshot/index.html`.
    const HEADING: &str = "<h3 id=\"purpose_of_this_document\"><bdi class=\"secno\">1.1 </bdi>\n\t\t<span its-locale-filter-list=\"en\" lang=\"en\">Purpose of This Document</span>\n\t\t<span its-locale-filter-list=\"ja\" lang=\"ja\">この文書の目的</span>\n\t<a class=\"self-link\" aria-label=\"§\" href=\"#purpose_of_this_document\"></a></h3>";

    /// A top-level heading, whose rendered number carries a trailing period.
    const TOP: &str = "<h2 id=\"introduction\"><bdi class=\"secno\">1. </bdi>\n    <span its-locale-filter-list=\"en\" lang=\"en\">Introduction</span>\n    <span its-locale-filter-list=\"ja\" lang=\"ja\">序論</span>\n    <a class=\"self-link\" aria-label=\"§\" href=\"#introduction\"></a></h2>";

    /// An appendix heading the address grammar refuses, copied out of the snapshot.
    const APPENDIX: &str = "<h2 id=\"references\"><bdi class=\"secno\">H. </bdi>\n<span its-locale-filter-list=\"en\" lang=\"en\">References</span>\n<span its-locale-filter-list=\"ja\" lang=\"ja\">参考文献</span>\n<a class=\"self-link\" aria-label=\"§\" href=\"#references\"></a></h2>";

    /// The five headings the published rendering gives no section number, so that a fixture
    /// satisfies the count the scanner checks.
    const FRONT_MATTER: &str = "<h1 class=\"title\" id=\"title\">Requirements</h1><h2>W3C Working Group Note</h2><h2>Abstract</h2><h2>Status of This Document</h2><h2 id=\"table-of-contents\">Table of Contents</h2>";

    /// A paragraph carrying a cross reference, which repeats the referenced heading in both
    /// languages. Copied out of §A's opening paragraph.
    const CROSS_REFERENCE: &str = "<p its-locale-filter-list=\"en\" lang=\"en\">The following are lists of characters grouped by character class according to the classification explained in <a href=\"#grouping\" class=\"sec-ref\">§&nbsp;<bdi class=\"secno\">3.9.2 </bdi>\n<span its-locale-filter-list=\"en\" lang=\"en\">Grouping of Characters and Symbols depending on their Positioning</span>\n<span its-locale-filter-list=\"ja\" lang=\"ja\">文字・記号を振る舞い方により分ける</span>\n</a>. </p>";

    /// §B.1's opening, whose second item opens a list of its own. Copied from the snapshot.
    const NESTED: &str = "<ol class=\"decimal\">\n<li id=\"id546\">\n<p its-locale-filter-list=\"en\" lang=\"en\">The left-most column, labeled \"before\", lists preceding character classes.</p>\n<p its-locale-filter-list=\"ja\" lang=\"ja\">“before”（表の左端）と示した欄に，前に配置する文字クラスを示す．</p>\n</li>\n<li id=\"id548\">\n<p its-locale-filter-list=\"en\" lang=\"en\">The following notations in cells are used.</p>\n<p its-locale-filter-list=\"ja\" lang=\"ja\">表のそれぞれの小間に，次の記号で示す．</p>        <ol>\n<li id=\"id549\">\n<p its-locale-filter-list=\"en\" lang=\"en\">blank: Set solid between two adjacent characters.</p>\n<p its-locale-filter-list=\"ja\" lang=\"ja\">無印：文字間をベタ組にする．</p>\n</li>\n<li id=\"id550\">\n<p its-locale-filter-list=\"en\" lang=\"en\">× mark: The combination is not allowed.</p>\n<p its-locale-filter-list=\"ja\" lang=\"ja\">×印：このような配置を禁止する．</p>\n</li>\n</ol>\n</li>\n</ol>";

    /// A section whose own prose sits beside a Note, which the rendering makes a container.
    const WITH_NOTE: &str = "<p its-locale-filter-list=\"en\" lang=\"en\">Lead.</p>\n<p its-locale-filter-list=\"ja\" lang=\"ja\">導入．</p>\n<div class=\"note\" role=\"note\" id=\"n62\"><p its-locale-filter-list=\"en\" lang=\"en\">In vertical writing mode this differs.</p>\n<p its-locale-filter-list=\"ja\" lang=\"ja\">縦組では異なる．</p></div>";

    /// Read one heading out of a fragment, with the front matter the count check expects.
    fn single(fragment: &str) -> Heading {
        let text = format!("{fragment}{FRONT_MATTER}");
        let mut found = headings(&text).expect("five unnumbered headings satisfy the count");
        assert_eq!(found.len(), 1, "{found:?}");
        found.remove(0)
    }

    #[test]
    fn a_bilingual_heading_is_read_one_language_at_a_time() {
        for (fragment, number, anchor, english, japanese) in [
            (
                HEADING,
                "1.1",
                "purpose_of_this_document",
                "Purpose of This Document",
                "この文書の目的",
            ),
            (TOP, "1", "introduction", "Introduction", "序論"),
            (APPENDIX, "H", "references", "References", "参考文献"),
        ] {
            let one = single(fragment);
            assert_eq!(one.rendered, number);
            assert_eq!(one.anchor, anchor);
            assert_eq!(one.title_en, english);
            assert_eq!(one.title_ja, japanese);
        }
    }

    #[test]
    fn the_rendered_number_is_what_the_address_is_read_from() {
        assert_eq!(single(TOP).address.as_deref(), Some("1"));
        assert_eq!(single(HEADING).address.as_deref(), Some("1.1"));
        assert_eq!(
            single(APPENDIX).address,
            None,
            "the grammar runs A through G, so H is a heading and not an address"
        );
        assert_eq!(canonical("3.1.10"), Some("3.1.10".to_owned()));
        assert_eq!(canonical("A.30"), Some("A.30".to_owned()));
        assert_eq!(canonical("J"), None);
    }

    #[test]
    fn a_heading_that_is_not_the_documents_shape_is_refused() {
        let missing = "<h3 id=\"x\"><bdi class=\"secno\">1.1 </bdi><span its-locale-filter-list=\"en\" lang=\"en\">Only English</span></h3>";
        let violation = headings(&format!("{missing}{FRONT_MATTER}"))
            .expect_err("a heading with one locale is not this document's shape");
        assert!(violation.contains("`ja` span"), "{violation}");

        let unnumbered =
            headings("<h2>Abstract</h2>").expect_err("the count of unnumbered headings is checked");
        assert!(
            unnumbered.contains("carry no rendered section number"),
            "{unnumbered}"
        );
    }

    #[test]
    fn the_other_locale_is_removed_before_a_paragraph_is_read() {
        let read = paragraphs(CROSS_REFERENCE).expect("the fragment is well formed");
        assert_eq!(read.len(), 1);
        let paragraph = read.first().expect("one paragraph");
        assert_eq!(paragraph.depth, 0);
        assert_eq!(paragraph.locale, Some(Locale::English));
        assert!(
            paragraph.text.contains(
                "§ 3.9.2 Grouping of Characters and Symbols depending on their Positioning"
            ),
            "the cross reference keeps its own rendered number: {text}",
            text = paragraph.text
        );
        assert!(
            !paragraph.text.contains("文字・記号"),
            "an English statement must not end in Japanese: {text}",
            text = paragraph.text
        );
        assert!(
            paragraph.text.ends_with('.'),
            "{text}",
            text = paragraph.text
        );
    }

    #[test]
    fn a_paragraph_inside_a_note_is_not_the_sections_own_prose() {
        let read = paragraphs(WITH_NOTE).expect("the fragment is well formed");
        let own: Vec<&str> = read
            .iter()
            .filter(|paragraph| paragraph.depth == 0)
            .map(|paragraph| paragraph.text.as_str())
            .collect();
        assert_eq!(own, ["Lead.", "導入．"]);
        assert_eq!(read.len(), 4, "the Note's two paragraphs are read, deeper");
        assert!(
            read.iter()
                .any(|paragraph| paragraph.depth == 1 && paragraph.text.contains("vertical")),
            "{read:?}"
        );
    }

    #[test]
    fn a_nested_list_does_not_add_items_to_the_one_above_it() {
        let read = items(NESTED).expect("the fragment is well formed");
        assert_eq!(read.len(), 2, "two items, not four: {read:?}");
        assert!(read.iter().any(|item| item.contains("left-most column")));
        assert!(
            read.iter().any(|item| item.contains("blank: Set solid")),
            "the nested list stays inside the item that opens it"
        );
    }

    #[test]
    fn every_entity_the_document_uses_is_decoded_and_no_other_is() {
        assert_eq!(plain("a&nbsp;b").as_deref(), Ok("a b"));
        assert_eq!(plain("&lt;0254, 0300&gt;").as_deref(), Ok("<0254, 0300>"));
        assert_eq!(plain("R&amp;D &quot;x&quot;").as_deref(), Ok("R&D \"x\""));
        assert!(
            plain("&copy;").is_err(),
            "an entity the scanner cannot decode is refused rather than passed through"
        );
    }

    #[test]
    fn markup_becomes_one_line_of_text() {
        assert_eq!(
            plain("<p>one\n   two\t<em>three</em></p>").as_deref(),
            Ok("one two three")
        );
        assert_eq!(collapse("  a \n b  "), "a b");
    }

    #[test]
    fn a_writing_mode_is_recognized_in_either_language() {
        let english = Some(Locale::English);
        let japanese = Some(Locale::Japanese);
        assert!(names_a_direction(english, "In vertical writing mode"));
        assert!(names_a_direction(english, "Horizontal-in-Vertical"));
        assert!(!names_a_direction(english, "a solid setting"));
        assert!(names_a_direction(japanese, "縦組では異なる．"));
        assert!(names_a_direction(japanese, "縦中横の処理"));
        assert!(!names_a_direction(japanese, "ベタ組にする．"));
        assert!(!names_a_direction(None, "vertical"));
    }

    #[test]
    fn the_scope_is_line_composition_and_the_appendices_it_defers_to() {
        for inside in ["3", "3.1.9", "B", "B.2", "C.3", "D.1", "E.2", "F.3"] {
            assert!(in_scope(inside), "{inside}");
        }
        for outside in ["1", "2.1.2", "4.5.1", "A", "A.19", "G"] {
            assert!(!in_scope(outside), "{outside}");
        }
    }

    #[test]
    fn a_constant_name_is_the_documents_own_heading() {
        let mut violations = Vec::new();
        assert_eq!(
            identifier("Characters Not Starting a Line", &mut violations),
            "CHARACTERS_NOT_STARTING_A_LINE"
        );
        assert_eq!(
            identifier(
                "Handling of Tate-chu-yoko (Horizontal-in-Vertical Settings)",
                &mut violations
            ),
            "HANDLING_OF_TATE_CHU_YOKO_HORIZONTAL_IN_VERTICAL_SETTINGS"
        );
        assert_eq!(
            identifier("Legend of Table 1", &mut violations),
            "LEGEND_OF_TABLE_1"
        );
        assert!(violations.is_empty(), "{violations:?}");
        let _ = identifier("1 leading digit", &mut violations);
        assert_eq!(violations.len(), 1, "{violations:?}");
    }

    #[test]
    fn a_locale_span_is_dropped_with_its_nesting() {
        let kept = one_locale(
            "a<span its-locale-filter-list=\"ja\" lang=\"ja\">x<span>y</span>z</span>b",
            Locale::English,
        )
        .expect("the fragment is well formed");
        assert_eq!(kept, "ab");
    }

    #[test]
    fn a_field_holding_a_tab_or_a_newline_is_refused() {
        assert_eq!(row(&["a", "b"]).as_deref(), Ok("a\tb\n"));
        assert!(row(&["a\tb"]).is_err());
        assert!(row(&["a\nb"]).is_err());
    }

    #[test]
    fn the_marked_rules_are_the_ones_the_decision_record_fixes() {
        assert_eq!(DIRECTION_CONDITIONAL, ["3.1.3", "3.2.5", "3.3.5"]);
        for marked in DIRECTION_CONDITIONAL {
            assert!(
                shared::address(marked).is_some(),
                "a marked rule is addressable: {marked}"
            );
        }
    }

    #[test]
    fn the_declared_figures_agree_with_one_another() {
        assert_eq!(RULE_ROWS, RULE_SECTIONS.saturating_add(NOTE_RULES));
        assert_eq!(
            NOTE_RULES,
            NOTE_SECTIONS.iter().map(|(_, count)| *count).sum::<usize>()
        );
        assert_eq!(
            ADDRESSABLE_SECTIONS.saturating_add(UNADDRESSABLE.len()),
            NUMBERED_HEADINGS
        );
        assert_eq!(
            NUMBERED_HEADINGS.saturating_add(UNNUMBERED_HEADINGS),
            HEADINGS
        );
    }

    #[test]
    fn the_snapshot_reads_into_the_counts_this_repository_declares() {
        let root = shared::workspace_root().expect("the workspace root");
        let html = std::fs::read_to_string(root.join(SNAPSHOT)).expect("the vendored snapshot");
        let document = read_document(&html).expect("the snapshot reads cleanly");
        assert_eq!(document.headings.len(), NUMBERED_HEADINGS);
        assert_eq!(document.notes.len(), NOTE_RULES);
        assert_eq!(document.rules.len(), RULE_ROWS);
        assert_eq!(
            document
                .rules
                .iter()
                .filter(|rule| rule.direction_conditional)
                .count(),
            DIRECTION_CONDITIONAL.len()
        );
    }

    #[test]
    fn every_derived_table_states_its_columns_on_the_first_line_it_does_not_skip() {
        let root = shared::workspace_root().expect("the workspace root");
        let html = std::fs::read_to_string(root.join(SNAPSHOT)).expect("the vendored snapshot");
        let sources = vec![html];
        for derivation in [ANCHORS, RULES, NOTES] {
            let rows = (derivation.read)(&sources).expect(derivation.output);
            let first = rows.lines().next().unwrap_or("");
            assert!(
                first.starts_with("address\t"),
                "every reader of a derived table takes the first line it does not skip as \
                 the header, and each of them finds it that way rather than by counting \
                 lines, because `derive` writes a comment block above whatever a derivation \
                 emits. A derivation whose own first line were a comment would hide its \
                 column line inside that block: {first}"
            );
            assert!(!rows.contains('\r'), "the specification data is LF");
            for line in rows.lines().skip(1) {
                assert!(
                    line.starts_with('#') || line.contains('\t'),
                    "every line after the header is a comment or a row: {line}"
                );
            }
        }
    }

    #[test]
    fn a_name_that_will_not_spell_a_constant_is_refused() {
        assert!(is_constant_name(
            "POSITIONING_OF_OPENING_BRACKETS_AT_LINE_HEAD"
        ));
        assert!(is_constant_name("TABLE_1"));
        assert!(
            !is_constant_name("positioning"),
            "the emitted constants are SCREAMING_SNAKE_CASE and a name is checked, not fixed"
        );
        assert!(!is_constant_name(""), "a heading that reduced to nothing");
        assert!(
            !is_constant_name("TRAILING_"),
            "a trailing separator is what a heading ending in punctuation would leave"
        );
        assert!(
            !is_constant_name("DOUBLE__SEPARATOR"),
            "two separators in a row would make two headings spell one name"
        );
        assert!(!is_constant_name("HAS-A-DASH"));
    }

    #[test]
    fn a_statement_becomes_a_rust_literal_that_says_the_same_thing() {
        assert_eq!(literal("plain"), "\"plain\"");
        assert_eq!(
            literal("the mark \u{300c}\u{300d} is quoted"),
            "\"the mark \u{300c}\u{300d} is quoted\"",
            "a quotation JLReq writes in Japanese needs no escaping and gets none"
        );
        assert_eq!(
            literal("says \"this\""),
            "\"says \\\"this\\\"\"",
            "a quotation mark inside a statement would end the literal early"
        );
        assert_eq!(
            literal("a backslash \\ stands for itself"),
            "\"a backslash \\\\ stands for itself\""
        );
    }

    #[test]
    fn a_declaration_is_wrapped_exactly_where_rustfmt_would_wrap_it() {
        let short = declaration("SHORT", 3);
        assert_eq!(short, "    pub const SHORT: Self = Self(3);\n");
        assert!(short.trim_end().chars().count() <= MAX_WIDTH);

        let long = declaration(
            "DIFFERENCES_IN_VERTICAL_AND_HORIZONTAL_COMPOSITION_IN_USE_OF_PUNCTUATION_MARKS",
            0,
        );
        assert!(
            long.lines().count() == 2 && long.lines().all(|line| line.chars().count() <= MAX_WIDTH),
            "JLReq's own headings spell some of these past a hundred columns, and the \
             emitted file has to be what `cargo fmt` would leave alone"
        );
    }

    #[test]
    fn the_generated_inventory_is_read_from_the_derived_one() {
        // The whole path, over the file this repository commits: the emitted table holds one
        // row per derived row, and the identifiers are that statement counted.
        let text = std::fs::read_to_string(
            crate::shared::workspace_root()
                .expect("a workspace root")
                .join(RULES_FILE),
        )
        .expect("the derived inventory is committed");
        let table = crate::generate::read_table(RULES_FILE.to_owned(), &text)
            .expect("the derived inventory is tab separated");
        let emission = emit_inventory(&table).expect("the derived inventory is readable");
        assert_eq!(emission.entries, RULE_ROWS);
        assert_eq!(
            emission.items.matches("    Rule {").count(),
            RULE_ROWS,
            "one emitted row per inventoried statement"
        );
        assert_eq!(
            emission.items.matches("pub const ").count(),
            RULE_ROWS,
            "and one named identifier per row, so a rule is cited by name in code and by \
             address in a report"
        );
        assert_eq!(
            emission
                .items
                .matches("direction_conditional: true")
                .count(),
            DIRECTION_CONDITIONAL.len(),
            "the emitted mark and the derived mark are two forms of one inventory"
        );
    }
}
