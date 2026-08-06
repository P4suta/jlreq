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
//! # What is here today
//!
//! The facade re-exports the layers, so a caller depends on one crate and names one path
//! for a type that lives in whichever layer owns it (`docs/design/api-spine.md`). Three
//! layers exist at M0 — the units, the specification's own vocabulary, and character
//! classification — and their surfaces are re-exported below; `jlreq-spacing`,
//! `jlreq-line` and `jlreq-inline` join them as they are written.
//!
//! The one thing the facade will own rather than re-export is `diagnose`, which reports
//! what a caller's input says that is unlikely to be what they meant. It needs the
//! constructs, so it arrives with the crate that carries them.
//!
//! # Status
//!
//! Bootstrap. No composition is implemented yet; see `ROADMAP.md`.
//!
//! [jlreq]: https://www.w3.org/TR/jlreq/

#![no_std]

pub use jlreq_class::{
    Annotation, AnnotationIndex, AxisSet, Class, ClassSet, Classified, Member, Members,
    Reclassification, Subject, Text, TextError, Usage, classify, classify_annotation,
    fold_compatibility, members, resolve, usage,
};
pub use jlreq_spec::{
    Address, Answer, Choice, Policy, PolicyConflict, Provenance, Question, RuleId, Standing,
};
pub use jlreq_unit::{
    Advance, BlockDemand, BlockExtent, BlockOffset, ByteOffset, Carry, Construct, ConstructKind,
    ConstructRef, Direction, Distribution, Em, FormulaSetting, Frame, GroupId, InlineCursor,
    InlineEdge, InlineExtent, InlineOffset, Interior, Item, ItemIndex, Ratio, RemainderRule, Role,
    RubyOverhang, RunId, Runs, RunsError, Scale, ScaleId, Segment, Separation, Side, Size,
    Straddle, UNITS_PER_EM, distribute,
};
