// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The recorded defects of the published document: `spec/derived/defects.tsv`.
//!
//! JLReq is internally inconsistent in twelve places this repository has to read. A code
//! point is listed twice, three Remarks cells lose their English half to a missing locale
//! span, a §D.2 note contradicts three of its siblings on a priority ordinal, a note reads
//! "vertical" in English against 横組 in Japanese, a ReSpec cross reference was never
//! expanded, and every table ordinal in an anchor id or a PDF filename is one higher than
//! the number the rendering prints. Each is recorded here so that a revision which fixes
//! one fails the build rather than drifting silently past us (ADR 0009).
//!
//! # Why this is a derivation and not an attestation
//!
//! [ADR 0009](../../docs/adr/0009-generated-data-and-attested-transcription.md) draws the
//! line at machine readability: generated where the specification is machine-readable,
//! attested where it is not. Every one of these twelve is a property of
//! `spec/snapshot/index.html`, which is the machine-readable half. None of them is a
//! property of the six matrices W3C publishes only as PDF. So this file belongs on the
//! derived side, `derive`'s scan for stray files requires a derivation to claim it, and
//! `attest` reads it as a derived table.
//!
//! Being derived is a claim with teeth, and the shape of this module is what earns it.
//! A list of twelve sentences in a constant, printed into a file, would be an attestation
//! wearing a derivation's header: `derive --check` would prove only that the constant had
//! not changed. So every defect carries a **detector** — a measurement over the vendored
//! rendering — and the row's evidence is composed from what that detector measured. A
//! detector that no longer finds its defect fails the derivation and names what to do
//! about it. That is ADR 0009's "a defect fixed upstream fails the gate and forces a
//! review", enforced rather than asserted.
//!
//! Every one of the twelve is detected. There is no attested half, and this module says so
//! only because it was worth checking: three of the readings that look like human judgment
//! turn out to have exact predicates, and each one fires on exactly the passage recorded.
//!
//! - "vertical" against 横組 (§3.1.3) is the only English/Japanese paragraph pair in the
//!   whole rendering where one half names a writing mode, the other names the other, and
//!   neither names both.
//! - "simple-ruby" where jukugo-ruby is meant (§B.2#11) is the only paragraph whose English
//!   half names the simple-ruby complex where its Japanese half never names 熟語ルビ以外.
//! - The reduction that is uniform in English and 文字サイズ比で均等に — proportional to
//!   character size — in Japanese is one of exactly two pairs whose Japanese half says
//!   文字サイズ比 and whose English half never says "character size"; the other twenty-five
//!   agree. Both are step 1 of a §3.8.3 priority list, so the recorded defect covers both
//!   and the detector reports the pair.
//!
//! # The three columns the document owns, and the one this reader owns
//!
//! `where` and `evidence` are measured. `treatment` is this repository's sentence about
//! what it does with the defect — a reading, not a property of the text — and it is a
//! column of a derived file for the same reason `direction_conditional` is a column of
//! `rules.tsv`: the reading is published beside the measurement it applies to rather than
//! buried in a module nobody reading the table will open. The file's own explanation block
//! says which is which.
//!
//! # What the identifiers are held against
//!
//! `xtask/src/attest.rs` carries the same twelve identifiers in `RECORDED_DEFECTS`, and
//! `attest` requires the two lists to be equal. Two lists rather than one shared constant is
//! deliberate: the gate that checks the file is not the program that writes it, so a row
//! deleted here is a failure there.
//!
//! See `docs/design/generation.md`, `docs/adr/0009` and `docs/adr/0013`.

use std::collections::{BTreeMap, BTreeSet};

use crate::derive::Derivation;

/// The vendored rendering of the published document.
const SNAPSHOT: &str = "spec/snapshot/index.html";

/// The recorded defects of the published document.
pub(crate) const DEFECTS: Derivation = Derivation {
    sources: &[SNAPSHOT],
    reader: &["xtask/src/defects.rs"],
    output: "spec/derived/defects.tsv",
    caption: "Every defect of the published document this repository records, in the order \
              the rendering states them, each with the measurement that must still find it.",
    read: read_defects,
};

// ---------------------------------------------------------------------------------------
// The figures this module declares, each measured against the vendored snapshot
// ---------------------------------------------------------------------------------------

/// The class tables Appendix A publishes. Five of the thirty classes enumerate nothing.
const CLASS_TABLES: usize = 25;

/// Every data row of those tables, the one duplicate included.
const APPENDIX_A_ROWS: usize = 1687;

/// Every adjacent English/Japanese paragraph pair the rendering holds.
const PARAGRAPH_PAIRS: usize = 1536;

/// The two locale tags the bilingual rendering marks its spans with.
const LOCALES: [&str; 2] = ["en", "ja"];

// ---------------------------------------------------------------------------------------
// The catalogue
// ---------------------------------------------------------------------------------------

/// One defect of the published document, with the measurement that must still find it.
#[derive(Debug)]
struct Defect {
    /// The identifier this project records the defect under.
    id: &'static str,
    /// Where the rendering states it, spelled as ADR 0013 spells an address.
    site: &'static str,
    /// What this repository does about it.
    ///
    /// This reader's sentence rather than the document's, the way `direction_conditional`
    /// in `rules.tsv` is ADR 0011's reading rather than a property of the text. It states
    /// what the repository does today: a defect nothing has met yet says so, because a
    /// treatment column that promised a milestone's work would be the one unchecked claim
    /// in a file whose whole subject is unchecked claims.
    treatment: &'static str,
    /// The measurement, which returns the evidence or says why it no longer holds.
    detect: fn(&Document<'_>) -> Result<String, String>,
}

/// The twelve defects, in the order the rendering states them.
const RECORDED: [Defect; 12] = [
    Defect {
        id: "line-composition-note-locale-divergence",
        site: "3.1.3",
        treatment: "rules.tsv emits §3.1.3 in both locales and marks it \
                    direction-conditional. The unified pipeline keeps the document's \
                    divergence explicit and protocol cases fix the observable vertical \
                    punctuation and construct-context behavior.",
        detect: direction_divergence,
    },
    Defect {
        id: "dividing-punctuation-note-unresolved-reference",
        site: "3.1.6",
        treatment: "The Japanese half resolves the reference to §B, which is where the \
                    composition rules for cl-04 and cl-03 are tabulated; notes.tsv and \
                    rules.tsv emit both halves, while the conformance gate validates every \
                    canonical address used by protocol metadata, so the unresolved one is \
                    the document's alone.",
        detect: unresolved_cross_reference,
    },
    Defect {
        id: "appendix-d-table-numbering-off-by-one",
        site: "3.8.3, D",
        treatment: "Nothing here cites a JLReq table by number: ADR 0013 addresses a rule \
                    by its rendered section number, and a captured matrix is named by the \
                    number its own caption prints, which is Appendix D's and never \
                    §3.8.3's.",
        detect: appendix_d_numbering,
    },
    Defect {
        id: "reduction-step-1-locale-divergence",
        site: "3.8.3",
        treatment: "rules.tsv emits §3.8.3 in both locales. The unified integer adjustment \
                    pipeline uses the captured table stages and referent character sizes; \
                    mixed-size protocol cases freeze that observable reading.",
        detect: reduction_step_divergence,
    },
    Defect {
        id: "bracket-class-enumeration-mismatch",
        site: "3.9.2, A.28, A.29",
        treatment: "kumihan's private classifier uses the three members §A.28 and §A.29 enumerate, \
                    which is the only closed statement the document makes about either \
                    class. Whether a fourth bracket belongs is a silence for \
                    docs/decisions/ rather than an answer this stage may invent.",
        detect: bracket_enumeration,
    },
    Defect {
        id: "cl-19-duplicate-u216b",
        site: "A.19",
        treatment: "appendix-a.tsv keeps both rows, xtask/src/classes.rs records the \
                    duplicate in RECORDED_DUPLICATES and fails on one it has not recorded, \
                    and the emitted cl-19 table holds the distinct members.",
        detect: duplicate_row,
    },
    Defect {
        id: "cl-24-remarks-role-stated-only-in-japanese",
        site: "A.24, A.25",
        treatment: "The role is read from the Japanese half, which is the half that states \
                    it, and emitted for both locales. xtask/src/classes.rs records each \
                    cell with the count of cells holding it, so a Remarks cell that gained \
                    or lost the line fails the derivation.",
        detect: role_only_in_japanese,
    },
    Defect {
        id: "cl-25-remarks-without-locale-span",
        site: "A.25",
        treatment: "The three cells are read as the remark the localized cells state, so \
                    the Proportional frame is emitted for all of them. \
                    xtask/src/classes.rs enumerates every distinct Remarks cell shape, and \
                    a shape nobody has read fails the derivation.",
        detect: unlocalised_remarks,
    },
    Defect {
        id: "legend-anchor-and-filename-off-by-one",
        site: "B.1, C.1, D.1, E.1",
        treatment: "anchors.tsv is built from each heading's rendered section number with \
                    the anchor id beside it as a second column, and xtask/src/attest.rs \
                    computes a matrix's published filename as its table number plus one. \
                    Both off-by-ones are asserted rather than absorbed.",
        detect: legend_numbering,
    },
    Defect {
        id: "b2-note-7-locale-class-divergence",
        site: "B.2#7",
        treatment: "notes.tsv emits both halves, so the two classes the English half omits \
                    are in the inventory through the Japanese column, which is the half \
                    that states them.",
        detect: note_class_divergence,
    },
    Defect {
        id: "b2-note-11-simple-ruby-misnomer",
        site: "B.2#11",
        treatment: "notes.tsv emits both halves verbatim. The generated Appendix B cell is \
                    cl-23 and the unified construct pipeline therefore applies it only to \
                    jukugo-ruby runs; the English misnomer is never used as a class key.",
        detect: simple_ruby_misnomer,
    },
    Defect {
        id: "d2-note-5-line-end-qualifier-omitted-in-english",
        site: "D.2#5",
        treatment: "The role of the position is read from the Japanese half, which is the \
                    half that states it, exactly as the two other locale divergences are: \
                    notes.tsv emits all five notes in full, and the reduction ladder reads \
                    §3.8.3's own two steps, where the line-end reduction and the mid-line \
                    one are separate list items with separate ordinals.",
        detect: line_end_qualifier_omitted,
    },
];

// ---------------------------------------------------------------------------------------
// The reader
// ---------------------------------------------------------------------------------------

/// Read the recorded defects out of the published rendering.
fn read_defects(sources: &[String]) -> Result<String, String> {
    let document = Document::read(only(sources)?)?;
    let mut out = String::from("id\twhere\tevidence\ttreatment\n");
    out.push_str(&explanation(&[
        "`where` and `evidence` are the document's. The site is where the rendering states",
        "the defect, addressed by its own rendered section number (docs/adr/0013), and the",
        "evidence is what xtask/src/defects.rs measured in the vendored snapshot. Line",
        "numbers are lines of that file at the digest this header states.",
        "",
        "`treatment` is this repository's, the way `direction_conditional` in rules.tsv is a",
        "reading rather than a property of the text. It says what the pipeline does today,",
        "so a defect nothing has met yet says that rather than promising a milestone.",
        "",
        "Every row is a detection: each defect carries a measurement that must still find it",
        "in the rendering, and this derivation fails when one does not. A defect corrected",
        "upstream therefore fails the build and forces a review instead of changing an",
        "answer quietly (docs/adr/0009). The identifiers are the ones RECORDED_DEFECTS in",
        "xtask/src/attest.rs carries, and `attest` requires the two lists to be equal.",
    ]));

    let mut rows = String::new();
    let mut lost = Vec::new();
    for defect in &RECORDED {
        match (defect.detect)(&document) {
            Ok(evidence) => rows.push_str(&row(&[
                defect.id,
                defect.site,
                &evidence,
                &collapse(defect.treatment),
            ])?),
            Err(reason) => lost.push(format!("{id}: {reason}", id = defect.id)),
        }
    }
    if !lost.is_empty() {
        return Err(lost.join("\n  "));
    }
    out.push_str(&rows);
    Ok(out)
}

/// The one source this derivation reads.
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
/// For the reason `xtask/src/inventory.rs` states: every reader of a derived table skips a
/// comment wherever it appears, and one of them takes the first line it does not skip as
/// the header.
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
/// is refused rather than escaped. Every string this module composes is collapsed to one
/// line first.
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

/// What a detector says when the rendering no longer states its defect.
///
/// A correction upstream is the outcome this file exists to make loud, so the message is
/// the review procedure and not an apology.
fn corrected(measurement: &str) -> String {
    format!(
        "{measurement}. If the published document has been corrected, delete this defect \
         here, delete its identifier from RECORDED_DEFECTS in xtask/src/attest.rs and its \
         row from the table in docs/design/generation.md, and review whatever read the \
         defect (ADR 0009)"
    )
}

// ---------------------------------------------------------------------------------------
// The document, scanned once
// ---------------------------------------------------------------------------------------

/// The vendored rendering, scanned into the shapes the detectors read.
#[derive(Debug)]
struct Document<'a> {
    /// The rendering itself.
    html: &'a str,
    /// The byte offset each line starts at, so a finding can name a line.
    line_starts: Vec<usize>,
    /// Every heading, in document order.
    headings: Vec<Heading<'a>>,
    /// The Appendix A class tables, in class order.
    tables: Vec<ClassTable<'a>>,
    /// Every adjacent English/Japanese paragraph pair.
    pairs: Vec<Pair>,
}

/// One heading, with the body it opens.
#[derive(Debug)]
struct Heading<'a> {
    /// The anchor id the rendering gives it, which this module reads only to record that
    /// it disagrees with the number beside it.
    anchor: &'a str,
    /// The rendered section number, without its trailing dot: `3.1.6`, `B.1`, `D`.
    number: String,
    /// The English heading text.
    title: String,
    /// The byte range from the end of the heading to the start of the next one.
    body: (usize, usize),
}

/// One Appendix A class table.
#[derive(Debug)]
struct ClassTable<'a> {
    /// The class it enumerates, written `cl-19`.
    class: &'a str,
    /// The rendered section number of its heading, written `A.19`.
    section: String,
    /// Its rows, in document order.
    rows: Vec<TableRow<'a>>,
}

/// One row of an Appendix A class table.
#[derive(Debug)]
struct TableRow<'a> {
    /// The line the row starts on.
    line: usize,
    /// The row's cells, as published markup.
    cells: Vec<&'a str>,
}

impl TableRow<'_> {
    /// The UCS cell: the key the row lists.
    fn key(&self) -> String {
        text(self.cells.get(1).copied().unwrap_or_default())
    }

    /// The Remarks cell, as published markup.
    fn remarks(&self) -> &str {
        self.cells.get(4).copied().unwrap_or_default()
    }

    /// The whole row as one line of text.
    fn line_text(&self) -> String {
        collapse(
            &self
                .cells
                .iter()
                .map(|cell| text(cell))
                .collect::<Vec<String>>()
                .join(" "),
        )
    }
}

/// One paragraph published twice, once in each locale.
#[derive(Debug)]
struct Pair {
    /// The line the English half starts on.
    line: usize,
    /// The byte offset of the English half.
    offset: usize,
    /// The English half as text, with the Japanese spans removed first.
    english: String,
    /// The Japanese half as text, likewise.
    japanese: String,
}

/// One item of an appendix `Notes` list.
#[derive(Debug)]
struct Note {
    /// Its position in the list, counting from one.
    ordinal: usize,
    /// The line the item starts on.
    line: usize,
    /// The English half.
    english: Half,
    /// The Japanese half.
    japanese: Half,
}

/// One locale's half of a passage, kept as markup and as text.
#[derive(Debug, Default)]
struct Half {
    /// The markup, with the other locale's spans removed.
    markup: String,
    /// The same, as one line of text.
    text: String,
}

impl Half {
    /// Read one locale's half out of every paragraph of a fragment.
    fn read(fragment: &str, locale: &str) -> Self {
        let other = other_locale(locale);
        let mut markup = String::new();
        for element in elements(fragment, "p") {
            if locale_of(element.attributes) != Some(locale) {
                continue;
            }
            if !markup.is_empty() {
                markup.push(' ');
            }
            markup.push_str(&drop_locale(element.content, other));
        }
        let text = text(&markup);
        Self { markup, text }
    }

    /// The character classes this half links to, as `cl-NN`.
    fn classes(&self) -> BTreeSet<String> {
        let marker = "href=\"#cl-";
        let mut found = BTreeSet::new();
        for (at, _) in self.markup.match_indices(marker) {
            if let Some(ordinal) = self
                .markup
                .get(at.saturating_add(marker.len())..)
                .and_then(two_digits)
            {
                found.insert(format!("cl-{ordinal}"));
            }
        }
        found
    }
}

impl<'a> Document<'a> {
    /// Scan the rendering, and refuse it when it is not the shape this module reads.
    fn read(html: &'a str) -> Result<Self, String> {
        let headings = headings(html);
        let tables = class_tables(html, &headings);
        let counted = tables.iter().map(|table| table.rows.len()).sum::<usize>();
        if tables.len() != CLASS_TABLES || counted != APPENDIX_A_ROWS {
            return Err(format!(
                "{SNAPSHOT}: holds {found} Appendix A table(s) of {counted} row(s); this \
                 module reads {CLASS_TABLES} of {APPENDIX_A_ROWS}",
                found = tables.len()
            ));
        }
        let pairs = pairs(html);
        if pairs.len() != PARAGRAPH_PAIRS {
            return Err(format!(
                "{SNAPSHOT}: holds {found} English/Japanese paragraph pair(s); this module \
                 reads {PARAGRAPH_PAIRS}",
                found = pairs.len()
            ));
        }
        let mut document = Self {
            html,
            line_starts: line_starts(html),
            headings,
            tables,
            pairs: Vec::new(),
        };
        document.pairs = pairs
            .into_iter()
            .map(|(offset, english, japanese)| Pair {
                line: document.line_of(offset),
                offset,
                english,
                japanese,
            })
            .collect();
        Ok(document)
    }

    /// The one-based line an offset sits on.
    fn line_of(&self, offset: usize) -> usize {
        self.line_starts.partition_point(|start| *start <= offset)
    }

    /// The section the rendering numbers `number`.
    ///
    /// Addressed by the rendered number and never by the anchor slug, which is the rule
    /// ADR 0013 fixes and which one of the defects below is the reason for.
    fn section(&self, number: &str) -> Result<&Heading<'a>, String> {
        self.headings
            .iter()
            .find(|heading| heading.number == number)
            .ok_or_else(|| {
                corrected(format!("the rendering numbers no section §{number}").as_str())
            })
    }

    /// The markup of a section's body.
    fn body(&self, heading: &Heading<'a>) -> &'a str {
        let (start, end) = heading.body;
        self.html.get(start..end).unwrap_or_default()
    }

    /// The notes of an appendix `Notes` section, numbered as `notes.tsv` numbers them.
    fn notes(&self, number: &str) -> Result<Vec<Note>, String> {
        let section = self.section(number)?;
        let body = self.body(section);
        let (start, _) = section.body;
        let mut found = Vec::new();
        for (index, (offset, item)) in list_items(body).into_iter().enumerate() {
            found.push(Note {
                ordinal: index.saturating_add(1),
                line: self.line_of(start.saturating_add(offset)),
                english: Half::read(item, "en"),
                japanese: Half::read(item, "ja"),
            });
        }
        if found.is_empty() {
            return Err(corrected(format!("§{number} publishes no notes").as_str()));
        }
        Ok(found)
    }

    /// The pairs whose English half starts inside a section.
    fn pairs_of(&self, heading: &Heading<'a>) -> Vec<&Pair> {
        let (start, end) = heading.body;
        self.pairs
            .iter()
            .filter(|pair| pair.offset >= start && pair.offset < end)
            .collect()
    }
}

// ---------------------------------------------------------------------------------------
// The twelve detectors
// ---------------------------------------------------------------------------------------

/// §3.1.3's closing Note states the rule for opposite writing modes in its two halves.
///
/// The predicate is general: one half names a writing mode, the other names the other, and
/// neither names both. It fires once in the whole rendering.
fn direction_divergence(document: &Document<'_>) -> Result<String, String> {
    let found: Vec<&Pair> = document
        .pairs
        .iter()
        .filter(|pair| crossed_direction(pair))
        .collect();
    let [pair] = found.as_slice() else {
        return Err(corrected(&format!(
            "{count} paragraph pair(s) name opposite writing modes, not one",
            count = found.len()
        )));
    };
    let section = document.section("3.1.3")?;
    let (start, end) = section.body;
    if pair.offset < start || pair.offset >= end {
        return Err(corrected(&format!(
            "the one crossed pair is at line {line}, outside §3.1.3",
            line = pair.line
        )));
    }
    Ok(format!(
        "§3.1.3's closing Note states the rule for opposite writing modes in its two \
         halves. The English opens `{english}` and the Japanese opens 横組において — in \
         horizontal writing mode — at line {line}: `{japanese}`. It is the only one of the \
         rendering's {PARAGRAPH_PAIRS} English/Japanese paragraph pairs where one half \
         names a writing mode, the other names the other, and neither names both.",
        english = head(&pair.english, 90),
        japanese = head(&pair.japanese, 60),
        line = pair.line,
    ))
}

/// Whether a pair's two halves name opposite writing modes.
fn crossed_direction(pair: &Pair) -> bool {
    let english = pair.english.to_lowercase();
    let vertical = english.contains("vertical writing mode");
    let horizontal = english.contains("horizontal writing mode");
    let tategumi = pair.japanese.contains('縦');
    let yokogumi = pair.japanese.contains('横');
    (vertical && !horizontal && yokogumi && !tategumi)
        || (horizontal && !vertical && tategumi && !yokogumi)
}

/// §3.1.6 leaves a ReSpec cross reference unexpanded in one English half.
fn unresolved_cross_reference(document: &Document<'_>) -> Result<String, String> {
    let found: Vec<usize> = document
        .html
        .match_indices("[[[")
        .map(|(offset, _)| offset)
        .collect();
    let [offset] = found.as_slice() else {
        return Err(corrected(&format!(
            "the rendering holds {count} unexpanded cross reference(s), not one",
            count = found.len()
        )));
    };
    let literal = document
        .html
        .get(*offset..)
        .and_then(|tail| tail.find("]]]").map(|end| end.saturating_add("]]]".len())))
        .and_then(|end| document.html.get(*offset..offset.saturating_add(end)))
        .unwrap_or_default();
    let section = document.section("3.1.6")?;
    let (start, end) = section.body;
    if *offset < start || *offset >= end {
        return Err(corrected(
            "the one unexpanded cross reference is outside §3.1.6",
        ));
    }
    let notes = document
        .body(section)
        .matches("<div class=\"note\"")
        .count();
    let before = document
        .html
        .get(start..*offset)
        .unwrap_or_default()
        .matches("<div class=\"note\"")
        .count();
    Ok(format!(
        "§3.1.6 publishes {notes} Notes, and note {before}, at line {line}, leaves a ReSpec \
         cross reference unexpanded in its English half: `{literal}`. The Japanese half of \
         the same note resolves it to §B Spacing between Characters. It is the only \
         unexpanded `[[[…]]]` in the rendering, so an English-only reader is left without \
         the table the sentence promises.",
        line = document.line_of(*offset),
    ))
}

/// §3.8.3 numbers Appendix D's tables one higher than Appendix D does.
///
/// Appendix D captions its three matrices Table 3, Table 4 and Table 5 inside containers
/// whose ids number them 4, 5 and 6. §3.8.3 cites the container ordinal, and the method
/// each citation names — JIS X 4051 or the document's own — is what pins a citation to a
/// caption without anything having to be understood.
fn appendix_d_numbering(document: &Document<'_>) -> Result<String, String> {
    let captions = table_blocks(document, document.section("D")?);
    let notes = note_blocks(document, document.section("3.8.3")?);
    let mut cited = Vec::new();
    for (line, markup) in &notes {
        if !markup
            .contains("#opportunities_for_intercharacter_space_reduction_during_line_adjustment")
        {
            continue;
        }
        let jis = markup.contains("JIS X 4051");
        for (locale, number) in cited_tables(markup) {
            cited.push((*line, locale, number, jis));
        }
    }
    let jis_caption = captions
        .iter()
        .find(|block| block.qualifier == Qualifier::Jis)
        .map(|block| block.caption);
    let plain_caption = captions
        .iter()
        .find(|block| block.qualifier == Qualifier::Plain)
        .map(|block| block.caption);
    let (Some(jis_caption), Some(plain_caption)) = (jis_caption, plain_caption) else {
        return Err(corrected(
            "Appendix D no longer captions both a plain and a JIS matrix",
        ));
    };
    let one_higher = |(_, _, number, jis): &(usize, &str, u32, bool)| {
        let caption = if *jis { jis_caption } else { plain_caption };
        *number == caption.saturating_add(1)
    };
    if cited.is_empty() || !cited.iter().all(one_higher) {
        return Err(corrected(&format!(
            "§3.8.3 makes {count} citation(s) of an Appendix D table and they are no longer \
             all one higher than Appendix D's own captions",
            count = cited.len()
        )));
    }
    Ok(format!(
        "Appendix D captions its three matrices Table {plain_caption}, Table {jis_caption} \
         (the method specified by JIS X 4051) and Table {books} (the method adopted by \
         books), inside containers whose ids number them {ids}. §3.8.3 cites the container \
         ordinal: {citations}. Every one of the {count} citations is one higher than the \
         caption of the matrix it names.",
        books = captions
            .iter()
            .find(|block| block.qualifier == Qualifier::Books)
            .map_or(0, |block| block.caption),
        ids = captions
            .iter()
            .map(|block| block.container.to_string())
            .collect::<Vec<String>>()
            .join(", "),
        citations = grouped(&cited),
        count = cited.len(),
    ))
}

/// The citations of one note, written as one clause per note.
fn grouped(cited: &[(usize, &str, u32, bool)]) -> String {
    let mut by_note: BTreeMap<(usize, u32, bool), Vec<&str>> = BTreeMap::new();
    for (line, locale, number, jis) in cited {
        by_note
            .entry((*line, *number, *jis))
            .or_default()
            .push(locale);
    }
    by_note
        .iter()
        .map(|((line, number, jis), locales)| {
            format!(
                "the note at line {line} names {method} and cites Table {number} in its \
                 {halves}",
                method = if *jis {
                    "JIS X 4051"
                } else {
                    "the method this document adopts"
                },
                halves = match locales.as_slice() {
                    [one] => format!("{one} half"),
                    _ => format!("{joined} halves", joined = locales.join(" and ")),
                }
            )
        })
        .collect::<Vec<String>>()
        .join("; ")
}

/// Step 1 of each of §3.8.3's two priority lists reduces uniformly in English and in
/// proportion to character size in Japanese.
fn reduction_step_divergence(document: &Document<'_>) -> Result<String, String> {
    let proportional: Vec<&Pair> = document
        .pairs
        .iter()
        .filter(|pair| pair.japanese.contains("文字サイズ比"))
        .collect();
    let diverging: Vec<&&Pair> = proportional
        .iter()
        .filter(|pair| !pair.english.to_lowercase().contains("character size"))
        .collect();
    let [first, second] = diverging.as_slice() else {
        return Err(corrected(&format!(
            "{count} paragraph pair(s) qualify a reduction with 文字サイズ比 in Japanese \
             and with no mention of character size in English, not two",
            count = diverging.len()
        )));
    };
    let section = document.section("3.8.3")?;
    let inside = document.pairs_of(section);
    if !inside.iter().any(|pair| pair.line == first.line)
        || !inside.iter().any(|pair| pair.line == second.line)
    {
        return Err(corrected(
            "the two diverging pairs are no longer both in §3.8.3",
        ));
    }
    Ok(format!(
        "The Japanese half of step 1 of each of §3.8.3's two priority lists qualifies the \
         reduction with 文字サイズ比で均等に — evenly, in proportion to character size — \
         where the English half states a uniform amount: `{first}` at line {first_line}, and \
         `{second}` at line {second_line}. On a line of mixed character sizes the two are \
         different operations. They are the only two of the {total} paragraph pairs whose \
         Japanese half says 文字サイズ比 whose English half never says `character size`; \
         the other {agreeing} say it in so many words.",
        first = head(&first.english, 190),
        first_line = first.line,
        second = head(&second.english, 150),
        second_line = second.line,
        total = proportional.len(),
        agreeing = proportional.len().saturating_sub(diverging.len()),
    ))
}

/// §3.9.2 illustrates cl-28 and cl-29 with an `etc.` over an enumeration Appendix A closes.
fn bracket_enumeration(document: &Document<'_>) -> Result<String, String> {
    let examples = class_examples(document)?;
    let counts: BTreeMap<&str, usize> = document
        .tables
        .iter()
        .map(|table| (table.class, table.rows.len()))
        .collect();
    let mut open = Vec::new();
    let mut closed = Vec::new();
    for example in &examples {
        let Some(listed) = counts.get(example.class.as_str()).copied() else {
            continue;
        };
        if example.open && listed == example.shown.chars().count() {
            open.push((example, listed));
        }
        if !example.open {
            closed.push((example, listed));
        }
    }
    let [(first, first_listed), (second, second_listed)] = open.as_slice() else {
        return Err(corrected(&format!(
            "{count} of §3.9.2's examples say `etc.` over an enumeration Appendix A closes \
             at the same count, not two",
            count = open.len()
        )));
    };
    if closed
        .iter()
        .any(|(example, listed)| example.shown.chars().count() != *listed)
    {
        return Err(corrected(
            "an example that carries no `etc.` no longer lists exactly what Appendix A \
             lists, so `etc.` is no longer the document's mark of an open example",
        ));
    }
    Ok(format!(
        "§3.9.2 illustrates {first_class} with `{first_shown} etc.` and {second_class} with \
         `{second_shown} etc.`, while §{first_section} enumerates exactly {first_listed} \
         members and §{second_section} exactly {second_listed}. Of the {illustrated} \
         classes §3.9.2 illustrates, these two are the only ones whose example says `etc.` \
         over an enumeration Appendix A closes at the same count; the {closed} examples \
         that carry no `etc.` each list exactly what Appendix A lists, so the `etc.` is the \
         document's own mark of an open example. Nothing states whether either warichu \
         bracket class is those {first_listed} members or more.",
        first_class = first.class,
        first_shown = first.shown,
        second_class = second.class,
        second_shown = second.shown,
        first_section = section_of(document, &first.class),
        second_section = section_of(document, &second.class),
        illustrated = examples.len(),
        closed = closed.len(),
    ))
}

/// The Appendix A section that enumerates a class.
fn section_of(document: &Document<'_>, class: &str) -> String {
    document
        .tables
        .iter()
        .find(|table| table.class == class)
        .map_or_else(|| class.to_owned(), |table| table.section.clone())
}

/// Appendix A lists one key twice.
fn duplicate_row(document: &Document<'_>) -> Result<String, String> {
    let mut duplicates = Vec::new();
    for table in &document.tables {
        let mut seen: BTreeMap<String, Vec<&TableRow<'_>>> = BTreeMap::new();
        for row in &table.rows {
            seen.entry(row.key()).or_default().push(row);
        }
        for (key, rows) in seen {
            if rows.len() > 1 {
                duplicates.push((table, key, rows));
            }
        }
    }
    let [(table, key, rows)] = duplicates.as_slice() else {
        return Err(corrected(&format!(
            "Appendix A lists {count} key(s) more than once, not one",
            count = duplicates.len()
        )));
    };
    let [first, second] = rows.as_slice() else {
        return Err(corrected(&format!(
            "U+{key} is listed {count} times, not twice",
            count = rows.len()
        )));
    };
    if first.line_text() != second.line_text() {
        return Err(corrected(&format!(
            "the two U+{key} rows of §{section} no longer state the same thing",
            section = table.section
        )));
    }
    Ok(format!(
        "§{section} publishes {rows} rows for {distinct} distinct keys: the row \
         `{row_text}` appears twice, at lines {first_line} and {second_line}, character for \
         character. U+{key} is the only key any of the {tables} Appendix A tables lists \
         twice, over {total} rows, so {class} has one more listing than it has members.",
        section = table.section,
        rows = table.rows.len(),
        distinct = table.rows.len().saturating_sub(1),
        row_text = first.line_text(),
        first_line = first.line,
        second_line = second.line,
        tables = document.tables.len(),
        total = APPENDIX_A_ROWS,
        class = table.class,
    ))
}

/// Three Remarks cells state a line in Japanese that their English half does not.
fn role_only_in_japanese(document: &Document<'_>) -> Result<String, String> {
    let mut japanese_states_more = Vec::new();
    let mut english_states_more = 0usize;
    let mut wrapped = Vec::new();
    for (table, row) in remarks_cells(document) {
        let Some(english) = locale_span(row.remarks(), "en") else {
            continue;
        };
        let Some(japanese) = locale_span(row.remarks(), "ja") else {
            continue;
        };
        let (english, japanese) = (statements(english), statements(japanese));
        if japanese.len() > english.len() {
            japanese_states_more.push((table, row, english, japanese));
        } else if english.len() > japanese.len() {
            english_states_more = english_states_more.saturating_add(1);
        } else if lines(row.remarks(), "ja") > lines(row.remarks(), "en") {
            wrapped.push((table, row));
        }
    }
    if japanese_states_more.len() != 3 || english_states_more != 0 {
        return Err(corrected(&format!(
            "{count} Remarks cell(s) state more in Japanese than in English and \
             {reverse} state more in English, not 3 and 0",
            count = japanese_states_more.len(),
            reverse = english_states_more
        )));
    }
    let Some(control) = decimal_point(document) else {
        return Err(corrected(
            "§A.24's U+002E no longer carries the decimal-point line in both halves, which \
             is the control that makes this a divergence and not a house style",
        ));
    };
    Ok(format!(
        "{cells}. The extra line names the digit-grouping role and exists in Japanese \
         alone. §A.24's U+002E is the control and does not diverge: both its halves carry \
         the decimal-point line, {control}. A line ending in `，` continues into the next \
         rather than starting a second statement, which is the case in {wrapped} of \
         Appendix A's {total} Remarks cells; no cell states more lines in English than in \
         Japanese.",
        cells = japanese_states_more
            .iter()
            .map(|(table, row, english, japanese)| format!(
                "§{section} U+{key} reads `{japanese}` against `{english}` at line {line}",
                section = table.section,
                key = row.key(),
                japanese = japanese.join(" / "),
                english = english.join(" / "),
                line = row.line,
            ))
            .collect::<Vec<String>>()
            .join("; "),
        wrapped = wrapped.len(),
        total = APPENDIX_A_ROWS,
    ))
}

/// §A.24's `U+002E` cell, when it still states the decimal-point line in both halves.
///
/// The control on the defect above. Three cells state a line in Japanese that their
/// English half does not; this one states the same two lines in both, so the divergence is
/// a divergence and not the appendix's house style.
fn decimal_point(document: &Document<'_>) -> Option<String> {
    for (table, row) in remarks_cells(document) {
        if table.class != "cl-24" || row.key() != "002E" {
            continue;
        }
        let english = locale_span(row.remarks(), "en").map(statements)?;
        let japanese = locale_span(row.remarks(), "ja").map(statements)?;
        if english.len() == japanese.len() && english.len() > 1 {
            return Some(format!(
                "`{english}` against `{japanese}`",
                english = english.join(" / "),
                japanese = japanese.join(" / ")
            ));
        }
    }
    None
}

/// Three Remarks cells hold text and no locale span at all.
fn unlocalised_remarks(document: &Document<'_>) -> Result<String, String> {
    let mut bare = Vec::new();
    let mut localized = 0usize;
    for (table, row) in remarks_cells(document) {
        let cell = row.remarks();
        if cell.contains("its-locale-filter-list") {
            if locale_span(cell, "ja").map(text).as_deref() == Some("プロポーショナル") {
                localized = localized.saturating_add(1);
            }
            continue;
        }
        if !text(cell).is_empty() {
            bare.push((table, row));
        }
    }
    let Some((table, _)) = bare.first() else {
        return Err(corrected("no Remarks cell holds text and no locale span"));
    };
    let sole: BTreeSet<String> = bare.iter().map(|(_, row)| text(row.remarks())).collect();
    if bare.len() != 3 || sole.len() != 1 || bare.iter().any(|(each, _)| each.class != table.class)
    {
        return Err(corrected(&format!(
            "{count} Remarks cell(s) hold text and no locale span, stating {shapes} \
             distinct thing(s); this defect is three cells of one class stating one thing",
            count = bare.len(),
            shapes = sole.len()
        )));
    }
    Ok(format!(
        "{cells} hold the bare string `{shape}` and no `its-locale-filter-list` span. The \
         other {localized} cells carrying that remark write it as two spans, one per \
         locale. They are the only {count} of Appendix A's {total} Remarks cells that hold \
         text and no locale span, so an English-locale extraction yields an empty remark \
         for {count} rows that mean proportionally-spaced.",
        cells = bare
            .iter()
            .map(|(table, row)| format!(
                "§{section} U+{key} at line {line}",
                section = table.section,
                key = row.key(),
                line = row.line
            ))
            .collect::<Vec<String>>()
            .join(", "),
        shape = sole.iter().next().map_or("", String::as_str),
        localized = localized,
        count = bare.len(),
        total = APPENDIX_A_ROWS,
    ))
}

/// Every Remarks cell of Appendix A, with the table it belongs to.
fn remarks_cells<'a>(document: &'a Document<'a>) -> Vec<(&'a ClassTable<'a>, &'a TableRow<'a>)> {
    document
        .tables
        .iter()
        .flat_map(|table| table.rows.iter().map(move |row| (table, row)))
        .collect()
}

/// Every table ordinal in an anchor id or a filename is one higher than the printed number.
fn legend_numbering(document: &Document<'_>) -> Result<String, String> {
    let mut legends = Vec::new();
    for heading in &document.headings {
        if !heading.anchor.starts_with("legend_of_table") {
            continue;
        }
        let anchor = numerals(heading.anchor);
        let printed = numerals(&heading.title);
        if anchor.len() != printed.len()
            || anchor
                .iter()
                .zip(&printed)
                .any(|(slug, shown)| *slug != shown.saturating_add(1))
        {
            return Err(corrected(&format!(
                "§{number}'s anchor `{anchor_id}` no longer numbers `{title}` one higher",
                number = heading.number,
                anchor_id = heading.anchor,
                title = heading.title
            )));
        }
        legends.push(heading);
    }
    if legends.is_empty() {
        return Err(corrected("the rendering publishes no legend section"));
    }
    // Every appendix that publishes a matrix, addressed by its rendered letter: B, C, D
    // and E. The scan is over every one-letter section rather than over those four by
    // name, so an appendix that gained a matrix is measured too.
    let all: Vec<TableBlock> = document
        .headings
        .iter()
        .filter(|heading| heading.number.chars().count() == 1)
        .flat_map(|heading| table_blocks(document, heading))
        .collect();
    if all.is_empty()
        || all
            .iter()
            .any(|block| block.container != block.caption.saturating_add(1))
        || all
            .iter()
            .any(|block| block.file != block.caption.saturating_add(1))
    {
        return Err(corrected(
            "a matrix container id or PDF filename no longer numbers its caption one higher",
        ));
    }
    Ok(format!(
        "Every table ordinal in an anchor id or a PDF filename is one higher than the \
         number the rendering prints. {legends}. The {count} containers `table{first}_pdf` \
         through `table{last}_pdf` hold the captions Table {first_caption} through Table \
         {last_caption}, and link `tables/table_en{first}.pdf` through \
         `tables/table_en{last}.pdf`. A tool keyed on an anchor id or a filename misnumbers \
         every one of the {count} matrices.",
        legends = legends
            .iter()
            .map(|heading| format!(
                "§{number}'s anchor is `{anchor}` under the heading `{title}`",
                number = heading.number,
                anchor = heading.anchor,
                title = heading.title
            ))
            .collect::<Vec<String>>()
            .join("; "),
        count = all.len(),
        first = all.first().map_or(0, |block| block.container),
        last = all.last().map_or(0, |block| block.container),
        first_caption = all.first().map_or(0, |block| block.caption),
        last_caption = all.last().map_or(0, |block| block.caption),
    ))
}

/// §B.2 note 7's two halves link different character classes.
fn note_class_divergence(document: &Document<'_>) -> Result<String, String> {
    let notes = document.notes("B.2")?;
    let mut diverging = Vec::new();
    for note in &notes {
        let (english, japanese) = (note.english.classes(), note.japanese.classes());
        if english != japanese {
            diverging.push((note, english, japanese));
        }
    }
    let [(note, english, japanese)] = diverging.as_slice() else {
        return Err(corrected(&format!(
            "{count} of §B.2's notes link different classes in their two halves, not one",
            count = diverging.len()
        )));
    };
    let only_japanese: Vec<&String> = japanese.difference(english).collect();
    let only_english: Vec<&String> = english.difference(japanese).collect();
    if only_japanese.is_empty() || !only_english.is_empty() {
        return Err(corrected(
            "§B.2's one diverging note no longer omits classes from its English half alone",
        ));
    }
    Ok(format!(
        "§B.2 note {ordinal}'s Japanese half links {missing} where its English half does \
         not, at line {line}. The two halves link {shared} in common. It is the only one of \
         §B.2's {total} notes whose halves link different character classes, so a reader \
         taking the English half alone extends the ruby permission over {count} fewer \
         classes than the Japanese half grants it.",
        ordinal = note.ordinal,
        missing = only_japanese
            .iter()
            .map(|class| class.as_str())
            .collect::<Vec<&str>>()
            .join(" and "),
        line = note.line,
        shared = english
            .intersection(japanese)
            .map(String::as_str)
            .collect::<Vec<&str>>()
            .join(", "),
        total = notes.len(),
        count = only_japanese.len(),
    ))
}

/// §B.2 note 11 names the simple-ruby complex in a note about the jukugo-ruby complex.
fn simple_ruby_misnomer(document: &Document<'_>) -> Result<String, String> {
    let notes = document.notes("B.2")?;
    let found: Vec<&Note> = notes
        .iter()
        .filter(|note| {
            note.english.text.contains("simple-ruby")
                && !note.japanese.text.contains("熟語ルビ以外")
        })
        .collect();
    let [note] = found.as_slice() else {
        return Err(corrected(&format!(
            "{count} of §B.2's notes name the simple-ruby complex in English without naming \
             熟語ルビ以外 in Japanese, not one",
            count = found.len()
        )));
    };
    let document_wide = document
        .pairs
        .iter()
        .filter(|pair| {
            pair.english.contains("simple-ruby") && !pair.japanese.contains("熟語ルビ以外")
        })
        .count();
    if document_wide != 1 {
        return Err(corrected(&format!(
            "{document_wide} paragraph pairs in the rendering name the simple-ruby complex \
             in English alone, not one"
        )));
    }
    Ok(format!(
        "§B.2 note {ordinal} is about the jukugo-ruby character complex: its Japanese half \
         names 熟語ルビ throughout and never names 熟語ルビ以外, the document's own term for \
         the simple-ruby complex. Its English second sentence reads `{sentence}` at line \
         {line}, which is verbatim the second sentence of the note above it and names the \
         wrong complex. It is the only paragraph in the rendering whose English half names \
         the simple-ruby complex where its Japanese half does not name 熟語ルビ以外.",
        ordinal = note.ordinal,
        sentence = sentences(&note.english.text)
            .into_iter()
            .find(|sentence| sentence.contains("simple-ruby"))
            .unwrap_or_default()
            .trim(),
        line = note.line,
    ))
}

/// The English half of a §D.2 note drops the line-end qualification its Japanese half states.
///
/// The reading this replaces said §D.2 note 5 contradicts notes 1, 2 and 3 on a priority
/// ordinal, and the document does not contain that contradiction. The two ordinals are given
/// for two different reductions, and §3.8.3 lists both: its step 3 is 行末に配置する中点類 —
/// the middle dot *placed at the line end*, whose spaces are set solid together — and its
/// step 4 is 行中の中点類, the one in the middle of a line. Note 5 is the first, notes 1 to 3
/// are the second, and each ordinal is the right one for its step.
///
/// What is really wrong is narrower and is a defect of one locale: note 5's Japanese half
/// opens 表3に限るが，行末に配置する中点類（cl-05）… and its English half states no position at
/// all, so an English-only reader meets two ordinals for what looks like one reduction. §3.8.3
/// step 3's English half makes the same omission, which is what shows this is the rendering
/// dropping a qualifier rather than the specification stating two rules.
///
/// Note 4 is the control the measurement rests on: it carries 行末 in both halves, so the
/// rendering does state the qualifier in English where the English half is complete. A
/// correction upstream therefore clears this row rather than leaving it quoting a sentence
/// that no longer omits anything.
fn line_end_qualifier_omitted(document: &Document<'_>) -> Result<String, String> {
    let notes = document.notes("D.2")?;
    let positioned: Vec<&Note> = notes
        .iter()
        .filter(|note| note.japanese.text.contains(LINE_END_JA))
        .collect();
    let omitted: Vec<&Note> = positioned
        .iter()
        .copied()
        .filter(|note| !states_line_end(&note.english.text))
        .collect();
    let control: Vec<&Note> = positioned
        .iter()
        .copied()
        .filter(|note| states_line_end(&note.english.text))
        .collect();
    let [note] = omitted.as_slice() else {
        return Err(corrected(&format!(
            "{count} of §D.2's notes state 行末 in Japanese and no position in English, not \
             one",
            count = omitted.len()
        )));
    };
    let [witness] = control.as_slice() else {
        return Err(corrected(&format!(
            "{count} of §D.2's notes state the line end in both halves, not one, so nothing \
             shows that the rendering does write 行末 in English where the English half is \
             complete",
            count = control.len()
        )));
    };
    let (line_end, mid_line) = middle_dot_steps(document)?;
    Ok(format!(
        "§D.2 note {ordinal}'s English half drops the position its Japanese half states. The \
         Japanese reads `{japanese}` at line {japanese_line} — 表3に限るが，行末に配置する, \
         Table 3 only, for a middle dot *placed at the line end* — and the English reads \
         `{english}` at line {line}, which states no position and tests false for every one \
         of {probes:?}. Read alone it therefore appears to give the reduction of the \
         conditional quarter em space accompanying middle dots (cl-05) a second priority \
         ordinal, against the fourth that §D.2's other notes state. §3.8.3 shows there is no \
         second: its step {line_end_step} is the line-end reduction, 行末に配置する中点類（cl-05）\
         の前及び後ろの四分アキを一緒にベタ組にする at line {line_end_line}, and its step \
         {mid_line_step} is 行中の中点類（cl-05）の前後の四分アキ at line {mid_line_line}. The \
         English half of §3.8.3 step {line_end_step} omits the same qualifier. Note {control} \
         is the control: it carries 行末 in both halves, so the omission is this note's and \
         not the rendering's convention. §D.2 publishes {total} notes and {positioned} of \
         them state 行末 in Japanese.",
        ordinal = note.ordinal,
        japanese = note.japanese.text.trim(),
        japanese_line = note.line,
        english = note.english.text.trim(),
        line = note.line,
        probes = POSITIONS,
        line_end_step = line_end.0,
        line_end_line = line_end.1,
        mid_line_step = mid_line.0,
        mid_line_line = mid_line.1,
        control = witness.ordinal,
        total = notes.len(),
        positioned = positioned.len(),
    ))
}

/// 行末, the word the Japanese half states the position with.
const LINE_END_JA: &str = "行末";

/// Every way the rendering writes that position in English, so an omission is measured
/// against the whole vocabulary rather than against one phrasing.
const POSITIONS: [&str; 4] = ["line end", "end of a line", "end of the line", "at the end"];

/// Whether an English half states the position at all.
fn states_line_end(text: &str) -> bool {
    POSITIONS.iter().any(|probe| text.contains(probe))
}

/// One list item of §3.8.3: its ordinal in the list, and the line it starts on.
type Step = (usize, usize);

/// §3.8.3's two middle-dot reduction steps, each as its ordinal in the list and its line.
///
/// They are what makes the note's two ordinals two answers to two questions rather than two
/// answers to one, so the row measures them rather than asserting them.
fn middle_dot_steps(document: &Document<'_>) -> Result<(Step, Step), String> {
    let steps = document.notes("3.8.3")?;
    let of = |mark: &str| -> Option<(usize, usize)> {
        let found: Vec<&Note> = steps
            .iter()
            .filter(|step| {
                step.japanese.text.contains("中点類") && step.japanese.text.contains(mark)
            })
            .collect();
        match found.as_slice() {
            [step] => Some((step.ordinal, step.line)),
            _ => None,
        }
    };
    let (Some(line_end), Some(mid_line)) = (of("行末に配置する"), of("行中の")) else {
        return Err(corrected(
            "§3.8.3 does not list exactly one line-end middle-dot reduction and exactly one \
             mid-line one, so the two ordinals §D.2 states can no longer be told apart",
        ));
    };
    Ok((line_end, mid_line))
}

/// Every table a passage names.
fn tables_named(text: &str) -> BTreeSet<u32> {
    text.match_indices("Table ")
        .filter_map(|(at, _)| text.get(at.saturating_add("Table ".len())..))
        .filter_map(|tail| tail.chars().next())
        .filter_map(|digit| digit.to_digit(10))
        .collect()
}

/// The one member of a set, when it has exactly one.
fn sole(numbers: &BTreeSet<u32>) -> Option<u32> {
    match numbers.iter().copied().collect::<Vec<u32>>().as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

// ---------------------------------------------------------------------------------------
// The shapes the detectors read
// ---------------------------------------------------------------------------------------

/// Which of Appendix D's three methods a matrix or a citation names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Qualifier {
    /// The method this document adopts, which its caption qualifies with nothing.
    Plain,
    /// The method JIS X 4051 specifies.
    Jis,
    /// The method books adopt.
    Books,
}

/// One `See "Table N …" (PDF)` block: the ordinals its three names carry.
///
/// A matrix is named three times over — by the id of the `div` that holds the link, by the
/// table number the caption prints, and by the filename the link points at — and only the
/// second of the three is the number a reader sees.
#[derive(Debug)]
struct TableBlock {
    /// The ordinal in the container's `table<N>_pdf` id.
    container: u32,
    /// The table number the caption prints.
    caption: u32,
    /// The ordinal in the linked filename, which both locales agree on.
    file: u32,
    /// Which of Appendix D's methods the caption names.
    qualifier: Qualifier,
}

/// Every matrix block of a section, in document order.
fn table_blocks<'a>(document: &'a Document<'a>, heading: &Heading<'a>) -> Vec<TableBlock> {
    let body = document.body(heading);
    let mut found = Vec::new();
    for element in elements(body, "div") {
        let Some(id) = attribute(element.attributes, "id") else {
            continue;
        };
        let Some(container) = id
            .strip_prefix("table")
            .and_then(|rest| rest.strip_suffix("_pdf"))
            .and_then(|digits| digits.parse::<u32>().ok())
        else {
            continue;
        };
        let english = Half::read(element.content, "en");
        let japanese = Half::read(element.content, "ja");
        let Some(caption) = tables_named(&english.text).iter().next().copied() else {
            continue;
        };
        let files: BTreeSet<u32> = LOCALES
            .iter()
            .filter_map(|locale| {
                let half = if *locale == "en" { &english } else { &japanese };
                half.markup
                    .split_once(&format!("tables/table_{locale}"))
                    .and_then(|(_, tail)| tail.split_once(".pdf"))
                    .and_then(|(digits, _)| digits.parse::<u32>().ok())
            })
            .collect();
        let Some(file) = sole(&files) else { continue };
        found.push(TableBlock {
            container,
            caption,
            file,
            qualifier: qualifier(&english.text),
        });
    }
    found
}

/// Which method a caption or a citation names.
fn qualifier(text: &str) -> Qualifier {
    if text.contains("JIS X 4051") {
        Qualifier::Jis
    } else if text.contains("books") {
        Qualifier::Books
    } else {
        Qualifier::Plain
    }
}

/// Every `<div class="note">` of a section, with the line it starts on.
fn note_blocks<'a>(document: &'a Document<'a>, heading: &Heading<'a>) -> Vec<(usize, &'a str)> {
    let (start, _) = heading.body;
    let body = document.body(heading);
    let mut found = Vec::new();
    let mut cursor = 0usize;
    while let Some(at) = body
        .get(cursor..)
        .and_then(|rest| rest.find("<div class=\"note\""))
    {
        let opened = cursor.saturating_add(at);
        let end = body
            .get(opened..)
            .and_then(|tail| tail.find("</aside></div>"))
            .map_or(body.len(), |length| opened.saturating_add(length));
        found.push((
            document.line_of(start.saturating_add(opened)),
            body.get(opened..end).unwrap_or_default(),
        ));
        cursor = end.max(opened.saturating_add(1));
    }
    found
}

/// The table numbers a note cites, with the locale each citation is written in.
fn cited_tables(markup: &str) -> Vec<(&'static str, u32)> {
    let mut found = Vec::new();
    let english = Half::read(markup, "en");
    for (at, _) in english.text.match_indices("the Table ") {
        if let Some(number) = english
            .text
            .get(at.saturating_add("the Table ".len())..)
            .and_then(|tail| tail.chars().next())
            .and_then(|digit| digit.to_digit(10))
        {
            found.push(("English", number));
        }
    }
    let japanese = Half::read(markup, "ja");
    for (at, _) in japanese.text.match_indices("の表") {
        if let Some(number) = japanese
            .text
            .get(at.saturating_add("の表".len())..)
            .and_then(|tail| tail.chars().next())
            .and_then(|digit| digit.to_digit(10))
        {
            found.push(("Japanese", number));
        }
    }
    found
}

/// One class of §3.9.2, with the example the section illustrates it by.
#[derive(Debug)]
struct Example {
    /// The class, written `cl-28`.
    class: String,
    /// The characters the example shows.
    shown: String,
    /// Whether the example says `etc.`, which is the document's mark of an open example.
    open: bool,
}

/// Every class §3.9.2 illustrates by example.
fn class_examples(document: &Document<'_>) -> Result<Vec<Example>, String> {
    let section = document.section("3.9.2")?;
    let mut found = Vec::new();
    for (_, item) in list_items(document.body(section)) {
        let paragraphs = elements(item, "p");
        // The item's own first English paragraph is the class name, `Warichu opening
        // brackets (cl-28)`. Everything after it — the notes — names other classes, so the
        // class is read from the name and never from the last citation in the item.
        let name = paragraphs
            .iter()
            .find(|element| locale_of(element.attributes) == Some("en"))
            .map(|element| text(element.content))
            .unwrap_or_default();
        let Some(class) = name
            .rsplit_once("(cl-")
            .and_then(|(_, tail)| two_digits(tail))
        else {
            continue;
        };
        let Some(example) = paragraphs
            .iter()
            .find(|element| attribute(element.attributes, "class") == Some("asis"))
        else {
            continue;
        };
        let open = example.content.contains("etc.");
        let mut shown = String::new();
        for character in text(&drop_locale(&drop_locale(example.content, "en"), "ja")).chars() {
            if !character.is_whitespace() {
                shown.push(character);
            }
        }
        found.push(Example {
            class: format!("cl-{class}"),
            shown,
            open,
        });
    }
    if found.is_empty() {
        return Err(corrected("§3.9.2 illustrates no class by example"));
    }
    Ok(found)
}

// ---------------------------------------------------------------------------------------
// The scanner
// ---------------------------------------------------------------------------------------

/// One tag of a fragment.
#[derive(Debug, Clone, Copy)]
struct Tag<'a> {
    /// The element name, lowercase as the rendering writes it.
    name: &'a str,
    /// Whether it is a closing tag.
    closing: bool,
    /// The byte offset of the `<`.
    start: usize,
    /// The byte offset just past the `>`.
    end: usize,
}

/// One element, with its attributes and its content.
#[derive(Debug, Clone, Copy)]
struct Element<'a> {
    /// The attribute text of the opening tag.
    attributes: &'a str,
    /// Everything between the opening and closing tags.
    content: &'a str,
}

/// Every tag of a fragment, in document order.
fn tags(fragment: &str) -> Vec<Tag<'_>> {
    let mut found = Vec::new();
    let mut cursor = 0usize;
    while let Some(at) = fragment.get(cursor..).and_then(|rest| rest.find('<')) {
        let start = cursor.saturating_add(at);
        let Some(length) = fragment.get(start..).and_then(|head| head.find('>')) else {
            break;
        };
        let inside = fragment.get(start.saturating_add(1)..start.saturating_add(length));
        let inside = inside.unwrap_or_default();
        let closing = inside.starts_with('/');
        let body = inside.strip_prefix('/').unwrap_or(inside);
        let split = body
            .find(char::is_whitespace)
            .unwrap_or(body.len())
            .min(body.len());
        cursor = start.saturating_add(length).saturating_add(1);
        found.push(Tag {
            name: body.get(..split).unwrap_or_default(),
            closing,
            start,
            end: cursor,
        });
    }
    found
}

/// Every `<name …>…</name>` of a fragment, for a name the rendering never nests.
///
/// `p`, `td`, `tr` and the matrix `div`s are read this way; `li`, which does nest, is read
/// by [`list_items`], and a locale span, which also nests, by [`drop_locale`].
fn elements<'a>(fragment: &'a str, name: &str) -> Vec<Element<'a>> {
    let (open, close) = (format!("<{name}"), format!("</{name}>"));
    let mut found = Vec::new();
    let mut cursor = 0usize;
    while let Some(at) = fragment.get(cursor..).and_then(|rest| rest.find(&open)) {
        let start = cursor.saturating_add(at);
        let after = start.saturating_add(open.len());
        let delimiter = fragment.get(after..).and_then(|tail| tail.chars().next());
        let Some(delimiter) = delimiter else { break };
        if delimiter != '>' && !delimiter.is_whitespace() {
            cursor = after;
            continue;
        }
        let Some(head) = fragment.get(start..).and_then(|tail| tail.find('>')) else {
            break;
        };
        let opened = start.saturating_add(head);
        let content_start = opened.saturating_add(1);
        let Some(length) = fragment
            .get(content_start..)
            .and_then(|tail| tail.find(&close))
        else {
            break;
        };
        let content_end = content_start.saturating_add(length);
        found.push(Element {
            attributes: fragment.get(after..opened).unwrap_or_default(),
            content: fragment.get(content_start..content_end).unwrap_or_default(),
        });
        cursor = content_end.saturating_add(close.len());
    }
    found
}

/// The direct items of the first list in a fragment, each with its offset.
///
/// Nesting is counted, because the published lists contain lists and tables: a count that
/// read a nested item as an item of the list above would number everything after it wrongly.
fn list_items(fragment: &str) -> Vec<(usize, &str)> {
    let mut found = Vec::new();
    let mut depth = 0usize;
    let mut list: Option<usize> = None;
    let mut opened: Option<usize> = None;
    for tag in tags(fragment) {
        match tag.name {
            "ol" | "ul" | "table" if !tag.closing => {
                depth = depth.saturating_add(1);
                if list.is_none() && tag.name != "table" {
                    list = Some(depth);
                }
            },
            "ol" | "ul" | "table" if tag.closing => {
                if list == Some(depth) {
                    push_item(fragment, &mut opened, tag.start, &mut found);
                    return found;
                }
                depth = depth.saturating_sub(1);
            },
            "li" if list == Some(depth) => {
                if tag.closing {
                    push_item(fragment, &mut opened, tag.start, &mut found);
                } else if opened.is_none() {
                    opened = Some(tag.end);
                }
            },
            _ => {},
        }
    }
    found
}

/// Close the item opened at `opened`, if one is open.
fn push_item<'a>(
    fragment: &'a str,
    opened: &mut Option<usize>,
    at: usize,
    found: &mut Vec<(usize, &'a str)>,
) {
    if let Some(start) = opened.take() {
        found.push((start, fragment.get(start..at).unwrap_or_default()));
    }
}

/// Every heading of the rendering, with the body it opens.
fn headings(html: &str) -> Vec<Heading<'_>> {
    let mut starts = Vec::new();
    for tag in tags(html) {
        if tag.closing || !is_heading(tag.name) {
            continue;
        }
        starts.push(tag);
    }
    let mut found = Vec::new();
    for (index, tag) in starts.iter().enumerate() {
        let close = format!("</{name}>", name = tag.name);
        let Some(length) = html.get(tag.end..).and_then(|tail| tail.find(&close)) else {
            continue;
        };
        let content_end = tag.end.saturating_add(length);
        let content = html.get(tag.end..content_end).unwrap_or_default();
        let attributes = html
            .get(tag.start.saturating_add(1).saturating_add(tag.name.len())..tag.end)
            .unwrap_or_default();
        let body_start = content_end.saturating_add(close.len());
        let body_end = starts
            .get(index.saturating_add(1))
            .map_or(html.len(), |next| next.start);
        found.push(Heading {
            anchor: attribute(attributes, "id").unwrap_or_default(),
            number: collapse(&text(between(content, "<bdi class=\"secno\">", "</bdi>")))
                .trim_end_matches('.')
                .to_owned(),
            title: locale_span(content, "en").map_or_else(|| text(content), text),
            body: (body_start, body_end),
        });
    }
    found
}

/// Whether a tag name is one of the six heading levels.
fn is_heading(name: &str) -> bool {
    matches!(name, "h1" | "h2" | "h3" | "h4" | "h5" | "h6")
}

/// The Appendix A class tables, in the order the appendix publishes them.
fn class_tables<'a>(html: &'a str, headings: &[Heading<'a>]) -> Vec<ClassTable<'a>> {
    let mut found = Vec::new();
    for heading in headings {
        if !heading.anchor.starts_with("cl-") {
            continue;
        }
        let (start, end) = heading.body;
        let body = html.get(start..end).unwrap_or_default();
        let mut rows = Vec::new();
        for group in elements(body, "tbody") {
            for element in elements(group.content, "tr") {
                let offset = offset_of(html, element.content);
                rows.push(TableRow {
                    line: line_at(html, offset),
                    cells: elements(element.content, "td")
                        .into_iter()
                        .map(|cell| cell.content)
                        .collect(),
                });
            }
        }
        if rows.is_empty() {
            continue;
        }
        found.push(ClassTable {
            class: heading.anchor,
            section: heading.number.clone(),
            rows,
        });
    }
    found
}

/// The offset of a borrowed slice inside the string it was taken from.
fn offset_of(whole: &str, part: &str) -> usize {
    let (base, inner) = (whole.as_ptr() as usize, part.as_ptr() as usize);
    inner.saturating_sub(base)
}

/// The one-based line an offset sits on, counted directly.
fn line_at(html: &str, offset: usize) -> usize {
    html.get(..offset)
        .unwrap_or_default()
        .matches('\n')
        .count()
        .saturating_add(1)
}

/// The byte offset each line of a text starts at.
fn line_starts(text: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    for (at, _) in text.match_indices('\n') {
        starts.push(at.saturating_add(1));
    }
    starts
}

/// Every English/Japanese paragraph pair, as `(offset of the English half, English text,
/// Japanese text)`.
fn pairs(html: &str) -> Vec<(usize, String, String)> {
    let mut halves = Vec::new();
    for element in elements(html, "p") {
        let Some(locale) = locale_of(element.attributes) else {
            continue;
        };
        halves.push((
            offset_of(html, element.content),
            locale,
            text(&drop_locale(element.content, other_locale(locale))),
        ));
    }
    let mut found = Vec::new();
    let mut index = 0usize;
    while let (Some(first), Some(second)) = (halves.get(index), halves.get(index.saturating_add(1)))
    {
        if first.1 == "en" && second.1 == "ja" {
            found.push((first.0, first.2.clone(), second.2.clone()));
            index = index.saturating_add(2);
        } else {
            index = index.saturating_add(1);
        }
    }
    found
}

/// Which locale an element's attributes mark it with.
fn locale_of(attributes: &str) -> Option<&'static str> {
    LOCALES
        .into_iter()
        .find(|locale| attributes.contains(&format!("its-locale-filter-list=\"{locale}\"")))
}

/// The other of the two locales.
fn other_locale(locale: &str) -> &'static str {
    if locale == "en" { "ja" } else { "en" }
}

/// The content of the first span of a locale, with the other locale's spans removed.
fn locale_span<'a>(fragment: &'a str, locale: &str) -> Option<&'a str> {
    let marker = format!("<span its-locale-filter-list=\"{locale}\"");
    let at = fragment.find(&marker)?;
    let opened = fragment
        .get(at..)?
        .find('>')?
        .saturating_add(at)
        .saturating_add(1);
    let end = span_end(fragment, at)?;
    fragment.get(opened..end.saturating_sub("</span>".len()))
}

/// A fragment with every span of one locale removed, nesting and all.
fn drop_locale(fragment: &str, locale: &str) -> String {
    let marker = format!("<span its-locale-filter-list=\"{locale}\"");
    let mut out = String::new();
    let mut cursor = 0usize;
    while let Some(at) = fragment.get(cursor..).and_then(|rest| rest.find(&marker)) {
        let start = cursor.saturating_add(at);
        out.push_str(fragment.get(cursor..start).unwrap_or_default());
        let Some(end) = span_end(fragment, start) else {
            break;
        };
        cursor = end;
    }
    out.push_str(fragment.get(cursor..).unwrap_or_default());
    out
}

/// The offset just past the `</span>` closing the span that opens at `start`.
fn span_end(fragment: &str, start: usize) -> Option<usize> {
    let mut cursor = start.saturating_add("<span".len());
    let mut depth = 1usize;
    while depth > 0 {
        let rest = fragment.get(cursor..)?;
        let close = rest.find("</span>")?;
        match rest.find("<span") {
            Some(open) if open < close => {
                depth = depth.saturating_add(1);
                cursor = cursor.saturating_add(open).saturating_add("<span".len());
            },
            _ => {
                depth = depth.saturating_sub(1);
                cursor = cursor.saturating_add(close).saturating_add("</span>".len());
            },
        }
    }
    Some(cursor)
}

/// The text a fragment of markup renders as: tags dropped, entities decoded, one line.
fn text(markup: &str) -> String {
    let mut out = String::new();
    let mut cursor = 0usize;
    while let Some(at) = markup.get(cursor..).and_then(|rest| rest.find('<')) {
        let start = cursor.saturating_add(at);
        out.push_str(markup.get(cursor..start).unwrap_or_default());
        let Some(length) = markup.get(start..).and_then(|tail| tail.find('>')) else {
            break;
        };
        cursor = start.saturating_add(length).saturating_add(1);
    }
    out.push_str(markup.get(cursor..).unwrap_or_default());
    for (entity, character) in [
        ("&nbsp;", " "),
        ("&lt;", "<"),
        ("&gt;", ">"),
        ("&quot;", "\""),
        ("&amp;", "&"),
    ] {
        out = out.replace(entity, character);
    }
    collapse(&out)
}

/// The text between two markers, or the empty string.
fn between<'a>(text: &'a str, open: &str, close: &str) -> &'a str {
    text.split_once(open)
        .and_then(|(_, rest)| rest.split_once(close))
        .map_or("", |(inside, _)| inside)
}

/// One line, with every run of whitespace collapsed to a single space.
fn collapse(text: &str) -> String {
    let mut out = String::new();
    for word in text.split_whitespace() {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    out
}

/// The statements one half of a Remarks cell makes.
///
/// The published cells separate statements with `<br>`. One Japanese cell wraps a single
/// statement across two lines after an ideographic comma, so a line ending in `，`
/// continues into the next rather than starting a second statement — which is what makes
/// three cells a divergence and the fourth a line break.
fn statements(markup: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for line in broken_lines(markup) {
        match found.last_mut() {
            Some(previous) if previous.ends_with('，') => previous.push_str(&line),
            _ => found.push(line),
        }
    }
    found
}

/// The `<br>`-separated lines of a fragment, each as text, the empty ones dropped.
///
/// A split on `<br` leaves the rest of the tag — `>`, `/>` or ` />` — at the head of every
/// piece but the first, so each piece is taken from past its own `>`.
fn broken_lines(markup: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (index, piece) in markup.split("<br").enumerate() {
        let piece = if index == 0 {
            piece
        } else {
            piece.split_once('>').map_or("", |(_, rest)| rest)
        };
        let line = text(piece);
        if !line.is_empty() {
            found.push(line);
        }
    }
    found
}

/// How many `<br>`-separated lines one half of a Remarks cell holds.
fn lines(cell: &str, locale: &str) -> usize {
    locale_span(cell, locale).map_or(0, |half| broken_lines(half).len())
}

/// A text split into sentences at `. `.
fn sentences(text: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut start = 0usize;
    for (at, _) in text.match_indices(". ") {
        let end = at.saturating_add(1);
        if let Some(sentence) = text.get(start..end) {
            found.push(sentence);
        }
        start = end.saturating_add(1);
    }
    if let Some(last) = text.get(start..).filter(|rest| !rest.trim().is_empty()) {
        found.push(last);
    }
    found
}

/// Every decimal numeral a string holds, in order.
fn numerals(text: &str) -> Vec<u32> {
    let mut found = Vec::new();
    let mut current: Option<u32> = None;
    for character in text.chars() {
        match character.to_digit(10) {
            Some(digit) => {
                current = Some(
                    current
                        .unwrap_or(0)
                        .saturating_mul(10)
                        .saturating_add(digit),
                );
            },
            None => {
                if let Some(number) = current.take() {
                    found.push(number);
                }
            },
        }
    }
    found.extend(current);
    found
}

/// The two-digit ordinal a string opens with.
fn two_digits(text: &str) -> Option<&str> {
    let digits = text.get(..2)?;
    digits
        .bytes()
        .all(|byte| byte.is_ascii_digit())
        .then_some(digits)
}

/// One attribute's value, or `None` when the element does not carry it.
fn attribute<'a>(attributes: &'a str, name: &str) -> Option<&'a str> {
    let marker = format!("{name}=\"");
    let at = attributes.find(&marker)?;
    let rest = attributes.get(at.saturating_add(marker.len())..)?;
    rest.split_once('"').map(|(value, _)| value)
}

/// The first `width` characters of a text, with an ellipsis when it was cut.
fn head(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_owned();
    }
    let mut out: String = text.chars().take(width).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::{
        Document, RECORDED, collapse, drop_locale, elements, head, list_items, numerals, row,
        sentences, statements, text,
    };
    use crate::shared;
    use std::fs;

    /// The vendored rendering these tests read.
    fn snapshot() -> String {
        let root = shared::workspace_root().expect("the workspace root");
        fs::read_to_string(root.join("spec/snapshot/index.html")).expect("the vendored snapshot")
    }

    #[test]
    fn every_recorded_defect_is_still_in_the_published_document() {
        // The point of the file. Each detector is a measurement over the rendering rather
        // than a sentence in a constant, so this is the check that the twelve are still
        // there — and the one that fails, loudly, when a revision fixes one.
        let html = snapshot();
        let document = Document::read(&html).expect("the rendering is the shape this module reads");
        let mut lost = Vec::new();
        for defect in &RECORDED {
            match (defect.detect)(&document) {
                Ok(evidence) => assert!(
                    !evidence.is_empty(),
                    "{id} composed no evidence",
                    id = defect.id
                ),
                Err(reason) => lost.push(format!("{id}: {reason}", id = defect.id)),
            }
        }
        assert!(lost.is_empty(), "{lost:#?}");
    }

    #[test]
    fn the_recorded_identifiers_are_distinct_and_the_sites_are_addresses() {
        let mut seen = std::collections::BTreeSet::new();
        for defect in &RECORDED {
            assert!(seen.insert(defect.id), "`{id}` twice", id = defect.id);
            assert!(
                !defect.site.starts_with('§'),
                "a site is written `3.1.6`, and the § is the reader's: `{site}`",
                site = defect.site
            );
            assert!(
                !defect.treatment.is_empty(),
                "{id} says nothing about what this repository does with it",
                id = defect.id
            );
        }
    }

    #[test]
    fn a_defect_the_document_no_longer_states_fails_the_derivation() {
        // The proof that this is a derivation and not a list. The fixture corrects the one
        // defect a single edit can correct — the duplicated cl-19 row — and the detector
        // must refuse rather than reprint what it was told.
        let html = snapshot();
        let first = html.find("<td>216B</td>").expect("the duplicated key");
        let second = html
            .get(first.saturating_add(1)..)
            .and_then(|tail| tail.find("<td>216B</td>"))
            .map(|at| at.saturating_add(first).saturating_add(1))
            .expect("the second listing of it");
        let mut corrected = html.clone();
        corrected.replace_range(
            second..second.saturating_add("<td>216B</td>".len()),
            "<td>216C</td>",
        );
        assert_ne!(corrected, html, "the fixture edits the row it names");

        let document = Document::read(&corrected).expect("the fixture is still readable");
        let defect = RECORDED
            .iter()
            .find(|each| each.id == "cl-19-duplicate-u216b")
            .expect("the recorded duplicate");
        let refused = (defect.detect)(&document).expect_err("a corrected document is a failure");
        assert!(
            refused.contains("lists 0 key(s) more than once"),
            "the refusal states what it measured: {refused}"
        );
        assert!(
            refused.contains("RECORDED_DEFECTS"),
            "and what to do about it: {refused}"
        );
    }

    #[test]
    fn supplying_the_missing_qualifier_upstream_clears_the_line_end_row() {
        // The same proof for the row whose reading M0-b replaced. Until then this row said
        // §D.2 note 5 contradicts notes 1 to 3 on a priority ordinal, and it survived this
        // very substitution — the corrected sentence was still reprinted as evidence of a
        // contradiction it no longer even appeared to state, which is how the reading was
        // found to be measuring something the document does not say. §3.8.3 lists the
        // line-end reduction and the mid-line one as two steps with two ordinals; the
        // defect is that note 5's English half drops the position, and supplying it is what
        // ends the row.
        let omitted = "<a class=\"characterClass\" href=\"#cl-05\">middle dots (cl-05)</a> to \
                       be reduced to leave no space.";
        let supplied = "<a class=\"characterClass\" href=\"#cl-05\">middle dots (cl-05)</a> \
                        placed at the line end to be reduced to leave no space.";
        let html = snapshot();
        assert_eq!(
            html.matches(omitted).count(),
            1,
            "the fixture edits one sentence, and it is §D.2 note 5's English half"
        );
        let corrected = html.replace(omitted, supplied);
        let document = Document::read(&corrected).expect("the fixture is still readable");
        let defect = RECORDED
            .iter()
            .find(|each| each.id == "d2-note-5-line-end-qualifier-omitted-in-english")
            .expect("the recorded omission");
        let refused = (defect.detect)(&document).expect_err("a corrected document is a failure");
        assert!(
            refused.contains("state 行末 in Japanese and no position in English"),
            "the refusal states what it measured: {refused}"
        );
        assert!(
            refused.contains("RECORDED_DEFECTS"),
            "and what to do about it: {refused}"
        );
    }

    #[test]
    fn a_locale_span_is_dropped_with_its_nesting() {
        let fragment = "a<span its-locale-filter-list=\"ja\" lang=\"ja\">x<span>y</span>z</span>b";
        assert_eq!(drop_locale(fragment, "ja"), "ab");
        assert_eq!(drop_locale(fragment, "en"), fragment);
    }

    #[test]
    fn an_element_scan_does_not_match_a_longer_name() {
        let fragment = "<table><td>one</td></table>";
        let cells = elements(fragment, "td");
        assert_eq!(cells.len(), 1, "{cells:?}");
        assert_eq!(cells.first().map(|cell| cell.content), Some("one"));
    }

    #[test]
    fn a_nested_list_is_not_more_items_of_the_list_above() {
        let fragment = "<ol><li>one<ol><li>a</li><li>b</li></ol></li><li>two</li></ol>";
        let items = list_items(fragment);
        assert_eq!(items.len(), 2, "{items:?}");
        assert!(items.last().is_some_and(|(_, item)| *item == "two"));
    }

    #[test]
    fn a_japanese_line_ending_in_a_comma_continues_into_the_next() {
        assert_eq!(
            statements("位取りの空白<br>字幅は四分角"),
            vec!["位取りの空白".to_owned(), "字幅は四分角".to_owned()],
            "two statements, which is the divergence"
        );
        assert_eq!(
            statements("字幅は三分角，<br>半角又はプロポーショナル"),
            vec!["字幅は三分角，半角又はプロポーショナル".to_owned()],
            "one statement wrapped across two lines, which is not"
        );
    }

    #[test]
    fn markup_renders_to_one_line_of_decoded_text() {
        assert_eq!(text("<p>a&nbsp;b\n  c</p>"), "a b c");
        assert_eq!(collapse("  a \n b "), "a b");
    }

    #[test]
    fn a_text_splits_into_the_sentences_that_state_its_ordinals() {
        let read = sentences("One thing. The priority order is the third.");
        assert_eq!(read.len(), 2, "{read:?}");
        assert!(read.last().is_some_and(|last| last.ends_with("third.")));
    }

    #[test]
    fn numerals_are_read_in_the_order_they_appear() {
        assert_eq!(numerals("legend_of_tables_4_5_6"), vec![4, 5, 6]);
        assert_eq!(numerals("Legend of Tables 3, 4 and 5"), vec![3, 4, 5]);
    }

    #[test]
    fn a_field_holding_a_tab_is_refused() {
        assert!(row(&["a", "b\tc", "d", "e"]).is_err());
        assert_eq!(
            row(&["a", "b", "c", "d"]).ok(),
            Some("a\tb\tc\td\n".to_owned())
        );
    }

    #[test]
    fn a_quotation_is_cut_on_a_character_boundary() {
        assert_eq!(head("縦組と横組", 2), "縦組…");
        assert_eq!(head("short", 40), "short");
    }
}
