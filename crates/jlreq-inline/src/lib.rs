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
//! a line ends. Placement comes after it, and must: ruby may extend over a neighbor only
//! as far as the space that survives line adjustment (§3.3.8 rule 3), so `jlreq-line`
//! resolves the overhang allowance and this crate places annotations against one it is
//! told (see `docs/adr/0015`). The facade orders the three steps — lower, compose, place.
//!
//! # Status
//!
//! Bootstrap. Nothing is implemented yet; see `ROADMAP.md` (M4).

#![no_std]
