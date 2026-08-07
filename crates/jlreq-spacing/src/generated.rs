// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The six generated matrices, and the figures this crate is written against.
//!
//! Every Rust file inside `src/generated/` is machine-written: `cargo run -p xtask --
//! generate` emits it from `spec/captured/table<N>.en.tsv`, and `generate --check` fails
//! when regenerating it would change a byte. This file is the one hand-written module in
//! the neighborhood, which is why the module declarations live here beside the directory
//! rather than in a `src/generated/mod.rs` inside it (`docs/design/generation.md`).
//!
//! Tables 1, 3, 4 and 5 carry twenty-nine rows and twenty-nine columns: the thirty classes
//! less cl-17 and cl-18 (math symbols and math operators, whose spacing §3.7.4 states as a
//! function of the formula setting rather than as a table cell — see `evaluate`'s formula
//! override), plus the line head or the line end. Tables 2 and 6 carry twenty-eight rows
//! and twenty-eight columns: the same twenty-nine less the line edges, which §C.1 and §E.1
//! give neither. `29 * 29 = 841` and `28 * 28 = 784`, exactly the counts `xtask attest`
//! reports for the committed capture.
//!
//! The assertions below are `const` blocks rather than tests, so a table that is wrong does
//! not link (see `jlreq_class::generated`'s module doc for the fuller statement of why).

pub(crate) mod table1;
pub(crate) mod table2;
pub(crate) mod table3;
pub(crate) mod table4;
pub(crate) mod table5;
pub(crate) mod table6;

/// Rows and columns of Tables 1, 3, 4 and 5: the twenty-eight non-math classes, plus the
/// line head or the line end.
const CELLS_WITH_LINE_EDGE: usize = 841;

/// Rows and columns of Tables 2 and 6: the twenty-eight non-math classes, with no line-edge
/// axis at all.
const CELLS_WITHOUT_LINE_EDGE: usize = 784;

/// The class ordinals Tables 1 through 6 never carry: cl-17 and cl-18, whose spacing is a
/// function of the formula setting rather than of the class pair (§3.7.4).
const fn is_math_class(class: u8) -> bool {
    class == 17 || class == 18
}

/// Whether a `before`/`after` axis value is well formed: the line-edge sentinel, or a class
/// ordinal this table's rows may actually name.
const fn axis_ok(value: u8) -> bool {
    value == 0 || (value >= 1 && value <= 30 && !is_math_class(value))
}

/// Whether coordinate `index` repeats an earlier one, over a spacing table's cells.
const fn spacing_repeats(cells: &[crate::raw::RawSpacingCell], index: usize) -> bool {
    let mut earlier = 0;
    while earlier < index {
        if cells[earlier].before == cells[index].before
            && cells[earlier].after == cells[index].after
        {
            return true;
        }
        earlier = earlier.saturating_add(1);
    }
    false
}

/// Whether coordinate `index` repeats an earlier one, over a break table's cells.
const fn break_repeats(cells: &[crate::raw::RawBreakCell], index: usize) -> bool {
    let mut earlier = 0;
    while earlier < index {
        if cells[earlier].before == cells[index].before
            && cells[earlier].after == cells[index].after
        {
            return true;
        }
        earlier = earlier.saturating_add(1);
    }
    false
}

/// Whether coordinate `index` repeats an earlier one, over a ranged table's cells.
const fn ranged_repeats(cells: &[crate::raw::RawRangedCell], index: usize) -> bool {
    let mut earlier = 0;
    while earlier < index {
        if cells[earlier].before == cells[index].before
            && cells[earlier].after == cells[index].after
        {
            return true;
        }
        earlier = earlier.saturating_add(1);
    }
    false
}

const _: () = assert!(
    table1::CELLS.len() == CELLS_WITH_LINE_EDGE,
    "Table 1 no longer holds the number of cells this crate was written against"
);
const _: () = {
    let mut index = 0;
    while index < table1::CELLS.len() {
        let cell = &table1::CELLS[index];
        assert!(
            axis_ok(cell.before) && axis_ok(cell.after),
            "Table 1 names cl-17 or cl-18"
        );
        assert!(
            cell.terms.len() <= 2,
            "a boundary carries at most two conditional spaces"
        );
        assert!(
            !spacing_repeats(table1::CELLS, index),
            "Table 1 transcribes one coordinate twice"
        );
        index = index.saturating_add(1);
    }
};

const _: () = assert!(
    table2::CELLS.len() == CELLS_WITHOUT_LINE_EDGE,
    "Table 2 no longer holds the number of cells this crate was written against"
);
const _: () = {
    let mut index = 0;
    while index < table2::CELLS.len() {
        let cell = &table2::CELLS[index];
        assert!(
            cell.before != 0 && cell.after != 0 && axis_ok(cell.before) && axis_ok(cell.after),
            "Table 2 carries no line-edge axis and no math class"
        );
        assert!(
            !break_repeats(table2::CELLS, index),
            "Table 2 transcribes one coordinate twice"
        );
        index = index.saturating_add(1);
    }
};

const _: () = assert!(table3::CELLS.len() == CELLS_WITH_LINE_EDGE);
const _: () = assert!(table4::CELLS.len() == CELLS_WITH_LINE_EDGE);
const _: () = assert!(table5::CELLS.len() == CELLS_WITH_LINE_EDGE);
const _: () = assert!(table6::CELLS.len() == CELLS_WITHOUT_LINE_EDGE);

const _: () = {
    let mut index = 0;
    while index < table3::CELLS.len() {
        let cell = &table3::CELLS[index];
        assert!(axis_ok(cell.before) && axis_ok(cell.after));
        assert!(!ranged_repeats(table3::CELLS, index));
        index = index.saturating_add(1);
    }
};
const _: () = {
    let mut index = 0;
    while index < table4::CELLS.len() {
        let cell = &table4::CELLS[index];
        assert!(axis_ok(cell.before) && axis_ok(cell.after));
        assert!(!ranged_repeats(table4::CELLS, index));
        index = index.saturating_add(1);
    }
};
const _: () = {
    let mut index = 0;
    while index < table5::CELLS.len() {
        let cell = &table5::CELLS[index];
        assert!(axis_ok(cell.before) && axis_ok(cell.after));
        assert!(!ranged_repeats(table5::CELLS, index));
        index = index.saturating_add(1);
    }
};
const _: () = {
    let mut index = 0;
    while index < table6::CELLS.len() {
        let cell = &table6::CELLS[index];
        assert!(
            cell.before != 0 && cell.after != 0 && axis_ok(cell.before) && axis_ok(cell.after),
            "Table 6 carries no line-edge axis and no math class"
        );
        assert!(!ranged_repeats(table6::CELLS, index));
        index = index.saturating_add(1);
    }
};
