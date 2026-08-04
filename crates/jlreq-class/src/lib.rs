// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! JLReq character class determination.
//!
//! Every Japanese line composition rule is expressed in terms of the thirty character
//! classes defined by [Requirements for Japanese Text Layout][jlreq] — opening brackets
//! (cl-1), closing brackets (cl-2), commas (cl-7), ideographs (cl-19), and so on. Nothing
//! else in this workspace can be written until a code point can be mapped to its class.
//!
//! This crate is the mapping and nothing more. It does not allocate, does not depend on
//! `std`, and holds no state: the class of a code point is a property of the writing
//! system, not of a document or a font.
//!
//! # Status
//!
//! Bootstrap. The class tables are not implemented yet; see `ROADMAP.md` (M0).
//!
//! [jlreq]: https://www.w3.org/TR/jlreq/

#![no_std]
