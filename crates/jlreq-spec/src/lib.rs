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
//! `Policy::remainder` is the single function that derives `RemainderRule` from a policy
//! — the derivation `docs/adr/0019` requires so that a choice reaching `distribute` as a
//! parameter is a transport and not a second carrier — and `RemainderRule` is a quantity
//! type. The edge runs this way and not the other because ADR 0019 and the crate graph
//! both state that `jlreq-unit` depends on nothing, so the seam types carry no rule
//! address and provenance travels in an [`Answer`] instead (see `docs/adr/0020`). The
//! dependency is declared in this crate's manifest when `Policy::remainder` lands with the
//! generated policy space, because a declared dependency nothing uses is a `cargo shear`
//! failure.
//!
//! # Status
//!
//! The vocabulary is complete; the inventories it indexes are not yet emitted. The address
//! grammar, the provenance chain, and the structural validation that keeps a
//! self-contradictory [`Policy`] from being built are implemented and tested here.
//! [`RuleId::ALL`] and [`Question::ALL`] are empty, and every table-reading answer is
//! empty with them, until `spec/derived/rules.tsv` and `spec/derived/questions.tsv` exist
//! and `cargo run -p xtask -- generate` fills the two inventories from them. Both are
//! empty and hand-written today, beside the schema each is written against, because there
//! is no generator output to commit: a generated file is one the emitter wrote and
//! `generate --check` reproduces byte for byte, and claiming that provenance for a table
//! nobody generated is the overclaim `docs/adr/0009` exists to prevent. That is the honest
//! state of a table awaiting its pipeline rather than a placeholder: nothing here answers
//! a question the specification has not been read for.
//!
//! Three published items wait on the same data and are absent for the same reason, rather
//! than present and answering wrongly: the named `RuleId` constants, the named [`Question`]
//! constants, and `Policy::remainder`, which reads `Question::REMAINDER` and has no
//! question to read and no answer to give until the policy space exists.

#![no_std]

mod answer;
mod policy;
mod rule;

pub use crate::answer::{Answer, Provenance};
pub use crate::policy::{Choice, Policy, PolicyConflict, Question};
pub use crate::rule::{Address, RuleId, Standing};
