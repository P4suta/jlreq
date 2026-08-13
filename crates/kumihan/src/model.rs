// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use alloc::{string::String, vec::Vec};
use core::ops::Range;

use crate::InputError;

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
    /// A European numeral handled as a grouped Japanese numeral.
    GroupedNumeral,
    /// A character inside a unit symbol.
    UnitSymbol,
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
    source: String,
    size: Size,
    frame: Frame,
    clusters: Vec<Cluster>,
}

impl ShapedText {
    /// Validate and own a source string and its shaped cluster sequence.
    pub fn new<S, I>(source: S, size: Size, frame: Frame, clusters: I) -> Result<Self, InputError>
    where
        S: Into<String>,
        I: IntoIterator<Item = Cluster>,
    {
        let source = source.into();
        let clusters: Vec<_> = clusters.into_iter().collect();
        validate_clusters(&source, frame, &clusters)?;
        Ok(Self {
            source,
            size,
            frame,
            clusters,
        })
    }

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

    pub(crate) fn cluster_boundary(&self, at: usize) -> bool {
        let shaped_boundary = at == 0
            || at == self.source.len()
            || self
                .clusters
                .iter()
                .any(|cluster| cluster.range.start == at || cluster.range.end == at);
        shaped_boundary && !splits_appendix_pair(&self.source, at)
    }
}

fn validate_clusters(
    source: &str,
    default_frame: Frame,
    clusters: &[Cluster],
) -> Result<(), InputError> {
    if source.is_empty() {
        if clusters.is_empty() {
            return Ok(());
        }
        return Err(InputError::new(
            "input.cluster-out-of-range",
            clusters.first().map(Cluster::range),
            "an empty source cannot contain shaped clusters",
        ));
    }
    if clusters.is_empty() {
        return Err(InputError::new(
            "input.uncovered-text",
            Some(0..source.len()),
            "non-empty source text must be covered by shaped clusters",
        ));
    }

    let mut cursor = 0;
    for cluster in clusters {
        let range = &cluster.range;
        if range.start >= range.end || range.end > source.len() {
            return Err(InputError::new(
                "input.cluster-out-of-range",
                Some(range.clone()),
                "a cluster range is empty or outside the source",
            ));
        }
        if !source.is_char_boundary(range.start) || !source.is_char_boundary(range.end) {
            return Err(InputError::new(
                "input.invalid-utf8-boundary",
                Some(range.clone()),
                "a cluster endpoint is not a UTF-8 code-point boundary",
            ));
        }
        if range.start != cursor {
            let code = if range.start < cursor {
                "input.overlapping-clusters"
            } else {
                "input.uncovered-text"
            };
            return Err(InputError::new(
                code,
                Some(range.clone()),
                "clusters must cover the source exactly once in source order",
            ));
        }
        if cluster.advance < 0 {
            return Err(InputError::new(
                "input.negative-advance",
                Some(range.clone()),
                "a shaped advance cannot be negative",
            ));
        }
        let piece = &source[range.clone()];
        if piece.chars().count() > 1
            && cluster.frame.unwrap_or(default_frame) != Frame::Proportional
            && !is_appendix_pair(piece)
        {
            return Err(InputError::new(
                "input.cluster-covers-multiple-keys",
                Some(range.clone()),
                "a non-proportional shaped cluster may cover only one Appendix A key",
            ));
        }
        cursor = range.end;
    }
    if cursor != source.len() {
        return Err(InputError::new(
            "input.uncovered-text",
            Some(cursor..source.len()),
            "clusters must cover the source exactly once",
        ));
    }
    Ok(())
}

fn splits_appendix_pair(source: &str, at: usize) -> bool {
    if at == 0 || at >= source.len() || !source.is_char_boundary(at) {
        return false;
    }
    let Some(before) = source[..at].chars().next_back() else {
        return false;
    };
    let Some(after) = source[at..].chars().next() else {
        return false;
    };
    appendix_pair(before, after)
}

fn is_appendix_pair(piece: &str) -> bool {
    let mut characters = piece.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    let Some(second) = characters.next() else {
        return false;
    };
    characters.next().is_none() && appendix_pair(first, second)
}

fn appendix_pair(first: char, second: char) -> bool {
    matches!(
        (first, second),
        ('\u{00e6}', '\u{0300}')
            | (
                '\u{0254}' | '\u{0259}' | '\u{025a}' | '\u{028c}',
                '\u{0300}' | '\u{0301}'
            )
            | ('\u{02e5}', '\u{02e9}')
            | ('\u{02e9}', '\u{02e5}')
            | (
                '\u{304b}'
                    | '\u{304d}'
                    | '\u{304f}'
                    | '\u{3051}'
                    | '\u{3053}'
                    | '\u{30ab}'
                    | '\u{30ad}'
                    | '\u{30af}'
                    | '\u{30b1}'
                    | '\u{30b3}'
                    | '\u{30bb}'
                    | '\u{30c4}'
                    | '\u{30c8}'
                    | '\u{31f7}',
                '\u{309a}'
            )
    )
}
