// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Line composition: turning a sequence of characters into lines.
//!
//! The Unicode line breaking algorithm (UAX #14) says where a break is *permitted*. JLReq
//! says which of those breaks are *acceptable* — kinsoku (禁則) forbids `、` and `。` from
//! starting a line and forbids an opening bracket from ending one — and what to do when
//! no acceptable break exists: compress the line to pull the character back (追い込み,
//! oikomi) or expand it to push the character down (追い出し, oidashi), optionally hanging
//! the punctuation past the line end (ぶら下げ).
//!
//! That gap is why text laid out by a UAX #14 implementation alone breaks in places a
//! Japanese reader immediately recognizes as wrong. This crate closes it. It consumes
//! break opportunities rather than discovering them (see `docs/adr/0003`).
//!
//! Lines advance along an *inline* axis and stack along a *block* axis. There is no `x`
//! and no `y` here, and consequently no separate vertical implementation (see
//! `docs/adr/0004`).
//!
//! # Status
//!
//! Bootstrap. No composition is implemented yet; see `ROADMAP.md` (M1, M3).

#![no_std]
