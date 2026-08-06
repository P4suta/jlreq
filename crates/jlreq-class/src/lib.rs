// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! JLReq character class determination.
//!
//! Every Japanese line composition rule is expressed in terms of the thirty character
//! classes defined by [Requirements for Japanese Text Layout][jlreq] — opening brackets
//! (cl-01), closing brackets (cl-02), commas (cl-07), ideographs (cl-19), and so on.
//! Nothing else in this workspace can be written until an occurrence of a character in a
//! text can be mapped to its class.
//!
//! An occurrence is what carries a class, and a code point is not enough to determine
//! one. Two in five of Appendix A's enumerated keys are named by more than one class, and
//! the axis that separates them is how the character was set — full-width, half-width, or
//! proportional — which the document decided and the caller already knows (see
//! `docs/adr/0008`). What belongs to the writing system is the table; the answer belongs
//! to the document.
//!
//! This crate is that mapping and the text it reads, and nothing more. It does not
//! allocate, does not depend on `std`, and holds no state of its own.
//!
//! # Status
//!
//! Bootstrap. The class tables are not implemented yet; see `ROADMAP.md` (M0).
//!
//! [jlreq]: https://www.w3.org/TR/jlreq/

#![no_std]
