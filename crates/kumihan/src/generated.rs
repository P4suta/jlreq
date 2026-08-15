// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Machine-generated specification tables owned privately by `kumihan`.
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

const fn distinct_keys() -> usize {
    let mut distinct = 0_usize;
    let mut index = 0_usize;
    while index < appendix_a::LISTINGS.len() {
        if index == 0
            || !same_key(
                &appendix_a::LISTINGS[index.saturating_sub(1)],
                &appendix_a::LISTINGS[index],
            )
        {
            distinct = distinct.saturating_add(1);
        }
        index = index.saturating_add(1);
    }
    distinct
}

const fn multi_class_keys() -> usize {
    let mut shared = 0_usize;
    let mut index = 0_usize;
    while index < appendix_a::LISTINGS.len() {
        let begins = index == 0
            || !same_key(
                &appendix_a::LISTINGS[index.saturating_sub(1)],
                &appendix_a::LISTINGS[index],
            );
        let next = index.saturating_add(1);
        let continues = next < appendix_a::LISTINGS.len()
            && same_key(&appendix_a::LISTINGS[index], &appendix_a::LISTINGS[next]);
        if begins && continues {
            shared = shared.saturating_add(1);
        }
        index = next;
    }
    shared
}

const fn covered_ideographs() -> u32 {
    let mut total = 0_u32;
    let mut index = 0_usize;
    while index < ideograph::RANGES.len() {
        let range = &ideograph::RANGES[index];
        total = total.saturating_add(range.last.saturating_sub(range.first).saturating_add(1));
        index = index.saturating_add(1);
    }
    total
}

const _: () = assert!(appendix_a::MAX_KEY_LEN == 2);
const _: () = assert!(appendix_a::LISTINGS.len() == LISTING_COUNT);
const _: () = assert!(distinct_keys() == DISTINCT_KEY_COUNT);
const _: () = assert!(multi_class_keys() == MULTI_CLASS_KEY_COUNT);
const _: () = assert!(appendix_a::REMARKS.len() == REMARK_COUNT);
const _: () = assert!(appendix_a::FRAMES_UNSTATED == 0);
const _: () = assert!(appendix_a::USAGE_UNQUALIFIED == 0);
const _: () = assert!(appendix_a::USAGE_HORIZONTAL_ONLY == 1);
const _: () = assert!(appendix_a::USAGE_VERTICAL_ONLY == 2);
const _: () = assert!(appendix_a::ROLE_UNSTATED == 0);
const _: () = assert!(appendix_a::ROLE_DECIMAL_POINT == 1);
const _: () = assert!(appendix_a::ROLE_DIGIT_GROUP_SEPARATOR == 2);

const _: () = {
    let mut index = 0_usize;
    while index < appendix_a::LISTINGS.len() {
        let listing = &appendix_a::LISTINGS[index];
        assert!(listing.class >= 1 && listing.class <= CLASS_COUNT);
        assert!(listing.remark < 14);
        assert!(listing.key_len >= 1 && listing.key_len <= 2);
        assert!(listing.key[0] != 0);
        assert!((listing.key_len == 1) == (listing.key[1] == 0));
        if index > 0 {
            assert!(ascends(
                &appendix_a::LISTINGS[index.saturating_sub(1)],
                listing
            ));
        }
        index = index.saturating_add(1);
    }
};

const _: () = assert!(table3::CELLS.len() == TABLE_WITH_LINE_EDGE_COUNT);
const _: () = assert!(table4::CELLS.len() == TABLE_WITH_LINE_EDGE_COUNT);
const _: () = assert!(table5::CELLS.len() == TABLE_WITH_LINE_EDGE_COUNT);
const _: () = assert!(table6::CELLS.len() == TABLE_WITHOUT_LINE_EDGE_COUNT);
const _: () = {
    let tables = [table3::CELLS, table4::CELLS, table5::CELLS, table6::CELLS];
    let mut table = 0_usize;
    while table < tables.len() {
        let cells = tables[table];
        let mut index = 0_usize;
        while index < cells.len() {
            let cell = &cells[index];
            assert!(cell.before <= CLASS_COUNT && cell.after <= CLASS_COUNT);
            assert!(cell.before != 17 && cell.before != 18);
            assert!(cell.after != 17 && cell.after != 18);
            if let Some(limit) = cell.limit {
                assert!(limit >= 0 && limit <= 720);
            }
            assert!(cell.stage <= 6);
            let _ = cell.two_valued;
            let _ = cell.residual;
            let _ = cell.rule;
            index = index.saturating_add(1);
        }
        table = table.saturating_add(1);
    }
};

const _: () = assert!(table2::CELLS.len() == TABLE_WITHOUT_LINE_EDGE_COUNT);
const _: () = {
    let mut index = 0_usize;
    while index < table2::CELLS.len() {
        let cell = &table2::CELLS[index];
        assert!(cell.before >= 1 && cell.before <= CLASS_COUNT);
        assert!(cell.after >= 1 && cell.after <= CLASS_COUNT);
        assert!(cell.before != 17 && cell.before != 18);
        assert!(cell.after != 17 && cell.after != 18);
        assert!(cell.levels <= 0b1111);
        let _ = cell.prohibited;
        let _ = cell.rule;
        index = index.saturating_add(1);
    }
};

const _: () = assert!(table1::CELLS.len() == TABLE_WITH_LINE_EDGE_COUNT);
const _: () = {
    let mut index = 0_usize;
    while index < table1::CELLS.len() {
        let cell = &table1::CELLS[index];
        assert!(cell.before <= CLASS_COUNT && cell.after <= CLASS_COUNT);
        assert!(cell.before != 17 && cell.before != 18);
        assert!(cell.after != 17 && cell.after != 18);
        assert!(cell.terms.len() <= 2);
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
            assert!(cell.terms[term].amount >= 0 && cell.terms[term].amount <= 720);
            term = term.saturating_add(1);
        }
        index = index.saturating_add(1);
    }
};

const _: () = {
    let mut index = 0_usize;
    while index < appendix_a::REMARKS.len() {
        let remark = &appendix_a::REMARKS[index];
        assert!((remark.frames & !ALL_FRAMES) == 0);
        assert!(remark.usage <= appendix_a::USAGE_VERTICAL_ONLY);
        assert!(remark.role <= appendix_a::ROLE_DIGIT_GROUP_SEPARATOR);
        assert!(index != 0 || (remark.en.is_empty() && remark.ja.is_empty()));
        assert!(index == 0 || !remark.ja.is_empty());
        index = index.saturating_add(1);
    }
};

const _: () = assert!(ideograph::RANGES.len() == IDEOGRAPH_RANGE_COUNT);
const _: () = assert!(covered_ideographs() == IDEOGRAPH_COUNT);
const _: () = {
    let mut index = 0_usize;
    while index < ideograph::RANGES.len() {
        let range = &ideograph::RANGES[index];
        assert!(range.first <= range.last);
        if index > 0 {
            assert!(ideograph::RANGES[index.saturating_sub(1)].last < range.first);
        }
        index = index.saturating_add(1);
    }
};

const _: () = assert!(folding::FOLDS.len() == FOLD_COUNT);
const _: () = {
    let mut index = 0_usize;
    while index < folding::FOLDS.len() {
        let fold = &folding::FOLDS[index];
        assert!(fold.source != fold.target);
        assert!(fold.frame == appendix_a::FRAME_FULL_EM || fold.frame == appendix_a::FRAME_HALF_EM);
        if index > 0 {
            assert!(folding::FOLDS[index.saturating_sub(1)].source < fold.source);
        }
        index = index.saturating_add(1);
    }
};

const _: () = assert!(script::RANGES.len() == SCRIPT_RANGE_COUNT);
const _: () = {
    let mut index = 0_usize;
    while index < script::RANGES.len() {
        let range = &script::RANGES[index];
        assert!(range.first <= range.last);
        assert!(range.script == script::HIRAGANA || range.script == script::KATAKANA);
        if index > 0 {
            assert!(script::RANGES[index.saturating_sub(1)].last < range.first);
        }
        index = index.saturating_add(1);
    }
};
