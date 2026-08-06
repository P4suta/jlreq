// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The generated specification tables, and the figures this crate is written against.
//!
//! Every Rust file inside `src/generated/` is machine-written: `cargo run -p xtask --
//! generate` emits it from `spec/derived/`, and `generate --check` fails when regenerating
//! it would change a byte. This file is the one hand-written module in the neighborhood,
//! which is why the module declarations live here beside the directory rather than in a
//! `src/generated/mod.rs` inside it — the `generate` gate reads that convention and
//! reports any file *in* the directory that no generation unit writes.
//!
//! # Why the assertions are here and not in the generated files
//!
//! A generated file asserting its own shape proves that the emitter is self-consistent and
//! nothing more: a specification revision would regenerate the data and the expectation
//! together, and the check would pass by construction. The figures below were measured
//! against the published document and are maintained by hand, so a revision of JLReq or of
//! the Unicode Character Database that moves a row, drops a key, or reorders a table is a
//! compile error naming the number that changed. That is the property
//! `docs/design/generation.md` asks for, and it holds only while these constants are
//! written where regeneration cannot touch them.
//!
//! The assertions are `const` blocks rather than tests for the same reason the emitter
//! writes tables rather than functions: a table that is wrong should not link. Every one of
//! them is evaluated whether or not anything reads the table.
//!
//! JLReq: §A, §A.19, §C.2#3

pub(crate) mod appendix_a;
pub(crate) mod class_name;
pub(crate) mod folding;
pub(crate) mod ideograph;
// The two kana scripts are read by the assertions below and by no classification path, and
// that is where the design puts them rather than an oversight: §C.2 note 3's small-kana
// fallback is a *reclassification*, and `classify::RECLASSIFICATIONS` is empty until the
// appendix note table and the policy space are generated, because a reclassification
// invented here would publish an alternative the specification does not permit (ADR 0009).
// The data is emitted at M0 because `docs/design/api-spine.md` places it here, and because
// the assertions over it are what make a revision that moves a kana range a build failure
// rather than a silent change to a rule nobody has written yet.
pub(crate) mod script;

use jlreq_spec::Address;

/// How many listings the published Appendix A yields.
///
/// The document holds 1687 rows and lists one of them twice — `U+216B` in cl-19, which is a
/// recorded defect — so the table holds one fewer.
const LISTINGS: usize = 1686;

/// How many distinct keys those listings name.
const KEYS: usize = 1133;

/// How many of those keys more than one class names.
///
/// Two in five. This is the measurement `docs/adr/0008` turns on: there is no total
/// function from a code point to a class, because Appendix A does not define one.
const MULTI_CLASS_KEYS: usize = 473;

/// The longest key Appendix A enumerates, in code points.
///
/// Twenty-five keys are ordered pairs, and cl-27 lists `<02E5, 02E9>` and `<02E9, 02E5>` as
/// two distinct members, so the key is a sequence and its order matters.
const MAX_KEY_LEN: usize = 2;

/// The same bound as a key length is written, so no cast is needed to check one.
const MAX_KEY_LEN_U8: u8 = 2;

/// How many distinct Remarks cells the published Appendix A writes.
const REMARKS: usize = 14;

/// The same bound as a remark ordinal is written.
const REMARKS_U8: u8 = 14;

/// How many classes §3.9.2 closes the set at.
const CLASS_COUNT: u8 = 30;

/// How many of those thirty classes Appendix A enumerates members for.
///
/// Twenty-five. The other five — cl-20 through cl-23 and cl-30 — have a heading and no
/// table, because their section text reads in full "Any character may participate in …":
/// membership is a property of the construct the occurrence sits in and not of the code
/// point, which is the sharpest instance of what `docs/adr/0008` decided.
const ENUMERATING_CLASSES: usize = 25;

/// How many of the thirty class names Appendix A enumerates a section for.
const fn enumerating_classes() -> usize {
    let mut found: usize = 0;
    let mut index: usize = 0;
    while index < class_name::CLASSES.len() {
        if !class_name::CLASSES[index].enumeration.is_empty() {
            found = found.saturating_add(1);
        }
        index = index.saturating_add(1);
    }
    found
}

/// How many ranges the vendored `PropList.txt` gives `Unified_Ideograph`.
const IDEOGRAPH_RANGES: usize = 16;

/// How many code points those ranges cover.
const IDEOGRAPHS: u32 = 101_996;

/// How many Wide and Narrow decompositions the vendored `UnicodeData.txt` holds.
const FOLDS: usize = 226;

/// How many ranges the vendored `Scripts.txt` gives the two kana scripts together.
const SCRIPT_RANGES: usize = 22;

/// Every frame bit, as one mask.
///
/// Written as the union of the generated names rather than as a number, so a frame added to
/// the vocabulary widens this without anyone remembering to.
const ALL_FRAMES: u8 = appendix_a::FRAME_FULL_EM
    | appendix_a::FRAME_HALF_EM
    | appendix_a::FRAME_THIRD_EM
    | appendix_a::FRAME_QUARTER_EM
    | appendix_a::FRAME_PROPORTIONAL;

/// Whether one listing sorts strictly before another: by key, then by class.
///
/// Strictly, so that the same check proves the table is ordered for a binary search and
/// that it holds no row twice.
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

/// Whether two listings name the same key.
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

/// How many distinct keys the table names, counted from the table itself.
const fn distinct_keys() -> usize {
    let mut distinct: usize = 0;
    let mut index = 0;
    while index < appendix_a::LISTINGS.len() {
        let previous = index.saturating_sub(1);
        if index == 0
            || !same_key(
                &appendix_a::LISTINGS[previous],
                &appendix_a::LISTINGS[index],
            )
        {
            distinct = distinct.saturating_add(1);
        }
        index = index.saturating_add(1);
    }
    distinct
}

/// How many of those keys begin a run of more than one listing.
const fn multi_class_keys() -> usize {
    let mut shared: usize = 0;
    let mut index = 0;
    while index < appendix_a::LISTINGS.len() {
        let previous = index.saturating_sub(1);
        let next = index.saturating_add(1);
        let begins = index == 0
            || !same_key(
                &appendix_a::LISTINGS[previous],
                &appendix_a::LISTINGS[index],
            );
        let continues = next < appendix_a::LISTINGS.len()
            && same_key(&appendix_a::LISTINGS[index], &appendix_a::LISTINGS[next]);
        if begins && continues {
            shared = shared.saturating_add(1);
        }
        index = index.saturating_add(1);
    }
    shared
}

/// How many code points the ideograph ranges cover.
const fn covered_ideographs() -> u32 {
    let mut total: u32 = 0;
    let mut index = 0;
    while index < ideograph::RANGES.len() {
        let range = &ideograph::RANGES[index];
        total = total.saturating_add(range.last.saturating_sub(range.first).saturating_add(1));
        index = index.saturating_add(1);
    }
    total
}

// The three totals of Appendix A. A revision that adds a member, drops one, or moves one
// between classes changes at least one of them.
const _: () = assert!(
    appendix_a::LISTINGS.len() == LISTINGS,
    "Appendix A no longer holds the number of listings this crate was written against"
);
const _: () = assert!(
    distinct_keys() == KEYS,
    "Appendix A no longer enumerates the number of keys this crate was written against"
);
const _: () = assert!(
    multi_class_keys() == MULTI_CLASS_KEYS,
    "the number of keys named by more than one class has changed, and it is the measurement \
     the whole classification design rests on"
);
const _: () = assert!(
    appendix_a::MAX_KEY_LEN == MAX_KEY_LEN,
    "a key is one or two code points; a longer one needs a wider lookup, not a truncation"
);
const _: () = assert!(appendix_a::REMARKS.len() == REMARKS);
// A length is a `usize` and a key ordinal is a `u8`, so each bound is written twice. There
// is deliberately no cast between them: the two spellings are held equal here instead,
// where a reader can see both numbers at once.
const _: () = assert!(MAX_KEY_LEN == 2 && MAX_KEY_LEN_U8 == 2);
const _: () = assert!(REMARKS == 14 && REMARKS_U8 == 14);

// Every listing is well formed, and the table is sorted strictly, which is both what a
// binary search needs and what says the one repeated row was removed exactly once.
const _: () = {
    let mut index = 0;
    while index < appendix_a::LISTINGS.len() {
        let listing = &appendix_a::LISTINGS[index];
        assert!(
            listing.class >= 1 && listing.class <= CLASS_COUNT,
            "a listing names a class outside cl-01 through cl-30"
        );
        assert!(
            listing.remark < REMARKS_U8,
            "a listing names a Remarks cell the table does not hold"
        );
        assert!(
            listing.key_len >= 1 && listing.key_len <= MAX_KEY_LEN_U8,
            "a key holds one or two code points"
        );
        assert!(listing.key[0] != 0, "a key does not begin with U+0000");
        assert!(
            (listing.key_len == 1) == (listing.key[1] == 0),
            "a key of one code point is padded with zero and a key of two is not"
        );
        if index > 0 {
            assert!(
                ascends(
                    &appendix_a::LISTINGS[index.saturating_sub(1)],
                    &appendix_a::LISTINGS[index]
                ),
                "the table is sorted by key and then by class, and holds no listing twice"
            );
        }
        index = index.saturating_add(1);
    }
};

// Every Remarks cell states something, in Japanese at least. Three cells carry no English
// at all — a recorded defect of the published document — and none carries no Japanese, so
// an extraction that read only one locale would have lost three qualifications.
const _: () = {
    let mut index = 0;
    while index < appendix_a::REMARKS.len() {
        let remark = &appendix_a::REMARKS[index];
        assert!(
            (remark.frames & !ALL_FRAMES) == 0,
            "a Remarks cell names a frame that is not in the vocabulary"
        );
        assert!(
            remark.usage <= appendix_a::USAGE_VERTICAL_ONLY,
            "a Remarks cell names a writing-direction qualification that is not in the \
             vocabulary"
        );
        assert!(
            remark.role <= appendix_a::ROLE_DIGIT_GROUP_SEPARATOR,
            "a Remarks cell names a role that is not in the vocabulary"
        );
        assert!(
            index != 0 || (remark.en.is_empty() && remark.ja.is_empty()),
            "the first Remarks cell is the empty one, which states nothing"
        );
        assert!(
            index == 0 || !remark.ja.is_empty(),
            "every Remarks cell that states anything states it in Japanese"
        );
        index = index.saturating_add(1);
    }
};

// The ideograph predicate. The count is asserted twice over — the number of ranges and the
// number of code points they cover — because a Unicode revision that merges two adjacent
// ranges changes only the first and one that extends a block changes only the second.
const _: () = assert!(
    ideograph::RANGES.len() == IDEOGRAPH_RANGES,
    "the Unicode revision this crate was written against gives Unified_Ideograph a \
     different number of ranges"
);
const _: () = assert!(
    covered_ideographs() == IDEOGRAPHS,
    "Unified_Ideograph covers a different number of code points than this crate was \
     written against"
);
const _: () = {
    let mut index = 0;
    while index < ideograph::RANGES.len() {
        let range = &ideograph::RANGES[index];
        assert!(range.first <= range.last, "a range ends before it begins");
        if index > 0 {
            assert!(
                ideograph::RANGES[index.saturating_sub(1)].last < range.first,
                "the ranges are sorted and disjoint, which is what a binary search needs"
            );
        }
        index = index.saturating_add(1);
    }
};

// The compatibility folding: sorted by source, one target each, and only the two frames a
// compatibility code point can assert.
const _: () = assert!(folding::FOLDS.len() == FOLDS);
const _: () = {
    let mut index = 0;
    while index < folding::FOLDS.len() {
        let fold = &folding::FOLDS[index];
        assert!(
            fold.source != fold.target,
            "a code point that folds onto itself is not a folding"
        );
        assert!(
            fold.frame == appendix_a::FRAME_FULL_EM || fold.frame == appendix_a::FRAME_HALF_EM,
            "only a full-width and a half-width form fold; full compatibility folding would \
             fold U+2160, a genuine cl-19 member, onto the letter I"
        );
        if index > 0 {
            assert!(
                folding::FOLDS[index.saturating_sub(1)].source < fold.source,
                "the folds are sorted by source and no source folds two ways"
            );
        }
        index = index.saturating_add(1);
    }
};

// The class vocabulary of §3.9.2: thirty classes, each named in both locales, each either
// enumerated by an Appendix A section whose address parses or enumerating nothing at all.
const _: () = assert!(
    class_name::CLASSES.len() == CLASS_COUNT as usize,
    "§3.9.2 no longer closes the set at the number of classes this crate was written against"
);
const _: () = assert!(
    enumerating_classes() == ENUMERATING_CLASSES,
    "the number of classes Appendix A enumerates members for has changed, and the five that \
     enumerate nothing are what `docs/adr/0008` turns on"
);
const _: () = {
    let mut index = 0;
    while index < class_name::CLASSES.len() {
        let named = &class_name::CLASSES[index];
        assert!(
            !named.en.is_empty() && !named.ja.is_empty(),
            "§3.9.2 publishes every class name in both locales"
        );
        assert!(
            named.id.len() == 5,
            "a class id is `cl-` and two digits, which is what every rule sentence writes"
        );
        assert!(
            named.enumeration.is_empty() || Address::parse(named.enumeration).is_some(),
            "a class states the Appendix A section enumerating it as an address the grammar \
             accepts, or states none at all"
        );
        index = index.saturating_add(1);
    }
};

// The two kana scripts behind §C.2 note 3's fallback.
const _: () = assert!(script::RANGES.len() == SCRIPT_RANGES);
const _: () = {
    let mut index = 0;
    while index < script::RANGES.len() {
        let range = &script::RANGES[index];
        assert!(range.first <= range.last, "a range ends before it begins");
        assert!(
            range.script == script::HIRAGANA || range.script == script::KATAKANA,
            "§C.2 note 3 reads two scripts and no others"
        );
        if index > 0 {
            assert!(
                script::RANGES[index.saturating_sub(1)].last < range.first,
                "the ranges are sorted and disjoint, so no code point has two scripts"
            );
        }
        index = index.saturating_add(1);
    }
};
