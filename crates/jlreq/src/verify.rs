// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! What must be true of the physical coordinates this crate returns.
//!
//! [`jlreq_core::verify`] answers the same question one layer down, about the
//! composer's logical layout. This one is about the geometry a renderer is
//! handed: whether the cells a glyph is drawn into, the cells hit testing
//! measures against, and the cells carets and selections are cut from are the
//! same cells.
//!
//! It exists because nothing else in this workspace compared one physical
//! answer against another. The trace channel records the composer's reasoning
//! and the core's checker holds the logical layout together, but every
//! rectangle the facade derives from them was only ever returned. Three
//! defects lived in that gap: a class boundary's shared conditional space was
//! counted twice; the two halves of a tate-chu-yoko run — which share one
//! inline coordinate — were advanced past each other; and that run's members
//! were mapped onto the page from their own upright orientation rather than
//! the paragraph's, which placed them clear of the column. All three moved
//! drawn text away from the layout that hit testing was still using.
//!
//! It reports rather than panics, for the reason [`jlreq_core::verify`] gives:
//! an invariant is a typed value the caller decides what to do with. A test
//! asserts on the report, a fuzz target prints it, an editor could refuse to
//! draw and say why.
//!
//! ```no_run
//! # fn check(layout: &jlreq::TextLayout) {
//! let report = jlreq::verify::inspect(layout);
//! assert!(report.is_sound(), "{report}");
//! # }
//! ```
//!
//! The coordinate system these statements are about is written down in
//! `docs/design/geometry.md`.
//!
//! Cost: the structural statements are linear in glyphs, but the two
//! interaction statements ask [`TextLayout::hit_test`] and
//! [`TextLayout::caret_rect`] once per glyph and per line edge, and each of
//! those scans the layout. Checking a very large layout is therefore quadratic.
//! It is a diagnostic, not a step in laying text out.

use core::fmt;
use std::ops::Range;

use crate::result::{GlyphPlacement, Rect, TextLayout, TextLine};
use crate::{Affinity, Point, WritingMode};

/// One way a layout's geometry failed to agree with itself.
///
/// Every variant that belongs to a line names it, so a report reads as a list
/// of places rather than a list of complaints.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Fault {
    /// A body glyph's [`GlyphPlacement::cell_bounds`] leaves its line along the
    /// **block** axis — above or below the line in horizontal writing, left or
    /// right of the column in vertical.
    ///
    /// This is the axis a line does not negotiate: its block extent is what the
    /// composer reserved, and the next line begins where it ends, so a cell
    /// past that edge is a cell drawn onto a neighbour. The measure is the
    /// other axis and has its own statement, because it has exemptions this
    /// one does not.
    ///
    /// Compared against the line's own composed box, never against
    /// [`TextLine::bounds`](crate::TextLine::bounds): `bounds` is defined as
    /// the union of the very cells it would be asked about, so a check against
    /// it can have no witness, which `docs/design/invariants.md` rules out.
    CellEscapesItsLine {
        /// The line ordinal.
        line: usize,
        /// The bytes the glyph is attributed to.
        range: Range<usize>,
    },
    /// A body glyph sits outside its line's own measure — the **inline** axis —
    /// and it is neither the line's last cell nor covered by a
    /// `layout.overfull` diagnostic.
    ///
    /// Two things legitimately pass the measure. A line may hold more than it
    /// fits, which the diagnostic reports; and the last cell may hang past it,
    /// which is JLReq's `ぶら下げ` and is why
    /// [`TextLine::inline_extent`](crate::TextLine::inline_extent) excludes it.
    /// Neither excuses an *interior* cell, which is a line silently holding
    /// more than it says it does.
    CellEscapesTheMeasureSilently {
        /// The line ordinal.
        line: usize,
        /// The bytes the glyph is attributed to.
        range: Range<usize>,
    },
    /// An annotation's cell overlaps the body text it annotates on the block
    /// axis, instead of standing beside it.
    AnnotationOverlapsItsBase {
        /// The line ordinal.
        line: usize,
        /// The bytes the annotation is attributed to.
        range: Range<usize>,
    },
    /// Two consecutive lines occupy the same block-axis coordinates.
    LinesOverlap {
        /// The later line.
        line: usize,
    },
    /// Lines changed the direction they progress in partway down the layout.
    BlockProgressionReverses {
        /// The line whose origin broke the run.
        line: usize,
        /// The direction the lines before it established, as a sign.
        established: i32,
        /// The step it took instead.
        observed: i32,
    },
    /// The first line does not start at the beginning of the source.
    CoverageStartsLate {
        /// Where the first line starts.
        observed: usize,
    },
    /// The last line does not reach the end of the source.
    CoverageEndsEarly {
        /// Where the last line ends.
        observed: usize,
        /// Where the source ends.
        source: usize,
    },
    /// Two consecutive lines leave a gap in the source or claim it twice.
    LinesDoNotMeet {
        /// The later line.
        line: usize,
        /// Where the previous line ended.
        previous_end: usize,
        /// Where this one starts.
        observed_start: usize,
    },
    /// A caret rectangle stands on none of the layout's lines.
    ///
    /// It is not required to stand on the line that holds its offset: at a wrap
    /// the two [`Affinity`] answers deliberately name different lines. What must
    /// hold is that it stands on one of them — and that there is one at all,
    /// since every offset asked is a line edge the layout named itself.
    ///
    /// The weaker of this module's statements, and deliberately so.
    /// [`TextLayout::caret_rect`](crate::TextLayout::caret_rect) builds the
    /// caret from a cell of the line it found it on, and a line's `bounds`
    /// contain its cells, so today only the missing-caret half can fail. It is
    /// kept rather than reduced to a unit test because the two halves are
    /// reached through `caret_rect`'s own search, affinity and bidi handling
    /// rather than through the fields it is compared against — a caret that
    /// stopped being derived from a cell would be caught here and nowhere else.
    CaretStandsOnNoLine {
        /// The byte offset asked for.
        offset: usize,
        /// Whether the caret was asked for upstream of that offset.
        upstream: bool,
    },
    /// A hit test in the middle of a glyph's own cell answered with bytes
    /// outside that glyph.
    HitTestMissesItsOwnCell {
        /// The line ordinal.
        line: usize,
        /// The bytes the glyph is attributed to.
        range: Range<usize>,
        /// The offset the hit test answered with.
        answered: usize,
    },
}

impl Fault {
    /// The stable name of this kind of fault.
    ///
    /// There is no wildcard arm: a new [`Fault`] does not compile until it is
    /// named here. That is the frozen-projection requirement of
    /// [ADR 0012](https://github.com/P4suta/jlreq) met by the compiler rather
    /// than by review, the same way [`jlreq_core::verify::Fault`] and
    /// [`jlreq_core::trace::Fact`] meet it.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match *self {
            Self::CellEscapesItsLine { .. } => "cell-escapes-its-line",
            Self::CellEscapesTheMeasureSilently { .. } => "cell-escapes-the-measure-silently",
            Self::AnnotationOverlapsItsBase { .. } => "annotation-overlaps-its-base",
            Self::LinesOverlap { .. } => "lines-overlap",
            Self::BlockProgressionReverses { .. } => "block-progression-reverses",
            Self::CoverageStartsLate { .. } => "coverage-starts-late",
            Self::CoverageEndsEarly { .. } => "coverage-ends-early",
            Self::LinesDoNotMeet { .. } => "lines-do-not-meet",
            Self::CaretStandsOnNoLine { .. } => "caret-stands-on-no-line",
            Self::HitTestMissesItsOwnCell { .. } => "hit-test-misses-its-own-cell",
        }
    }

    /// The line this fault belongs to, where one does.
    ///
    /// There is no wildcard arm, for the same reason [`Fault::kind`] has none.
    #[must_use]
    pub const fn line(&self) -> Option<usize> {
        match *self {
            Self::LinesOverlap { line }
            | Self::BlockProgressionReverses { line, .. }
            | Self::LinesDoNotMeet { line, .. }
            | Self::CellEscapesItsLine { line, .. }
            | Self::CellEscapesTheMeasureSilently { line, .. }
            | Self::AnnotationOverlapsItsBase { line, .. }
            | Self::HitTestMissesItsOwnCell { line, .. } => Some(line),
            Self::CoverageStartsLate { .. }
            | Self::CoverageEndsEarly { .. }
            | Self::CaretStandsOnNoLine { .. } => None,
        }
    }
}

impl fmt::Display for Fault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.kind())?;
        if let Some(line) = self.line() {
            write!(formatter, " on line {line}")?;
        }
        match self {
            Self::CellEscapesItsLine { range, .. }
            | Self::CellEscapesTheMeasureSilently { range, .. }
            | Self::AnnotationOverlapsItsBase { range, .. } => {
                write!(formatter, " at bytes {}..{}", range.start, range.end)
            },
            Self::HitTestMissesItsOwnCell {
                range, answered, ..
            } => write!(
                formatter,
                " at bytes {}..{}, answered {answered}",
                range.start, range.end
            ),
            Self::CaretStandsOnNoLine { offset, upstream } => {
                let affinity = if *upstream { "upstream" } else { "downstream" };
                write!(formatter, " at offset {offset} ({affinity})")
            },
            Self::BlockProgressionReverses {
                established,
                observed,
                ..
            } => write!(
                formatter,
                ": established {established}, observed {observed}"
            ),
            Self::CoverageStartsLate { observed } => write!(formatter, " at {observed}"),
            Self::CoverageEndsEarly { observed, source } => {
                write!(formatter, ": ended {observed} of {source}")
            },
            Self::LinesDoNotMeet {
                previous_end,
                observed_start,
                ..
            } => write!(formatter, ": {previous_end} then {observed_start}"),
            Self::LinesOverlap { .. } => Ok(()),
        }
    }
}

/// Everything that was not true of one layout's geometry.
///
/// An empty report is the answer a caller wants, and [`Report::is_sound`] says
/// so without making them count.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct Report {
    faults: Vec<Fault>,
}

impl Report {
    /// Whether the layout satisfied every invariant this release checks.
    #[must_use]
    pub fn is_sound(&self) -> bool {
        self.faults.is_empty()
    }

    /// The faults, in the order they were found.
    #[must_use]
    pub fn faults(&self) -> &[Fault] {
        &self.faults
    }

    fn note(&mut self, fault: Fault) {
        self.faults.push(fault);
    }
}

impl fmt::Display for Report {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.faults.is_empty() {
            return formatter.write_str("sound");
        }
        writeln!(formatter, "{} geometric fault(s):", self.faults.len())?;
        for fault in &self.faults {
            writeln!(formatter, "  {fault}")?;
        }
        Ok(())
    }
}

/// Check one layout's physical geometry against itself.
///
/// The layout is enough on its own: it owns its source, its lines, its glyphs,
/// and the diagnostics that excuse a line from its measure.
#[must_use]
pub fn inspect(layout: &TextLayout) -> Report {
    let mut report = Report::default();
    check_coverage(layout, &mut report);
    check_progression(layout, &mut report);

    let overfull: Vec<Range<usize>> = layout
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.code() == "layout.overfull")
        .filter_map(crate::Diagnostic::range)
        .collect();

    let mode = layout.writing_mode();
    let mut previous_body: Option<Rect> = None;
    for line in layout.lines() {
        let body = body_cell(line);
        check_line(line, body, mode, &overfull, &mut report);
        if let Some(previous) = previous_body
            && overlaps_block(mode, previous, body)
        {
            report.note(Fault::LinesOverlap { line: line.index() });
        }
        previous_body = Some(body);
    }

    check_carets(layout, &mut report);
    check_hit_tests(layout, &mut report);
    report
}

/// The lines partition the whole document source. The core states this one
/// paragraph at a time; only the facade can state it for the document, because
/// only the facade split the paragraphs.
fn check_coverage(layout: &TextLayout, report: &mut Report) {
    let Some(first) = layout.lines().first() else {
        return;
    };
    if first.range().start != 0 {
        report.note(Fault::CoverageStartsLate {
            observed: first.range().start,
        });
    }
    let mut previous_end = first.range().start;
    for line in layout.lines() {
        let range = line.range();
        if range.start != previous_end && !is_paragraph_separator(layout, previous_end..range.start)
        {
            report.note(Fault::LinesDoNotMeet {
                line: line.index(),
                previous_end,
                observed_start: range.start,
            });
        }
        previous_end = range.end;
    }
    let source = layout.source().len();
    if previous_end != source {
        report.note(Fault::CoverageEndsEarly {
            observed: previous_end,
            source,
        });
    }
}

/// The one thing allowed to sit between two lines and belong to neither.
///
/// The core states its coverage invariant over a single paragraph, where no
/// such byte exists. The facade split the paragraphs, so the bytes it split
/// them *at* are exactly the gap it is allowed to leave — and only those. A
/// separator dropped anywhere else is input the layout silently lost.
fn is_paragraph_separator(layout: &TextLayout, gap: Range<usize>) -> bool {
    let Some(text) = layout.source().get(gap) else {
        return false;
    };
    matches!(
        text,
        "\n" | "\r" | "\r\n" | "\u{b}" | "\u{c}" | "\u{85}" | "\u{2028}" | "\u{2029}"
    )
}

/// Once the lines have established a direction they never turn around. This is
/// deliberately the weaker of the two available statements, for the reason
/// `jlreq_core::verify` gives: no layout in the corpus witnesses two lines
/// sharing a block origin, and an invariant with no witness is a guess.
fn check_progression(layout: &TextLayout, report: &mut Report) {
    let mode = layout.writing_mode();
    let mut established = 0_i32;
    let mut previous: Option<i32> = None;
    for line in layout.lines() {
        let origin = block_origin(line, mode);
        if let Some(previous) = previous {
            let step = origin.saturating_sub(previous);
            let sign = step.signum();
            if sign != 0 {
                if established != 0 && sign != established {
                    report.note(Fault::BlockProgressionReverses {
                        line: line.index(),
                        established,
                        observed: step,
                    });
                }
                established = sign;
            }
        }
        previous = Some(origin);
    }
}

fn check_line(
    line: &TextLine,
    body: Rect,
    mode: WritingMode,
    overfull: &[Range<usize>],
    report: &mut Report,
) {
    let excused = overfull
        .iter()
        .any(|range| range.start < line.range().end && line.range().start < range.end);
    // Hanging punctuation puts one cell past the measure on purpose, and it is
    // always the last one along the inline axis. Exempting exactly that cell
    // keeps the statement about every other one.
    let hanging = line
        .glyphs()
        .iter()
        .filter(|glyph| glyph.annotation().is_none())
        .map(|glyph| inline_start(mode, glyph.cell_bounds()))
        .max();
    // The base is the text, not the line. A line's block extent is grown to
    // reserve room for its annotations, so a subscript standing correctly in
    // the room reserved for it is inside the line's box and outside every text
    // cell — asking the box rather than the text called that an overlap.
    let base = line
        .glyphs()
        .iter()
        .filter(|glyph| glyph.annotation().is_none())
        .map(GlyphPlacement::cell_bounds)
        .reduce(Rect::union);

    for glyph in line.glyphs() {
        let cell = glyph.cell_bounds();
        if glyph.annotation().is_some() {
            // An annotation is outside its base's block extent by design — that
            // is what standing beside the text means — so it is asked the one
            // question that distinguishes beside from over.
            if base.is_some_and(|base| overlaps_block(mode, base, cell)) {
                report.note(Fault::AnnotationOverlapsItsBase {
                    line: line.index(),
                    range: glyph.source_range(),
                });
            }
            continue;
        }
        // The two axes are asked separately because they have different
        // exemptions and different consequences: past the block edge is a cell
        // on a neighbouring line, past the measure is a line holding more than
        // it reports. Naming them apart is what makes a report say which.
        if !within_block(mode, body, cell) {
            report.note(Fault::CellEscapesItsLine {
                line: line.index(),
                range: glyph.source_range(),
            });
        }
        if !within_inline(mode, body, cell) && !excused && hanging != Some(inline_start(mode, cell))
        {
            report.note(Fault::CellEscapesTheMeasureSilently {
                line: line.index(),
                range: glyph.source_range(),
            });
        }
    }
}

fn check_carets(layout: &TextLayout, report: &mut Report) {
    let bounds: Vec<Rect> = layout.lines().iter().map(TextLine::bounds).collect();
    let mode = layout.writing_mode();
    let mut offsets: Vec<usize> = layout
        .lines()
        .iter()
        .flat_map(|line| [line.range().start, line.range().end])
        .collect();
    offsets.sort_unstable();
    offsets.dedup();

    for offset in offsets {
        // One affinity legitimately has no answer: nothing ends at the start of
        // the document and nothing starts at its end. Both having none is the
        // failure, and it is the same failure as a caret nowhere — an editor
        // cannot put the cursor there either — so it is reported rather than
        // skipped, which is what asking each affinity in isolation did.
        let mut placed = false;
        for (upstream, affinity) in [(false, Affinity::Downstream), (true, Affinity::Upstream)] {
            let Some(caret) = layout.caret_rect(offset, affinity) else {
                continue;
            };
            placed = true;
            // A caret marks a position and is one quantized unit thick so that
            // it can be filled. At a line's end that unit lies on the trailing
            // edge, and a blank paragraph's caret is a whole em standing on a
            // line of zero inline extent, so the thickness is taken back off
            // before asking where the position is.
            let position = caret_position(caret, mode);
            if !bounds.iter().any(|line| contains(*line, position)) {
                report.note(Fault::CaretStandsOnNoLine { offset, upstream });
            }
        }
        if !placed {
            report.note(Fault::CaretStandsOnNoLine {
                offset,
                upstream: false,
            });
        }
    }
}

fn check_hit_tests(layout: &TextLayout, report: &mut Report) {
    for line in layout.lines() {
        for glyph in line.glyphs() {
            if glyph.annotation().is_some() {
                continue;
            }
            let Some(point) = centre(glyph.cell_bounds()) else {
                continue;
            };
            let hit = layout.hit_test(point);
            let range = glyph.source_range();
            if hit.byte_offset() < range.start || hit.byte_offset() > range.end {
                report.note(Fault::HitTestMissesItsOwnCell {
                    line: line.index(),
                    range,
                    answered: hit.byte_offset(),
                });
            }
        }
    }
}

/// The rectangle a line's own origin and extents describe, before an annotation
/// widens [`TextLine::bounds`].
fn body_cell(line: &TextLine) -> Rect {
    let (x, y) = (line.origin().x_26_6(), line.origin().y_26_6());
    let inline = line.inline_extent_26_6();
    let block = line.block_extent_26_6();
    match line.writing_mode() {
        WritingMode::VerticalRl => Rect::from_fixed(x.saturating_sub(block), y, block, inline),
        _ => Rect::from_fixed(x, y, inline, block),
    }
}

fn block_origin(line: &TextLine, mode: WritingMode) -> i32 {
    match mode {
        WritingMode::VerticalRl => line.origin().x_26_6(),
        _ => line.origin().y_26_6(),
    }
}

/// The corner [`GlyphPlacement::origin`] is defined to be.
///
/// The axes are the *paragraph's*, for every cell on the line including a
/// tate-chu-yoko member: the construct changes which way the glyph faces, not
/// which way the column runs.
///
/// This is not a [`Fault`]: both sides are derived from the same fields, so no
/// layout can break it and a check with no possible witness is a guess. It is
/// held by a unit test instead, so that a change to `cell_bounds` cannot end the
/// relationship a renderer relies on without saying so.
#[cfg(test)]
fn cell_corner(glyph: &GlyphPlacement) -> (i32, i32) {
    let (x, y, width, height) = glyph.cell_bounds().as_26_6();
    match glyph.writing_mode() {
        WritingMode::VerticalRl => (x.saturating_add(width), y),
        _ => (x, y.saturating_add(height)),
    }
}

fn caret_position(caret: Rect, mode: WritingMode) -> Rect {
    let (x, y, width, height) = caret.as_26_6();
    match mode {
        WritingMode::VerticalRl => Rect::from_fixed(x, y, width, 0),
        _ => Rect::from_fixed(x, y, 0, height),
    }
}

fn centre(cell: Rect) -> Option<Point> {
    let (x, y, width, height) = cell.as_26_6();
    Some(Point::from_fixed(
        x.checked_add(width / 2)?,
        y.checked_add(height / 2)?,
    ))
}

/// Does `inner` stay inside `outer` along the axis lines progress down?
fn within_block(mode: WritingMode, outer: Rect, inner: Rect) -> bool {
    let (ox, oy, ow, oh) = outer.as_26_6();
    let (ix, iy, iw, ih) = inner.as_26_6();
    match mode {
        WritingMode::VerticalRl => ix >= ox && ix.saturating_add(iw) <= ox.saturating_add(ow),
        _ => iy >= oy && iy.saturating_add(ih) <= oy.saturating_add(oh),
    }
}

/// Does `inner` stay inside `outer` along the axis a line runs along?
fn within_inline(mode: WritingMode, outer: Rect, inner: Rect) -> bool {
    let (ox, oy, ow, oh) = outer.as_26_6();
    let (ix, iy, iw, ih) = inner.as_26_6();
    match mode {
        WritingMode::VerticalRl => iy >= oy && iy.saturating_add(ih) <= oy.saturating_add(oh),
        _ => ix >= ox && ix.saturating_add(iw) <= ox.saturating_add(ow),
    }
}

fn contains(outer: Rect, inner: Rect) -> bool {
    let (ox, oy, ow, oh) = outer.as_26_6();
    let (ix, iy, iw, ih) = inner.as_26_6();
    ix >= ox
        && iy >= oy
        && ix.saturating_add(iw) <= ox.saturating_add(ow)
        && iy.saturating_add(ih) <= oy.saturating_add(oh)
}

/// A cell's coordinate along the inline axis, whichever axis that is.
fn inline_start(mode: WritingMode, cell: Rect) -> i32 {
    let (x, y, _, _) = cell.as_26_6();
    match mode {
        WritingMode::VerticalRl => y,
        _ => x,
    }
}

fn overlaps_block(mode: WritingMode, left: Rect, right: Rect) -> bool {
    let (lx, ly, lw, lh) = left.as_26_6();
    let (rx, ry, rw, rh) = right.as_26_6();
    match mode {
        WritingMode::VerticalRl => lx < rx.saturating_add(rw) && rx < lx.saturating_add(lw),
        _ => ly < ry.saturating_add(rh) && ry < ly.saturating_add(lh),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{Fault, Report, cell_corner, inspect};
    use crate::result::{
        AnnotationSource, GlyphPlacement, GlyphTransform, Point, Rect, TextLayout, TextLine,
    };
    use crate::{FontId, FontLibrary, LayoutOptions, WritingMode};

    const EM: i32 = 1024;

    fn font_id() -> FontId {
        let mut fonts = FontLibrary::new();
        fonts
            .register_font(Arc::<[u8]>::from(font_test_data::NOTO_SANS_JP_CFF))
            .expect("the fixture font parses")
    }

    /// A body glyph filling one em cell whose block-end corner is at `(x, y)`.
    fn glyph(range: std::ops::Range<usize>, x: i32, y: i32) -> GlyphPlacement {
        GlyphPlacement {
            font_id: font_id(),
            glyph_id: 1,
            source_range: range,
            annotation: None,
            x,
            y,
            advance_x: EM,
            advance_y: 0,
            offset_x: 0,
            offset_y: 0,
            font_size: EM,
            variations: Arc::from([]),
            transform: GlyphTransform::Identity,
            bidi_level: 0,
            writing_mode: WritingMode::HorizontalTb,
            construct: None,
        }
    }

    fn line(index: usize, range: std::ops::Range<usize>, glyphs: Vec<GlyphPlacement>) -> TextLine {
        let inline_extent = i32::try_from(glyphs.len()).unwrap_or(0).saturating_mul(EM);
        let hit_bounds = TextLine::hit_bounds_for(&glyphs);
        TextLine {
            range,
            origin: Point::from_fixed(0, i32::try_from(index).unwrap_or(0).saturating_mul(EM)),
            inline_extent,
            block_extent: EM,
            writing_mode: WritingMode::HorizontalTb,
            glyphs,
            hit_bounds,
            index,
            paragraph_index: 0,
            first_in_paragraph: index == 0,
            last_in_paragraph: true,
        }
    }

    fn layout(source: &str, lines: Vec<TextLine>) -> TextLayout {
        TextLayout {
            source: source.to_owned(),
            lines,
            fonts: Vec::new(),
            diagnostics: Vec::new(),
            writing_mode: WritingMode::HorizontalTb,
            options: LayoutOptions::try_new(64.0, 16.0).expect("the options are valid"),
        }
    }

    /// Two full-width characters on one line, placed the way this crate places
    /// them. Every witness below breaks exactly one thing about it.
    fn sound_layout() -> TextLayout {
        layout(
            "日本",
            vec![line(0, 0..6, vec![glyph(0..3, 0, EM), glyph(3..6, EM, EM)])],
        )
    }

    fn kinds(report: &Report) -> Vec<&'static str> {
        report.faults().iter().map(Fault::kind).collect()
    }

    // One layout per statement, each breaking exactly the thing its name says.
    // They are named rather than inlined so that
    // `every_fault_kind_is_produced_by_some_layout` can hold the whole set to
    // the rule the module states: a fault nobody can witness is a guess.

    fn cell_outside_its_line() -> TextLayout {
        let mut broken = sound_layout();
        broken.lines[0].glyphs[1].y = 10 * EM;
        broken
    }

    /// Three cells and a measure of one, so the fault is an *interior* cell:
    /// the last one past the measure is hanging punctuation and is exempt.
    fn cell_past_the_measure() -> TextLayout {
        let mut broken = layout(
            "日本語",
            vec![line(
                0,
                0..9,
                vec![
                    glyph(0..3, 0, EM),
                    glyph(3..6, EM, EM),
                    glyph(6..9, 2 * EM, EM),
                ],
            )],
        );
        broken.lines[0].inline_extent = EM;
        broken
    }

    fn annotation_over_its_base() -> TextLayout {
        let mut broken = sound_layout();
        broken.lines[0].glyphs[1].annotation = Some(AnnotationSource::new(0, 0..4));
        broken
    }

    fn lines_at_one_place() -> TextLayout {
        let mut broken = layout(
            "日本",
            vec![
                line(0, 0..3, vec![glyph(0..3, 0, EM)]),
                line(1, 3..6, vec![glyph(3..6, 0, 2 * EM)]),
            ],
        );
        broken.lines[1].origin = Point::from_fixed(0, 0);
        broken
    }

    /// Three lines, the third stepping back the way it came.
    fn progression_that_reverses() -> TextLayout {
        let mut broken = layout(
            "日本語",
            vec![
                line(0, 0..3, vec![glyph(0..3, 0, EM)]),
                line(1, 3..6, vec![glyph(3..6, 0, 2 * EM)]),
                line(2, 6..9, vec![glyph(6..9, 0, 3 * EM)]),
            ],
        );
        broken.lines[2].origin = Point::from_fixed(0, -2 * EM);
        broken.lines[2].glyphs[0].y = -EM;
        broken.lines[2].hit_bounds = TextLine::hit_bounds_for(&broken.lines[2].glyphs);
        broken
    }

    fn lines_that_miss_the_source() -> TextLayout {
        layout("日本語", vec![line(0, 3..6, vec![glyph(3..6, 0, EM)])])
    }

    /// A gap between two lines that the source does not hold a separator in.
    fn lines_with_a_gap() -> TextLayout {
        layout(
            "日本語語",
            vec![
                line(0, 0..6, vec![glyph(0..3, 0, EM), glyph(3..6, EM, EM)]),
                line(1, 9..12, vec![glyph(9..12, 0, 2 * EM)]),
            ],
        )
    }

    /// Two cells at one place, attributed to bytes far enough apart that the
    /// answer for one cannot also be an answer for the other.
    fn cell_the_hit_test_cannot_reach() -> TextLayout {
        layout(
            "日本語",
            vec![line(0, 0..9, vec![glyph(0..3, 0, EM), glyph(6..9, 0, EM)])],
        )
    }

    /// A line whose glyph is attributed to bytes the line's own range does not
    /// end at: `caret_rect` finds no glyph edge at the line's end and no empty
    /// line to fall back to, so neither affinity places a caret there.
    fn line_edge_with_no_caret() -> TextLayout {
        layout("日本語", vec![line(0, 0..9, vec![glyph(0..3, 0, EM)])])
    }

    fn witness_layouts() -> Vec<TextLayout> {
        vec![
            cell_outside_its_line(),
            cell_past_the_measure(),
            annotation_over_its_base(),
            lines_at_one_place(),
            progression_that_reverses(),
            lines_that_miss_the_source(),
            lines_with_a_gap(),
            cell_the_hit_test_cannot_reach(),
            line_edge_with_no_caret(),
        ]
    }

    #[test]
    fn a_layout_this_crate_would_produce_is_sound() {
        let report = inspect(&sound_layout());
        assert!(report.is_sound(), "{report}");
        assert_eq!(report.to_string(), "sound");
    }

    /// Compared against the line's composed box, not against
    /// [`TextLine::bounds`]. `bounds` unions in the cells of the very glyphs
    /// this walks, so a check against it holds for every layout the engine can
    /// build and would only ever have failed on a fixture whose `hit_bounds`
    /// was left stale — which is what this test used to do.
    #[test]
    fn a_cell_outside_its_line_is_reported() {
        let broken = cell_outside_its_line();
        let report = inspect(&broken);
        assert!(
            kinds(&report).contains(&"cell-escapes-its-line"),
            "{report}"
        );
        // The block axis, and only it. Moving the same cell along the inline
        // axis is the other statement, and this one must stay quiet about it.
        let mut sideways = sound_layout();
        sideways.lines[0].glyphs[1].x = 10 * EM;
        assert!(
            !kinds(&inspect(&sideways)).contains(&"cell-escapes-its-line"),
            "{}",
            inspect(&sideways)
        );
    }

    /// `origin` is the cell's inline-start, block-end corner. Today the two are
    /// derived from the same fields, so no layout can break the relationship and
    /// it is not a `Fault` — a check with no possible witness is a guess, which
    /// `docs/design/invariants.md` rules out. It is pinned here instead, because
    /// a renderer relies on it and a change to `cell_bounds` could end it.
    #[test]
    fn an_origin_is_its_own_cell_corner_in_every_orientation() {
        for mode in [WritingMode::HorizontalTb, WritingMode::VerticalRl] {
            for transform in [
                GlyphTransform::Identity,
                GlyphTransform::RotateClockwise,
                GlyphTransform::TateChuYoko,
            ] {
                let mut placed = glyph(0..3, 3 * EM, 5 * EM);
                placed.writing_mode = mode;
                placed.transform = transform;
                assert_eq!(
                    cell_corner(&placed),
                    (placed.origin().x_26_6(), placed.origin().y_26_6()),
                    "{mode:?} {transform:?}"
                );
            }
        }
    }

    /// An interior cell past the measure is the fault; the last one is
    /// hanging punctuation and is exempt, so the witness needs three cells.
    #[test]
    fn an_interior_cell_past_the_measure_is_reported_and_a_hanging_one_is_not() {
        let mut broken = cell_past_the_measure();
        let report = inspect(&broken);
        assert_eq!(
            report
                .faults()
                .iter()
                .filter(|fault| fault.kind() == "cell-escapes-the-measure-silently")
                .count(),
            1,
            "{report}"
        );

        // With the measure covering everything but the last cell, that cell is
        // exactly what hanging punctuation looks like and nothing is reported.
        broken.lines[0].inline_extent = 2 * EM;
        let report = inspect(&broken);
        assert!(
            !kinds(&report).contains(&"cell-escapes-the-measure-silently"),
            "{report}"
        );
    }

    #[test]
    fn lines_sharing_block_coordinates_are_reported() {
        let report = inspect(&lines_at_one_place());
        assert!(kinds(&report).contains(&"lines-overlap"), "{report}");
    }

    #[test]
    fn lines_that_start_late_or_stop_early_are_reported() {
        assert_eq!(
            kinds(&inspect(&lines_that_miss_the_source())),
            ["coverage-starts-late", "coverage-ends-early"]
        );
    }

    #[test]
    fn a_gap_is_a_fault_unless_it_is_the_separator_the_source_holds() {
        let separated = layout(
            "日本\n語",
            vec![
                line(0, 0..6, vec![glyph(0..3, 0, EM), glyph(3..6, EM, EM)]),
                line(1, 7..10, vec![glyph(7..10, 0, 2 * EM)]),
            ],
        );
        let report = inspect(&separated);
        assert!(!kinds(&report).contains(&"lines-do-not-meet"), "{report}");

        let report = inspect(&lines_with_a_gap());
        assert!(kinds(&report).contains(&"lines-do-not-meet"), "{report}");
    }

    /// Ruby stands beside the body; a cell that shares the body's block range
    /// is printed over the text it annotates.
    #[test]
    fn an_annotation_over_its_base_is_reported() {
        let broken = annotation_over_its_base();
        assert!(
            kinds(&inspect(&broken)).contains(&"annotation-overlaps-its-base"),
            "{}",
            inspect(&broken)
        );
    }

    /// Three lines, the third stepping back the way it came.
    #[test]
    fn a_line_that_steps_back_up_the_block_axis_is_reported() {
        let broken = progression_that_reverses();
        assert!(
            kinds(&inspect(&broken)).contains(&"block-progression-reverses"),
            "{}",
            inspect(&broken)
        );
    }

    /// Two cells at one place, attributed to bytes far enough apart that the
    /// answer for one cannot also be an answer for the other.
    #[test]
    fn a_cell_the_hit_test_cannot_reach_is_reported() {
        let broken = cell_the_hit_test_cannot_reach();
        assert!(
            kinds(&inspect(&broken)).contains(&"hit-test-misses-its-own-cell"),
            "{}",
            inspect(&broken)
        );
    }

    /// A line edge with no caret is the same failure as a caret nowhere, and
    /// used to be skipped in silence.
    #[test]
    fn a_line_edge_with_no_caret_is_reported() {
        let kinds = kinds(&inspect(&line_edge_with_no_caret()));
        assert!(kinds.contains(&"caret-stands-on-no-line"), "{kinds:?}");
    }

    #[test]
    fn every_fault_names_itself_and_says_whether_it_has_a_line() {
        for fault in every_fault() {
            let rendered = fault.to_string();
            assert!(rendered.starts_with(fault.kind()), "{rendered}");
            match fault.line() {
                Some(line) => assert!(rendered.contains(&format!("line {line}")), "{rendered}"),
                None => assert!(!rendered.contains(" on line "), "{rendered}"),
            }
        }
    }

    /// Every kind is produced by a layout, not merely constructible by hand.
    ///
    /// `the_fixture_holds_one_of_every_fault` walks a list of `Fault` values
    /// built with struct literals, which is what the rendering tests need and
    /// what cannot tell a live statement from dead code. Four kinds had only
    /// that, and one of the four turned out to be a statement no layout could
    /// break. This is the guard: a new `Fault` needs a layout that produces it
    /// before it can be added, which is what `docs/design/invariants.md` asks.
    #[test]
    fn every_fault_kind_is_produced_by_some_layout() {
        let mut produced: Vec<&'static str> = witness_layouts()
            .iter()
            .flat_map(|layout| kinds(&inspect(layout)))
            .collect();
        produced.sort_unstable();
        produced.dedup();

        let mut declared: Vec<&'static str> = every_fault().iter().map(Fault::kind).collect();
        declared.sort_unstable();
        declared.dedup();

        assert_eq!(produced, declared, "a fault has no layout that produces it");
    }

    #[test]
    fn the_fixture_holds_one_of_every_fault() {
        let mut seen = [false; 10];
        for fault in every_fault() {
            let index = match fault {
                Fault::CellEscapesItsLine { .. } => 0,
                Fault::CellEscapesTheMeasureSilently { .. } => 1,
                Fault::AnnotationOverlapsItsBase { .. } => 2,
                Fault::LinesOverlap { .. } => 3,
                Fault::BlockProgressionReverses { .. } => 4,
                Fault::CoverageStartsLate { .. } => 5,
                Fault::CoverageEndsEarly { .. } => 6,
                Fault::LinesDoNotMeet { .. } => 7,
                Fault::CaretStandsOnNoLine { .. } => 8,
                Fault::HitTestMissesItsOwnCell { .. } => 9,
            };
            seen[index] = true;
        }
        assert!(seen.iter().all(|hit| *hit), "a fault is unrepresented");
    }

    #[test]
    fn a_report_reads_as_a_list_of_places() {
        let mut report = Report::default();
        report.note(Fault::LinesOverlap { line: 2 });
        report.note(Fault::CoverageStartsLate { observed: 3 });
        assert_eq!(
            report.to_string(),
            "2 geometric fault(s):\n  lines-overlap on line 2\n  coverage-starts-late at 3\n"
        );
    }

    #[test]
    fn rectangles_are_compared_in_exact_units() {
        let cell = Rect::from_fixed(0, 0, EM, EM);
        assert!(cell.contains(Point::from_fixed(EM, EM)));
        assert!(!cell.contains(Point::from_fixed(EM + 1, 0)));
        assert_eq!(
            cell.union(Rect::from_fixed(2 * EM, 0, EM, EM)).as_26_6(),
            (0, 0, 3 * EM, EM)
        );
    }

    fn every_fault() -> Vec<Fault> {
        vec![
            Fault::CellEscapesItsLine {
                line: 1,
                range: 0..3,
            },
            Fault::CellEscapesTheMeasureSilently {
                line: 2,
                range: 6..9,
            },
            Fault::AnnotationOverlapsItsBase {
                line: 2,
                range: 0..6,
            },
            Fault::LinesOverlap { line: 3 },
            Fault::BlockProgressionReverses {
                line: 4,
                established: 1,
                observed: -1024,
            },
            Fault::CoverageStartsLate { observed: 3 },
            Fault::CoverageEndsEarly {
                observed: 6,
                source: 9,
            },
            Fault::LinesDoNotMeet {
                line: 5,
                previous_end: 6,
                observed_start: 9,
            },
            Fault::CaretStandsOnNoLine {
                offset: 4,
                upstream: true,
            },
            Fault::HitTestMissesItsOwnCell {
                line: 6,
                range: 0..3,
                answered: 9,
            },
        ]
    }
}
