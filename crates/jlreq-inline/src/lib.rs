// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Inline constructs: ruby, tate-chu-yoko, emphasis dots, and warichu.
//!
//! These are the constructs that sit inside a line but are not simply another character
//! in it. Ruby (ルビ) annotates a base with a smaller reading and comes in three
//! attachment styles — mono, group, and jukugo — that place the same reading differently.
//! Tate-chu-yoko (縦中横) sets a short horizontal run inside a vertical line. Emphasis
//! dots (圏点) and warichu (割注) each occupy space the base text does not. Furiwake
//! (振分け), jidori (字取り), reference marks, the ornamented character complex, and
//! formulae are the same shape and lower through the same seam.
//!
//! Each of them can push a line taller or wider than its characters imply, so what they
//! demand is lowered into the line layer's own vocabulary before that layer decides where
//! a line ends. Placement comes after it: ruby may extend over a neighbor only as far as
//! the space that survives line adjustment (§3.3.8 rule 3), but this round's [`place`]
//! reads no such allowance — see its own module doc for why the `overhang` parameter
//! `docs/design/api-spine.md`'s sketch names is this round's own deliberate omission, and
//! for which of §3.3.5's positioning cases are real geometry here regardless. The facade
//! orders the three steps — lower, compose, place.
//!
//! # Status
//!
//! M4-a's fourth slice: mono-ruby is fully lowered and fully placed across three of
//! §3.3.5's four positioning cases, group-ruby is placed across §3.3.6's
//! ruby-not-longer-than-base half, and jukugo-ruby is now placed too, across both of
//! §3.3.7's own paragraphs. [`lower`] turns a caller-declared [`Ruby`] into the four
//! things that cross the seam to `jlreq-line` — [`jlreq_unit::Runs`], forced boundary
//! [`jlreq_unit::Separation`], [`jlreq_unit::BlockDemand`], and the rules applied —
//! genuinely computed for [`RubyStyle::MonoRuby`] (and, for the alignment question alone,
//! for [`RubyStyle::JukugoRuby`] too — `crate::lower`'s own module doc states the hoist),
//! and run identity plus block demand only for [`RubyStyle::GroupRuby`] and
//! [`RubyStyle::JukugoRuby`] otherwise (`crate::lower`'s own module doc names what remains
//! unfilled there: `Question::RUBY_OVERHANG_KANA`, `Question::RUBY_OVERHANG_INDENT`, and
//! §F's own `phonetic` distribution — §3.3.6's own `Question::GROUP_RUBY_DISTRIBUTION` and
//! §3.3.7's own `Question::JUKUGO_RUBY_LAYOUT` are each filled now, but at placement time,
//! not here; see `crate::place`'s own module doc for why `lower` itself is otherwise
//! unchanged).
//! [`place`] then reads a composed line's own placements: every [`RubyStyle::MonoRuby`]
//! attachment nakatsuki or katatsuki centers or start-aligns, declining only §3.3.5(c)'s
//! own katatsuki-with-overflow choice (task #81; `crate::place`'s own module doc states
//! why); every [`RubyStyle::GroupRuby`] attachment is laid out under whichever of
//! `Question::GROUP_RUBY_DISTRIBUTION`'s two answers the policy names, declining only
//! §3.3.6 paragraph 3's own ruby-longer-than-base half, whose method spreads the base apart
//! rather than the ruby (`crate::place`'s own module doc states why that stays a structural
//! blocker this round does not close); and every [`RubyStyle::JukugoRuby`] compound is
//! either placed per base character (paragraph 1, delegating to §3.3.5's own method) or as
//! one compound-wide unit (paragraph 2's own `group` answer, reusing §3.3.6's own geometry
//! forced to `jis` — `decision:jukugo-group-layout-distribution`), declining §F's own
//! `phonetic` answer outright, the ruby-longer-than-base half paragraph 2's own `group`
//! answer inherits from §3.3.6, and a compound §C.2#8's own break permission has split
//! across two lines (`crate::place`'s own module doc states all three in full).
//! [`TateChuYoko::new`] states §3.2.5's own
//! availability fact and nothing past it. `crates/jlreq-conform`'s own `lower` kind (task
//! #74) observes a [`Contribution`] against a case-declared [`Constructs`], closing
//! §3.3.5's alignment question and §3.3.8 rule 1's forced separation; its own eighth kind,
//! `place` (task #80, a separately authored phase, ADR-0006), observes an [`Attachments`]
//! the identical way, over both `RubyAlignment`s and [`RubyStyle::MonoRuby`] initially, and
//! now over [`RubyStyle::GroupRuby`] too: task #85, §3.3.6's own conformance phase by
//! ADR-0006's own separated-phases discipline, has since run — `crates/jlreq-conform/cases/
//! 3.3.6.json` publishes four cases naming rule `3.3.6`, and the rule moves to `[[owned]]`
//! in `docs/conformance-deferrals.toml` on their strength (`crate::place`'s own module doc
//! states the four in full). Task #90 is §3.3.7's own identical conformance phase, and it has
//! since run too: `crates/jlreq-conform/cases/3.3.7.json` publishes three cases naming rule
//! `3.3.7`, and §3.3.7 has moved to `[[owned]]` in `docs/conformance-deferrals.toml` on their
//! strength (`crate::place`'s own module doc states the three in full, alongside the honest
//! scope limit their own `[[owned]]` entry records).
//! Jukugo-ruby placement *geometry* is real now, for paragraphs 1 and 2's own `group`
//! answer; §F's own `phonetic` answer and every other of the eight remaining constructs
//! `docs/design/api-spine.md` names remain unstarted; see `ROADMAP.md` (M4).

#![no_std]

extern crate alloc;

mod lower;
mod place;
mod ruby;
mod tcy;

pub use crate::lower::{Constructs, Contribution, LowerError, Lowered, lower};
pub use crate::place::{Attachment, Attachments, place};
pub use crate::ruby::{Ruby, RubyAlignment, RubyError, RubyRun, RubyStyle};
pub use crate::tcy::{NotAvailable, TateChuYoko};
