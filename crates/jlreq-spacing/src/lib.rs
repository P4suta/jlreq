// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Mojikumi (文字組み): spacing between adjacent JLReq character classes.
//!
//! Japanese punctuation is drawn inside a full ideographic em with the ink pushed to one
//! side, so setting it without adjustment leaves a visible hole. A closing bracket
//! followed by an opening bracket must lose half an em between them; a comma at the end
//! of a line may be compressed to nothing.
//!
//! The two adjacent classes decide most of an amount and never all of it. The appendix
//! notes also ask which member of a class an item is, whether both items belong to the
//! same ruby or warichu (割注) run, what role the document gave an item, which direction
//! the text is set in, and how the policy questions are answered. All six matrices and
//! every note live here together because they are one coupled rule system — §E.1 defines
//! a blank by reference to Table 2's answer — and a crate boundary between them would run
//! through the middle of one evaluator.
//!
//! This is the layer the CSS `text-spacing-trim` property exposes to the web. Outside a
//! browser there has been no implementation to call.
//!
//! Amounts are fractions of an em in the workspace's fixed-point unit, never absolute
//! measurements, and each says whose em it is a fraction of, because one line may be set
//! in several sizes at once (see `docs/adr/0007`). The em itself comes from the caller
//! (see `docs/adr/0002`), and the arithmetic is integer (see `docs/adr/0005`).
//!
//! # Status
//!
//! Bootstrap. The spacing tables are not implemented yet; see `ROADMAP.md` (M2).

#![no_std]
