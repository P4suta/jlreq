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
//! See `docs/adr/0006-conformance-suite-as-artifact.md`.
//!
//! # Status
//!
//! Bootstrap. No cases are written yet; see `ROADMAP.md` (M0).
