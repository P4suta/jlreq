// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Japanese line composition for already-shaped text.
//!
//! kumihan owns Japanese spacing, line selection, inline constructs, and physical
//! placement. Font loading, shaping, Unicode line segmentation, bidi resolution, and
//! rendering stay with the caller. Every input position is a UTF-8 byte offset.
//!
//! ```
//! use kumihan::{Break, Cluster, Frame, Paragraph, ShapedText, Size, Style};
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
//! let layout = kumihan::compose(&paragraph, &Style::book_2020());
//!
//! assert_eq!(layout.lines().len(), 2);
//! # Ok::<(), kumihan::InputError>(())
//! ```

#![no_std]

extern crate alloc;

mod construct;
mod generated;
mod layout;
mod model;
mod paragraph;
mod pipeline;
mod spec;
pub mod style;

pub use construct::{Construct, Ruby, RubyKind, RubyRun};
pub use layout::{
    Attachment, ClusterPlacement, CoordinateTransform, Diagnostic, Layout, Line, PlacementOrigin,
    Severity,
};
pub use model::{Cluster, ClusterRole, Frame, ShapedText, Size, WritingMode};
pub use paragraph::{Alignment, Break, Paragraph, ParagraphBuilder, TabAlignment, TabStop, Widow};
pub use pipeline::Composer;
pub use style::{Style, StyleBuilder, StyleError};

/// The stable specification identifier implemented by this release line.
pub const SPECIFICATION: &str = "jlreq-2020-08-11+unicode-17.0.0";

/// Compose one validated paragraph with a fresh scratch allocator.
///
/// Use Composer when composing repeatedly so its temporary buffers can be reused.
#[must_use]
pub fn compose(paragraph: &Paragraph, style: &Style) -> Layout {
    Composer::new().compose(paragraph, style)
}

/// A rejected shaped-text or paragraph input.
///
/// The fields are deliberately private. The code is stable; the message is explanatory
/// and may be refined without a breaking release.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct InputError {
    code: &'static str,
    range: Option<core::ops::Range<usize>>,
    message: &'static str,
}

impl InputError {
    pub(crate) const fn new(
        code: &'static str,
        range: Option<core::ops::Range<usize>>,
        message: &'static str,
    ) -> Self {
        Self {
            code,
            range,
            message,
        }
    }

    /// A stable, language-independent error code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.code
    }

    /// The offending UTF-8 byte range, when one input range is responsible.
    #[must_use]
    pub fn range(&self) -> Option<core::ops::Range<usize>> {
        self.range.clone()
    }

    /// A short English explanation intended for people, not matching in programs.
    #[must_use]
    pub const fn message(&self) -> &'static str {
        self.message
    }
}

impl core::fmt::Display for InputError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.message)
    }
}
