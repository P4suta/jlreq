// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Japanese line composition (組版) following [JLReq][jlreq] and JIS X 4051.
//!
//! This is the crate to depend on. It composes `jlreq-class`, `jlreq-spacing`,
//! `jlreq-line`, and `jlreq-inline` into one surface; the individual crates exist for
//! callers who need only part of the stack.
//!
//! # What it does and does not do
//!
//! Given text, its already-measured advances, and candidate break positions, it returns
//! where each character goes. It does not load fonts, shape glyphs, rasterize, or read
//! files — the core is `no_std`, allocation-light, and free of floating point, which is
//! what lets it run in a browser, in a game engine, or on a microcontroller.
//!
//! It sits above ICU4X and HarfRust rather than replacing them:
//!
//! ```text
//!    your application / Typst / Parley / PDF writer / game engine
//!                             ▲
//!                          jlreq
//!                             ▲
//!      ICU4X (UAX #14 break opportunities) / HarfRust (shaping)
//! ```
//!
//! The design constraints behind that boundary are recorded in `docs/adr/`.
//!
//! # Status
//!
//! Bootstrap. No composition is implemented yet; see `ROADMAP.md`.
//!
//! [jlreq]: https://www.w3.org/TR/jlreq/

#![no_std]
