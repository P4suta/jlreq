// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::ops::Range;
use std::sync::Arc;

use icu_segmenter::options::{SentenceBreakInvariantOptions, WordBreakInvariantOptions};
use icu_segmenter::{GraphemeClusterSegmenter, SentenceSegmenter, WordSegmenter};

use crate::units::{finite, quantize, to_f32};
use crate::{FontId, FontResource, LayoutError, LayoutOptions, OptionKind, WritingMode};

/// Physical point represented internally in deterministic 26.6 fixed point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Point {
    x: i32,
    y: i32,
}

impl Point {
    /// Validate and quantize a physical point.
    pub fn try_new(x: f32, y: f32) -> Result<Self, LayoutError> {
        let x = finite(x, OptionKind::Point)?;
        let y = finite(y, OptionKind::Point)?;
        Ok(Self {
            x: quantize(x),
            y: quantize(y),
        })
    }

    pub(crate) const fn from_fixed(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Horizontal coordinate.
    #[must_use]
    pub fn x(self) -> f32 {
        to_f32(self.x)
    }

    /// Vertical coordinate.
    #[must_use]
    pub fn y(self) -> f32 {
        to_f32(self.y)
    }

    /// Raw 26.6 horizontal coordinate.
    #[must_use]
    pub const fn x_26_6(self) -> i32 {
        self.x
    }

    /// Raw 26.6 vertical coordinate.
    #[must_use]
    pub const fn y_26_6(self) -> i32 {
        self.y
    }
}

/// Axis-aligned physical rectangle in deterministic 26.6 fixed point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

impl Rect {
    pub(crate) const fn from_fixed(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Left edge.
    #[must_use]
    pub fn x(self) -> f32 {
        to_f32(self.x)
    }

    /// Top edge.
    #[must_use]
    pub fn y(self) -> f32 {
        to_f32(self.y)
    }

    /// Width.
    #[must_use]
    pub fn width(self) -> f32 {
        to_f32(self.width)
    }

    /// Height.
    #[must_use]
    pub fn height(self) -> f32 {
        to_f32(self.height)
    }

    /// Raw 26.6 components `(x, y, width, height)`.
    #[must_use]
    pub const fn as_26_6(self) -> (i32, i32, i32, i32) {
        (self.x, self.y, self.width, self.height)
    }

    fn contains(self, point: Point) -> bool {
        point.x >= self.x
            && point.y >= self.y
            && point.x <= self.x.saturating_add(self.width)
            && point.y <= self.y.saturating_add(self.height)
    }

    fn union(self, other: Self) -> Self {
        let min_x = self.x.min(other.x);
        let min_y = self.y.min(other.y);
        let max_x = self
            .x
            .saturating_add(self.width)
            .max(other.x.saturating_add(other.width));
        let max_y = self
            .y
            .saturating_add(self.height)
            .max(other.y.saturating_add(other.height));
        Self::from_fixed(
            min_x,
            min_y,
            max_x.saturating_sub(min_x),
            max_y.saturating_sub(min_y),
        )
    }
}

/// Local transform a renderer applies around a glyph origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum GlyphTransform {
    /// No rotation.
    #[default]
    Identity,
    /// Rotate a horizontal glyph clockwise in vertical text.
    RotateClockwise,
    /// Keep a short horizontal run upright inside vertical text.
    TateChuYoko,
}

/// Annotation stream attribution for a glyph not stored in the main source string.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct AnnotationSource {
    construct: usize,
    range: Range<usize>,
}

impl AnnotationSource {
    pub(crate) const fn new(construct: usize, range: Range<usize>) -> Self {
        Self { construct, range }
    }

    /// Ordinal in the document's construct list.
    #[must_use]
    pub const fn construct(&self) -> usize {
        self.construct
    }

    /// UTF-8 byte range in the annotation string.
    #[must_use]
    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }
}

/// One glyph in visual draw order with no renderer-specific handles.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct GlyphPlacement {
    pub(crate) font_id: FontId,
    pub(crate) glyph_id: u32,
    pub(crate) source_range: Range<usize>,
    pub(crate) annotation: Option<AnnotationSource>,
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) advance_x: i32,
    pub(crate) advance_y: i32,
    pub(crate) offset_x: i32,
    pub(crate) offset_y: i32,
    pub(crate) font_size: i32,
    pub(crate) variations: Arc<[crate::FontVariation]>,
    pub(crate) transform: GlyphTransform,
    pub(crate) bidi_level: u8,
    pub(crate) writing_mode: WritingMode,
    pub(crate) construct: Option<usize>,
}

impl GlyphPlacement {
    /// Font resource identifier.
    #[must_use]
    pub const fn font_id(&self) -> FontId {
        self.font_id
    }

    /// OpenType glyph identifier. Zero is `.notdef`.
    #[must_use]
    pub const fn glyph_id(&self) -> u32 {
        self.glyph_id
    }

    /// Original document UTF-8 byte range responsible for this glyph.
    #[must_use]
    pub fn source_range(&self) -> Range<usize> {
        self.source_range.clone()
    }

    /// Annotation-local attribution, if this glyph came from ruby or another attachment.
    #[must_use]
    pub const fn annotation(&self) -> Option<&AnnotationSource> {
        self.annotation.as_ref()
    }

    /// Ordinal of the typed construct this glyph belongs to, when it does.
    ///
    /// Base glyphs inside a construct's range and the construct's own
    /// annotation glyphs both report the same ordinal, which indexes
    /// [`crate::Document::construct`] and matches
    /// [`AnnotationSource::construct`]. Editors use this to select or
    /// highlight a whole ruby group or other structure at once.
    #[must_use]
    pub const fn construct(&self) -> Option<usize> {
        self.construct
    }

    /// Physical glyph origin.
    #[must_use]
    pub const fn origin(&self) -> Point {
        Point::from_fixed(self.x, self.y)
    }

    /// Physical draw origin after applying the shaper's glyph offset.
    ///
    /// Renderers should place the glyph outline at this point, then apply
    /// [`Self::transform`]. [`Self::origin`] remains the unoffset advance-cell
    /// origin.
    #[must_use]
    pub const fn draw_origin(&self) -> Point {
        Point::from_fixed(
            self.x.saturating_add(self.offset_x),
            self.y.saturating_add(self.offset_y),
        )
    }

    /// Physical horizontal origin.
    #[must_use]
    pub fn x(&self) -> f32 {
        to_f32(self.x)
    }

    /// Physical vertical origin.
    #[must_use]
    pub fn y(&self) -> f32 {
        to_f32(self.y)
    }

    /// Shaped horizontal advance.
    #[must_use]
    pub fn advance_x(&self) -> f32 {
        to_f32(self.advance_x)
    }

    /// Shaped vertical advance.
    #[must_use]
    pub fn advance_y(&self) -> f32 {
        to_f32(self.advance_y)
    }

    /// Shaped horizontal offset.
    #[must_use]
    pub fn offset_x(&self) -> f32 {
        to_f32(self.offset_x)
    }

    /// Shaped vertical offset.
    #[must_use]
    pub fn offset_y(&self) -> f32 {
        to_f32(self.offset_y)
    }

    /// Resolved font size used to shape this glyph.
    #[must_use]
    pub fn font_size(&self) -> f32 {
        to_f32(self.font_size)
    }

    /// Resolved font size in signed 26.6 fixed point.
    #[must_use]
    pub const fn font_size_26_6(&self) -> i32 {
        self.font_size
    }

    /// Effective variable-font settings used for shaping.
    ///
    /// The backing slice is shared by glyphs with the same resolved style.
    #[must_use]
    pub fn variations(&self) -> &[crate::FontVariation] {
        &self.variations
    }

    /// `(x, y, advance_x, advance_y, offset_x, offset_y)` in raw 26.6 units.
    #[must_use]
    pub const fn geometry_26_6(&self) -> (i32, i32, i32, i32, i32, i32) {
        (
            self.x,
            self.y,
            self.advance_x,
            self.advance_y,
            self.offset_x,
            self.offset_y,
        )
    }

    /// Local renderer transform.
    #[must_use]
    pub const fn transform(&self) -> GlyphTransform {
        self.transform
    }

    /// Resolved UAX #9 embedding level.
    #[must_use]
    pub const fn bidi_level(&self) -> u8 {
        self.bidi_level
    }

    /// Physical layout-cell bounds, including advance space but not glyph ink.
    ///
    /// Rasterizer outline bounds may be smaller or may overhang this rectangle.
    #[must_use]
    pub fn cell_bounds(&self) -> Rect {
        match (self.writing_mode, self.transform) {
            (WritingMode::HorizontalTb, _) | (_, GlyphTransform::TateChuYoko) => {
                let width = self.advance_x.abs().max(1);
                Rect::from_fixed(
                    self.x,
                    self.y.saturating_sub(self.font_size),
                    width,
                    self.font_size,
                )
            },
            (WritingMode::VerticalRl, _) => {
                let height = self.advance_y.abs().max(1);
                Rect::from_fixed(
                    self.x.saturating_sub(self.font_size),
                    self.y,
                    self.font_size,
                    height,
                )
            },
        }
    }
}

/// Severity of a recoverable layout diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DiagnosticSeverity {
    /// Informational fallback or policy note.
    Info,
    /// Complete output whose quality or fit deserves attention.
    Warning,
    /// A requested constraint was not satisfied, but all input was preserved.
    Error,
}

/// Recoverable, positioned issue accompanying a complete layout.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Diagnostic {
    pub(crate) code: &'static str,
    pub(crate) severity: DiagnosticSeverity,
    pub(crate) range: Option<Range<usize>>,
    pub(crate) message: &'static str,
    pub(crate) jlreq: Option<&'static str>,
}

impl Diagnostic {
    /// Stable machine-readable code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.code
    }

    /// Diagnostic severity.
    #[must_use]
    pub const fn severity(&self) -> DiagnosticSeverity {
        self.severity
    }

    /// Responsible document byte range.
    #[must_use]
    pub fn range(&self) -> Option<Range<usize>> {
        self.range.clone()
    }

    /// Short stable English explanation.
    #[must_use]
    pub const fn message(&self) -> &'static str {
        self.message
    }

    /// JLReq address when this diagnostic originated in the core composer.
    #[must_use]
    pub const fn jlreq(&self) -> Option<&'static str> {
        self.jlreq
    }
}

/// One physical line and its glyphs in visual draw order.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct TextLine {
    pub(crate) range: Range<usize>,
    pub(crate) origin: Point,
    pub(crate) inline_extent: i32,
    pub(crate) block_extent: i32,
    pub(crate) writing_mode: WritingMode,
    pub(crate) glyphs: Vec<GlyphPlacement>,
    pub(crate) hit_bounds: Option<Rect>,
    pub(crate) index: usize,
    pub(crate) paragraph_index: usize,
    pub(crate) first_in_paragraph: bool,
    pub(crate) last_in_paragraph: bool,
}

impl TextLine {
    /// Original document bytes assigned to this line.
    #[must_use]
    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }

    /// Position of this line in [`TextLayout::lines`].
    #[must_use]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Ordinal of the source paragraph this line belongs to.
    ///
    /// Paragraphs are the segments produced by the paragraph separators
    /// (`\n`, `\r\n`, U+2028, U+2029), counted from zero.
    #[must_use]
    pub const fn paragraph_index(&self) -> usize {
        self.paragraph_index
    }

    /// Whether this is its paragraph's first line — where a first-line
    /// indent renders and a drop cap would sit.
    #[must_use]
    pub const fn is_first_in_paragraph(&self) -> bool {
        self.first_in_paragraph
    }

    /// Whether this is its paragraph's final line — the line the widow
    /// policy governs.
    #[must_use]
    pub const fn is_last_in_paragraph(&self) -> bool {
        self.last_in_paragraph
    }

    /// Physical line origin.
    #[must_use]
    pub const fn origin(&self) -> Point {
        self.origin
    }

    /// Occupied inline length.
    #[must_use]
    pub fn inline_extent(&self) -> f32 {
        to_f32(self.inline_extent)
    }

    /// Block-axis demand.
    #[must_use]
    pub fn block_extent(&self) -> f32 {
        to_f32(self.block_extent)
    }

    /// Glyphs in visual draw order, including automatically shaped annotations.
    #[must_use]
    pub fn glyphs(&self) -> &[GlyphPlacement] {
        &self.glyphs
    }

    /// Writing mode used for physical conversion.
    #[must_use]
    pub const fn writing_mode(&self) -> WritingMode {
        self.writing_mode
    }

    /// Physical layout-cell bounds for the line.
    ///
    /// These are not ink bounds. They retain whitespace and include the cell
    /// bounds of ruby and other automatically positioned annotations.
    #[must_use]
    pub fn bounds(&self) -> Rect {
        let base = match self.writing_mode {
            WritingMode::HorizontalTb => Rect::from_fixed(
                self.origin.x,
                self.origin.y,
                self.inline_extent,
                self.block_extent,
            ),
            WritingMode::VerticalRl => Rect::from_fixed(
                self.origin.x.saturating_sub(self.block_extent),
                self.origin.y,
                self.block_extent,
                self.inline_extent,
            ),
        };
        self.hit_bounds.map_or(base, |bounds| base.union(bounds))
    }

    pub(crate) fn hit_bounds_for(glyphs: &[GlyphPlacement]) -> Option<Rect> {
        let mut bounds = glyphs.iter().map(GlyphPlacement::cell_bounds);
        let first = bounds.next()?;
        let mut min_x = first.x;
        let mut min_y = first.y;
        let mut max_x = first.x.saturating_add(first.width);
        let mut max_y = first.y.saturating_add(first.height);
        for rect in bounds {
            min_x = min_x.min(rect.x);
            min_y = min_y.min(rect.y);
            max_x = max_x.max(rect.x.saturating_add(rect.width));
            max_y = max_y.max(rect.y.saturating_add(rect.height));
        }
        Some(Rect::from_fixed(
            min_x,
            min_y,
            max_x.saturating_sub(min_x),
            max_y.saturating_sub(min_y),
        ))
    }
}

/// Which logical side of a UTF-8 position was selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Affinity {
    /// Position before the following cluster in logical order.
    Upstream,
    /// Position after the preceding cluster in logical order.
    Downstream,
}

/// Result of mapping a physical point back to source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HitTest {
    byte_offset: usize,
    affinity: Affinity,
    inside: bool,
}

impl HitTest {
    /// UTF-8 byte boundary nearest the point.
    #[must_use]
    pub const fn byte_offset(self) -> usize {
        self.byte_offset
    }

    /// Logical edge selected at that boundary.
    #[must_use]
    pub const fn affinity(self) -> Affinity {
        self.affinity
    }

    /// Whether the point fell inside a glyph cell rather than merely nearest to it.
    #[must_use]
    pub const fn is_inside(self) -> bool {
        self.inside
    }
}

/// Complete renderer-independent layout that owns every font resource it references.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct TextLayout {
    pub(crate) source: String,
    pub(crate) lines: Vec<TextLine>,
    pub(crate) fonts: Vec<FontResource>,
    pub(crate) diagnostics: Vec<Diagnostic>,
    pub(crate) writing_mode: WritingMode,
    pub(crate) options: LayoutOptions,
}

impl TextLayout {
    /// Original unnormalized UTF-8 text.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Writing mode the layout was produced in.
    ///
    /// Available even for an empty layout, which has no lines to ask.
    #[must_use]
    pub const fn writing_mode(&self) -> WritingMode {
        self.writing_mode
    }

    /// The exact options the layout was produced with.
    ///
    /// Editors can clone this value to lay the same content out again after
    /// an edit without retaining their own copy.
    #[must_use]
    pub const fn options(&self) -> &LayoutOptions {
        &self.options
    }

    /// Lines in paragraph order.
    #[must_use]
    pub fn lines(&self) -> &[TextLine] {
        &self.lines
    }

    /// Font resources sufficient to draw every returned glyph.
    #[must_use]
    pub fn fonts(&self) -> &[FontResource] {
        &self.fonts
    }

    /// Look up a retained font by its library identifier.
    ///
    /// Retained resources can have non-contiguous identifiers because a
    /// layout owns only the faces used by its glyphs. An identifier minted by
    /// a different [`crate::FontLibrary`] returns `None` instead of the wrong
    /// font, even when its slot index is in range.
    #[must_use]
    pub fn font(&self, id: FontId) -> Option<&FontResource> {
        self.fonts
            .binary_search_by_key(&id, FontResource::id)
            .ok()
            .and_then(|index| self.fonts.get(index))
            .filter(|font| font.id.same_provenance(id))
    }

    /// Union of all physical line-cell bounds, or `None` for an empty layout.
    ///
    /// This is a layout-cell boundary rather than an ink boundary: whitespace
    /// and annotation cells are included.
    #[must_use]
    pub fn bounds(&self) -> Option<Rect> {
        let mut lines = self.lines.iter().map(TextLine::bounds);
        let first = lines.next()?;
        Some(lines.fold(first, Rect::union))
    }

    /// Recoverable issues in source order.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Iterate every glyph in global visual draw order.
    pub fn glyphs(&self) -> impl Iterator<Item = &GlyphPlacement> {
        self.lines.iter().flat_map(|line| line.glyphs.iter())
    }

    /// Index of the line holding the caret position `offset`, when one holds it.
    ///
    /// Every offset a caret can occupy is addressable: an offset inside a
    /// line belongs to it, an offset ending a line belongs to that line
    /// unless the next line starts there (a wrap, where the following line
    /// owns the position), and an empty line — a blank paragraph — holds its
    /// own start. Only an offset past the end of the text returns `None`.
    #[must_use]
    pub fn line_index_at(&self, offset: usize) -> Option<usize> {
        let index = self.lines.partition_point(|line| {
            let held_end = line.range.end.max(line.range.start.saturating_add(1));
            held_end <= offset
        });
        if self
            .lines
            .get(index)
            .is_some_and(|line| line.range.start <= offset)
        {
            return Some(index);
        }
        // The offset ends the preceding line: a paragraph separator follows,
        // or the text does.
        let previous = index.checked_sub(1)?;
        self.lines
            .get(previous)
            .filter(|line| line.range.end == offset)
            .map(|_| previous)
    }

    /// The grapheme-cluster boundary strictly after `offset`, when one exists.
    ///
    /// This is the position an editor's forward arrow or delete works on;
    /// stepping bytes or chars instead can land inside a combining sequence
    /// or emoji. Mid-cluster offsets snap forward to the cluster's end.
    #[must_use]
    pub fn next_grapheme_boundary(&self, offset: usize) -> Option<usize> {
        if offset >= self.source.len() {
            return None;
        }
        GraphemeClusterSegmenter::new()
            .segment_str(&self.source)
            .find(|boundary| *boundary > offset)
    }

    /// The grapheme-cluster boundary strictly before `offset`, when one exists.
    #[must_use]
    pub fn prev_grapheme_boundary(&self, offset: usize) -> Option<usize> {
        if offset == 0 {
            return None;
        }
        let mut previous = None;
        for boundary in GraphemeClusterSegmenter::new().segment_str(&self.source) {
            if boundary >= offset.min(self.source.len()) {
                break;
            }
            previous = Some(boundary);
        }
        if offset > self.source.len() {
            return Some(self.source.len());
        }
        previous
    }

    /// The UAX #29 word segment containing `offset`, when one exists.
    ///
    /// Segmentation uses the dictionary-backed automatic segmenter, so
    /// double-click selection works for Japanese text without spaces. The
    /// enclosing segment is returned even over whitespace or punctuation;
    /// inspect the source slice to distinguish. `offset` past the end
    /// returns `None`.
    #[must_use]
    pub fn word_range_at(&self, offset: usize) -> Option<Range<usize>> {
        self.segment_range_at(
            offset,
            WordSegmenter::new_auto(WordBreakInvariantOptions::default()).segment_str(&self.source),
        )
    }

    /// The UAX #29 sentence segment containing `offset`, when one exists.
    #[must_use]
    pub fn sentence_range_at(&self, offset: usize) -> Option<Range<usize>> {
        self.segment_range_at(
            offset,
            SentenceSegmenter::new(SentenceBreakInvariantOptions::default())
                .segment_str(&self.source),
        )
    }

    fn segment_range_at(
        &self,
        offset: usize,
        boundaries: impl Iterator<Item = usize>,
    ) -> Option<Range<usize>> {
        if offset >= self.source.len() {
            return None;
        }
        let mut start = 0;
        for boundary in boundaries {
            if boundary > offset {
                return Some(start..boundary);
            }
            start = boundary;
        }
        None
    }

    /// The caret one visual position toward the line's inline end.
    ///
    /// "Visual" means the reading surface: in bidi text this can jump
    /// logically, and at a line's end it continues onto the following line.
    /// The starting position must be a valid caret on this layout.
    #[must_use]
    pub fn next_visual_caret(&self, offset: usize, affinity: Affinity) -> Option<HitTest> {
        self.step_visual_caret(offset, affinity, true)
    }

    /// The caret one visual position toward the line's inline start.
    #[must_use]
    pub fn prev_visual_caret(&self, offset: usize, affinity: Affinity) -> Option<HitTest> {
        self.step_visual_caret(offset, affinity, false)
    }

    fn step_visual_caret(
        &self,
        offset: usize,
        affinity: Affinity,
        forward: bool,
    ) -> Option<HitTest> {
        let current = self.caret_rect(offset, affinity)?;
        let line_index = self.line_index_for_rect(current)?;
        let current_inline = self.rect_inline_start(current);
        let same_line = self
            .caret_candidates(line_index)
            .into_iter()
            .filter(|(rect, _, _)| {
                if self.line_index_for_rect(*rect) != Some(line_index) {
                    return false;
                }
                let inline = self.rect_inline_start(*rect);
                if forward {
                    inline > current_inline
                } else {
                    inline < current_inline
                }
            })
            .min_by_key(|(rect, candidate_offset, candidate_affinity)| {
                let inline = self.rect_inline_start(*rect);
                let distance = inline.abs_diff(current_inline);
                (
                    distance,
                    *candidate_offset,
                    matches!(candidate_affinity, Affinity::Downstream),
                )
            });
        if let Some((_, next_offset, next_affinity)) = same_line {
            return Some(HitTest {
                byte_offset: next_offset,
                affinity: next_affinity,
                inside: true,
            });
        }
        let neighbor = if forward {
            line_index.saturating_add(1)
        } else {
            line_index.checked_sub(1)?
        };
        let mut candidates = self.caret_candidates(neighbor);
        candidates.retain(|(rect, _, _)| self.line_index_for_rect(*rect) == Some(neighbor));
        let extreme = if forward {
            candidates
                .into_iter()
                .min_by_key(|(rect, candidate_offset, _)| {
                    (self.rect_inline_start(*rect), *candidate_offset)
                })
        } else {
            candidates
                .into_iter()
                .max_by_key(|(rect, candidate_offset, _)| {
                    (
                        self.rect_inline_start(*rect),
                        usize::MAX.saturating_sub(*candidate_offset),
                    )
                })
        };
        extreme.map(|(_, next_offset, next_affinity)| HitTest {
            byte_offset: next_offset,
            affinity: next_affinity,
            inside: true,
        })
    }

    /// The caret at the same inline position on the previous line.
    ///
    /// Lines are in reading order, so in horizontal writing this is the line
    /// above and in vertical writing the column to the right. The first line
    /// returns `None`.
    #[must_use]
    pub fn caret_previous_line(&self, offset: usize, affinity: Affinity) -> Option<HitTest> {
        self.caret_on_neighbor_line(offset, affinity, false)
    }

    /// The caret at the same inline position on the following line.
    #[must_use]
    pub fn caret_next_line(&self, offset: usize, affinity: Affinity) -> Option<HitTest> {
        self.caret_on_neighbor_line(offset, affinity, true)
    }

    fn caret_on_neighbor_line(
        &self,
        offset: usize,
        affinity: Affinity,
        forward: bool,
    ) -> Option<HitTest> {
        let current = self.caret_rect(offset, affinity)?;
        let line_index = self.line_index_for_rect(current)?;
        let target = if forward {
            line_index.saturating_add(1)
        } else {
            line_index.checked_sub(1)?
        };
        let bounds = self.lines.get(target)?.bounds();
        let point = match self.writing_mode {
            WritingMode::HorizontalTb => Point::from_fixed(
                self.rect_inline_start(current),
                bounds.y.saturating_add(bounds.height / 2),
            ),
            WritingMode::VerticalRl => Point::from_fixed(
                bounds.x.saturating_add(bounds.width / 2),
                self.rect_inline_start(current),
            ),
        };
        Some(self.hit_test(point))
    }

    fn rect_inline_start(&self, rect: Rect) -> i32 {
        match self.writing_mode {
            WritingMode::HorizontalTb => rect.x,
            WritingMode::VerticalRl => rect.y,
        }
    }

    fn line_index_for_rect(&self, rect: Rect) -> Option<usize> {
        let center = match self.writing_mode {
            WritingMode::HorizontalTb => {
                Point::from_fixed(rect.x, rect.y.saturating_add(rect.height / 2))
            },
            WritingMode::VerticalRl => {
                Point::from_fixed(rect.x.saturating_add(rect.width / 2), rect.y)
            },
        };
        let mut nearest: Option<(usize, i64)> = None;
        for (index, line) in self.lines.iter().enumerate() {
            let distance = rect_distance(center, line.bounds());
            if nearest.is_none_or(|(_, kept)| distance < kept) {
                nearest = Some((index, distance));
            }
        }
        nearest.map(|(index, _)| index)
    }

    fn caret_candidates(&self, line_index: usize) -> Vec<(Rect, usize, Affinity)> {
        let Some(line) = self.lines.get(line_index) else {
            return Vec::new();
        };
        let mut offsets = vec![line.range.start, line.range.end];
        for glyph in &line.glyphs {
            if glyph.annotation.is_some() {
                continue;
            }
            offsets.push(glyph.source_range.start);
            offsets.push(glyph.source_range.end);
        }
        offsets.sort_unstable();
        offsets.dedup();
        let mut result = Vec::new();
        for candidate_offset in offsets {
            for candidate_affinity in [Affinity::Upstream, Affinity::Downstream] {
                if let Some(rect) = self.caret_rect(candidate_offset, candidate_affinity) {
                    result.push((rect, candidate_offset, candidate_affinity));
                }
            }
        }
        result
    }

    /// Selection rectangles with each line filled to its layout edge.
    ///
    /// [`selection_rects`](Self::selection_rects) returns exact glyph-cell
    /// unions; this variant returns what editors usually paint instead — one
    /// rectangle per touched line, extended to the line's trailing layout
    /// edge whenever the selection continues past that line, and to its
    /// leading edge whenever the selection began earlier. A bidi selection
    /// yields each line's bounding box rather than split runs. The same
    /// validity rules apply: an empty or misaligned range yields no
    /// rectangles.
    #[must_use]
    pub fn selection_rects_filled(&self, range: Range<usize>) -> Vec<Rect> {
        if !is_valid_selection_range(&self.source, &range) {
            return Vec::new();
        }
        let mut result = Vec::new();
        for line in &self.lines {
            if line.range.start >= range.end || line.range.end <= range.start {
                continue;
            }
            let clamped = range.start.max(line.range.start)..range.end.min(line.range.end);
            let pieces = self.selection_rects(clamped.clone());
            let mut piece_iter = pieces.iter().copied();
            let Some(first) = piece_iter.next() else {
                continue;
            };
            let merged = piece_iter.fold(first, Rect::union);
            let bounds = line.bounds();
            let (mut lead, mut trail) = match self.writing_mode {
                WritingMode::HorizontalTb => (merged.x, merged.x.saturating_add(merged.width)),
                WritingMode::VerticalRl => (merged.y, merged.y.saturating_add(merged.height)),
            };
            let (line_lead, line_trail) = match self.writing_mode {
                WritingMode::HorizontalTb => (bounds.x, bounds.x.saturating_add(bounds.width)),
                WritingMode::VerticalRl => (bounds.y, bounds.y.saturating_add(bounds.height)),
            };
            if range.start < line.range.start {
                lead = lead.min(line_lead);
            }
            if range.end > line.range.end {
                trail = trail.max(line_trail);
            }
            result.push(match self.writing_mode {
                WritingMode::HorizontalTb => {
                    Rect::from_fixed(lead, merged.y, trail.saturating_sub(lead), merged.height)
                },
                WritingMode::VerticalRl => {
                    Rect::from_fixed(merged.x, lead, merged.width, trail.saturating_sub(lead))
                },
            });
        }
        result
    }

    /// Map a physical point to the nearest logical UTF-8 boundary.
    #[must_use]
    pub fn hit_test(&self, point: Point) -> HitTest {
        let mut nearest_line: Option<(&TextLine, i64)> = None;
        for line in &self.lines {
            let bounds = line.bounds();
            let distance = rect_distance(point, bounds);
            if nearest_line.is_none_or(|(_, old)| distance < old) {
                nearest_line = Some((line, distance));
            }
        }
        let Some((line, _)) = nearest_line else {
            return HitTest {
                byte_offset: 0,
                affinity: Affinity::Downstream,
                inside: false,
            };
        };

        let mut best: Option<(&GlyphPlacement, i64, bool)> = None;
        for glyph in line
            .glyphs
            .iter()
            .filter(|glyph| glyph.annotation.is_none())
        {
            let bounds = glyph.cell_bounds();
            let inside = bounds.contains(point);
            let distance = rect_distance(point, bounds);
            if best.is_none_or(|(_, old, old_inside)| better_hit(inside, distance, old_inside, old))
            {
                best = Some((glyph, distance, inside));
            }
        }
        let Some((glyph, _, inside)) = best else {
            return HitTest {
                byte_offset: line.range.start,
                affinity: Affinity::Downstream,
                inside: line.bounds().contains(point),
            };
        };
        let bounds = glyph.cell_bounds();
        let after_visual_midpoint = is_after_visual_midpoint(point, bounds, glyph.writing_mode);
        let logical_after = is_logically_after(after_visual_midpoint, glyph.bidi_level);
        HitTest {
            byte_offset: if logical_after {
                glyph.source_range.end
            } else {
                glyph.source_range.start
            },
            affinity: if logical_after {
                Affinity::Downstream
            } else {
                Affinity::Upstream
            },
            inside,
        }
    }

    /// Convenience wrapper around [`Self::hit_test`].
    pub fn hit_test_xy(&self, x: f32, y: f32) -> Result<HitTest, LayoutError> {
        Ok(self.hit_test(Point::try_new(x, y)?))
    }

    /// Return a one-subpixel caret for one logical side of a UTF-8 boundary.
    ///
    /// Affinity disambiguates wrapping, paragraph breaks, and bidirectional
    /// boundaries. Passing back both fields of [`HitTest`] reproduces the same
    /// visual edge.
    #[must_use]
    pub fn caret_rect(&self, byte_offset: usize, affinity: Affinity) -> Option<Rect> {
        if !is_valid_caret_offset(&self.source, byte_offset) {
            return None;
        }
        for line in &self.lines {
            let mut candidates = line
                .glyphs
                .iter()
                .filter(|glyph| glyph.annotation.is_none())
                .filter(|glyph| match affinity {
                    Affinity::Upstream => glyph.source_range.start == byte_offset,
                    Affinity::Downstream => glyph.source_range.end == byte_offset,
                });
            if let Some(first) = candidates.next() {
                let bounds = candidates.fold(first.cell_bounds(), |bounds, glyph| {
                    bounds.union(glyph.cell_bounds())
                });
                let visual_end = is_visual_end(affinity == Affinity::Upstream, first.bidi_level);
                return Some(caret_for_bounds(bounds, visual_end, first.writing_mode));
            }
            let has_main_glyph = line.glyphs.iter().any(|glyph| glyph.annotation.is_none());
            if line.range.start == byte_offset && !has_main_glyph {
                return Some(empty_line_caret(line));
            }
        }
        None
    }

    /// Return one rectangle per visually contiguous selected run on each line.
    #[must_use]
    pub fn selection_rects(&self, range: Range<usize>) -> Vec<Rect> {
        if !is_valid_selection_range(&self.source, &range) {
            return Vec::new();
        }
        let mut result = Vec::new();
        let first_line = self
            .lines
            .partition_point(|line| line.range.end <= range.start);
        for line in &self.lines[first_line..] {
            if line.range.start >= range.end {
                break;
            }
            let mut current = None;
            for glyph in line
                .glyphs
                .iter()
                .filter(|glyph| glyph.annotation.is_none())
            {
                if ranges_overlap(&glyph.source_range, &range) {
                    let bounds = glyph.cell_bounds();
                    current = Some(current.map_or(bounds, |rect: Rect| rect.union(bounds)));
                } else if let Some(bounds) = current.take() {
                    result.push(bounds);
                }
            }
            if let Some(bounds) = current {
                result.push(bounds);
            }
        }
        result
    }
}

fn axis_distance(value: i32, start: i32, extent: i32) -> i64 {
    let end = start.saturating_add(extent);
    let before = start.saturating_sub(value).max(0);
    let after = value.saturating_sub(end).max(0);
    i64::from(before.max(after))
}

fn rect_distance(point: Point, bounds: Rect) -> i64 {
    let dx = axis_distance(point.x, bounds.x, bounds.width);
    let dy = axis_distance(point.y, bounds.y, bounds.height);
    dx.saturating_mul(dx).saturating_add(dy.saturating_mul(dy))
}

fn better_hit(inside: bool, distance: i64, old_inside: bool, old_distance: i64) -> bool {
    (inside && !old_inside) || (inside == old_inside && distance < old_distance)
}

fn is_after_visual_midpoint(point: Point, bounds: Rect, mode: WritingMode) -> bool {
    match mode {
        WritingMode::HorizontalTb => point.x >= bounds.x.saturating_add(bounds.width / 2),
        WritingMode::VerticalRl => point.y >= bounds.y.saturating_add(bounds.height / 2),
    }
}

fn is_logically_after(after_visual_midpoint: bool, bidi_level: u8) -> bool {
    after_visual_midpoint ^ (bidi_level % 2 == 1)
}

fn is_valid_caret_offset(source: &str, offset: usize) -> bool {
    offset <= source.len() && source.is_char_boundary(offset)
}

fn is_visual_end(at_start: bool, bidi_level: u8) -> bool {
    at_start == (bidi_level % 2 == 1)
}

fn caret_for_bounds(bounds: Rect, visual_end: bool, mode: WritingMode) -> Rect {
    match mode {
        WritingMode::HorizontalTb => Rect::from_fixed(
            if visual_end {
                bounds.x.saturating_add(bounds.width)
            } else {
                bounds.x
            },
            bounds.y,
            1,
            bounds.height,
        ),
        WritingMode::VerticalRl => Rect::from_fixed(
            bounds.x,
            if visual_end {
                bounds.y.saturating_add(bounds.height)
            } else {
                bounds.y
            },
            bounds.width,
            1,
        ),
    }
}

fn empty_line_caret(line: &TextLine) -> Rect {
    let bounds = line.bounds();
    match line.writing_mode {
        WritingMode::HorizontalTb => Rect::from_fixed(line.origin.x, bounds.y, 1, bounds.height),
        WritingMode::VerticalRl => Rect::from_fixed(bounds.x, line.origin.y, bounds.width, 1),
    }
}

fn is_valid_selection_range(source: &str, range: &Range<usize>) -> bool {
    if range.start >= range.end {
        return false;
    }
    if range.end > source.len() {
        return false;
    }
    source.is_char_boundary(range.start) && source.is_char_boundary(range.end)
}

fn ranges_overlap(left: &Range<usize>, right: &Range<usize>) -> bool {
    left.start < right.end && right.start < left.end
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn test_options() -> LayoutOptions {
        LayoutOptions::try_new(100.0, 10.0).unwrap()
    }

    fn assert_float(actual: f32, expected: f32) {
        assert_eq!(actual.to_bits(), expected.to_bits());
    }

    fn font_id() -> FontId {
        let mut fonts = crate::FontLibrary::new();
        fonts
            .register_font(Arc::<[u8]>::from(font_test_data::NOTO_SANS_JP_CFF))
            .unwrap()
    }

    fn glyph(mode: WritingMode) -> GlyphPlacement {
        GlyphPlacement {
            font_id: font_id(),
            glyph_id: 77,
            source_range: 2..5,
            annotation: Some(AnnotationSource::new(3, 7..11)),
            x: 128,
            y: 320,
            advance_x: 256,
            advance_y: -384,
            offset_x: 96,
            offset_y: -128,
            font_size: 192,
            variations: Arc::from([crate::FontVariation::try_new(
                crate::OpenTypeTag::try_new("wght").unwrap(),
                650.0,
            )
            .unwrap()]),
            transform: GlyphTransform::RotateClockwise,
            bidi_level: 2,
            writing_mode: mode,
            construct: Some(3),
        }
    }

    fn linear_hit_test(layout: &TextLayout, point: Point) -> HitTest {
        let Some(line) = layout
            .lines
            .iter()
            .min_by_key(|line| rect_distance(point, line.bounds()))
        else {
            return HitTest {
                byte_offset: 0,
                affinity: Affinity::Downstream,
                inside: false,
            };
        };
        let mut best: Option<(&GlyphPlacement, i64, bool)> = None;
        for glyph in line
            .glyphs
            .iter()
            .filter(|glyph| glyph.annotation.is_none())
        {
            let bounds = glyph.cell_bounds();
            let inside = bounds.contains(point);
            let dx = axis_distance(point.x, bounds.x, bounds.width);
            let dy = axis_distance(point.y, bounds.y, bounds.height);
            let distance = dx.saturating_mul(dx).saturating_add(dy.saturating_mul(dy));
            if best.is_none_or(|(_, old, old_inside)| better_hit(inside, distance, old_inside, old))
            {
                best = Some((glyph, distance, inside));
            }
        }
        let Some((glyph, _, inside)) = best else {
            return HitTest {
                byte_offset: line.range.start,
                affinity: Affinity::Downstream,
                inside: line.bounds().contains(point),
            };
        };
        let bounds = glyph.cell_bounds();
        let after_visual_midpoint = is_after_visual_midpoint(point, bounds, glyph.writing_mode);
        let logical_after = is_logically_after(after_visual_midpoint, glyph.bidi_level);
        HitTest {
            byte_offset: if logical_after {
                glyph.source_range.end
            } else {
                glyph.source_range.start
            },
            affinity: if logical_after {
                Affinity::Downstream
            } else {
                Affinity::Upstream
            },
            inside,
        }
    }

    fn linear_caret_rect(
        layout: &TextLayout,
        byte_offset: usize,
        affinity: Affinity,
    ) -> Option<Rect> {
        if !is_valid_caret_offset(&layout.source, byte_offset) {
            return None;
        }
        for line in &layout.lines {
            let mut candidates = line
                .glyphs
                .iter()
                .filter(|glyph| glyph.annotation.is_none())
                .filter(|glyph| match affinity {
                    Affinity::Upstream => glyph.source_range.start == byte_offset,
                    Affinity::Downstream => glyph.source_range.end == byte_offset,
                });
            if let Some(glyph) = candidates.next() {
                let bounds = glyph.cell_bounds();
                let at_start = affinity == Affinity::Upstream;
                let visual_end = is_visual_end(at_start, glyph.bidi_level);
                return Some(match glyph.writing_mode {
                    WritingMode::HorizontalTb => Rect::from_fixed(
                        if visual_end {
                            bounds.x.saturating_add(bounds.width)
                        } else {
                            bounds.x
                        },
                        bounds.y,
                        1,
                        bounds.height,
                    ),
                    WritingMode::VerticalRl => Rect::from_fixed(
                        bounds.x,
                        if visual_end {
                            bounds.y.saturating_add(bounds.height)
                        } else {
                            bounds.y
                        },
                        bounds.width,
                        1,
                    ),
                });
            }
            if line.range.start == byte_offset
                && !line.glyphs.iter().any(|glyph| glyph.annotation.is_none())
            {
                return Some(empty_line_caret(line));
            }
        }
        None
    }

    fn linear_selection_rects(layout: &TextLayout, range: Range<usize>) -> Vec<Rect> {
        if !is_valid_selection_range(&layout.source, &range) {
            return Vec::new();
        }
        let mut result = Vec::new();
        for line in &layout.lines {
            let mut current = None;
            for glyph in line
                .glyphs
                .iter()
                .filter(|glyph| glyph.annotation.is_none())
            {
                if ranges_overlap(&glyph.source_range, &range) {
                    let bounds = glyph.cell_bounds();
                    current = Some(current.map_or(bounds, |rect: Rect| rect.union(bounds)));
                } else if let Some(bounds) = current.take() {
                    result.push(bounds);
                }
            }
            if let Some(bounds) = current {
                result.push(bounds);
            }
        }
        result
    }

    #[test]
    fn rectangle_accessors_preserve_all_signed_fixed_components() {
        let rect = Rect::from_fixed(128, -192, 256, 320);
        assert_float(rect.x(), 2.0);
        assert_float(rect.y(), -3.0);
        assert_float(rect.width(), 4.0);
        assert_float(rect.height(), 5.0);
        assert_eq!(rect.as_26_6(), (128, -192, 256, 320));
        assert!(rect.contains(Point::from_fixed(128, -192)));
        assert!(rect.contains(Point::from_fixed(384, 128)));
        assert!(!rect.contains(Point::from_fixed(127, -192)));
        assert!(!rect.contains(Point::from_fixed(128, 129)));
    }

    #[test]
    fn hit_distance_priority_and_both_writing_mode_midpoints_are_exact() {
        assert_eq!(axis_distance(5, 10, 4), 5);
        assert_eq!(axis_distance(10, 10, 4), 0);
        assert_eq!(axis_distance(12, 10, 4), 0);
        assert_eq!(axis_distance(14, 10, 4), 0);
        assert_eq!(axis_distance(15, 10, 4), 1);

        assert!(better_hit(true, 100, false, 1));
        assert!(!better_hit(false, 1, true, 100));
        assert!(better_hit(false, 4, false, 5));
        assert!(!better_hit(false, 5, false, 5));
        assert!(!better_hit(false, 6, false, 5));
        assert!(better_hit(true, 4, true, 5));

        let bounds = Rect::from_fixed(100, 200, 80, 120);
        assert!(!is_after_visual_midpoint(
            Point::from_fixed(139, 259),
            bounds,
            WritingMode::HorizontalTb
        ));
        assert!(is_after_visual_midpoint(
            Point::from_fixed(140, 259),
            bounds,
            WritingMode::HorizontalTb
        ));
        assert!(!is_after_visual_midpoint(
            Point::from_fixed(139, 259),
            bounds,
            WritingMode::VerticalRl
        ));
        assert!(is_after_visual_midpoint(
            Point::from_fixed(139, 260),
            bounds,
            WritingMode::VerticalRl
        ));

        assert!(!is_logically_after(false, 0));
        assert!(is_logically_after(true, 0));
        assert!(is_logically_after(false, 1));
        assert!(!is_logically_after(true, 1));
        assert!(!is_logically_after(false, 2));
        assert!(is_logically_after(false, 3));
    }

    #[test]
    fn caret_and_selection_predicates_preserve_utf8_and_half_open_edges() {
        let source = "éAB";
        assert!(is_valid_caret_offset(source, 0));
        assert!(!is_valid_caret_offset(source, 1));
        assert!(is_valid_caret_offset(source, 2));
        assert!(is_valid_caret_offset(source, source.len()));
        assert!(!is_valid_caret_offset(source, source.len() + 1));
        assert!(!is_visual_end(true, 0));
        assert!(is_visual_end(false, 0));
        assert!(is_visual_end(true, 1));
        assert!(!is_visual_end(false, 1));

        assert!(is_valid_selection_range(source, &(0..2)));
        assert!(!is_valid_selection_range(source, &(2..2)));
        assert!(!is_valid_selection_range(source, &(0..source.len() + 1)));
        assert!(!is_valid_selection_range(source, &(1..2)));
        assert!(!is_valid_selection_range(source, &(0..1)));
        assert!(ranges_overlap(&(2..5), &(1..3)));
        assert!(!ranges_overlap(&(2..5), &(0..2)));
        assert!(!ranges_overlap(&(2..5), &(5..6)));
    }

    #[test]
    fn glyph_and_annotation_accessors_preserve_draw_ready_geometry() {
        let horizontal = glyph(WritingMode::HorizontalTb);
        assert_eq!(horizontal.font_id().get(), 0);
        assert_eq!(horizontal.glyph_id(), 77);
        assert_eq!(horizontal.source_range(), 2..5);
        let annotation = horizontal.annotation().unwrap();
        assert_eq!(annotation.construct(), 3);
        assert_eq!(annotation.range(), 7..11);
        assert_eq!(horizontal.origin().x_26_6(), 128);
        assert_eq!(horizontal.origin().y_26_6(), 320);
        assert_eq!(horizontal.draw_origin().x_26_6(), 224);
        assert_eq!(horizontal.draw_origin().y_26_6(), 192);
        assert_float(horizontal.x(), 2.0);
        assert_float(horizontal.y(), 5.0);
        assert_float(horizontal.advance_x(), 4.0);
        assert_float(horizontal.advance_y(), -6.0);
        assert_float(horizontal.offset_x(), 1.5);
        assert_float(horizontal.offset_y(), -2.0);
        assert_float(horizontal.font_size(), 3.0);
        assert_eq!(horizontal.font_size_26_6(), 192);
        assert_eq!(horizontal.variations()[0].value_26_6(), 650 * 64);
        assert_eq!(horizontal.geometry_26_6(), (128, 320, 256, -384, 96, -128));
        assert_eq!(horizontal.transform(), GlyphTransform::RotateClockwise);
        assert_eq!(horizontal.bidi_level(), 2);
        assert_eq!(horizontal.cell_bounds().as_26_6(), (128, 128, 256, 192));

        let vertical = glyph(WritingMode::VerticalRl);
        assert_eq!(vertical.cell_bounds().as_26_6(), (-64, 320, 192, 384));
        let mut tate_chu_yoko = vertical;
        tate_chu_yoko.transform = GlyphTransform::TateChuYoko;
        assert_eq!(tate_chu_yoko.cell_bounds().as_26_6(), (128, 128, 256, 192));
    }

    #[test]
    fn diagnostic_line_hit_caret_and_selection_values_are_exact() {
        let diagnostic = Diagnostic {
            code: "test.code",
            severity: DiagnosticSeverity::Error,
            range: Some(2..5),
            message: "test message",
            jlreq: Some("3.1.2"),
        };
        assert_eq!(diagnostic.code(), "test.code");
        assert_eq!(diagnostic.severity(), DiagnosticSeverity::Error);
        assert_eq!(diagnostic.range(), Some(2..5));
        assert_eq!(diagnostic.message(), "test message");
        assert_eq!(diagnostic.jlreq(), Some("3.1.2"));

        let mut rendered = glyph(WritingMode::HorizontalTb);
        rendered.annotation = None;
        let glyphs = vec![rendered];
        let hit_bounds = TextLine::hit_bounds_for(&glyphs);
        let line = TextLine {
            range: 2..5,
            origin: Point::from_fixed(32, 64),
            inline_extent: 448,
            block_extent: 192,
            writing_mode: WritingMode::HorizontalTb,
            glyphs,
            hit_bounds,
            index: 0,
            paragraph_index: 0,
            first_in_paragraph: true,
            last_in_paragraph: true,
        };
        assert_eq!(line.range(), 2..5);
        assert_eq!(line.origin().x_26_6(), 32);
        assert_eq!(line.origin().y_26_6(), 64);
        assert_float(line.inline_extent(), 7.0);
        assert_float(line.block_extent(), 3.0);
        assert_eq!(line.writing_mode(), WritingMode::HorizontalTb);
        assert_eq!(line.glyphs().len(), 1);
        assert_eq!(line.bounds().as_26_6(), (32, 64, 448, 256));

        let layout = TextLayout {
            source: "abXYZq".to_owned(),
            lines: vec![line],
            fonts: Vec::new(),
            diagnostics: vec![diagnostic],
            writing_mode: WritingMode::HorizontalTb,
            options: test_options(),
        };
        assert_eq!(layout.bounds().unwrap().as_26_6(), (32, 64, 448, 256));
        let before = layout.hit_test(Point::from_fixed(160, 200));
        assert_eq!(before.byte_offset(), 2);
        assert_eq!(before.affinity(), Affinity::Upstream);
        assert!(before.is_inside());
        let after = layout.hit_test(Point::from_fixed(352, 200));
        assert_eq!(after.byte_offset(), 5);
        assert_eq!(after.affinity(), Affinity::Downstream);
        assert!(after.is_inside());
        assert_eq!(
            layout.caret_rect(2, Affinity::Upstream).unwrap().as_26_6(),
            (128, 128, 1, 192)
        );
        assert_eq!(
            layout
                .caret_rect(5, Affinity::Downstream)
                .unwrap()
                .as_26_6(),
            (384, 128, 1, 192)
        );
        assert_eq!(
            layout.selection_rects(2..5)[0].as_26_6(),
            (128, 128, 256, 192)
        );
        assert!(layout.selection_rects(0..2).is_empty());
        assert!(layout.selection_rects(5..6).is_empty());

        let mut rtl = layout.clone();
        rtl.lines[0].glyphs[0].bidi_level = 1;
        assert_eq!(
            rtl.caret_rect(2, Affinity::Upstream).unwrap().as_26_6(),
            (384, 128, 1, 192)
        );
        assert_eq!(
            rtl.caret_rect(5, Affinity::Downstream).unwrap().as_26_6(),
            (128, 128, 1, 192)
        );

        let empty_horizontal = TextLayout {
            source: "A".into(),
            lines: vec![TextLine {
                range: 0..0,
                origin: Point::from_fixed(320, 640),
                inline_extent: 0,
                block_extent: 192,
                writing_mode: WritingMode::HorizontalTb,
                glyphs: Vec::new(),
                hit_bounds: None,
                index: 0,
                paragraph_index: 0,
                first_in_paragraph: true,
                last_in_paragraph: true,
            }],
            fonts: Vec::new(),
            diagnostics: Vec::new(),
            writing_mode: WritingMode::HorizontalTb,
            options: test_options(),
        };
        assert_eq!(
            empty_horizontal
                .caret_rect(0, Affinity::Downstream)
                .unwrap()
                .as_26_6(),
            (320, 640, 1, 192)
        );
        assert!(
            empty_horizontal
                .caret_rect(1, Affinity::Downstream)
                .is_none()
        );
        let mut empty_vertical = empty_horizontal;
        empty_vertical.lines[0].writing_mode = WritingMode::VerticalRl;
        assert_eq!(
            empty_vertical
                .caret_rect(0, Affinity::Downstream)
                .unwrap()
                .as_26_6(),
            (128, 640, 192, 1)
        );
    }

    #[test]
    fn affinity_round_trips_wrapped_and_bidi_visual_edges_and_empty_lines() {
        let mut top = glyph(WritingMode::HorizontalTb);
        top.annotation = None;
        top.source_range = 0..1;
        top.x = 0;
        top.y = 64;
        top.advance_x = 64;
        top.font_size = 64;
        top.bidi_level = 0;
        let mut bottom = top.clone();
        bottom.source_range = 1..2;
        bottom.y = 192;
        let top_glyphs = vec![top];
        let bottom_glyphs = vec![bottom];
        let layout = TextLayout {
            source: "ab\n".into(),
            lines: vec![
                TextLine {
                    range: 0..1,
                    origin: Point::from_fixed(0, 0),
                    inline_extent: 64,
                    block_extent: 64,
                    writing_mode: WritingMode::HorizontalTb,
                    hit_bounds: TextLine::hit_bounds_for(&top_glyphs),
                    glyphs: top_glyphs,
                    index: 0,
                    paragraph_index: 0,
                    first_in_paragraph: true,
                    last_in_paragraph: true,
                },
                TextLine {
                    range: 1..2,
                    origin: Point::from_fixed(0, 128),
                    inline_extent: 64,
                    block_extent: 64,
                    writing_mode: WritingMode::HorizontalTb,
                    hit_bounds: TextLine::hit_bounds_for(&bottom_glyphs),
                    glyphs: bottom_glyphs,
                    index: 0,
                    paragraph_index: 0,
                    first_in_paragraph: true,
                    last_in_paragraph: true,
                },
                TextLine {
                    range: 3..3,
                    origin: Point::from_fixed(0, 256),
                    inline_extent: 0,
                    block_extent: 64,
                    writing_mode: WritingMode::HorizontalTb,
                    hit_bounds: None,
                    glyphs: Vec::new(),
                    index: 0,
                    paragraph_index: 0,
                    first_in_paragraph: true,
                    last_in_paragraph: true,
                },
            ],
            fonts: Vec::new(),
            diagnostics: Vec::new(),
            writing_mode: WritingMode::HorizontalTb,
            options: test_options(),
        };

        let wrapped_end = layout.hit_test(Point::from_fixed(48, 32));
        assert_eq!(
            (wrapped_end.byte_offset(), wrapped_end.affinity()),
            (1, Affinity::Downstream)
        );
        assert_eq!(
            layout
                .caret_rect(wrapped_end.byte_offset(), wrapped_end.affinity())
                .unwrap()
                .as_26_6(),
            (64, 0, 1, 64)
        );
        let wrapped_start = layout.hit_test(Point::from_fixed(16, 160));
        assert_eq!(
            (wrapped_start.byte_offset(), wrapped_start.affinity()),
            (1, Affinity::Upstream)
        );
        assert_eq!(
            layout
                .caret_rect(wrapped_start.byte_offset(), wrapped_start.affinity())
                .unwrap()
                .as_26_6(),
            (0, 128, 1, 64)
        );
        let empty = layout.hit_test(Point::from_fixed(0, 280));
        assert_eq!(empty.byte_offset(), 3);
        assert_eq!(empty.affinity(), Affinity::Downstream);
        assert_eq!(
            layout
                .caret_rect(empty.byte_offset(), empty.affinity())
                .unwrap()
                .as_26_6(),
            (0, 256, 1, 64)
        );

        let mut visual = Vec::new();
        for (position, (range, level)) in [(0..1, 0), (3..4, 1), (2..3, 1), (1..2, 1), (4..5, 0)]
            .into_iter()
            .enumerate()
        {
            let mut placed = glyph(WritingMode::HorizontalTb);
            placed.annotation = None;
            placed.source_range = range;
            placed.x = i32::try_from(position).unwrap().saturating_mul(64);
            placed.y = 64;
            placed.advance_x = 64;
            placed.font_size = 64;
            placed.bidi_level = level;
            visual.push(placed);
        }
        let bidi_line = TextLine {
            range: 0..5,
            origin: Point::from_fixed(0, 0),
            inline_extent: 320,
            block_extent: 64,
            writing_mode: WritingMode::HorizontalTb,
            hit_bounds: TextLine::hit_bounds_for(&visual),
            glyphs: visual,
            index: 0,
            paragraph_index: 0,
            first_in_paragraph: true,
            last_in_paragraph: true,
        };
        let bidi = TextLayout {
            source: "abcde".into(),
            lines: vec![bidi_line],
            fonts: Vec::new(),
            diagnostics: Vec::new(),
            writing_mode: WritingMode::HorizontalTb,
            options: test_options(),
        };
        for glyph in bidi.glyphs() {
            let bounds = glyph.cell_bounds();
            for (x, expected) in [
                (bounds.x.saturating_add(16), bounds.x),
                (
                    bounds.x.saturating_add(48),
                    bounds.x.saturating_add(bounds.width),
                ),
            ] {
                let hit = bidi.hit_test(Point::from_fixed(x, 32));
                assert_eq!(
                    bidi.caret_rect(hit.byte_offset(), hit.affinity())
                        .unwrap()
                        .as_26_6()
                        .0,
                    expected
                );
            }
        }
    }

    #[test]
    fn bidi_selection_returns_only_visually_contiguous_runs() {
        let mut visual = Vec::new();
        for (position, range) in [0..1, 3..4, 2..3, 1..2, 4..5].into_iter().enumerate() {
            let mut placed = glyph(WritingMode::HorizontalTb);
            placed.annotation = None;
            placed.source_range = range;
            placed.x = i32::try_from(position).unwrap().saturating_mul(64);
            placed.y = 64;
            placed.advance_x = 64;
            placed.font_size = 64;
            visual.push(placed);
        }
        let line = TextLine {
            range: 0..5,
            origin: Point::from_fixed(0, 0),
            inline_extent: 320,
            block_extent: 64,
            writing_mode: WritingMode::HorizontalTb,
            hit_bounds: TextLine::hit_bounds_for(&visual),
            glyphs: visual,
            index: 0,
            paragraph_index: 0,
            first_in_paragraph: true,
            last_in_paragraph: true,
        };
        let layout = TextLayout {
            source: "abcde".into(),
            lines: vec![line],
            fonts: Vec::new(),
            diagnostics: Vec::new(),
            writing_mode: WritingMode::HorizontalTb,
            options: test_options(),
        };

        assert_eq!(
            layout
                .selection_rects(0..2)
                .iter()
                .map(|rect| rect.as_26_6())
                .collect::<Vec<_>>(),
            [(0, 0, 64, 64), (192, 0, 64, 64)]
        );
        assert_eq!(
            layout.selection_rects(1..4),
            [Rect::from_fixed(64, 0, 192, 64)]
        );
        assert_eq!(
            layout.selection_rects(0..5),
            [Rect::from_fixed(0, 0, 320, 64)]
        );
    }

    /// Three stacked lines of two glyphs each: bytes 0..2, 2..4, 4..6.
    fn stacked_layout() -> TextLayout {
        let mut lines = Vec::new();
        for (index, (range, y)) in [(0..2, 0_i32), (2..4, 64), (4..6, 128)]
            .into_iter()
            .enumerate()
        {
            let mut left = glyph(WritingMode::HorizontalTb);
            left.annotation = None;
            left.source_range = range.start..range.start.saturating_add(1);
            left.x = 0;
            // The pipeline places a glyph at the line's block origin plus its
            // own size, so the cell sits inside the line's own band.
            left.y = y.saturating_add(64);
            left.advance_x = 64;
            left.font_size = 64;
            left.bidi_level = 0;
            let mut right = left.clone();
            right.source_range = range.start.saturating_add(1)..range.end;
            right.x = 64;
            let glyphs = vec![left, right];
            lines.push(TextLine {
                range,
                origin: Point::from_fixed(-32, y),
                inline_extent: 192,
                block_extent: 64,
                writing_mode: WritingMode::HorizontalTb,
                hit_bounds: TextLine::hit_bounds_for(&glyphs),
                glyphs,
                index,
                paragraph_index: 0,
                first_in_paragraph: index == 0,
                last_in_paragraph: index == 2,
            });
        }
        TextLayout {
            source: "abcdef".to_owned(),
            lines,
            fonts: Vec::new(),
            diagnostics: Vec::new(),
            writing_mode: WritingMode::HorizontalTb,
            options: test_options(),
        }
    }

    /// Three vertical columns of two glyphs each, contiguous and progressing
    /// right to left the way `WritingMode::VerticalRl` composes them.
    fn stacked_vertical_layout() -> TextLayout {
        let mut lines = Vec::new();
        for (index, (range, block_origin)) in [(0..2, 0_i32), (2..4, -64), (4..6, -128)]
            .into_iter()
            .enumerate()
        {
            let mut first = glyph(WritingMode::VerticalRl);
            first.annotation = None;
            first.source_range = range.start..range.start.saturating_add(1);
            // A vertical cell spans `x - font_size ..= x`, so the glyph sits at
            // the column's own trailing edge.
            first.x = block_origin;
            first.y = 0;
            first.advance_x = 0;
            first.advance_y = 64;
            first.font_size = 64;
            first.bidi_level = 0;
            first.transform = GlyphTransform::Identity;
            let mut second = first.clone();
            second.source_range = range.start.saturating_add(1)..range.end;
            second.y = 64;
            let glyphs = vec![first, second];
            lines.push(TextLine {
                range,
                origin: Point::from_fixed(block_origin, 0),
                inline_extent: 128,
                block_extent: 64,
                writing_mode: WritingMode::VerticalRl,
                hit_bounds: None,
                glyphs,
                index,
                paragraph_index: 0,
                first_in_paragraph: index == 0,
                last_in_paragraph: index == 2,
            });
        }
        TextLayout {
            source: "abcdef".to_owned(),
            lines,
            fonts: Vec::new(),
            diagnostics: Vec::new(),
            writing_mode: WritingMode::VerticalRl,
            options: test_options(),
        }
    }

    #[test]
    fn vertical_line_geometry_probes_the_column_it_targets() {
        let layout = stacked_vertical_layout();
        let bounds: Vec<_> = layout
            .lines
            .iter()
            .map(|line| line.bounds().as_26_6())
            .collect();
        assert_eq!(bounds[0].0.saturating_add(bounds[0].2), 0);
        assert_eq!(bounds[1].0.saturating_add(bounds[1].2), bounds[0].0);

        // Attribution probes a rectangle's own middle on the block axis, so a
        // full column resolves to itself while its trailing edge — shared with
        // the previous column — resolves to the earlier one.
        assert_eq!(
            layout.line_index_for_rect(Rect::from_fixed(bounds[1].0, 0, bounds[1].2, 1)),
            Some(1)
        );
        assert_eq!(
            layout.line_index_for_rect(Rect::from_fixed(bounds[0].0, 0, 0, 1)),
            Some(0),
            "the shared column edge belongs to the earlier column"
        );
        // A rectangle straddling two columns is decided by its middle, not by
        // the column its leading edge happens to start in.
        assert_eq!(
            layout.line_index_for_rect(Rect::from_fixed(bounds[2].0.saturating_add(32), 0, 64, 1)),
            Some(1),
            "the middle lands on the shared edge, which the earlier column wins"
        );

        // Column-to-column motion probes the middle of the target column, so
        // it lands there rather than on its edge or past it.
        let next = layout
            .caret_next_line(0, Affinity::Upstream)
            .expect("a following column exists");
        assert_eq!(layout.line_index_at(next.byte_offset()), Some(1));
        let further = layout
            .caret_next_line(next.byte_offset(), next.affinity())
            .expect("a third column exists");
        assert_eq!(layout.line_index_at(further.byte_offset()), Some(2));
        let back = layout
            .caret_previous_line(further.byte_offset(), further.affinity())
            .expect("a preceding column exists");
        assert_eq!(layout.line_index_at(back.byte_offset()), Some(1));
    }

    #[test]
    fn line_geometry_queries_pick_the_line_the_caret_is_actually_on() {
        let layout = stacked_layout();

        // A caret rect is attributed to the line that holds it, not to a
        // neighbor: the midpoint of the rect must stay inside its own line.
        // Affinity picks the side, so the first position is upstream-only and
        // the last downstream-only; a wrap boundary resolves to the line the
        // chosen side belongs to.
        for (offset, affinity, expected) in [
            (0_usize, Affinity::Upstream, 0_usize),
            (2, Affinity::Upstream, 1),
            (2, Affinity::Downstream, 0),
            (4, Affinity::Upstream, 2),
            (4, Affinity::Downstream, 1),
            (6, Affinity::Downstream, 2),
        ] {
            let rect = layout
                .caret_rect(offset, affinity)
                .unwrap_or_else(|| panic!("offset {offset} {affinity:?} has no caret"));
            assert_eq!(
                layout.line_index_for_rect(rect),
                Some(expected),
                "caret at {offset} ({affinity:?}) belongs to line {expected}"
            );
        }

        // Line-to-line motion lands on the adjacent line, never skipping one
        // or collapsing onto the same line.
        let start = layout
            .caret_next_line(0, Affinity::Upstream)
            .expect("a following line exists");
        assert_eq!(layout.line_index_at(start.byte_offset()), Some(1));
        let further = layout
            .caret_next_line(start.byte_offset(), start.affinity())
            .expect("a third line exists");
        assert_eq!(layout.line_index_at(further.byte_offset()), Some(2));
        assert_eq!(
            layout.caret_next_line(further.byte_offset(), further.affinity()),
            None,
            "the final line has no following line"
        );
        let back = layout
            .caret_previous_line(further.byte_offset(), further.affinity())
            .expect("a preceding line exists");
        assert_eq!(layout.line_index_at(back.byte_offset()), Some(1));

        // Composed lines are contiguous, so attribution has to probe a
        // rectangle's middle: its own top edge is equally close to the line
        // above, and the earlier line wins a tie. That keeps a caret from
        // drifting as it is re-resolved.
        assert_eq!(
            layout.line_index_for_rect(Rect::from_fixed(0, 64, 1, 64)),
            Some(1),
            "a rectangle filling the second line belongs to it"
        );
        assert_eq!(
            layout.line_index_for_rect(Rect::from_fixed(0, 64, 1, 0)),
            Some(0),
            "a rectangle on the shared edge belongs to the earlier line"
        );
        assert_eq!(
            layout.line_index_for_rect(Rect::from_fixed(0, 0, 1, 64)),
            Some(0)
        );
        assert_eq!(
            layout.line_index_for_rect(Rect::from_fixed(0, 128, 1, 64)),
            Some(2)
        );
    }

    #[test]
    fn filled_selection_extends_only_the_sides_the_selection_continues_past() {
        let layout = stacked_layout();

        // A selection confined to one line touches exactly that line, and
        // needs no extension, so it equals the exact glyph-cell union.
        let middle = layout.selection_rects_filled(2..4);
        assert_eq!(middle.len(), 1);
        let exact_middle = layout
            .selection_rects(2..4)
            .into_iter()
            .reduce(Rect::union)
            .expect("the middle line is selected");
        assert_eq!(middle[0].as_26_6(), exact_middle.as_26_6());

        // Spanning three lines fills the interior line edge to edge, extends
        // the first line only forward, and the last line only backward.
        let spanning = layout.selection_rects_filled(1..5);
        assert_eq!(spanning.len(), 3);
        let bounds: Vec<_> = layout
            .lines
            .iter()
            .map(|line| line.bounds().as_26_6())
            .collect();
        assert_eq!(
            spanning[0].as_26_6().0,
            64,
            "the first line keeps its own leading edge"
        );
        assert_eq!(
            spanning[0]
                .as_26_6()
                .0
                .saturating_add(spanning[0].as_26_6().2),
            bounds[0].0.saturating_add(bounds[0].2),
            "the first line fills to its trailing edge"
        );
        assert_eq!(spanning[1].as_26_6().0, bounds[1].0);
        assert_eq!(
            spanning[1]
                .as_26_6()
                .0
                .saturating_add(spanning[1].as_26_6().2),
            bounds[1].0.saturating_add(bounds[1].2),
            "the interior line fills both edges"
        );
        assert_eq!(spanning[2].as_26_6().0, bounds[2].0);
        assert_eq!(
            spanning[2]
                .as_26_6()
                .0
                .saturating_add(spanning[2].as_26_6().2),
            64,
            "the last line keeps its own trailing edge"
        );

        // A selection that starts exactly where a line starts, or ends
        // exactly where one ends, extends nothing: only continuing past the
        // line reaches its layout edge, which is wider than its glyphs here.
        let bounds_of_middle = layout.lines[1].bounds().as_26_6();
        assert!(
            bounds_of_middle.0 < 0,
            "the line cell reaches past its glyphs"
        );
        let flush_start = layout.selection_rects_filled(2..3);
        assert_eq!(flush_start.len(), 1);
        assert_eq!(
            flush_start[0].as_26_6().0,
            0,
            "a selection flush with the line start keeps the glyph edge"
        );
        let flush_end = layout.selection_rects_filled(3..4);
        assert_eq!(flush_end.len(), 1);
        assert_eq!(
            flush_end[0]
                .as_26_6()
                .0
                .saturating_add(flush_end[0].as_26_6().2),
            128,
            "a selection flush with the line end keeps the glyph edge"
        );
        assert!(layout.selection_rects_filled(3..3).is_empty());
    }

    #[test]
    fn indexed_result_queries_match_full_linear_scans_across_lines() {
        let mut lines = Vec::new();
        for (range, y) in [(0..2, 64), (2..4, 192), (4..6, 320)] {
            let mut left = glyph(WritingMode::HorizontalTb);
            left.annotation = None;
            left.source_range = range.start..range.start.saturating_add(1);
            left.x = 0;
            left.y = y;
            left.advance_x = 64;
            left.font_size = 64;
            let mut right = left.clone();
            right.source_range = range.start.saturating_add(1)..range.end;
            right.x = 64;
            right.bidi_level = u8::from(range.start == 2);
            let glyphs = vec![left, right];
            lines.push(TextLine {
                range,
                origin: Point::from_fixed(0, y),
                inline_extent: 128,
                block_extent: 64,
                writing_mode: WritingMode::HorizontalTb,
                hit_bounds: TextLine::hit_bounds_for(&glyphs),
                glyphs,
                index: 0,
                paragraph_index: 0,
                first_in_paragraph: true,
                last_in_paragraph: true,
            });
        }
        let layout = TextLayout {
            source: "abcdef".to_owned(),
            lines,
            fonts: Vec::new(),
            diagnostics: Vec::new(),
            writing_mode: WritingMode::HorizontalTb,
            options: test_options(),
        };

        for x in [-32, 0, 63, 64, 96, 160] {
            for y in [0, 64, 128, 192, 256, 320, 384] {
                let point = Point::from_fixed(x, y);
                assert_eq!(layout.hit_test(point), linear_hit_test(&layout, point));
            }
        }
        for offset in 0..=layout.source.len() {
            for affinity in [Affinity::Upstream, Affinity::Downstream] {
                assert_eq!(
                    layout.caret_rect(offset, affinity),
                    linear_caret_rect(&layout, offset, affinity)
                );
            }
        }
        for range in [0..1, 0..2, 1..3, 2..4, 3..6, 5..6] {
            assert_eq!(
                layout.selection_rects(range.clone()),
                linear_selection_rects(&layout, range)
            );
        }
    }
}
