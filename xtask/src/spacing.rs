// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Stage 2 generation for the unified engine: the six captured matrices of Appendices B
//! through E, turned into private `static` arrays under `crates/jlreq-core/src/generated/`.
//!
//! `spec/captured/table<N>.en.tsv` is the input for each unit. The Japanese rendering,
//! `table<N>.ja.tsv`, is not: `generate`'s own `Unit` reads one file, and the control that
//! the *other* locale agrees cell for cell is `xtask attest`'s double entry, which runs in
//! the same `just design` recipe as `generate --check`. Together the two gates prove what
//! neither proves alone — that the committed Rust is the byte-identical output of the
//! English transcription, and that the English transcription agrees with the independently
//! keyed Japanese one — which is the resolution `docs/design/generation.md` and ADR-0009
//! call for: the transcription in `spec/captured/` is the one primary source, and the
//! generated module is its sole machine-written projection, not a second copy of the data
//! (see the library's own `src/lib.rs` for the fuller statement of this).
//!
//! Every cell's citing rule is the qualifying appendix note (`B.2#3` reads as
//! `RuleId::B_2_NOTE_3`) when the `note` column names one, and the appendix's own legend
//! rule otherwise (`RuleId::SPACING_BETWEEN_CHARACTERS` for Table 1, and so on) — so every
//! emitted cell cites something, never nothing. The name is assembled as text: `xtask`
//! deliberately reads the derived inventory rather than linking the layout library, so
//! generated data remains a reproducible projection of the snapshot.
//!
//! The token grammar is the one `xtask/src/attest.rs`'s module doc comment publishes.
//! Reading it here is a second, independent implementation of that grammar rather than a
//! shared one, because `attest`'s `Value` and this module's raw cell shapes serve different
//! ends — `attest` builds an abstract value it cross-checks between two renderings and
//! against the registered invariants, this module builds the integer stage data the unified
//! evaluator reads at run time — and a shared value type would have had to serve both. The
//! two are proven to agree the same way the two locales are: `just design`
//! runs `attest` and `generate --check` together, over the same committed files, so a
//! divergent reading of one cell fails one gate or the other rather than neither.

use std::fmt::Write as _;

use crate::generate::{Emission, Table, Unit};

/// The six generated matrices, in `docs/design/generation.md`'s order.
pub(crate) const TABLE1: Unit = Unit {
    input: "spec/captured/table1.en.tsv",
    generator: &["xtask/src/spacing.rs"],
    output: "crates/jlreq-spacing/src/generated/table1.rs",
    summary: "Table 1, \"Spacing between Characters\" (Appendix B).",
    emit: emit_table1,
};

/// Table 1 behind the public core library's dependency-free private boundary.
pub(crate) const UNIFIED_TABLE1: Unit = Unit {
    input: "spec/captured/table1.en.tsv",
    generator: &["xtask/src/spacing.rs"],
    output: "crates/jlreq-core/src/generated/table1.rs",
    summary: "Table 1, \"Spacing between Characters\" (Appendix B).",
    emit: emit_unified_table1,
};

/// Table 2, "Possibilities for Line-breaking between Characters" (Appendix C).
pub(crate) const TABLE2: Unit = Unit {
    input: "spec/captured/table2.en.tsv",
    generator: &["xtask/src/spacing.rs"],
    output: "crates/jlreq-spacing/src/generated/table2.rs",
    summary: "Table 2, \"Possibilities for Line-breaking between Characters\" (Appendix C).",
    emit: emit_table2,
};

/// Table 2 behind the public core library's dependency-free private boundary.
pub(crate) const UNIFIED_TABLE2: Unit = Unit {
    input: "spec/captured/table2.en.tsv",
    generator: &["xtask/src/spacing.rs"],
    output: "crates/jlreq-core/src/generated/table2.rs",
    summary: "Table 2, \"Possibilities for Line-breaking between Characters\" (Appendix C).",
    emit: emit_unified_table2,
};

/// Table 3, JLReq's own reduction-priority reading (Appendix D).
pub(crate) const TABLE3: Unit = Unit {
    input: "spec/captured/table3.en.tsv",
    generator: &["xtask/src/spacing.rs"],
    output: "crates/jlreq-spacing/src/generated/table3.rs",
    summary: "Table 3, JLReq's own reduction-priority reading (Appendix D).",
    emit: |table| emit_ranged(table, RangedTable::Reduction, "LEGEND_OF_TABLES_3_4_AND_5"),
};

/// Table 3 behind the public core library's dependency-free private boundary.
pub(crate) const UNIFIED_TABLE3: Unit = Unit {
    input: "spec/captured/table3.en.tsv",
    generator: &["xtask/src/spacing.rs"],
    output: "crates/jlreq-core/src/generated/table3.rs",
    summary: "Table 3, JLReq's own reduction-priority reading (Appendix D).",
    emit: |table| emit_unified_ranged(table, RangedTable::Reduction, "D.1"),
};

/// Table 4, the JIS X 4051 reduction-priority reading (Appendix D).
pub(crate) const TABLE4: Unit = Unit {
    input: "spec/captured/table4.en.tsv",
    generator: &["xtask/src/spacing.rs"],
    output: "crates/jlreq-spacing/src/generated/table4.rs",
    summary: "Table 4, the JIS X 4051 reduction-priority reading (Appendix D).",
    emit: |table| emit_ranged(table, RangedTable::Reduction, "LEGEND_OF_TABLES_3_4_AND_5"),
};

/// Table 4 behind the public core library's dependency-free private boundary.
pub(crate) const UNIFIED_TABLE4: Unit = Unit {
    input: "spec/captured/table4.en.tsv",
    generator: &["xtask/src/spacing.rs"],
    output: "crates/jlreq-core/src/generated/table4.rs",
    summary: "Table 4, the JIS X 4051 reduction-priority reading (Appendix D).",
    emit: |table| emit_unified_ranged(table, RangedTable::Reduction, "D.1"),
};

/// Table 5, the book-practice reduction-priority reading (Appendix D).
pub(crate) const TABLE5: Unit = Unit {
    input: "spec/captured/table5.en.tsv",
    generator: &["xtask/src/spacing.rs"],
    output: "crates/jlreq-spacing/src/generated/table5.rs",
    summary: "Table 5, the book-practice reduction-priority reading (Appendix D).",
    emit: |table| emit_ranged(table, RangedTable::Reduction, "LEGEND_OF_TABLES_3_4_AND_5"),
};

/// Table 5 behind the public core library's dependency-free private boundary.
pub(crate) const UNIFIED_TABLE5: Unit = Unit {
    input: "spec/captured/table5.en.tsv",
    generator: &["xtask/src/spacing.rs"],
    output: "crates/jlreq-core/src/generated/table5.rs",
    summary: "Table 5, the book-practice reduction-priority reading (Appendix D).",
    emit: |table| emit_unified_ranged(table, RangedTable::Reduction, "D.1"),
};

/// Table 6, "Opportunities for Inter-character Space Expansion" (Appendix E).
pub(crate) const TABLE6: Unit = Unit {
    input: "spec/captured/table6.en.tsv",
    generator: &["xtask/src/spacing.rs"],
    output: "crates/jlreq-spacing/src/generated/table6.rs",
    summary: "Table 6, \"Opportunities for Inter-character Space Expansion\" (Appendix E).",
    emit: |table| {
        emit_ranged(
            table,
            RangedTable::Expansion,
            "OPPORTUNITIES_FOR_INTER_CHARACTER_SPACE_EXPANSION_DURING_LINE_ADJUSTMENT",
        )
    },
};

/// Table 6 behind the public core library's dependency-free private boundary.
pub(crate) const UNIFIED_TABLE6: Unit = Unit {
    input: "spec/captured/table6.en.tsv",
    generator: &["xtask/src/spacing.rs"],
    output: "crates/jlreq-core/src/generated/table6.rs",
    summary: "Table 6, \"Opportunities for Inter-character Space Expansion\" (Appendix E).",
    emit: |table| emit_unified_ranged(table, RangedTable::Expansion, "E"),
};

/// One transcribed row, read by column name rather than position (`generate::Record`'s
/// fields are positional, and a captured file's column order is provenance-bearing but not
/// guaranteed stable to a generator that only wants four of its six columns).
struct Row<'t> {
    line: usize,
    before: &'t str,
    after: &'t str,
    token: &'t str,
    note: &'t str,
}

/// Read every row of a captured matrix, by column name.
fn rows(table: &Table) -> Result<Vec<Row<'_>>, String> {
    let column = |name: &str| {
        table
            .columns
            .iter()
            .position(|each| each == name)
            .ok_or_else(|| format!("{}: has no `{name}` column", table.source))
    };
    let before = column("before")?;
    let after = column("after")?;
    let token = column("token")?;
    let note = column("note")?;
    let mut out = Vec::with_capacity(table.records.len());
    for record in &table.records {
        let field = |index: usize| {
            record
                .fields
                .get(index)
                .map(String::as_str)
                .ok_or_else(|| format!("{}:{}: too few fields", table.source, record.line))
        };
        out.push(Row {
            line: record.line,
            before: field(before)?,
            after: field(after)?,
            token: field(token)?,
            note: field(note)?,
        });
    }
    Ok(out)
}

/// Read a `cl-NN`, `line-head` or `line-end` axis label into the `u8` sentinel form
/// `crates/jlreq-spacing/src/raw.rs` uses: `0` for the line edge, `1..=30` for a class.
fn axis(label: &str) -> Result<u8, String> {
    if label == "line-head" || label == "line-end" {
        return Ok(0);
    }
    let Some(digits) = label.strip_prefix("cl-") else {
        return Err(format!("`{label}` is not a class label or a line edge"));
    };
    digits
        .parse::<u8>()
        .ok()
        .filter(|number| (1..=30).contains(number))
        .ok_or_else(|| format!("`{label}` is not cl-01 through cl-30"))
}

/// Read a fraction (`1/4`, `1/2`, `1`, `0`) into units of 1/720 em (ADR-0007), exactly —
/// `xtask attest`'s `amounts-are-multiples-of-the-unit` invariant is what already proves
/// every committed amount survives this without rounding.
fn units(text: &str) -> Result<i32, String> {
    let (numerator, denominator) = text.split_once('/').unwrap_or((text, "1"));
    let numerator: i64 = numerator
        .trim()
        .parse()
        .map_err(|_| format!("`{text}` is not an amount"))?;
    let denominator: i64 = denominator
        .trim()
        .parse()
        .map_err(|_| format!("`{text}` is not an amount"))?;
    if denominator == 0 {
        return Err(format!("`{text}` divides by zero"));
    }
    let scaled = numerator
        .checked_mul(720)
        .ok_or_else(|| format!("`{text}` overflows"))?;
    let remainder = scaled
        .checked_rem(denominator)
        .ok_or_else(|| format!("`{text}` divides by zero"))?;
    if remainder != 0 {
        return Err(format!("`{text}` is not an exact multiple of 1/720 em"));
    }
    let divided = scaled
        .checked_div(denominator)
        .ok_or_else(|| format!("`{text}` divides by zero"))?;
    i32::try_from(divided).map_err(|_| format!("`{text}` overflows i32"))
}

/// The rule that states one cell: the qualifying appendix note when `note` names one, or
/// `fallback` — the appendix's own legend rule — when it is empty.
///
/// The identifier is assembled as text and never resolved here (`xtask` declares no
/// dependency on `jlreq-spec`); a wrong one is a compile failure in `jlreq-spacing`, which
/// is the same trust boundary `xtask/src/classes.rs` and `xtask/src/inventory.rs` already
/// cross when they emit an `Address::assembled` or a `Standing::Normative` they cannot read
/// back.
fn rule_constant(note: &str, fallback: &str) -> Result<String, String> {
    if note.is_empty() {
        return Ok(format!("jlreq_spec::RuleId::{fallback}"));
    }
    let Some((section, ordinal)) = note.split_once('#') else {
        return Err(format!("`{note}` is not `<section>#<ordinal>`"));
    };
    let section = section.replace('.', "_");
    Ok(format!("jlreq_spec::RuleId::{section}_NOTE_{ordinal}"))
}

/// A Table 1 cell's terms: one `(trailing, units)` pair per referent, at most two
/// (ADR-0014).
type Terms = Vec<(bool, i32)>;

/// Table 1: parse one cell token into its terms, its ruby-overhang permission, and whether
/// the adjacency itself is prohibited.
fn spacing_cell(row: &Row<'_>) -> Result<(bool, &'static str, Terms), String> {
    if row.token == "×" {
        return Ok((true, "RawHang::None", Vec::new()));
    }
    if row.token == "blank" {
        return Ok((false, "RawHang::None", Vec::new()));
    }
    if row.token == "ruby hang" {
        return Ok((false, "RawHang::OverCharacter", Vec::new()));
    }
    let (body, hang) = row
        .token
        .strip_suffix(" hang")
        .map_or((row.token, "RawHang::None"), |body| {
            (body, "RawHang::OverSpace")
        });
    let mut terms = Vec::new();
    for part in body.split('+') {
        let part = part.trim();
        let Some((amount, referent)) = part.rsplit_once(' ') else {
            return Err(format!(
                "{}:{}: `{part}` names no referent",
                "table1", row.line
            ));
        };
        let trailing = match referent {
            "af" => true,
            "be" => false,
            other => return Err(format!("`{other}` is not `be` or `af`")),
        };
        terms.push((trailing, units(amount)?));
    }
    Ok((false, hang, terms))
}

/// Render the `terms: &[...]` field of one `RawSpacingCell`, in the exact shape
/// `cargo fmt` gives it: empty inline, one term with the array bracket and the term's own
/// opening brace on one line, two terms each fully expanded on their own block.
///
/// `struct_lit_width` (18, the stable default `rustfmt.toml` does not override) is well
/// under the width of a `RawTerm { trailing: .., amount: .. }` literal, so every non-empty
/// term is always multi-line; the only question rustfmt answers differently by count is
/// whether the array's own `[` shares a line with the first element.
fn render_terms(terms: &[(bool, i32)]) -> String {
    match terms {
        [] => "terms: &[],\n".to_owned(),
        [(trailing, value)] => format!(
            "terms: &[RawTerm {{\n            trailing: {trailing},\n            amount: em({value}),\n        }}],\n"
        ),
        many => {
            let mut rendered = String::from("terms: &[\n");
            for (trailing, value) in many {
                let _ = write!(
                    rendered,
                    "            RawTerm {{\n                trailing: {trailing},\n                amount: em({value}),\n            }},\n"
                );
            }
            rendered.push_str("        ],\n");
            rendered
        },
    }
}

/// Emit Table 1's generated module.
fn emit_table1(table: &Table) -> Result<Emission, String> {
    let rows = rows(table)?;
    let mut items = String::from(
        "use crate::raw::{RawHang, RawSpacingCell, RawTerm, em};\n\n\
         /// Table 1's cells, in the order the transcription was read.\n\
         ///\n\
         /// JLReq: §B.1\n\
         pub(crate) static CELLS: &[RawSpacingCell] = &[\n",
    );
    let mut entries = 0usize;
    for row in &rows {
        let before = axis(row.before)?;
        let after = axis(row.after)?;
        let (prohibited, hang, terms) = spacing_cell(row)
            .map_err(|reason| format!("{}:{}: {reason}", table.source, row.line))?;
        let rule = rule_constant(row.note, "SPACING_BETWEEN_CHARACTERS")?;
        let _ = write!(
            items,
            "    RawSpacingCell {{\n        before: {before},\n        after: {after},\n        \
             prohibited: {prohibited},\n        hang: {hang},\n        rule: {rule},\n        \
             {terms_field}    }},\n",
            terms_field = render_terms(&terms),
        );
        entries = entries.saturating_add(1);
    }
    items.push_str("];\n");
    Ok(Emission { items, entries })
}

/// Emit Table 1 for `jlreq`, retaining observable JLReq references without `RuleId`.
fn emit_unified_table1(table: &Table) -> Result<Emission, String> {
    let rows = rows(table)?;
    let mut items = String::from(
        "use crate::spec::{RawHang, RawSpacingCell, RawTerm, em};\n\n\
         /// Table 1's cells, in the order the transcription was read.\n\
         ///\n\
         /// JLReq: §B.1\n\
         pub(crate) static CELLS: &[RawSpacingCell] = &[\n",
    );
    let mut entries = 0usize;
    for row in &rows {
        let before = axis(row.before)?;
        let after = axis(row.after)?;
        let (prohibited, hang, terms) = spacing_cell(row)
            .map_err(|reason| format!("{}:{}: {reason}", table.source, row.line))?;
        let rule = if row.note.is_empty() { "B.1" } else { row.note };
        let _ = write!(
            items,
            "    RawSpacingCell {{\n        before: {before},\n        after: {after},\n        \
             prohibited: {prohibited},\n        hang: {hang},\n        rule: {rule:?},\n        \
             {terms_field}    }},\n",
            terms_field = render_terms(&terms),
        );
        entries = entries.saturating_add(1);
    }
    items.push_str("];\n");
    Ok(Emission { items, entries })
}

/// Table 2: parse one cell token into its strictness-level bitmask.
fn break_cell(row: &Row<'_>) -> Result<(bool, u8), String> {
    if row.token == "×" {
        return Ok((true, 0b1111));
    }
    if row.token == "blank" {
        return Ok((false, 0));
    }
    let Some(rest) = row.token.strip_prefix("not") else {
        return Err(format!(
            "`{}` is not `not`, `not <levels>`, `blank` or `×`",
            row.token
        ));
    };
    let rest = rest.trim();
    if rest.is_empty() {
        return Ok((false, 0b1111));
    }
    let mut levels = 0u8;
    for part in rest.split(',') {
        let level: u8 = part
            .trim()
            .parse()
            .map_err(|_| format!("`{part}` is not a strictness level"))?;
        let bit = match level {
            1 => 0b0001,
            2 => 0b0010,
            3 => 0b0100,
            4 => 0b1000,
            other => return Err(format!("level {other} is outside §C.3's four levels")),
        };
        levels |= bit;
    }
    Ok((false, levels))
}

/// Emit Table 2's generated module.
fn emit_table2(table: &Table) -> Result<Emission, String> {
    let rows = rows(table)?;
    let mut items = String::from(
        "use crate::raw::RawBreakCell;\n\n\
         /// Table 2's cells, in the order the transcription was read.\n\
         ///\n\
         /// JLReq: §C.1\n\
         pub(crate) static CELLS: &[RawBreakCell] = &[\n",
    );
    let mut entries = 0usize;
    for row in &rows {
        let before = axis(row.before)?;
        let after = axis(row.after)?;
        let (prohibited, levels) =
            break_cell(row).map_err(|reason| format!("{}:{}: {reason}", table.source, row.line))?;
        let rule = rule_constant(
            row.note,
            "POSSIBILITIES_FOR_LINE_BREAKING_BETWEEN_CHARACTERS",
        )?;
        let _ = write!(
            items,
            "    RawBreakCell {{\n        before: {before},\n        after: {after},\n        \
             prohibited: {prohibited},\n        levels: 0b{levels:04b},\n        rule: {rule},\n    }},\n"
        );
        entries = entries.saturating_add(1);
    }
    items.push_str("];\n");
    Ok(Emission { items, entries })
}

/// Emit Table 2 for `jlreq`, retaining observable JLReq references without `RuleId`.
fn emit_unified_table2(table: &Table) -> Result<Emission, String> {
    let rows = rows(table)?;
    let mut items = String::from(
        "use crate::spec::RawBreakCell;\n\n\
         /// Table 2's cells, in the order the transcription was read.\n\
         ///\n\
         /// JLReq: §C.1\n\
         pub(crate) static CELLS: &[RawBreakCell] = &[\n",
    );
    let mut entries = 0usize;
    for row in &rows {
        let before = axis(row.before)?;
        let after = axis(row.after)?;
        let (prohibited, levels) =
            break_cell(row).map_err(|reason| format!("{}:{}: {reason}", table.source, row.line))?;
        let rule = if row.note.is_empty() { "C" } else { row.note };
        let _ = write!(
            items,
            "    RawBreakCell {{\n        before: {before},\n        after: {after},\n        \
             prohibited: {prohibited},\n        levels: 0b{levels:04b},\n        rule: {rule:?},\n    }},\n"
        );
        entries = entries.saturating_add(1);
    }
    items.push_str("];\n");
    Ok(Emission { items, entries })
}

/// Which of the two ladders a ranged table belongs to (ADR-0014): the name is used only for
/// the doc comment the emitted module carries, because the two share one cell shape and
/// differ only in what the crate that reads them does with `limit`.
#[derive(Debug, Clone, Copy)]
enum RangedTable {
    Reduction,
    Expansion,
}

/// One Table 3 through 6 cell token, parsed.
#[derive(Debug, PartialEq, Eq)]
struct RangedToken {
    prohibited: bool,
    amount: i32,
    limit: Option<i32>,
    two_valued: bool,
    residual: bool,
    stage: Option<u8>,
}

/// Tables 3 through 6: parse one cell token into its amount, its limit, and its stage.
fn ranged_cell(token: &str) -> Result<RangedToken, String> {
    if token == "×" {
        return Ok(RangedToken {
            prohibited: true,
            amount: 0,
            limit: None,
            two_valued: false,
            residual: false,
            stage: None,
        });
    }
    if token == "blank" {
        return Ok(RangedToken {
            prohibited: false,
            amount: 0,
            limit: None,
            two_valued: false,
            residual: false,
            stage: None,
        });
    }
    let (body, stage) = match token.rsplit_once(" stage ") {
        Some((body, ordinal)) => (
            body.trim(),
            Some(
                ordinal
                    .trim()
                    .parse::<u8>()
                    .map_err(|_| format!("`{ordinal}` is not a stage ordinal"))?,
            ),
        ),
        None => (token, None),
    };
    if body == "residual" {
        return Ok(RangedToken {
            prohibited: false,
            amount: 0,
            limit: None,
            two_valued: false,
            residual: true,
            stage,
        });
    }
    let body = body.replace('\u{2013}', "-");
    let (amount, limit, two_valued) = match (body.split_once('='), body.split_once('-')) {
        (Some((amount, limit)), _) => (amount, Some(limit), true),
        (None, Some((amount, limit))) => (amount, Some(limit), false),
        (None, None) => (body.as_str(), None, false),
    };
    let amount_units = units(amount)?;
    let limit_units = limit.map(units).transpose()?;
    Ok(RangedToken {
        prohibited: false,
        amount: amount_units,
        limit: limit_units,
        two_valued,
        residual: false,
        stage,
    })
}

/// Emit one of Tables 3 through 6's generated modules.
fn emit_ranged(table: &Table, which: RangedTable, fallback: &str) -> Result<Emission, String> {
    let rows = rows(table)?;
    let doc = match which {
        RangedTable::Reduction => "§D.1",
        RangedTable::Expansion => "§E.1",
    };
    let mut items = format!(
        "use crate::raw::{{RawRangedCell, em}};\n\n\
         /// This table's cells, in the order the transcription was read.\n\
         ///\n\
         /// JLReq: {doc}\n\
         pub(crate) static CELLS: &[RawRangedCell] = &[\n"
    );
    let mut entries = 0usize;
    for row in &rows {
        let before = axis(row.before)?;
        let after = axis(row.after)?;
        let token = ranged_cell(row.token)
            .map_err(|reason| format!("{}:{}: {reason}", table.source, row.line))?;
        let rule = rule_constant(row.note, fallback)?;
        let limit_text = token
            .limit
            .map_or_else(|| "None".to_owned(), |value| format!("Some(em({value}))"));
        let stage_text = token.stage.unwrap_or(0);
        let _ = write!(
            items,
            "    RawRangedCell {{\n        before: {before},\n        after: {after},\n        \
             limit: {limit_text},\n        two_valued: {two_valued},\n        residual: {residual},\n        \
             stage: {stage_text},\n        rule: {rule},\n    }},\n",
            two_valued = token.two_valued,
            residual = token.residual,
        );
        entries = entries.saturating_add(1);
    }
    items.push_str("];\n");
    Ok(Emission { items, entries })
}

/// Emit one of Tables 3 through 6 for `jlreq`, retaining JLReq references as strings.
fn emit_unified_ranged(
    table: &Table,
    which: RangedTable,
    fallback: &str,
) -> Result<Emission, String> {
    let rows = rows(table)?;
    let doc = match which {
        RangedTable::Reduction => "§D.1",
        RangedTable::Expansion => "§E.1",
    };
    let mut items = format!(
        "use crate::spec::{{RawRangedCell, em}};\n\n\
         /// This table's cells, in the order the transcription was read.\n\
         ///\n\
         /// JLReq: {doc}\n\
         pub(crate) static CELLS: &[RawRangedCell] = &[\n"
    );
    let mut entries = 0usize;
    for row in &rows {
        let before = axis(row.before)?;
        let after = axis(row.after)?;
        let token = ranged_cell(row.token)
            .map_err(|reason| format!("{}:{}: {reason}", table.source, row.line))?;
        let rule = if row.note.is_empty() {
            fallback
        } else {
            row.note
        };
        let limit_text = token
            .limit
            .map_or_else(|| "None".to_owned(), |value| format!("Some(em({value}))"));
        let stage_text = token.stage.unwrap_or(0);
        let _ = write!(
            items,
            "    RawRangedCell {{\n        before: {before},\n        after: {after},\n        \
             limit: {limit_text},\n        two_valued: {two_valued},\n        residual: {residual},\n        \
             stage: {stage_text},\n        rule: {rule:?},\n    }},\n",
            two_valued = token.two_valued,
            residual = token.residual,
        );
        entries = entries.saturating_add(1);
    }
    items.push_str("];\n");
    Ok(Emission { items, entries })
}

#[cfg(test)]
mod tests {
    use super::{Row, axis, break_cell, ranged_cell, rule_constant, spacing_cell, units};

    fn row(before: &str, after: &str, token: &str, note: &str) -> Row<'static> {
        Row {
            line: 1,
            before: Box::leak(before.to_owned().into_boxed_str()),
            after: Box::leak(after.to_owned().into_boxed_str()),
            token: Box::leak(token.to_owned().into_boxed_str()),
            note: Box::leak(note.to_owned().into_boxed_str()),
        }
    }

    #[test]
    fn a_class_axis_reads_its_ordinal() {
        assert_eq!(axis("cl-05"), Ok(5));
        assert_eq!(axis("cl-30"), Ok(30));
        assert_eq!(axis("line-head"), Ok(0));
        assert_eq!(axis("line-end"), Ok(0));
        assert!(axis("cl-31").is_err());
        assert!(axis("cl-00").is_err());
    }

    #[test]
    fn a_fraction_is_exact_in_units_of_one_seven_hundred_twentieth() {
        assert_eq!(units("1/2"), Ok(360));
        assert_eq!(units("1/4"), Ok(180));
        assert_eq!(units("0"), Ok(0));
        assert_eq!(units("1"), Ok(720));
    }

    #[test]
    fn a_note_reads_as_its_own_named_constant() {
        assert_eq!(
            rule_constant("B.2#3", "SPACING_BETWEEN_CHARACTERS").as_deref(),
            Ok("jlreq_spec::RuleId::B_2_NOTE_3")
        );
        assert_eq!(
            rule_constant("", "SPACING_BETWEEN_CHARACTERS").as_deref(),
            Ok("jlreq_spec::RuleId::SPACING_BETWEEN_CHARACTERS")
        );
    }

    #[test]
    fn a_two_term_spacing_cell_reads_both_referents() {
        let cell = row("cl-06", "cl-05", "1/2 be + 1/4 af", "B.2#2");
        let (prohibited, hang, terms) = spacing_cell(&cell).expect("well formed");
        assert!(!prohibited);
        assert_eq!(hang, "RawHang::None");
        assert_eq!(terms, vec![(false, 360), (true, 180)]);
    }

    #[test]
    fn a_hang_suffix_qualifies_the_whole_cell() {
        let cell = row("cl-01", "cl-22", "1/4 af hang", "B.2#1");
        let (_, hang, terms) = spacing_cell(&cell).expect("well formed");
        assert_eq!(hang, "RawHang::OverSpace");
        assert_eq!(terms, vec![(true, 180)]);
    }

    #[test]
    fn ruby_hang_is_solid_with_the_character_permission() {
        let cell = row("cl-16", "cl-22", "ruby hang", "B.2#7");
        let (prohibited, hang, terms) = spacing_cell(&cell).expect("well formed");
        assert!(!prohibited);
        assert_eq!(hang, "RawHang::OverCharacter");
        assert!(terms.is_empty());
    }

    #[test]
    fn a_prohibited_spacing_cell_has_no_terms() {
        let cell = row("cl-14", "line-head", "×", "");
        let (prohibited, _, terms) = spacing_cell(&cell).expect("well formed");
        assert!(prohibited);
        assert!(terms.is_empty());
    }

    #[test]
    fn table_two_reads_not_and_its_level_list() {
        assert_eq!(
            break_cell(&row("cl-01", "cl-02", "not", "")),
            Ok((false, 0b1111))
        );
        assert_eq!(
            break_cell(&row("cl-24", "cl-27", "not 3,4", "")),
            Ok((false, 0b1100))
        );
        assert_eq!(
            break_cell(&row("cl-01", "cl-29", "×", "")),
            Ok((true, 0b1111))
        );
    }

    fn token(
        prohibited: bool,
        amount: i32,
        limit: Option<i32>,
        two_valued: bool,
        residual: bool,
        stage: Option<u8>,
    ) -> super::RangedToken {
        super::RangedToken {
            prohibited,
            amount,
            limit,
            two_valued,
            residual,
            stage,
        }
    }

    #[test]
    fn a_ranged_cell_reads_amount_limit_and_stage() {
        assert_eq!(
            ranged_cell("1/2-0 stage 4"),
            Ok(token(false, 360, Some(0), false, false, Some(4)))
        );
        assert_eq!(
            ranged_cell("1/2=0 stage 2"),
            Ok(token(false, 360, Some(0), true, false, Some(2)))
        );
        assert_eq!(
            ranged_cell("1/4 stage 3"),
            Ok(token(false, 180, None, false, false, Some(3)))
        );
        assert_eq!(
            ranged_cell("residual"),
            Ok(token(false, 0, None, false, true, None))
        );
        assert_eq!(
            ranged_cell("blank"),
            Ok(token(false, 0, None, false, false, None))
        );
        assert_eq!(
            ranged_cell("×"),
            Ok(token(true, 0, None, false, false, None))
        );
    }

    #[test]
    fn the_generation_units_hold_over_this_repository() {
        for unit in [
            super::TABLE1,
            super::UNIFIED_TABLE1,
            super::TABLE2,
            super::UNIFIED_TABLE2,
            super::TABLE3,
            super::UNIFIED_TABLE3,
            super::TABLE4,
            super::UNIFIED_TABLE4,
            super::TABLE5,
            super::UNIFIED_TABLE5,
            super::TABLE6,
            super::UNIFIED_TABLE6,
        ] {
            assert!(unit.input.starts_with("spec/captured/table"));
            assert!(unit.output.contains("/src/generated/table"));
        }
    }
}
