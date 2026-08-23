// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use alloc::vec::Vec;
use core::ops::Range;

use crate::model::{Frame, Size, WritingMode};

/// Attribution of a placed item to the caller's input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PlacementOrigin {
    /// An ordinal in the shaped-text cluster slice.
    Cluster(usize),
    /// An ordinal in the paragraph's construct slice.
    Construct(usize),
}

/// A local transform sufficient to draw vertical text and tate-chu-yoko.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum CoordinateTransform {
    /// No local transform.
    #[default]
    Identity,
    /// Rotate a horizontal glyph run clockwise into vertical writing.
    RotateClockwise,
    /// Keep a horizontal run upright inside vertical writing.
    TateChuYoko,
}

/// One placed shaped cluster.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ClusterPlacement {
    pub(crate) origin: PlacementOrigin,
    pub(crate) range: Range<usize>,
    pub(crate) inline: i32,
    pub(crate) block: i32,
    pub(crate) advance: i32,
    pub(crate) size: Size,
    pub(crate) frame: Frame,
    pub(crate) writing_mode: WritingMode,
    pub(crate) transform: CoordinateTransform,
}

impl ClusterPlacement {
    /// The input cluster or construct ordinal responsible for this placement.
    #[must_use]
    pub const fn origin(&self) -> PlacementOrigin {
        self.origin
    }

    /// The original source UTF-8 byte range.
    #[must_use]
    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }

    /// Logical inline coordinate in caller units.
    #[must_use]
    pub const fn inline(&self) -> i32 {
        self.inline
    }

    /// Logical block coordinate in caller units.
    #[must_use]
    pub const fn block(&self) -> i32 {
        self.block
    }

    /// The placed inline advance in caller units.
    #[must_use]
    pub const fn advance(&self) -> i32 {
        self.advance
    }

    /// The local size at which the cluster was shaped.
    #[must_use]
    pub const fn size(&self) -> Size {
        self.size
    }

    /// The metrics frame supplied for this cluster.
    #[must_use]
    pub const fn frame(&self) -> Frame {
        self.frame
    }

    /// The cluster's local writing mode.
    #[must_use]
    pub const fn writing_mode(&self) -> WritingMode {
        self.writing_mode
    }

    /// The local transform needed before drawing.
    #[must_use]
    pub const fn transform(&self) -> CoordinateTransform {
        self.transform
    }
}

/// One annotation or mark attached to a base construct.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Attachment {
    pub(crate) construct: usize,
    pub(crate) range: Range<usize>,
    pub(crate) inline: i32,
    pub(crate) block: i32,
    pub(crate) advance: i32,
    pub(crate) size: Size,
    pub(crate) writing_mode: WritingMode,
    pub(crate) transform: CoordinateTransform,
    pub(crate) symbol: Option<char>,
}

impl Attachment {
    /// The ordinal in the paragraph's construct slice.
    #[must_use]
    pub const fn construct(&self) -> usize {
        self.construct
    }

    /// The annotation stream's UTF-8 byte range, or an empty range for a repeated mark.
    #[must_use]
    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }

    /// Logical inline coordinate in caller units.
    #[must_use]
    pub const fn inline(&self) -> i32 {
        self.inline
    }

    /// Logical block coordinate in caller units.
    #[must_use]
    pub const fn block(&self) -> i32 {
        self.block
    }

    /// The annotation's shaped inline advance.
    #[must_use]
    pub const fn advance(&self) -> i32 {
        self.advance
    }

    /// The annotation size.
    #[must_use]
    pub const fn size(&self) -> Size {
        self.size
    }

    /// The annotation's local writing mode.
    #[must_use]
    pub const fn writing_mode(&self) -> WritingMode {
        self.writing_mode
    }

    /// The local transform needed before drawing.
    #[must_use]
    pub const fn transform(&self) -> CoordinateTransform {
        self.transform
    }

    /// A repeated mark such as an emphasis dot, or None for shaped annotations.
    #[must_use]
    pub const fn symbol(&self) -> Option<char> {
        self.symbol
    }
}

/// One composed line and its read-only placement views.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Line {
    pub(crate) range: Range<usize>,
    pub(crate) inline_origin: i32,
    pub(crate) block_origin: i32,
    pub(crate) inline_extent: i32,
    pub(crate) block_extent: i32,
    pub(crate) clusters: Vec<ClusterPlacement>,
    pub(crate) attachments: Vec<Attachment>,
}

impl Line {
    /// The source bytes on this line.
    #[must_use]
    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }

    /// The line's logical inline origin.
    #[must_use]
    pub const fn inline_origin(&self) -> i32 {
        self.inline_origin
    }

    /// The line's logical block origin.
    #[must_use]
    pub const fn block_origin(&self) -> i32 {
        self.block_origin
    }

    /// The occupied inline extent, excluding hanging punctuation.
    #[must_use]
    pub const fn inline_extent(&self) -> i32 {
        self.inline_extent
    }

    /// The line's block-axis demand.
    #[must_use]
    pub const fn block_extent(&self) -> i32 {
        self.block_extent
    }

    /// Base cluster placements in source order.
    #[must_use]
    pub fn clusters(&self) -> &[ClusterPlacement] {
        &self.clusters
    }

    /// Ruby, emphasis, reference-mark, and script attachments.
    #[must_use]
    pub fn attachments(&self) -> &[Attachment] {
        &self.attachments
    }
}

/// Diagnostic severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Severity {
    /// Informational fallback or policy note.
    Info,
    /// Legal output whose quality or fit deserves attention.
    Warning,
    /// Output was preserved, but a requested constraint could not be satisfied.
    Error,
}

/// A stable black-box composition diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Diagnostic {
    pub(crate) code: &'static str,
    pub(crate) severity: Severity,
    pub(crate) range: Option<Range<usize>>,
    pub(crate) jlreq: &'static str,
}

impl Diagnostic {
    /// A stable, language-independent code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.code
    }

    /// Severity.
    #[must_use]
    pub const fn severity(&self) -> Severity {
        self.severity
    }

    /// The responsible paragraph byte range, if one can be isolated.
    #[must_use]
    pub fn range(&self) -> Option<Range<usize>> {
        self.range.clone()
    }

    /// A stable JLReq 2020 address string.
    #[must_use]
    pub const fn jlreq(&self) -> &'static str {
        self.jlreq
    }
}

/// Complete paragraph layout and diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct Layout {
    pub(crate) lines: Vec<Line>,
    pub(crate) diagnostics: Vec<Diagnostic>,
}

impl Layout {
    /// Lines in logical paragraph order.
    #[must_use]
    pub fn lines(&self) -> &[Line] {
        &self.lines
    }

    /// Stable black-box diagnostics in source order.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}
