// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The generated rule inventory, and the figures this crate is written against.
//!
//! Every Rust file inside `src/generated/` is machine-written: `cargo run -p xtask --
//! generate` emits it from `spec/derived/`, and `generate --check` fails when regenerating
//! it would change a byte. This file is the one hand-written module in the neighborhood,
//! which is why the module declaration lives here beside the directory rather than in a
//! `src/generated/mod.rs` inside it — the `generate` gate reads that convention and reports
//! any file *in* the directory that no generation unit writes.
//!
//! # Why the assertions are here and not in the generated file
//!
//! A generated file asserting its own shape proves that the emitter is self-consistent and
//! nothing more: a specification revision would regenerate the data and the expectation
//! together, and the check would pass by construction. The figures below were measured
//! against the published document and are maintained by hand, so a revision of JLReq that
//! adds a section, drops a note, or changes which rules read the writing direction is a
//! compile error naming the number that changed.
//!
//! The address of every row is checked separately, in [`crate::rule`], because the emitter
//! writes an address as its components rather than as text and a component the grammar
//! refuses would otherwise be a rule nobody can cite.
//!
//! JLReq: §3, §B, §C, §D, §E, §F

pub(crate) mod inventory;

use crate::rule::{RuleId, Standing};

/// How many statements the inventory holds.
///
/// 106: the 59 sections of §3 and Appendices B through F that state something in their own
/// words, and the 47 notes of §B.2, §C.2, §D.2 and §E.2.
const RULES: usize = 106;

/// How many of those rows are notes rather than whole sections.
const NOTES: usize = 47;

/// How many rules read the writing direction.
///
/// Three, and `docs/adr/0011` is the decision that fixes them: §3.1.3, §3.2.5 and §3.3.5.
/// A fourth is a change to the generated data plus a code-owner review, never an incidental
/// branch, and this is the arithmetic half of that sentence.
const DIRECTION_CONDITIONAL: usize = 3;

/// How many rows the inventory carries at a standing other than `Normative`.
///
/// None. Every row quotes the published document, so an `Alternative` arrives with the
/// policy space and an `Unstated` or an `Adjudicated` from `docs/decisions/`: those are
/// this project's published readings, written rather than derived (ADR-0009).
const NOT_NORMATIVE: usize = 0;

/// How many rows of the inventory carry the direction mark.
const fn direction_conditional() -> usize {
    let mut marked: usize = 0;
    let mut index: usize = 0;
    while index < inventory::RULES.len() {
        if inventory::RULES[index].direction_conditional {
            marked = marked.saturating_add(1);
        }
        index = index.saturating_add(1);
    }
    marked
}

/// How many rows address a note of a section rather than a whole section.
const fn notes() -> usize {
    let mut found: usize = 0;
    let mut index: usize = 0;
    while index < inventory::RULES.len() {
        if inventory::RULES[index].address.is_note() {
            found = found.saturating_add(1);
        }
        index = index.saturating_add(1);
    }
    found
}

/// How many rows state a claim of a kind the specification does not make in its own words.
const fn not_normative() -> usize {
    let mut found: usize = 0;
    let mut index: usize = 0;
    while index < inventory::RULES.len() {
        if !matches!(inventory::RULES[index].standing, Standing::Normative) {
            found = found.saturating_add(1);
        }
        index = index.saturating_add(1);
    }
    found
}

// The size of the inventory, and the two partitions of it that are load bearing. A revision
// that adds a section, drops a note, or moves a statement between the two changes one of
// them.
const _: () = assert!(
    inventory::RULES.len() == RULES,
    "the rule inventory no longer holds the number of statements this crate was written \
     against"
);
const _: () = assert!(
    notes() == NOTES,
    "the four `Notes` sections no longer hold the number of notes this crate was written \
     against"
);
const _: () = assert!(
    not_normative() == NOT_NORMATIVE,
    "a row of the inventory claims something the specification does not state in its own \
     words; a reading of this project's own is published from docs/decisions/ and never \
     from the derivation"
);

// ADR 0011's invariant, in the one place a number can carry it. `just direction` holds the
// same set against the items that read it; this holds its size against the decision.
const _: () = assert!(
    direction_conditional() == DIRECTION_CONDITIONAL,
    "the number of rules that read the writing direction has changed, and vertical writing \
     being a direction rather than a second implementation is what that number says"
);

// Every identifier addresses a row, which is what makes `RuleId::address` total.
const _: () = assert!(RuleId::ALL.len() == inventory::RULES.len());
