// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::ops::Range;

use crate::units::{finite, quantize, to_f32};
use crate::{FontId, FontResource, LayoutError, OptionKind, WritingMode};

/// Physical point represented internally in deterministic 26.6 fixed point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Point {
    x: i32,
    y: i32,
}

impl Point {
    /// Validate and quantize a physical point.
    pub fn try_new(x: f32, y: f32) -> Result<Self, LayoutError> {
        let x = finite(x, OptionKind::ConstructGeometry)?;
        let y = finite(y, OptionKind::ConstructGeometry)?;
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
    pub(crate) transform: GlyphTransform,
    pub(crate) bidi_level: u8,
    pub(crate) writing_mode: WritingMode,
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

    /// Alias for [`source_range`](Self::source_range).
    #[must_use]
    pub fn range(&self) -> Range<usize> {
        self.source_range()
    }

    /// Annotation-local attribution, if this glyph came from ruby or another attachment.
    #[must_use]
    pub const fn annotation(&self) -> Option<&AnnotationSource> {
        self.annotation.as_ref()
    }

    /// Physical glyph origin.
    #[must_use]
    pub const fn origin(&self) -> Point {
        Point::from_fixed(self.x, self.y)
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

    fn bounds(&self) -> Rect {
        match self.writing_mode {
            WritingMode::HorizontalTb => {
                let width = self.advance_x.abs().max(1);
                Rect::from_fixed(
                    self.x,
                    self.y.saturating_sub(self.font_size),
                    width,
                    self.font_size,
                )
            },
            WritingMode::VerticalRl => {
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
}

impl TextLine {
    /// Original document bytes assigned to this line.
    #[must_use]
    pub fn range(&self) -> Range<usize> {
        self.range.clone()
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
}

impl TextLayout {
    /// Original unnormalized UTF-8 text.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
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

    /// Recoverable issues in source order.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Iterate every glyph in global visual draw order.
    pub fn glyphs(&self) -> impl Iterator<Item = &GlyphPlacement> {
        self.lines.iter().flat_map(|line| line.glyphs.iter())
    }

    /// Map a physical point to the nearest logical UTF-8 boundary.
    #[must_use]
    pub fn hit_test(&self, point: Point) -> HitTest {
        let mut best: Option<(&GlyphPlacement, i64, bool)> = None;
        for glyph in self.glyphs().filter(|glyph| glyph.annotation.is_none()) {
            let bounds = glyph.bounds();
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
                byte_offset: 0,
                affinity: Affinity::Downstream,
                inside: false,
            };
        };
        let bounds = glyph.bounds();
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

    /// Return a one-subpixel caret at the closest rendered edge of a UTF-8 boundary.
    #[must_use]
    pub fn caret_rect(&self, byte_offset: usize) -> Option<Rect> {
        if !is_valid_caret_offset(&self.source, byte_offset) {
            return None;
        }
        for line in &self.lines {
            let mut candidates = line
                .glyphs
                .iter()
                .filter(|glyph| glyph.annotation.is_none())
                .filter(|glyph| glyph_touches_boundary(&glyph.source_range, byte_offset));
            if let Some(glyph) = candidates.next() {
                let bounds = glyph.bounds();
                let at_start = glyph.source_range.start == byte_offset;
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
            if line.range.start == byte_offset && line.glyphs.is_empty() {
                return Some(match line.writing_mode {
                    WritingMode::HorizontalTb => Rect::from_fixed(
                        line.origin.x,
                        line.origin.y.saturating_sub(line.block_extent),
                        1,
                        line.block_extent,
                    ),
                    WritingMode::VerticalRl => Rect::from_fixed(
                        line.origin.x.saturating_sub(line.block_extent),
                        line.origin.y,
                        line.block_extent,
                        1,
                    ),
                });
            }
        }
        None
    }

    /// Return one merged selection rectangle per touched line.
    #[must_use]
    pub fn selection_rects(&self, range: Range<usize>) -> Vec<Rect> {
        if !is_valid_selection_range(&self.source, &range) {
            return Vec::new();
        }
        let mut result = Vec::new();
        for line in &self.lines {
            let mut boxes = line
                .glyphs
                .iter()
                .filter(|glyph| glyph.annotation.is_none())
                .filter(|glyph| ranges_overlap(&glyph.source_range, &range))
                .map(GlyphPlacement::bounds);
            let Some(first) = boxes.next() else {
                continue;
            };
            let mut min_x = first.x;
            let mut min_y = first.y;
            let mut max_x = first.x.saturating_add(first.width);
            let mut max_y = first.y.saturating_add(first.height);
            for rect in boxes {
                min_x = min_x.min(rect.x);
                min_y = min_y.min(rect.y);
                max_x = max_x.max(rect.x.saturating_add(rect.width));
                max_y = max_y.max(rect.y.saturating_add(rect.height));
            }
            result.push(Rect::from_fixed(
                min_x,
                min_y,
                max_x.saturating_sub(min_x),
                max_y.saturating_sub(min_y),
            ));
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

fn glyph_touches_boundary(range: &Range<usize>, offset: usize) -> bool {
    range.start == offset || range.end == offset
}

fn is_visual_end(at_start: bool, bidi_level: u8) -> bool {
    at_start == (bidi_level % 2 == 1)
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
            transform: GlyphTransform::RotateClockwise,
            bidi_level: 2,
            writing_mode: mode,
        }
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
        assert!(glyph_touches_boundary(&(2..5), 2));
        assert!(glyph_touches_boundary(&(2..5), 5));
        assert!(!glyph_touches_boundary(&(2..5), 3));
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
        assert_eq!(horizontal.range(), 2..5);
        let annotation = horizontal.annotation().unwrap();
        assert_eq!(annotation.construct(), 3);
        assert_eq!(annotation.range(), 7..11);
        assert_eq!(horizontal.origin().x_26_6(), 128);
        assert_eq!(horizontal.origin().y_26_6(), 320);
        assert_float(horizontal.x(), 2.0);
        assert_float(horizontal.y(), 5.0);
        assert_float(horizontal.advance_x(), 4.0);
        assert_float(horizontal.advance_y(), -6.0);
        assert_float(horizontal.offset_x(), 1.5);
        assert_float(horizontal.offset_y(), -2.0);
        assert_eq!(horizontal.geometry_26_6(), (128, 320, 256, -384, 96, -128));
        assert_eq!(horizontal.transform(), GlyphTransform::RotateClockwise);
        assert_eq!(horizontal.bidi_level(), 2);
        assert_eq!(horizontal.bounds().as_26_6(), (128, 128, 256, 192));

        let vertical = glyph(WritingMode::VerticalRl);
        assert_eq!(vertical.bounds().as_26_6(), (-64, 320, 192, 384));
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
        let line = TextLine {
            range: 2..5,
            origin: Point::from_fixed(32, 64),
            inline_extent: 448,
            block_extent: 192,
            writing_mode: WritingMode::HorizontalTb,
            glyphs: vec![rendered],
        };
        assert_eq!(line.range(), 2..5);
        assert_eq!(line.origin().x_26_6(), 32);
        assert_eq!(line.origin().y_26_6(), 64);
        assert_float(line.inline_extent(), 7.0);
        assert_float(line.block_extent(), 3.0);
        assert_eq!(line.writing_mode(), WritingMode::HorizontalTb);
        assert_eq!(line.glyphs().len(), 1);

        let layout = TextLayout {
            source: "abXYZq".to_owned(),
            lines: vec![line],
            fonts: Vec::new(),
            diagnostics: vec![diagnostic],
            writing_mode: WritingMode::HorizontalTb,
        };
        let before = layout.hit_test(Point::from_fixed(160, 200));
        assert_eq!(before.byte_offset(), 2);
        assert_eq!(before.affinity(), Affinity::Upstream);
        assert!(before.is_inside());
        let after = layout.hit_test(Point::from_fixed(352, 200));
        assert_eq!(after.byte_offset(), 5);
        assert_eq!(after.affinity(), Affinity::Downstream);
        assert!(after.is_inside());
        assert_eq!(layout.caret_rect(2).unwrap().as_26_6(), (128, 128, 1, 192));
        assert_eq!(layout.caret_rect(5).unwrap().as_26_6(), (384, 128, 1, 192));
        assert_eq!(
            layout.selection_rects(2..5)[0].as_26_6(),
            (128, 128, 256, 192)
        );
        assert!(layout.selection_rects(0..2).is_empty());
        assert!(layout.selection_rects(5..6).is_empty());

        let mut rtl = layout.clone();
        rtl.lines[0].glyphs[0].bidi_level = 1;
        assert_eq!(rtl.caret_rect(2).unwrap().as_26_6(), (384, 128, 1, 192));
        assert_eq!(rtl.caret_rect(5).unwrap().as_26_6(), (128, 128, 1, 192));

        let empty_horizontal = TextLayout {
            source: "A".into(),
            lines: vec![TextLine {
                range: 0..0,
                origin: Point::from_fixed(320, 640),
                inline_extent: 0,
                block_extent: 192,
                writing_mode: WritingMode::HorizontalTb,
                glyphs: Vec::new(),
            }],
            fonts: Vec::new(),
            diagnostics: Vec::new(),
            writing_mode: WritingMode::HorizontalTb,
        };
        assert_eq!(
            empty_horizontal.caret_rect(0).unwrap().as_26_6(),
            (320, 448, 1, 192)
        );
        assert!(empty_horizontal.caret_rect(1).is_none());
        let mut empty_vertical = empty_horizontal;
        empty_vertical.lines[0].writing_mode = WritingMode::VerticalRl;
        assert_eq!(
            empty_vertical.caret_rect(0).unwrap().as_26_6(),
            (128, 640, 192, 1)
        );
    }
}
