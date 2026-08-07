// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Mojikumi (文字組み): spacing between adjacent JLReq character classes.
//!
//! Japanese punctuation is drawn inside a full ideographic em with the ink pushed to one
//! side, so setting it without adjustment leaves a visible hole. A closing bracket followed
//! by an opening bracket must lose half an em between them; a comma at the end of a line
//! may be compressed to nothing. Appendices B through E state this as six matrices and
//! roughly forty notes; this crate reads that data and evaluates it, and states nothing
//! itself that the specification does not.
//!
//! # Where the data lives, and why this is not a second copy
//!
//! `spec/captured/table1.en.tsv` through `table6.en.tsv` are the primary source (ADR-0009):
//! transcribed independently from the English and Japanese renderings of each matrix,
//! cross-checked cell for cell by `xtask attest`. `cargo run -p xtask -- generate` turns
//! the English transcription of each into one `static` array under `src/generated/`, byte
//! checked by `generate --check` against that one input. The generated module is *not* a
//! second, hand-copied table: it is the sole machine-written projection of the captured
//! TSV, exactly as `crates/jlreq-class/src/generated/appendix_a.rs` is of
//! `spec/derived/appendix-a.tsv`. What differs from the derived pipeline is which locale
//! `generate` reads — one `Unit` reads one file — and the complementary control for the
//! other: `xtask attest`'s double entry proves the English and Japanese transcriptions
//! agree cell for cell, and `just design` runs `attest` and `generate --check` in the same
//! pass, over the same committed files, so a divergent reading of one cell fails one gate
//! or the other rather than neither.
//!
//! [`space::ConditionalSpace`], [`boundary::Boundary`] and [`evaluate::boundary`] read that
//! generated data through this crate's private `raw` module's cell shapes; nothing in this
//! crate's own source states an amount, a breakability or a placement that a table or a
//! cited note does not.
//!
//! # Status
//!
//! Table 1 (spacing), Table 2 (line-breaking) and Appendix D/E's reduction and expansion
//! ladders are wired into one evaluator, [`evaluate::boundary`]. §3.7.4's math-formula
//! spacing (cl-17, cl-18) is not yet implemented — those two classes are absent from every
//! one of the six matrices by the specification's own printed axis, and this crate answers
//! an adjacency naming either with "no table constrains this" rather than with the
//! quarter-em and solid settings §3.7.4 states in prose; see `evaluate`'s module doc for the
//! full accounting of which of the forty-seven appendix notes are wired in today. Kinsoku
//! relaxation and line breaking proper are `jlreq-line`'s, a later milestone.

#![no_std]

mod axis;
mod boundary;
mod evaluate;
mod generated;
mod raw;
mod space;

pub use crate::axis::{After, Before};
pub use crate::boundary::{Boundary, Breakable, Delegation, Placement};
pub use crate::evaluate::{Adjacency, Predicate, boundary, rules_fired};
pub use crate::space::{
    ConditionalSpace, Expansion, ExpansionStage, Reduction, ReductionStage, Referent,
};
