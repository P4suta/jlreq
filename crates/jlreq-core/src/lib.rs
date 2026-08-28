// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Japanese line composition for already-shaped text.
//!
//! `jlreq-core` owns Japanese spacing, line selection, inline constructs, and physical
//! placement. Font loading, shaping, Unicode line segmentation, bidi resolution, and
//! rendering stay with the caller. Every input position is a UTF-8 byte offset.
//!
//! ```
//! use jlreq_core::{Break, Cluster, Frame, Paragraph, ShapedText, Size, Style};
//!
//! let source = "日本語組版";
//! let clusters = source.char_indices().map(|(start, ch)| {
//!     let end = start + ch.len_utf8();
//!     Cluster::new(start..end, 1_000)
//! });
//! let text = ShapedText::new(source, Size::square(1_000)?, Frame::FullEm, clusters)?;
//! let paragraph = Paragraph::builder(text, 4_000)
//!     .breaks(source.char_indices().skip(1).map(|(at, _)| Break::allowed(at)))
//!     .build()?;
//! let layout = jlreq_core::compose(&paragraph, &Style::book_2020())
//!     .expect("small paragraph is within the default resource limits");
//!
//! assert_eq!(layout.lines().len(), 2);
//! # Ok::<(), jlreq_core::InputError>(())
//! ```

#![no_std]

extern crate alloc;

mod construct;
mod generated;
mod layout;
mod limits;
mod model;
mod normalize;
mod paragraph;
mod pipeline;
mod spec;
pub mod style;

pub use construct::{Construct, Ruby, RubyKind, RubyRun};
pub use layout::{
    Attachment, ClusterPlacement, CoordinateTransform, Diagnostic, Layout, Line, PlacementOrigin,
    Severity,
};
pub use limits::{ComposeError, CompositionLimits, CompositionResource};
pub use model::{Cluster, ClusterRole, Frame, InputError, ShapedText, Size, WritingMode};
pub use paragraph::{Alignment, Break, Paragraph, ParagraphBuilder, TabAlignment, TabStop, Widow};
pub use pipeline::Composer;
pub use style::{Style, StyleBuilder, StyleError};

/// The stable specification identifier implemented by this release line.
pub const SPECIFICATION: &str = "jlreq-2020-08-11+unicode-17.0.0";

/// Compose one validated paragraph with a fresh scratch allocator.
///
/// Use Composer when composing repeatedly so its temporary buffers can be reused.
pub fn compose(paragraph: &Paragraph, style: &Style) -> Result<Layout, ComposeError> {
    Composer::new().compose(paragraph, style)
}
