// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use alloc::{string::String, vec::Vec};
use core::ops::Range;

/// A rejected shaped-text or paragraph input.
///
/// The fields are deliberately private. The code is stable; the message is explanatory
/// and may be refined without a breaking release.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct InputError {
    code: &'static str,
    range: Option<Range<usize>>,
    message: &'static str,
}

impl InputError {
    pub(crate) const fn new(
        code: &'static str,
        range: Option<Range<usize>>,
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
    pub fn range(&self) -> Option<Range<usize>> {
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

impl core::error::Error for InputError {}

#[cfg(test)]
mod error_tests {
    use super::*;
    use alloc::format;

    #[test]
    fn input_error_and_cluster_accessors_preserve_values() {
        let error = InputError::new("input.uncovered-text", Some(3..7), "test message");
        assert_eq!(error.code(), "input.uncovered-text");
        assert_eq!(error.range(), Some(3..7));
        assert_eq!(error.message(), "test message");
        assert_eq!(format!("{error}"), "test message");

        let cluster = Cluster::new(3..7, 19);
        assert_eq!(cluster.range(), 3..7);
        assert_eq!(cluster.advance(), 19);
    }
}

/// A caller-unit font size along the inline and block axes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Size {
    inline: i32,
    block: i32,
}

impl Size {
    /// Build a positive, axis-specific size.
    pub fn new(inline: i32, block: i32) -> Result<Self, InputError> {
        if inline <= 0 || block <= 0 {
            return Err(InputError::new(
                "input.invalid-size",
                None,
                "sizes must be positive i32 values",
            ));
        }
        Ok(Self { inline, block })
    }

    /// Build a square size.
    pub fn square(size: i32) -> Result<Self, InputError> {
        Self::new(size, size)
    }

    /// The inline-axis em in caller units.
    #[must_use]
    pub const fn inline(self) -> i32 {
        self.inline
    }

    /// The block-axis em in caller units.
    #[must_use]
    pub const fn block(self) -> i32 {
        self.block
    }

    /// Half the size on each axis, rounding an indivisible caller unit upward.
    pub(crate) const fn half_rounded_up(self) -> Self {
        Self {
            inline: (self.inline / 2).saturating_add(self.inline % 2),
            block: (self.block / 2).saturating_add(self.block % 2),
        }
    }
}

/// The metrics frame used when classifying and spacing a shaped cluster.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Frame {
    /// A Japanese full-em virtual body.
    FullEm,
    /// Proportional metrics, normally used for Western text.
    Proportional,
    /// A half-em virtual body.
    HalfEm,
}

/// The semantic job of a shaped cluster when code points alone are ambiguous.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ClusterRole {
    /// Ordinary prose, with no additional contextual assertion.
    Text,
    /// A punctuation mark used as a decimal point.
    DecimalPoint,
    /// A punctuation mark used as a digit-group separator.
    DigitGroupSeparator,
    /// A question or exclamation mark used inside a sentence.
    SentenceMedial,
    /// A question or exclamation mark ending a sentence.
    SentenceTerminator,
    /// A European numeral handled as a grouped Japanese numeral.
    GroupedNumeral,
    /// A character inside a unit symbol.
    UnitSymbol,
    /// A Western character used as a symbol of a quantity.
    QuantitySymbol,
    /// A character inside a mathematical or chemical formula.
    Formula,
    /// A bracket delimiting warichu.
    WarichuBracket,
}

/// The paragraph's logical-to-physical writing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum WritingMode {
    /// Inline progression is left-to-right and lines progress top-to-bottom.
    #[default]
    HorizontalTb,
    /// Inline progression is top-to-bottom and lines progress right-to-left.
    VerticalRl,
}

/// One indivisible shaped cluster supplied by the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Cluster {
    range: Range<usize>,
    advance: i32,
    size: Option<Size>,
    frame: Option<Frame>,
    role: Option<ClusterRole>,
}

impl Cluster {
    /// Declare a cluster by UTF-8 byte range and shaped inline advance.
    #[must_use]
    pub const fn new(range: Range<usize>, advance: i32) -> Self {
        Self {
            range,
            advance,
            size: None,
            frame: None,
            role: None,
        }
    }

    /// Override the stream's size for this cluster.
    #[must_use]
    pub const fn with_size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
    }

    /// Override the stream's metrics frame for this cluster.
    #[must_use]
    pub const fn with_frame(mut self, frame: Frame) -> Self {
        self.frame = Some(frame);
        self
    }

    /// Supply the document role needed to resolve an ambiguous character.
    #[must_use]
    pub const fn with_role(mut self, role: ClusterRole) -> Self {
        self.role = Some(role);
        self
    }

    /// The source UTF-8 byte range.
    #[must_use]
    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }

    /// The shaped inline advance in caller units.
    #[must_use]
    pub const fn advance(&self) -> i32 {
        self.advance
    }

    /// The size override, if present.
    #[must_use]
    pub const fn size_override(&self) -> Option<Size> {
        self.size
    }

    /// The metrics-frame override, if present.
    #[must_use]
    pub const fn frame_override(&self) -> Option<Frame> {
        self.frame
    }

    /// The document-role override, if present.
    #[must_use]
    pub const fn role(&self) -> Option<ClusterRole> {
        self.role
    }
}

/// Validated source text and its already-shaped, indivisible clusters.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ShapedText {
    pub(crate) source: String,
    pub(crate) size: Size,
    pub(crate) frame: Frame,
    pub(crate) clusters: Vec<Cluster>,
}

impl ShapedText {
    /// The original, unnormalized UTF-8 source.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The stream's default size.
    #[must_use]
    pub const fn size(&self) -> Size {
        self.size
    }

    /// The stream's explicit default metrics frame.
    #[must_use]
    pub const fn frame(&self) -> Frame {
        self.frame
    }

    /// The caller's shaped clusters in source order.
    #[must_use]
    pub fn clusters(&self) -> &[Cluster] {
        &self.clusters
    }
}
