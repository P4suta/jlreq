// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! An executable conformance suite for Japanese line composition.
//!
//! JLReq and JIS X 4051 describe correct Japanese typesetting in prose. TeX, browsers,
//! and InDesign each encode an interpretation of that prose in code nobody else can run.
//! There has never been an executable statement of what correct means, which is the real
//! reason this problem has stayed unsolved: anyone starting has to re-derive the rules
//! and then has nothing to check the result against.
//!
//! This crate is that missing artifact, and it is deliberately not a `tests/` directory.
//! Cases are addressed to specification sections rather than to any implementation's
//! internals, and they are data — text, advances, expected placement. This workspace
//! supplies one implementation to evaluate them against; another implementation can
//! supply another and run the identical suite.
//!
//! Where JLReq permits alternatives, a case records every permitted outcome rather than
//! choosing one. Where this project's reading differs from LaTeX's `jlreq` class or from
//! a browser, the case records the disagreement and the reasoning.
//!
//! See `docs/adr/0006-conformance-suite-as-artifact.md` and
//! `docs/design/conformance.md`, which is the format, the trait and the gates.
//!
//! # Running it
//!
//! ```no_run
//! use std::path::Path;
//!
//! use jlreq_conform::{Kumihan, load, run};
//!
//! let suite = load(Path::new("crates/jlreq-conform/cases"))?;
//! let report = run(&suite, &Kumihan::default());
//! println!("{census}", census = report.census());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [`Compose`] returns `Option` throughout, and `None` means *not attempted* rather than
//! *failed*: an engine that exposes only line composition scores honestly on line
//! composition, and the count of cases it never reached is a fact about that engine rather
//! than a verdict on it.
//!
//! # Status
//!
//! M1-b, plus M4-a's own `lower` and `place` kinds. The suite holds cases for all eight
//! questions — `classify`, `boundary`, `compose`, `align`, `tab`, `feasible`, `lower` and
//! `place` — and this workspace's own [`Kumihan`] answers all eight, each over exactly as
//! much of the corpus as `crates/jlreq-conform/src/kumihan.rs`'s own `# What is answered
//! today, and what is not` states; nothing here invents an answer no evaluator produced, and
//! a `None` is still exactly what it always was, *not attempted*, rather than a failure.
//! Reading, running and scoring are written; the `judge` path that scores an answers file
//! produced by an implementation in another language arrives with the layers whose answers
//! it would carry.

mod case;
mod json;
mod kumihan;
mod run;

pub use crate::case::{
    Case, CaseAmount, CaseConstruct, CaseFile, CaseInput, CaseItem, CasePolicy, CaseRun, CaseScale,
    CaseStream, Expect, ExpectAttachment, ExpectBoundary, ExpectClass, ExpectExpansion,
    ExpectFeasible, ExpectLine, ExpectLower, ExpectLowerSeparation, ExpectPart, ExpectPlace,
    ExpectSameRun, ExpectSpace, ExpectTrim, Forbidden, LoadError, Permitted, Suite, load,
};
pub use crate::json::{Json, JsonError};
pub use crate::kumihan::{Kumihan, Stream, refusals};
pub use crate::run::{
    CaseAttachment, CaseBoundary, CaseClass, CaseExpansion, CaseFeasible, CaseLine, CaseLower,
    CaseOutput, CasePart, CasePlace, CaseSpace, CaseTrim, Compose, Disagreement, Edge, Report, run,
    run_file,
};
