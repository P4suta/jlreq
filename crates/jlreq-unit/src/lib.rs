// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Quantities, axes, and the item vocabulary of Japanese line composition (組版).
//!
//! Every layer above this one measures something, and no two of them agree on what a
//! number means until one crate says it first. This is that crate.
//!
//! Two kinds of length exist and they never mix. A quantity the writing system *states* —
//! half an em (二分アキ, nibu aki) between two classes, the amount a table names — is a
//! fraction of the ideographic em (全角, zenkaku) in a fixed-point unit, and is exact. A
//! quantity someone *measured* is an advance in the caller's own unit, supplied rather
//! than computed (see `docs/adr/0002`). Keeping them apart is what lets a spacing amount
//! be compared for equality and a rendered position be stated in the caller's coordinates;
//! one private computation, given a declared em, is the only bridge (see `docs/adr/0007`).
//!
//! Positions repeat the argument one axis out. A line advances along an inline axis and
//! lines stack along a block axis, so there is no `x` and no `y` here, no conversion
//! between the two axes, and consequently no separate vertical implementation: vertical
//! writing is a direction three rules read, not a mode (see `docs/adr/0004` and
//! `docs/adr/0011`).
//!
//! The item vocabulary is what the layers hand each other. One item is one occurrence: a
//! code point together with the character frame (字幅) the caller's advance covers and the
//! role it plays, because a class is a property of an occurrence and not of a code point
//! (see `docs/adr/0008` and `docs/adr/0018`). The text assembled from items lives in
//! `jlreq-class`, where the table that validates it is.
//!
//! This crate holds no specification knowledge, no tables, and no state, and it depends on
//! nothing.
//!
//! # No operators
//!
//! No `core::ops` trait is implemented for any type here, so a bare `+` on a length is a
//! compile error rather than a lint finding. The arithmetic is inherent, closed over each
//! type, and states its overflow behavior in its name: [`Em::add_sat`] saturates at the
//! shared bound and [`Em::add_checked`] refuses past it. Nothing in the workspace needs an
//! operator, which is why none is offered (see `docs/adr/0007` and `docs/adr/0011`).
//!
//! # Status
//!
//! Complete for M0, including every type that crosses the seam between the construct layer
//! and the line layer. The shape is frozen in `docs/design/api-spine.md`.

#![no_std]

mod arith;
mod axis;
mod item;
mod length;
mod run;
mod seam;

pub use crate::arith::{Distribution, InlineCursor, RemainderRule, distribute};
pub use crate::axis::{
    BlockExtent, BlockOffset, Direction, InlineEdge, InlineExtent, InlineOffset, Side,
};
pub use crate::item::{ByteOffset, Frame, Item, ItemIndex, Role};
pub use crate::length::{Advance, Carry, Em, Ratio, Scale, ScaleId, Size, UNITS_PER_EM};
pub use crate::run::{
    Construct, ConstructKind, ConstructRef, FormulaSetting, GroupId, RunId, Runs, RunsError,
};
pub use crate::seam::{BlockDemand, Interior, RubyOverhang, Segment, Separation, Straddle};
