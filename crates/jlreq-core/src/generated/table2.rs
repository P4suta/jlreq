// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Table 2, "Possibilities for Line-breaking between Characters" (Appendix C).
//!
//! Do not edit. `cargo run -p xtask -- generate` writes this file, and
//! `generate --check` fails when regenerating it would change a byte. A hand
//! edit is a bug even when it is correct, because the next revision of the
//! specification will not carry it forward (ADR 0009).
//!
//! - Source: `spec/captured/table2.en.tsv`
//! - Source SHA-256: `3e93d1104a5c730bc9eca01880ef989520b7c3ebb1ef98833be0424a44edbd66`
//! - Specification: JLReq, 2020-08-11
//! - Generator: `xtask/src/generate.rs`, `xtask/src/spacing.rs`
//! - Generator SHA-256: `02c2e2c3acf5e6532ae3311484d2a3913a8224c7f50075782607830573080ddc`
//! - Entries: 784

use crate::spec::RawBreakCell;

/// Table 2's cells, in the order the transcription was read.
///
/// JLReq: §C.1
pub(crate) static CELLS: &[RawBreakCell] = &[
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: true,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C.2#4",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C.2#5",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: true,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C.2#6",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C.2#7",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: true,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C.2#8",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: true,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C.2#9",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C.2#10",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C.2#11",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C.2#12",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: true,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: true,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: true,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: true,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: true,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: true,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: true,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: true,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: true,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: true,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: true,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: true,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: true,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: true,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: true,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: true,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b1111,
        rule: "C",
    },
    RawBreakCell {
        prohibited: false,
        levels: 0b0000,
        rule: "C.2#13",
    },
];

const ROW_INDEX: [u8; 31] = [
    255, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 255, 255, 16, 17, 18, 19, 20, 21,
    22, 23, 24, 25, 26, 27,
];
const COLUMN_INDEX: [u8; 31] = [
    255, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 255, 255, 16, 17, 18, 19, 20, 21,
    22, 23, 24, 25, 26, 27,
];
const COLUMN_COUNT: usize = 28;

/// Look up one cell directly from its class or line-edge coordinates.
pub(crate) fn cell(before: u8, after: u8) -> Option<&'static RawBreakCell> {
    let row = *ROW_INDEX.get(usize::from(before))?;
    let column = *COLUMN_INDEX.get(usize::from(after))?;
    if row == u8::MAX || column == u8::MAX {
        return None;
    }
    let index = usize::from(row)
        .checked_mul(COLUMN_COUNT)?
        .checked_add(usize::from(column))?;
    CELLS.get(index)
}
