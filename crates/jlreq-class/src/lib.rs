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
//! # What a caller supplies, and why
//!
//! [`Text::new`] takes a string, one [`Item`] per occurrence, and the character sizes the
//! stream declares, and refuses every stream this crate could not answer for. Three of its
//! refusals are the substance of [ADR 0018]: an Appendix A key split across two items, an
//! item covering several keys that is not a Western ligature, and an unstated frame (字幅)
//! on one of the five classes whose character advance §3.1.2 states. The last is why
//! `Text` lives here rather than in `jlreq-unit`: its validity is a statement about
//! Appendix A, and a constructor that cannot read the table it checks against is a
//! constructor that documents its invariant instead of holding it.
//!
//! [`classify`] then answers over one item, and [`resolve`] is the total variant for a
//! caller that must have a class. Every answer carries the rules that produced it, and the
//! two answers this project reads out of a silence say so in their [`Standing`].
//!
//! ```
//! use jlreq_class::{Class, Classified, Text, classify};
//! use jlreq_spec::{Policy, Standing};
//! use jlreq_unit::{Advance, ByteOffset, Frame, InlineExtent, Item, ItemIndex, Scale, ScaleId};
//!
//! let em = Advance::new(1000).expect("a length in the caller's own unit");
//! let scales = [Scale::square(em).expect("a character size is positive")];
//! let advance = InlineExtent::new(1000).expect("the caller measured this");
//!
//! // One ideograph and one full stop. The full stop declares a frame because §3.1.2
//! // states its advance, and there the frame decides a geometry rather than a class.
//! let items = [
//!     Item::new(ByteOffset::new(0), advance, ScaleId::BASE),
//!     Item::new(ByteOffset::new(3), advance, ScaleId::BASE).with_frame(Frame::FullEm),
//! ];
//! let text = Text::new("字。", &items, &scales).expect("one item per Appendix A key");
//!
//! let Classified::One(one) = classify(text, ItemIndex::new(1), Policy::JLREQ) else {
//!     panic!("§A.6 names U+3002 under one class and nothing else names it")
//! };
//! assert_eq!(one.value(), Class::FullStop);
//! assert_eq!(one.why().standing(), Standing::Normative);
//!
//! // Leaving that frame unstated has no representation at all: there is no answer to
//! // report on the geometry, so the stream is refused rather than guessed at.
//! let unstated = [items[0], Item::new(ByteOffset::new(3), advance, ScaleId::BASE)];
//! assert!(Text::new("字。", &unstated, &scales).is_err());
//! ```
//!
//! # Status
//!
//! Complete for M0. `src/generated/` holds Appendix A's 1133 keys as 1686 listings, the
//! thirty class names §3.9.2 publishes, the cl-19 ideograph predicate, the Wide and Narrow
//! compatibility folding, and the two kana scripts §C.2 note 3's fallback reads, each
//! emitted by `cargo run -p xtask -- generate` from the vendored specification snapshot and
//! each byte-checked by `generate --check`. `src/generated.rs` states, by hand, the figures
//! the tables were measured against, so a revision of JLReq that moves a row does not
//! regenerate quietly.
//!
//! One mechanism here is written and has no data yet, and it is named rather than hidden:
//! the reclassification §C.2 notes 1 through 3 state — "shall be treated as a member of"
//! another class — becomes data with the appendix note table and the policy space, neither
//! of which is generated. Until then no reclassification is in force, and one invented here
//! would publish an alternative the specification does not permit.
//!
//! [ADR 0018]: https://github.com/P4suta/kumihan/blob/main/docs/adr/0018-an-item-is-one-occurrence.md
//! [jlreq]: https://www.w3.org/TR/jlreq/
//! [`Item`]: jlreq_unit::Item
//! [`Standing`]: jlreq_spec::Standing

#![no_std]

mod class;
mod classify;
mod generated;
mod member;
mod text;
mod usage;

pub use crate::class::{Class, ClassSet};
pub use crate::classify::{
    AxisSet, Classified, Reclassification, Subject, classify, classify_annotation, resolve,
};
pub use crate::member::{Member, Members, fold_compatibility, members};
pub use crate::text::{Annotation, AnnotationIndex, Text, TextError};
pub use crate::usage::{Usage, usage};
