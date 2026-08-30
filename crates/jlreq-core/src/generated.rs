// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Machine-generated specification tables owned privately by `jlreq`.
//!
//! Every Rust file inside `src/generated/` is emitted from the vendored snapshot by
//! `cargo run -p xtask -- generate`. This hand-written module is intentionally adjacent
//! to that directory so the generation gate can reject unclaimed files inside it.

pub(crate) mod appendix_a;
pub(crate) mod folding;
pub(crate) mod ideograph;
pub(crate) mod script;
pub(crate) mod table1;
pub(crate) mod table2;
pub(crate) mod table3;
pub(crate) mod table4;
pub(crate) mod table5;
pub(crate) mod table6;

const LISTING_COUNT: usize = 1_686;
const DISTINCT_KEY_COUNT: usize = 1_133;
const MULTI_CLASS_KEY_COUNT: usize = 473;
const REMARK_COUNT: usize = 14;
const IDEOGRAPH_RANGE_COUNT: usize = 16;
const IDEOGRAPH_COUNT: u32 = 101_996;
const FOLD_COUNT: usize = 226;
const SCRIPT_RANGE_COUNT: usize = 22;
const CLASS_COUNT: u8 = 30;
const TABLE_WITH_LINE_EDGE_COUNT: usize = 841;
const TABLE_WITHOUT_LINE_EDGE_COUNT: usize = 784;

const ALL_FRAMES: u8 = appendix_a::FRAME_FULL_EM
    | appendix_a::FRAME_HALF_EM
    | appendix_a::FRAME_THIRD_EM
    | appendix_a::FRAME_QUARTER_EM
    | appendix_a::FRAME_PROPORTIONAL;

const fn same_key(before: &appendix_a::Listing, after: &appendix_a::Listing) -> bool {
    let mut position = 0;
    while position < appendix_a::MAX_KEY_LEN {
        if before.key[position] != after.key[position] {
            return false;
        }
        position = position.saturating_add(1);
    }
    true
}

const fn ascends(before: &appendix_a::Listing, after: &appendix_a::Listing) -> bool {
    let mut position = 0;
    while position < appendix_a::MAX_KEY_LEN {
        if before.key[position] != after.key[position] {
            return before.key[position] < after.key[position];
        }
        position = position.saturating_add(1);
    }
    before.class < after.class
}

const fn distinct_keys(listings: &[appendix_a::Listing]) -> usize {
    let mut distinct = 0_usize;
    let mut index = 0_usize;
    while index < listings.len() {
        if index == 0 || !same_key(&listings[index.saturating_sub(1)], &listings[index]) {
            distinct = distinct.saturating_add(1);
        }
        index = index.saturating_add(1);
    }
    distinct
}

const fn multi_class_keys(listings: &[appendix_a::Listing]) -> usize {
    let mut shared = 0_usize;
    let mut index = 0_usize;
    while index < listings.len() {
        let begins = index == 0 || !same_key(&listings[index.saturating_sub(1)], &listings[index]);
        let next = index.saturating_add(1);
        let continues = next < listings.len() && same_key(&listings[index], &listings[next]);
        if begins && continues {
            shared = shared.saturating_add(1);
        }
        index = next;
    }
    shared
}

const fn covered_ideographs(ranges: &[ideograph::Range]) -> u32 {
    let mut total = 0_u32;
    let mut index = 0_usize;
    while index < ranges.len() {
        let range = &ranges[index];
        total = total.saturating_add(range.last.saturating_sub(range.first).saturating_add(1));
        index = index.saturating_add(1);
    }
    total
}

const fn listings_valid(listings: &[appendix_a::Listing]) -> bool {
    let mut index = 0_usize;
    while index < listings.len() {
        let listing = &listings[index];
        if listing.class < 1 || listing.class > CLASS_COUNT {
            return false;
        }
        if listing.remark as usize >= REMARK_COUNT {
            return false;
        }
        if listing.key_len < 1 || listing.key_len as usize > appendix_a::MAX_KEY_LEN {
            return false;
        }
        if listing.key[0] == 0 {
            return false;
        }
        if (listing.key_len == 1) != (listing.key[1] == 0) {
            return false;
        }
        if index > 0 && !ascends(&listings[index.saturating_sub(1)], listing) {
            return false;
        }
        index = index.saturating_add(1);
    }
    true
}

const fn ranged_cells_valid(tables: &[&[crate::spec::RawRangedCell]]) -> bool {
    let mut table = 0_usize;
    while table < tables.len() {
        let cells = tables[table];
        let mut index = 0_usize;
        while index < cells.len() {
            let cell = &cells[index];
            if let Some(limit) = cell.limit {
                if limit < 0 || limit > 720 {
                    return false;
                }
            }
            if cell.stage > 6 {
                return false;
            }
            let _ = cell.two_valued;
            let _ = cell.residual;
            let _ = cell.rule;
            index = index.saturating_add(1);
        }
        table = table.saturating_add(1);
    }
    true
}

const fn break_cells_valid(cells: &[crate::spec::RawBreakCell]) -> bool {
    let mut index = 0_usize;
    while index < cells.len() {
        let cell = &cells[index];
        if cell.levels > 0b1111 {
            return false;
        }
        let _ = cell.prohibited;
        let _ = cell.rule;
        index = index.saturating_add(1);
    }
    true
}

const fn spacing_cells_valid(cells: &[crate::spec::RawSpacingCell]) -> bool {
    let mut index = 0_usize;
    while index < cells.len() {
        let cell = &cells[index];
        if cell.terms.len() > 2 {
            return false;
        }
        let _ = cell.prohibited;
        let _ = cell.rule;
        match cell.hang {
            crate::spec::RawHang::None
            | crate::spec::RawHang::OverSpace
            | crate::spec::RawHang::OverCharacter => {},
        }
        let mut term = 0_usize;
        while term < cell.terms.len() {
            let _ = cell.terms[term].trailing;
            if cell.terms[term].amount < 0 || cell.terms[term].amount > 720 {
                return false;
            }
            term = term.saturating_add(1);
        }
        index = index.saturating_add(1);
    }
    true
}

const fn remarks_valid(remarks: &[appendix_a::Remark]) -> bool {
    let mut index = 0_usize;
    while index < remarks.len() {
        let remark = &remarks[index];
        if (remark.frames & !ALL_FRAMES) != 0 {
            return false;
        }
        if remark.usage > appendix_a::USAGE_VERTICAL_ONLY {
            return false;
        }
        if remark.role > appendix_a::ROLE_DIGIT_GROUP_SEPARATOR {
            return false;
        }
        if index == 0 && (!remark.en.is_empty() || !remark.ja.is_empty()) {
            return false;
        }
        if index > 0 && remark.ja.is_empty() {
            return false;
        }
        index = index.saturating_add(1);
    }
    true
}

const fn ideograph_ranges_valid(ranges: &[ideograph::Range]) -> bool {
    let mut index = 0_usize;
    while index < ranges.len() {
        let range = &ranges[index];
        if range.first > range.last {
            return false;
        }
        if index > 0 && ranges[index.saturating_sub(1)].last >= range.first {
            return false;
        }
        index = index.saturating_add(1);
    }
    true
}

const fn folds_valid(folds: &[folding::Fold]) -> bool {
    let mut index = 0_usize;
    while index < folds.len() {
        let fold = &folds[index];
        if fold.source == fold.target {
            return false;
        }
        if fold.frame != appendix_a::FRAME_FULL_EM && fold.frame != appendix_a::FRAME_HALF_EM {
            return false;
        }
        if index > 0 && folds[index.saturating_sub(1)].source >= fold.source {
            return false;
        }
        index = index.saturating_add(1);
    }
    true
}

const fn script_ranges_valid(ranges: &[script::Range]) -> bool {
    let mut index = 0_usize;
    while index < ranges.len() {
        let range = &ranges[index];
        if range.first > range.last {
            return false;
        }
        if range.script != script::HIRAGANA && range.script != script::KATAKANA {
            return false;
        }
        if index > 0 && ranges[index.saturating_sub(1)].last >= range.first {
            return false;
        }
        index = index.saturating_add(1);
    }
    true
}

const _: () = assert!(appendix_a::MAX_KEY_LEN == 2);
const _: () = assert!(appendix_a::LISTINGS.len() == LISTING_COUNT);
const _: () = assert!(distinct_keys(appendix_a::LISTINGS) == DISTINCT_KEY_COUNT);
const _: () = assert!(multi_class_keys(appendix_a::LISTINGS) == MULTI_CLASS_KEY_COUNT);
const _: () = assert!(appendix_a::REMARKS.len() == REMARK_COUNT);
const _: () = assert!(appendix_a::FRAMES_UNSTATED == 0);
const _: () = assert!(appendix_a::USAGE_UNQUALIFIED == 0);
const _: () = assert!(appendix_a::USAGE_HORIZONTAL_ONLY == 1);
const _: () = assert!(appendix_a::USAGE_VERTICAL_ONLY == 2);
const _: () = assert!(appendix_a::ROLE_UNSTATED == 0);
const _: () = assert!(appendix_a::ROLE_DECIMAL_POINT == 1);
const _: () = assert!(appendix_a::ROLE_DIGIT_GROUP_SEPARATOR == 2);

const _: () = assert!(listings_valid(appendix_a::LISTINGS));

const _: () = assert!(table3::CELLS.len() == TABLE_WITH_LINE_EDGE_COUNT);
const _: () = assert!(table4::CELLS.len() == TABLE_WITH_LINE_EDGE_COUNT);
const _: () = assert!(table5::CELLS.len() == TABLE_WITH_LINE_EDGE_COUNT);
const _: () = assert!(table6::CELLS.len() == TABLE_WITHOUT_LINE_EDGE_COUNT);
const _: () = assert!(ranged_cells_valid(&[
    table3::CELLS,
    table4::CELLS,
    table5::CELLS,
    table6::CELLS,
]));

const _: () = assert!(table2::CELLS.len() == TABLE_WITHOUT_LINE_EDGE_COUNT);
const _: () = assert!(break_cells_valid(table2::CELLS));

const _: () = assert!(table1::CELLS.len() == TABLE_WITH_LINE_EDGE_COUNT);
const _: () = assert!(spacing_cells_valid(table1::CELLS));

const _: () = assert!(remarks_valid(appendix_a::REMARKS));

const _: () = assert!(ideograph::RANGES.len() == IDEOGRAPH_RANGE_COUNT);
const _: () = assert!(covered_ideographs(ideograph::RANGES) == IDEOGRAPH_COUNT);
const _: () = assert!(ideograph_ranges_valid(ideograph::RANGES));

const _: () = assert!(folding::FOLDS.len() == FOLD_COUNT);
const _: () = assert!(folds_valid(folding::FOLDS));

const _: () = assert!(script::RANGES.len() == SCRIPT_RANGE_COUNT);
const _: () = assert!(script_ranges_valid(script::RANGES));

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{RawBreakCell, RawHang, RawRangedCell, RawSpacingCell, RawTerm};

    static VALID_TERMS: [RawTerm; 2] = [
        RawTerm {
            trailing: false,
            amount: 0,
        },
        RawTerm {
            trailing: true,
            amount: 720,
        },
    ];
    static TOO_MANY_TERMS: [RawTerm; 3] = [
        RawTerm {
            trailing: false,
            amount: 0,
        },
        RawTerm {
            trailing: false,
            amount: 1,
        },
        RawTerm {
            trailing: true,
            amount: 2,
        },
    ];
    static NEGATIVE_TERM: [RawTerm; 1] = [RawTerm {
        trailing: false,
        amount: -1,
    }];
    static OVERSIZED_TERM: [RawTerm; 1] = [RawTerm {
        trailing: true,
        amount: 721,
    }];

    const fn listing(
        first: u32,
        second: u32,
        key_len: u8,
        class: u8,
        remark: u8,
    ) -> appendix_a::Listing {
        appendix_a::Listing {
            key: [first, second],
            key_len,
            class,
            remark,
        }
    }

    fn valid_ranged_cell() -> RawRangedCell {
        RawRangedCell {
            limit: Some(720),
            two_valued: true,
            residual: true,
            stage: 6,
            rule: "test",
        }
    }

    fn assert_ranged_cell_rejected(cell: RawRangedCell) {
        assert!(!ranged_cells_valid(&[&[], &[cell]]));
    }

    fn valid_break_cell() -> RawBreakCell {
        RawBreakCell {
            prohibited: false,
            levels: 0b1111,
            rule: "test",
        }
    }

    fn assert_break_cell_rejected(cell: RawBreakCell) {
        assert!(!break_cells_valid(&[cell]));
    }

    fn valid_spacing_cell(terms: &'static [RawTerm]) -> RawSpacingCell {
        RawSpacingCell {
            prohibited: false,
            hang: RawHang::None,
            rule: "test",
            terms,
        }
    }

    fn assert_spacing_cell_rejected(cell: RawSpacingCell) {
        assert!(!spacing_cells_valid(&[cell]));
    }

    fn empty_remark() -> appendix_a::Remark {
        appendix_a::Remark {
            en: "",
            ja: "",
            frames: ALL_FRAMES,
            usage: appendix_a::USAGE_VERTICAL_ONLY,
            role: appendix_a::ROLE_DIGIT_GROUP_SEPARATOR,
        }
    }

    fn described_remark() -> appendix_a::Remark {
        appendix_a::Remark {
            en: "English",
            ja: "日本語",
            frames: ALL_FRAMES,
            usage: appendix_a::USAGE_VERTICAL_ONLY,
            role: appendix_a::ROLE_DIGIT_GROUP_SEPARATOR,
        }
    }

    #[test]
    fn listing_helpers_cover_equality_ordering_and_counts() {
        let first = listing(1, 0, 1, 1, 0);
        let same_key_later_class = listing(1, 0, 1, 2, 0);
        let second_code_point = listing(1, 2, 2, 1, 0);
        let later_key = listing(2, 0, 1, 1, 0);

        assert!(same_key(&first, &same_key_later_class));
        assert!(!same_key(&first, &second_code_point));
        assert!(ascends(&first, &same_key_later_class));
        assert!(!ascends(&first, &first));
        assert!(ascends(&first, &second_code_point));
        assert!(!ascends(&second_code_point, &first));
        assert!(ascends(&second_code_point, &later_key));
        assert!(!ascends(&later_key, &second_code_point));

        let listings = [first, same_key_later_class, later_key];
        assert_eq!(distinct_keys(&listings), 2);
        assert_eq!(multi_class_keys(&listings), 1);
        assert!(listings_valid(&listings));
    }

    #[test]
    fn listing_validation_rejects_every_invalid_field_and_order() {
        assert!(listings_valid(&[listing(1, 2, 2, CLASS_COUNT, 13,)]));
        for invalid in [
            listing(1, 0, 1, 0, 0),
            listing(1, 0, 1, CLASS_COUNT.saturating_add(1), 0),
            listing(1, 0, 1, 1, 14),
            listing(1, 0, 0, 1, 0),
            listing(1, 2, 3, 1, 0),
            listing(0, 0, 1, 1, 0),
            listing(1, 2, 1, 1, 0),
            listing(1, 0, 2, 1, 0),
        ] {
            assert!(!listings_valid(&[invalid]));
        }
        assert!(!listings_valid(&[
            listing(1, 0, 1, 1, 0),
            listing(1, 0, 1, 1, 0),
        ]));
        assert!(!listings_valid(&[
            listing(2, 0, 1, 1, 0),
            listing(1, 0, 1, 1, 0),
        ]));
    }

    #[test]
    fn ranged_cell_validation_checks_all_boundaries() {
        let valid = valid_ranged_cell();
        assert!(ranged_cells_valid(&[&[], &[valid]]));
        for invalid in [
            RawRangedCell {
                limit: Some(-1),
                ..valid
            },
            RawRangedCell {
                limit: Some(721),
                ..valid
            },
            RawRangedCell { stage: 7, ..valid },
        ] {
            assert_ranged_cell_rejected(invalid);
        }
    }

    #[test]
    fn break_cell_validation_checks_all_boundaries() {
        let valid = valid_break_cell();
        assert!(break_cells_valid(&[valid]));
        assert_break_cell_rejected(RawBreakCell {
            levels: 0b1_0000,
            ..valid
        });
    }

    #[test]
    fn spacing_cell_validation_checks_all_boundaries() {
        let valid = valid_spacing_cell(&VALID_TERMS);
        assert!(spacing_cells_valid(&[valid]));
        for invalid in [
            valid_spacing_cell(&TOO_MANY_TERMS),
            valid_spacing_cell(&NEGATIVE_TERM),
            valid_spacing_cell(&OVERSIZED_TERM),
        ] {
            assert_spacing_cell_rejected(invalid);
        }
    }

    #[test]
    fn generated_matrix_accessors_cover_exactly_the_transcribed_axes() {
        for before in 0_u8..=31 {
            for after in 0_u8..=31 {
                let class =
                    |value: u8| (1..=CLASS_COUNT).contains(&value) && value != 17 && value != 18;
                let with_edge = (before == 0 || class(before)) && (after == 0 || class(after));
                let without_edge = class(before) && class(after);
                assert_eq!(table1::cell(before, after).is_some(), with_edge);
                assert_eq!(table2::cell(before, after).is_some(), without_edge);
                assert_eq!(table3::cell(before, after).is_some(), with_edge);
                assert_eq!(table4::cell(before, after).is_some(), with_edge);
                assert_eq!(table5::cell(before, after).is_some(), with_edge);
                assert_eq!(table6::cell(before, after).is_some(), without_edge);
            }
        }
    }

    #[test]
    fn remarks_validation_checks_masks_qualifiers_and_empty_cells() {
        assert!(remarks_valid(&[empty_remark(), described_remark()]));
        assert!(!remarks_valid(&[appendix_a::Remark {
            frames: 0b0010_0000,
            ..empty_remark()
        }]));
        assert!(!remarks_valid(&[appendix_a::Remark {
            usage: appendix_a::USAGE_VERTICAL_ONLY.saturating_add(1),
            ..empty_remark()
        }]));
        assert!(!remarks_valid(&[appendix_a::Remark {
            role: appendix_a::ROLE_DIGIT_GROUP_SEPARATOR.saturating_add(1),
            ..empty_remark()
        }]));
        assert!(!remarks_valid(&[appendix_a::Remark {
            en: "not empty",
            ..empty_remark()
        }]));
        assert!(!remarks_valid(&[
            empty_remark(),
            appendix_a::Remark {
                en: "English",
                ja: "",
                frames: 0,
                usage: 0,
                role: 0,
            },
        ]));
    }

    #[test]
    fn range_and_fold_validation_rejects_reversal_overlap_and_bad_tags() {
        let ideographs = [
            ideograph::Range { first: 1, last: 2 },
            ideograph::Range { first: 4, last: 4 },
        ];
        assert_eq!(covered_ideographs(&ideographs), 3);
        assert!(ideograph_ranges_valid(&ideographs));
        assert!(!ideograph_ranges_valid(&[ideograph::Range {
            first: 2,
            last: 1,
        }]));
        assert!(!ideograph_ranges_valid(&[
            ideograph::Range { first: 1, last: 2 },
            ideograph::Range { first: 2, last: 3 },
        ]));

        let folds = [
            folding::Fold {
                source: 2,
                target: 1,
                frame: appendix_a::FRAME_FULL_EM,
            },
            folding::Fold {
                source: 4,
                target: 3,
                frame: appendix_a::FRAME_HALF_EM,
            },
        ];
        assert!(folds_valid(&folds));
        assert!(!folds_valid(&[folding::Fold {
            source: 1,
            target: 1,
            frame: appendix_a::FRAME_FULL_EM,
        }]));
        assert!(!folds_valid(&[folding::Fold {
            source: 2,
            target: 1,
            frame: 0,
        }]));
        assert!(!folds_valid(&[
            folding::Fold {
                source: 2,
                target: 1,
                frame: appendix_a::FRAME_FULL_EM,
            },
            folding::Fold {
                source: 2,
                target: 0,
                frame: appendix_a::FRAME_FULL_EM,
            },
        ]));

        let scripts = [
            script::Range {
                first: 1,
                last: 2,
                script: script::HIRAGANA,
            },
            script::Range {
                first: 4,
                last: 5,
                script: script::KATAKANA,
            },
        ];
        assert!(script_ranges_valid(&scripts));
        assert!(!script_ranges_valid(&[script::Range {
            first: 2,
            last: 1,
            script: script::HIRAGANA,
        }]));
        assert!(!script_ranges_valid(&[script::Range {
            first: 1,
            last: 2,
            script: 0,
        }]));
        assert!(!script_ranges_valid(&[
            script::Range {
                first: 1,
                last: 2,
                script: script::HIRAGANA,
            },
            script::Range {
                first: 2,
                last: 3,
                script: script::KATAKANA,
            },
        ]));
    }
}
