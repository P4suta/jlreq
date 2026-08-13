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

const LISTING_COUNT: usize = 1_686;
const DISTINCT_KEY_COUNT: usize = 1_133;
const MULTI_CLASS_KEY_COUNT: usize = 473;
const REMARK_COUNT: usize = 14;
const IDEOGRAPH_RANGE_COUNT: usize = 16;
const IDEOGRAPH_COUNT: u32 = 101_996;
const FOLD_COUNT: usize = 226;
const SCRIPT_RANGE_COUNT: usize = 22;
const CLASS_COUNT: u8 = 30;

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
