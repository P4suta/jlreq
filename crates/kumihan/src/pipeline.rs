// SPDX-FileCopyrightText: 2026 kumihan contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use alloc::vec::Vec;
use core::ops::Range;

use crate::{
    Alignment, Attachment, ClusterPlacement, CoordinateTransform, Diagnostic, Frame, Layout, Line,
    Paragraph, PlacementOrigin, Severity, Size, Style, TabAlignment, Widow, WritingMode,
    construct::ConstructKind,
    style::{AdjustmentPreference, KinsokuLevel, Remainder, RubyAlignment},
};

const INFINITE_COST: i64 = i64::MAX / 4;

#[derive(Debug, Clone, Copy)]
struct Candidate {
    offset: usize,
    mandatory: bool,
    discretionary: bool,
}

#[derive(Debug, Clone, Copy)]
struct Node {
    cost: i64,
    previous: usize,
    line_count: usize,
}

/// A reusable whole-paragraph composer.
///
/// All temporary search memory is retained between calls. The returned Layout owns its
/// placements and never borrows this scratch state.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct Composer {
    candidates: Vec<Candidate>,
    nodes: Vec<Node>,
    chosen: Vec<usize>,
    line_advances: Vec<i32>,
}

impl Composer {
    /// Build an empty reusable composer.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            candidates: Vec::new(),
            nodes: Vec::new(),
            chosen: Vec::new(),
            line_advances: Vec::new(),
        }
    }

    /// Normalize, choose breaks globally, and place one validated paragraph.
    #[must_use]
    pub fn compose(&mut self, paragraph: &Paragraph, style: &Style) -> Layout {
        if paragraph.text.clusters().is_empty() {
            return Layout::default();
        }
        self.prepare_candidates(paragraph);
        self.search(paragraph, style);
        self.backtrack();
        self.place(paragraph, style)
    }

    fn prepare_candidates(&mut self, paragraph: &Paragraph) {
        self.candidates.clear();
        self.candidates.push(Candidate {
            offset: 0,
            mandatory: true,
            discretionary: false,
        });
        self.candidates
            .extend(paragraph.breaks.iter().map(|opportunity| Candidate {
                offset: opportunity.offset(),
                mandatory: opportunity.is_mandatory(),
                discretionary: opportunity.is_discretionary(),
            }));
    }

    fn search(&mut self, paragraph: &Paragraph, style: &Style) {
        self.nodes.clear();
        self.nodes.resize(
            self.candidates.len(),
            Node {
                cost: INFINITE_COST,
                previous: 0,
                line_count: 0,
            },
        );
        self.nodes[0] = Node {
            cost: 0,
            previous: 0,
            line_count: 0,
        };

        for end in 1..self.candidates.len() {
            let candidate = self.candidates[end];
            if !candidate.mandatory && !break_is_legal(paragraph, style, candidate.offset) {
                continue;
            }
            for start in 0..end {
                if self.nodes[start].cost == INFINITE_COST
                    || self.candidates[start.saturating_add(1)..end]
                        .iter()
                        .any(|inner| inner.mandatory)
                {
                    continue;
                }
                let line_number = self.nodes[start].line_count;
                let width = measure_line(
                    paragraph,
                    self.candidates[start].offset,
                    candidate.offset,
                    line_number,
                );
                let available = i64::from(paragraph.line_extent);
                let delta = available.saturating_sub(width);
                let is_last = end.saturating_add(1) == self.candidates.len();
                let mut cost = line_badness(delta, is_last, style.adjustment_preference());
                if candidate.discretionary {
                    cost = cost.saturating_add(100_000);
                }
                if is_last {
                    cost = cost.saturating_add(widow_penalty(
                        paragraph,
                        self.candidates[start].offset,
                        candidate.offset,
                    ));
                }
                cost = cost.saturating_add(self.nodes[start].cost);
                if cost < self.nodes[end].cost {
                    self.nodes[end] = Node {
                        cost,
                        previous: start,
                        line_count: line_number.saturating_add(1),
                    };
                }
            }
        }

        let last = self.nodes.len().saturating_sub(1);
        if self.nodes[last].cost == INFINITE_COST {
            self.nodes[last] = Node {
                cost: 0,
                previous: 0,
                line_count: 1,
            };
        }
    }

    fn backtrack(&mut self) {
        self.chosen.clear();
        let mut cursor = self.nodes.len().saturating_sub(1);
        self.chosen.push(cursor);
        while cursor != 0 {
            let previous = self.nodes[cursor].previous;
            if previous == cursor {
                break;
            }
            cursor = previous;
            self.chosen.push(cursor);
        }
        self.chosen.reverse();
    }

    fn place(&mut self, paragraph: &Paragraph, style: &Style) -> Layout {
        let mut layout = Layout::default();
        let mut block_cursor = 0_i64;
        for line_index in 0..self.chosen.len().saturating_sub(1) {
            let start_offset = self.candidates[self.chosen[line_index]].offset;
            let next_line = line_index.saturating_add(1);
            let end_offset = self.candidates[self.chosen[next_line]].offset;
            let start_cluster = cluster_index_at_or_after(paragraph, start_offset);
            let end_cluster = cluster_index_at_or_after(paragraph, end_offset);
            let is_last = line_index.saturating_add(2) == self.chosen.len();
            let block_origin = match paragraph.writing_mode {
                WritingMode::HorizontalTb => clamp_i32(block_cursor),
                WritingMode::VerticalRl => clamp_i32(block_cursor.saturating_neg()),
            };
            let line = self.place_line(
                paragraph,
                style,
                start_cluster..end_cluster,
                line_index,
                block_origin,
                is_last,
            );
            if i64::from(line.inline_extent) > i64::from(paragraph.line_extent) {
                layout.diagnostics.push(Diagnostic {
                    code: "layout.overfull",
                    severity: Severity::Warning,
                    range: Some(line.range.clone()),
                    jlreq: "3.8.1",
                });
            }
            block_cursor = block_cursor.saturating_add(i64::from(line.block_extent.max(1)));
            layout.lines.push(line);
        }
        add_widow_diagnostic(paragraph, &mut layout);
        layout
    }

    fn place_line(
        &mut self,
        paragraph: &Paragraph,
        style: &Style,
        cluster_range: Range<usize>,
        line_index: usize,
        block_origin: i32,
        is_last: bool,
    ) -> Line {
        let start_cluster = cluster_range.start;
        let end_cluster = cluster_range.end;
        self.line_advances.clear();
        let clusters = &paragraph.text.clusters()[start_cluster..end_cluster];
        self.line_advances.extend(
            (start_cluster..end_cluster)
                .map(|ordinal| effective_cluster_advance(paragraph, ordinal)),
        );
        apply_tabs(
            paragraph,
            start_cluster,
            end_cluster,
            line_index,
            &mut self.line_advances,
        );

        let indent = if line_index == 0 {
            paragraph.first_line_indent
        } else {
            0
        };
        let content_width = self
            .line_advances
            .iter()
            .fold(i64::from(indent), |sum, advance| {
                sum.saturating_add(i64::from(*advance))
            });
        let remaining = i64::from(paragraph.line_extent).saturating_sub(content_width);
        let alignment_offset = match paragraph.alignment {
            Alignment::Start | Alignment::Justify => 0,
            Alignment::Center => remaining.max(0) / 2,
            Alignment::End => remaining.max(0),
        };
        let justify = paragraph.alignment == Alignment::Justify
            && !is_last
            && remaining > 0
            && clusters.len() > 1;
        let logical_item_count = (start_cluster..end_cluster)
            .filter(|ordinal| !is_tate_chu_yoko_continuation(paragraph, *ordinal))
            .count();
        let gap_count = logical_item_count.saturating_sub(1);
        let gap = if justify {
            remaining
                .checked_div(i64::try_from(gap_count).unwrap_or(1))
                .unwrap_or(0)
        } else {
            0
        };
        let remainder = if justify {
            remaining
                .checked_rem(i64::try_from(gap_count).unwrap_or(1))
                .unwrap_or(0)
        } else {
            0
        };

        let mut placed = Vec::with_capacity(clusters.len());
        let mut cursor = i64::from(indent).saturating_add(alignment_offset);
        let mut block_extent = paragraph.text.size().block();
        let mut local = 0;
        let mut gap_ordinal = 0_usize;
        while local < clusters.len() {
            let ordinal = start_cluster.saturating_add(local);
            if let Some(group) = tate_chu_yoko_cluster_range(paragraph, ordinal)
                && group.start == ordinal
            {
                let group_end = group.end.min(end_cluster);
                let member_count = group_end.saturating_sub(ordinal);
                let horizontal_width = paragraph.text.clusters()[ordinal..group_end]
                    .iter()
                    .fold(0_i64, |sum, cluster| {
                        sum.saturating_add(i64::from(cluster.advance()))
                    });
                block_extent = block_extent.max(clamp_i32(horizontal_width));
                let mut member_block = i64::from(block_origin)
                    .saturating_sub(horizontal_width.checked_div(2).unwrap_or(0));
                for (member_local, cluster) in paragraph.text.clusters()[ordinal..group_end]
                    .iter()
                    .enumerate()
                {
                    let member_ordinal = ordinal.saturating_add(member_local);
                    let size = cluster.size_override().unwrap_or(paragraph.text.size());
                    let frame = cluster.frame_override().unwrap_or(paragraph.text.frame());
                    let (writing_mode, transform) =
                        local_orientation(paragraph, member_ordinal, frame);
                    placed.push(ClusterPlacement {
                        origin: PlacementOrigin::Cluster(member_ordinal),
                        range: cluster.range(),
                        inline: clamp_i32(cursor),
                        block: clamp_i32(member_block),
                        advance: cluster.advance(),
                        size,
                        frame,
                        writing_mode,
                        transform,
                    });
                    member_block = member_block.saturating_add(i64::from(cluster.advance()));
                }
                cursor = cursor.saturating_add(i64::from(self.line_advances[local]));
                local = local.saturating_add(member_count);
            } else {
                let cluster = &clusters[local];
                let advance = self.line_advances[local];
                let size = cluster.size_override().unwrap_or(paragraph.text.size());
                let frame = cluster.frame_override().unwrap_or(paragraph.text.frame());
                block_extent = block_extent.max(size.block());
                let (writing_mode, transform) = local_orientation(paragraph, ordinal, frame);
                placed.push(ClusterPlacement {
                    origin: PlacementOrigin::Cluster(ordinal),
                    range: cluster.range(),
                    inline: clamp_i32(cursor),
                    block: block_origin,
                    advance,
                    size,
                    frame,
                    writing_mode,
                    transform,
                });
                cursor = cursor.saturating_add(i64::from(advance));
                local = local.saturating_add(1);
            }

            if local < clusters.len() {
                cursor = cursor.saturating_add(gap);
                let receives_remainder = match style.remainder() {
                    Remainder::Leading => {
                        i64::try_from(gap_ordinal).unwrap_or(i64::MAX) < remainder
                    },
                    Remainder::Trailing => {
                        i64::try_from(gap_count.saturating_sub(gap_ordinal.saturating_add(1)))
                            .unwrap_or(i64::MAX)
                            < remainder
                    },
                };
                if receives_remainder {
                    cursor = cursor.saturating_add(1);
                }
                gap_ordinal = gap_ordinal.saturating_add(1);
            }
        }

        let range = if let (Some(first), Some(last)) = (clusters.first(), clusters.last()) {
            first.range().start..last.range().end
        } else {
            0..0
        };
        let mut line = Line {
            range,
            inline_origin: clamp_i32(alignment_offset),
            block_origin,
            inline_extent: clamp_i32(cursor.saturating_sub(alignment_offset)),
            block_extent,
            clusters: placed,
            attachments: Vec::new(),
        };
        place_attachments(paragraph, style, &mut line);
        line
    }
}

fn cluster_index_at_or_after(paragraph: &Paragraph, offset: usize) -> usize {
    paragraph
        .text
        .clusters()
        .partition_point(|cluster| cluster.range().start < offset)
}

fn tate_chu_yoko_cluster_range(paragraph: &Paragraph, ordinal: usize) -> Option<Range<usize>> {
    if paragraph.writing_mode != WritingMode::VerticalRl {
        return None;
    }
    let cluster = paragraph.text.clusters().get(ordinal)?.range();
    let range = paragraph.constructs.iter().find_map(|construct| {
        let ConstructKind::TateChuYoko(range) = construct.kind() else {
            return None;
        };
        (range.start <= cluster.start && cluster.end <= range.end).then_some(range)
    })?;
    Some(
        cluster_index_at_or_after(paragraph, range.start)
            ..cluster_index_at_or_after(paragraph, range.end),
    )
}

fn is_tate_chu_yoko_continuation(paragraph: &Paragraph, ordinal: usize) -> bool {
    tate_chu_yoko_cluster_range(paragraph, ordinal).is_some_and(|group| group.start != ordinal)
}

fn effective_cluster_advance(paragraph: &Paragraph, ordinal: usize) -> i32 {
    let advance = if let Some(group) = tate_chu_yoko_cluster_range(paragraph, ordinal) {
        if group.start != ordinal {
            return 0;
        }
        paragraph.text.clusters()[group]
            .iter()
            .map(|cluster| {
                cluster
                    .size_override()
                    .unwrap_or(paragraph.text.size())
                    .block()
            })
            .max()
            .unwrap_or(0)
    } else {
        paragraph.text.clusters()[ordinal].advance()
    };
    advance.saturating_add(tate_chu_yoko_boundary_space_after(paragraph, ordinal))
}

fn tate_chu_yoko_boundary_space_after(paragraph: &Paragraph, ordinal: usize) -> i32 {
    if paragraph.writing_mode != WritingMode::VerticalRl {
        return 0;
    }
    let clusters = paragraph.text.clusters();
    let current_group = tate_chu_yoko_cluster_range(paragraph, ordinal);
    if let Some(group) = current_group {
        if group.start != ordinal || group.end >= clusters.len() {
            return 0;
        }
        let following = &clusters[group.end];
        let character = single_cluster_character(paragraph, following);
        if character.is_some_and(is_opening_bracket) {
            return half_inline_size(paragraph, following);
        }
        return 0;
    }

    let following_ordinal = ordinal.saturating_add(1);
    if following_ordinal >= clusters.len()
        || tate_chu_yoko_cluster_range(paragraph, following_ordinal)
            .is_none_or(|group| group.start != following_ordinal)
    {
        return 0;
    }
    let current = &clusters[ordinal];
    let character = single_cluster_character(paragraph, current);
    if character.is_some_and(|character| {
        is_closing_bracket(character) || is_full_stop(character) || is_comma(character)
    }) {
        half_inline_size(paragraph, current)
    } else {
        0
    }
}

fn single_cluster_character(paragraph: &Paragraph, cluster: &crate::Cluster) -> Option<char> {
    let mut characters = paragraph.text.source()[cluster.range()].chars();
    let character = characters.next()?;
    characters.next().is_none().then_some(character)
}

fn half_inline_size(paragraph: &Paragraph, cluster: &crate::Cluster) -> i32 {
    let size = cluster
        .size_override()
        .unwrap_or(paragraph.text.size())
        .inline();
    (size / 2).saturating_add(size % 2)
}

fn is_opening_bracket(character: char) -> bool {
    "（([｛〔〈《「『【〘〖〝‘“｟«".contains(character)
}

fn is_closing_bracket(character: char) -> bool {
    "）)]｝〕〉》」』】〙〗〟’”｠»".contains(character)
}

fn is_full_stop(character: char) -> bool {
    "。．".contains(character)
}

fn is_comma(character: char) -> bool {
    "、，".contains(character)
}

fn measure_line(paragraph: &Paragraph, start: usize, end: usize, line_number: usize) -> i64 {
    let start_cluster = cluster_index_at_or_after(paragraph, start);
    let end_cluster = cluster_index_at_or_after(paragraph, end);
    let mut cursor = if line_number == 0 {
        i64::from(paragraph.first_line_indent)
    } else {
        0
    };
    let mut tab_index = 0;
    for (local, cluster) in paragraph.text.clusters()[start_cluster..end_cluster]
        .iter()
        .enumerate()
    {
        if &paragraph.text.source()[cluster.range()] == "\t" {
            let after_tab = start_cluster.saturating_add(local).saturating_add(1);
            let segment_width = segment_width(paragraph, after_tab, end_cluster);
            if let Some(stop) = paragraph
                .tab_stops
                .iter()
                .skip(tab_index)
                .find(|stop| i64::from(stop.position()) > cursor)
            {
                tab_index = tab_index.saturating_add(1);
                let target = tab_target(paragraph, *stop, after_tab, end_cluster, segment_width);
                cursor = cursor.max(target);
            }
        } else {
            let ordinal = start_cluster.saturating_add(local);
            cursor =
                cursor.saturating_add(i64::from(effective_cluster_advance(paragraph, ordinal)));
        }
    }
    cursor
}

fn segment_width(paragraph: &Paragraph, start: usize, end: usize) -> i64 {
    paragraph.text.clusters()[start..end]
        .iter()
        .enumerate()
        .take_while(|(_, cluster)| &paragraph.text.source()[cluster.range()] != "\t")
        .fold(0_i64, |sum, (local, _)| {
            sum.saturating_add(i64::from(effective_cluster_advance(
                paragraph,
                start.saturating_add(local),
            )))
        })
}

fn tab_target(
    paragraph: &Paragraph,
    stop: crate::TabStop,
    start: usize,
    end: usize,
    segment_width: i64,
) -> i64 {
    let position = i64::from(stop.position());
    match stop.alignment() {
        TabAlignment::Start => position,
        TabAlignment::Center => position.saturating_sub(segment_width / 2),
        TabAlignment::End => position.saturating_sub(segment_width),
        TabAlignment::Character(character) => {
            let before = paragraph.text.clusters()[start..end]
                .iter()
                .enumerate()
                .take_while(|cluster| {
                    !paragraph.text.source()[cluster.1.range()].contains(character)
                        && &paragraph.text.source()[cluster.1.range()] != "\t"
                })
                .fold(0_i64, |sum, (local, _)| {
                    sum.saturating_add(i64::from(effective_cluster_advance(
                        paragraph,
                        start.saturating_add(local),
                    )))
                });
            position.saturating_sub(before)
        },
    }
}

fn apply_tabs(
    paragraph: &Paragraph,
    start: usize,
    end: usize,
    line_number: usize,
    advances: &mut [i32],
) {
    let mut cursor = if line_number == 0 {
        i64::from(paragraph.first_line_indent)
    } else {
        0
    };
    let mut tab_index = 0;
    for local in 0..advances.len() {
        let ordinal = start.saturating_add(local);
        let after_tab = local.saturating_add(1);
        let cluster = &paragraph.text.clusters()[ordinal];
        if &paragraph.text.source()[cluster.range()] == "\t" {
            let width = advances[after_tab..]
                .iter()
                .zip(&paragraph.text.clusters()[ordinal.saturating_add(1)..end])
                .take_while(|(_, following)| &paragraph.text.source()[following.range()] != "\t")
                .fold(0_i64, |sum, (advance, _)| {
                    sum.saturating_add(i64::from(*advance))
                });
            if let Some(stop) = paragraph
                .tab_stops
                .iter()
                .skip(tab_index)
                .find(|stop| i64::from(stop.position()) > cursor)
            {
                tab_index = tab_index.saturating_add(1);
                let target = tab_target(paragraph, *stop, ordinal.saturating_add(1), end, width);
                advances[local] = clamp_i32(target.saturating_sub(cursor).max(0));
            } else {
                advances[local] = paragraph.text.size().inline();
            }
        }
        cursor = cursor.saturating_add(i64::from(advances[local]));
    }
}

fn line_badness(delta: i64, is_last: bool, preference: AdjustmentPreference) -> i64 {
    let magnitude = delta.saturating_abs().min(1_000_000);
    let square = magnitude.saturating_mul(magnitude);
    if delta < 0 {
        return square.saturating_mul(1_000).saturating_add(10_000_000);
    }
    if is_last {
        return square / 100;
    }
    match preference {
        AdjustmentPreference::LeastAdjustment => square,
        AdjustmentPreference::EvenTexture => square.saturating_mul(2),
    }
}

fn widow_penalty(paragraph: &Paragraph, start: usize, end: usize) -> i64 {
    let Widow::MinimumClusters(minimum) = paragraph.widow else {
        return 0;
    };
    let count = cluster_index_at_or_after(paragraph, end)
        .saturating_sub(cluster_index_at_or_after(paragraph, start));
    if count < usize::from(minimum) {
        1_000_000_000
    } else {
        0
    }
}

fn add_widow_diagnostic(paragraph: &Paragraph, layout: &mut Layout) {
    let Widow::MinimumClusters(minimum) = paragraph.widow else {
        return;
    };
    let Some(last) = layout.lines.last() else {
        return;
    };
    if last.clusters.len() < usize::from(minimum) {
        layout.diagnostics.push(Diagnostic {
            code: "layout.widow",
            severity: Severity::Warning,
            range: Some(last.range.clone()),
            jlreq: "3.1.9",
        });
    }
}

fn break_is_legal(paragraph: &Paragraph, style: &Style, offset: usize) -> bool {
    if offset == paragraph.text.source().len() || style.kinsoku_level() == KinsokuLevel::VeryLoose {
        return true;
    }
    let before = paragraph.text.source()[..offset].chars().next_back();
    let after = paragraph.text.source()[offset..].chars().next();
    !before.is_some_and(is_line_end_prohibited)
        && !after.is_some_and(|character| is_line_head_prohibited(character, style.kinsoku_level()))
}

fn is_line_end_prohibited(character: char) -> bool {
    "（([｛〔〈《「『【〘〖〝‘“｟«".contains(character)
}

fn is_line_head_prohibited(character: char, level: KinsokuLevel) -> bool {
    let ordinary = "、。，．・：；？！‼⁇⁈⁉)]｝〕〉》」』】〙〗〟’”｠»";
    if ordinary.contains(character) {
        return true;
    }
    level != KinsokuLevel::Loose
        && "ぁぃぅぇぉっゃゅょゎァィゥェォッャュョヮヵヶ々ー".contains(character)
}

fn local_orientation(
    paragraph: &Paragraph,
    ordinal: usize,
    frame: Frame,
) -> (WritingMode, CoordinateTransform) {
    if paragraph.writing_mode == WritingMode::HorizontalTb {
        return (WritingMode::HorizontalTb, CoordinateTransform::Identity);
    }
    let range = paragraph.text.clusters()[ordinal].range();
    if paragraph.constructs.iter().any(|construct| {
        matches!(construct.kind(), ConstructKind::TateChuYoko(_))
            && ranges_overlap(&construct.range(), &range)
    }) {
        return (WritingMode::HorizontalTb, CoordinateTransform::TateChuYoko);
    }
    if frame == Frame::Proportional {
        (
            WritingMode::VerticalRl,
            CoordinateTransform::RotateClockwise,
        )
    } else {
        (WritingMode::VerticalRl, CoordinateTransform::Identity)
    }
}

fn place_attachments(paragraph: &Paragraph, style: &Style, line: &mut Line) {
    let mut attachment_extent = 0;
    for (ordinal, construct) in paragraph.constructs.iter().enumerate() {
        if !ranges_overlap(&construct.range(), &line.range) {
            continue;
        }
        match construct.kind() {
            ConstructKind::Ruby(ruby) => {
                let base = bounds_for_range(line, &ruby.base());
                if let Some((base_start, base_end)) = base {
                    let annotation_width = ruby
                        .annotation()
                        .clusters()
                        .iter()
                        .fold(0_i64, |sum, cluster| {
                            sum.saturating_add(i64::from(cluster.advance()))
                        });
                    let base_width = i64::from(base_end).saturating_sub(i64::from(base_start));
                    let mut inline = match style.ruby_alignment() {
                        RubyAlignment::Nakatsuki => i64::from(base_start)
                            .saturating_add((base_width.saturating_sub(annotation_width)) / 2),
                        RubyAlignment::Katatsuki => i64::from(base_start),
                    };
                    for cluster in ruby.annotation().clusters() {
                        let size = cluster.size_override().unwrap_or(ruby.annotation().size());
                        line.attachments.push(Attachment {
                            construct: ordinal,
                            range: cluster.range(),
                            inline: clamp_i32(inline),
                            block: attachment_block(paragraph, line, size),
                            advance: cluster.advance(),
                            size,
                            writing_mode: paragraph.writing_mode,
                            transform: CoordinateTransform::Identity,
                            symbol: None,
                        });
                        attachment_extent = attachment_extent.max(size.block());
                        inline = inline.saturating_add(i64::from(cluster.advance()));
                    }
                }
            },
            ConstructKind::Emphasis { range, mark } => {
                for placement in line
                    .clusters
                    .iter()
                    .filter(|placement| ranges_overlap(&placement.range, range))
                {
                    let size = placement.size.half_rounded_up();
                    let inline = i64::from(placement.inline).saturating_add(
                        i64::from(placement.advance).saturating_sub(i64::from(size.inline())) / 2,
                    );
                    line.attachments.push(Attachment {
                        construct: ordinal,
                        range: 0..0,
                        inline: clamp_i32(inline),
                        block: attachment_block(paragraph, line, size),
                        advance: 0,
                        size,
                        writing_mode: paragraph.writing_mode,
                        transform: CoordinateTransform::Identity,
                        symbol: Some(*mark),
                    });
                    attachment_extent = attachment_extent.max(size.block());
                }
            },
            ConstructKind::ReferenceMark { range, mark }
            | ConstructKind::Script {
                range,
                annotation: mark,
            } => {
                if let Some((start, end)) = bounds_for_range(line, range) {
                    let width = mark.clusters().iter().fold(0_i64, |sum, cluster| {
                        sum.saturating_add(i64::from(cluster.advance()))
                    });
                    let mut inline = i64::from(start).saturating_add(
                        (i64::from(end)
                            .saturating_sub(i64::from(start))
                            .saturating_sub(width))
                            / 2,
                    );
                    for cluster in mark.clusters() {
                        let size = cluster.size_override().unwrap_or(mark.size());
                        line.attachments.push(Attachment {
                            construct: ordinal,
                            range: cluster.range(),
                            inline: clamp_i32(inline),
                            block: attachment_block(paragraph, line, size),
                            advance: cluster.advance(),
                            size,
                            writing_mode: paragraph.writing_mode,
                            transform: CoordinateTransform::Identity,
                            symbol: None,
                        });
                        attachment_extent = attachment_extent.max(size.block());
                        inline = inline.saturating_add(i64::from(cluster.advance()));
                    }
                }
            },
            ConstructKind::TateChuYoko(_)
            | ConstructKind::Warichu(_)
            | ConstructKind::Furawake { .. }
            | ConstructKind::Jidori { .. }
            | ConstructKind::Formula(_) => {},
        }
    }
    line.block_extent = line.block_extent.saturating_add(attachment_extent);
}

fn attachment_block(paragraph: &Paragraph, line: &Line, size: Size) -> i32 {
    match paragraph.writing_mode {
        WritingMode::HorizontalTb => line.block_origin.saturating_sub(size.block()),
        WritingMode::VerticalRl => line.block_origin.saturating_add(size.block()),
    }
}

fn bounds_for_range(line: &Line, range: &Range<usize>) -> Option<(i32, i32)> {
    let mut matching = line
        .clusters
        .iter()
        .filter(|placement| ranges_overlap(&placement.range, range));
    let first = matching.next()?;
    let mut end = placement_inline_end(first);
    for placement in matching {
        end = end.max(placement_inline_end(placement));
    }
    Some((first.inline, end))
}

fn placement_inline_end(placement: &ClusterPlacement) -> i32 {
    let advance = if placement.transform == CoordinateTransform::TateChuYoko {
        placement.size.block()
    } else {
        placement.advance
    };
    placement.inline.saturating_add(advance)
}

fn ranges_overlap(left: &Range<usize>, right: &Range<usize>) -> bool {
    left.start < right.end && right.start < left.end
}

fn clamp_i32(value: i64) -> i32 {
    i32::try_from(value).unwrap_or_else(|_| {
        if value.is_negative() {
            i32::MIN
        } else {
            i32::MAX
        }
    })
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use crate::{Break, Cluster, Frame, Paragraph, ShapedText, Size, Style, Widow, WritingMode};

    fn text(source: &str) -> ShapedText {
        let clusters = source.char_indices().map(|(start, character)| {
            Cluster::new(start..start.saturating_add(character.len_utf8()), 1_000)
        });
        ShapedText::new(
            source,
            Size::square(1_000).expect("positive fixture size"),
            Frame::FullEm,
            clusters,
        )
        .expect("valid fixture text")
    }

    #[test]
    fn optimal_search_uses_the_whole_paragraph() {
        let source = "日本語組版";
        let paragraph = Paragraph::builder(text(source), 4_000)
            .breaks(
                source
                    .char_indices()
                    .skip(1)
                    .map(|(offset, _)| Break::allowed(offset)),
            )
            .widow(Widow::MinimumClusters(2))
            .build()
            .expect("valid paragraph");
        let layout = crate::compose(&paragraph, &Style::default());
        assert_eq!(layout.lines().len(), 2);
        assert_eq!(layout.lines()[0].clusters().len(), 3);
    }

    #[test]
    fn vertical_lines_progress_toward_negative_block_coordinates() {
        let paragraph = Paragraph::builder(text("日本"), 1_000)
            .breaks(vec![Break::allowed(3)])
            .writing_mode(WritingMode::VerticalRl)
            .build()
            .expect("valid paragraph");
        let layout = crate::compose(&paragraph, &Style::default());
        assert_eq!(layout.lines().len(), 2);
        assert!(layout.lines()[1].block_origin() < layout.lines()[0].block_origin());
    }
}
