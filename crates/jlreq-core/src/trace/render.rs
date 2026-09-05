// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! One decision per line, in a shape a person reads and a diff aligns.
//!
//! Every quantity here is already an integer or a closed enum, so the rendering needs no
//! allocation, no `std`, and no `Debug` — `Debug` output is not a stable format and a
//! golden file written against it would be pinned to the compiler rather than to this
//! crate. The field order is fixed per variant for the same reason.

use core::fmt;

use super::{Event, Fact, RuleAddress, Site, Trace};
use crate::model::WritingMode;
use crate::paragraph::Alignment;

/// The width the kind column is padded to, so sites and fields line up in a diff.
const KIND_WIDTH: usize = 24;

/// The identifier a reader can use to tell one rendering generation from another.
const FORMAT: &str = "jlreq.trace/1";

impl fmt::Display for RuleAddress {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Section(section) => write!(formatter, "{section}"),
            Self::Note(appendix, note) => write!(formatter, "{appendix}#{note}"),
            Self::Cell(table, before, after) => {
                write!(formatter, "{table}@cl-{before:02},cl-{after:02}")
            },
        }
    }
}

impl fmt::Display for Site {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(line) => write!(formatter, "L{line:02} ")?,
            None => write!(formatter, "P   ")?,
        }
        if self.clusters.start.saturating_add(1) == self.clusters.end {
            write!(formatter, "c{}", self.clusters.start)?;
        } else {
            write!(formatter, "c{}..{}", self.clusters.start, self.clusters.end)?;
        }
        write!(formatter, " b{}..{}", self.bytes.start, self.bytes.end)
    }
}

impl fmt::Display for Event {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = self.fact.kind();
        write!(formatter, "{kind:KIND_WIDTH$} {site} ", site = self.site)?;
        write_fields(&self.fact, formatter)?;
        if let Some(address) = self.fact.jlreq() {
            write!(formatter, " {address}")?;
        }
        Ok(())
    }
}

impl fmt::Display for Trace {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            formatter,
            "{FORMAT} events={count} categories=0x{bits:04x} truncated={truncated}",
            count = self.events().len(),
            bits = self.categories().bits(),
            truncated = flag(self.is_truncated()),
        )?;
        for (ordinal, event) in self.events().iter().enumerate() {
            writeln!(formatter, "{ordinal:04} {event}")?;
        }
        Ok(())
    }
}

/// A boolean as one column, because `true` and `false` do not align.
const fn flag(value: bool) -> u8 {
    if value { 1 } else { 0 }
}

/// The writing mode as a stable token.
const fn writing_mode(mode: WritingMode) -> &'static str {
    match mode {
        WritingMode::HorizontalTb => "horizontal-tb",
        WritingMode::VerticalRl => "vertical-rl",
    }
}

/// The alignment as a stable token.
const fn alignment(value: Alignment) -> &'static str {
    match value {
        Alignment::Start => "start",
        Alignment::Center => "center",
        Alignment::End => "end",
        Alignment::Justify => "justify",
    }
}

/// The fields of one decision, in the order this variant fixes.
///
/// There is no wildcard arm: a new [`Fact`] does not compile until it is rendered.
fn write_fields(fact: &Fact, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    match *fact {
        Fact::ParagraphPrepared {
            clusters,
            candidates,
            constructs,
            fast_measure,
            line_extent,
            writing_mode: mode,
            alignment: align,
        } => write!(
            formatter,
            "clusters={clusters} candidates={candidates} constructs={constructs} \
             fast={fast} extent={line_extent} mode={mode} align={align}",
            fast = flag(fast_measure),
            mode = writing_mode(mode),
            align = alignment(align),
        ),
        Fact::SearchCandidate {
            start_candidate,
            end_candidate,
            natural_width,
            reduced_width,
            available,
            delta,
            badness,
            discretionary,
            warichu,
            formula,
            widow,
            edge_cost,
            total_cost,
            is_last,
            accepted,
        } => write!(
            formatter,
            "start={start_candidate} end={end_candidate} natural={natural_width} \
             reduced={reduced_width} avail={available} delta={delta} badness={badness} \
             disc={discretionary} warichu={warichu} formula={formula} widow={widow} \
             edge={edge_cost} total={total_cost} last={last} accepted={accepted}",
            last = flag(is_last),
            accepted = flag(accepted),
        ),
        Fact::SearchCandidateRefused { candidate } => {
            write!(formatter, "candidate={candidate}")
        },
        Fact::SearchBoundStop {
            start_candidate,
            end_candidate,
            minimum_width,
            available,
            best_cost,
        } => write!(
            formatter,
            "start={start_candidate} end={end_candidate} minimum={minimum_width} \
             avail={available} best={best_cost}"
        ),
        Fact::SearchRefused { charged, limit } => {
            write!(formatter, "charged={charged} limit={limit}")
        },
        Fact::LineChosen {
            line,
            start_candidate,
            end_candidate,
            edge_cost,
        } => write!(
            formatter,
            "line={line} start={start_candidate} end={end_candidate} edge={edge_cost}"
        ),
        Fact::LineFit {
            is_last,
            content_width,
            available,
            remaining,
            cluster_count,
            justify,
            need,
            alignment_offset,
        } => write!(
            formatter,
            "last={last} content={content_width} avail={available} remaining={remaining} \
             clusters={cluster_count} justify={justify} need={need} offset={alignment_offset}",
            last = flag(is_last),
            justify = flag(justify),
        ),
        Fact::ReductionSite {
            boundary,
            weight,
            capacity,
            stage,
            discrete,
        } => write!(
            formatter,
            "boundary={boundary} weight={weight} capacity={capacity} stage={stage} \
             discrete={discrete}",
            discrete = flag(discrete),
        ),
        Fact::ReductionStage {
            stage,
            need,
            discrete_taken,
            capacity,
            taken,
            remaining,
        } => write!(
            formatter,
            "stage={stage} need={need} discrete={discrete_taken} capacity={capacity} \
             taken={taken} remaining={remaining}"
        ),
        Fact::ExpansionSite {
            boundary,
            weight,
            cap,
            stage,
            residual,
        } => {
            write!(formatter, "boundary={boundary} weight={weight}")?;
            if let Some(cap) = cap {
                write!(formatter, " cap={cap}")?;
            }
            if let Some(stage) = stage {
                write!(formatter, " stage={stage}")?;
            }
            write!(formatter, " residual={residual}", residual = flag(residual))
        },
        Fact::ExpansionStage {
            stage,
            sites,
            capacity,
            taken,
            remaining,
        } => write!(
            formatter,
            "stage={stage} sites={sites} capacity={capacity} taken={taken} remaining={remaining}"
        ),
        Fact::ExpansionResidual { sites, amount } => {
            write!(formatter, "sites={sites} amount={amount}")
        },
        Fact::Hanging {
            occupied,
            available,
            amount,
        } => write!(
            formatter,
            "occupied={occupied} avail={available} amount={amount}"
        ),
        Fact::LineFinished {
            inline_origin,
            block_origin,
            inline_extent,
            block_extent,
            clusters,
            attachments,
        } => write!(
            formatter,
            "inline={inline_origin} block={block_origin} extent={inline_extent} \
             block_extent={block_extent} clusters={clusters} attachments={attachments}"
        ),
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;
    use alloc::string::String;
    use alloc::vec::Vec;

    use super::{FORMAT, KIND_WIDTH};
    use crate::model::WritingMode;
    use crate::paragraph::Alignment;
    use crate::trace::{Categories, Fact, RuleAddress, Site, Trace};

    /// One instance of every variant beside the exact line it must render as.
    ///
    /// The field values are deliberately distinct and non-round, so a swapped field, a
    /// dropped separator, or an off-by-one in the padding changes the expected string.
    fn every_rendering() -> Vec<(Site, Fact, &'static str)> {
        alloc::vec![
            (
                Site::paragraph(0..7, 0..21),
                Fact::ParagraphPrepared {
                    clusters: 7,
                    candidates: 5,
                    constructs: 3,
                    fast_measure: true,
                    line_extent: 4_001,
                    writing_mode: WritingMode::VerticalRl,
                    alignment: Alignment::Center,
                },
                "prepare.paragraph        P    c0..7 b0..21 clusters=7 candidates=5 \
                 constructs=3 fast=1 extent=4001 mode=vertical-rl align=center",
            ),
            (
                Site::paragraph(2..9, 6..27),
                Fact::SearchCandidate {
                    start_candidate: 2,
                    end_candidate: 9,
                    natural_width: 4_103,
                    reduced_width: 3_907,
                    available: 4_001,
                    delta: 94,
                    badness: 13,
                    discretionary: 100_000,
                    warichu: 17,
                    formula: 19,
                    widow: 23,
                    edge_cost: 100_072,
                    total_cost: 100_089,
                    is_last: false,
                    accepted: true,
                },
                "search.candidate         P    c2..9 b6..27 start=2 end=9 natural=4103 \
                 reduced=3907 avail=4001 delta=94 badness=13 disc=100000 warichu=17 \
                 formula=19 widow=23 edge=100072 total=100089 last=0 accepted=1 3.8.1",
            ),
            (
                Site::paragraph(4..4, 12..12),
                Fact::SearchCandidateRefused { candidate: 4 },
                "search.refused-candidate P    c4..4 b12..12 candidate=4 3.1.9",
            ),
            (
                Site::paragraph(1..6, 3..18),
                Fact::SearchBoundStop {
                    start_candidate: 1,
                    end_candidate: 6,
                    minimum_width: 4_201,
                    available: 4_001,
                    best_cost: 29,
                },
                "search.bound-stop        P    c1..6 b3..18 start=1 end=6 minimum=4201 \
                 avail=4001 best=29 3.8.1",
            ),
            (
                Site::paragraph(0..7, 0..21),
                Fact::SearchRefused {
                    charged: 67,
                    limit: 67,
                },
                "search.refused           P    c0..7 b0..21 charged=67 limit=67",
            ),
            (
                Site::on_line(3, 5..6, 15..18),
                Fact::LineChosen {
                    line: 3,
                    start_candidate: 2,
                    end_candidate: 5,
                    edge_cost: 31,
                },
                "search.chosen            L03  c5 b15..18 line=3 start=2 end=5 edge=31 3.1.1",
            ),
            (
                Site::on_line(1, 0..5, 0..15),
                Fact::LineFit {
                    is_last: false,
                    content_width: 4_307,
                    available: 4_001,
                    remaining: -306,
                    cluster_count: 5,
                    justify: false,
                    need: -306,
                    alignment_offset: 37,
                },
                "line.fit                 L01  c0..5 b0..15 last=0 content=4307 avail=4001 \
                 remaining=-306 clusters=5 justify=0 need=-306 offset=37 3.8.1",
            ),
            (
                Site::on_line(1, 2..3, 6..9),
                Fact::ReductionSite {
                    boundary: 2,
                    weight: 1_009,
                    capacity: 251,
                    stage: 3,
                    discrete: true,
                },
                "reduce.site              L01  c2 b6..9 boundary=2 weight=1009 capacity=251 \
                 stage=3 discrete=1 3.8.3",
            ),
            (
                Site::on_line(1, 0..5, 0..15),
                Fact::ReductionStage {
                    stage: 4,
                    need: 306,
                    discrete_taken: 41,
                    capacity: 199,
                    taken: 173,
                    remaining: 92,
                },
                "reduce.stage             L01  c0..5 b0..15 stage=4 need=306 discrete=41 \
                 capacity=199 taken=173 remaining=92 3.8.3",
            ),
            (
                Site::on_line(2, 3..4, 9..12),
                Fact::ExpansionSite {
                    boundary: 3,
                    weight: 1_013,
                    cap: Some(257),
                    stage: Some(2),
                    residual: false,
                },
                "expand.site              L02  c3 b9..12 boundary=3 weight=1013 cap=257 stage=2 \
                 residual=0 3.8.3",
            ),
            (
                // A residual-only site states no ceiling and no rung, and the rendering
                // omits both rather than printing a placeholder a reader could misread.
                Site::on_line(2, 4..5, 12..15),
                Fact::ExpansionSite {
                    boundary: 4,
                    weight: 1_019,
                    cap: None,
                    stage: None,
                    residual: true,
                },
                "expand.site              L02  c4 b12..15 boundary=4 weight=1019 residual=1 3.8.3",
            ),
            (
                Site::on_line(2, 0..6, 0..18),
                Fact::ExpansionStage {
                    stage: 3,
                    sites: 2,
                    capacity: 514,
                    taken: 263,
                    remaining: 269,
                },
                "expand.stage             L02  c0..6 b0..18 stage=3 sites=2 capacity=514 \
                 taken=263 remaining=269 3.8.3",
            ),
            (
                Site::on_line(2, 0..6, 0..18),
                Fact::ExpansionResidual {
                    sites: 3,
                    amount: 269,
                },
                "expand.residual          L02  c0..6 b0..18 sites=3 amount=269 3.8.3",
            ),
            (
                Site::on_line(4, 0..7, 0..21),
                Fact::Hanging {
                    occupied: 4_271,
                    available: 4_001,
                    amount: 271,
                },
                "hang.line-end            L04  c0..7 b0..21 occupied=4271 avail=4001 amount=271 \
                 2.5.1",
            ),
            (
                Site::on_line(5, 0..8, 0..24),
                Fact::LineFinished {
                    inline_origin: 37,
                    block_origin: 1_009,
                    inline_extent: 3_989,
                    block_extent: 1_013,
                    clusters: 8,
                    attachments: 2,
                },
                "line.finished            L05  c0..8 b0..24 inline=37 block=1009 extent=3989 \
                 block_extent=1013 clusters=8 attachments=2 3.8.1",
            ),
        ]
    }

    /// Collapse the source-level line continuations the fixtures are wrapped with.
    fn expected(raw: &str) -> String {
        let mut collapsed = String::new();
        let mut pending_space = false;
        for piece in raw.split_whitespace() {
            if pending_space {
                collapsed.push(' ');
            }
            collapsed.push_str(piece);
            pending_space = true;
        }
        collapsed
    }

    #[test]
    fn every_fact_renders_exactly() {
        for (site, fact, wanted) in every_rendering() {
            let mut trace = Trace::with_categories(Categories::ALL);
            trace.push(site, fact);
            let rendered = format!("{}", trace.events()[0]);
            // The padding is what aligns a diff, so compare the collapsed form for the
            // content and the raw form for the column the kind ends in.
            assert_eq!(expected(&rendered), expected(wanted));
            assert!(rendered.len() > KIND_WIDTH);
        }
    }

    #[test]
    fn the_kind_column_is_padded_to_a_fixed_width() {
        for (site, fact, _) in every_rendering() {
            let mut trace = Trace::with_categories(Categories::ALL);
            let kind = fact.kind();
            trace.push(site, fact);
            let rendered = format!("{}", trace.events()[0]);
            assert!(rendered.starts_with(kind));
            let padded = rendered.get(..KIND_WIDTH).unwrap_or_default();
            assert_eq!(padded.trim_end(), kind);
        }
    }

    #[test]
    fn a_trace_renders_a_header_and_one_numbered_line_per_event() {
        let mut trace = Trace::with_categories(Categories::ALL);
        for (site, fact, _) in every_rendering() {
            trace.push(site, fact);
        }
        let rendered = format!("{trace}");
        let mut lines = rendered.lines();
        assert_eq!(
            lines.next(),
            Some("jlreq.trace/1 events=15 categories=0x0fff truncated=0")
        );
        for (ordinal, line) in lines.enumerate() {
            let wanted = format!("{ordinal:04} ");
            assert!(line.starts_with(&wanted));
            assert_eq!(line.trim_end(), line);
        }
        assert!(rendered.ends_with('\n'));
    }

    #[test]
    fn an_empty_trace_renders_its_header_alone() {
        let trace = Trace::with_categories(Categories::NONE);
        let rendered = format!("{trace}");
        assert_eq!(
            rendered,
            format!("{FORMAT} events=0 categories=0x0000 truncated=0\n")
        );
    }

    #[test]
    fn a_truncated_trace_says_so_in_its_header() {
        let mut trace = Trace::with_categories(Categories::ALL);
        trace.set_max_events(0);
        trace.push(
            Site::paragraph(0..1, 0..3),
            Fact::SearchCandidateRefused { candidate: 0 },
        );
        let rendered = format!("{trace}");
        assert!(rendered.contains("truncated=1"));
        assert!(rendered.contains("events=0"));
    }

    #[test]
    fn a_rule_address_renders_in_the_published_grammar() {
        assert_eq!(format!("{}", RuleAddress::Section("3.8.3")), "3.8.3");
        assert_eq!(format!("{}", RuleAddress::Note("C.2", 5)), "C.2#5");
        assert_eq!(
            format!("{}", RuleAddress::Cell("B.1", 5, 5)),
            "B.1@cl-05,cl-05"
        );
        assert_eq!(
            format!("{}", RuleAddress::Cell("E.1", 1, 27)),
            "E.1@cl-01,cl-27"
        );
    }

    #[test]
    fn a_site_names_a_single_cluster_without_a_range() {
        assert_eq!(
            format!("{}", Site::paragraph(4..5, 12..15)),
            "P   c4 b12..15"
        );
        assert_eq!(
            format!("{}", Site::on_line(11, 4..9, 12..27)),
            "L11 c4..9 b12..27"
        );
    }
}
