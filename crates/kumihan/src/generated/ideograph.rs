// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The members of cl-19 that §A.19's table deliberately does not list.
//!
//! Do not edit. `cargo run -p xtask -- generate` writes this file, and
//! `generate --check` fails when regenerating it would change a byte. A hand
//! edit is a bug even when it is correct, because the next revision of the
//! specification will not carry it forward (ADR 0009).
//!
//! - Source: `spec/derived/ideographs.tsv`
//! - Source SHA-256: `8e9f16b8cd0afc0081a0912131a83232a33dea0e067d9174aeb30a0b08589ab3`
//! - Specification: JLReq, 2020-08-11
//! - Generator: `xtask/src/classes.rs`, `xtask/src/generate.rs`
//! - Generator SHA-256: `4b9ee91056000031fce83c7745f05ec8b25f69e28a048a1e84d51548f66ab331`
//! - Entries: 16

/// One range of code points the Unicode Character Database gives
/// `Unified_Ideograph=Yes`.
///
/// JLReq: §A.19
#[derive(Debug)]
pub(crate) struct Range {
    /// The first code point of the range.
    pub(crate) first: u32,
    /// The last code point of the range, inclusive.
    pub(crate) last: u32,
}

impl Range {
    /// One row of the table below.
    const fn new(first: u32, last: u32) -> Self {
        Self { first, last }
    }
}

/// Every such range, sorted and disjoint.
///
/// §A.19's table lists only the *non-ideographic* members of cl-19, so the
/// ideographs come from here. `Unified_Ideograph` is the property and the
/// alternatives are both wrong: `Ideographic` over-covers with Tangut, Nushu and
/// Khitan, and `Script=Han` over-covers with `U+3005`, which JLReq puts in cl-09.
///
/// JLReq: §A.19
pub(crate) const RANGES: &[Range] = &[
    Range::new(0x0000_3400, 0x0000_4DBF),
    Range::new(0x0000_4E00, 0x0000_9FFF),
    Range::new(0x0000_FA0E, 0x0000_FA0F),
    Range::new(0x0000_FA11, 0x0000_FA11),
    Range::new(0x0000_FA13, 0x0000_FA14),
    Range::new(0x0000_FA1F, 0x0000_FA1F),
    Range::new(0x0000_FA21, 0x0000_FA21),
    Range::new(0x0000_FA23, 0x0000_FA24),
    Range::new(0x0000_FA27, 0x0000_FA29),
    Range::new(0x0002_0000, 0x0002_A6DF),
    Range::new(0x0002_A700, 0x0002_B81D),
    Range::new(0x0002_B820, 0x0002_CEAD),
    Range::new(0x0002_CEB0, 0x0002_EBE0),
    Range::new(0x0002_EBF0, 0x0002_EE5D),
    Range::new(0x0003_0000, 0x0003_134A),
    Range::new(0x0003_1350, 0x0003_3479),
];
