// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Repository maintenance tasks for the jlreq workspace.
//!
//! Run with `cargo run -p xtask -- <task>`, or through the `Justfile` recipes.
//!
//! This file is the dispatcher and nothing else. Each task is one module exposing one
//! `GATE`: the name it is invoked by, the sentence its holding justifies, the document to
//! read when it does not hold, and the check itself. The table below is the whole routing
//! decision, so a task is added by writing a module and one line here, and a task is
//! filled in without touching a file any other task shares.
//!
//! The gates this design calls for are enumerated in `docs/design/api-spine.md`, and every
//! one of them is written. Every gate reports the exact census it examined, and no gate
//! ever states that a check it could not run held.

mod api;
mod attest;
mod classes;
mod conform;
mod defects;
mod deferral;
mod derive;
mod direction;
mod examples;
mod generate;
mod inventory;
mod mutation;
mod placeholder;
mod policy;
mod purity;
mod repository;
mod shared;
mod spacing;

use std::process::ExitCode;

use crate::shared::Gate;

/// Every task, in the order a reader meets them in `docs/design/api-spine.md`.
const GATES: &[Gate] = &[
    purity::GATE,
    placeholder::GATE,
    api::GATE,
    direction::GATE,
    derive::GATE,
    generate::GATE,
    attest::GATE,
    conform::GATE,
    examples::GATE,
    mutation::GATE,
    repository::GATE,
];

fn main() -> ExitCode {
    let mut arguments = std::env::args().skip(1);
    let Some(task) = arguments.next() else {
        print_usage();
        return ExitCode::FAILURE;
    };
    let Some(gate) = GATES.iter().find(|gate| gate.name == task) else {
        eprintln!("xtask: unknown task `{task}`");
        print_usage();
        return ExitCode::FAILURE;
    };
    gate.report(&arguments.collect::<Vec<String>>())
}

/// Print the available tasks and what each one states.
fn print_usage() {
    eprintln!("usage: cargo run -p xtask -- <task>");
    for gate in GATES {
        eprintln!(
            "  {name:<11}  {purpose}",
            name = gate.name,
            purpose = gate.purpose
        );
    }
}
