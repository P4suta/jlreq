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
//! for a type that lives in whichever layer owns it (`docs/design/api-spine.md`). Six
//! layers exist so far — the units, the specification's own vocabulary, character
//! classification, mojikumi spacing, line composition, and inline constructs — and their
//! surfaces are re-exported below.
//!
//! The one thing the facade will own rather than re-export is `diagnose`, which reports
//! what a caller's input says that is unlikely to be what they meant. It needs the
//! constructs, which `jlreq-inline` now carries, but `diagnose` itself is not written —
//! naming the crate that would carry the constructs was never the same fact as the
//! function existing, and it still is not; see `ROADMAP.md`.
//!
//! # Status
//!
//! M1-b's frame: kinsoku (line-start and line-end prohibition), single line alignment
//! (`align`) and tab setting (`tab_line`) are real and wired through
//! `jlreq_spacing::boundary`'s tables, exactly as far as `jlreq-line`'s own `# Status`
//! states. The reduction, hanging and expansion ladders that justify a line rather than
//! set it solid are no longer unfilled slots: `jlreq_line::ladder` implements all three and
//! `Search::Optimal`, the whole-paragraph break search, exists alongside the greedy
//! `Search::FirstFit` (M1-b, M3). Every construct-bearing input is `jlreq-inline`'s, and
//! that claim is now more true than false: mono-ruby (ルビ) is genuinely lowered into the
//! seam `jlreq-line` reads — `jlreq_inline::lower` computes real run identity, forced
//! boundary spacing and block demand for it — and `place()` now positions it too, three of
//! §3.3.5's four cases as real geometry (`jlreq_inline::place`'s own module doc states
//! which, and which one it declines and why). The other eight constructs
//! `docs/design/api-spine.md` names (group- and jukugo-ruby's own distribution among them)
//! remain unwritten; see `ROADMAP.md`.
//!
//! [jlreq]: https://www.w3.org/TR/jlreq/

#![no_std]

pub use jlreq_class::{
    Annotation, AnnotationIndex, AxisSet, Class, ClassSet, Classified, Member, Members,
    Reclassification, Subject, Text, TextError, Usage, classify, classify_annotation,
    fold_compatibility, members, resolve, usage,
};
pub use jlreq_inline::{
    Attachment, Attachments, Constructs, Contribution, LowerError, Lowered, NotAvailable, Ruby,
    RubyAlignment, RubyError, RubyRun, RubyStyle, TateChuYoko, lower, place,
};
pub use jlreq_line::{
    Adjustment, Alignment, Badness, Candidate, CandidateIndex, ComposeError, Composition, Deepest,
    Demerits, Feasible, FeasibleBreak, Fit, Hanging, Ladder, Line, Paragraph, Part, Preference,
    PullUp, Rewrite, Search, Site, TabKind, TabLine, TabStop, Trim, Violation, ViolationKind,
    align, compose, tab_line,
};
pub use jlreq_spacing::{
    Adjacency, After, Before, Boundary, Breakable, ConditionalSpace, Delegation, Expansion,
    ExpansionStage, Placement, Predicate, Reduction, ReductionStage, Referent, boundary,
    rules_fired,
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
