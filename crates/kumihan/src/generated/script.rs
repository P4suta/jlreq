// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The two kana scripts §C.2 note 3's small-kana fallback reads.
//!
//! Do not edit. `cargo run -p xtask -- generate` writes this file, and
//! `generate --check` fails when regenerating it would change a byte. A hand
//! edit is a bug even when it is correct, because the next revision of the
//! specification will not carry it forward (ADR 0009).
//!
//! - Source: `spec/derived/scripts.tsv`
//! - Source SHA-256: `d4864a39d1fe165269aa18608f5d3ed74fa21c8510d462afd6f86e9bba57d00e`
//! - Specification: JLReq, 2020-08-11
//! - Generator: `xtask/src/classes.rs`, `xtask/src/generate.rs`
//! - Generator SHA-256: `4b9ee91056000031fce83c7745f05ec8b25f69e28a048a1e84d51548f66ab331`
//! - Entries: 22

/// The `Script=Hiragana` tag.
///
/// JLReq: §C.2#3
pub(crate) const HIRAGANA: u8 = 1;

/// The `Script=Katakana` tag.
///
/// JLReq: §C.2#3
pub(crate) const KATAKANA: u8 = 2;

/// One range of code points the Unicode Character Database gives one kana script.
///
/// JLReq: §C.2#3
#[derive(Debug)]
pub(crate) struct Range {
    /// The first code point of the range.
    pub(crate) first: u32,
    /// The last code point of the range, inclusive.
    pub(crate) last: u32,
    /// Which script, `HIRAGANA` or `KATAKANA`.
    pub(crate) script: u8,
}

impl Range {
    /// One row of the table below.
    const fn new(first: u32, last: u32, script: u8) -> Self {
        Self {
            first,
            last,
            script,
        }
    }
}

/// Every such range, sorted by first code point and disjoint.
///
/// §C.2 note 3 permits a small kana to be treated as the full-size one at a line
/// head, and the fallback needs to know a kana when it sees one beyond the forty
/// §A.11 enumerates.
///
/// JLReq: §C.2#3
pub(crate) const RANGES: &[Range] = &[
    Range::new(0x0000_3041, 0x0000_3096, HIRAGANA),
    Range::new(0x0000_309D, 0x0000_309E, HIRAGANA),
    Range::new(0x0000_309F, 0x0000_309F, HIRAGANA),
    Range::new(0x0000_30A1, 0x0000_30FA, KATAKANA),
    Range::new(0x0000_30FD, 0x0000_30FE, KATAKANA),
    Range::new(0x0000_30FF, 0x0000_30FF, KATAKANA),
    Range::new(0x0000_31F0, 0x0000_31FF, KATAKANA),
    Range::new(0x0000_32D0, 0x0000_32FE, KATAKANA),
    Range::new(0x0000_3300, 0x0000_3357, KATAKANA),
    Range::new(0x0000_FF66, 0x0000_FF6F, KATAKANA),
    Range::new(0x0000_FF71, 0x0000_FF9D, KATAKANA),
    Range::new(0x0001_AFF0, 0x0001_AFF3, KATAKANA),
    Range::new(0x0001_AFF5, 0x0001_AFFB, KATAKANA),
    Range::new(0x0001_AFFD, 0x0001_AFFE, KATAKANA),
    Range::new(0x0001_B000, 0x0001_B000, KATAKANA),
    Range::new(0x0001_B001, 0x0001_B11F, HIRAGANA),
    Range::new(0x0001_B120, 0x0001_B122, KATAKANA),
    Range::new(0x0001_B132, 0x0001_B132, HIRAGANA),
    Range::new(0x0001_B150, 0x0001_B152, HIRAGANA),
    Range::new(0x0001_B155, 0x0001_B155, KATAKANA),
    Range::new(0x0001_B164, 0x0001_B167, KATAKANA),
    Range::new(0x0001_F200, 0x0001_F200, HIRAGANA),
];
