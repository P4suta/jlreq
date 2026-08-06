// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `attest` gate.
//!
//! W3C publishes Appendix A as markup and Appendices B through E as PDF, so ADR 0009
//! splits the specification data in two: derived data is generated, and the roughly 5400
//! cells of Tables 1 through 6 are transcribed. This gate is the control that earns the
//! second half of that split. It checks that the transcription is confined to one
//! directory, that every cell was entered twice — once from the English and once from the
//! Japanese rendering, which W3C publishes as separate documents — that every cell records
//! its provenance, that the two entries agree, and that the cross-table invariants of
//! `docs/design/generation.md` hold. With `--digests` it additionally verifies any
//! documents a developer placed in the gitignored `spec/upstream/` against
//! `spec/PROVENANCE.toml`. It never fetches anything, because a gate that needs the network
//! is a gate that fails for reasons unrelated to the code.
//!
//! Double entry is a procedural control and this gate says so rather than pretending
//! otherwise: it can require that the two files were keyed by different people in different
//! orders, and it does, but a systematic misreading survives that. The mechanical control is
//! the invariant catalogue below, every entry of which is derived from prose that *is*
//! machine-readable.
//!
//! # What it examines
//!
//! Every run prints a census: how many cells were read, of how many the six matrices hold;
//! how many invariants ran and which of them await an input a later milestone emits; and
//! which control files were absent. Until the transcription lands there is nothing to
//! object to, and the census is what makes that different from reporting that a check
//! passed. No check is conditioned on a milestone: each one runs the moment its subject
//! exists.
//!
//! # The formats this gate reads
//!
//! `spec/captured/table<N>.<locale>.tsv`, for `N` in 1 through 6 and `locale` in `en` and
//! `ja`, is one matrix read from one rendering. A comment preamble carries the capture
//! block, then the six columns of `docs/design/generation.md`:
//!
//! ```text
//! # [capture]
//! # author = "A. Transcriber"
//! # date = "2026-08-07"
//! source | table | before | after | token | note
//! table_en2.pdf | 1 | cl-05 | cl-05 | 1/4 be + 1/4 af | B.2#3
//! table_en2.pdf | 1 | cl-02 | line-end | 1/2 be | B.2#2
//! ```
//!
//! The separator is a tab. The bars stand in for it above, because a tab inside a doc
//! comment is itself a lint.
//!
//! `source` is the published file the cell was read from, and the off-by-one in the upstream
//! filenames is checked rather than absorbed: Table 1 is `table_en2.pdf`. `before` and
//! `after` are the row and column labels as printed — `cl-01` through `cl-30`, plus
//! `line-head` on the row axis and `line-end` on the column axis of the four matrices
//! that have them. Those two carry a hyphen because that is how the address space spells
//! them, and ADR 0013's one mechanical claim is that a rule has one spelling in the
//! capture, in the generated inventory, in a doc comment and in a case file — so there is
//! nothing between this file and `B.1@cl-02,line-end` for anyone to translate. `token` is the legend token; an empty cell is written `blank`, because a
//! trailing empty field cannot be told from a truncated row. `note` is the appendix note
//! that qualifies the cell, written `B.2#3` and drawn from that appendix's own note list —
//! §B.2 for Table 1, §C.2 for Table 2, §D.2 for Tables 3 through 5, §E.2 for Table 6. It is
//! the only column that may be empty.
//!
//! The token vocabulary is the legends', written in the fraction notation rather than in
//! either language's words, because the datum is the amount and not its spelling:
//!
//! ```text
//! any table     ×                       the adjacency is prohibited
//!               blank                   an empty cell
//! Table 1       1/4 be + 1/4 af         amounts, each taken from one neighbor's em
//!               1/2 be hang             ruby may extend over that space (§B.1)
//!               ruby hang               ruby may extend over the character itself
//! Table 2       not                     a break is prohibited at all four levels
//!               not 3,4                 prohibited at §C.3 levels 3 and 4 only
//! Tables 3-6    1/2-0 stage 4           movable to a limit, at a priority stage: a floor
//!                                       in Appendix D, a ceiling in Appendix E
//!               1/2=0 stage 2           two-valued: the amount or the limit (§3.1.9)
//!               1/4 stage 3             rigid at that stage
//!               residual                §3.8.4 step (d), Table 6 only
//! ```
//!
//! The stage ordinal is a word here because the published tables encode it as a cell
//! background color whose key is a raster image, so it exists nowhere as text. An en dash is
//! accepted wherever a hyphen is. A token outside this vocabulary is a violation rather than
//! an ignored string, so extending the vocabulary is a deliberate edit to one function.
//!
//! Any other capture is a pair on the same terms — `spec/captured/figures.<locale>.tsv`
//! holds the arrangements JLReq states only in a figure (§3.4.3, §3.7.2). Its columns
//! belong to the workflow that transcribes it, so the gate checks what is true of every
//! capture and says plainly that it has not read the payload.
//!
//! `spec/captured/invariants.tsv` publishes the catalogue — `id`, `citation`, `sentence` —
//! and the gate requires its identifiers and its citations to be exactly the ones
//! implemented here, so a published invariant that no code enforces fails the build.
//!
//! `spec/derived/defects.tsv` records the defects of the published document — `id`, `where`,
//! `evidence` — and the gate requires its identifiers to be exactly the ones this design
//! records, so a defect fixed upstream forces a review instead of changing behavior quietly.
//!
//! `spec/PROVENANCE.toml` records one `[[document]]` per upstream file, with `path` relative
//! to `spec/`, `url`, `retrieved` and `sha256`.
//!
//! See `docs/design/generation.md`, `docs/adr/0009`, `docs/adr/0007` and `docs/adr/0014`.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

use crate::shared::{self, Gate};

/// The `attest` gate, as the dispatcher sees it.
pub(crate) const GATE: Gate = Gate {
    name: "attest",
    purpose: concat!(
        "every transcribed cell examined is double-entered, carries its provenance, ",
        "and satisfies the cross-table invariants"
    ),
    reference: "docs/design/generation.md",
    run,
};

/// The fixed-point denominator (ADR 0007). Every amount must be an exact multiple of it.
const UNITS_PER_EM: i64 = 720;

/// A half em, in units of 1/720.
const HALF_EM: i32 = 360;

/// A quarter em, in units of 1/720.
const QUARTER_EM: i32 = 180;

/// Solid setting, in units of 1/720.
const SOLID: i32 = 0;

/// The number of character classes, `cl-01` through `cl-30`.
const CLASS_COUNT: u8 = 30;

/// The four strictness levels of §C.3, lowest to highest.
const LEVELS: [u8; 4] = [1, 2, 3, 4];

/// The strictest level, at which §3.1.7's and §3.1.8's prohibitions are read.
const VERY_STRICT: u8 = 4;

/// The two renderings every capture is entered from.
const LOCALES: [&str; 2] = ["en", "ja"];

/// The published catalogue of the invariants below.
const CATALOGUE: &str = "invariants.tsv";

/// The columns of a transcribed matrix, in order.
const MATRIX_COLUMNS: [&str; 6] = ["source", "table", "before", "after", "token", "note"];

/// The columns of the published invariant catalogue, in order.
const CATALOGUE_COLUMNS: [&str; 3] = ["id", "citation", "sentence"];

/// The columns of the recorded defect list, in order.
const DEFECT_COLUMNS: [&str; 3] = ["id", "where", "evidence"];

/// How many violations of one kind are printed before the rest are counted instead.
///
/// A systematic transcription error touches thousands of cells, and a gate that answers it
/// with thousands of lines is a gate nobody reads. The count is never suppressed.
const REPORTED_PER_KIND: usize = 8;

/// Directories the confinement scan does not walk.
const SKIPPED_DIRECTORIES: [&str; 2] = [".git", "target"];

/// The defects of the published document that `docs/design/generation.md` records.
///
/// One identifier per row of that document's table. `spec/derived/defects.tsv` must record
/// exactly these: a defect that disappears upstream fails this gate and forces a review,
/// rather than changing an answer quietly (ADR 0009).
const RECORDED_DEFECTS: [&str; 12] = [
    "cl-19-duplicate-u216b",
    "cl-25-remarks-without-locale-span",
    "cl-24-remarks-role-stated-only-in-japanese",
    "d2-note-5-priority-contradiction",
    "reduction-step-1-locale-divergence",
    "b2-note-11-simple-ruby-misnomer",
    "b2-note-7-locale-class-divergence",
    "line-composition-note-locale-divergence",
    "dividing-punctuation-note-unresolved-reference",
    "appendix-d-table-numbering-off-by-one",
    "legend-anchor-and-filename-off-by-one",
    "bracket-class-enumeration-mismatch",
];

/// Check the transcription and gather the findings.
fn run(arguments: &[String]) -> io::Result<Vec<String>> {
    let digests = wants_digests(arguments)?;
    let root = shared::workspace_root()?;
    let spec = root.join("spec");
    let mut findings = Findings::default();
    let mut census = Vec::new();

    check_confinement(&root, &mut findings)?;
    let capture = read_capture(&spec, &mut findings, &mut census)?;
    run_invariants(&capture, &mut findings, &mut census);
    check_catalogue(&spec, &mut findings, &mut census)?;
    check_defects(&spec, &mut findings, &mut census)?;
    check_provenance(&spec, digests, &mut findings, &mut census)?;

    for line in &census {
        println!("attest: {line}");
    }
    Ok(findings.into_violations())
}

/// Read the one flag this gate takes.
fn wants_digests(arguments: &[String]) -> io::Result<bool> {
    match arguments {
        [] => Ok(false),
        [flag] if flag == "--digests" => Ok(true),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "usage: attest [--digests]; got `{given}`",
                given = arguments.join(" ")
            ),
        )),
    }
}

/// Violations, grouped by the check that produced them.
#[derive(Debug, Default)]
struct Findings {
    /// The first [`REPORTED_PER_KIND`] messages of each kind, in the order found.
    shown: BTreeMap<&'static str, Vec<String>>,
    /// How many of each kind were found, including the ones not shown.
    counted: BTreeMap<&'static str, usize>,
}

impl Findings {
    /// Record one violation of `kind`.
    fn push(&mut self, kind: &'static str, message: String) {
        let seen = self.counted.entry(kind).or_default();
        *seen = seen.saturating_add(1);
        let shown = self.shown.entry(kind).or_default();
        if shown.len() < REPORTED_PER_KIND {
            shown.push(message);
        }
    }

    /// The messages the dispatcher prints, with a count for anything abridged.
    fn into_violations(self) -> Vec<String> {
        let mut violations = Vec::new();
        for (kind, messages) in &self.shown {
            for message in messages {
                violations.push(format!("{kind}: {message}"));
            }
            let found = self.counted.get(kind).copied().unwrap_or_default();
            let hidden = found.saturating_sub(messages.len());
            if hidden > 0 {
                violations.push(format!("{kind}: and {hidden} further violation(s) here"));
            }
        }
        violations
    }
}

/// One end of a table coordinate.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Axis {
    /// One of the thirty character classes.
    Class(u8),
    /// The line head, which is a row and only on Tables 1, 3, 4 and 5.
    LineHead,
    /// The line end, which is a column and only on Tables 1, 3, 4 and 5.
    LineEnd,
}

impl fmt::Display for Axis {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Class(ordinal) => write!(formatter, "cl-{ordinal:02}"),
            Self::LineHead => formatter.write_str("line-head"),
            Self::LineEnd => formatter.write_str("line-end"),
        }
    }
}

/// Read one axis label as the published table prints it.
fn parse_axis(label: &str) -> Result<Axis, String> {
    if label == "line-head" {
        return Ok(Axis::LineHead);
    }
    if label == "line-end" {
        return Ok(Axis::LineEnd);
    }
    let Some(ordinal) = label.strip_prefix("cl-") else {
        return Err(format!(
            "`{label}` is not an axis label; expected `cl-NN`, `line-head` or `line-end`"
        ));
    };
    if ordinal.len() != 2 || !ordinal.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(format!(
            "`{label}` is not a class label; the classes are written `cl-01`, never `cl-1`"
        ));
    }
    let number = ordinal
        .parse::<u8>()
        .map_err(|_| format!("`{label}` has no class ordinal"))?;
    if number == 0 || number > CLASS_COUNT {
        return Err(format!("`{label}` is outside cl-01 through cl-30"));
    }
    Ok(Axis::Class(number))
}

/// Whether an appendix gives its matrix a line head row and a line end column.
fn has_line_edge_axes(table: u8) -> bool {
    matches!(table, 1 | 3 | 4 | 5)
}

/// How many cells a complete matrix holds: 31 × 31 with the line edges, 30 × 30 without.
fn full_size(table: u8) -> usize {
    if has_line_edge_axes(table) { 961 } else { 900 }
}

/// An amount as the table prints it: a fraction of one neighbor's em.
///
/// Kept as the printed fraction rather than as units of 1/720, so that the exactness
/// ADR 0007 requires is a check with a name rather than a parse failure.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Amount {
    /// The printed numerator, reduced.
    numerator: i64,
    /// The printed denominator, reduced. Never zero.
    denominator: i64,
}

impl Amount {
    /// Solid setting: no space at all.
    const ZERO: Self = Self {
        numerator: 0,
        denominator: 1,
    };

    /// Read `1/2`, `1`, or `0`.
    fn parse(text: &str) -> Result<Self, String> {
        let (numerator, denominator) = text.split_once('/').unwrap_or((text, "1"));
        let numerator = numerator
            .trim()
            .parse::<i64>()
            .map_err(|_| format!("`{text}` is not an amount"))?;
        let denominator = denominator
            .trim()
            .parse::<i64>()
            .map_err(|_| format!("`{text}` is not an amount"))?;
        if numerator < 0 || denominator <= 0 {
            return Err(format!("`{text}` is not a non-negative fraction"));
        }
        let divisor = greatest_common_divisor(numerator, denominator).max(1);
        let (Some(numerator), Some(denominator)) = (
            numerator.checked_div(divisor),
            denominator.checked_div(divisor),
        ) else {
            return Err(format!("`{text}` cannot be reduced"));
        };
        Ok(Self {
            numerator,
            denominator,
        })
    }

    /// The amount in units of 1/720 em, or `None` when 1/720 cannot state it exactly.
    fn units(self) -> Option<i32> {
        let scaled = self.numerator.checked_mul(UNITS_PER_EM)?;
        if scaled.checked_rem(self.denominator)? != 0 {
            return None;
        }
        i32::try_from(scaled.checked_div(self.denominator)?).ok()
    }
}

impl fmt::Display for Amount {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.denominator == 1 {
            write!(formatter, "{}", self.numerator)
        } else {
            write!(formatter, "{}/{}", self.numerator, self.denominator)
        }
    }
}

/// Euclid, with a checked remainder so no operand can panic.
fn greatest_common_divisor(left: i64, right: i64) -> i64 {
    let (mut left, mut right) = (left, right);
    while right != 0 {
        let Some(remainder) = left.checked_rem(right) else {
            return 1;
        };
        left = right;
        right = remainder;
    }
    left
}

/// Which neighbor's em an amount is a fraction of. Appendix B writes these `be` and `af`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Referent {
    /// `be`: the preceding character.
    Preceding,
    /// `af`: the trailing character.
    Trailing,
}

/// One term of a Table 1 cell: one neighbor's contribution to the space (ADR 0014).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Term {
    /// The amount.
    amount: Amount,
    /// Whose em it is a fraction of.
    referent: Referent,
}

/// Appendix B's two structurally different ruby permissions.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Hang {
    /// The cell grants none.
    None,
    /// `hang`: ruby may extend over this space, and not over the character.
    OverSpace,
    /// `ruby hang`: the cell is solid and ruby may extend over the character itself.
    OverCharacter,
}

/// An amount that may move to a limit at a ladder stage: Appendix D and Appendix E.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Ranged {
    /// The unadjusted amount, which §D.1 says is Appendix B's.
    amount: Amount,
    /// The floor it may be reduced to, or the ceiling it may be expanded to.
    limit: Option<Amount>,
    /// Written `=` rather than `-`: the amount or the limit, nothing between (§3.1.9).
    two_valued: bool,
    /// §3.8.4 step (d): expansion with no upper limit.
    residual: bool,
    /// The priority ordinal, which the published table encodes as a cell color.
    stage: Option<u8>,
}

impl Ranged {
    /// Whether this cell offers an adjustment opportunity at all.
    fn movable(self) -> bool {
        self.limit.is_some() || self.residual
    }
}

/// What a transcribed cell says, normalized so the two renderings are comparable.
#[derive(Clone, PartialEq, Eq, Debug)]
enum Value {
    /// `×`: the adjacency itself is prohibited.
    Prohibited,
    /// An empty cell. Solid in Table 1; in Table 6 it means expansion is impossible
    /// because there is no line break opportunity.
    Blank,
    /// A Table 1 cell.
    Spacing {
        /// The conditional spaces here, at most one per referent (ADR 0014).
        terms: Vec<Term>,
        /// The ruby permission the cell carries.
        hang: Hang,
    },
    /// A Table 2 cell: the strictness levels at which a break here is prohibited.
    Break {
        /// The levels, out of §C.3's four.
        prohibited: BTreeSet<u8>,
    },
    /// A Table 3, 4, 5 or 6 cell.
    Ranged(Ranged),
}

/// Read one cell token of `table`.
fn parse_value(table: u8, token: &str) -> Result<Value, String> {
    if token == "×" {
        return Ok(Value::Prohibited);
    }
    if token == "blank" {
        return Ok(Value::Blank);
    }
    match table {
        1 => parse_spacing(token),
        2 => parse_break(token),
        3..=6 => parse_ranged(table, token),
        _ => Err(format!("there is no Table {table}")),
    }
}

/// Read an Appendix B token.
fn parse_spacing(token: &str) -> Result<Value, String> {
    if token == "ruby hang" {
        return Ok(Value::Spacing {
            terms: Vec::new(),
            hang: Hang::OverCharacter,
        });
    }
    let (body, hang) = token
        .strip_suffix(" hang")
        .map_or((token, Hang::None), |body| (body, Hang::OverSpace));
    let mut terms = Vec::new();
    for part in body.split('+') {
        terms.push(parse_term(part.trim())?);
    }
    Ok(Value::Spacing { terms, hang })
}

/// Read one `<amount> be` or `<amount> af`.
fn parse_term(text: &str) -> Result<Term, String> {
    let Some((amount, referent)) = text.rsplit_once(' ') else {
        return Err(format!(
            "`{text}` is not `<amount> be` or `<amount> af`; Appendix B names the em"
        ));
    };
    let referent = match referent {
        "be" => Referent::Preceding,
        "af" => Referent::Trailing,
        other => {
            return Err(format!(
                "`{other}` is not a referent; Appendix B writes `be` and `af`"
            ));
        },
    };
    Ok(Term {
        amount: Amount::parse(amount)?,
        referent,
    })
}

/// Read an Appendix C token.
fn parse_break(token: &str) -> Result<Value, String> {
    let Some(rest) = token.strip_prefix("not") else {
        return Err(format!(
            "`{token}` is not `not`, `not <levels>`, `blank` or `×`"
        ));
    };
    let rest = rest.trim();
    if rest.is_empty() {
        return Ok(Value::Break {
            prohibited: LEVELS.into_iter().collect(),
        });
    }
    let mut prohibited = BTreeSet::new();
    for part in rest.split(',') {
        let level = part
            .trim()
            .parse::<u8>()
            .map_err(|_| format!("`{part}` is not a strictness level"))?;
        if !LEVELS.contains(&level) {
            return Err(format!("level {level} is outside §C.3's four levels"));
        }
        prohibited.insert(level);
    }
    Ok(Value::Break { prohibited })
}

/// Read an Appendix D or Appendix E token.
fn parse_ranged(table: u8, token: &str) -> Result<Value, String> {
    let (body, stage) = split_stage(token)?;
    if body == "residual" {
        if table != 6 {
            return Err("`residual` is §3.8.4 step (d) and belongs to Table 6".to_owned());
        }
        return Ok(Value::Ranged(Ranged {
            amount: Amount::ZERO,
            limit: None,
            two_valued: false,
            residual: true,
            stage,
        }));
    }
    let body = body.replace('\u{2013}', "-");
    let (amount, limit, two_valued) = match (body.split_once('='), body.split_once('-')) {
        (Some((amount, limit)), _) => (amount, Some(limit), true),
        (None, Some((amount, limit))) => (amount, Some(limit), false),
        (None, None) => (body.as_str(), None, false),
    };
    let limit = limit.map(Amount::parse).transpose()?;
    Ok(Value::Ranged(Ranged {
        amount: Amount::parse(amount)?,
        limit,
        two_valued,
        residual: false,
        stage,
    }))
}

/// Split a trailing ` stage N` off a token.
fn split_stage(token: &str) -> Result<(&str, Option<u8>), String> {
    let Some((body, ordinal)) = token.rsplit_once(" stage ") else {
        return Ok((token, None));
    };
    let stage = ordinal
        .trim()
        .parse::<u8>()
        .map_err(|_| format!("`{ordinal}` is not a stage ordinal"))?;
    Ok((body.trim(), Some(stage)))
}

/// One transcribed cell.
#[derive(Clone, Debug)]
struct Cell {
    /// The legend token as the source prints it.
    token: String,
    /// What that token says.
    value: Value,
    /// The appendix note qualifying the cell, empty when there is none.
    note: String,
}

/// The coordinate of a cell: its row label and its column label.
type Coordinate = (Axis, Axis);

/// One matrix as one rendering was read.
#[derive(Debug, Default)]
struct Matrix {
    /// Who keyed it, from the capture block.
    author: String,
    /// The order the rows were keyed in, which the two renderings must not share.
    order: Vec<Coordinate>,
    /// The cells that parsed.
    cells: BTreeMap<Coordinate, Cell>,
}

/// The transcription, reduced to the cells both renderings agree on.
#[derive(Debug, Default)]
struct Capture {
    /// The agreed cells of each matrix, by table number.
    tables: BTreeMap<u8, BTreeMap<Coordinate, Cell>>,
}

/// A tab-separated file, split but not interpreted.
///
/// Named for the file format rather than for JLReq's tables, which are [`Matrix`] here.
#[derive(Debug, Default)]
struct Tsv {
    /// The comment preamble's `key = "value"` lines.
    capture: BTreeMap<String, String>,
    /// The header row's field names.
    header: Vec<String>,
    /// Every data row, with the one-based line it was read from.
    rows: Vec<(usize, Vec<String>)>,
}

/// Split a tab-separated file into its capture block, its header and its rows.
fn split_tsv(text: &str) -> Tsv {
    let mut table = Tsv::default();
    for (index, line) in text.lines().enumerate() {
        let number = index.saturating_add(1);
        if let Some(comment) = line.strip_prefix('#') {
            if let Some((key, value)) = comment.split_once('=') {
                if let Some(value) = quoted(value) {
                    table
                        .capture
                        .insert(key.trim().to_owned(), value.to_owned());
                }
            }
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<String> = line.split('\t').map(str::to_owned).collect();
        if table.header.is_empty() {
            table.header = fields;
        } else {
            table.rows.push((number, fields));
        }
    }
    table
}

/// The first double-quoted run on a line, without its quotes.
fn quoted(text: &str) -> Option<&str> {
    let (_, after) = text.split_once('"')?;
    let (value, _) = after.split_once('"')?;
    Some(value)
}

/// Check the capture block every transcribed file carries.
fn check_capture_block(file: &str, table: &Tsv, findings: &mut Findings) -> String {
    let author = table.capture.get("author").cloned().unwrap_or_default();
    if author.trim().is_empty() {
        findings.push(
            "capture-block",
            format!(
                "{file} names no author; double entry is a procedural control and the \
                 procedure is recorded or it did not happen (ADR 0009)"
            ),
        );
    }
    let date = table.capture.get("date").cloned().unwrap_or_default();
    let dated = date.len() == 10
        && date
            .bytes()
            .enumerate()
            .all(|(index, byte)| match_date_byte(index, byte));
    if !dated {
        findings.push(
            "capture-block",
            format!("{file} has no `date = \"YYYY-MM-DD\"` in its capture block"),
        );
    }
    author
}

/// Whether one byte of a `YYYY-MM-DD` date is what that position allows.
fn match_date_byte(index: usize, byte: u8) -> bool {
    if index == 4 || index == 7 {
        byte == b'-'
    } else {
        byte.is_ascii_digit()
    }
}

/// Check the header of a transcribed file against the columns it must have.
fn check_header(file: &str, table: &Tsv, expected: &[&str], findings: &mut Findings) -> bool {
    if table.header.iter().eq(expected.iter()) {
        return true;
    }
    findings.push(
        "provenance",
        format!(
            "{file} heads its columns `{found}`; the format is `{expected}`",
            found = table.header.join(" "),
            expected = expected.join(" ")
        ),
    );
    false
}

/// Read one matrix out of the text of one rendering.
fn parse_matrix(
    file: &str,
    number: u8,
    locale: &str,
    text: &str,
    findings: &mut Findings,
) -> Matrix {
    let table = split_tsv(text);
    let author = check_capture_block(file, &table, findings);
    let mut matrix = Matrix {
        author,
        ..Matrix::default()
    };
    if !check_header(file, &table, &MATRIX_COLUMNS, findings) {
        return matrix;
    }
    let expected_source = published_file(number, locale);
    for (line, fields) in &table.rows {
        match parse_row(number, &expected_source, fields) {
            Err(reason) => findings.push("provenance", format!("{file}:{line}: {reason}")),
            Ok((coordinate, cell)) => {
                if matrix.cells.insert(coordinate, cell).is_some() {
                    findings.push(
                        "provenance",
                        format!(
                            "{file}:{line}: row {before}, column {after} is transcribed twice",
                            before = coordinate.0,
                            after = coordinate.1
                        ),
                    );
                }
                matrix.order.push(coordinate);
            },
        }
    }
    matrix
}

/// The published file a matrix is read from.
///
/// The upstream filenames are off by one from the table numbers — Table 1 is
/// `table_en2.pdf` — which `docs/design/generation.md` records as a defect. Checking it
/// here means a corrected upstream fails loudly rather than misnumbering a table.
fn published_file(number: u8, locale: &str) -> String {
    let published = number.saturating_add(1);
    format!("table_{locale}{published}.pdf")
}

/// Read one transcribed row.
fn parse_row(
    number: u8,
    expected_source: &str,
    fields: &[String],
) -> Result<(Coordinate, Cell), String> {
    let [source, table, before, after, token, note] = fields else {
        return Err(format!(
            "has {count} field(s); every cell records source, table, row label, column \
             label, token and note",
            count = fields.len()
        ));
    };
    if source != expected_source {
        return Err(format!(
            "was read from `{source}`; Table {number} is published as `{expected_source}`"
        ));
    }
    if table.parse::<u8>() != Ok(number) {
        return Err(format!("says table `{table}` in a file of Table {number}"));
    }
    if token.trim().is_empty() {
        return Err("records no token; an empty cell is written `blank`".to_owned());
    }
    let before = parse_axis(before)?;
    let after = parse_axis(after)?;
    if matches!(before, Axis::LineEnd) || matches!(after, Axis::LineHead) {
        return Err(format!(
            "puts `{before}` on the row axis and `{after}` on the column axis; the line \
             head is a row and the line end is a column"
        ));
    }
    let value = parse_value(number, token)?;
    check_note(number, note)?;
    Ok((
        (before, after),
        Cell {
            token: token.clone(),
            value,
            note: note.clone(),
        },
    ))
}

/// The note list a matrix's cells are qualified by.
///
/// Each appendix carries its own, and a cell of one appendix cannot be qualified by
/// another's note, so a misfiled reference is a mistake this can catch rather than one the
/// reader has to notice.
fn note_list(number: u8) -> &'static str {
    match number {
        1 => "B.2",
        2 => "C.2",
        6 => "E.2",
        _ => "D.2",
    }
}

/// Require a note reference to name a note of this matrix's appendix.
fn check_note(number: u8, note: &str) -> Result<(), String> {
    if note.is_empty() {
        return Ok(());
    }
    let list = note_list(number);
    let ordinal = note
        .strip_prefix(list)
        .and_then(|rest| rest.strip_prefix('#'))
        .and_then(|ordinal| ordinal.parse::<u16>().ok());
    if ordinal.is_none_or(|ordinal| ordinal == 0) {
        return Err(format!(
            "cites `{note}`; a cell of Table {number} is qualified by a note of §{list}, \
             written `{list}#3`"
        ));
    }
    Ok(())
}

/// Check the two renderings of one matrix against each other, and return the agreed cells.
fn double_entry(
    stem: &str,
    english: &Matrix,
    japanese: &Matrix,
    findings: &mut Findings,
) -> BTreeMap<Coordinate, Cell> {
    if !english.author.is_empty() && english.author == japanese.author {
        findings.push(
            "double-entry",
            format!(
                "{stem} was keyed twice by `{author}`; the two renderings are entered \
                 independently or the control is not one (ADR 0009)",
                author = english.author
            ),
        );
    }
    if english.order.len() > 1 && english.order == japanese.order {
        findings.push(
            "double-entry",
            format!(
                "{stem}.en.tsv and {stem}.ja.tsv are keyed in identical row order; each \
                 rendering is read in its own order so that a copied file is visible"
            ),
        );
    }
    let mut agreed = BTreeMap::new();
    for (coordinate, cell) in &english.cells {
        let (before, after) = *coordinate;
        let Some(other) = japanese.cells.get(coordinate) else {
            findings.push(
                "double-entry",
                format!("{stem}: row {before}, column {after} is missing from the ja entry"),
            );
            continue;
        };
        if cell.value != other.value || cell.note != other.note {
            findings.push(
                "double-entry",
                format!(
                    "{stem}: row {before}, column {after} reads `{english_token}` in en and \
                     `{japanese_token}` in ja",
                    english_token = cell.token,
                    japanese_token = other.token
                ),
            );
            continue;
        }
        agreed.insert(*coordinate, cell.clone());
    }
    for (before, after) in japanese.cells.keys() {
        if !english.cells.contains_key(&(*before, *after)) {
            findings.push(
                "double-entry",
                format!("{stem}: row {before}, column {after} is missing from the en entry"),
            );
        }
    }
    agreed
}

/// Read `spec/captured/`, checking layout, provenance and double entry as it goes.
fn read_capture(
    spec: &Path,
    findings: &mut Findings,
    census: &mut Vec<String>,
) -> io::Result<Capture> {
    let directory = spec.join("captured");
    if !directory.is_dir() {
        census.push(
            "spec/captured/ does not exist, so no cell has been transcribed yet and the \
             invariants below had nothing to run over"
                .to_owned(),
        );
        return Ok(Capture::default());
    }
    let mut pairs: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    for name in captured_names(&directory)? {
        if name == CATALOGUE {
            continue;
        }
        let Some((stem, locale)) = split_locale(&name) else {
            findings.push(
                "capture-layout",
                format!(
                    "spec/captured/{name} is not `<stem>.en.tsv` or `<stem>.ja.tsv`; every \
                     capture is entered twice, once from each published rendering (ADR 0009)"
                ),
            );
            continue;
        };
        pairs
            .entry(stem.to_owned())
            .or_default()
            .insert(locale.to_owned(), name.clone());
    }

    let mut capture = Capture::default();
    for (stem, locales) in &pairs {
        if !complete_pair(stem, locales, findings) {
            continue;
        }
        match matrix_number(stem) {
            Some(number) => read_matrix_pair(&directory, stem, number, &mut capture, findings)?,
            None => read_other_pair(&directory, stem, findings)?,
        }
    }
    report_coverage(&capture, census);
    Ok(capture)
}

/// The `.tsv` files of the captured directory, in a stable order.
fn captured_names(directory: &Path) -> io::Result<Vec<String>> {
    let mut names = Vec::new();
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
            names.push(name.to_owned());
        }
    }
    names.sort();
    Ok(names)
}

/// Split `table1.en.tsv` into its stem and its locale.
fn split_locale(name: &str) -> Option<(&str, &str)> {
    let base = name.strip_suffix(".tsv")?;
    let (stem, locale) = base.rsplit_once('.')?;
    LOCALES.contains(&locale).then_some((stem, locale))
}

/// The table number of a matrix stem.
fn matrix_number(stem: &str) -> Option<u8> {
    let number = stem.strip_prefix("table")?.parse::<u8>().ok()?;
    (1..=6).contains(&number).then_some(number)
}

/// Require both renderings of a capture to be present.
fn complete_pair(stem: &str, locales: &BTreeMap<String, String>, findings: &mut Findings) -> bool {
    let mut complete = true;
    for locale in LOCALES {
        if !locales.contains_key(locale) {
            complete = false;
            findings.push(
                "double-entry",
                format!(
                    "spec/captured/{stem}.{locale}.tsv is missing; a capture entered once is \
                     not double entry (ADR 0009)"
                ),
            );
        }
    }
    complete
}

/// Read both renderings of one matrix into the capture.
fn read_matrix_pair(
    directory: &Path,
    stem: &str,
    number: u8,
    capture: &mut Capture,
    findings: &mut Findings,
) -> io::Result<()> {
    let mut read = |locale: &str| -> io::Result<Matrix> {
        let name = format!("{stem}.{locale}.tsv");
        let text = fs::read_to_string(directory.join(&name))?;
        Ok(parse_matrix(&name, number, locale, &text, findings))
    };
    let english = read("en")?;
    let japanese = read("ja")?;
    let agreed = double_entry(stem, &english, &japanese, findings);
    capture.tables.insert(number, agreed);
    Ok(())
}

/// Check a capture that is not one of the six matrices.
///
/// `spec/captured/figures.<locale>.tsv` holds the arrangements JLReq states only in a
/// figure (§3.4.3, §3.7.2). Its columns belong to the workflow that transcribes it, so this
/// checks what is true of every capture — a capture block, two different authors, a
/// provenance in the first column of every row, and the same keys on both sides — and says
/// plainly that it has not read the payload.
fn read_other_pair(directory: &Path, stem: &str, findings: &mut Findings) -> io::Result<()> {
    let mut keys: Vec<BTreeSet<String>> = Vec::new();
    let mut authors: Vec<String> = Vec::new();
    for locale in LOCALES {
        let name = format!("{stem}.{locale}.tsv");
        let text = fs::read_to_string(directory.join(&name))?;
        let table = split_tsv(&text);
        authors.push(check_capture_block(&name, &table, findings));
        let width = table.header.len();
        let mut found = BTreeSet::new();
        for (line, fields) in &table.rows {
            if fields.len() != width {
                findings.push(
                    "provenance",
                    format!(
                        "{name}:{line}: has {count} field(s), not {width}",
                        count = fields.len()
                    ),
                );
                continue;
            }
            match fields.first() {
                Some(key) if !key.trim().is_empty() => {
                    found.insert(key.clone());
                },
                _ => findings.push(
                    "provenance",
                    format!("{name}:{line}: records no source; a datum without provenance fails"),
                ),
            }
        }
        keys.push(found);
    }
    if let ([english, japanese], [first, second]) = (keys.as_slice(), authors.as_slice()) {
        if !first.is_empty() && first == second {
            findings.push(
                "double-entry",
                format!("{stem} was keyed twice by `{first}`"),
            );
        }
        for missing in english.symmetric_difference(japanese) {
            findings.push(
                "double-entry",
                format!("{stem}: `{missing}` is recorded in only one of the two renderings"),
            );
        }
    }
    Ok(())
}

/// State what was transcribed, so a report of no violations states what it covered.
fn report_coverage(capture: &Capture, census: &mut Vec<String>) {
    if capture.tables.is_empty() {
        census.push("spec/captured/ holds no matrix, so 0 cells were examined".to_owned());
        return;
    }
    let mut total = 0usize;
    let mut parts = Vec::new();
    for (number, cells) in &capture.tables {
        total = total.saturating_add(cells.len());
        parts.push(format!(
            "table{number} {found}/{full}",
            found = cells.len(),
            full = full_size(*number)
        ));
    }
    census.push(format!(
        "{total} double-entered cell(s) examined: {parts}",
        parts = parts.join(", ")
    ));
}

/// How much of an invariant can run today.
#[derive(Debug)]
enum Check {
    /// Runs in full over the transcription.
    Whole {
        /// The check.
        run: fn(&Capture, &mut Findings),
    },
    /// Runs as far as the transcription alone can settle it.
    Partial {
        /// The part that runs today.
        run: fn(&Capture, &mut Findings),
        /// The input the rest waits for.
        awaiting: &'static str,
        /// What that input will add.
        remainder: &'static str,
    },
    /// Has nothing to run over until an input a later milestone emits exists.
    Awaiting {
        /// The input it waits for.
        input: &'static str,
        /// What it will check then.
        remainder: &'static str,
    },
}

/// One cross-table invariant of `docs/design/generation.md`.
#[derive(Debug)]
struct Invariant {
    /// The identifier `spec/captured/invariants.tsv` publishes it under.
    id: &'static str,
    /// The sentence of the specification that justifies it.
    citation: &'static str,
    /// How much of it runs, and the check itself.
    check: Check,
}

/// The eighteen invariants of `docs/design/generation.md`, in that document's order.
const INVARIANTS: &[Invariant] = &[
    Invariant {
        id: "prohibition-agrees-across-tables",
        citation: "§B.1, §C.1, §D.1, §E.1",
        check: Check::Whole {
            run: prohibition_agrees_across_tables,
        },
    },
    Invariant {
        id: "table6-blank-faces-table2-not",
        citation: "§E.1",
        check: Check::Whole {
            run: table6_blank_faces_table2_not,
        },
    },
    Invariant {
        id: "unadjusted-amount-is-table1",
        citation: "§D.1",
        check: Check::Whole {
            run: unadjusted_amount_is_table1,
        },
    },
    Invariant {
        id: "no-reduction-at-the-line-head",
        citation: "§D.1",
        check: Check::Whole {
            run: no_reduction_at_the_line_head,
        },
    },
    Invariant {
        id: "table4-no-reduction-at-the-line-end",
        citation: "§D.1",
        check: Check::Whole {
            run: table4_no_reduction_at_the_line_end,
        },
    },
    Invariant {
        id: "line-edge-axes-only-where-they-exist",
        citation: "§C.1, §E.1",
        check: Check::Whole {
            run: line_edge_axes_only_where_they_exist,
        },
    },
    Invariant {
        id: "table2-prohibited-at-all-levels",
        citation: "§C.3",
        check: Check::Whole {
            run: table2_prohibited_at_all_levels,
        },
    },
    Invariant {
        id: "line-start-prohibited-classes",
        citation: "§3.1.7, §C.3",
        check: Check::Partial {
            run: line_start_prohibited_classes,
            awaiting: "spec/derived/rules.tsv",
            remainder: "which ten classes they are, rather than that there are ten",
        },
    },
    Invariant {
        id: "line-end-prohibited-classes",
        citation: "§3.1.8",
        check: Check::Partial {
            run: line_end_prohibited_classes,
            awaiting: "spec/derived/rules.tsv",
            remainder: "which two classes they are, rather than that there are two",
        },
    },
    Invariant {
        id: "punctuation-pattern-holds-three-times",
        citation: "§3.2.4, §3.2.5, §3.2.6",
        check: Check::Awaiting {
            input: "spec/derived/rules.tsv",
            remainder: "the five-rule pattern in the cl-19, cl-30 and cl-27 rows and columns",
        },
    },
    Invariant {
        id: "hang-sits-on-a-space",
        citation: "§B.1",
        check: Check::Whole {
            run: hang_sits_on_a_space,
        },
    },
    Invariant {
        id: "bracket-classes-mirror-their-originals",
        citation: "§3.9.2, §3.1.10",
        check: Check::Partial {
            run: bracket_classes_mirror_their_originals,
            awaiting: "spec/derived/notes.tsv",
            remainder: "the identity of each noted difference with the note that states it",
        },
    },
    Invariant {
        id: "table4-line-end-follows-the-jis-reading",
        citation: "§3.1.9",
        check: Check::Whole {
            run: table4_line_end_follows_the_jis_reading,
        },
    },
    Invariant {
        id: "stage-ordinals-are-contiguous",
        citation: "§3.8.3, §3.8.4",
        check: Check::Partial {
            run: stage_ordinals_are_contiguous,
            awaiting: "spec/derived/notes.tsv",
            remainder: "the equality of the ordinals with the prose order of the two ladders",
        },
    },
    Invariant {
        id: "priority-ordinals-agree-with-the-notes",
        citation: "§D.2",
        check: Check::Awaiting {
            input: "spec/derived/notes.tsv",
            remainder: "the ordinals read from cell color against the ones §D.2 states",
        },
    },
    Invariant {
        id: "conformance-cases-agree-with-the-cells",
        citation: "ADR 0006",
        check: Check::Awaiting {
            input: "tests/conformance/",
            remainder: "every cell a published case exercises against that case",
        },
    },
    Invariant {
        id: "amounts-are-multiples-of-the-unit",
        citation: "ADR 0007",
        check: Check::Whole {
            run: amounts_are_multiples_of_the_unit,
        },
    },
    Invariant {
        id: "at-most-one-space-per-referent",
        citation: "ADR 0014",
        check: Check::Partial {
            run: at_most_one_space_per_referent,
            awaiting: "spec/derived/notes.tsv",
            remainder: "the overrides the appendix notes produce, as well as the cells",
        },
    },
];

/// Run every invariant that has something to run over, and say which did not.
fn run_invariants(capture: &Capture, findings: &mut Findings, census: &mut Vec<String>) {
    let mut ran = 0usize;
    let mut waiting = Vec::new();
    for invariant in INVARIANTS {
        match &invariant.check {
            Check::Whole { run } => {
                run(capture, findings);
                ran = ran.saturating_add(1);
            },
            Check::Partial {
                run,
                awaiting,
                remainder,
            } => {
                run(capture, findings);
                ran = ran.saturating_add(1);
                waiting.push(format!(
                    "  {id} still awaits {awaiting} for {remainder}",
                    id = invariant.id
                ));
            },
            Check::Awaiting { input, remainder } => waiting.push(format!(
                "  {id} awaits {input} for {remainder}",
                id = invariant.id
            )),
        }
    }
    census.push(format!(
        "{total} invariant(s) registered, {ran} of which ran over the transcription",
        total = INVARIANTS.len()
    ));
    census.extend(waiting);
}

/// A `×` at a coordinate is a `×` at that coordinate in every table that has it.
fn prohibition_agrees_across_tables(capture: &Capture, findings: &mut Findings) {
    let mut seen: BTreeMap<Coordinate, (Vec<u8>, Vec<u8>)> = BTreeMap::new();
    for (number, cells) in &capture.tables {
        for (coordinate, cell) in cells {
            let entry = seen.entry(*coordinate).or_default();
            entry.0.push(*number);
            if cell.value == Value::Prohibited {
                entry.1.push(*number);
            }
        }
    }
    for ((before, after), (present, prohibited)) in &seen {
        if !prohibited.is_empty() && prohibited.len() != present.len() {
            findings.push(
                "prohibition-agrees-across-tables",
                format!(
                    "row {before}, column {after} is `×` in table(s) {prohibited:?} but not in \
                     all of {present:?}; the four legends define `×` identically"
                ),
            );
        }
    }
}

/// §E.1's blank means expansion is impossible because there is no break opportunity.
fn table6_blank_faces_table2_not(capture: &Capture, findings: &mut Findings) {
    let (Some(expansion), Some(breaks)) = (capture.tables.get(&6), capture.tables.get(&2)) else {
        return;
    };
    for (coordinate, cell) in expansion {
        if cell.value != Value::Blank {
            continue;
        }
        let Some(facing) = breaks.get(coordinate) else {
            continue;
        };
        let never = match &facing.value {
            Value::Break { prohibited } => prohibited.len() == LEVELS.len(),
            _ => false,
        };
        if !never {
            let (before, after) = *coordinate;
            findings.push(
                "table6-blank-faces-table2-not",
                format!(
                    "row {before}, column {after} is blank in Table 6, which §E.1 defines as \
                     `no line break opportunity`, but Table 2 reads `{token}`",
                    token = facing.token
                ),
            );
        }
    }
}

/// The Table 1 amount at a coordinate, in units of 1/720 em.
fn table1_units(value: &Value) -> Option<i32> {
    match value {
        Value::Blank => Some(0),
        Value::Spacing { terms, .. } => {
            let mut total: i32 = 0;
            for term in terms {
                total = total.checked_add(term.amount.units()?)?;
            }
            Some(total)
        },
        _ => None,
    }
}

/// §D.1: the unadjusted amounts of Tables 3, 4 and 5 are Appendix B's, and a cell that may
/// be reduced has something to reduce.
fn unadjusted_amount_is_table1(capture: &Capture, findings: &mut Findings) {
    let Some(spacing) = capture.tables.get(&1) else {
        return;
    };
    for number in [3u8, 4, 5] {
        let Some(cells) = capture.tables.get(&number) else {
            continue;
        };
        for (coordinate, cell) in cells {
            let Value::Ranged(ranged) = &cell.value else {
                continue;
            };
            let (Some(reference), Some(actual)) = (spacing.get(coordinate), ranged.amount.units())
            else {
                continue;
            };
            let Some(expected) = table1_units(&reference.value) else {
                continue;
            };
            let (before, after) = *coordinate;
            if actual != expected {
                findings.push(
                    "unadjusted-amount-is-table1",
                    format!(
                        "Table {number} row {before}, column {after} is unadjusted {actual}/720 \
                         em where Table 1 says {expected}/720"
                    ),
                );
            } else if ranged.movable() && expected == SOLID {
                findings.push(
                    "unadjusted-amount-is-table1",
                    format!(
                        "Table {number} row {before}, column {after} offers an adjustment where \
                         Table 1 sets the boundary solid"
                    ),
                );
            }
        }
    }
}

/// §D.1: no reduction opportunity in the line head row of Tables 3, 4 and 5.
fn no_reduction_at_the_line_head(capture: &Capture, findings: &mut Findings) {
    for number in [3u8, 4, 5] {
        let Some(cells) = capture.tables.get(&number) else {
            continue;
        };
        for ((before, after), cell) in cells {
            if *before != Axis::LineHead {
                continue;
            }
            if matches!(&cell.value, Value::Ranged(ranged) if ranged.movable()) {
                findings.push(
                    "no-reduction-at-the-line-head",
                    format!(
                        "Table {number} offers a reduction in the line-head row, column {after} \
                         (`{token}`)",
                        token = cell.token
                    ),
                );
            }
        }
    }
}

/// §D.1: Table 4 additionally has no reduction opportunity in the line end column.
fn table4_no_reduction_at_the_line_end(capture: &Capture, findings: &mut Findings) {
    let Some(cells) = capture.tables.get(&4) else {
        return;
    };
    for ((before, after), cell) in cells {
        if *after != Axis::LineEnd {
            continue;
        }
        if matches!(&cell.value, Value::Ranged(ranged) if ranged.movable()) {
            findings.push(
                "table4-no-reduction-at-the-line-end",
                format!(
                    "Table 4 offers a reduction at row {before}, line-end (`{token}`)",
                    token = cell.token
                ),
            );
        }
    }
}

/// §C.1 and §E.1: Tables 2 and 6 have no line-edge axes at all.
fn line_edge_axes_only_where_they_exist(capture: &Capture, findings: &mut Findings) {
    for (number, cells) in &capture.tables {
        if has_line_edge_axes(*number) {
            continue;
        }
        for (before, after) in cells.keys() {
            if *before == Axis::LineHead || *after == Axis::LineEnd {
                findings.push(
                    "line-edge-axes-only-where-they-exist",
                    format!(
                        "Table {number} records row {before}, column {after}; that appendix \
                         names no line-edge axis, so the cell has no provenance to record"
                    ),
                );
            }
        }
    }
}

/// §C.3: cl-01 as a row and cl-02, cl-06 and cl-07 as columns are prohibited at all levels.
fn table2_prohibited_at_all_levels(capture: &Capture, findings: &mut Findings) {
    let Some(cells) = capture.tables.get(&2) else {
        return;
    };
    for ((before, after), cell) in cells {
        let row = *before == Axis::Class(1);
        let column = matches!(after, Axis::Class(ordinal) if [2, 6, 7].contains(ordinal));
        if !row && !column {
            continue;
        }
        let all = match &cell.value {
            Value::Break { prohibited } => prohibited.len() == LEVELS.len(),
            _ => false,
        };
        if !all {
            findings.push(
                "table2-prohibited-at-all-levels",
                format!(
                    "Table 2 row {before}, column {after} reads `{token}`; §C.3's preamble \
                     prohibits this at all levels",
                    token = cell.token
                ),
            );
        }
    }
}

/// Whether a Table 2 cell prohibits a break at `level`.
fn prohibits_at(cell: &Cell, level: u8) -> bool {
    match &cell.value {
        Value::Break { prohibited } => prohibited.contains(&level),
        _ => false,
    }
}

/// The classes prohibited throughout at `level`, on whichever axis `axis_of` selects.
fn prohibited_throughout(
    cells: &BTreeMap<Coordinate, Cell>,
    level: u8,
    axis_of: fn(Coordinate) -> Axis,
) -> BTreeSet<Axis> {
    let mut allowed = BTreeSet::new();
    let mut seen = BTreeSet::new();
    for (coordinate, cell) in cells {
        let axis = axis_of(*coordinate);
        seen.insert(axis);
        if !prohibits_at(cell, level) {
            allowed.insert(axis);
        }
    }
    seen.difference(&allowed).copied().collect()
}

/// Whether Table 2 has been transcribed in full, which counting over it requires.
fn table2_is_complete(capture: &Capture) -> Option<&BTreeMap<Coordinate, Cell>> {
    let cells = capture.tables.get(&2)?;
    (cells.len() == full_size(2)).then_some(cells)
}

/// §3.1.7: ten classes may not start a line.
fn line_start_prohibited_classes(capture: &Capture, findings: &mut Findings) {
    let Some(cells) = table2_is_complete(capture) else {
        return;
    };
    let found = prohibited_throughout(cells, VERY_STRICT, |(_, after)| after);
    if found.len() != 10 {
        findings.push(
            "line-start-prohibited-classes",
            format!(
                "{count} Table 2 column(s) are prohibited throughout at the Very strict level; \
                 §3.1.7 names ten: {found:?}",
                count = found.len()
            ),
        );
    }
}

/// §3.1.8: two classes may not end a line.
fn line_end_prohibited_classes(capture: &Capture, findings: &mut Findings) {
    let Some(cells) = table2_is_complete(capture) else {
        return;
    };
    let found = prohibited_throughout(cells, VERY_STRICT, |(before, _)| before);
    if found.len() != 2 {
        findings.push(
            "line-end-prohibited-classes",
            format!(
                "{count} Table 2 row(s) are prohibited throughout at the Very strict level; \
                 §3.1.8 names two: {found:?}",
                count = found.len()
            ),
        );
    }
}

/// §B.1: `hang` sits on a half or quarter em, and `ruby hang` on a solid cell.
fn hang_sits_on_a_space(capture: &Capture, findings: &mut Findings) {
    let Some(cells) = capture.tables.get(&1) else {
        return;
    };
    for ((before, after), cell) in cells {
        let Value::Spacing { terms, hang } = &cell.value else {
            continue;
        };
        let units = table1_units(&cell.value);
        let complaint = match hang {
            Hang::OverSpace if units != Some(HALF_EM) && units != Some(QUARTER_EM) => {
                Some("§B.1 lets ruby extend over a half or quarter em space, and there is no other")
            },
            Hang::OverCharacter if !terms.is_empty() => {
                Some("`ruby hang` is the solid case, where there is no space to extend over")
            },
            Hang::None | Hang::OverSpace | Hang::OverCharacter => None,
        };
        if let Some(reason) = complaint {
            findings.push(
                "hang-sits-on-a-space",
                format!(
                    "row {before}, column {after} reads `{token}`; {reason}",
                    token = cell.token
                ),
            );
        }
    }
}

/// cl-28 and cl-29 match cl-01 and cl-02 except where a note states a difference.
fn bracket_classes_mirror_their_originals(capture: &Capture, findings: &mut Findings) {
    for (mirror, original) in [(28u8, 1u8), (29, 2)] {
        for (number, cells) in &capture.tables {
            for (coordinate, cell) in cells {
                let (before, after) = *coordinate;
                let facing = if before == Axis::Class(mirror) {
                    (Axis::Class(original), after)
                } else if after == Axis::Class(mirror) {
                    (before, Axis::Class(original))
                } else {
                    continue;
                };
                let Some(reference) = cells.get(&facing) else {
                    continue;
                };
                if reference.value != cell.value
                    && cell.note.is_empty()
                    && reference.note.is_empty()
                {
                    findings.push(
                        "bracket-classes-mirror-their-originals",
                        format!(
                            "Table {number} row {before}, column {after} reads `{token}` where \
                             cl-{original:02} reads `{other}`, and no note states the difference",
                            token = cell.token,
                            other = reference.token
                        ),
                    );
                }
            }
        }
    }
}

/// §3.1.9: a half em after cl-06 at the line end, and solid after cl-02, cl-05 and cl-07.
fn table4_line_end_follows_the_jis_reading(capture: &Capture, findings: &mut Findings) {
    let Some(cells) = capture.tables.get(&4) else {
        return;
    };
    for (class, expected) in [(6u8, HALF_EM), (2, SOLID), (5, SOLID), (7, SOLID)] {
        let Some(cell) = cells.get(&(Axis::Class(class), Axis::LineEnd)) else {
            continue;
        };
        let actual = match &cell.value {
            Value::Ranged(ranged) => ranged.amount.units(),
            Value::Blank => Some(SOLID),
            _ => None,
        };
        if actual != Some(expected) {
            findings.push(
                "table4-line-end-follows-the-jis-reading",
                format!(
                    "Table 4 sets row cl-{class:02}, line-end to `{token}`; §3.1.9's JIS reading \
                     is {expected}/720 em",
                    token = cell.token
                ),
            );
        }
    }
}

/// The ladders have six and four steps, and a table uses a contiguous run of them.
fn stage_ordinals_are_contiguous(capture: &Capture, findings: &mut Findings) {
    for (number, steps) in [(3u8, 6u8), (4, 6), (5, 6), (6, 4)] {
        let Some(cells) = capture.tables.get(&number) else {
            continue;
        };
        let mut stages = BTreeSet::new();
        for cell in cells.values() {
            if let Value::Ranged(ranged) = &cell.value {
                if let Some(stage) = ranged.stage {
                    stages.insert(stage);
                }
            }
        }
        for stage in &stages {
            if *stage == 0 || *stage > steps {
                findings.push(
                    "stage-ordinals-are-contiguous",
                    format!("Table {number} names stage {stage}; that ladder has {steps} steps"),
                );
            }
        }
        let (Some(first), Some(last)) = (stages.first(), stages.last()) else {
            continue;
        };
        for stage in *first..=*last {
            if !stages.contains(&stage) {
                findings.push(
                    "stage-ordinals-are-contiguous",
                    format!(
                        "Table {number} uses stages {stages:?}, which skips {stage}; the ladder \
                         is an ordering and has no gap"
                    ),
                );
            }
        }
    }
}

/// Every amount the transcription names.
fn amounts_of(value: &Value) -> Vec<Amount> {
    match value {
        Value::Spacing { terms, .. } => terms.iter().map(|term| term.amount).collect(),
        Value::Ranged(ranged) => {
            let mut amounts = vec![ranged.amount];
            amounts.extend(ranged.limit);
            amounts
        },
        Value::Prohibited | Value::Blank | Value::Break { .. } => Vec::new(),
    }
}

/// ADR 0007: every amount is an exact multiple of 1/720 em.
fn amounts_are_multiples_of_the_unit(capture: &Capture, findings: &mut Findings) {
    for (number, cells) in &capture.tables {
        for ((before, after), cell) in cells {
            for amount in amounts_of(&cell.value) {
                if amount.units().is_none() {
                    findings.push(
                        "amounts-are-multiples-of-the-unit",
                        format!(
                            "Table {number} row {before}, column {after} names {amount} em, which \
                             1/720 cannot state exactly; 720 was chosen so that it could"
                        ),
                    );
                }
            }
        }
    }
}

/// ADR 0014: a boundary yields at most one conditional space per referent.
fn at_most_one_space_per_referent(capture: &Capture, findings: &mut Findings) {
    let Some(cells) = capture.tables.get(&1) else {
        return;
    };
    for ((before, after), cell) in cells {
        let Value::Spacing { terms, .. } = &cell.value else {
            continue;
        };
        for referent in [Referent::Preceding, Referent::Trailing] {
            let count = terms
                .iter()
                .filter(|term| term.referent == referent)
                .count();
            if count > 1 {
                findings.push(
                    "at-most-one-space-per-referent",
                    format!(
                        "row {before}, column {after} reads `{token}`, which is {count} \
                         contributions from one neighbor; a space has two owners and no more",
                        token = cell.token
                    ),
                );
            }
        }
    }
}

/// Require the published catalogue to name exactly the invariants that are implemented.
fn check_catalogue(
    spec: &Path,
    findings: &mut Findings,
    census: &mut Vec<String>,
) -> io::Result<()> {
    let path = spec.join("captured").join(CATALOGUE);
    if !path.is_file() {
        census.push(format!(
            "spec/captured/{CATALOGUE} does not exist, so the published catalogue was not \
             checked against the {count} invariants above",
            count = INVARIANTS.len()
        ));
        return Ok(());
    }
    let text = fs::read_to_string(&path)?;
    let published = check_id_column(
        CATALOGUE,
        &text,
        &CATALOGUE_COLUMNS,
        &INVARIANTS.iter().map(|each| each.id).collect::<Vec<_>>(),
        "catalogue",
        findings,
    );
    for invariant in INVARIANTS {
        let Some(cited) = published.get(invariant.id).and_then(|row| row.get(1)) else {
            continue;
        };
        if cited != invariant.citation {
            findings.push(
                "catalogue",
                format!(
                    "{CATALOGUE} cites {cited} for `{id}`; the check here is justified by \
                     {citation}",
                    id = invariant.id,
                    citation = invariant.citation
                ),
            );
        }
    }
    census.push(format!(
        "spec/captured/{CATALOGUE} was checked against the {count} implemented invariants",
        count = INVARIANTS.len()
    ));
    Ok(())
}

/// Require the recorded defects to be exactly the ones this design records.
fn check_defects(spec: &Path, findings: &mut Findings, census: &mut Vec<String>) -> io::Result<()> {
    let path = spec.join("derived").join("defects.tsv");
    if !path.is_file() {
        census.push(
            "spec/derived/defects.tsv does not exist, so the recorded defects of the published \
             document were not checked"
                .to_owned(),
        );
        return Ok(());
    }
    let text = fs::read_to_string(&path)?;
    check_id_column(
        "defects.tsv",
        &text,
        &DEFECT_COLUMNS,
        &RECORDED_DEFECTS,
        "defects",
        findings,
    );
    census.push(format!(
        "spec/derived/defects.tsv was checked against the {count} defects this design records",
        count = RECORDED_DEFECTS.len()
    ));
    Ok(())
}

/// Check a file whose first column is a set of identifiers, and return what it records.
fn check_id_column(
    file: &str,
    text: &str,
    columns: &[&str],
    expected: &[&str],
    kind: &'static str,
    findings: &mut Findings,
) -> BTreeMap<String, Vec<String>> {
    let mut recorded = BTreeMap::new();
    let table = split_tsv(text);
    if !check_header(file, &table, columns, findings) {
        return recorded;
    }
    for (line, fields) in &table.rows {
        if fields.len() != columns.len() || fields.iter().any(|field| field.trim().is_empty()) {
            findings.push(
                kind,
                format!(
                    "{file}:{line}: every row states {count} non-empty column(s)",
                    count = columns.len()
                ),
            );
            continue;
        }
        let Some(id) = fields.first() else { continue };
        if recorded.insert(id.clone(), fields.clone()).is_some() {
            findings.push(kind, format!("{file}:{line}: `{id}` is recorded twice"));
        }
    }
    let found: BTreeSet<&String> = recorded.keys().collect();
    let expected: BTreeSet<String> = expected.iter().map(|each| (*each).to_owned()).collect();
    for missing in expected.iter().filter(|each| !found.contains(each)) {
        findings.push(
            kind,
            format!("{file} does not record `{missing}`, which this design does"),
        );
    }
    for extra in found.iter().filter(|each| !expected.contains(**each)) {
        findings.push(
            kind,
            format!("{file} records `{extra}`, which this design does not"),
        );
    }
    recorded
}

/// One upstream document, as `spec/PROVENANCE.toml` records it.
#[derive(Debug, Default)]
struct Document {
    /// Where the file sits, relative to `spec/`.
    path: String,
    /// Where it came from.
    url: String,
    /// When it was retrieved.
    retrieved: String,
    /// Its SHA-256, lowercase hex.
    sha256: String,
    /// The line its block starts on.
    line: usize,
}

/// Read the `[[document]]` blocks of a provenance record.
///
/// Hand-rolled for the reason stated on the `purity` module's manifest scan: the tool that
/// enforces "the layout core declares no outside dependencies" declares none itself. It
/// understands the one form this repository writes and reads nothing else.
fn read_provenance(text: &str) -> Vec<Document> {
    let mut documents: Vec<Document> = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = strip_comment(line).trim();
        if line == "[[document]]" {
            documents.push(Document {
                line: index.saturating_add(1),
                ..Document::default()
            });
            continue;
        }
        if line.starts_with('[') {
            continue;
        }
        let (Some((key, value)), Some(document)) = (line.split_once('='), documents.last_mut())
        else {
            continue;
        };
        let Some(value) = quoted(value) else {
            continue;
        };
        match key.trim() {
            "path" => value.clone_into(&mut document.path),
            "url" => value.clone_into(&mut document.url),
            "retrieved" => value.clone_into(&mut document.retrieved),
            "sha256" => value.clone_into(&mut document.sha256),
            _ => {},
        }
    }
    documents
}

/// Everything before the first `#` that is not inside a string.
fn strip_comment(line: &str) -> &str {
    let mut inside_string = false;
    for (index, character) in line.char_indices() {
        match character {
            '"' => inside_string = !inside_string,
            '#' if !inside_string => return line.get(..index).unwrap_or(line),
            _ => {},
        }
    }
    line
}

/// Check the provenance record, and with `--digests` the files it names.
fn check_provenance(
    spec: &Path,
    digests: bool,
    findings: &mut Findings,
    census: &mut Vec<String>,
) -> io::Result<()> {
    let path = spec.join("PROVENANCE.toml");
    if !path.is_file() {
        if digests {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "spec/PROVENANCE.toml does not exist yet, so there is no recorded digest for \
                 `--digests` to verify against",
            ));
        }
        census.push(
            "spec/PROVENANCE.toml does not exist, so no upstream document was verified".to_owned(),
        );
        return Ok(());
    }
    let documents = read_provenance(&fs::read_to_string(&path)?);
    let mut recorded = BTreeSet::new();
    for document in &documents {
        check_document_record(document, &mut recorded, findings);
    }
    if digests {
        verify_digests(spec, &documents, &recorded, findings, census)?;
    } else {
        census.push(format!(
            "spec/PROVENANCE.toml records {count} upstream document(s); run `attest --digests` \
             to verify the ones present on disk",
            count = documents.len()
        ));
    }
    Ok(())
}

/// Check one provenance record for the four things it must state.
fn check_document_record(
    document: &Document,
    recorded: &mut BTreeSet<String>,
    findings: &mut Findings,
) {
    let line = document.line;
    for (field, value) in [
        ("path", &document.path),
        ("url", &document.url),
        ("retrieved", &document.retrieved),
        ("sha256", &document.sha256),
    ] {
        if value.trim().is_empty() {
            findings.push(
                "provenance-record",
                format!("PROVENANCE.toml:{line}: the document states no `{field}`"),
            );
        }
    }
    let hexadecimal = document.sha256.len() == 64
        && document
            .sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase());
    if !document.sha256.is_empty() && !hexadecimal {
        findings.push(
            "provenance-record",
            format!(
                "PROVENANCE.toml:{line}: `{sha}` is not a lowercase SHA-256",
                sha = document.sha256
            ),
        );
    }
    if !document.path.is_empty() && !recorded.insert(document.path.clone()) {
        findings.push(
            "provenance-record",
            format!(
                "PROVENANCE.toml:{line}: `{path}` is recorded twice",
                path = document.path
            ),
        );
    }
}

/// Verify every recorded document that is present, and require every present one to be
/// recorded.
fn verify_digests(
    spec: &Path,
    documents: &[Document],
    recorded: &BTreeSet<String>,
    findings: &mut Findings,
    census: &mut Vec<String>,
) -> io::Result<()> {
    let mut verified = 0usize;
    let mut absent = 0usize;
    for document in documents {
        let path = spec.join(&document.path);
        if !path.is_file() {
            absent = absent.saturating_add(1);
            continue;
        }
        let Some(digest) = sha256_hex(&fs::read(&path)?) else {
            findings.push(
                "digest",
                format!("{path} is too large to digest", path = document.path),
            );
            continue;
        };
        if digest == document.sha256 {
            verified = verified.saturating_add(1);
        } else {
            findings.push(
                "digest",
                format!(
                    "{path} hashes to {digest}, and PROVENANCE.toml records {recorded}",
                    path = document.path,
                    recorded = document.sha256
                ),
            );
        }
    }
    let upstream = spec.join("upstream");
    for present in relative_files(&upstream, spec)? {
        if !recorded.contains(&present) {
            findings.push(
                "digest",
                format!("spec/{present} is present but PROVENANCE.toml does not record it"),
            );
        }
    }
    census.push(format!(
        "{verified} upstream document(s) verified against PROVENANCE.toml, {absent} recorded \
         but not present"
    ));
    Ok(())
}

/// Every file under `dir`, named relative to `base`, with forward slashes.
fn relative_files(dir: &Path, base: &Path) -> io::Result<Vec<String>> {
    let mut found = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(directory) = stack.pop() {
        if !directory.is_dir() {
            continue;
        }
        for entry in fs::read_dir(&directory)? {
            let path = entry?.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                found.push(shared::relative_name(&path, base).replace('\\', "/"));
            }
        }
    }
    found.sort();
    Ok(found)
}

/// Require the transcription to sit in `spec/captured/` and nowhere else.
fn check_confinement(root: &Path, findings: &mut Findings) -> io::Result<()> {
    let captured = root.join("spec").join("captured");
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        for entry in fs::read_dir(&directory)? {
            let path = entry?.path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if path.is_dir() {
                if !SKIPPED_DIRECTORIES.contains(&name) {
                    stack.push(path);
                }
            } else if is_transcription(name) && path.parent() != Some(captured.as_path()) {
                findings.push(
                    "confinement",
                    format!(
                        "{file} is a transcription outside spec/captured/; the capture is \
                         confined to one directory so that it can be reviewed as one (ADR 0009)",
                        file = shared::relative_name(&path, root).replace('\\', "/")
                    ),
                );
            }
        }
    }
    Ok(())
}

/// Whether a file name is one the captured directory owns.
fn is_transcription(name: &str) -> bool {
    name == CATALOGUE || split_locale(name).is_some()
}

/// The SHA-256 round constants: the first 32 bits of the fractional parts of the cube roots
/// of the first 64 primes (FIPS 180-4 §4.2.2).
const ROUND_CONSTANTS: [u32; 64] = [
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

/// The initial hash value: the first 32 bits of the fractional parts of the square roots of
/// the first eight primes (FIPS 180-4 §5.3.3).
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

/// The SHA-256 of a byte string, as lowercase hex.
///
/// `std` alone, because `xtask` keeps an empty dependency table: it is the tool that
/// enforces "the layout core declares no outside dependencies".
fn sha256_hex(message: &[u8]) -> Option<String> {
    let mut state = INITIAL_STATE;
    let mut block = [0u8; 64];
    let mut blocks = message.chunks_exact(64);
    for chunk in &mut blocks {
        block.copy_from_slice(chunk);
        compress(&mut state, block);
    }
    let bits = u64::try_from(message.len()).ok()?.checked_mul(8)?;
    let mut tail = Vec::with_capacity(128);
    tail.extend_from_slice(blocks.remainder());
    tail.push(0x80);
    while tail.len().checked_rem(64) != Some(56) {
        tail.push(0);
    }
    tail.extend_from_slice(&bits.to_be_bytes());
    for chunk in tail.chunks_exact(64) {
        block.copy_from_slice(chunk);
        compress(&mut state, block);
    }
    hexadecimal(state)
}

/// One SHA-256 compression, over one 64-byte block (FIPS 180-4 §6.2.2).
fn compress(state: &mut [u32; 8], block: [u8; 64]) {
    let mut schedule = [0u32; 64];
    for (slot, chunk) in schedule.iter_mut().zip(block.chunks_exact(4)) {
        let mut word = [0u8; 4];
        word.copy_from_slice(chunk);
        *slot = u32::from_be_bytes(word);
    }
    for index in 16..64usize {
        let near = schedule[index.wrapping_sub(15)];
        let far = schedule[index.wrapping_sub(2)];
        let mixed = near.rotate_right(7) ^ near.rotate_right(18) ^ near.wrapping_shr(3);
        let spread = far.rotate_right(17) ^ far.rotate_right(19) ^ far.wrapping_shr(10);
        schedule[index] = schedule[index.wrapping_sub(16)]
            .wrapping_add(mixed)
            .wrapping_add(schedule[index.wrapping_sub(7)])
            .wrapping_add(spread);
    }
    let mut work = *state;
    for (word, constant) in schedule.iter().zip(ROUND_CONSTANTS.iter()) {
        let [va, vb, vc, vd, ve, vf, vg, vh] = work;
        let sum1 = ve.rotate_right(6) ^ ve.rotate_right(11) ^ ve.rotate_right(25);
        let choice = (ve & vf) ^ (!ve & vg);
        let first = vh
            .wrapping_add(sum1)
            .wrapping_add(choice)
            .wrapping_add(*constant)
            .wrapping_add(*word);
        let sum0 = va.rotate_right(2) ^ va.rotate_right(13) ^ va.rotate_right(22);
        let majority = (va & vb) ^ (va & vc) ^ (vb & vc);
        let second = sum0.wrapping_add(majority);
        work = [
            first.wrapping_add(second),
            va,
            vb,
            vc,
            vd.wrapping_add(first),
            ve,
            vf,
            vg,
        ];
    }
    for (slot, value) in state.iter_mut().zip(work.iter()) {
        *slot = slot.wrapping_add(*value);
    }
}

/// A hash state as lowercase hex.
fn hexadecimal(state: [u32; 8]) -> Option<String> {
    let mut text = String::with_capacity(64);
    for word in state {
        for byte in word.to_be_bytes() {
            for nibble in [byte.wrapping_shr(4), byte & 0x0f] {
                text.push(char::from_digit(u32::from(nibble), 16)?);
            }
        }
    }
    Some(text)
}

#[cfg(test)]
mod tests {
    use super::{
        Amount, Axis, CATALOGUE_COLUMNS, Capture, Cell, Check, DEFECT_COLUMNS, Findings,
        INVARIANTS, LEVELS, RECORDED_DEFECTS, amounts_are_multiples_of_the_unit,
        at_most_one_space_per_referent, bracket_classes_mirror_their_originals, check_id_column,
        double_entry, full_size, hang_sits_on_a_space, is_transcription,
        line_edge_axes_only_where_they_exist, line_end_prohibited_classes,
        line_start_prohibited_classes, no_reduction_at_the_line_head, parse_axis, parse_matrix,
        parse_value, prohibition_agrees_across_tables, published_file, read_provenance, sha256_hex,
        split_locale, stage_ordinals_are_contiguous, table2_prohibited_at_all_levels,
        table4_line_end_follows_the_jis_reading, table4_no_reduction_at_the_line_end,
        table6_blank_faces_table2_not, unadjusted_amount_is_table1, wants_digests,
    };

    /// The findings of a check, as one string to assert against.
    fn reported(findings: Findings) -> String {
        findings.into_violations().join("\n")
    }

    /// Build a capture from `(table, before, after, token)` tuples.
    fn capture(cells: &[(u8, &str, &str, &str)]) -> Capture {
        let mut capture = Capture::default();
        for (table, before, after, token) in cells {
            let value = parse_value(*table, token).expect("the fixture token parses");
            capture.tables.entry(*table).or_default().insert(
                (
                    parse_axis(before).expect("the fixture row label parses"),
                    parse_axis(after).expect("the fixture column label parses"),
                ),
                Cell {
                    token: (*token).to_owned(),
                    value,
                    note: String::new(),
                },
            );
        }
        capture
    }

    /// Run one check over a capture and return what it reported.
    fn run_over(check: fn(&Capture, &mut Findings), cells: &[(u8, &str, &str, &str)]) -> String {
        let mut findings = Findings::default();
        check(&capture(cells), &mut findings);
        reported(findings)
    }

    /// A complete Table 2 whose `prohibited` columns and rows are `not` at every level.
    fn full_table2(columns: &[u8], rows: &[u8]) -> Capture {
        let mut capture = Capture::default();
        let cells = capture.tables.entry(2).or_default();
        for before in 1..=30u8 {
            for after in 1..=30u8 {
                let token = if columns.contains(&after) || rows.contains(&before) {
                    "not"
                } else {
                    "blank"
                };
                cells.insert(
                    (Axis::Class(before), Axis::Class(after)),
                    Cell {
                        token: token.to_owned(),
                        value: parse_value(2, token).expect("the fixture token parses"),
                        note: String::new(),
                    },
                );
            }
        }
        capture
    }

    #[test]
    fn a_class_label_is_written_with_two_digits() {
        assert_eq!(parse_axis("cl-01"), Ok(Axis::Class(1)));
        assert_eq!(parse_axis("line-head"), Ok(Axis::LineHead));
        assert_eq!(parse_axis("line-end"), Ok(Axis::LineEnd));
        assert!(
            parse_axis("cl-1").is_err(),
            "cl-1 is the spelling ADR 0008 corrected"
        );
        assert!(parse_axis("cl-31").is_err(), "there are thirty classes");
        assert!(parse_axis("").is_err());
    }

    #[test]
    fn an_amount_is_exact_in_the_unit_or_it_is_not_an_amount() {
        assert_eq!(Amount::parse("1/2").ok().and_then(Amount::units), Some(360));
        assert_eq!(Amount::parse("1").ok().and_then(Amount::units), Some(720));
        assert_eq!(Amount::parse("0").ok().and_then(Amount::units), Some(0));
        assert_eq!(Amount::parse("1/5").ok().and_then(Amount::units), Some(144));
        assert_eq!(
            Amount::parse("2/4"),
            Amount::parse("1/2"),
            "a fraction is reduced"
        );
        assert_eq!(
            Amount::parse("1/16").ok().and_then(Amount::units),
            Some(45),
            "720 is 16 times 45, so a sixteenth *is* exact; ADR 0007 and generation.md both \
             offer a sixteenth as the example of an amount that would fail, and it does not. \
             A thirty-second is 22.5 units and does fail, which is what this gate rejects."
        );
        assert_eq!(
            Amount::parse("1/32").ok().and_then(Amount::units),
            None,
            "1/720 cannot state a thirty-second exactly"
        );
        assert_eq!(Amount::parse("1/7").ok().and_then(Amount::units), None);
        assert!(Amount::parse("1/0").is_err());
        assert!(Amount::parse("half").is_err());
    }

    #[test]
    fn the_token_vocabulary_is_the_legends_and_nothing_else() {
        assert!(parse_value(1, "1/4 be + 1/4 af").is_ok());
        assert!(parse_value(1, "1/2 be hang").is_ok());
        assert!(parse_value(1, "ruby hang").is_ok());
        assert!(parse_value(2, "not 3,4").is_ok());
        assert!(parse_value(3, "1/2-0 stage 4").is_ok());
        assert!(
            parse_value(3, "1/2\u{2013}0 stage 4").is_ok(),
            "an en dash is a hyphen here"
        );
        assert!(parse_value(6, "residual").is_ok());
        assert!(
            parse_value(1, "1/2").is_err(),
            "Appendix B names the em of every amount"
        );
        assert!(parse_value(1, "1/2 both").is_err());
        assert!(parse_value(2, "sometimes").is_err());
        assert!(parse_value(2, "not 5").is_err(), "§C.3 has four levels");
        assert!(
            parse_value(3, "residual").is_err(),
            "the residual step is Appendix E's"
        );
        assert!(parse_value(3, "1/2-0 stage x").is_err());
    }

    #[test]
    fn the_published_filenames_are_off_by_one_and_stay_that_way() {
        assert_eq!(published_file(1, "en"), "table_en2.pdf");
        assert_eq!(published_file(6, "ja"), "table_ja7.pdf");
    }

    /// One well-formed English entry of Table 1, for the tests that vary one thing.
    const ENGLISH: &str = "# [capture]\n# author = \"A\"\n# date = \"2026-08-07\"\n\
         source\ttable\tbefore\tafter\ttoken\tnote\n\
         table_en2.pdf\t1\tcl-05\tcl-05\t1/4 be + 1/4 af\tB.2#3\n\
         table_en2.pdf\t1\tcl-02\tline-end\t1/2 be\tB.2#2\n";

    #[test]
    fn a_well_formed_entry_reports_nothing() {
        let mut findings = Findings::default();
        let matrix = parse_matrix("table1.en.tsv", 1, "en", ENGLISH, &mut findings);
        assert_eq!(matrix.cells.len(), 2);
        assert_eq!(matrix.author, "A");
        assert_eq!(reported(findings), "");
    }

    #[test]
    fn a_cell_without_provenance_fails() {
        let cases = [
            (
                "table_en3.pdf\t1\tcl-05\tcl-05\t1/2 be\t\n",
                "table_en2.pdf",
            ),
            ("table_en2.pdf\t2\tcl-05\tcl-05\t1/2 be\t\n", "table `2`"),
            ("table_en2.pdf\t1\tcl-5\tcl-05\t1/2 be\t\n", "cl-01"),
            (
                "table_en2.pdf\t1\tcl-05\tcl-05\t\t\n",
                "empty cell is written `blank`",
            ),
            ("table_en2.pdf\t1\tcl-05\tcl-05\t1/2 be\n", "field(s)"),
            (
                "table_en2.pdf\t1\tline-end\tcl-05\t1/2 be\t\n",
                "line head is a row",
            ),
            (
                "table_en2.pdf\t1\tcl-05\tcl-05\t1/2 be\tD.2#3\n",
                "note of §B.2",
            ),
            (
                "table_en2.pdf\t1\tcl-05\tcl-05\t1/2 be\tB.2 note 3\n",
                "note of §B.2",
            ),
        ];
        for (row, expected) in cases {
            let text = format!(
                "# [capture]\n# author = \"A\"\n# date = \"2026-08-07\"\n\
                 source\ttable\tbefore\tafter\ttoken\tnote\n{row}"
            );
            let mut findings = Findings::default();
            let matrix = parse_matrix("table1.en.tsv", 1, "en", &text, &mut findings);
            let report = reported(findings);
            assert!(report.contains(expected), "{row} reported {report}");
            assert!(matrix.cells.is_empty(), "a rejected row is not a cell");
        }
    }

    #[test]
    fn a_capture_without_an_author_or_a_date_fails() {
        let text = "# [capture]\nsource\ttable\tbefore\tafter\ttoken\tnote\n";
        let mut findings = Findings::default();
        parse_matrix("table1.en.tsv", 1, "en", text, &mut findings);
        let report = reported(findings);
        assert!(report.contains("names no author"), "{report}");
        assert!(report.contains("YYYY-MM-DD"), "{report}");
    }

    #[test]
    fn a_reordered_header_fails() {
        let text = "# [capture]\n# author = \"A\"\n# date = \"2026-08-07\"\n\
                    table\tsource\tbefore\tafter\ttoken\tnote\n";
        let mut findings = Findings::default();
        parse_matrix("table1.en.tsv", 1, "en", text, &mut findings);
        assert!(reported(findings).contains("the format is"));
    }

    /// Parse a fixture pair and return what double entry reported.
    fn entered_twice(english: &str, japanese: &str) -> String {
        let mut findings = Findings::default();
        let first = parse_matrix("table1.en.tsv", 1, "en", english, &mut findings);
        let second = parse_matrix("table1.ja.tsv", 1, "ja", japanese, &mut findings);
        double_entry("table1", &first, &second, &mut findings);
        reported(findings)
    }

    #[test]
    fn two_entries_that_agree_report_nothing() {
        let japanese = "# [capture]\n# author = \"B\"\n# date = \"2026-08-08\"\n\
             source\ttable\tbefore\tafter\ttoken\tnote\n\
             table_ja2.pdf\t1\tcl-02\tline-end\t1/2 be\tB.2#2\n\
             table_ja2.pdf\t1\tcl-05\tcl-05\t1/4 be + 1/4 af\tB.2#3\n";
        assert_eq!(entered_twice(ENGLISH, japanese), "");
    }

    #[test]
    fn two_entries_that_disagree_fail() {
        let japanese = "# [capture]\n# author = \"B\"\n# date = \"2026-08-08\"\n\
             source\ttable\tbefore\tafter\ttoken\tnote\n\
             table_ja2.pdf\t1\tcl-02\tline-end\t1/2 be\tB.2#2\n\
             table_ja2.pdf\t1\tcl-05\tcl-05\t1/4 be + 1/4 be\tB.2#3\n";
        let report = entered_twice(ENGLISH, japanese);
        assert!(report.contains("reads `1/4 be + 1/4 af` in en"), "{report}");
    }

    #[test]
    fn a_cell_entered_once_is_not_double_entry() {
        let japanese = "# [capture]\n# author = \"B\"\n# date = \"2026-08-08\"\n\
             source\ttable\tbefore\tafter\ttoken\tnote\n\
             table_ja2.pdf\t1\tcl-02\tline-end\t1/2 be\tB.2#2\n\
             table_ja2.pdf\t1\tcl-06\tcl-06\tblank\t\n";
        let report = entered_twice(ENGLISH, japanese);
        assert!(report.contains("missing from the ja entry"), "{report}");
        assert!(report.contains("missing from the en entry"), "{report}");
    }

    #[test]
    fn one_author_keying_both_entries_is_not_double_entry() {
        let japanese = ENGLISH.replace("table_en2.pdf", "table_ja2.pdf");
        let report = entered_twice(ENGLISH, &japanese);
        assert!(report.contains("keyed twice by `A`"), "{report}");
        assert!(report.contains("identical row order"), "{report}");
    }

    #[test]
    fn a_prohibition_holds_in_every_table_that_has_the_cell() {
        assert_eq!(
            run_over(
                prohibition_agrees_across_tables,
                &[(1, "cl-05", "cl-06", "×"), (3, "cl-05", "cl-06", "×")]
            ),
            ""
        );
        let report = run_over(
            prohibition_agrees_across_tables,
            &[
                (1, "cl-05", "cl-06", "×"),
                (3, "cl-05", "cl-06", "1/2-0 stage 4"),
            ],
        );
        assert!(report.contains("define `×` identically"), "{report}");
    }

    #[test]
    fn a_table6_blank_needs_a_table2_prohibition() {
        assert_eq!(
            run_over(
                table6_blank_faces_table2_not,
                &[(6, "cl-05", "cl-06", "blank"), (2, "cl-05", "cl-06", "not")]
            ),
            ""
        );
        let report = run_over(
            table6_blank_faces_table2_not,
            &[
                (6, "cl-05", "cl-06", "blank"),
                (2, "cl-05", "cl-06", "not 4"),
            ],
        );
        assert!(report.contains("no line break opportunity"), "{report}");
    }

    #[test]
    fn a_reduction_table_states_table1_amounts() {
        assert_eq!(
            run_over(
                unadjusted_amount_is_table1,
                &[
                    (1, "cl-05", "cl-06", "1/2 be"),
                    (3, "cl-05", "cl-06", "1/2-0 stage 4")
                ]
            ),
            ""
        );
        let wrong = run_over(
            unadjusted_amount_is_table1,
            &[
                (1, "cl-05", "cl-06", "1/2 be"),
                (3, "cl-05", "cl-06", "1/4-0 stage 4"),
            ],
        );
        assert!(wrong.contains("Table 1 says 360/720"), "{wrong}");
        let nothing_to_reduce = run_over(
            unadjusted_amount_is_table1,
            &[
                (1, "cl-05", "cl-06", "blank"),
                (3, "cl-05", "cl-06", "0-0 stage 4"),
            ],
        );
        assert!(
            nothing_to_reduce.contains("sets the boundary solid"),
            "{nothing_to_reduce}"
        );
    }

    #[test]
    fn no_reduction_sits_on_a_line_edge_that_forbids_one() {
        assert_eq!(
            run_over(
                no_reduction_at_the_line_head,
                &[(3, "line-head", "cl-06", "1/2 stage 4")]
            ),
            ""
        );
        let head = run_over(
            no_reduction_at_the_line_head,
            &[(3, "line-head", "cl-06", "1/2-0 stage 4")],
        );
        assert!(head.contains("reduction in the line-head row"), "{head}");
        let end = run_over(
            table4_no_reduction_at_the_line_end,
            &[(4, "cl-06", "line-end", "1/2-0 stage 4")],
        );
        assert!(end.contains("row cl-06, line-end"), "{end}");
        assert_eq!(
            run_over(
                table4_no_reduction_at_the_line_end,
                &[(3, "cl-06", "line-end", "1/2-0 stage 4")]
            ),
            "",
            "the line end restriction is Table 4's alone"
        );
    }

    #[test]
    fn tables_without_line_edge_axes_may_not_use_them() {
        assert_eq!(
            run_over(
                line_edge_axes_only_where_they_exist,
                &[(1, "line-head", "cl-06", "1/2 af")]
            ),
            ""
        );
        let report = run_over(
            line_edge_axes_only_where_they_exist,
            &[(2, "line-head", "cl-06", "not")],
        );
        assert!(report.contains("names no line-edge axis"), "{report}");
    }

    #[test]
    fn table2_prohibits_its_preamble_classes_at_every_level() {
        assert_eq!(
            run_over(
                table2_prohibited_at_all_levels,
                &[(2, "cl-01", "cl-05", "not"), (2, "cl-05", "cl-08", "not 4")]
            ),
            ""
        );
        let row = run_over(
            table2_prohibited_at_all_levels,
            &[(2, "cl-01", "cl-05", "not 4")],
        );
        assert!(row.contains("prohibits this at all levels"), "{row}");
        let column = run_over(
            table2_prohibited_at_all_levels,
            &[(2, "cl-05", "cl-06", "blank")],
        );
        assert!(column.contains("prohibits this at all levels"), "{column}");
    }

    #[test]
    fn the_line_edge_prohibitions_are_counted_over_a_complete_table() {
        let mut findings = Findings::default();
        let ten = [1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        line_start_prohibited_classes(&full_table2(&ten, &[11, 12]), &mut findings);
        line_end_prohibited_classes(&full_table2(&ten, &[11, 12]), &mut findings);
        assert_eq!(reported(findings), "");

        let mut findings = Findings::default();
        line_start_prohibited_classes(&full_table2(&[1, 2, 3], &[11, 12]), &mut findings);
        line_end_prohibited_classes(&full_table2(&ten, &[11]), &mut findings);
        let report = reported(findings);
        assert!(report.contains("§3.1.7 names ten"), "{report}");
        assert!(report.contains("§3.1.8 names two"), "{report}");

        let mut findings = Findings::default();
        line_start_prohibited_classes(&capture(&[(2, "cl-05", "cl-06", "not")]), &mut findings);
        assert_eq!(
            reported(findings),
            "",
            "counting over a partial table would report every untranscribed column"
        );
    }

    #[test]
    fn a_hang_needs_a_space_to_hang_over() {
        assert_eq!(
            run_over(
                hang_sits_on_a_space,
                &[
                    (1, "cl-05", "cl-06", "1/2 be hang"),
                    (1, "cl-06", "cl-07", "ruby hang")
                ]
            ),
            ""
        );
        let third = run_over(
            hang_sits_on_a_space,
            &[(1, "cl-05", "cl-06", "1/3 be hang")],
        );
        assert!(third.contains("half or quarter em space"), "{third}");
    }

    #[test]
    fn a_bracket_class_that_differs_from_its_original_says_why() {
        assert_eq!(
            run_over(
                bracket_classes_mirror_their_originals,
                &[
                    (1, "cl-28", "cl-05", "1/2 be"),
                    (1, "cl-01", "cl-05", "1/2 be")
                ]
            ),
            ""
        );
        let report = run_over(
            bracket_classes_mirror_their_originals,
            &[
                (1, "cl-28", "cl-05", "1/4 be"),
                (1, "cl-01", "cl-05", "1/2 be"),
            ],
        );
        assert!(report.contains("no note states the difference"), "{report}");
    }

    #[test]
    fn table4_states_the_jis_reading_of_the_line_end() {
        assert_eq!(
            run_over(
                table4_line_end_follows_the_jis_reading,
                &[
                    (4, "cl-06", "line-end", "1/2 stage 3"),
                    (4, "cl-02", "line-end", "blank")
                ]
            ),
            ""
        );
        let report = run_over(
            table4_line_end_follows_the_jis_reading,
            &[(4, "cl-06", "line-end", "1/4 stage 3")],
        );
        assert!(
            report.contains("§3.1.9's JIS reading is 360/720"),
            "{report}"
        );
    }

    #[test]
    fn a_ladder_is_an_ordering_with_no_gap_and_no_extra_step() {
        assert_eq!(
            run_over(
                stage_ordinals_are_contiguous,
                &[
                    (3, "cl-05", "cl-06", "1/2-0 stage 4"),
                    (3, "cl-06", "cl-07", "1/2-0 stage 5")
                ]
            ),
            ""
        );
        let outside = run_over(
            stage_ordinals_are_contiguous,
            &[(6, "cl-05", "cl-06", "1/4-1/2 stage 5")],
        );
        assert!(outside.contains("that ladder has 4 steps"), "{outside}");
        let gap = run_over(
            stage_ordinals_are_contiguous,
            &[
                (3, "cl-05", "cl-06", "1/2-0 stage 2"),
                (3, "cl-06", "cl-07", "1/2-0 stage 4"),
            ],
        );
        assert!(gap.contains("which skips 3"), "{gap}");
    }

    #[test]
    fn an_amount_the_unit_cannot_state_fails_the_build() {
        assert_eq!(
            run_over(
                amounts_are_multiples_of_the_unit,
                &[(1, "cl-05", "cl-06", "1/8 be")]
            ),
            ""
        );
        let report = run_over(
            amounts_are_multiples_of_the_unit,
            &[(1, "cl-05", "cl-06", "1/32 be")],
        );
        assert!(report.contains("1/720 cannot state exactly"), "{report}");
        let limit = run_over(
            amounts_are_multiples_of_the_unit,
            &[(3, "cl-05", "cl-06", "1/2-1/7 stage 4")],
        );
        assert!(limit.contains("1/720 cannot state exactly"), "{limit}");
    }

    #[test]
    fn a_boundary_has_two_owners_and_no_more() {
        assert_eq!(
            run_over(
                at_most_one_space_per_referent,
                &[(1, "cl-05", "cl-05", "1/4 be + 1/4 af")]
            ),
            ""
        );
        let report = run_over(
            at_most_one_space_per_referent,
            &[(1, "cl-05", "cl-05", "1/4 be + 1/4 be + 1/4 af")],
        );
        assert!(
            report.contains("2 contributions from one neighbor"),
            "{report}"
        );
    }

    /// Check a catalogue fixture the way `check_catalogue` does, without the filesystem.
    fn catalogue(text: &str) -> String {
        let mut findings = Findings::default();
        let published = check_id_column(
            "invariants.tsv",
            text,
            &CATALOGUE_COLUMNS,
            &INVARIANTS.iter().map(|each| each.id).collect::<Vec<_>>(),
            "catalogue",
            &mut findings,
        );
        for invariant in INVARIANTS {
            if let Some(cited) = published.get(invariant.id).and_then(|row| row.get(1)) {
                if cited != invariant.citation {
                    findings.push("catalogue", format!("{id} is miscited", id = invariant.id));
                }
            }
        }
        reported(findings)
    }

    #[test]
    fn the_published_catalogue_names_exactly_the_implemented_invariants() {
        let mut complete = String::from("id\tcitation\tsentence\n");
        for invariant in INVARIANTS {
            complete.push_str(invariant.id);
            complete.push('\t');
            complete.push_str(invariant.citation);
            complete.push_str("\tstated\n");
        }
        assert_eq!(catalogue(&complete), "");

        let renamed = complete.replacen("hang-sits-on-a-space", "made-up", 1);
        let report = catalogue(&renamed);
        assert!(report.contains("records `made-up`"), "{report}");
        assert!(
            report.contains("does not record `hang-sits-on-a-space`"),
            "{report}"
        );

        let miscited = complete.replacen("§B.1, §C.1, §D.1, §E.1", "§B.1", 1);
        assert!(catalogue(&miscited).contains("is miscited"));
    }

    #[test]
    fn a_defect_that_disappears_forces_a_review() {
        let mut rows = String::new();
        for id in RECORDED_DEFECTS {
            rows.push_str(id);
            rows.push_str("\t§A.19\tmeasured\n");
        }
        let mut findings = Findings::default();
        check_id_column(
            "defects.tsv",
            &format!("id\twhere\tevidence\n{rows}"),
            &DEFECT_COLUMNS,
            &RECORDED_DEFECTS,
            "defects",
            &mut findings,
        );
        assert_eq!(reported(findings), "");

        rows = rows.replace("cl-19-duplicate-u216b\t§A.19\tmeasured\n", "");
        let mut findings = Findings::default();
        check_id_column(
            "defects.tsv",
            &format!("id\twhere\tevidence\n{rows}"),
            &DEFECT_COLUMNS,
            &RECORDED_DEFECTS,
            "defects",
            &mut findings,
        );
        assert!(reported(findings).contains("cl-19-duplicate-u216b"));

        let mut findings = Findings::default();
        check_id_column(
            "defects.tsv",
            "id\twhere\tevidence\ncl-19-duplicate-u216b\t\tmeasured\n",
            &DEFECT_COLUMNS,
            &RECORDED_DEFECTS,
            "defects",
            &mut findings,
        );
        assert!(reported(findings).contains("non-empty column(s)"));
    }

    #[test]
    fn a_transcription_is_recognized_wherever_it_sits() {
        assert!(is_transcription("table1.en.tsv"));
        assert!(is_transcription("figures.ja.tsv"));
        assert!(is_transcription("invariants.tsv"));
        assert!(!is_transcription("appendix-a.tsv"));
        assert!(!is_transcription("table1.tsv"));
        assert_eq!(split_locale("table3.ja.tsv"), Some(("table3", "ja")));
        assert_eq!(split_locale("table3.de.tsv"), None);
    }

    #[test]
    fn a_provenance_record_states_four_things() {
        let text = "[[document]]\npath = \"upstream/table_en2.pdf\"\n\
                    url = \"https://example.invalid/table_en2.pdf\"\n\
                    retrieved = \"2026-08-07\"\nsha256 = \"00\"\n";
        let documents = read_provenance(text);
        assert_eq!(documents.len(), 1);
        let document = documents.first().expect("one document");
        assert_eq!(document.path, "upstream/table_en2.pdf");
        assert_eq!(document.retrieved, "2026-08-07");
        assert_eq!(document.line, 1);
        assert!(read_provenance("[package]\nname = \"x\"\n").is_empty());
    }

    #[test]
    fn the_digest_is_the_published_one() {
        assert_eq!(
            sha256_hex(b"").as_deref(),
            Some("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
        );
        assert_eq!(
            sha256_hex(b"abc").as_deref(),
            Some("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
        );
        assert_eq!(
            sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq").as_deref(),
            Some("248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1")
        );
        let long = vec![b'a'; 1_000_000];
        assert_eq!(
            sha256_hex(&long).as_deref(),
            Some("cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0")
        );
    }

    #[test]
    fn the_gate_takes_one_flag_and_refuses_the_rest() {
        assert_eq!(wants_digests(&[]).ok(), Some(false));
        assert_eq!(wants_digests(&["--digests".to_owned()]).ok(), Some(true));
        assert!(wants_digests(&["--check".to_owned()]).is_err());
    }

    #[test]
    fn a_systematic_error_reports_a_sample_and_a_count() {
        let mut findings = Findings::default();
        for index in 0..20u8 {
            findings.push("kind", format!("violation {index}"));
        }
        let violations = findings.into_violations();
        assert_eq!(violations.len(), 9, "eight shown and one count");
        assert!(
            violations
                .last()
                .is_some_and(|last| last.contains("12 further")),
            "{violations:?}"
        );
    }

    #[test]
    fn every_invariant_is_named_once() {
        let mut names: Vec<&str> = INVARIANTS.iter().map(|each| each.id).collect();
        names.sort_unstable();
        let unique = names.len();
        names.dedup();
        assert_eq!(
            names.len(),
            unique,
            "an identifier is published and must be stable"
        );
        assert_eq!(INVARIANTS.len(), 18, "generation.md states eighteen");
        assert_eq!(LEVELS.len(), 4, "§C.3 states four");
    }

    #[test]
    fn an_empty_capture_is_examined_and_reports_nothing() {
        let capture = Capture::default();
        let mut findings = Findings::default();
        for invariant in INVARIANTS {
            if let Check::Whole { run } | Check::Partial { run, .. } = &invariant.check {
                run(&capture, &mut findings);
            }
        }
        assert_eq!(
            reported(findings),
            "",
            "nothing transcribed is nothing to object to"
        );
    }

    #[test]
    fn the_matrix_shapes_are_the_ones_the_appendices_have() {
        assert_eq!(full_size(1), 961);
        assert_eq!(full_size(2), 900);
        assert_eq!(full_size(4), 961);
        assert_eq!(full_size(6), 900);
    }

    /// A silenced check is the one failure mode a gate must not have.
    #[test]
    fn every_registered_invariant_that_runs_can_fail() {
        let unfailable = capture(&[(1, "cl-05", "cl-06", "1/2 be")]);
        let mut findings = Findings::default();
        prohibition_agrees_across_tables(&unfailable, &mut findings);
        assert_eq!(reported(findings), "");
        let mut findings = Findings::default();
        prohibition_agrees_across_tables(
            &capture(&[(1, "cl-05", "cl-06", "×"), (6, "cl-05", "cl-06", "blank")]),
            &mut findings,
        );
        assert_ne!(reported(findings), "");
    }
}
