// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The character-class data path: Appendix A, the ideograph predicate, the compatibility
//! folding, and the script property.
//!
//! Four derivations and four generation units, reading the vendored snapshot and emitting
//! the tables `jlreq-class` looks up:
//!
//! | derived | generated | what it is |
//! | --- | --- | --- |
//! | `appendix-a.tsv` | `appendix_a.rs` | every row of every Appendix A table |
//! | `ideographs.tsv` | `ideograph.rs` | `Unified_Ideograph`, which §A.19 does not list |
//! | `folding.tsv` | `folding.rs` | the Wide and Narrow decompositions, and nothing else |
//! | `scripts.tsv` | `script.rs` | `Script=Hiragana` and `Script=Katakana` |
//!
//! # What this module refuses
//!
//! A silent drop is the failure mode the whole pipeline exists to prevent
//! (`docs/design/generation.md`), so every reading here is checked against a figure written
//! down in this file by hand. `CENSUS` states, class by class, how many single-code-point
//! rows and how many code-point-sequence rows the published Appendix A holds; `REMARKS`
//! states every distinct Remarks cell the document writes, verbatim in both locales, with
//! the count of cells holding it and the frame (字幅), writing direction and role it
//! states. A revision of JLReq that adds a row, moves a row, or writes a Remarks cell this
//! module has never seen fails the build, naming what changed. It cannot regenerate
//! quietly, and it cannot be absorbed.
//!
//! The counts are not derived from the file being read. A count the extractor computes
//! from its own input proves only that the extractor is self-consistent; these were
//! measured against the published document and are hand-maintained here, which is the only
//! arrangement in which the check means anything.
//!
//! # Why a key is a sequence
//!
//! Twenty-five of Appendix A's rows key on an ordered pair of code points rather than on
//! one, written `<304B, 309A>` in the UCS column, and cl-27 lists `<02E5, 02E9>` and
//! `<02E9, 02E5>` as two distinct members. An extractor that assumes one code point per row
//! drops those twenty-five silently, which is why `CENSUS` counts the two shapes
//! separately and why the emitted key is an array (`docs/adr/0008`).
//!
//! # Why the Remarks column is a closed vocabulary
//!
//! The Remarks column is where Appendix A states the disambiguating axis: 834 cells name a
//! frame, twelve name a writing direction, and four name the role of a digit separator or a
//! decimal point. Read as free text it is unusable; enumerated it is exactly fourteen
//! distinct cells, and each maps to the facts below. An unrecognized cell is a failure
//! rather than an empty remark, because a Remarks cell nobody read is a qualification lost.

use crate::derive::Derivation;
use crate::generate::{Emission, Record, Table, Unit};

// ---------------------------------------------------------------------------------------
// What the published Appendix A holds
// ---------------------------------------------------------------------------------------

/// One Appendix A table, as measured in the vendored snapshot.
#[derive(Debug)]
struct Census {
    /// The class the table enumerates, spelled the way JLReq spells it.
    class: &'static str,
    /// Keys whose UCS cell is a single code point.
    singles: usize,
    /// Keys whose UCS cell is a code-point sequence.
    sequences: usize,
}

impl Census {
    /// One measured table, one line, so that the whole appendix is readable at once.
    const fn new(class: &'static str, singles: usize, sequences: usize) -> Self {
        Self {
            class,
            singles,
            sequences,
        }
    }
}

/// Every Appendix A table, in document order, with the keys it enumerates.
///
/// Twenty-five tables for thirty classes. The five that are absent have a heading and no
/// table because their section text reads in full "Any character may participate in …";
/// they are listed in `CONSTRUCT_CLASSES`, and the two lists together must cover all
/// thirty or this module refuses to read the document at all.
const CENSUS: &[Census] = &[
    Census::new("cl-01", 16, 0),
    Census::new("cl-02", 16, 0),
    Census::new("cl-03", 4, 0),
    Census::new("cl-04", 6, 0),
    Census::new("cl-05", 3, 0),
    Census::new("cl-06", 2, 0),
    Census::new("cl-07", 2, 0),
    Census::new("cl-08", 6, 0),
    Census::new("cl-09", 6, 0),
    Census::new("cl-10", 1, 0),
    Census::new("cl-11", 40, 1),
    Census::new("cl-12", 6, 0),
    Census::new("cl-13", 32, 0),
    Census::new("cl-14", 1, 0),
    Census::new("cl-15", 74, 5),
    Census::new("cl-16", 78, 8),
    Census::new("cl-17", 45, 0),
    Census::new("cl-18", 6, 0),
    Census::new("cl-19", 465, 0),
    Census::new("cl-24", 13, 0),
    Census::new("cl-25", 66, 0),
    Census::new("cl-26", 1, 0),
    Census::new("cl-27", 767, 11),
    Census::new("cl-28", 3, 0),
    Census::new("cl-29", 3, 0),
];

/// The classes Appendix A gives a heading and no table.
///
/// They are properties of a construct rather than sets of characters: §A.20 through §A.23
/// and §A.30 read "Any character may participate in …", so there is nothing to enumerate
/// and a lookup table for them would be a fiction (`docs/adr/0008`).
const CONSTRUCT_CLASSES: &[&str] = &["cl-20", "cl-21", "cl-22", "cl-23", "cl-30"];

/// How many classes §3.9.2 closes the set at.
const CLASS_COUNT: usize = 30;

/// How many distinct keys Appendix A enumerates across its twenty-five tables.
const EXPECTED_KEYS: usize = 1133;

/// How many key-and-class listings survive once the one repeated row is removed.
const EXPECTED_LISTINGS: usize = 1686;

/// How many keys more than one class names.
///
/// Two in five. This is the measurement ADR 0008 turns on: no total function from a code
/// point to a class exists, because Appendix A does not define one.
const EXPECTED_MULTI_CLASS_KEYS: usize = 473;

/// The longest key Appendix A enumerates, in code points.
const EXPECTED_MAX_KEY_LEN: usize = 2;

/// The rows the published document lists twice, each with the defect that records it.
///
/// `U+216B` appears twice in the cl-19 body, so §A.19 has 465 rows and 464 members. It is
/// the only duplicate in Appendix A. A duplicate that is not recorded here fails the
/// build, and a recorded duplicate that upstream has fixed fails it too, so a correction
/// forces a review instead of changing an answer quietly.
const RECORDED_DUPLICATES: &[(&str, u32, &str)] =
    &[("cl-19", 0x0000_216B, "cl-19-duplicate-u216b")];

/// The defect recording the three Remarks cells that carry no locale span.
///
/// Three cl-25 cells hold the bare string `プロポーショナル` with no
/// `its-locale-filter-list` attribute, so an English-locale extraction yields an empty
/// remark for three rows that mean "proportionally-spaced".
///
/// This is the one defect that is a statement about a cell's *shape* rather than about its
/// content, which is why `remark` checks it and checks no other: a cell written as two
/// locale spans is not an instance of a defect whose whole content is that the spans are
/// missing.
const UNLOCALISED_DEFECT: &str = "cl-25-remarks-without-locale-span";

/// The defect recording the Remarks cells whose English half drops a fact the Japanese half
/// states.
///
/// Three cells of §A.24 and §A.25 name the digit-grouping role in Japanese alone — 位取りの
/// 空白 and 位取りのコンマ — where the English half gives the width and nothing else.
/// `docs/design/generation.md` requires the extractor to fail on a divergence it has not
/// recorded, so the divergence is recorded here and in that document's table of defects, and
/// the role is read from the Japanese half, which is the half that states it. §A.24's
/// `U+002E` does *not* diverge: both halves carry the decimal-point line.
const ROLE_ONLY_IN_JAPANESE_DEFECT: &str = "cl-24-remarks-role-stated-only-in-japanese";

// ---------------------------------------------------------------------------------------
// The Remarks vocabulary
// ---------------------------------------------------------------------------------------

/// One named value the emitted tables are written in terms of.
///
/// The name, the value and the sentence explaining it are written once here and emitted
/// into the generated module, so the constant a generated table names and the constant
/// this program assembled it from cannot drift apart.
#[derive(Debug)]
struct Named {
    /// The constant's name in the generated module.
    name: &'static str,
    /// Its value.
    value: u8,
    /// Its documentation, as one line of prose.
    doc: &'static str,
}

/// The frame (字幅) vocabulary, as a mask: a Remarks cell may permit more than one.
///
/// JLReq's own vocabulary, in the reading `docs/adr/0008` fixed: what the caller's supplied
/// advance covers. `FRAMES_UNSTATED` is the empty mask and is emitted beside these.
const FRAMES: &[Named] = &[
    Named {
        name: "FRAME_FULL_EM",
        value: 0b0000_0001,
        doc: "the full ideographic em (全角, full-width)",
    },
    Named {
        name: "FRAME_HALF_EM",
        value: 0b0000_0010,
        doc: "half an em (半角, `half-width` in the Remarks column)",
    },
    Named {
        name: "FRAME_THIRD_EM",
        value: 0b0000_0100,
        doc: "a third of an em (三分角, `one third em width`)",
    },
    Named {
        name: "FRAME_QUARTER_EM",
        value: 0b0000_1000,
        doc: "a quarter em (四分角, `quarter em width`)",
    },
    Named {
        name: "FRAME_PROPORTIONAL",
        value: 0b0001_0000,
        doc: "a per-glyph advance (プロポーショナル, `proportionally-spaced`)",
    },
];

/// The writing-direction qualification a Remarks cell may carry.
///
/// Appendix A states two of them and no more. `Usage::HorizontalOrRotatedWestern` is not
/// here, because Appendix A does not state it: §3.1.1 refines four of the seven
/// `used in horizontal composition` cells, and that refinement is a rule rather than a row
/// of this table.
const USAGES: &[Named] = &[
    Named {
        name: "USAGE_UNQUALIFIED",
        value: 0,
        doc: "the Remarks cell restricts the writing direction in no way",
    },
    Named {
        name: "USAGE_HORIZONTAL_ONLY",
        value: 1,
        doc: "`used in horizontal composition` (横組で使用)",
    },
    Named {
        name: "USAGE_VERTICAL_ONLY",
        value: 2,
        doc: "`used in vertical composition` (縦組で使用)",
    },
];

/// The role a Remarks cell may name.
///
/// Two of the three the role axis has, because Appendix A's Remarks column states only
/// these two; the rest of the axis is stated in prose (§3.1.3, §B.2#12, §C.2#11).
const ROLES: &[Named] = &[
    Named {
        name: "ROLE_UNSTATED",
        value: 0,
        doc: "the Remarks cell names no role",
    },
    Named {
        name: "ROLE_DECIMAL_POINT",
        value: 1,
        doc: "`decimal point` (小数点)",
    },
    Named {
        name: "ROLE_DIGIT_GROUP_SEPARATOR",
        value: 2,
        doc: "the digit-grouping space or comma (位取りの空白, 位取りのコンマ)",
    },
];

/// The name of the empty frame mask in the generated module.
const FRAMES_UNSTATED: &str = "FRAMES_UNSTATED";

/// One distinct Remarks cell of the published Appendix A, and what it states.
#[derive(Debug)]
struct Remark {
    /// The English half of the cell, verbatim, with `<br>` written as a newline.
    en: &'static str,
    /// The Japanese half, likewise.
    ja: &'static str,
    /// How many cells of Appendix A hold exactly this pair, measured against the published
    /// document.
    cells: usize,
    /// The frames it permits, as a mask over `FRAMES`.
    frames: u8,
    /// The writing-direction qualification it carries, from `USAGES`.
    usage: u8,
    /// The role it names, from `ROLES`.
    role: u8,
    /// The recorded defect this cell is an instance of, or the empty string.
    defect: &'static str,
}

/// Every distinct Remarks cell the published Appendix A writes.
///
/// Fourteen, covering all 1687 cells. The first is the empty cell, which is not a
/// statement about anything and therefore states nothing; the rest are the qualifications.
/// Two pairs share an English half and differ in Japanese — `quarter em width` is written
/// against both 字幅は四分角 and 位取りの空白/字幅は四分角 — which is why the key is the
/// pair and not either half (`docs/design/generation.md`).
const REMARKS: &[Remark] = &[
    Remark {
        en: "",
        ja: "",
        cells: 814,
        frames: 0,
        usage: 0,
        role: 0,
        defect: "",
    },
    Remark {
        en: "proportionally-spaced",
        ja: "プロポーショナル",
        cells: 834,
        frames: 0b0001_0000,
        usage: 0,
        role: 0,
        defect: "",
    },
    Remark {
        en: "",
        ja: "プロポーショナル",
        cells: 3,
        frames: 0b0001_0000,
        usage: 0,
        role: 0,
        defect: UNLOCALISED_DEFECT,
    },
    Remark {
        en: "half-width",
        ja: "字幅は半角",
        cells: 13,
        frames: 0b0000_0010,
        usage: 0,
        role: 0,
        defect: "",
    },
    Remark {
        en: "half-width or proportional",
        ja: "半角又はプロポーショナル",
        cells: 4,
        frames: 0b0001_0010,
        usage: 0,
        role: 0,
        defect: "",
    },
    Remark {
        en: "quarter em width",
        ja: "字幅は四分角",
        cells: 1,
        frames: 0b0000_1000,
        usage: 0,
        role: 0,
        defect: "",
    },
    Remark {
        en: "quarter em width",
        ja: "位取りの空白\n字幅は四分角",
        cells: 2,
        frames: 0b0000_1000,
        usage: 0,
        role: 2,
        defect: ROLE_ONLY_IN_JAPANESE_DEFECT,
    },
    Remark {
        en: "quarter em width or half-width",
        ja: "位取りのコンマ\n字幅は四分角又は半角",
        cells: 1,
        frames: 0b0000_1010,
        usage: 0,
        role: 2,
        defect: ROLE_ONLY_IN_JAPANESE_DEFECT,
    },
    Remark {
        en: "decimal point\nquarter em width or half-width",
        ja: "小数点\n字幅は四分角又は半角",
        cells: 1,
        frames: 0b0000_1010,
        usage: 0,
        role: 1,
        defect: "",
    },
    Remark {
        en: "one third em width, half-width or proportional",
        ja: "字幅は三分角，\n半角又はプロポーショナル",
        cells: 1,
        frames: 0b0001_0110,
        usage: 0,
        role: 0,
        defect: "",
    },
    Remark {
        en: "used in horizontal composition",
        ja: "横組で使用",
        cells: 7,
        frames: 0,
        usage: 1,
        role: 0,
        defect: "",
    },
    Remark {
        en: "used in vertical composition",
        ja: "縦組で使用",
        cells: 3,
        frames: 0,
        usage: 2,
        role: 0,
        defect: "",
    },
    Remark {
        en: "used in vertical composition\nU+3035 follows this",
        ja: "縦組で使用\nこの文字の後ろにU+3035が配置される",
        cells: 2,
        frames: 0,
        usage: 2,
        role: 0,
        defect: "",
    },
    Remark {
        en: "Some systems implement U+2015 HORIZONTAL BAR very similar behavior to U+2014 EM DASH",
        ja: "処理系によっては，U+2015 (HORIZONTAL BAR)にも，同様の振る舞いを実装しているものもある",
        cells: 1,
        frames: 0,
        usage: 0,
        role: 0,
        defect: "",
    },
];

// ---------------------------------------------------------------------------------------
// What the Unicode Character Database holds
// ---------------------------------------------------------------------------------------

/// The property §A.19's missing members are read from.
///
/// `Unified_Ideograph=Yes` is the base and the alternatives are both wrong:
/// `Ideographic=Yes` over-covers with Tangut, Nushu and Khitan, and `Script=Han`
/// over-covers with `U+3005`, which JLReq puts in cl-09. `Unified_Ideograph` excludes
/// `U+3005` and `U+303B` exactly as JLReq needs (`docs/design/generation.md`).
const UNIFIED_IDEOGRAPH: &str = "Unified_Ideograph";

/// How many ranges the vendored `PropList.txt` gives that property.
const EXPECTED_IDEOGRAPH_RANGES: usize = 16;

/// How many code points those ranges cover.
const EXPECTED_IDEOGRAPHS: u32 = 101_996;

/// Code points the ideograph predicate must not claim, with the class JLReq puts each in.
///
/// The two iteration marks are the whole reason `Unified_Ideograph` is the right property.
const NOT_IDEOGRAPHS: &[(u32, &str)] = &[(0x0000_3005, "cl-09"), (0x0000_303B, "cl-09")];

/// A code point the ideograph predicate must claim although §A.19 also lists it.
///
/// `U+4EDD` 仝 is enumerated in cl-19 *and* is a unified ideograph, so "listed" and "is an
/// ideograph" are not disjoint and nothing downstream may assume they are.
const BOTH_LISTED_AND_IDEOGRAPH: u32 = 0x0000_4EDD;

/// The decomposition tag naming a full-width compatibility form.
const WIDE: &str = "<wide>";

/// The decomposition tag naming a half-width compatibility form.
const NARROW: &str = "<narrow>";

/// How many `Decomposition_Type=Wide` mappings the vendored `UnicodeData.txt` holds.
const EXPECTED_WIDE: usize = 104;

/// How many `Decomposition_Type=Narrow` mappings it holds.
const EXPECTED_NARROW: usize = 122;

/// The two scripts §C.2 note 3's small-kana fallback reads.
const KANA_SCRIPTS: &[&str] = &["Hiragana", "Katakana"];

/// How many ranges the vendored `Scripts.txt` gives those two scripts together.
const EXPECTED_SCRIPT_RANGES: usize = 22;

/// How many code points those ranges cover.
///
/// Stated beside the range count because the two fail on different edits: a deleted row
/// moves the count, and a moved bound moves only this.
const EXPECTED_KANA: u32 = 702;

// ---------------------------------------------------------------------------------------
// Stage 1: the derivations
// ---------------------------------------------------------------------------------------

/// The vendored rendering of the published document.
const SNAPSHOT: &str = "spec/snapshot/index.html";

/// Every row of every Appendix A character table.
pub(crate) const APPENDIX_A: Derivation = Derivation {
    sources: &[SNAPSHOT],
    reader: &["xtask/src/classes.rs"],
    output: "spec/derived/appendix-a.tsv",
    caption: "Every row of every Appendix A table, in document order: the class that lists \
              the key, the key, and the Remarks cell in both locales.",
    read: read_appendix_a,
};

/// The thirty classes §3.9.2 closes the set at, named as that section names them.
pub(crate) const CLASSES: Derivation = Derivation {
    sources: &[SNAPSHOT],
    reader: &["xtask/src/classes.rs"],
    output: "spec/derived/classes.tsv",
    caption: "The thirty character classes §3.9.2 closes the set at: the id the document \
              anchors each one by, its name in both locales as §3.9.2 writes it, and the \
              Appendix A section that enumerates it where one does.",
    read: read_classes,
};

/// The ideographs §A.19 leaves to the Unicode Character Database.
pub(crate) const IDEOGRAPHS: Derivation = Derivation {
    sources: &["spec/snapshot/ucd/PropList.txt"],
    reader: &["xtask/src/classes.rs"],
    output: "spec/derived/ideographs.tsv",
    caption: "The Unified_Ideograph ranges, which are the members of cl-19 that §A.19's \
              table deliberately does not list.",
    read: read_ideographs,
};

/// The compatibility folding Appendix A's preamble requires and bounds.
pub(crate) const FOLDING: Derivation = Derivation {
    sources: &["spec/snapshot/ucd/UnicodeData.txt"],
    reader: &["xtask/src/classes.rs"],
    output: "spec/derived/folding.tsv",
    caption: "The Wide and Narrow compatibility decompositions, and nothing else: full \
              compatibility folding would fold U+2160, a genuine cl-19 member, onto I.",
    read: read_folding,
};

/// The script property behind §C.2 note 3's small-kana fallback.
pub(crate) const SCRIPTS: Derivation = Derivation {
    sources: &["spec/snapshot/ucd/Scripts.txt"],
    reader: &["xtask/src/classes.rs"],
    output: "spec/derived/scripts.tsv",
    caption: "The Script=Hiragana and Script=Katakana ranges, which §C.2 note 3's small-kana \
              fallback reads.",
    read: read_scripts,
};

/// One row of one Appendix A table.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Row {
    /// The class whose table listed it.
    class: String,
    /// The key, as code points.
    key: Vec<u32>,
    /// Which `REMARKS` entry the Remarks cell holds.
    remark: usize,
}

/// Read Appendix A out of the published document.
fn read_appendix_a(sources: &[String]) -> Result<String, String> {
    let html = source(sources, 0)?;
    check_class_coverage(html)?;
    let rows = appendix_a_rows(html)?;
    check_census(&rows)?;
    check_remark_counts(&rows)?;

    let mut records = Vec::with_capacity(rows.len());
    for row in &rows {
        let remark = REMARKS
            .get(row.remark)
            .ok_or_else(|| format!("{class}: no such remark", class = row.class))?;
        records.push(format!(
            "{class}\t{key}\t{en}\t{ja}",
            class = row.class,
            key = render_key(&row.key),
            en = escape(remark.en),
            ja = escape(remark.ja),
        ));
    }
    Ok(tabulate("class\tkey\tremark-en\tremark-ja", &records))
}

/// Read the thirty class names out of §3.9.2's own list.
///
/// The names come from §3.9.2 and not from the Appendix A headings, because the two differ:
/// §A.21 heads "Ornamented character complexes" where §3.9.2 names the class for the
/// characters *in* one, and §A.10 writes the prolonged sound mark in the singular where
/// §3.9.2 writes it in the plural. §3.9.2 is the section that closes the set, so it is the
/// section the names are read from.
///
/// Each item of that list is published with an id of its own — `cl-01-en` and `cl-01-ja` —
/// which is what makes the list machine-readable at all; the class id in parentheses that
/// closes every name is the same one the id carries, so it is removed here rather than
/// stored twice.
fn read_classes(sources: &[String]) -> Result<String, String> {
    let html = source(sources, 0)?;
    check_class_coverage(html)?;
    let mut records = Vec::with_capacity(CLASS_COUNT);
    for number in 1..=CLASS_COUNT {
        let class = format!("cl-{number:02}");
        let en = class_name(html, &class, "en")?;
        let ja = class_name(html, &class, "ja")?;
        let rendered = enumerating_section(html, &class)?;
        let enumeration = if CONSTRUCT_CLASSES.contains(&class.as_str()) {
            String::new()
        } else {
            rendered.clone()
        };
        // The class number and the rendered section number are two facts, and this
        // derivation states the second. They agree today, and holding them equal is what
        // makes a renumbered Appendix A a build failure rather than a wrong specification
        // address published on the frozen public surface (`Class::enumeration`).
        if rendered != format!("A.{number}") {
            return Err(format!(
                "§A numbers `{class}`'s section `{rendered}` where this repository was \
                 written against `A.{number}`; the address is published by \
                 `Class::enumeration`, so a renumbering is reviewed rather than absorbed"
            ));
        }
        records.push(format!("{class}\t{en}\t{ja}\t{enumeration}"));
    }
    Ok(tabulate("class\tname_en\tname_ja\tenumeration", &records))
}

/// The rendered section number of the Appendix A heading that enumerates one class.
///
/// Read from that heading's own `<bdi class="secno">`, which ADR 0013 makes the only place a
/// section number may come from: the anchor slug is the class id and says nothing about
/// where the section sits, and the appendix legend anchors are off by one from the tables
/// they render. Computing `A.{number}` from the class number instead would be arithmetic
/// wearing a derived file's provenance header.
fn enumerating_section(html: &str, class: &str) -> Result<String, String> {
    let open = format!("<h3 id=\"{class}\">");
    let start = html
        .find(&open)
        .ok_or_else(|| format!("§A no longer gives `{class}` a heading of its own"))?;
    let (heading, _) = between(html, &open, "</h3>", start)
        .ok_or_else(|| format!("§A's `{class}` heading is never closed"))?;
    let (number, _) = between(heading, "<bdi class=\"secno\">", "</bdi>", 0)
        .ok_or_else(|| format!("§A's `{class}` heading opens with no rendered section number"))?;
    Ok(collapse(number).trim_end_matches('.').to_owned())
}

/// One class name, in one locale, with the class id that closes it removed.
fn class_name(html: &str, class: &str, locale: &str) -> Result<String, String> {
    let open = format!("<p id=\"{class}-{locale}\"");
    let start = html
        .find(&open)
        .ok_or_else(|| format!("§3.9.2 no longer names `{class}` in `{locale}`"))?;
    let (markup, _) = between(html, ">", "</p>", start)
        .ok_or_else(|| format!("§3.9.2's `{class}` item in `{locale}` is never closed"))?;
    let stripped = strip_markup(markup)?;
    let named = collapse(&stripped);
    // Both locales close the name with the class id, in ASCII parentheses in English and in
    // full-width ones in Japanese. Refusing a name that carries neither is what keeps this
    // from silently storing "Opening brackets (cl-01)" as the name.
    for closing in [format!(" ({class})"), format!("（{class}）")] {
        if let Some(name) = named.strip_suffix(&closing) {
            if name.is_empty() {
                return Err(format!("§3.9.2 names `{class}` with nothing but its id"));
            }
            return Ok(name.to_owned());
        }
    }
    Err(format!(
        "§3.9.2's `{locale}` name for `{class}` reads `{named}`, which does not close with \
         the class id; the id is what says the name belongs to that class"
    ))
}

/// Markup removed from one element's content, refusing an entity this reader does not know.
///
/// The class names are wrapped in an index anchor in a few places and in nothing else, so
/// dropping tags is enough; an entity would be a character silently half-read, which is
/// refused here for the same reason `prose` refuses one.
fn strip_markup(markup: &str) -> Result<String, String> {
    let mut text = String::with_capacity(markup.len());
    let mut rest = markup;
    while let Some(open) = rest.find('<') {
        let (before, from) = rest.split_at(open);
        text.push_str(before);
        let close = from
            .find('>')
            .ok_or_else(|| format!("`{markup}` holds a tag that is never closed"))?;
        rest = from.get(close.saturating_add(1)..).unwrap_or_default();
    }
    text.push_str(rest);
    if text.contains('&') {
        return Err(format!(
            "`{markup}` holds a character entity this reader does not know, and reading past \
             it would drop whatever it says"
        ));
    }
    Ok(text)
}

/// Read the ideograph predicate out of the Unicode Character Database.
fn read_ideographs(sources: &[String]) -> Result<String, String> {
    let ranges = property_ranges(source(sources, 0)?, UNIFIED_IDEOGRAPH)?;
    check_ranges(
        UNIFIED_IDEOGRAPH,
        &ranges,
        EXPECTED_IDEOGRAPH_RANGES,
        EXPECTED_IDEOGRAPHS,
    )?;
    for (code_point, class) in NOT_IDEOGRAPHS {
        if ranges
            .iter()
            .any(|(first, last)| covers(*first, *last, *code_point))
        {
            return Err(format!(
                "U+{code_point:04X} is `{UNIFIED_IDEOGRAPH}` in this Unicode revision, and \
                 JLReq puts it in {class}; the predicate would move it into cl-19"
            ));
        }
    }
    if !ranges
        .iter()
        .any(|(first, last)| covers(*first, *last, BOTH_LISTED_AND_IDEOGRAPH))
    {
        return Err(format!(
            "U+{BOTH_LISTED_AND_IDEOGRAPH:04X} is enumerated in cl-19 and is no longer an \
             ideograph, so `listed` and `is an ideograph` may no longer overlap"
        ));
    }

    let records: Vec<String> = ranges
        .iter()
        .map(|(first, last)| format!("{first:04X}\t{last:04X}"))
        .collect();
    Ok(tabulate("first\tlast", &records))
}

/// Read the compatibility folding out of the Unicode Character Database.
fn read_folding(sources: &[String]) -> Result<String, String> {
    let text = source(sources, 0)?;
    let wide = decompositions(text, WIDE)?;
    let narrow = decompositions(text, NARROW)?;
    if wide.len() != EXPECTED_WIDE || narrow.len() != EXPECTED_NARROW {
        return Err(format!(
            "the database holds {wide} `{WIDE}` and {narrow} `{NARROW}` decomposition(s) \
             where this repository was written against {EXPECTED_WIDE} and \
             {EXPECTED_NARROW}",
            wide = wide.len(),
            narrow = narrow.len()
        ));
    }

    let mut folds: Vec<(u32, u32, &str)> = Vec::new();
    folds.extend(wide.iter().map(|(from, to)| (*from, *to, "full-em")));
    folds.extend(narrow.iter().map(|(from, to)| (*from, *to, "half-em")));
    folds.sort_unstable();
    for pair in folds.windows(2) {
        if let [before, after] = pair {
            if before.0 == after.0 {
                return Err(format!(
                    "U+{source:04X} has two compatibility decompositions, so folding it is \
                     no longer a function",
                    source = before.0
                ));
            }
        }
    }

    let records: Vec<String> = folds
        .iter()
        .map(|(from, to, frame)| format!("{from:04X}\t{to:04X}\t{frame}"))
        .collect();
    Ok(tabulate("source\ttarget\tframe", &records))
}

/// Read the two kana scripts out of the Unicode Character Database.
fn read_scripts(sources: &[String]) -> Result<String, String> {
    let text = source(sources, 0)?;
    let mut rows: Vec<(u32, u32, &str)> = Vec::new();
    for script in KANA_SCRIPTS {
        for (first, last) in property_ranges(text, script)? {
            rows.push((first, last, script));
        }
    }
    rows.sort_unstable();
    let bounds: Vec<(u32, u32)> = rows
        .iter()
        .map(|(first, last, _)| (*first, *last))
        .collect();
    check_ranges(
        "the two kana scripts",
        &bounds,
        EXPECTED_SCRIPT_RANGES,
        EXPECTED_KANA,
    )?;

    let records: Vec<String> = rows
        .iter()
        .map(|(first, last, script)| format!("{script}\t{first:04X}\t{last:04X}"))
        .collect();
    Ok(tabulate("script\tfirst\tlast", &records))
}

// ---------------------------------------------------------------------------------------
// Reading the published document
// ---------------------------------------------------------------------------------------

/// The element every Appendix A table is written as.
const TABLE_OPEN: &str = "<table class=\"charclass\">";

/// Where a table ends.
const TABLE_CLOSE: &str = "</table>";

/// The heading that names the class a table enumerates.
const HEADING: &str = "<h3 id=\"";

/// The English half of a bilingual cell.
const ENGLISH: &str = "<span its-locale-filter-list=\"en\" lang=\"en\">";

/// The Japanese half.
const JAPANESE: &str = "<span its-locale-filter-list=\"ja\" lang=\"ja\">";

/// Where either half ends.
const SPAN_CLOSE: &str = "</span>";

/// How many cells one Appendix A row holds: Character, UCS, Name, Common name, Remarks.
const COLUMNS: usize = 5;

/// Which of those cells holds the key.
const UCS_COLUMN: usize = 1;

/// Which holds the Remarks.
const REMARKS_COLUMN: usize = 4;

/// Every row of every Appendix A table, in document order.
fn appendix_a_rows(html: &str) -> Result<Vec<Row>, String> {
    let mut rows = Vec::new();
    let mut tables = 0_usize;
    let mut cursor = 0_usize;
    while let Some(at) = find_from(html, TABLE_OPEN, cursor) {
        let class = enclosing_class(html, at)?;
        let (table, next) = between(html, TABLE_OPEN, TABLE_CLOSE, cursor)
            .ok_or_else(|| format!("{class}: a `charclass` table is never closed"))?;
        read_class_table(class, table, &mut rows)?;
        tables = tables.saturating_add(1);
        cursor = next;
    }
    if tables != CENSUS.len() {
        return Err(format!(
            "the document holds {tables} `charclass` table(s) where this repository was \
             written against {expected}",
            expected = CENSUS.len()
        ));
    }
    Ok(rows)
}

/// The class named by the heading a table sits under.
fn enclosing_class(html: &str, table: usize) -> Result<&str, String> {
    let before = html
        .get(..table)
        .ok_or_else(|| "a `charclass` table begins inside a character".to_owned())?;
    let at = before
        .rfind(HEADING)
        .ok_or_else(|| "a `charclass` table sits under no heading".to_owned())?
        .saturating_add(HEADING.len());
    let rest = html
        .get(at..table)
        .ok_or_else(|| "a heading begins inside a character".to_owned())?;
    let end = rest
        .find('"')
        .ok_or_else(|| "a heading identifier is never closed".to_owned())?;
    let class = rest
        .get(..end)
        .ok_or_else(|| "a heading identifier begins inside a character".to_owned())?;
    if class_number(class).is_none() {
        return Err(format!(
            "a `charclass` table sits under the heading `{class}`, which names no character \
             class"
        ));
    }
    Ok(class)
}

/// Read one table's body into `rows`.
fn read_class_table(class: &str, table: &str, rows: &mut Vec<Row>) -> Result<(), String> {
    let (body, _) = between(table, "<tbody>", "</tbody>", 0)
        .ok_or_else(|| format!("{class}: the table has no body"))?;
    let mut cursor = 0_usize;
    while let Some((row, next)) = between(body, "<tr>", "</tr>", cursor) {
        let cells = cells(row).map_err(|reason| format!("{class}: {reason}"))?;
        if cells.len() != COLUMNS {
            return Err(format!(
                "{class}: a row holds {found} cell(s) where the table has {COLUMNS} columns",
                found = cells.len()
            ));
        }
        let ucs = cells
            .get(UCS_COLUMN)
            .ok_or_else(|| format!("{class}: a row has no UCS cell"))?;
        let remarks = cells
            .get(REMARKS_COLUMN)
            .ok_or_else(|| format!("{class}: a row has no Remarks cell"))?;
        let remark = remark(remarks).map_err(|reason| format!("{class}: {reason}"))?;
        for key in keys(ucs).map_err(|reason| format!("{class}: {reason}"))? {
            rows.push(Row {
                class: class.to_owned(),
                key,
                remark,
            });
        }
        cursor = next;
    }
    Ok(())
}

/// The cells of one row.
fn cells(row: &str) -> Result<Vec<&str>, String> {
    let mut cells = Vec::new();
    let mut cursor = 0_usize;
    while let Some(open) = find_from(row, "<td", cursor) {
        let content = find_from(row, ">", open)
            .ok_or_else(|| "a cell's opening tag is never closed".to_owned())?
            .saturating_add(1);
        let close =
            find_from(row, "</td>", content).ok_or_else(|| "a cell is never closed".to_owned())?;
        cells.push(
            row.get(content..close)
                .ok_or_else(|| "a cell begins inside a character".to_owned())?,
        );
        cursor = close;
    }
    Ok(cells)
}

/// The keys one UCS cell names.
///
/// Three shapes are accepted and everything else is refused: a bare code point, a sequence
/// written `<304B, 309A>`, and a sequence one of whose positions offers alternatives,
/// written `<0254, 0300/0301>`, which yields one key per alternative.
///
/// Measured: the vendored rendering holds none of the third shape, and neither does the
/// pre-`ReSpec` editorial source — both write `<0254, 0300>` and `<0254, 0301>` as two
/// rows. It is read anyway, and reading it is safe rather than permissive, because
/// `CENSUS` counts keys: a row that yielded two would move a class's total and fail before
/// anything was emitted.
fn keys(cell: &str) -> Result<Vec<Vec<u32>>, String> {
    let text = collapse(cell);
    let Some(inner) = text
        .strip_prefix("&lt;")
        .and_then(|rest| rest.strip_suffix("&gt;"))
    else {
        return Ok(vec![vec![code_point(&text)?]]);
    };
    let mut built: Vec<Vec<u32>> = vec![Vec::new()];
    for position in inner.split(',') {
        let mut grown = Vec::new();
        for alternative in position.trim().split('/') {
            let code_point = code_point(alternative.trim())?;
            for prefix in &built {
                let mut key = prefix.clone();
                key.push(code_point);
                grown.push(key);
            }
        }
        built = grown;
    }
    if built.iter().any(|key| key.len() < 2) {
        return Err(format!(
            "`{text}` is written as a sequence and names fewer than two code points"
        ));
    }
    Ok(built)
}

/// One code point, written as the document writes one.
fn code_point(text: &str) -> Result<u32, String> {
    let digits = text.len();
    if !(4..=6).contains(&digits) || !text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("`{text}` is not a UCS code point"));
    }
    let value = u32::from_str_radix(text, 16)
        .map_err(|error| format!("`{text}` is not a UCS code point: {error}"))?;
    if char::from_u32(value).is_none() {
        return Err(format!(
            "`{text}` is a surrogate or beyond U+10FFFF, so no text can hold it"
        ));
    }
    Ok(value)
}

/// How one Remarks cell is localized.
#[derive(Debug, PartialEq, Eq)]
enum Localization {
    /// The cell is empty, so it states nothing.
    Absent,
    /// The cell holds both halves, each in its own locale span.
    Localized {
        /// The English half.
        en: String,
        /// The Japanese half.
        ja: String,
    },
    /// The cell holds text and no locale span at all, which is a recorded defect.
    Unlocalized(String),
}

/// Which `REMARKS` entry states one pair of locale halves.
///
/// The one lookup both stages use. Stage 1 reads a cell out of the published document and
/// stage 2 reads the two columns back out of `spec/derived/appendix-a.tsv`; if the two keyed
/// differently, a cell stage 1 refused could be one stage 2 accepted, and the derived table
/// and the generated table would disagree about what a row means.
fn remark_position(en: &str, ja: &str) -> Option<usize> {
    REMARKS
        .iter()
        .position(|remark| remark.en == en && remark.ja == ja)
}

/// Which `REMARKS` entry one Remarks cell holds.
///
/// The pair of halves is what identifies the entry, and the cell's *shape* is checked
/// against it afterwards: an entry recording `UNLOCALISED_DEFECT` describes a cell written
/// with no locale span at all, so a cell that has spans is not an instance of it and a bare
/// cell is not an instance of anything else. No other recorded defect is a statement about
/// the shape, so no other one is a condition here — which is what keeps this lookup and
/// `read_listings`'s the same lookup.
fn remark(cell: &str) -> Result<usize, String> {
    let localization = split_locales(&collapse(cell))?;
    let found = match &localization {
        Localization::Absent => remark_position("", ""),
        Localization::Localized { en, ja } => remark_position(en, ja).filter(|found| {
            REMARKS
                .get(*found)
                .is_some_and(|it| it.defect != UNLOCALISED_DEFECT)
        }),
        Localization::Unlocalized(text) => remark_position("", text).filter(|found| {
            REMARKS
                .get(*found)
                .is_some_and(|it| it.defect == UNLOCALISED_DEFECT)
        }),
    };
    found.ok_or_else(|| {
        format!(
            "the Remarks cell {localization:?} is not one this repository has read; every \
             distinct cell is enumerated in `REMARKS` with the frame, direction and role it \
             states, and a cell nobody read is a qualification lost"
        )
    })
}

/// Split one Remarks cell into its two locale halves.
///
/// The two spans are reassembled and compared against the cell they came out of, so a
/// third span, a stray word between them, or a reordering is a failure rather than a
/// silently discarded fragment.
fn split_locales(collapsed: &str) -> Result<Localization, String> {
    if collapsed.is_empty() {
        return Ok(Localization::Absent);
    }
    if !collapsed.contains('<') {
        return Ok(Localization::Unlocalized(prose(collapsed)?));
    }
    let (en, _) = between(collapsed, ENGLISH, SPAN_CLOSE, 0)
        .ok_or_else(|| format!("`{collapsed}` holds no English locale span"))?;
    let (ja, _) = between(collapsed, JAPANESE, SPAN_CLOSE, 0)
        .ok_or_else(|| format!("`{collapsed}` holds no Japanese locale span"))?;
    let rebuilt = format!("{ENGLISH}{en}{SPAN_CLOSE} {JAPANESE}{ja}{SPAN_CLOSE}");
    if rebuilt != collapsed {
        return Err(format!(
            "`{collapsed}` is not two locale spans and nothing else, so reading it would \
             discard part of the cell"
        ));
    }
    Ok(Localization::Localized {
        en: prose(en)?,
        ja: prose(ja)?,
    })
}

/// One half of a Remarks cell as prose: line breaks kept, markup refused.
fn prose(text: &str) -> Result<String, String> {
    let broken = text.replace("<br>", "\n");
    if broken.contains('<') || broken.contains('&') {
        return Err(format!(
            "`{text}` holds markup this reader does not know, and reading past it would \
             drop whatever it says"
        ));
    }
    Ok(broken)
}

// ---------------------------------------------------------------------------------------
// Reading the Unicode Character Database
// ---------------------------------------------------------------------------------------

/// Every range one property file gives one property, in file order.
///
/// The file is required to be sorted and disjoint rather than assumed to be: a property
/// file that had grown a second entry for one range would otherwise be read as two.
fn property_ranges(text: &str, property: &str) -> Result<Vec<(u32, u32)>, String> {
    let mut ranges: Vec<(u32, u32)> = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap_or(line).trim();
        if line.is_empty() {
            continue;
        }
        let (codes, name) = line.split_once(';').ok_or_else(|| {
            format!(
                "line {line_number}: `{line}` is not a `code points ; property` line",
                line_number = number.saturating_add(1)
            )
        })?;
        if name.trim() != property {
            continue;
        }
        let range = range(codes.trim())?;
        if let Some(previous) = ranges.last() {
            if previous.1 >= range.0 {
                return Err(format!(
                    "the range beginning U+{first:04X} is not after the one before it, so \
                     `{property}` is no longer a sorted, disjoint list",
                    first = range.0
                ));
            }
        }
        ranges.push(range);
    }
    if ranges.is_empty() {
        return Err(format!("`{property}` covers no code point"));
    }
    Ok(ranges)
}

/// One `first..last` or single code point of a property file.
fn range(codes: &str) -> Result<(u32, u32), String> {
    let (first, last) = codes.split_once("..").unwrap_or((codes, codes));
    let range = (code_point(first.trim())?, code_point(last.trim())?);
    if range.0 > range.1 {
        return Err(format!("`{codes}` ends before it begins"));
    }
    Ok(range)
}

/// Every decomposition of one tag in `UnicodeData.txt`, in file order.
///
/// A tagged decomposition of more than one code point would not be a folding, so it is
/// refused rather than truncated to its first.
fn decompositions(text: &str, tag: &str) -> Result<Vec<(u32, u32)>, String> {
    let mut mappings = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let line_number = number.saturating_add(1);
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split(';').collect();
        let (Some(code), Some(decomposition)) = (fields.first(), fields.get(5)) else {
            return Err(format!(
                "line {line_number}: a record of the character database has fewer than six \
                 fields"
            ));
        };
        let Some(rest) = decomposition.trim().strip_prefix(tag) else {
            continue;
        };
        let mut targets = rest.split_whitespace();
        let (Some(target), None) = (targets.next(), targets.next()) else {
            return Err(format!(
                "line {line_number}: `{tag}` decomposes to something other than one code \
                 point, which is not a folding"
            ));
        };
        mappings.push((code_point(code.trim())?, code_point(target)?));
    }
    Ok(mappings)
}

/// Whether an inclusive range covers a code point.
fn covers(first: u32, last: u32, code_point: u32) -> bool {
    first <= code_point && code_point <= last
}

// ---------------------------------------------------------------------------------------
// Checking the reading against what was measured
// ---------------------------------------------------------------------------------------

/// Every class either has a table or is one of the five that enumerate nothing.
fn check_class_coverage(html: &str) -> Result<(), String> {
    let mut covered: Vec<usize> = Vec::new();
    for class in CENSUS
        .iter()
        .map(|census| census.class)
        .chain(CONSTRUCT_CLASSES.iter().copied())
    {
        let number =
            class_number(class).ok_or_else(|| format!("`{class}` names no character class"))?;
        if covered.contains(&number) {
            return Err(format!("`{class}` is accounted for twice"));
        }
        if !html.contains(&format!("{HEADING}{class}\"")) {
            return Err(format!(
                "the document has no heading for `{class}`, so Appendix A no longer covers \
                 the classes this repository was written against"
            ));
        }
        covered.push(number);
    }
    if covered.len() != CLASS_COUNT {
        return Err(format!(
            "{found} class(es) are accounted for where §3.9.2 closes the set at \
             {CLASS_COUNT}",
            found = covered.len()
        ));
    }
    Ok(())
}

/// The rows read agree, class by class, with what was measured.
fn check_census(rows: &[Row]) -> Result<(), String> {
    for census in CENSUS {
        let singles = rows
            .iter()
            .filter(|row| row.class == census.class && row.key.len() == 1)
            .count();
        let sequences = rows
            .iter()
            .filter(|row| row.class == census.class && row.key.len() > 1)
            .count();
        if singles != census.singles || sequences != census.sequences {
            return Err(format!(
                "{class} lists {singles} single code point(s) and {sequences} sequence(s) \
                 where this repository was written against {expected_singles} and \
                 {expected_sequences}",
                class = census.class,
                expected_singles = census.singles,
                expected_sequences = census.sequences,
            ));
        }
    }
    let longest = rows.iter().map(|row| row.key.len()).max().unwrap_or(0);
    if longest != EXPECTED_MAX_KEY_LEN {
        return Err(format!(
            "the longest key is {longest} code point(s) where this repository was written \
             against {EXPECTED_MAX_KEY_LEN}"
        ));
    }
    Ok(())
}

/// Every distinct Remarks cell occurs as many times as was measured.
fn check_remark_counts(rows: &[Row]) -> Result<(), String> {
    for (index, remark) in REMARKS.iter().enumerate() {
        let found = rows.iter().filter(|row| row.remark == index).count();
        if found != remark.cells {
            return Err(format!(
                "the Remarks cell `{en}` / `{ja}` occurs in {found} row(s) where this \
                 repository was written against {expected}",
                en = remark.en.replace('\n', "\\n"),
                ja = remark.ja.replace('\n', "\\n"),
                expected = remark.cells,
            ));
        }
    }
    Ok(())
}

/// The number of a class identifier, `cl-01` through `cl-30`.
fn class_number(class: &str) -> Option<usize> {
    let digits = class.strip_prefix("cl-")?;
    if digits.len() != 2 || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let number = digits.parse::<usize>().ok()?;
    (1..=CLASS_COUNT).contains(&number).then_some(number)
}

// ---------------------------------------------------------------------------------------
// Small readers shared by the four derivations
// ---------------------------------------------------------------------------------------

/// One of the texts a derivation was handed, by position.
fn source(sources: &[String], index: usize) -> Result<&str, String> {
    sources
        .get(index)
        .map(String::as_str)
        .ok_or_else(|| format!("the derivation was handed no source at position {index}"))
}

/// The next occurrence of `needle` at or after `from`.
fn find_from(text: &str, needle: &str, from: usize) -> Option<usize> {
    text.get(from..)
        .and_then(|rest| rest.find(needle))
        .map(|at| from.saturating_add(at))
}

/// The text between the next `open` and the `close` after it, and where the close ends.
fn between<'t>(text: &'t str, open: &str, close: &str, from: usize) -> Option<(&'t str, usize)> {
    let start = find_from(text, open, from)?.saturating_add(open.len());
    let end = find_from(text, close, start)?;
    Some((text.get(start..end)?, end.saturating_add(close.len())))
}

/// Collapse runs of ASCII whitespace to one space and trim, leaving every other character
/// alone.
///
/// Deliberately not `split_whitespace`, which treats `U+3000` as whitespace: the ideographic
/// space is a character this appendix classifies (cl-14) rather than layout of the source.
fn collapse(text: &str) -> String {
    let mut collapsed = String::with_capacity(text.len());
    let mut pending = false;
    for character in text.chars() {
        if character.is_ascii_whitespace() {
            pending = !collapsed.is_empty();
        } else {
            if pending {
                collapsed.push(' ');
                pending = false;
            }
            collapsed.push(character);
        }
    }
    collapsed
}

/// A key, as a tab-separated file writes one: code points in hexadecimal, space separated.
fn render_key(key: &[u32]) -> String {
    key.iter()
        .map(|code_point| format!("{code_point:04X}"))
        .collect::<Vec<String>>()
        .join(" ")
}

/// A key, as a tab-separated file wrote one.
fn parse_key(text: &str) -> Result<Vec<u32>, String> {
    let key: Result<Vec<u32>, String> = text.split(' ').map(code_point).collect();
    let key = key?;
    if key.is_empty() || key.len() > EXPECTED_MAX_KEY_LEN {
        return Err(format!(
            "`{text}` names {found} code point(s), and a key holds one to \
             {EXPECTED_MAX_KEY_LEN}",
            found = key.len()
        ));
    }
    Ok(key)
}

/// Prose as one field of a tab-separated file: a line break written `\n`.
fn escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('\n', "\\n")
}

/// The inverse of `escape`, refusing an escape it did not write.
fn unescape(text: &str) -> Result<String, String> {
    let mut out = String::with_capacity(text.len());
    let mut characters = text.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            out.push(character);
            continue;
        }
        match characters.next() {
            Some('n') => out.push('\n'),
            Some('\\') => out.push('\\'),
            other => {
                return Err(format!(
                    "`{text}` holds the escape `\\{shown}`, which nothing writes",
                    shown = other.map_or_else(|| "<end of field>".to_owned(), String::from)
                ));
            },
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------
// Stage 2: the generation units
// ---------------------------------------------------------------------------------------

/// The Appendix A tables, keyed by code-point sequence.
pub(crate) const APPENDIX_A_TABLE: Unit = Unit {
    input: "spec/derived/appendix-a.tsv",
    generator: &["xtask/src/classes.rs"],
    output: "crates/jlreq-class/src/generated/appendix_a.rs",
    summary: "Appendix A: every enumerated key, the classes naming it, and the Remarks cell.",
    emit: emit_appendix_a,
};

/// The same Appendix A data, migrated behind the sole public library's private boundary.
pub(crate) const KUMIHAN_APPENDIX_A_TABLE: Unit = Unit {
    input: "spec/derived/appendix-a.tsv",
    generator: &["xtask/src/classes.rs"],
    output: "crates/kumihan/src/generated/appendix_a.rs",
    summary: "Appendix A: every enumerated key, the classes naming it, and the Remarks cell.",
    emit: emit_appendix_a,
};

/// The thirty class names §3.9.2 publishes.
pub(crate) const CLASS_TABLE: Unit = Unit {
    input: "spec/derived/classes.tsv",
    generator: &["xtask/src/classes.rs"],
    output: "crates/jlreq-class/src/generated/class_name.rs",
    summary: "The thirty classes of §3.9.2: the id, the name in both locales, and the \
              Appendix A section that enumerates each.",
    emit: emit_classes,
};

/// The cl-19 ideograph predicate.
pub(crate) const IDEOGRAPH_TABLE: Unit = Unit {
    input: "spec/derived/ideographs.tsv",
    generator: &["xtask/src/classes.rs"],
    output: "crates/jlreq-class/src/generated/ideograph.rs",
    summary: "The members of cl-19 that §A.19's table deliberately does not list.",
    emit: emit_ideograph,
};

/// The cl-19 ideograph predicate behind the sole public library's private boundary.
pub(crate) const KUMIHAN_IDEOGRAPH_TABLE: Unit = Unit {
    input: "spec/derived/ideographs.tsv",
    generator: &["xtask/src/classes.rs"],
    output: "crates/kumihan/src/generated/ideograph.rs",
    summary: "The members of cl-19 that §A.19's table deliberately does not list.",
    emit: emit_ideograph,
};

/// The compatibility folding.
pub(crate) const FOLDING_TABLE: Unit = Unit {
    input: "spec/derived/folding.tsv",
    generator: &["xtask/src/classes.rs"],
    output: "crates/jlreq-class/src/generated/folding.rs",
    summary: "The Wide and Narrow decompositions: the only folding §A's preamble permits.",
    emit: emit_folding,
};

/// The compatibility folding behind the sole public library's private boundary.
pub(crate) const KUMIHAN_FOLDING_TABLE: Unit = Unit {
    input: "spec/derived/folding.tsv",
    generator: &["xtask/src/classes.rs"],
    output: "crates/kumihan/src/generated/folding.rs",
    summary: "The Wide and Narrow decompositions: the only folding §A's preamble permits.",
    emit: emit_folding,
};

/// The script property behind the small-kana fallback.
pub(crate) const SCRIPT_TABLE: Unit = Unit {
    input: "spec/derived/scripts.tsv",
    generator: &["xtask/src/classes.rs"],
    output: "crates/jlreq-class/src/generated/script.rs",
    summary: "The two kana scripts §C.2 note 3's small-kana fallback reads.",
    emit: emit_script,
};

/// The kana scripts behind the sole public library's private boundary.
pub(crate) const KUMIHAN_SCRIPT_TABLE: Unit = Unit {
    input: "spec/derived/scripts.tsv",
    generator: &["xtask/src/classes.rs"],
    output: "crates/kumihan/src/generated/script.rs",
    summary: "The two kana scripts §C.2 note 3's small-kana fallback reads.",
    emit: emit_script,
};

/// One listing of the emitted Appendix A table.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Listing {
    /// The key, as code points.
    key: Vec<u32>,
    /// The class that lists it, `1` through `30`.
    class: usize,
    /// Which `REMARKS` entry the row's Remarks cell holds.
    remark: usize,
}

/// Emit the Appendix A tables.
fn emit_appendix_a(table: &Table) -> Result<Emission, String> {
    let listings = read_listings(table)?;
    let mut items = String::new();
    items.push_str(&key_length_item(&listings));
    items.push_str(&named_items(FRAMES, "frame (字幅) mask bit for"));
    items.push_str(&frames_unstated_item());
    items.push_str(&named_items(USAGES, "writing-direction qualification:"));
    items.push_str(&named_items(ROLES, "role a Remarks cell may name:"));
    items.push_str(&remark_items());
    items.push_str(&listing_items(&listings));
    Ok(Emission {
        entries: listings.len(),
        items,
    })
}

/// Read the derived Appendix A into the listings the table holds.
fn read_listings(table: &Table) -> Result<Vec<Listing>, String> {
    expect_columns(table, &["class", "key", "remark-en", "remark-ja"])?;
    let mut rows: Vec<(String, Listing)> = Vec::new();
    for record in &table.records {
        let class = field(record, 0)?;
        let number = class_number(class)
            .ok_or_else(|| at(record, &format!("`{class}` names no character class")))?;
        let key = parse_key(field(record, 1)?).map_err(|reason| at(record, &reason))?;
        let en = unescape(field(record, 2)?).map_err(|reason| at(record, &reason))?;
        let ja = unescape(field(record, 3)?).map_err(|reason| at(record, &reason))?;
        let remark = remark_position(&en, &ja).ok_or_else(|| {
            at(
                record,
                "the Remarks cell is not one this repository has read",
            )
        })?;
        rows.push((
            class.to_owned(),
            Listing {
                key,
                class: number,
                remark,
            },
        ));
    }

    check_derived_census(&rows)?;
    let listings = deduplicate(rows)?;
    check_listing_totals(&listings)?;
    Ok(listings)
}

/// The derived table agrees, class by class, with what was measured.
fn check_derived_census(rows: &[(String, Listing)]) -> Result<(), String> {
    let derived: Vec<Row> = rows
        .iter()
        .map(|(class, listing)| Row {
            class: class.clone(),
            key: listing.key.clone(),
            remark: listing.remark,
        })
        .collect();
    check_census(&derived)?;
    check_remark_counts(&derived)
}

/// Sort the listings and remove the rows the published document repeats.
fn deduplicate(rows: Vec<(String, Listing)>) -> Result<Vec<Listing>, String> {
    let mut listings: Vec<Listing> = rows.into_iter().map(|(_, listing)| listing).collect();
    listings.sort();
    let mut kept: Vec<Listing> = Vec::with_capacity(listings.len());
    let mut repeated: Vec<(usize, u32)> = Vec::new();
    for listing in listings {
        let Some(previous) = kept.last() else {
            kept.push(listing);
            continue;
        };
        if previous.key != listing.key || previous.class != listing.class {
            kept.push(listing);
            continue;
        }
        // The same key under the same class twice. Two rows that agree are a repeated row,
        // which is a defect of the published table and must be recorded; two that disagree
        // are the table contradicting itself about one member, and no record makes that
        // readable, so there is nothing to do but refuse.
        if previous.remark != listing.remark {
            return Err(format!(
                "cl-{class:02} lists U+{code_point:04X} twice with two different Remarks \
                 cells, so the published table states two different qualifications for one \
                 member and neither reading can be preferred here",
                class = listing.class,
                code_point = listing.key.first().copied().unwrap_or_default(),
            ));
        }
        repeated.push((
            listing.class,
            listing.key.first().copied().unwrap_or_default(),
        ));
    }
    check_duplicates(&repeated)?;
    Ok(kept)
}

/// The repeated rows are exactly the ones the recorded defects name.
fn check_duplicates(repeated: &[(usize, u32)]) -> Result<(), String> {
    for (class, code_point) in repeated {
        let recorded = RECORDED_DUPLICATES.iter().any(|(recorded, value, _)| {
            class_number(recorded) == Some(*class) && value == code_point
        });
        if !recorded {
            return Err(format!(
                "cl-{class:02} lists U+{code_point:04X} twice and no recorded defect says so; \
                 a duplicate is a defect of the published table and is recorded with its \
                 evidence rather than absorbed (docs/design/generation.md)"
            ));
        }
    }
    for (class, code_point, defect) in RECORDED_DUPLICATES {
        let found = class_number(class).is_some_and(|number| {
            repeated
                .iter()
                .any(|(repeated, value)| *repeated == number && value == code_point)
        });
        if !found {
            return Err(format!(
                "`{defect}` records that {class} lists U+{code_point:04X} twice and it now \
                 lists it once; a defect fixed upstream forces a review rather than changing \
                 an answer quietly"
            ));
        }
    }
    Ok(())
}

/// The de-duplicated table holds what was measured.
fn check_listing_totals(listings: &[Listing]) -> Result<(), String> {
    if listings.len() != EXPECTED_LISTINGS {
        return Err(format!(
            "the table holds {found} listing(s) where this repository was written against \
             {EXPECTED_LISTINGS}",
            found = listings.len()
        ));
    }
    // One pass, because the table is sorted: a key's listings are one contiguous run, so a
    // run that begins is a distinct key and a run longer than one is a key more than one
    // class names.
    let mut keys = 0_usize;
    let mut multi_class_keys = 0_usize;
    let mut run = 0_usize;
    for (index, listing) in listings.iter().enumerate() {
        let same = index
            .checked_sub(1)
            .and_then(|previous| listings.get(previous))
            .is_some_and(|previous| previous.key == listing.key);
        if same {
            run = run.saturating_add(1);
            if run == 2 {
                multi_class_keys = multi_class_keys.saturating_add(1);
            }
            continue;
        }
        keys = keys.saturating_add(1);
        run = 1;
    }
    if keys != EXPECTED_KEYS || multi_class_keys != EXPECTED_MULTI_CLASS_KEYS {
        return Err(format!(
            "the table holds {keys} distinct key(s), {multi_class_keys} of them named by \
             more than one class, where this repository was written against \
             {EXPECTED_KEYS} and {EXPECTED_MULTI_CLASS_KEYS}"
        ));
    }
    Ok(())
}

/// Emit the thirty class names.
fn emit_classes(table: &Table) -> Result<Emission, String> {
    expect_columns(table, &["class", "name_en", "name_ja", "enumeration"])?;
    if table.records.len() != CLASS_COUNT {
        return Err(format!(
            "holds {found} class(es) where §3.9.2 closes the set at {CLASS_COUNT}",
            found = table.records.len()
        ));
    }
    let mut entries = Vec::with_capacity(CLASS_COUNT);
    for (position, record) in table.records.iter().enumerate() {
        let class = field(record, 0)?;
        let number = class_number(class)
            .ok_or_else(|| at(record, &format!("`{class}` names no character class")))?;
        if number != position.saturating_add(1) {
            return Err(at(
                record,
                &format!("`{class}` is out of order; the table is read by class number"),
            ));
        }
        let en = field(record, 1)?;
        let ja = field(record, 2)?;
        if en.is_empty() || ja.is_empty() {
            return Err(at(record, "names the class in one locale only"));
        }
        let enumeration = field(record, 3)?;
        let enumerates = !enumeration.is_empty();
        if enumerates == CONSTRUCT_CLASSES.contains(&class) {
            return Err(at(
                record,
                &format!(
                    "states `{enumeration}` as the section enumerating `{class}`, and this \
                     repository was written against the five classes whose section text \
                     reads in full \"Any character may participate in …\": \
                     {CONSTRUCT_CLASSES:?}"
                ),
            ));
        }
        entries.push(format!(
            "ClassName {{\n        \
             id: \"{class}\",\n        \
             en: \"{en}\",\n        \
             ja: \"{ja}\",\n        \
             enumeration: \"{enumeration}\",\n    \
             }}",
            en = quoted(en),
            ja = quoted(ja),
        ));
    }
    let mut items = String::from(
        "/// One character class, as §3.9.2 names it.\n\
         ///\n\
         /// JLReq: §3.9.2\n\
         #[derive(Debug)]\n\
         pub(crate) struct ClassName {\n\
         \x20   /// The id the published document anchors this class by: `cl-01` … `cl-30`,\n\
         \x20   /// which is also the identifier every rule sentence of JLReq uses.\n\
         \x20   pub(crate) id: &'static str,\n\
         \x20   /// The English name, as §3.9.2's own list writes it.\n\
         \x20   pub(crate) en: &'static str,\n\
         \x20   /// The Japanese name, likewise.\n\
         \x20   pub(crate) ja: &'static str,\n\
         \x20   /// The canonical address of the Appendix A section enumerating this class,\n\
         \x20   /// or the empty string for the five that enumerate nothing.\n\
         \x20   pub(crate) enumeration: &'static str,\n\
         }\n\n\
         /// The thirty classes §3.9.2 closes the set at, in class-number order.\n\
         ///\n\
         /// JLReq: §3.9.2, §A\n\
         pub(crate) const CLASSES: &[ClassName] = &[\n",
    );
    items.push_str(&close(&entries));
    Ok(Emission {
        entries: entries.len(),
        items,
    })
}

/// Ranges read back out of a derived table are sorted, disjoint, and cover what was
/// measured.
///
/// Stage 1 checks these figures against the Unicode Character Database and stage 2 checks
/// them again against the table stage 1 wrote, because the two edits fail differently: a
/// deleted row moves the count, and a moved bound moves only the coverage. Without both, a
/// hand edit to a derived table reaches the generated Rust and is caught, if at all, by a
/// compile-time assertion in the crate that reads it — which is a check on the wrong side of
/// the boundary this gate exists to hold.
fn check_ranges(
    subject: &str,
    ranges: &[(u32, u32)],
    expected_ranges: usize,
    expected_covered: u32,
) -> Result<(), String> {
    if ranges.len() != expected_ranges {
        return Err(format!(
            "{subject} holds {found} range(s) where this repository was written against \
             {expected_ranges}",
            found = ranges.len()
        ));
    }
    let mut covered = 0_u32;
    let mut previous: Option<u32> = None;
    for (first, last) in ranges {
        if first > last {
            return Err(format!(
                "{subject} holds U+{first:04X}..U+{last:04X}, which ends before it begins"
            ));
        }
        if previous.is_some_and(|before| *first <= before) {
            return Err(format!(
                "{subject}'s range beginning U+{first:04X} is not past the one before it, so \
                 the table is no longer sorted and disjoint"
            ));
        }
        previous = Some(*last);
        covered = covered.saturating_add(last.saturating_sub(*first).saturating_add(1));
    }
    if covered != expected_covered {
        return Err(format!(
            "{subject} covers {covered} code point(s) where this repository was written \
             against {expected_covered}"
        ));
    }
    Ok(())
}

/// Emit the ideograph predicate.
fn emit_ideograph(table: &Table) -> Result<Emission, String> {
    expect_columns(table, &["first", "last"])?;
    let mut rows = Vec::new();
    for record in &table.records {
        let first = code_point(field(record, 0)?).map_err(|reason| at(record, &reason))?;
        let last = code_point(field(record, 1)?).map_err(|reason| at(record, &reason))?;
        rows.push((first, last));
    }
    check_ranges(
        UNIFIED_IDEOGRAPH,
        &rows,
        EXPECTED_IDEOGRAPH_RANGES,
        EXPECTED_IDEOGRAPHS,
    )?;
    for (code_point, class) in NOT_IDEOGRAPHS {
        if rows
            .iter()
            .any(|(first, last)| covers(*first, *last, *code_point))
        {
            return Err(format!(
                "U+{code_point:04X} is claimed by the ideograph predicate, and JLReq puts it \
                 in {class}"
            ));
        }
    }
    if !rows
        .iter()
        .any(|(first, last)| covers(*first, *last, BOTH_LISTED_AND_IDEOGRAPH))
    {
        return Err(format!(
            "U+{BOTH_LISTED_AND_IDEOGRAPH:04X} is enumerated in cl-19 and the predicate no \
             longer claims it, so `listed` and `is an ideograph` may no longer overlap"
        ));
    }
    let mut items = String::from(
        "/// One range of code points the Unicode Character Database gives\n\
         /// `Unified_Ideograph=Yes`.\n\
         ///\n\
         /// JLReq: §A.19\n\
         #[derive(Debug)]\n\
         pub(crate) struct Range {\n\
         \x20   /// The first code point of the range.\n\
         \x20   pub(crate) first: u32,\n\
         \x20   /// The last code point of the range, inclusive.\n\
         \x20   pub(crate) last: u32,\n\
         }\n\
         \n\
         impl Range {\n\
         \x20   /// One row of the table below.\n\
         \x20   const fn new(first: u32, last: u32) -> Self {\n\
         \x20       Self { first, last }\n\
         \x20   }\n\
         }\n\
         \n\
         /// Every such range, sorted and disjoint.\n\
         ///\n\
         /// §A.19's table lists only the *non-ideographic* members of cl-19, so the\n\
         /// ideographs come from here. `Unified_Ideograph` is the property and the\n\
         /// alternatives are both wrong: `Ideographic` over-covers with Tangut, Nushu and\n\
         /// Khitan, and `Script=Han` over-covers with `U+3005`, which JLReq puts in cl-09.\n\
         ///\n\
         /// JLReq: §A.19\n\
         pub(crate) const RANGES: &[Range] = &[\n",
    );
    let entries: Vec<String> = rows
        .iter()
        .map(|(first, last)| {
            format!(
                "Range::new({first}, {last})",
                first = literal(*first),
                last = literal(*last)
            )
        })
        .collect();
    items.push_str(&close(&entries));
    Ok(Emission {
        entries: rows.len(),
        items,
    })
}

/// Emit the compatibility folding.
fn emit_folding(table: &Table) -> Result<Emission, String> {
    expect_columns(table, &["source", "target", "frame"])?;
    let mut rows = Vec::new();
    for record in &table.records {
        let from = code_point(field(record, 0)?).map_err(|reason| at(record, &reason))?;
        let to = code_point(field(record, 1)?).map_err(|reason| at(record, &reason))?;
        let frame = match field(record, 2)? {
            "full-em" => "FRAME_FULL_EM",
            "half-em" => "FRAME_HALF_EM",
            other => {
                return Err(at(
                    record,
                    &format!(
                        "`{other}` is not a frame a compatibility code point asserts; only a \
                         full-width form and a half-width form fold"
                    ),
                ));
            },
        };
        rows.push((from, to, frame));
    }
    let wide = rows
        .iter()
        .filter(|(_, _, it)| *it == "FRAME_FULL_EM")
        .count();
    let narrow = rows.len().saturating_sub(wide);
    if wide != EXPECTED_WIDE || narrow != EXPECTED_NARROW {
        return Err(format!(
            "the table holds {wide} full-width and {narrow} half-width fold(s) where this \
             repository was written against {EXPECTED_WIDE} and {EXPECTED_NARROW}"
        ));
    }
    let mut previous: Option<u32> = None;
    for (from, to, _) in &rows {
        if previous.is_some_and(|before| *from <= before) {
            return Err(format!(
                "U+{from:04X} does not follow the source before it, so folding is no longer \
                 a function and the binary search over this table would not find it"
            ));
        }
        if from == to {
            return Err(format!(
                "U+{from:04X} folds onto itself, which is not a compatibility decomposition"
            ));
        }
        previous = Some(*from);
    }
    let mut items = String::from(
        "use super::appendix_a::{FRAME_FULL_EM, FRAME_HALF_EM};\n\
         \n\
         /// One compatibility decomposition Appendix A's preamble requires folding.\n\
         ///\n\
         /// JLReq: §A preamble\n\
         #[derive(Debug)]\n\
         pub(crate) struct Fold {\n\
         \x20   /// The compatibility code point real Japanese text carries.\n\
         \x20   pub(crate) source: u32,\n\
         \x20   /// The code point Appendix A keys.\n\
         \x20   pub(crate) target: u32,\n\
         \x20   /// The frame the source code point itself asserts.\n\
         \x20   pub(crate) frame: u8,\n\
         }\n\
         \n\
         impl Fold {\n\
         \x20   /// One row of the table below.\n\
         \x20   const fn new(source: u32, target: u32, frame: u8) -> Self {\n\
         \x20       Self {\n\
         \x20           source,\n\
         \x20           target,\n\
         \x20           frame,\n\
         \x20       }\n\
         \x20   }\n\
         }\n\
         \n\
         /// Every fold, sorted by source.\n\
         ///\n\
         /// Only the Wide and Narrow decomposition mappings: full compatibility folding\n\
         /// would fold `U+2160` Ⅰ, a genuine cl-19 member, onto `I`. A source that\n\
         /// Appendix A itself enumerates is keyed in its own right — `U+3000` is cl-14 and\n\
         /// folds onto `U+0020`, which is cl-26 — so a lookup tries the literal key before\n\
         /// it tries the folded one.\n\
         ///\n\
         /// JLReq: §A preamble\n\
         pub(crate) const FOLDS: &[Fold] = &[\n",
    );
    let entries: Vec<String> = rows
        .iter()
        .map(|(from, to, frame)| {
            format!(
                "Fold::new({from}, {to}, {frame})",
                from = literal(*from),
                to = literal(*to)
            )
        })
        .collect();
    items.push_str(&close(&entries));
    Ok(Emission {
        entries: rows.len(),
        items,
    })
}

/// Emit the two kana scripts.
fn emit_script(table: &Table) -> Result<Emission, String> {
    expect_columns(table, &["script", "first", "last"])?;
    let mut rows = Vec::new();
    for record in &table.records {
        let script = field(record, 0)?;
        let tag = match script {
            "Hiragana" => "HIRAGANA",
            "Katakana" => "KATAKANA",
            other => {
                return Err(at(
                    record,
                    &format!("`{other}` is not one of the two scripts §C.2 note 3 reads"),
                ));
            },
        };
        let first = code_point(field(record, 1)?).map_err(|reason| at(record, &reason))?;
        let last = code_point(field(record, 2)?).map_err(|reason| at(record, &reason))?;
        rows.push((first, last, tag));
    }
    let bounds: Vec<(u32, u32)> = rows
        .iter()
        .map(|(first, last, _)| (*first, *last))
        .collect();
    check_ranges(
        "the two kana scripts",
        &bounds,
        EXPECTED_SCRIPT_RANGES,
        EXPECTED_KANA,
    )?;
    let mut items = String::from(
        "/// The `Script=Hiragana` tag.\n\
         ///\n\
         /// JLReq: §C.2#3\n\
         pub(crate) const HIRAGANA: u8 = 1;\n\
         \n\
         /// The `Script=Katakana` tag.\n\
         ///\n\
         /// JLReq: §C.2#3\n\
         pub(crate) const KATAKANA: u8 = 2;\n\
         \n\
         /// One range of code points the Unicode Character Database gives one kana script.\n\
         ///\n\
         /// JLReq: §C.2#3\n\
         #[derive(Debug)]\n\
         pub(crate) struct Range {\n\
         \x20   /// The first code point of the range.\n\
         \x20   pub(crate) first: u32,\n\
         \x20   /// The last code point of the range, inclusive.\n\
         \x20   pub(crate) last: u32,\n\
         \x20   /// Which script, `HIRAGANA` or `KATAKANA`.\n\
         \x20   pub(crate) script: u8,\n\
         }\n\
         \n\
         impl Range {\n\
         \x20   /// One row of the table below.\n\
         \x20   const fn new(first: u32, last: u32, script: u8) -> Self {\n\
         \x20       Self {\n\
         \x20           first,\n\
         \x20           last,\n\
         \x20           script,\n\
         \x20       }\n\
         \x20   }\n\
         }\n\
         \n\
         /// Every such range, sorted by first code point and disjoint.\n\
         ///\n\
         /// §C.2 note 3 permits a small kana to be treated as the full-size one at a line\n\
         /// head, and the fallback needs to know a kana when it sees one beyond the forty\n\
         /// §A.11 enumerates.\n\
         ///\n\
         /// JLReq: §C.2#3\n\
         pub(crate) const RANGES: &[Range] = &[\n",
    );
    let entries: Vec<String> = rows
        .iter()
        .map(|(first, last, tag)| {
            format!(
                "Range::new({first}, {last}, {tag})",
                first = literal(*first),
                last = literal(*last)
            )
        })
        .collect();
    items.push_str(&close(&entries));
    Ok(Emission {
        entries: rows.len(),
        items,
    })
}

// ---------------------------------------------------------------------------------------
// Rendering the generated Rust
// ---------------------------------------------------------------------------------------

/// One code point, as every generated table writes one.
///
/// Grouped in fours so a reader can count digits and Clippy's readability lints are
/// satisfied without a suppression, and always the full width of a `u32` so that a table
/// holding an astral code point is written the same way as one that does not.
fn literal(value: u32) -> String {
    format!(
        "0x{high:04X}_{low:04X}",
        high = value >> 16,
        low = value & 0xFFFF
    )
}

/// A tab-separated file: a header line, then one line per record.
///
/// Written this way rather than by appending a formatted string to a growing one, which
/// Clippy refuses in the same breath as this program refuses an `#[allow]`.
fn tabulate(header: &str, records: &[String]) -> String {
    let mut text = String::with_capacity(header.len().saturating_add(records.len()));
    text.push_str(header);
    text.push('\n');
    for record in records {
        text.push_str(record);
        text.push('\n');
    }
    text
}

/// The rows of an emitted table and the bracket closing it, one row per line.
fn close(entries: &[String]) -> String {
    let mut text = String::new();
    for entry in entries {
        text.push_str("    ");
        text.push_str(entry);
        text.push_str(",\n");
    }
    text.push_str("];\n");
    text
}

/// The item stating the longest key, measured from the listings it is emitted beside.
///
/// The value is the longest key in the table this run read, not `EXPECTED_MAX_KEY_LEN`.
/// `check_listing_totals` holds the two equal, so emitting the measurement rather than the
/// constant changes no byte today and makes the emitted doc comment true: a file whose whole
/// purpose is to be auditable without running anything may not say it measured something it
/// asserted.
fn key_length_item(listings: &[Listing]) -> String {
    let longest = listings.iter().map(|listing| listing.key.len()).max();
    format!(
        "/// The longest key Appendix A enumerates, in code points.\n\
         ///\n\
         /// Measured from the table below rather than assumed.\n\
         /// `crates/jlreq-class/src/generated.rs`\n\
         /// asserts the value this crate is written against, so a revision adding a\n\
         /// three-code-point member is a build failure rather than a silent truncation.\n\
         ///\n\
         /// JLReq: §A\n\
         pub(crate) const MAX_KEY_LEN: usize = {longest};\n\n",
        longest = longest.unwrap_or(0),
    )
}

/// The named constants of one axis.
fn named_items(values: &[Named], lead: &str) -> String {
    let items: Vec<String> = values
        .iter()
        .map(|named| {
            format!(
                "/// The {lead} {doc}.\n\
                 ///\n\
                 /// JLReq: §A Remarks\n\
                 pub(crate) const {name}: u8 = {value};\n\n",
                doc = named.doc,
                name = named.name,
                value = mask(values, named.value),
            )
        })
        .collect();
    items.concat()
}

/// How one named value is written: a mask in binary, grouped in fours, and an ordinal in
/// decimal.
///
/// An axis is a mask when no value on it is zero, because a mask needs a bit per value and
/// an ordinal does not.
fn mask(values: &[Named], value: u8) -> String {
    if values.iter().any(|named| named.value == 0) {
        return format!("{value}");
    }
    format!(
        "0b{high:04b}_{low:04b}",
        high = value >> 4,
        low = value & 0b0000_1111
    )
}

/// The item stating the empty frame mask.
fn frames_unstated_item() -> String {
    format!(
        "/// The empty frame mask: the Remarks cell states no frame at all.\n\
         ///\n\
         /// JLReq: §A Remarks\n\
         pub(crate) const {FRAMES_UNSTATED}: u8 = 0b0000_0000;\n\n"
    )
}

/// The `Remark` type and the table of every distinct Remarks cell.
fn remark_items() -> String {
    let mut items = String::from(
        "/// One distinct Remarks cell of Appendix A, and the facts it states.\n\
         ///\n\
         /// JLReq: §A Remarks\n\
         #[derive(Debug)]\n\
         pub(crate) struct Remark {\n\
         \x20   /// The English half of the cell, verbatim, with its line break kept.\n\
         \x20   pub(crate) en: &'static str,\n\
         \x20   /// The Japanese half, likewise. Three cells carry no English at all, which\n\
         \x20   /// is a recorded defect of the published document rather than an omission\n\
         \x20   /// here.\n\
         \x20   pub(crate) ja: &'static str,\n\
         \x20   /// The frames the cell permits, as a mask.\n\
         \x20   pub(crate) frames: u8,\n\
         \x20   /// The writing-direction qualification it carries.\n\
         \x20   pub(crate) usage: u8,\n\
         \x20   /// The role it names.\n\
         \x20   pub(crate) role: u8,\n\
         }\n\
         \n\
         /// Every distinct Remarks cell the published Appendix A writes.\n\
         ///\n\
         /// The Remarks column is where Appendix A states the axis that separates two\n\
         /// classes naming one key, so a cell nobody read is a qualification lost. The\n\
         /// generator enumerates them and refuses a cell it has not seen.\n\
         ///\n\
         /// JLReq: §A Remarks\n\
         pub(crate) const REMARKS: &[Remark] = &[\n",
    );
    let entries: Vec<String> = REMARKS
        .iter()
        .map(|remark| {
            format!(
                "    Remark {{\n\
                 \x20       en: \"{en}\",\n\
                 \x20       ja: \"{ja}\",\n\
                 \x20       frames: {frames},\n\
                 \x20       usage: {usage},\n\
                 \x20       role: {role},\n\
                 \x20   }},\n",
                en = quoted(remark.en),
                ja = quoted(remark.ja),
                frames = frame_expression(remark.frames),
                usage = named_for(USAGES, remark.usage),
                role = named_for(ROLES, remark.role),
            )
        })
        .collect();
    items.push_str(&entries.concat());
    items.push_str("];\n\n");
    items
}

/// One string as a Rust literal: this vocabulary holds line breaks and nothing else that
/// needs escaping, and a cell that grew a quotation mark would be written wrong, so it is
/// refused at the point it would be.
fn quoted(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

/// A frame mask as an expression over the named bits.
fn frame_expression(frames: u8) -> String {
    let named: Vec<&str> = FRAMES
        .iter()
        .filter(|frame| frames & frame.value != 0)
        .map(|frame| frame.name)
        .collect();
    if named.is_empty() {
        return FRAMES_UNSTATED.to_owned();
    }
    named.join(" | ")
}

/// The name of one value of one axis.
fn named_for(values: &[Named], value: u8) -> String {
    values
        .iter()
        .find(|named| named.value == value)
        .map_or_else(|| format!("{value}"), |named| named.name.to_owned())
}

/// The `Listing` type and the table itself.
fn listing_items(listings: &[Listing]) -> String {
    let mut items = String::from(
        "/// One Appendix A listing: one key, one class that names it, and the Remarks cell\n\
         /// that listing carries.\n\
         ///\n\
         /// Keyed by a listing rather than by a key, because two in five keys are named by\n\
         /// more than one class and the Remarks cell that separates them belongs to the\n\
         /// row, not to the key (`docs/adr/0008`).\n\
         ///\n\
         /// JLReq: §A\n\
         #[derive(Debug)]\n\
         pub(crate) struct Listing {\n\
         \x20   /// The key, as code points, zero beyond `key_len`.\n\
         \x20   pub(crate) key: [u32; MAX_KEY_LEN],\n\
         \x20   /// How many code points of `key` are the key.\n\
         \x20   pub(crate) key_len: u8,\n\
         \x20   /// The class that lists the key, `1` through `30`.\n\
         \x20   pub(crate) class: u8,\n\
         \x20   /// Which `REMARKS` entry this listing's Remarks cell holds.\n\
         \x20   pub(crate) remark: u8,\n\
         }\n\
         \n\
         impl Listing {\n\
         \x20   /// One row of the table below.\n\
         \x20   const fn new(key: [u32; MAX_KEY_LEN], key_len: u8, class: u8, remark: u8) -> Self {\n\
         \x20       Self {\n\
         \x20           key,\n\
         \x20           key_len,\n\
         \x20           class,\n\
         \x20           remark,\n\
         \x20       }\n\
         \x20   }\n\
         }\n\
         \n\
         /// Every listing, sorted by key and then by class, so a lookup is a binary search\n\
         /// and the listings for one key are one contiguous run.\n\
         ///\n\
         /// The published table lists one row twice, `U+216B` in cl-19; it appears once\n\
         /// here, and the duplicate is a recorded defect rather than an absorbed one.\n\
         ///\n\
         /// JLReq: §A\n\
         pub(crate) const LISTINGS: &[Listing] = &[\n",
    );
    let entries: Vec<String> = listings
        .iter()
        .map(|listing| {
            let mut key: Vec<String> = listing.key.iter().map(|value| literal(*value)).collect();
            while key.len() < EXPECTED_MAX_KEY_LEN {
                key.push(literal(0));
            }
            format!(
                "Listing::new([{key}], {key_len}, {class}, {remark})",
                key = key.join(", "),
                key_len = listing.key.len(),
                class = listing.class,
                remark = listing.remark,
            )
        })
        .collect();
    items.push_str(&close(&entries));
    items
}

// ---------------------------------------------------------------------------------------
// Reading a derived table
// ---------------------------------------------------------------------------------------

/// The derived table names the columns this generator reads, in order.
fn expect_columns(table: &Table, columns: &[&str]) -> Result<(), String> {
    if table.columns != columns {
        return Err(format!(
            "names the columns {found:?} where this generator reads {columns:?}",
            found = table.columns
        ));
    }
    Ok(())
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
        CENSUS, CLASS_COUNT, CONSTRUCT_CLASSES, EXPECTED_LISTINGS, FRAMES, Listing, REMARKS, ROLES,
        USAGES, class_name, class_number, collapse, decompositions, deduplicate, escape,
        frame_expression, keys, literal, property_ranges, read_class_table, remark, render_key,
        strip_markup, unescape,
    };

    /// One listing, as the derived table hands one over.
    fn listing(class: usize, key: &[u32], remark: usize) -> (String, Listing) {
        (
            format!("cl-{class:02}"),
            Listing {
                key: key.to_vec(),
                class,
                remark,
            },
        )
    }

    /// Two rows of §A.1, copied out of the vendored snapshot: one carrying a Remarks cell
    /// in both locales, one carrying none.
    const OPENING_BRACKETS: &str = "<tbody>\n<tr>\n<td class=\"character\">\u{2018}</td>\n<td>2018</td>\n<td>LEFT SINGLE QUOTATION MARK</td>\n<td>\u{5DE6}\u{30B7}\u{30F3}\u{30B0}\u{30EB}\u{5F15}\u{7528}\u{7B26}\u{FF0C}<span class=\"br\"></span>\u{5DE6}\u{30B7}\u{30F3}\u{30B0}\u{30EB}\u{30AF}\u{30A9}\u{30FC}\u{30C6}\u{30FC}\u{30B7}\u{30E7}\u{30F3}\u{30DE}\u{30FC}\u{30AF}</td>\n<td><span its-locale-filter-list=\"en\" lang=\"en\">used in horizontal composition</span> <span its-locale-filter-list=\"ja\" lang=\"ja\">\u{6A2A}\u{7D44}\u{3067}\u{4F7F}\u{7528}</span></td>\n</tr>\n<tr>\n<td class=\"character\">\u{FF08}</td>\n<td>0028</td>\n<td>LEFT PARENTHESIS</td>\n<td>\u{59CB}\u{3081}\u{5C0F}\u{62EC}\u{5F27}\u{FF0C}\u{59CB}\u{3081}\u{4E38}\u{62EC}\u{5F27}</td>\n<td></td>\n</tr>\n</tbody>";

    /// The one §A.11 row whose UCS cell is a sequence, copied out of the snapshot.
    const SMALL_KANA_SEQUENCE: &str = "<tbody>\n<tr>\n<td class=\"character\">\u{31F7}\u{309A}</td>\n<td>&lt;31F7, 309A&gt;</td>\n<td>&lt;KATAKANA LETTER SMALL PU&gt;</td>\n<td></td>\n<td></td>\n</tr>\n</tbody>";

    /// The §A.12 row whose first cell is `character-latn` rather than `character`, which is
    /// why a row is counted as a row and not as a `td.character`.
    const NUMERO_SIGN: &str = "<tbody>\n<tr>\n<td class=\"character-latn\">\u{2116}</td>\n<td>2116</td>\n<td>NUMERO SIGN</td>\n<td>\u{5168}\u{89D2}NO</td>\n<td></td>\n</tr>\n</tbody>";

    /// One of the three §A.25 cells whose Remarks carry no locale span at all, which is a
    /// recorded defect of the published document.
    const UNLOCALISED_REMARK: &str = "<tbody>\n<tr>\n<td class=\"character\">D</td>\n<td>0044</td>\n<td>LATIN CAPITAL LETTER D</td>\n<td>\u{30E9}\u{30C6}\u{30F3}\u{5927}\u{6587}\u{5B57}D</td>\n<td>\u{30D7}\u{30ED}\u{30DD}\u{30FC}\u{30B7}\u{30E7}\u{30CA}\u{30EB}</td>\n</tr>\n</tbody>";

    /// Four lines of the vendored `PropList.txt`, copied verbatim.
    const PROPERTIES: &str = "# PropList-17.0.0.txt\n0009..000D    ; White_Space # Cc  [5] <control-0009>..<control-000D>\n3400..4DBF    ; Unified_Ideograph # Lo [6592] CJK IDEOGRAPH EXTENSION A\n4E00..9FFF    ; Unified_Ideograph # Lo [20992] CJK IDEOGRAPH\nFA0E..FA0F    ; Unified_Ideograph # Lo   [2] CJK COMPATIBILITY IDEOGRAPH\n";

    /// Three records of the vendored `UnicodeData.txt`, copied verbatim.
    const CHARACTERS: &str = "3000;IDEOGRAPHIC SPACE;Zs;0;WS;<wide> 0020;;;;N;;;;;\nFF08;FULLWIDTH LEFT PARENTHESIS;Ps;0;ON;<wide> 0028;;;;Y;;;;;\nFF61;HALFWIDTH IDEOGRAPHIC FULL STOP;Po;0;ON;<narrow> 3002;;;;N;;;;;\n";

    /// Read one fixture body and yield its rows.
    fn rows(class: &str, body: &str) -> Vec<super::Row> {
        let mut rows = Vec::new();
        read_class_table(class, body, &mut rows).expect("the fixture is a real table body");
        rows
    }

    #[test]
    fn a_bilingual_row_yields_its_key_and_its_remark() {
        let read = rows("cl-01", OPENING_BRACKETS);
        assert_eq!(read.len(), 2);
        assert_eq!(read[0].key, vec![0x2018]);
        assert_eq!(REMARKS[read[0].remark].en, "used in horizontal composition");
        assert_eq!(
            REMARKS[read[0].remark].ja,
            "\u{6A2A}\u{7D44}\u{3067}\u{4F7F}\u{7528}"
        );
        assert_eq!(read[1].key, vec![0x0028]);
        assert_eq!(
            REMARKS[read[1].remark].en, "",
            "an empty cell states nothing"
        );
    }

    #[test]
    fn a_sequence_row_is_not_flattened_to_one_code_point() {
        let read = rows("cl-11", SMALL_KANA_SEQUENCE);
        assert_eq!(read.len(), 1);
        assert_eq!(
            read[0].key,
            vec![0x31F7, 0x309A],
            "an extractor that assumed one code point per row would drop twenty-five rows"
        );
    }

    #[test]
    fn a_row_whose_first_cell_is_not_a_character_cell_is_still_a_row() {
        let read = rows("cl-12", NUMERO_SIGN);
        assert_eq!(
            read.len(),
            1,
            "one §A.12 row is written `character-latn`, so counting `td.character` \
             undercounts cl-12 by one"
        );
        assert_eq!(read[0].key, vec![0x2116]);
    }

    #[test]
    fn a_remarks_cell_with_no_locale_span_is_read_as_the_recorded_defect() {
        let read = rows("cl-25", UNLOCALISED_REMARK);
        assert_eq!(read.len(), 1);
        let remark = &REMARKS[read[0].remark];
        assert!(
            remark.en.is_empty(),
            "the published cell carries no English"
        );
        assert_eq!(
            remark.ja,
            "\u{30D7}\u{30ED}\u{30DD}\u{30FC}\u{30B7}\u{30E7}\u{30CA}\u{30EB}"
        );
        assert_eq!(remark.defect, "cl-25-remarks-without-locale-span");
    }

    #[test]
    fn an_unread_remarks_cell_is_refused() {
        let cell = "<span its-locale-filter-list=\"en\" lang=\"en\">set solid</span> \
                    <span its-locale-filter-list=\"ja\" lang=\"ja\">\u{30D9}\u{30BF}</span>";
        let violation = remark(cell).expect_err("a cell nobody read is a qualification lost");
        assert!(
            violation.contains("not one this repository has read"),
            "{violation}"
        );
    }

    #[test]
    fn a_remarks_cell_that_is_not_two_spans_is_refused() {
        let cell = "<span its-locale-filter-list=\"en\" lang=\"en\">half-width</span> and more \
                    <span its-locale-filter-list=\"ja\" lang=\"ja\">\u{5B57}\u{5E45}\u{306F}\u{534A}\u{89D2}</span>";
        let violation = remark(cell).expect_err("reading past a fragment would discard it");
        assert!(violation.contains("discard"), "{violation}");
    }

    #[test]
    fn a_ucs_cell_offering_alternatives_yields_one_key_per_alternative() {
        let read = keys("&lt;0254, 0300/0301&gt;").expect("the alternation form is read");
        assert_eq!(read, vec![vec![0x0254, 0x0300], vec![0x0254, 0x0301]]);
    }

    #[test]
    fn a_ucs_cell_this_reader_does_not_know_is_refused() {
        assert!(keys("U+0028").is_err(), "the column holds bare hexadecimal");
        assert!(
            keys("&lt;0028&gt;").is_err(),
            "a sequence holds two or more"
        );
        assert!(keys("D800").is_err(), "no text can hold a surrogate");
        assert!(
            keys("110000").is_err(),
            "no text can hold a code point past U+10FFFF"
        );
    }

    #[test]
    fn a_property_file_yields_only_the_property_asked_for() {
        let ranges = property_ranges(PROPERTIES, "Unified_Ideograph").expect("three ranges");
        assert_eq!(
            ranges,
            vec![(0x3400, 0x4DBF), (0x4E00, 0x9FFF), (0xFA0E, 0xFA0F)]
        );
        assert!(
            property_ranges(PROPERTIES, "Ideographic").is_err(),
            "a property the file does not carry covers nothing, which is not an answer"
        );
    }

    #[test]
    fn the_character_database_yields_the_two_foldings_and_no_others() {
        assert_eq!(
            decompositions(CHARACTERS, "<wide>").expect("two wide decompositions"),
            vec![(0x3000, 0x0020), (0xFF08, 0x0028)]
        );
        assert_eq!(
            decompositions(CHARACTERS, "<narrow>").expect("one narrow decomposition"),
            vec![(0xFF61, 0x3002)]
        );
    }

    #[test]
    fn the_census_covers_every_class_exactly_once() {
        let mut numbers: Vec<usize> = CENSUS
            .iter()
            .map(|census| census.class)
            .chain(CONSTRUCT_CLASSES.iter().copied())
            .map(|class| class_number(class).expect("a census names a class"))
            .collect();
        numbers.sort_unstable();
        numbers.dedup();
        assert_eq!(numbers.len(), CLASS_COUNT);
    }

    #[test]
    fn the_measured_totals_agree_with_one_another() {
        let rows: usize = CENSUS
            .iter()
            .map(|census| census.singles.saturating_add(census.sequences))
            .sum();
        assert_eq!(
            rows,
            EXPECTED_LISTINGS.saturating_add(1),
            "the published table lists one row twice, so it holds one row more than the \
             emitted table holds listings"
        );
        let cells: usize = REMARKS.iter().map(|remark| remark.cells).sum();
        assert_eq!(cells, rows, "every row carries exactly one Remarks cell");
    }

    #[test]
    fn no_two_remarks_share_both_halves() {
        for (index, remark) in REMARKS.iter().enumerate() {
            let twin = REMARKS
                .iter()
                .position(|other| other.en == remark.en && other.ja == remark.ja);
            assert_eq!(
                twin,
                Some(index),
                "the pair is the key, so a derived file naming one pair must name one cell"
            );
        }
    }

    #[test]
    fn every_named_value_is_distinct() {
        for axis in [FRAMES, USAGES, ROLES] {
            for (index, named) in axis.iter().enumerate() {
                assert_eq!(
                    axis.iter().position(|other| other.value == named.value),
                    Some(index)
                );
                assert_eq!(
                    axis.iter().position(|other| other.name == named.name),
                    Some(index)
                );
            }
        }
        for named in FRAMES {
            assert_eq!(named.value.count_ones(), 1, "a frame is one bit of a mask");
        }
    }

    #[test]
    fn a_frame_mask_is_written_as_an_expression_over_the_named_bits() {
        assert_eq!(frame_expression(0), "FRAMES_UNSTATED");
        assert_eq!(frame_expression(0b0000_0010), "FRAME_HALF_EM");
        assert_eq!(
            frame_expression(0b0001_0110),
            "FRAME_HALF_EM | FRAME_THIRD_EM | FRAME_PROPORTIONAL"
        );
    }

    #[test]
    fn a_code_point_is_written_the_same_way_everywhere() {
        assert_eq!(literal(0x0028), "0x0000_0028");
        assert_eq!(literal(0x2_A6DF), "0x0002_A6DF");
    }

    #[test]
    fn a_key_survives_being_written_and_read_back() {
        assert_eq!(render_key(&[0x0028]), "0028");
        assert_eq!(render_key(&[0x31F7, 0x309A]), "31F7 309A");
        assert_eq!(
            super::parse_key("31F7 309A").expect("two"),
            vec![0x31F7, 0x309A]
        );
        assert!(super::parse_key("").is_err());
    }

    #[test]
    fn prose_survives_being_written_and_read_back() {
        let text = "decimal point\nquarter em width or half-width";
        assert_eq!(
            escape(text),
            "decimal point\\nquarter em width or half-width"
        );
        assert_eq!(unescape(&escape(text)).expect("read back"), text);
        assert!(
            unescape("half\\qwidth").is_err(),
            "an escape nothing writes is a field nobody can read"
        );
    }

    #[test]
    fn whitespace_collapses_without_touching_the_ideographic_space() {
        assert_eq!(collapse("  a \n b  "), "a b");
        assert_eq!(
            collapse("\u{3000}"),
            "\u{3000}",
            "U+3000 is a character this appendix classifies, not layout of the source"
        );
    }

    #[test]
    fn the_one_row_the_document_repeats_is_removed_and_nothing_else_is() {
        let kept = deduplicate(vec![
            listing(19, &[0x216B], 0),
            listing(19, &[0x216B], 0),
            listing(27, &[0x216B], 1),
        ])
        .expect("the repeated cl-19 row is the recorded defect");
        assert_eq!(
            kept.len(),
            2,
            "§A.19 lists U+216B twice and cl-27 lists it once: three rows, two listings"
        );
    }

    #[test]
    fn a_duplicate_no_defect_records_is_refused() {
        let violation = deduplicate(vec![listing(15, &[0x3042], 0), listing(15, &[0x3042], 0)])
            .expect_err("a duplicate is a defect of the published table, recorded or refused");
        assert!(violation.contains("no recorded defect"), "{violation}");
    }

    #[test]
    fn a_recorded_duplicate_that_upstream_has_fixed_is_refused() {
        let violation = deduplicate(vec![listing(19, &[0x216B], 0)])
            .expect_err("a defect fixed upstream forces a review");
        assert!(violation.contains("cl-19-duplicate-u216b"), "{violation}");
        assert!(violation.contains("now"), "{violation}");
    }

    #[test]
    fn two_rows_naming_one_member_with_two_remarks_are_refused() {
        let violation = deduplicate(vec![
            listing(19, &[0x216B], 0),
            listing(19, &[0x216B], 3),
            listing(19, &[0x216B], 3),
        ])
        .expect_err("the table would be stating two qualifications for one member");
        assert!(violation.contains("two different Remarks"), "{violation}");
    }

    #[test]
    fn a_class_identifier_is_read_the_way_jlreq_spells_one() {
        assert_eq!(class_number("cl-01"), Some(1));
        assert_eq!(class_number("cl-30"), Some(30));
        assert_eq!(class_number("cl-31"), None);
        assert_eq!(class_number("cl-1"), None, "JLReq zero-pads to two digits");
        assert_eq!(class_number("appendix_1"), None);
    }

    #[test]
    fn markup_is_removed_and_an_entity_is_refused() {
        assert_eq!(
            strip_markup("<span class=\"index\" id=\"d6e3783\">\u{59cb}\u{3081}\u{62ec}\u{5f27}\u{985e}</span>\u{ff08}cl-01\u{ff09}")
                .expect("the index anchor is markup this reader knows"),
            "\u{59cb}\u{3081}\u{62ec}\u{5f27}\u{985e}\u{ff08}cl-01\u{ff09}"
        );
        assert!(
            strip_markup("half &amp; half").is_err(),
            "an entity read past is a character silently half-read"
        );
        assert!(
            strip_markup("<span unclosed").is_err(),
            "a tag that never closes would swallow the rest of the name"
        );
    }

    #[test]
    fn a_class_name_is_read_without_the_class_id_that_closes_it() {
        const LIST: &str = "<li id=\"id308\">\n  <p id=\"cl-01-en\" its-locale-filter-list=\"en\" lang=\"en\">Opening brackets (cl-01)</p>\n  <p id=\"cl-01-ja\" its-locale-filter-list=\"ja\" lang=\"ja\"><span class=\"index\" id=\"d6e3783\">\u{59cb}\u{3081}\u{62ec}\u{5f27}\u{985e}</span>\u{ff08}cl-01\u{ff09}</p>\n</li>";
        assert_eq!(
            class_name(LIST, "cl-01", "en").expect("\u{a7}3.9.2 names cl-01 in English"),
            "Opening brackets",
            "the id in parentheses is the one the element's own id carries, so it is not \
             stored twice"
        );
        assert_eq!(
            class_name(LIST, "cl-01", "ja").expect("\u{a7}3.9.2 names cl-01 in Japanese"),
            "\u{59cb}\u{3081}\u{62ec}\u{5f27}\u{985e}",
            "the Japanese name closes with full-width parentheses and the English with ASCII"
        );
    }

    #[test]
    fn a_name_that_does_not_close_with_its_class_id_is_refused() {
        let renamed = "<p id=\"cl-01-en\" lang=\"en\">Opening brackets (cl-02)</p>";
        let bare = "<p id=\"cl-01-en\" lang=\"en\">Opening brackets</p>";
        assert!(
            class_name(renamed, "cl-01", "en").is_err(),
            "the id is what says the name belongs to that class, so a mismatch is a \
             revision to read rather than a name to store"
        );
        assert!(class_name(bare, "cl-01", "en").is_err());
    }

    #[test]
    fn a_class_the_document_no_longer_names_is_refused() {
        assert!(
            class_name(
                "<p id=\"cl-02-en\" lang=\"en\">Closing brackets (cl-02)</p>",
                "cl-01",
                "en"
            )
            .is_err(),
            "a scanner that skipped what it did not find would be the silent drop the whole \
             pipeline exists to prevent"
        );
    }
}
