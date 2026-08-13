// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The specification-reference vocabulary: rule addresses, provenance, and policy.
//!
//! Every answer this workspace produces carries the rule that produced it, and a rule is
//! named by the specification's own address — `3.1.9`, `B.2#3`, `B.1@cl-05,cl-05` — rather
//! than by an identifier invented here. A failure report is then readable by someone who
//! has never seen this code, and getting the right number from the wrong sentence is a
//! failure rather than a pass, which matters because several answers are reachable by two
//! rules that respond to policy differently (see `docs/adr/0013`).
//!
//! Not every rule is a requirement. JLReq permits alternatives in some places, says
//! nothing in others, and states two incompatible things in a few; each of those is
//! recorded as what it is, so that a reading of this project's own is never published as
//! a requirement of the specification (see `docs/adr/0009`). Where the specification
//! permits more than one answer, the choice is a question the caller answers. A policy is
//! a property of a document rather than of a build — one quotation may be set to JIS
//! conventions inside a book set to JLReq conventions — so it is a value, never a Cargo
//! feature, and a policy the specification makes self-contradictory is refused at
//! construction rather than checked for at every entry point (see `docs/adr/0010`).
//!
//! The rule inventory and the question set are generated from the published specification
//! rather than transcribed by hand (see `docs/adr/0009` and `docs/design/generation.md`).
//! This crate holds no lengths, no classes, and no layout tables: the conformance suite
//! reaches the rule inventory to report coverage without pulling in the whole facade.
//!
//! It has one dependency in the crate graph, `jlreq-unit`, and one reason for it.
//! [`Policy::remainder`] is the single function that derives `RemainderRule` from a policy
//! — the derivation `docs/adr/0019` requires so that a choice reaching `distribute` as a
//! parameter is a transport and not a second carrier — and `RemainderRule` is a quantity
//! type. The edge runs this way and not the other because ADR 0019 and the crate graph
//! both state that `jlreq-unit` depends on nothing, so the seam types carry no rule
//! address and provenance travels in an [`Answer`] instead (see `docs/adr/0020`).
//!
//! # Status
//!
//! The vocabulary is complete. The rule inventory is generated and populated: `src/
//! generated/inventory.rs` holds one row and one named [`RuleId`] constant per statement of
//! §3 and of Appendices B through F, emitted by `cargo run -p xtask -- generate` from
//! `spec/derived/rules.tsv` and byte-checked by `generate --check`. The matrix cells
//! `Address` can address are transcribed rather than derived and join the inventory with
//! the captured matrices, so a cell address parses today and names no rule.
//!
//! The policy space is derived and emitted. `spec/derived/questions.tsv` records every
//! place JLReq permits more than one answer — the question, its address, where the
//! permission comes from, the answers, and the one `Policy::JLREQ` selects — because a
//! question is a section that states two readings, which is a reading of prose rather than
//! a property a scanner computes, and that reading is written down where it can be reviewed
//! and held against the document. Stage 2 has run: [`Question::ALL`], the named `Question`
//! constants and the five [`Policy`] presets are all populated from that file, and
//! [`Policy::remainder`] reads [`Question::REMAINDER`] the same way. Nothing here answers a
//! question the specification has not been read for: an invented one would publish a
//! permitted alternative the specification does not permit, which is the overclaim
//! `docs/adr/0009` exists to prevent.

#![no_std]

mod answer;
mod generated;
mod policy;
mod rule;

pub use crate::answer::{Answer, Provenance};
pub use crate::policy::{Choice, Policy, PolicyConflict, Question};
pub use crate::rule::{Address, RuleId, Standing};
