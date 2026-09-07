// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! What must be true of any layout this composer returns.
//!
//! The trace says why a layout came out as it did. This says whether the layout is
//! self-consistent at all — whether the lines partition the source, whether they progress
//! in one direction, whether every placement sits inside the line that claims it, whether
//! every attachment names a construct that exists.
//!
//! It reports rather than panics. This workspace has two `debug_assert!` and no `assert!`
//! in shipped code: an invariant is a typed value the caller decides what to do with, the
//! same way [`Paragraph`] validation returns [`InputError`](crate::InputError) rather than
//! aborting. A test asserts on the report; a fuzz target prints it; a renderer could refuse
//! to draw and say why.
//!
//! What it deliberately does not check: anything requiring the composer's own intermediate
//! state. "The ladder distributed exactly what it needed to" is a statement about numbers
//! that never reach [`Layout`], and re-deriving them here would make this a second
//! composer whose agreement with the first proves nothing. Those belong to the trace, which
//! records the arithmetic as it happens.

use alloc::vec::Vec;
use core::fmt;
use core::ops::Range;

use crate::layout::{CoordinateTransform, Layout, Line, PlacementOrigin};
use crate::model::WritingMode;
use crate::paragraph::Paragraph;

/// One way a layout failed to be self-consistent.
///
/// Every variant names the line it belongs to, so a report reads as a list of places rather
/// than a list of complaints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Fault {
    /// A line reported a negative inline or block extent.
    ExtentIsNegative {
        /// The line ordinal.
        line: usize,
        /// The inline extent it reported.
        inline: i32,
        /// The block extent it reported.
        block: i32,
    },
    /// Lines changed the direction they progress in partway down the paragraph.
    BlockProgressionReverses {
        /// The line whose origin broke the run.
        line: usize,
        /// The direction established by the lines before it, as a sign.
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
    /// Two consecutive lines do not meet: one leaves a gap or overlaps the next.
    LinesDoNotMeet {
        /// The later line.
        line: usize,
        /// Where the previous line ended.
        previous_end: usize,
        /// Where this one starts.
        observed_start: usize,
    },
    /// A line's own byte range is inverted.
    LineRangeIsInverted {
        /// The line ordinal.
        line: usize,
        /// Where it claims to start.
        start: usize,
        /// Where it claims to end.
        end: usize,
    },
    /// A placement claims bytes the line that holds it does not.
    ClusterEscapesItsLine {
        /// The line ordinal.
        line: usize,
        /// The placement's position within the line.
        cluster: usize,
        /// The bytes the placement claims.
        start: usize,
        /// The end of the bytes the placement claims.
        end: usize,
    },
    /// A placement is set in a writing mode other than the paragraph's, with no local
    /// transform that would explain it.
    ///
    /// One thing legitimately does this: tate-chu-yoko sets an upright horizontal group
    /// inside vertical text, and says so through its transform.
    ClusterOrientationUnexplained {
        /// The line ordinal.
        line: usize,
        /// The placement's position within the line.
        cluster: usize,
    },
    /// A placement reported a negative advance.
    ClusterAdvanceIsNegative {
        /// The line ordinal.
        line: usize,
        /// The placement's position within the line.
        cluster: usize,
        /// The advance it reported.
        advance: i32,
    },
    /// An attachment names a construct the paragraph does not hold.
    AttachmentNamesNoConstruct {
        /// The line ordinal.
        line: usize,
        /// The attachment's position within the line.
        attachment: usize,
        /// The construct ordinal it named.
        construct: usize,
        /// How many constructs the paragraph holds.
        constructs: usize,
    },
    /// An attachment claims bytes outside its construct's annotation stream.
    ///
    /// An attachment's range indexes the annotation, never the paragraph — so a repeated
    /// mark, which has no stream of its own, must claim an empty range.
    AttachmentEscapesItsAnnotation {
        /// The line ordinal.
        line: usize,
        /// The attachment's position within the line.
        attachment: usize,
        /// The end of the bytes the attachment claims.
        end: usize,
        /// How long the annotation stream is, or zero where the construct has none.
        annotation: usize,
    },
    /// A placement is attributed to a cluster or construct the paragraph does not hold.
    PlacementNamesNothing {
        /// The line ordinal.
        line: usize,
        /// The placement's position within the line.
        cluster: usize,
        /// The ordinal it named.
        ordinal: usize,
        /// How many items the slice it names holds.
        available: usize,
    },
    /// A diagnostic points outside the source it was raised against.
    DiagnosticEscapesTheSource {
        /// The diagnostic's position in the layout.
        diagnostic: usize,
        /// The end of the range it named.
        end: usize,
        /// Where the source ends.
        source: usize,
    },
}

impl Fault {
    /// The stable name of this kind of fault.
    ///
    /// There is no wildcard arm: a new [`Fault`] does not compile until it is named. That is
    /// [ADR 0012](https://github.com/jlreq/jlreq)'s frozen-projection requirement met by the
    /// compiler rather than by review, the same way [`crate::trace::Fact`] meets it.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match *self {
            Self::ExtentIsNegative { .. } => "extent-is-negative",
            Self::BlockProgressionReverses { .. } => "block-progression-reverses",
            Self::CoverageStartsLate { .. } => "coverage-starts-late",
            Self::CoverageEndsEarly { .. } => "coverage-ends-early",
            Self::LinesDoNotMeet { .. } => "lines-do-not-meet",
            Self::LineRangeIsInverted { .. } => "line-range-is-inverted",
            Self::ClusterEscapesItsLine { .. } => "cluster-escapes-its-line",
            Self::ClusterOrientationUnexplained { .. } => "cluster-orientation-unexplained",
            Self::ClusterAdvanceIsNegative { .. } => "cluster-advance-is-negative",
            Self::AttachmentNamesNoConstruct { .. } => "attachment-names-no-construct",
            Self::AttachmentEscapesItsAnnotation { .. } => "attachment-escapes-its-annotation",
            Self::PlacementNamesNothing { .. } => "placement-names-nothing",
            Self::DiagnosticEscapesTheSource { .. } => "diagnostic-escapes-the-source",
        }
    }

    /// The line this fault belongs to, where one does.
    ///
    /// There is no wildcard arm, for the same reason [`Fault::kind`] has none.
    #[must_use]
    pub const fn line(&self) -> Option<usize> {
        match *self {
            Self::ExtentIsNegative { line, .. }
            | Self::BlockProgressionReverses { line, .. }
            | Self::LinesDoNotMeet { line, .. }
            | Self::LineRangeIsInverted { line, .. }
            | Self::ClusterEscapesItsLine { line, .. }
            | Self::ClusterOrientationUnexplained { line, .. }
            | Self::ClusterAdvanceIsNegative { line, .. }
            | Self::AttachmentNamesNoConstruct { line, .. }
            | Self::AttachmentEscapesItsAnnotation { line, .. }
            | Self::PlacementNamesNothing { line, .. } => Some(line),
            Self::CoverageStartsLate { .. }
            | Self::CoverageEndsEarly { .. }
            | Self::DiagnosticEscapesTheSource { .. } => None,
        }
    }
}

/// Everything that was not true of one layout.
///
/// An empty report is the answer a caller wants; [`Report::is_sound`] says so without
/// making them count.
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

/// Check a layout against the paragraph it was composed from.
///
/// The paragraph is needed because most of these invariants are relational: a line's bytes
/// mean nothing without the source they index, and an attachment's construct ordinal means
/// nothing without the construct list it indexes.
#[must_use]
pub fn inspect(layout: &Layout, paragraph: &Paragraph) -> Report {
    let mut report = Report::default();
    let source = paragraph.text().source().len();
    let constructs = paragraph.constructs().len();
    let mode = paragraph.writing_mode();

    check_coverage(layout.lines(), source, &mut report);
    check_progression(layout.lines(), &mut report);
    for (ordinal, line) in layout.lines().iter().enumerate() {
        check_line(ordinal, line, mode, constructs, paragraph, &mut report);
    }
    for (ordinal, diagnostic) in layout.diagnostics().iter().enumerate() {
        // Not a `let` chain: this crate's MSRV is 1.85 and they landed in 1.88.
        if let Some(range) = diagnostic.range() {
            if range.end > source {
                report.note(Fault::DiagnosticEscapesTheSource {
                    diagnostic: ordinal,
                    end: range.end,
                    source,
                });
            }
        }
    }
    report
}

/// The lines partition the source: they start at its start, meet end to end, and finish at
/// its end. A layout that dropped or duplicated input would show up here and nowhere else.
fn check_coverage(lines: &[Line], source: usize, report: &mut Report) {
    let Some(first) = lines.first() else {
        return;
    };
    if first.range().start != 0 {
        report.note(Fault::CoverageStartsLate {
            observed: first.range().start,
        });
    }
    let mut previous_end = first.range().start;
    // One broken link suppresses exactly one derived complaint: after an inverted range
    // there is no meaningful end for the next line to meet, and saying so twice would
    // bury the fault that actually happened.
    let mut chained = true;
    for (ordinal, line) in lines.iter().enumerate() {
        let range = line.range();
        if range.start > range.end {
            report.note(Fault::LineRangeIsInverted {
                line: ordinal,
                start: range.start,
                end: range.end,
            });
            chained = false;
            previous_end = range.end;
            continue;
        }
        if ordinal > 0 && chained && range.start != previous_end {
            report.note(Fault::LinesDoNotMeet {
                line: ordinal,
                previous_end,
                observed_start: range.start,
            });
        }
        chained = true;
        previous_end = range.end;
    }
    if previous_end != source {
        report.note(Fault::CoverageEndsEarly {
            observed: previous_end,
            source,
        });
    }
}

/// Lines progress in one direction. Which one depends on the writing mode and on the
/// caller's coordinate conventions, so this checks the weaker, mode-independent thing: the
/// sequence of block origins never reverses. A layout that stepped forward and then back
/// would overlap two lines wherever it turned.
fn check_progression(lines: &[Line], report: &mut Report) {
    let mut established = 0_i32;
    let mut previous: Option<i32> = None;
    for (ordinal, line) in lines.iter().enumerate() {
        let origin = line.block_origin();
        if let Some(last) = previous {
            let step = origin.saturating_sub(last);
            let sign = step.signum();
            if sign != 0 {
                if established != 0 && sign != established {
                    report.note(Fault::BlockProgressionReverses {
                        line: ordinal,
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
    ordinal: usize,
    line: &Line,
    mode: WritingMode,
    constructs: usize,
    paragraph: &Paragraph,
    report: &mut Report,
) {
    if line.inline_extent() < 0 || line.block_extent() < 0 {
        report.note(Fault::ExtentIsNegative {
            line: ordinal,
            inline: line.inline_extent(),
            block: line.block_extent(),
        });
    }
    let bounds = line.range();
    let inputs = paragraph.text().clusters().len();
    for (index, cluster) in line.clusters().iter().enumerate() {
        let (named, available) = match cluster.origin() {
            PlacementOrigin::Cluster(named) => (named, inputs),
            PlacementOrigin::Construct(named) => (named, constructs),
        };
        if named >= available {
            report.note(Fault::PlacementNamesNothing {
                line: ordinal,
                cluster: index,
                ordinal: named,
                available,
            });
        }
        let range = cluster.range();
        if !encloses(&bounds, &range) {
            report.note(Fault::ClusterEscapesItsLine {
                line: ordinal,
                cluster: index,
                start: range.start,
                end: range.end,
            });
        }
        if cluster.writing_mode() != mode && cluster.transform() != CoordinateTransform::TateChuYoko
        {
            report.note(Fault::ClusterOrientationUnexplained {
                line: ordinal,
                cluster: index,
            });
        }
        if cluster.advance() < 0 {
            report.note(Fault::ClusterAdvanceIsNegative {
                line: ordinal,
                cluster: index,
                advance: cluster.advance(),
            });
        }
    }
    for (index, attachment) in line.attachments().iter().enumerate() {
        let construct = attachment.construct();
        let Some(owner) = paragraph.constructs().get(construct) else {
            report.note(Fault::AttachmentNamesNoConstruct {
                line: ordinal,
                attachment: index,
                construct,
                constructs,
            });
            continue;
        };
        let range = attachment.range();
        let annotation = owner.annotation_len().unwrap_or(0);
        if range.start > range.end || range.end > annotation {
            report.note(Fault::AttachmentEscapesItsAnnotation {
                line: ordinal,
                attachment: index,
                end: range.end,
                annotation,
            });
        }
    }
}

/// Whether `outer` covers `inner`, treating an inverted `inner` as escaping.
fn encloses(outer: &Range<usize>, inner: &Range<usize>) -> bool {
    inner.start <= inner.end && inner.start >= outer.start && inner.end <= outer.end
}

impl fmt::Display for Fault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = self.kind();
        match self.line() {
            Some(line) => write!(formatter, "L{line:02} {kind}"),
            None => write!(formatter, "P   {kind}"),
        }
    }
}

impl fmt::Display for Report {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.faults.is_empty() {
            return write!(formatter, "sound");
        }
        for (ordinal, fault) in self.faults.iter().enumerate() {
            if ordinal > 0 {
                write!(formatter, "; ")?;
            }
            write!(formatter, "{fault}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString as _;
    use alloc::vec;

    use super::*;
    use crate::layout::{Attachment, ClusterPlacement, Diagnostic, PlacementOrigin, Severity};
    use crate::model::{Cluster, Frame, ShapedText, Size};
    use crate::paragraph::Break;
    use crate::style::Style;

    fn em() -> Size {
        Size::square(1_000).expect("a square em is a valid size")
    }

    fn shaped(source: &str) -> ShapedText {
        let clusters = source.char_indices().map(|(start, character)| {
            Cluster::new(start..start.saturating_add(character.len_utf8()), 1_000)
        });
        ShapedText::new(source, em(), Frame::FullEm, clusters).expect("ordinary clusters shape")
    }

    fn paragraph(source: &str, extent: i32) -> Paragraph {
        Paragraph::builder(shaped(source), extent)
            .breaks(
                source
                    .char_indices()
                    .skip(1)
                    .map(|(offset, _)| Break::allowed(offset)),
            )
            .build()
            .expect("an ordinary paragraph builds")
    }

    /// A line the composer did not produce, so a negative case can state exactly one broken
    /// thing rather than hoping the composer can be talked into breaking it.
    fn line(range: Range<usize>, block_origin: i32) -> Line {
        Line {
            range,
            inline_origin: 0,
            block_origin,
            inline_extent: 1_000,
            block_extent: 1_000,
            clusters: Vec::new(),
            attachments: Vec::new(),
        }
    }

    fn placement(range: Range<usize>) -> ClusterPlacement {
        ClusterPlacement {
            origin: PlacementOrigin::Cluster(range.start / 3),
            range,
            inline: 0,
            block: 0,
            advance: 1_000,
            size: em(),
            frame: Frame::FullEm,
            writing_mode: WritingMode::HorizontalTb,
            transform: CoordinateTransform::Identity,
        }
    }

    fn attachment(construct: usize, range: Range<usize>) -> Attachment {
        Attachment {
            construct,
            range,
            inline: 0,
            block: 0,
            advance: 500,
            size: em(),
            writing_mode: WritingMode::HorizontalTb,
            transform: CoordinateTransform::Identity,
            symbol: None,
        }
    }

    fn layout(lines: Vec<Line>, diagnostics: Vec<Diagnostic>) -> Layout {
        Layout { lines, diagnostics }
    }

    /// One of every fault, so the tables above have something to be complete about.
    fn every_fault() -> Vec<Fault> {
        vec![
            Fault::ExtentIsNegative {
                line: 0,
                inline: -1,
                block: 0,
            },
            Fault::BlockProgressionReverses {
                line: 2,
                established: 1,
                observed: -1_000,
            },
            Fault::CoverageStartsLate { observed: 3 },
            Fault::CoverageEndsEarly {
                observed: 6,
                source: 9,
            },
            Fault::LinesDoNotMeet {
                line: 1,
                previous_end: 6,
                observed_start: 9,
            },
            Fault::LineRangeIsInverted {
                line: 0,
                start: 6,
                end: 3,
            },
            Fault::ClusterEscapesItsLine {
                line: 0,
                cluster: 1,
                start: 0,
                end: 12,
            },
            Fault::ClusterOrientationUnexplained {
                line: 0,
                cluster: 2,
            },
            Fault::ClusterAdvanceIsNegative {
                line: 0,
                cluster: 3,
                advance: -1,
            },
            Fault::AttachmentNamesNoConstruct {
                line: 0,
                attachment: 0,
                construct: 4,
                constructs: 1,
            },
            Fault::AttachmentEscapesItsAnnotation {
                line: 0,
                attachment: 0,
                end: 30,
                annotation: 6,
            },
            Fault::PlacementNamesNothing {
                line: 0,
                cluster: 0,
                ordinal: 9,
                available: 3,
            },
            Fault::DiagnosticEscapesTheSource {
                diagnostic: 0,
                end: 40,
                source: 9,
            },
        ]
    }

    #[test]
    fn the_fixture_holds_one_of_every_fault() {
        let mut seen = [false; 13];
        for fault in every_fault() {
            let index = match fault {
                Fault::ExtentIsNegative { .. } => 0,
                Fault::BlockProgressionReverses { .. } => 1,
                Fault::CoverageStartsLate { .. } => 2,
                Fault::CoverageEndsEarly { .. } => 3,
                Fault::LinesDoNotMeet { .. } => 4,
                Fault::LineRangeIsInverted { .. } => 5,
                Fault::ClusterEscapesItsLine { .. } => 6,
                Fault::ClusterOrientationUnexplained { .. } => 7,
                Fault::ClusterAdvanceIsNegative { .. } => 8,
                Fault::AttachmentNamesNoConstruct { .. } => 9,
                Fault::AttachmentEscapesItsAnnotation { .. } => 10,
                Fault::PlacementNamesNothing { .. } => 11,
                Fault::DiagnosticEscapesTheSource { .. } => 12,
            };
            seen[index] = true;
        }
        assert!(seen.iter().all(|hit| *hit), "a fault is unrepresented");
    }

    #[test]
    fn every_fault_names_itself_and_says_where_it_belongs() {
        for fault in every_fault() {
            let rendered = fault.to_string();
            assert!(rendered.contains(fault.kind()), "{rendered}");
            match fault.line() {
                Some(_) => assert!(rendered.starts_with('L'), "{rendered}"),
                None => assert!(rendered.starts_with("P   "), "{rendered}"),
            }
        }
    }

    #[test]
    fn a_report_reads_as_a_list_of_places() {
        let sound = Report::default();
        assert!(sound.is_sound());
        assert_eq!(sound.to_string(), "sound");
        let mut report = Report::default();
        report.note(Fault::CoverageStartsLate { observed: 3 });
        report.note(Fault::ClusterAdvanceIsNegative {
            line: 1,
            cluster: 0,
            advance: -1,
        });
        assert!(!report.is_sound());
        assert_eq!(report.faults().len(), 2);
        assert_eq!(
            report.to_string(),
            "P   coverage-starts-late; L01 cluster-advance-is-negative"
        );
    }

    /// Every layout this crate composes must be sound. If an invariant were wrong about the
    /// composer rather than about the layout, this is where it shows — before it reaches a
    /// fuzz target as an assertion.
    #[test]
    fn every_composed_layout_is_sound() {
        let cases = [
            ("plain", paragraph("日本語の組版、その理由。", 7_680)),
            ("one-line", paragraph("組版", 20_000)),
            ("tight", paragraph("日本語、その組版。", 2_560)),
            ("single-cluster-lines", paragraph("組版技術", 1_000)),
            ("punctuation", paragraph("「組版」、その。", 5_120)),
        ];
        let styles = [
            Style::jlreq_2020(),
            Style::book_2020(),
            Style::magazine_2020(),
            Style::newspaper_2020(),
            Style::jis_reading_2020(),
        ];
        for (name, paragraph) in &cases {
            for style in &styles {
                let composed = crate::compose(paragraph, style).expect("the fixture composes");
                let report = inspect(&composed, paragraph);
                assert!(report.is_sound(), "{name}: {report}");
            }
        }
    }

    #[test]
    fn every_composed_vertical_layout_is_sound() {
        let source = "日本語の組版、その理由。";
        let paragraph = Paragraph::builder(shaped(source), 5_120)
            .breaks(
                source
                    .char_indices()
                    .skip(1)
                    .map(|(offset, _)| Break::allowed(offset)),
            )
            .writing_mode(WritingMode::VerticalRl)
            .build()
            .expect("a vertical paragraph builds");
        let composed = crate::compose(&paragraph, &Style::jlreq_2020()).expect("it composes");
        let report = inspect(&composed, &paragraph);
        assert!(report.is_sound(), "{report}");
        assert!(composed.lines().len() > 1, "the case must reach two lines");
    }

    #[test]
    fn a_layout_that_drops_input_is_not_sound() {
        let paragraph = paragraph("日本語", 20_000);
        let truncated = layout(vec![line(0..3, 0)], Vec::new());
        let report = inspect(&truncated, &paragraph);
        assert_eq!(
            report.faults().first().map(Fault::kind),
            Some("coverage-ends-early")
        );
    }

    #[test]
    fn a_layout_that_starts_late_or_leaves_a_gap_is_not_sound() {
        let paragraph = paragraph("日本語", 20_000);
        let gapped = layout(vec![line(3..6, 0), line(9..9, 1_000)], Vec::new());
        let report = inspect(&gapped, &paragraph);
        let kinds: Vec<_> = report.faults().iter().map(Fault::kind).collect();
        assert_eq!(kinds, ["coverage-starts-late", "lines-do-not-meet"]);
    }

    #[test]
    fn an_inverted_line_range_does_not_cascade_into_the_next_line() {
        let paragraph = paragraph("日本語", 20_000);
        let backwards = Range { start: 6, end: 3 };
        let inverted = layout(vec![line(backwards, 0), line(3..9, 1_000)], Vec::new());
        let report = inspect(&inverted, &paragraph);
        let kinds: Vec<_> = report.faults().iter().map(Fault::kind).collect();
        assert_eq!(kinds, ["coverage-starts-late", "line-range-is-inverted"]);
    }

    #[test]
    fn a_placement_outside_its_line_is_not_sound() {
        let paragraph = paragraph("日本語", 20_000);
        let mut only = line(0..9, 0);
        only.clusters = vec![placement(0..30)];
        let report = inspect(&layout(vec![only], Vec::new()), &paragraph);
        assert_eq!(
            report.faults().first().map(Fault::kind),
            Some("cluster-escapes-its-line")
        );
    }

    #[test]
    fn an_unexplained_orientation_or_a_backwards_advance_is_not_sound() {
        let source = "日本語";
        let paragraph = Paragraph::builder(shaped(source), 20_000)
            .writing_mode(WritingMode::VerticalRl)
            .build()
            .expect("a vertical paragraph builds");
        let mut only = line(0..9, 0);
        let mut backwards = placement(0..3);
        backwards.advance = -1;
        only.clusters = vec![placement(3..6), backwards];
        let report = inspect(&layout(vec![only], Vec::new()), &paragraph);
        let kinds: Vec<_> = report.faults().iter().map(Fault::kind).collect();
        assert_eq!(
            kinds,
            [
                "cluster-orientation-unexplained",
                "cluster-orientation-unexplained",
                "cluster-advance-is-negative"
            ]
        );
    }

    #[test]
    fn a_placement_attributed_to_nothing_is_not_sound() {
        let paragraph = paragraph("日本語", 20_000);
        assert_eq!(paragraph.text().clusters().len(), 3);
        let mut only = line(0..9, 0);
        let mut stray = placement(0..3);
        stray.origin = PlacementOrigin::Cluster(9);
        let mut orphan = placement(3..6);
        orphan.origin = PlacementOrigin::Construct(0);
        only.clusters = vec![stray, orphan];
        let report = inspect(&layout(vec![only], Vec::new()), &paragraph);
        let kinds: Vec<_> = report.faults().iter().map(Fault::kind).collect();
        assert_eq!(
            kinds,
            ["placement-names-nothing", "placement-names-nothing"]
        );
    }

    #[test]
    fn a_negative_extent_is_not_sound() {
        let paragraph = paragraph("日本語", 20_000);
        let mut only = line(0..9, 0);
        only.inline_extent = -1;
        let report = inspect(&layout(vec![only], Vec::new()), &paragraph);
        assert_eq!(
            report.faults().first().map(Fault::kind),
            Some("extent-is-negative")
        );
    }

    #[test]
    fn a_reversing_progression_and_a_stray_diagnostic_are_not_sound() {
        let paragraph = paragraph("日本語", 20_000);
        let zigzag = layout(
            vec![line(0..3, 0), line(3..6, 1_000), line(6..9, 0)],
            vec![Diagnostic {
                code: "layout.overfull",
                severity: Severity::Warning,
                range: Some(0..40),
                jlreq: "3.8.1",
            }],
        );
        let report = inspect(&zigzag, &paragraph);
        let kinds: Vec<_> = report.faults().iter().map(Fault::kind).collect();
        assert_eq!(
            kinds,
            [
                "block-progression-reverses",
                "diagnostic-escapes-the-source"
            ]
        );
    }

    #[test]
    fn an_attachment_naming_no_construct_is_not_sound() {
        let paragraph = paragraph("日本語", 20_000);
        assert!(paragraph.constructs().is_empty());
        let mut only = line(0..9, 0);
        only.attachments = vec![attachment(7, 0..3)];
        let report = inspect(&layout(vec![only], Vec::new()), &paragraph);
        assert_eq!(
            report.faults().first().map(Fault::kind),
            Some("attachment-names-no-construct")
        );
    }

    /// An attachment's range indexes the annotation stream, so a ruby whose annotation is
    /// six bytes long may claim at most six. A warichu carries no stream at all, so its
    /// attachments — repeated marks — must claim nothing.
    #[test]
    fn an_attachment_outside_its_annotation_stream_is_not_sound() {
        let ruby = crate::construct::Ruby::new(
            crate::construct::RubyKind::Group,
            0..3,
            shaped("かな"),
            [crate::construct::RubyRun::new(0..3, 0..6)],
        )
        .expect("a group ruby over one cluster builds");
        let annotated = Paragraph::builder(shaped("日本語"), 20_000)
            .constructs([crate::construct::Construct::ruby(ruby)])
            .build()
            .expect("a paragraph with one ruby builds");
        let mut only = line(0..9, 0);
        only.attachments = vec![attachment(0, 0..6)];
        assert!(
            inspect(&layout(vec![only.clone()], Vec::new()), &annotated).is_sound(),
            "six bytes of a six-byte annotation is in bounds"
        );
        only.attachments = vec![attachment(0, 0..9)];
        let report = inspect(&layout(vec![only], Vec::new()), &annotated);
        assert_eq!(
            report.faults().first().map(Fault::kind),
            Some("attachment-escapes-its-annotation")
        );

        let unannotated = Paragraph::builder(shaped("日本語"), 20_000)
            .constructs([crate::construct::Construct::warichu(0..6)])
            .build()
            .expect("a paragraph with one warichu builds");
        let mut bare = line(0..9, 0);
        bare.attachments = vec![attachment(0, 0..3)];
        let report = inspect(&layout(vec![bare], Vec::new()), &unannotated);
        assert_eq!(
            report.faults().first().map(Fault::kind),
            Some("attachment-escapes-its-annotation")
        );
    }

    /// Zero is not negative, a range that ends exactly at the source has not
    /// escaped it, and the first line has no predecessor to meet.
    ///
    /// Every comparison below survived a mutation that moved it by one — `<` to
    /// `<=`, `>` to `>=` — because the corpus only ever asks them well away from
    /// their edge. Each of these is one step from a fault and must not be one.
    #[test]
    fn the_edge_of_every_comparison_is_sound() {
        let mut flush = line(0..3, 0);
        flush.inline_extent = 0;
        flush.block_extent = 0;
        flush.clusters = vec![ClusterPlacement {
            advance: 0,
            ..placement(0..3)
        }];
        let source = shaped("あ");
        let paragraph = Paragraph::builder(source, 1_000)
            .constructs([crate::construct::Construct::emphasis_dots(0..3, '・')])
            .build()
            .expect("a one-cluster paragraph builds");
        let report = inspect(
            &layout(
                vec![flush],
                vec![Diagnostic {
                    code: "layout.overfull",
                    severity: Severity::Warning,
                    range: Some(0..3),
                    jlreq: "3.8.1",
                }],
            ),
            &paragraph,
        );
        assert!(report.is_sound(), "{report}");
    }

    /// And one step the other way is a fault, so the comparisons are not simply
    /// always true either.
    #[test]
    fn one_step_past_each_edge_is_reported() {
        let mut negative = line(0..3, 0);
        negative.inline_extent = -1;
        let paragraph = Paragraph::builder(shaped("あ"), 1_000)
            .build()
            .expect("a one-cluster paragraph builds");
        let faults = inspect(&layout(vec![negative], Vec::new()), &paragraph);
        assert!(
            faults
                .faults()
                .iter()
                .any(|fault| matches!(fault, Fault::ExtentIsNegative { .. })),
            "{faults}"
        );

        let mut advancing = line(0..3, 0);
        advancing.clusters = vec![ClusterPlacement {
            advance: -1,
            ..placement(0..3)
        }];
        let faults = inspect(&layout(vec![advancing], Vec::new()), &paragraph);
        assert!(
            faults
                .faults()
                .iter()
                .any(|fault| matches!(fault, Fault::ClusterAdvanceIsNegative { .. })),
            "{faults}"
        );

        let escaping = layout(
            vec![line(0..3, 0)],
            vec![Diagnostic {
                code: "layout.overfull",
                severity: Severity::Warning,
                range: Some(0..4),
                jlreq: "3.8.1",
            }],
        );
        let faults = inspect(&escaping, &paragraph);
        assert!(
            faults
                .faults()
                .iter()
                .any(|fault| matches!(fault, Fault::DiagnosticEscapesTheSource { .. })),
            "{faults}"
        );
    }
}
