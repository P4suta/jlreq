// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Mojikumi (文字組み): spacing between adjacent JLReq character classes.
//!
//! Japanese punctuation is drawn inside a full ideographic em with the ink pushed to one
//! side, so setting it without adjustment leaves a visible hole. A closing bracket
//! followed by an opening bracket must lose half an em between them; a comma at the end
//! of a line may be compressed to nothing. These amounts are a function of the two
//! adjacent classes and nothing else, which is why they live in their own crate.
//!
//! This is the layer the CSS `text-spacing-trim` property exposes to the web. Outside a
//! browser there has been no implementation to call.
//!
//! Amounts are fractions of the ideographic em in the workspace's fixed-point unit, never
//! absolute measurements — the em comes from the caller (see `docs/adr/0002`), and the
//! arithmetic is integer (see `docs/adr/0005`).
//!
//! # Status
//!
//! Bootstrap. The spacing tables are not implemented yet; see `ROADMAP.md` (M2).

#![no_std]
