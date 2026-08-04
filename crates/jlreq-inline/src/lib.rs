// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Inline constructs: ruby, tate-chu-yoko, emphasis dots, and warichu.
//!
//! These are the constructs that sit inside a line but are not simply another character
//! in it. Ruby (ルビ) annotates a base with a smaller reading and comes in three
//! attachment styles — mono, group, and jukugo — that place the same reading differently.
//! Tate-chu-yoko (縦中横) sets a short horizontal run inside a vertical line. Emphasis
//! dots (圏点) and warichu (割注) each occupy space the base text does not.
//!
//! Each of them can push a line taller or wider than its characters imply, so they are
//! resolved before the line layer decides where a line ends rather than after.
//!
//! # Status
//!
//! Bootstrap. Nothing is implemented yet; see `ROADMAP.md` (M4).

#![no_std]
