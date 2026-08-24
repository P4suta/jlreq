// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use alloc::{vec, vec::Vec};
use core::ops::Range;

use crate::construct::{
    ConstructKind, Ruby, RubyKind, is_math_operator, is_math_symbol, is_math_token,
};
use crate::layout::{
    Attachment, ClusterPlacement, CoordinateTransform, Diagnostic, Layout, Line, PlacementOrigin,
    Severity,
};
use crate::model::{ClusterRole, Frame, Size, WritingMode};
use crate::paragraph::{Alignment, Paragraph, TabAlignment, TabStop, Widow};
use crate::style::{
    AdjustmentPreference, AmbiguousContext, GroupRubyDistribution, GroupedNumeralBeforeWestern,
    GroupedNumeralQualification, HangingPunctuation, IterationMarkAtLineHead,
    JapaneseLatinExpansionCeiling, JukugoRubyLayout, KinsokuLevel, LineEndFullStopComma,
    LineEndPunctuation, LineHeadOpeningBracket, ReductionTable, RelaxationMechanism, Remainder,
    RubyAlignment, RubyOverhangIndent, RubyOverhangKana, SentenceMedialDividingMark, Style,
    UnlistedCodePoint,
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

#[derive(Debug, Clone)]
struct WarichuSegment {
    range: Range<usize>,
    leading_bracket: Option<usize>,
    first_lane: Range<usize>,
    second_lane: Range<usize>,
    trailing_bracket: Option<usize>,
    first_width: i32,
    second_width: i32,
    advance: i32,
}

#[derive(Debug, Clone)]
struct FurawakeSegment {
    range: Range<usize>,
    lanes: Vec<Range<usize>>,
    block_extents: Vec<i32>,
    line_gap: i32,
    advance: i32,
    block_extent: i32,
}

#[derive(Debug, Clone)]
struct JidoriPlan {
    range: Range<usize>,
    extra_after: Vec<i32>,
}

impl JidoriPlan {
    fn extra_after(&self, ordinal: usize) -> i32 {
        ordinal
            .checked_sub(self.range.start)
            .and_then(|local| self.extra_after.get(local))
            .copied()
            .unwrap_or(0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ComplexKind {
    Ornamented,
    SimpleRuby,
    JukugoRuby,
    TateChuYoko,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ComplexIdentity {
    kind: ComplexKind,
    construct: usize,
    member: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExpansionSite {
    None,
    Site {
        weight: i32,
        bounded: Option<(i32, u8)>,
        residual: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReductionSite {
    boundary: usize,
    weight: i32,
    capacity: i32,
    stage: u8,
    discrete: bool,
}

#[derive(Debug, Clone, Copy)]
struct LineContext {
    start: usize,
    end: usize,
    index: usize,
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
    line_adjustments: Vec<i32>,
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
            line_adjustments: Vec::new(),
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
        self.candidates.extend(
            paragraph
                .breaks
                .iter()
                .filter(|opportunity| !is_internal_furawake_offset(paragraph, opportunity.offset()))
                .map(|opportunity| Candidate {
                    offset: opportunity.offset(),
                    mandatory: opportunity.is_mandatory(),
                    discretionary: opportunity.is_discretionary(),
                }),
        );
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
                let measured_width = measure_line(
                    paragraph,
                    style,
                    self.candidates[start].offset,
                    candidate.offset,
                    line_number,
                );
                let available = i64::from(paragraph.line_extent);
                let width = width_after_available_reduction(
                    paragraph,
                    style,
                    self.candidates[start].offset,
                    candidate.offset,
                    measured_width,
                    available,
                );
                let delta = available.saturating_sub(width);
                let is_last = end.saturating_add(1) == self.candidates.len();
                let mut cost = line_badness(delta, is_last, style.adjustment_preference());
                if candidate.discretionary {
                    cost = cost.saturating_add(100_000);
                }
                cost = cost.saturating_add(warichu_break_penalty(paragraph, candidate.offset));
                cost = cost.saturating_add(formula_break_penalty(paragraph, candidate.offset));
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
        self.line_advances
            .extend((start_cluster..end_cluster).map(|ordinal| {
                effective_cluster_advance_on_line(
                    paragraph,
                    style,
                    ordinal,
                    start_cluster,
                    end_cluster,
                    line_index,
                )
            }));
        apply_tabs(
            paragraph,
            style,
            start_cluster,
            end_cluster,
            line_index,
            &mut self.line_advances,
        );

        let indent = line_head_indent(paragraph, style, start_cluster, line_index);
        let ruby_leading =
            ruby_line_leading_separation(paragraph, style, start_cluster, end_cluster, line_index);
        let content_width = self.line_advances.iter().fold(
            i64::from(indent.saturating_add(ruby_leading)),
            |sum, advance| sum.saturating_add(i64::from(*advance)),
        );
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
        prepare_line_adjustments(
            paragraph,
            style,
            start_cluster,
            end_cluster,
            if remaining < 0 || justify {
                remaining
            } else {
                0
            },
            &mut self.line_adjustments,
        );

        let mut placed = Vec::with_capacity(clusters.len());
        let mut cursor = i64::from(indent)
            .saturating_add(i64::from(ruby_leading))
            .saturating_add(alignment_offset);
        let mut block_extent = paragraph.text.size().block();
        let mut local = 0;
        while local < clusters.len() {
            let ordinal = start_cluster.saturating_add(local);
            let previous_ordinal;
            if let Some((group, columns, line_gap)) = furawake_cluster_range(paragraph, ordinal)
                .filter(|(group, _, _)| group.start == ordinal)
            {
                let segment = furawake_segment(paragraph, group, columns, line_gap, end_cluster);
                previous_ordinal = segment.range.end.saturating_sub(1);
                block_extent = block_extent.max(segment.block_extent);
                place_furawake_segment(paragraph, &segment, cursor, block_origin, &mut placed);
                cursor = cursor.saturating_add(i64::from(self.line_advances[local]));
                local = local.saturating_add(segment.range.end.saturating_sub(ordinal));
            } else if let Some(group) = warichu_cluster_range(paragraph, ordinal)
                .filter(|group| group.start.max(start_cluster) == ordinal)
            {
                let segment = warichu_segment(paragraph, group, start_cluster, end_cluster);
                previous_ordinal = segment.range.end.saturating_sub(1);
                place_warichu_segment(paragraph, &segment, cursor, block_origin, &mut placed);
                cursor = cursor.saturating_add(i64::from(segment.advance));
                local = local.saturating_add(segment.range.end.saturating_sub(ordinal));
            } else if let Some(group) = tate_chu_yoko_cluster_range(paragraph, ordinal)
                .filter(|group| group.start == ordinal)
            {
                let group_end = group.end.min(end_cluster);
                let member_count = group_end.saturating_sub(ordinal);
                previous_ordinal = group_end.saturating_sub(1);
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
                previous_ordinal = ordinal;
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

            let boundary = previous_ordinal.saturating_sub(start_cluster);
            cursor = cursor.saturating_add(i64::from(
                self.line_adjustments.get(boundary).copied().unwrap_or(0),
            ));
        }

        let range = if let (Some(first), Some(last)) = (clusters.first(), clusters.last()) {
            first.range().start..last.range().end
        } else {
            0..0
        };
        let occupied = cursor.saturating_sub(alignment_offset);
        let hanging = hanging_amount(
            paragraph,
            style,
            end_cluster,
            occupied,
            i64::from(paragraph.line_extent),
        );
        let mut line = Line {
            range,
            inline_origin: clamp_i32(alignment_offset),
            block_origin,
            inline_extent: clamp_i32(occupied.saturating_sub(hanging)),
            block_extent,
            clusters: placed,
            attachments: Vec::new(),
        };
        place_attachments(paragraph, style, line_index, &mut line);
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

fn warichu_cluster_range(paragraph: &Paragraph, ordinal: usize) -> Option<Range<usize>> {
    let cluster = paragraph.text.clusters().get(ordinal)?.range();
    let range = paragraph.constructs.iter().find_map(|construct| {
        let ConstructKind::Warichu(range) = construct.kind() else {
            return None;
        };
        (range.start <= cluster.start && cluster.end <= range.end).then_some(range)
    })?;
    Some(
        cluster_index_at_or_after(paragraph, range.start)
            ..cluster_index_at_or_after(paragraph, range.end),
    )
}

fn furawake_cluster_range(
    paragraph: &Paragraph,
    ordinal: usize,
) -> Option<(Range<usize>, u16, i32)> {
    let cluster = paragraph.text.clusters().get(ordinal)?.range();
    let (range, columns, line_gap) = paragraph.constructs.iter().find_map(|construct| {
        let ConstructKind::Furawake {
            range,
            columns,
            line_gap,
        } = construct.kind()
        else {
            return None;
        };
        (range.start <= cluster.start && cluster.end <= range.end)
            .then_some((range, *columns, *line_gap))
    })?;
    Some((
        cluster_index_at_or_after(paragraph, range.start)
            ..cluster_index_at_or_after(paragraph, range.end),
        columns,
        line_gap,
    ))
}

fn jidori_cluster_range(paragraph: &Paragraph, ordinal: usize) -> Option<(Range<usize>, u16)> {
    let cluster = paragraph.text.clusters().get(ordinal)?.range();
    let (range, cells) = paragraph.constructs.iter().find_map(|construct| {
        let ConstructKind::Jidori { range, cells } = construct.kind() else {
            return None;
        };
        (range.start <= cluster.start && cluster.end <= range.end).then_some((range, *cells))
    })?;
    Some((
        cluster_index_at_or_after(paragraph, range.start)
            ..cluster_index_at_or_after(paragraph, range.end),
        cells,
    ))
}

fn is_internal_jidori_boundary(paragraph: &Paragraph, ordinal: usize) -> bool {
    let Some(cluster) = paragraph.text.clusters().get(ordinal) else {
        return false;
    };
    let boundary = cluster.range().end;
    paragraph.constructs.iter().any(|construct| {
        matches!(construct.kind(), ConstructKind::Jidori { range, .. }
            if range.start < boundary && boundary < range.end)
    })
}

/// Whether the boundary after `ordinal` falls inside a warichu or a furawake.
///
/// Both structures set their text on sublines that run *beside* the line, so a boundary
/// with the clusters on either side of it inside the same structure is no part of what the
/// line was composed from: the space there is the block's own, the seam where two sublines
/// meet carries nothing on the line at all, and §3.8.3's ladder adjusts the spacing of the
/// line (`docs/decisions/stacked-structure-geometry.md`). A boundary with one cluster
/// inside the structure and the next outside it is the line's — that is where the block
/// ends and the line resumes — and this is false there.
fn is_internal_stacked_boundary(paragraph: &Paragraph, ordinal: usize) -> bool {
    let Some(cluster) = paragraph.text.clusters().get(ordinal) else {
        return false;
    };
    let boundary = cluster.range().end;
    paragraph.constructs.iter().any(|construct| {
        matches!(
            construct.kind(),
            ConstructKind::Warichu(range) | ConstructKind::Furawake { range, .. }
                if range.start < boundary && boundary < range.end
        )
    })
}

fn formula_cluster_range(paragraph: &Paragraph, ordinal: usize) -> Option<Range<usize>> {
    let cluster = paragraph.text.clusters().get(ordinal)?.range();
    let range = paragraph.constructs.iter().find_map(|construct| {
        let ConstructKind::Formula(range) = construct.kind() else {
            return None;
        };
        (range.start <= cluster.start && cluster.end <= range.end).then_some(range)
    })?;
    Some(
        cluster_index_at_or_after(paragraph, range.start)
            ..cluster_index_at_or_after(paragraph, range.end),
    )
}

fn is_internal_furawake_offset(paragraph: &Paragraph, offset: usize) -> bool {
    paragraph.constructs.iter().any(|construct| {
        let ConstructKind::Furawake { range, .. } = construct.kind() else {
            return false;
        };
        range.start < offset && offset < range.end
    })
}

fn boundary_expansion_site(paragraph: &Paragraph, style: &Style, before: usize) -> ExpansionSite {
    let Some(cluster) = paragraph.text.clusters().get(before) else {
        return ExpansionSite::None;
    };
    let after = before.saturating_add(1);
    if after >= paragraph.text.clusters().len() {
        return ExpansionSite::None;
    }
    if is_western_word_space(paragraph, before) {
        let size = cluster
            .size_override()
            .unwrap_or(paragraph.text.size())
            .inline();
        let cap = half_rounded_up(size)
            .saturating_sub(effective_cluster_body_advance(paragraph, before))
            .max(0);
        let after_class = class_of_cluster_with_style(paragraph, style, after);
        let residual = crate::generated::table6::CELLS
            .iter()
            .any(|cell| cell.before == 26 && cell.after == after_class && cell.residual);
        return ExpansionSite::Site {
            weight: size,
            bounded: (cap > 0).then_some((cap, 1)),
            residual,
        };
    }
    let before_complex = expansion_complex_at(paragraph, before);
    let after_complex = expansion_complex_at(paragraph, after);
    if let (Some(before_complex), Some(after_complex)) = (before_complex, after_complex) {
        if before_complex == after_complex {
            return ExpansionSite::None;
        }
    }

    let boundary = cluster.range().end;
    if paragraph
        .constructs
        .iter()
        .any(|construct| match construct.kind() {
            ConstructKind::TateChuYoko(range)
            | ConstructKind::Warichu(range)
            | ConstructKind::Formula(range)
            | ConstructKind::Furawake { range, .. }
            | ConstructKind::Jidori { range, .. }
            | ConstructKind::Script { range, .. } => range.start < boundary && boundary < range.end,
            ConstructKind::Ruby(ruby) => {
                ruby.kind() != RubyKind::Mono
                    && ruby.base().start < boundary
                    && boundary < ruby.base().end
            },
            _ => false,
        })
    {
        return ExpansionSite::None;
    }

    let before_class = class_of_cluster_with_style(paragraph, style, before);
    let after_class = class_of_cluster_with_style(paragraph, style, after);
    let before_character = single_cluster_character(paragraph, cluster);
    let after_character = single_cluster_character(paragraph, &paragraph.text.clusters()[after]);
    if before_class == 8 && after_class == 8 && cl_08_same_kind(before_character, after_character) {
        return ExpansionSite::None;
    }
    if before_class == 27 && after_class == 13 {
        let role = cluster.role();
        if role == Some(ClusterRole::QuantitySymbol)
            || before_character.is_some_and(|character| character.is_ascii_digit())
        {
            return ExpansionSite::None;
        }
    }
    let Some(cell) = crate::generated::table6::CELLS
        .iter()
        .find(|cell| cell.before == before_class && cell.after == after_class)
    else {
        return ExpansionSite::None;
    };
    let before_size = cluster.size_override().unwrap_or(paragraph.text.size());
    let after_cluster = &paragraph.text.clusters()[after];
    let after_size = after_cluster
        .size_override()
        .unwrap_or(paragraph.text.size());
    let before_solid = before_character
        .is_some_and(|character| contextual_punctuation_is_solid(paragraph, cluster, character));
    let after_solid = after_character.is_some_and(|character| {
        contextual_punctuation_is_solid(paragraph, after_cluster, character)
    });
    let components = crate::spec::table_one_space_components(
        before_class,
        after_class,
        before_size,
        after_size,
        before_solid,
        after_solid,
    );
    let weight = match components {
        [amount, 0] if amount > 0 => before_size.inline(),
        [0, amount] if amount > 0 => after_size.inline(),
        _ => before_size.inline(),
    };
    if cell.residual {
        return ExpansionSite::Site {
            weight,
            bounded: None,
            residual: true,
        };
    }
    let Some(limit) = cell.limit else {
        return ExpansionSite::None;
    };
    let current = boundary_space_after_with_style(paragraph, style, before);
    let ceiling = expansion_ceiling(
        style,
        before_class,
        after_class,
        weight,
        crate::spec::scale_spec_units(weight, limit),
    );
    let cap = ceiling.saturating_sub(current).max(0);
    if cap == 0 || cell.stage == 0 {
        ExpansionSite::None
    } else {
        ExpansionSite::Site {
            weight,
            bounded: Some((cap, cell.stage)),
            residual: false,
        }
    }
}

fn expansion_ceiling(style: &Style, before: u8, after: u8, weight: i32, table: i32) -> i32 {
    if !matches!((before, after), (19, 27) | (27, 19)) {
        return table;
    }
    match style.japanese_latin_expansion_ceiling() {
        JapaneseLatinExpansionCeiling::HalfEm => half_rounded_up(weight),
        JapaneseLatinExpansionCeiling::ThirdEm => {
            (weight / 3).saturating_add(i32::from(weight % 3 != 0))
        },
        JapaneseLatinExpansionCeiling::Rigid => quarter_rounded_up(weight),
    }
}

fn boundary_expansion_site_on_line(
    paragraph: &Paragraph,
    style: &Style,
    before: usize,
    line_start: usize,
    line_end: usize,
) -> ExpansionSite {
    if (before == line_start && is_western_word_space(paragraph, before))
        || (before.saturating_add(2) == line_end
            && is_western_word_space(paragraph, before.saturating_add(1)))
    {
        ExpansionSite::None
    } else {
        boundary_expansion_site(paragraph, style, before)
    }
}

fn expansion_complex_at(paragraph: &Paragraph, ordinal: usize) -> Option<ComplexIdentity> {
    let cluster = paragraph.text.clusters().get(ordinal)?.range();
    paragraph
        .constructs
        .iter()
        .enumerate()
        .find_map(|(construct, candidate)| match candidate.kind() {
            ConstructKind::Script { range, .. }
                if range.start <= cluster.start && cluster.end <= range.end =>
            {
                Some(ComplexIdentity {
                    kind: ComplexKind::Ornamented,
                    construct,
                    member: 0,
                })
            },
            ConstructKind::Ruby(ruby)
                if ruby.base().start <= cluster.start && cluster.end <= ruby.base().end =>
            {
                let (kind, member) = match ruby.kind() {
                    RubyKind::Mono => (
                        ComplexKind::SimpleRuby,
                        ruby.runs()
                            .iter()
                            .position(|run| {
                                let range = run.base();
                                range.start <= cluster.start && cluster.end <= range.end
                            })
                            .unwrap_or(0),
                    ),
                    RubyKind::Group => (ComplexKind::SimpleRuby, 0),
                    RubyKind::Jukugo => (ComplexKind::JukugoRuby, 0),
                };
                Some(ComplexIdentity {
                    kind,
                    construct,
                    member,
                })
            },
            ConstructKind::TateChuYoko(range)
                if paragraph.writing_mode == WritingMode::VerticalRl
                    && range.start <= cluster.start
                    && cluster.end <= range.end =>
            {
                Some(ComplexIdentity {
                    kind: ComplexKind::TateChuYoko,
                    construct,
                    member: 0,
                })
            },
            _ => None,
        })
}

fn prepare_line_adjustments(
    paragraph: &Paragraph,
    style: &Style,
    line_start: usize,
    line_end: usize,
    need: i64,
    adjustments: &mut Vec<i32>,
) {
    adjustments.clear();
    adjustments.resize(line_end.saturating_sub(line_start), 0);
    if adjustments.is_empty() || need == 0 {
        return;
    }
    if need < 0 {
        prepare_line_reductions(
            paragraph,
            style,
            line_start,
            line_end,
            need.saturating_abs(),
            adjustments,
        );
        return;
    }

    let sites: Vec<_> = (line_start..line_end.saturating_sub(1))
        .map(|before| {
            boundary_expansion_site_on_line(paragraph, style, before, line_start, line_end)
        })
        .collect();
    let mut remaining = need;
    for stage in 1_u8..=3 {
        if remaining == 0 {
            return;
        }
        let stage_sites: Vec<_> = sites
            .iter()
            .enumerate()
            .filter_map(|(index, site)| match site {
                ExpansionSite::Site {
                    weight,
                    bounded: Some((cap, site_stage)),
                    ..
                } if *site_stage == stage => Some((index, *weight, Some(*cap))),
                _ => None,
            })
            .collect();
        let capacity = stage_sites.iter().fold(0_i64, |sum, (_, _, cap)| {
            sum.saturating_add(i64::from(cap.unwrap_or(0)))
        });
        let take = remaining.min(capacity);
        distribute_adjustment(take, &stage_sites, style.remainder(), adjustments);
        remaining = remaining.saturating_sub(take);
    }

    if remaining == 0 {
        return;
    }
    let union: Vec<_> = sites
        .iter()
        .enumerate()
        .filter_map(|(index, site)| match site {
            ExpansionSite::Site {
                weight,
                bounded,
                residual,
            } if *residual || bounded.is_some_and(|(_, stage)| (2..=3).contains(&stage)) => {
                Some((index, *weight, None))
            },
            ExpansionSite::None | ExpansionSite::Site { .. } => None,
        })
        .collect();
    distribute_adjustment(remaining, &union, style.remainder(), adjustments);
}

fn prepare_line_reductions(
    paragraph: &Paragraph,
    style: &Style,
    line_start: usize,
    line_end: usize,
    mut need: i64,
    adjustments: &mut [i32],
) {
    let sites = reduction_sites(paragraph, style, line_start, line_end);
    for stage in 1_u8..=6 {
        if need <= 0 {
            break;
        }
        let mut discrete: Vec<_> = sites
            .iter()
            .copied()
            .filter(|site| site.stage == stage && site.discrete)
            .collect();
        if style.remainder() == Remainder::Trailing {
            discrete.reverse();
        }
        for site in discrete {
            if need <= 0 {
                break;
            }
            apply_reduction(site.boundary, i64::from(site.capacity), adjustments);
            need = need.saturating_sub(i64::from(site.capacity));
        }

        if need <= 0 {
            break;
        }
        let continuous: Vec<_> = sites
            .iter()
            .copied()
            .filter(|site| site.stage == stage && !site.discrete)
            .collect();
        let capacity = continuous.iter().fold(0_i64, |sum, site| {
            sum.saturating_add(i64::from(site.capacity))
        });
        let take = need.min(capacity);
        distribute_reduction(take, &continuous, style.remainder(), adjustments);
        need = need.saturating_sub(take);
    }
}

fn reduction_sites(
    paragraph: &Paragraph,
    style: &Style,
    line_start: usize,
    line_end: usize,
) -> Vec<ReductionSite> {
    let mut sites = Vec::new();
    for ordinal in line_start..line_end.saturating_sub(1) {
        if is_internal_jidori_boundary(paragraph, ordinal)
            || is_internal_stacked_boundary(paragraph, ordinal)
        {
            continue;
        }
        if is_western_word_space(paragraph, ordinal) {
            let cluster = &paragraph.text.clusters()[ordinal];
            let minimum = quarter_inline_size(paragraph, cluster);
            let capacity = effective_cluster_body_advance(paragraph, ordinal)
                .saturating_sub(minimum)
                .max(0);
            push_reduction_site(
                &mut sites,
                ordinal.saturating_sub(line_start),
                paragraph.text.clusters()[ordinal]
                    .size_override()
                    .unwrap_or(paragraph.text.size())
                    .inline(),
                capacity,
                1,
                false,
            );
        }
        append_table_reduction_sites(
            paragraph,
            style,
            ordinal,
            ordinal.saturating_sub(line_start),
            &mut sites,
        );
    }
    if let Some(ordinal) = line_end.checked_sub(1).filter(|_| line_start < line_end) {
        append_line_end_reduction_site(
            paragraph,
            style,
            ordinal,
            ordinal.saturating_sub(line_start),
            &mut sites,
        );
    }
    sites
}

fn width_after_available_reduction(
    paragraph: &Paragraph,
    style: &Style,
    start: usize,
    end: usize,
    width: i64,
    available: i64,
) -> i64 {
    let need = width.saturating_sub(available);
    if need <= 0 {
        return width;
    }
    let line_start = cluster_index_at_or_after(paragraph, start);
    let line_end = cluster_index_at_or_after(paragraph, end);
    let capacity = reduction_sites(paragraph, style, line_start, line_end)
        .iter()
        .fold(0_i64, |sum, site| {
            sum.saturating_add(i64::from(site.capacity))
        });
    let reduced = width.saturating_sub(need.min(capacity));
    reduced.saturating_sub(hanging_amount(
        paragraph, style, line_end, reduced, available,
    ))
}

fn append_line_end_reduction_site(
    paragraph: &Paragraph,
    style: &Style,
    ordinal: usize,
    boundary: usize,
    sites: &mut Vec<ReductionSite>,
) {
    let Some(cluster) = paragraph.text.clusters().get(ordinal) else {
        return;
    };
    let before = class_of_cluster_with_style(paragraph, style, ordinal);
    let cells = match style.reduction_table() {
        ReductionTable::Table3 => crate::generated::table3::CELLS,
        ReductionTable::Table4 => crate::generated::table4::CELLS,
        ReductionTable::Table5 => crate::generated::table5::CELLS,
    };
    let Some(cell) = cells
        .iter()
        .find(|cell| cell.before == before && cell.after == 0)
    else {
        return;
    };
    let Some(limit) = cell.limit.filter(|_| cell.stage != 0) else {
        return;
    };
    let size = cluster.size_override().unwrap_or(paragraph.text.size());
    let current = line_end_space_after(paragraph, style, ordinal);
    let floor = crate::spec::scale_spec_units(size.inline(), limit);
    push_reduction_site(
        sites,
        boundary,
        size.inline(),
        current.saturating_sub(floor),
        cell.stage,
        cell.two_valued,
    );
}

fn hanging_amount(
    paragraph: &Paragraph,
    style: &Style,
    line_end: usize,
    occupied: i64,
    available: i64,
) -> i64 {
    if style.hanging_punctuation() != HangingPunctuation::Hanging || occupied <= available {
        return 0;
    }
    let Some(ordinal) = line_end.checked_sub(1) else {
        return 0;
    };
    if !matches!(
        class_of_cluster_with_style(paragraph, style, ordinal),
        crate::spec::FULL_STOP | crate::spec::COMMA
    ) {
        return 0;
    }
    occupied
        .saturating_sub(available)
        .min(i64::from(effective_cluster_body_advance(paragraph, ordinal)).max(0))
}

fn append_table_reduction_sites(
    paragraph: &Paragraph,
    style: &Style,
    ordinal: usize,
    boundary: usize,
    sites: &mut Vec<ReductionSite>,
) {
    let clusters = paragraph.text.clusters();
    let Some(before_cluster) = clusters.get(ordinal) else {
        return;
    };
    let Some(after_cluster) = clusters.get(ordinal.saturating_add(1)) else {
        return;
    };
    let before = class_of_cluster_with_style(paragraph, style, ordinal);
    let after = class_of_cluster_with_style(paragraph, style, ordinal.saturating_add(1));
    let before_size = before_cluster
        .size_override()
        .unwrap_or(paragraph.text.size());
    let after_size = after_cluster
        .size_override()
        .unwrap_or(paragraph.text.size());
    let before_character = single_cluster_character(paragraph, before_cluster);
    let after_character = single_cluster_character(paragraph, after_cluster);
    let before_solid = before_character.is_some_and(|character| {
        contextual_punctuation_is_solid(paragraph, before_cluster, character)
    });
    let after_solid = after_character.is_some_and(|character| {
        contextual_punctuation_is_solid(paragraph, after_cluster, character)
    });
    let components = crate::spec::table_one_space_components(
        before,
        after,
        before_size,
        after_size,
        before_solid,
        after_solid,
    );

    if append_special_reduction_sites(
        style.reduction_table(),
        before,
        after,
        components,
        [before_size.inline(), after_size.inline()],
        boundary,
        sites,
    ) {
        return;
    }

    let cells = match style.reduction_table() {
        ReductionTable::Table3 => crate::generated::table3::CELLS,
        ReductionTable::Table4 => crate::generated::table4::CELLS,
        ReductionTable::Table5 => crate::generated::table5::CELLS,
    };
    let Some(cell) = cells
        .iter()
        .find(|cell| cell.before == before && cell.after == after)
    else {
        return;
    };
    let active = match components {
        [amount, 0] if amount > 0 => Some((amount, before_size.inline())),
        [0, amount] if amount > 0 => Some((amount, after_size.inline())),
        _ => None,
    };
    let (amount, weight) = match (active, cell.limit) {
        (Some(active), Some(_)) if cell.stage != 0 => active,
        _ => return,
    };
    let floor = crate::spec::scale_spec_units(weight, cell.limit.unwrap_or(0));
    push_reduction_site(
        sites,
        boundary,
        weight,
        amount.saturating_sub(floor),
        cell.stage,
        cell.two_valued,
    );
}

fn append_special_reduction_sites(
    table: ReductionTable,
    before: u8,
    after: u8,
    components: [i32; 2],
    weights: [i32; 2],
    boundary: usize,
    sites: &mut Vec<ReductionSite>,
) -> bool {
    let mut push = |component: usize, floor: i32, stage: u8| {
        push_reduction_site(
            sites,
            boundary,
            weights[component],
            components[component].saturating_sub(floor),
            stage,
            false,
        );
    };
    match (before, after, table) {
        (5, 5, ReductionTable::Table3) => {
            push(0, 0, 4);
            push(1, 0, 4);
        },
        (5, 5, ReductionTable::Table4) => {
            push(0, 0, 2);
            push(1, 0, 2);
        },
        (5 | 6, 5, ReductionTable::Table5) => {},
        (6, 5, ReductionTable::Table3) => push(1, 0, 4),
        (6 | 7, 5, ReductionTable::Table4) => push(1, 0, 2),
        (7, 5, ReductionTable::Table3) => {
            push(0, 0, 5);
            push(1, 0, 4);
        },
        (7, 5, ReductionTable::Table5) => {
            push(0, quarter_rounded_up(weights[0]), 3);
        },
        _ => return false,
    }
    true
}

fn quarter_rounded_up(value: i32) -> i32 {
    (value / 4).saturating_add(i32::from(value % 4 != 0))
}

fn push_reduction_site(
    sites: &mut Vec<ReductionSite>,
    boundary: usize,
    weight: i32,
    capacity: i32,
    stage: u8,
    discrete: bool,
) {
    if capacity > 0 && stage != 0 {
        sites.push(ReductionSite {
            boundary,
            weight,
            capacity,
            stage,
            discrete,
        });
    }
}

fn distribute_reduction(
    amount: i64,
    sites: &[ReductionSite],
    remainder: Remainder,
    adjustments: &mut [i32],
) {
    if amount <= 0 || sites.is_empty() {
        return;
    }
    let weight_sum = sites.iter().fold(0_i64, |sum, site| {
        sum.saturating_add(i64::from(site.weight.max(1)))
    });
    let mut assigned: Vec<i64> = sites
        .iter()
        .map(|site| {
            amount
                .saturating_mul(i64::from(site.weight.max(1)))
                .checked_div(weight_sum.max(1))
                .unwrap_or(0)
                .min(i64::from(site.capacity))
        })
        .collect();
    let mut left = amount.saturating_sub(
        assigned
            .iter()
            .fold(0_i64, |sum, take| sum.saturating_add(*take)),
    );
    while left > 0 {
        let mut progressed = false;
        match remainder {
            Remainder::Leading => {
                for (site, take) in sites.iter().zip(&mut assigned) {
                    if left == 0 {
                        break;
                    }
                    if *take < i64::from(site.capacity) {
                        *take = take.saturating_add(1);
                        left = left.saturating_sub(1);
                        progressed = true;
                    }
                }
            },
            Remainder::Trailing => {
                for (site, take) in sites.iter().zip(&mut assigned).rev() {
                    if left == 0 {
                        break;
                    }
                    if *take < i64::from(site.capacity) {
                        *take = take.saturating_add(1);
                        left = left.saturating_sub(1);
                        progressed = true;
                    }
                }
            },
        }
        if !progressed {
            break;
        }
    }
    for (site, take) in sites.iter().zip(assigned) {
        apply_reduction(site.boundary, take, adjustments);
    }
}

fn apply_reduction(boundary: usize, amount: i64, adjustments: &mut [i32]) {
    if let Some(adjustment) = adjustments.get_mut(boundary) {
        *adjustment = adjustment.saturating_sub(clamp_i32(amount));
    }
}

fn distribute_adjustment(
    amount: i64,
    sites: &[(usize, i32, Option<i32>)],
    remainder: Remainder,
    adjustments: &mut [i32],
) {
    if amount <= 0 || sites.is_empty() {
        return;
    }
    let weight_sum = sites.iter().fold(0_i64, |sum, (_, weight, _)| {
        sum.saturating_add(i64::from((*weight).max(1)))
    });
    let mut placed = 0_i64;
    for &(index, weight, cap) in sites {
        let proportional = amount
            .saturating_mul(i64::from(weight.max(1)))
            .checked_div(weight_sum.max(1))
            .unwrap_or(0);
        let share = cap.map_or(proportional, |cap| proportional.min(i64::from(cap)));
        if let Some(adjustment) = adjustments.get_mut(index) {
            *adjustment = adjustment.saturating_add(clamp_i32(share));
            placed = placed.saturating_add(share);
        }
    }

    let mut left = amount.saturating_sub(placed);
    while left > 0 {
        let mut progressed = false;
        match remainder {
            Remainder::Leading => {
                for &(index, _, cap) in sites {
                    if left == 0 {
                        break;
                    }
                    let Some(adjustment) = adjustments.get_mut(index) else {
                        continue;
                    };
                    if cap.is_none_or(|cap| *adjustment < cap) {
                        *adjustment = adjustment.saturating_add(1);
                        left = left.saturating_sub(1);
                        progressed = true;
                    }
                }
            },
            Remainder::Trailing => {
                for &(index, _, cap) in sites.iter().rev() {
                    if left == 0 {
                        break;
                    }
                    let Some(adjustment) = adjustments.get_mut(index) else {
                        continue;
                    };
                    if cap.is_none_or(|cap| *adjustment < cap) {
                        *adjustment = adjustment.saturating_add(1);
                        left = left.saturating_sub(1);
                        progressed = true;
                    }
                }
            },
        }
        if !progressed {
            break;
        }
    }
}

fn effective_cluster_advance(paragraph: &Paragraph, style: &Style, ordinal: usize) -> i32 {
    effective_cluster_body_advance(paragraph, ordinal)
        .saturating_add(boundary_space_after_with_style(paragraph, style, ordinal))
}

fn effective_cluster_body_advance(paragraph: &Paragraph, ordinal: usize) -> i32 {
    if let Some((group, columns, line_gap)) = furawake_cluster_range(paragraph, ordinal) {
        if group.start != ordinal {
            return 0;
        }
        furawake_segment(
            paragraph,
            group,
            columns,
            line_gap,
            paragraph.text.clusters().len(),
        )
        .advance
    } else {
        nested_cluster_body_advance(paragraph, ordinal)
    }
}

fn nested_cluster_body_advance(paragraph: &Paragraph, ordinal: usize) -> i32 {
    if let Some(group) = tate_chu_yoko_cluster_range(paragraph, ordinal) {
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
    } else if formula_cluster_range(paragraph, ordinal).is_some()
        && single_cluster_character(paragraph, &paragraph.text.clusters()[ordinal])
            .is_some_and(is_math_token)
    {
        paragraph.text.clusters()[ordinal]
            .size_override()
            .unwrap_or(paragraph.text.size())
            .inline()
    } else {
        paragraph.text.clusters()[ordinal].advance()
    }
}

fn effective_cluster_advance_on_line(
    paragraph: &Paragraph,
    style: &Style,
    ordinal: usize,
    line_start: usize,
    line_end: usize,
    line_index: usize,
) -> i32 {
    ordinary_cluster_advance_on_line(paragraph, style, ordinal, line_start, line_end, line_index)
        .saturating_add(jidori_extra_after(
            paragraph, style, ordinal, line_start, line_end, line_index,
        ))
}

fn ordinary_cluster_advance_on_line(
    paragraph: &Paragraph,
    style: &Style,
    ordinal: usize,
    line_start: usize,
    line_end: usize,
    line_index: usize,
) -> i32 {
    effective_cluster_advance_on_line_without_ruby(paragraph, style, ordinal, line_start, line_end)
        .saturating_add(ruby_boundary_separation_after(
            paragraph, style, ordinal, line_start, line_end, line_index,
        ))
}

fn jidori_extra_after(
    paragraph: &Paragraph,
    style: &Style,
    ordinal: usize,
    line_start: usize,
    line_end: usize,
    line_index: usize,
) -> i32 {
    let Some((range, cells)) = jidori_cluster_range(paragraph, ordinal) else {
        return 0;
    };
    if range.start < line_start || range.end > line_end || range.start >= range.end {
        return 0;
    }
    jidori_plan(
        paragraph, style, range, cells, line_start, line_end, line_index,
    )
    .extra_after(ordinal)
}

fn jidori_plan(
    paragraph: &Paragraph,
    style: &Style,
    range: Range<usize>,
    cells: u16,
    line_start: usize,
    line_end: usize,
    line_index: usize,
) -> JidoriPlan {
    let natural = range.clone().fold(0_i64, |sum, member| {
        let advance = if member.saturating_add(1) == range.end {
            effective_cluster_body_advance(paragraph, member)
        } else {
            ordinary_cluster_advance_on_line(
                paragraph, style, member, line_start, line_end, line_index,
            )
        };
        sum.saturating_add(i64::from(advance))
    });
    let target = i64::from(paragraph.text.size().inline()).saturating_mul(i64::from(cells));
    let surplus = target.saturating_sub(natural).max(0);
    let mut extra_after = vec![0; range.end.saturating_sub(range.start)];
    let mut eligible = Vec::new();
    for before in range.start..range.end.saturating_sub(1) {
        if paragraph
            .text
            .clusters()
            .get(before)
            .is_some_and(|cluster| break_is_legal(paragraph, style, cluster.range().end))
        {
            eligible.push(before);
        }
    }
    if eligible.is_empty() {
        if let Some(last) = extra_after.last_mut() {
            *last = clamp_i32(surplus);
        }
    } else {
        let divisor = i64::try_from(eligible.len()).unwrap_or(i64::MAX).max(1);
        let quotient = surplus.checked_div(divisor).unwrap_or(0);
        let remainder = usize::try_from(surplus.rem_euclid(divisor)).unwrap_or(usize::MAX);
        for (position, boundary) in eligible.iter().enumerate() {
            let receives_remainder = match style.remainder() {
                Remainder::Leading => position < remainder,
                Remainder::Trailing => {
                    eligible.len().saturating_sub(position.saturating_add(1)) < remainder
                },
            };
            if let Some(extra) = extra_after.get_mut(boundary.saturating_sub(range.start)) {
                *extra = clamp_i32(quotient.saturating_add(i64::from(receives_remainder)));
            }
        }
    }
    JidoriPlan { range, extra_after }
}

fn effective_cluster_advance_on_line_without_ruby(
    paragraph: &Paragraph,
    style: &Style,
    ordinal: usize,
    line_start: usize,
    line_end: usize,
) -> i32 {
    if let Some((group, columns, line_gap)) = furawake_cluster_range(paragraph, ordinal) {
        if ordinal != group.start {
            return 0;
        }
        return furawake_segment(paragraph, group, columns, line_gap, line_end).advance;
    }
    if let Some(group) = warichu_cluster_range(paragraph, ordinal) {
        let segment_start = group.start.max(line_start);
        if ordinal != segment_start {
            return 0;
        }
        return warichu_segment(paragraph, group, line_start, line_end).advance;
    }
    if is_western_word_space(paragraph, ordinal)
        && (ordinal == line_start || ordinal.saturating_add(1) == line_end)
    {
        return 0;
    }
    if ordinal.saturating_add(1) == line_end {
        return effective_cluster_body_advance(paragraph, ordinal)
            .saturating_add(line_end_space_after(paragraph, style, ordinal));
    }
    effective_cluster_advance(paragraph, style, ordinal)
}

fn line_end_space_after(paragraph: &Paragraph, style: &Style, ordinal: usize) -> i32 {
    let Some(cluster) = paragraph.text.clusters().get(ordinal) else {
        return 0;
    };
    let class = class_of_cluster_with_style(paragraph, style, ordinal);
    if (class == crate::spec::CLOSING_BRACKET
        && style.line_end_punctuation() == LineEndPunctuation::Solid)
        || (class == crate::spec::COMMA
            && style.line_end_full_stop_comma() == LineEndFullStopComma::Jis)
    {
        return 0;
    }
    let size = cluster.size_override().unwrap_or(paragraph.text.size());
    let character = single_cluster_character(paragraph, cluster);
    let solid = character
        .is_some_and(|character| contextual_punctuation_is_solid(paragraph, cluster, character));
    crate::spec::table_one_space(class, 0, size, size, solid, false)
}

#[derive(Debug, Clone)]
struct RubyOverhang {
    base: Range<usize>,
    leading: i32,
    trailing: i32,
    ruby_em: i32,
}

#[derive(Debug, Clone)]
struct GroupRubyBasePlan {
    base: Range<usize>,
    leading: i32,
    trailing: i32,
    gaps_after: Vec<(usize, i32)>,
}

impl GroupRubyBasePlan {
    fn gap_after(&self, ordinal: usize) -> i32 {
        self.gaps_after
            .iter()
            .find_map(|(boundary, gap)| (*boundary == ordinal).then_some(*gap))
            .unwrap_or(0)
    }
}

#[derive(Debug, Clone)]
struct PhoneticJukugoRun {
    base: Range<usize>,
    annotation: Range<usize>,
    base_start: i64,
    base_end: i64,
    annotation_start: i64,
    annotation_width: i64,
    ruby_em: i32,
    annotation_count: usize,
}

#[derive(Debug, Clone)]
struct PhoneticJukugoPlan {
    runs: Vec<PhoneticJukugoRun>,
    leading_gap: i32,
    gaps_after: Vec<(usize, i32)>,
}

impl PhoneticJukugoPlan {
    fn gap_after(&self, ordinal: usize) -> i32 {
        self.gaps_after
            .iter()
            .find_map(|(boundary, gap)| (*boundary == ordinal).then_some(*gap))
            .unwrap_or(0)
    }
}

#[derive(Debug, Clone, Copy)]
struct PhoneticEdges {
    line: LineContext,
    leading_allowance: i32,
    trailing_allowance: i32,
}

#[derive(Debug, Clone, Copy)]
enum RubySide {
    Leading,
    Trailing,
}

fn visit_ruby_spans(
    paragraph: &Paragraph,
    style: &Style,
    mut visit: impl FnMut(&Ruby, Range<usize>, Range<usize>),
) {
    for construct in &paragraph.constructs {
        let ConstructKind::Ruby(ruby) = construct.kind() else {
            continue;
        };
        match ruby.kind() {
            RubyKind::Group => visit(ruby, ruby.base(), 0..ruby.annotation().source().len()),
            RubyKind::Mono => {
                for run in ruby.runs() {
                    visit(ruby, run.base(), run.annotation());
                }
            },
            RubyKind::Jukugo => {
                if style.jukugo_ruby_layout() == JukugoRubyLayout::Phonetic {
                    continue;
                }
                let per_base = ruby
                    .runs()
                    .iter()
                    .all(|run| annotation_cluster_count(ruby.annotation(), &run.annotation()) <= 2);
                if per_base {
                    for run in ruby.runs() {
                        visit(ruby, run.base(), run.annotation());
                    }
                } else {
                    visit(ruby, ruby.base(), 0..ruby.annotation().source().len());
                }
            },
        }
    }
}

fn phonetic_jukugo_plan(
    paragraph: &Paragraph,
    style: &Style,
    ruby: &Ruby,
    line_start: usize,
    line_end: usize,
    line_index: usize,
) -> Option<PhoneticJukugoPlan> {
    if ruby.kind() != RubyKind::Jukugo || style.jukugo_ruby_layout() != JukugoRubyLayout::Phonetic {
        return None;
    }

    let mut runs = Vec::new();
    for run in ruby.runs() {
        let base = cluster_index_at_or_after(paragraph, run.base().start)
            ..cluster_index_at_or_after(paragraph, run.base().end);
        if base.start < line_start || base.end > line_end || base.start >= base.end {
            continue;
        }
        let (annotation_count, annotation_width, ruby_em) =
            ruby_annotation_metrics(ruby.annotation(), &run.annotation());
        let base_width = ruby_base_width(paragraph, style, &base);
        runs.push(PhoneticJukugoRun {
            base,
            annotation: run.annotation(),
            base_start: 0,
            base_end: base_width,
            annotation_start: 0,
            annotation_width,
            ruby_em,
            annotation_count,
        });
    }
    let (first, last) = (runs.first()?, runs.last()?);
    let leading_allowance = if first.base.start == line_start {
        if line_index == 0 && style.ruby_overhang_indent() == RubyOverhangIndent::Permitted {
            line_head_indent(paragraph, style, line_start, line_index).min(first.ruby_em)
        } else {
            0
        }
    } else {
        ruby_neighbor_overhang_allowance(
            paragraph,
            style,
            first.base.start.saturating_sub(1),
            RubySide::Leading,
            first.ruby_em,
        )
    };
    let trailing_allowance = if last.base.end == line_end {
        0
    } else {
        ruby_neighbor_overhang_allowance(
            paragraph,
            style,
            last.base.end,
            RubySide::Trailing,
            last.ruby_em,
        )
    };
    let edges = PhoneticEdges {
        line: LineContext {
            start: line_start,
            end: line_end,
            index: line_index,
        },
        leading_allowance,
        trailing_allowance,
    };
    let maximum_expansion =
        runs.iter()
            .filter(|run| run.annotation_count > 2)
            .fold(0_i64, |sum, run| {
                let base_width = run.base_end.saturating_sub(run.base_start);
                sum.saturating_add(run.annotation_width.saturating_sub(base_width).max(0))
            });

    build_phonetic_jukugo_plan(paragraph, style, &runs, 0, edges).or_else(|| {
        let mut lower = 1_i64;
        let mut upper = maximum_expansion;
        let upper_plan = build_phonetic_jukugo_plan(paragraph, style, &runs, upper, edges)?;
        while lower < upper {
            let middle = lower.saturating_add(upper.saturating_sub(lower) / 2);
            if build_phonetic_jukugo_plan(paragraph, style, &runs, middle, edges).is_some() {
                upper = middle;
            } else {
                lower = middle.saturating_add(1);
            }
        }
        if lower == maximum_expansion {
            Some(upper_plan)
        } else {
            build_phonetic_jukugo_plan(paragraph, style, &runs, lower, edges)
        }
    })
}

fn build_phonetic_jukugo_plan(
    paragraph: &Paragraph,
    style: &Style,
    raw_runs: &[PhoneticJukugoRun],
    expansion: i64,
    edges: PhoneticEdges,
) -> Option<PhoneticJukugoPlan> {
    let assigned = apportion_phonetic_expansion(raw_runs, expansion, style.remainder());
    let mut before = alloc::vec![0_i64; raw_runs.len()];
    let mut after = alloc::vec![0_i64; raw_runs.len()];
    for (index, (run, amount)) in raw_runs.iter().zip(assigned).enumerate() {
        if run.annotation_count <= 2 || amount == 0 {
            continue;
        }
        if run.base.start == edges.line.start && run.base.end != edges.line.end {
            after[index] = amount;
        } else if run.base.end == edges.line.end && run.base.start != edges.line.start {
            before[index] = amount;
        } else {
            let half = amount / 2;
            let odd = amount % 2;
            match style.remainder() {
                Remainder::Leading => {
                    before[index] = half.saturating_add(odd);
                    after[index] = half;
                },
                Remainder::Trailing => {
                    before[index] = half;
                    after[index] = half.saturating_add(odd);
                },
            }
        }
    }

    let mut leading_gap = 0_i64;
    let mut gaps_after = Vec::<(usize, i64)>::new();
    for (index, run) in raw_runs.iter().enumerate() {
        if before[index] != 0 {
            if run.base.start == edges.line.start {
                leading_gap = leading_gap.saturating_add(before[index]);
            } else {
                add_phonetic_gap(
                    &mut gaps_after,
                    run.base.start.saturating_sub(1),
                    before[index],
                );
            }
        }
        if after[index] != 0 {
            add_phonetic_gap(
                &mut gaps_after,
                run.base.end.saturating_sub(1),
                after[index],
            );
        }
    }

    let first = raw_runs.first()?;
    let mut cursor = if first.base.start == edges.line.start {
        leading_gap
    } else {
        phonetic_gap_after(&gaps_after, first.base.start.saturating_sub(1))
    };
    let mut runs = Vec::with_capacity(raw_runs.len());
    for (index, raw) in raw_runs.iter().enumerate() {
        let base_start = cursor;
        let mut base_end = base_start;
        for ordinal in raw.base.clone() {
            cursor = cursor.saturating_add(i64::from(effective_cluster_body_advance(
                paragraph, ordinal,
            )));
            base_end = cursor;
            if ordinal.saturating_add(1) < raw.base.end {
                cursor = cursor
                    .saturating_add(i64::from(boundary_space_after(paragraph, ordinal)))
                    .saturating_add(phonetic_gap_after(&gaps_after, ordinal));
            }
        }
        if index.saturating_add(1) < raw_runs.len() {
            let boundary = raw.base.end.saturating_sub(1);
            cursor = cursor
                .saturating_add(i64::from(boundary_space_after(paragraph, boundary)))
                .saturating_add(phonetic_gap_after(&gaps_after, boundary));
        }
        runs.push(PhoneticJukugoRun {
            base: raw.base.clone(),
            annotation: raw.annotation.clone(),
            base_start,
            base_end,
            annotation_start: 0,
            annotation_width: raw.annotation_width,
            ruby_em: raw.ruby_em,
            annotation_count: raw.annotation_count,
        });
    }

    let mut lower = Vec::with_capacity(runs.len());
    let mut upper = Vec::with_capacity(runs.len());
    for (index, run) in runs.iter().enumerate() {
        let minimum = if index == 0 {
            run.base_start
                .saturating_sub(before[index])
                .saturating_sub(i64::from(edges.leading_allowance))
        } else {
            runs[index.saturating_sub(1)]
                .base_end
                .saturating_sub(i64::from(run.ruby_em))
        };
        let maximum_end = if index.saturating_add(1) == runs.len() {
            run.base_end
                .saturating_add(after[index])
                .saturating_add(i64::from(edges.trailing_allowance))
        } else {
            runs[index.saturating_add(1)]
                .base_start
                .saturating_add(i64::from(run.ruby_em))
        };
        lower.push(minimum);
        upper.push(maximum_end.saturating_sub(run.annotation_width));
    }

    let mut latest = upper.clone();
    for index in (0..latest.len().saturating_sub(1)).rev() {
        latest[index] = latest[index]
            .min(latest[index.saturating_add(1)].saturating_sub(runs[index].annotation_width));
    }
    if latest
        .iter()
        .zip(&lower)
        .any(|(latest, lower)| latest < lower)
    {
        return None;
    }

    let mut previous_end: Option<i64> = None;
    for (index, run) in runs.iter_mut().enumerate() {
        let minimum = previous_end.map_or(lower[index], |end| end.max(lower[index]));
        let preferred = run.base_start.max(minimum);
        let start = preferred.min(latest[index]);
        if start < minimum {
            return None;
        }
        run.annotation_start = start;
        previous_end = Some(start.saturating_add(run.annotation_width));
    }

    Some(PhoneticJukugoPlan {
        runs,
        leading_gap: clamp_i32(leading_gap),
        gaps_after: gaps_after
            .into_iter()
            .map(|(ordinal, gap)| (ordinal, clamp_i32(gap)))
            .collect(),
    })
}

fn apportion_phonetic_expansion(
    runs: &[PhoneticJukugoRun],
    total: i64,
    remainder: Remainder,
) -> Vec<i64> {
    let weight = runs
        .iter()
        .filter(|run| run.annotation_count > 2)
        .fold(0_i64, |sum, run| {
            sum.saturating_add(run.annotation_width.max(1))
        });
    let mut assigned: Vec<_> = runs
        .iter()
        .map(|run| {
            if run.annotation_count > 2 && weight != 0 {
                total
                    .saturating_mul(run.annotation_width.max(1))
                    .checked_div(weight)
                    .unwrap_or(0)
            } else {
                0
            }
        })
        .collect();
    let mut remainder_units = total.saturating_sub(
        assigned
            .iter()
            .fold(0_i64, |sum, amount| sum.saturating_add(*amount)),
    );
    let indices: Vec<_> = match remainder {
        Remainder::Leading => (0..runs.len()).collect(),
        Remainder::Trailing => (0..runs.len()).rev().collect(),
    };
    for index in indices {
        if remainder_units == 0 {
            break;
        }
        if runs[index].annotation_count > 2 {
            assigned[index] = assigned[index].saturating_add(1);
            remainder_units = remainder_units.saturating_sub(1);
        }
    }
    assigned
}

fn add_phonetic_gap(gaps: &mut Vec<(usize, i64)>, ordinal: usize, amount: i64) {
    if let Some((_, gap)) = gaps.iter_mut().find(|(boundary, _)| *boundary == ordinal) {
        *gap = gap.saturating_add(amount);
    } else {
        gaps.push((ordinal, amount));
    }
}

fn phonetic_gap_after(gaps: &[(usize, i64)], ordinal: usize) -> i64 {
    gaps.iter()
        .find_map(|(boundary, gap)| (*boundary == ordinal).then_some(*gap))
        .unwrap_or(0)
}

fn ruby_annotation_metrics(
    annotation: &crate::ShapedText,
    range: &Range<usize>,
) -> (usize, i64, i32) {
    annotation
        .clusters()
        .iter()
        .filter(|cluster| {
            let cluster = cluster.range();
            range.start <= cluster.start && cluster.end <= range.end
        })
        .fold((0_usize, 0_i64, 0_i32), |(count, width, em), cluster| {
            (
                count.saturating_add(1),
                width.saturating_add(i64::from(cluster.advance())),
                em.max(
                    cluster
                        .size_override()
                        .unwrap_or(annotation.size())
                        .inline(),
                ),
            )
        })
}

fn ruby_base_width(paragraph: &Paragraph, style: &Style, base: &Range<usize>) -> i64 {
    base.clone().fold(0_i64, |sum, ordinal| {
        let boundary = if ordinal.saturating_add(1) < base.end {
            boundary_space_after_with_style(paragraph, style, ordinal)
        } else {
            0
        };
        sum.saturating_add(i64::from(effective_cluster_body_advance(
            paragraph, ordinal,
        )))
        .saturating_add(i64::from(boundary))
    })
}

fn base_distribution_plan(
    paragraph: &Paragraph,
    style: &Style,
    base_range: &Range<usize>,
    annotation: &crate::ShapedText,
    annotation_range: &Range<usize>,
    distribution: GroupRubyDistribution,
) -> Option<GroupRubyBasePlan> {
    let base = cluster_index_at_or_after(paragraph, base_range.start)
        ..cluster_index_at_or_after(paragraph, base_range.end);
    let count = base.end.saturating_sub(base.start);
    if count < 2 {
        return None;
    }
    let (_, annotation_width, _) = ruby_annotation_metrics(annotation, annotation_range);
    let surplus = annotation_width.saturating_sub(ruby_base_width(paragraph, style, &base));
    if surplus <= 0 {
        return None;
    }

    let weights = match distribution {
        GroupRubyDistribution::Jis => {
            let mut weights = Vec::with_capacity(count.saturating_add(1));
            weights.push(1);
            weights.extend((1..count).map(|_| 2));
            weights.push(1);
            weights
        },
        GroupRubyDistribution::Flush => vec![1; count.saturating_sub(1)],
    };
    let shares = proportional_shares(surplus, &weights, style.remainder());
    let (leading, trailing, interior) = match distribution {
        GroupRubyDistribution::Jis => (
            shares.first().copied().unwrap_or(0),
            shares.last().copied().unwrap_or(0),
            shares
                .iter()
                .skip(1)
                .take(count.saturating_sub(1))
                .copied()
                .collect::<Vec<_>>(),
        ),
        GroupRubyDistribution::Flush => (0, 0, shares),
    };
    let gaps_after = interior
        .into_iter()
        .enumerate()
        .map(|(offset, gap)| (base.start.saturating_add(offset), gap))
        .collect();
    Some(GroupRubyBasePlan {
        base,
        leading,
        trailing,
        gaps_after,
    })
}

fn group_ruby_base_plan(
    paragraph: &Paragraph,
    style: &Style,
    ruby: &Ruby,
    base: &Range<usize>,
    annotation: &Range<usize>,
) -> Option<GroupRubyBasePlan> {
    let distribution = match ruby.kind() {
        RubyKind::Group => style.group_ruby_distribution(),
        RubyKind::Jukugo
            if style.jukugo_ruby_layout() != JukugoRubyLayout::Phonetic && *base == ruby.base() =>
        {
            GroupRubyDistribution::Jis
        },
        RubyKind::Mono | RubyKind::Jukugo => return None,
    };
    base_distribution_plan(
        paragraph,
        style,
        base,
        ruby.annotation(),
        annotation,
        distribution,
    )
}

fn ruby_span_overhang(
    paragraph: &Paragraph,
    style: &Style,
    ruby: &Ruby,
    base: Range<usize>,
    annotation: Range<usize>,
    line_start: usize,
    line_end: usize,
) -> Option<RubyOverhang> {
    if group_ruby_base_plan(paragraph, style, ruby, &base, &annotation).is_some() {
        return None;
    }
    let base = cluster_index_at_or_after(paragraph, base.start)
        ..cluster_index_at_or_after(paragraph, base.end);
    if base.start < line_start || base.end > line_end || base.start >= base.end {
        return None;
    }
    let base_width = base.clone().fold(0_i64, |sum, ordinal| {
        let body = i64::from(effective_cluster_body_advance(paragraph, ordinal));
        let boundary = if ordinal.saturating_add(1) < base.end {
            i64::from(boundary_space_after_with_style(paragraph, style, ordinal))
        } else {
            0
        };
        sum.saturating_add(body).saturating_add(boundary)
    });
    let mut annotation_width = 0_i64;
    let mut ruby_em = 0_i32;
    for cluster in ruby.annotation().clusters().iter().filter(|cluster| {
        let cluster = cluster.range();
        annotation.start <= cluster.start && cluster.end <= annotation.end
    }) {
        annotation_width = annotation_width.saturating_add(i64::from(cluster.advance()));
        ruby_em = ruby_em.max(
            cluster
                .size_override()
                .unwrap_or(ruby.annotation().size())
                .inline(),
        );
    }
    let surplus = annotation_width.saturating_sub(base_width).max(0);
    if surplus == 0 {
        return None;
    }
    let half = surplus / 2;
    let odd = surplus % 2;
    let (leading, trailing) = match style.remainder() {
        Remainder::Leading => (half.saturating_add(odd), half),
        Remainder::Trailing => (half, half.saturating_add(odd)),
    };
    Some(RubyOverhang {
        base,
        leading: clamp_i32(leading),
        trailing: clamp_i32(trailing),
        ruby_em,
    })
}

fn ruby_boundary_separation_after(
    paragraph: &Paragraph,
    style: &Style,
    before: usize,
    line_start: usize,
    line_end: usize,
    line_index: usize,
) -> i32 {
    let mut required = 0_i32;
    let mut distributed = 0_i32;
    visit_ruby_spans(paragraph, style, |ruby, base, annotation| {
        if let Some(plan) = group_ruby_base_plan(paragraph, style, ruby, &base, &annotation) {
            if plan.base.start < line_start || plan.base.end > line_end {
                return;
            }
            if plan.base.start == before.saturating_add(1) && before >= line_start {
                distributed = distributed.saturating_add(plan.leading);
            }
            distributed = distributed.saturating_add(plan.gap_after(before));
            if plan.base.end == before.saturating_add(1) {
                distributed = distributed.saturating_add(plan.trailing);
            }
            return;
        }
        let Some(overhang) = ruby_span_overhang(
            paragraph, style, ruby, base, annotation, line_start, line_end,
        ) else {
            return;
        };
        if overhang.base.start == before.saturating_add(1) && before >= line_start {
            let allowance = ruby_neighbor_overhang_allowance(
                paragraph,
                style,
                before,
                RubySide::Leading,
                overhang.ruby_em,
            );
            required = required.max(overhang.leading.saturating_sub(allowance));
        }
        if overhang.base.end == before.saturating_add(1) {
            let allowance = if overhang.base.end < line_end {
                ruby_neighbor_overhang_allowance(
                    paragraph,
                    style,
                    overhang.base.end,
                    RubySide::Trailing,
                    overhang.ruby_em,
                )
            } else {
                0
            };
            required = required.max(overhang.trailing.saturating_sub(allowance));
        }
    });
    for construct in &paragraph.constructs {
        let ConstructKind::Ruby(ruby) = construct.kind() else {
            continue;
        };
        if let Some(plan) =
            phonetic_jukugo_plan(paragraph, style, ruby, line_start, line_end, line_index)
        {
            required = required.max(plan.gap_after(before));
        }
    }
    required.saturating_add(distributed)
}

fn ruby_line_leading_separation(
    paragraph: &Paragraph,
    style: &Style,
    line_start: usize,
    line_end: usize,
    line_index: usize,
) -> i32 {
    let mut required = 0_i32;
    let mut distributed = 0_i32;
    visit_ruby_spans(paragraph, style, |ruby, base, annotation| {
        if let Some(plan) = group_ruby_base_plan(paragraph, style, ruby, &base, &annotation) {
            if plan.base.start == line_start && plan.base.end <= line_end {
                distributed = distributed.saturating_add(plan.leading);
            }
            return;
        }
        let Some(overhang) = ruby_span_overhang(
            paragraph, style, ruby, base, annotation, line_start, line_end,
        ) else {
            return;
        };
        if overhang.base.start != line_start {
            return;
        }
        let allowance =
            if line_index == 0 && style.ruby_overhang_indent() == RubyOverhangIndent::Permitted {
                line_head_indent(paragraph, style, line_start, line_index).min(overhang.ruby_em)
            } else {
                0
            };
        required = required.max(overhang.leading.saturating_sub(allowance));
    });
    for construct in &paragraph.constructs {
        let ConstructKind::Ruby(ruby) = construct.kind() else {
            continue;
        };
        if let Some(plan) =
            phonetic_jukugo_plan(paragraph, style, ruby, line_start, line_end, line_index)
        {
            required = required.max(plan.leading_gap);
        }
    }
    required.saturating_add(distributed)
}

fn ruby_neighbor_overhang_allowance(
    paragraph: &Paragraph,
    style: &Style,
    ordinal: usize,
    side: RubySide,
    ruby_em: i32,
) -> i32 {
    let Some(cluster) = paragraph.text.clusters().get(ordinal) else {
        return 0;
    };
    let Some(character) = single_cluster_character(paragraph, cluster) else {
        return 0;
    };
    let adjacent_space = match side {
        RubySide::Leading => boundary_space_after_with_style(paragraph, style, ordinal),
        RubySide::Trailing => ordinal.checked_sub(1).map_or(0, |before| {
            boundary_space_after_with_style(paragraph, style, before)
        }),
    };
    let punctuation_allowance = match side {
        RubySide::Leading if is_opening_bracket(character) => ruby_em,
        RubySide::Leading
            if is_closing_bracket(character) || is_full_stop(character) || is_comma(character) =>
        {
            ruby_em.min(adjacent_space)
        },
        RubySide::Trailing if is_opening_bracket(character) => ruby_em.min(adjacent_space),
        RubySide::Trailing
            if is_closing_bracket(character) || is_full_stop(character) || is_comma(character) =>
        {
            ruby_em
        },
        _ => 0,
    };
    let kana = match style.ruby_overhang_kana() {
        RubyOverhangKana::Kana => is_hiragana(character) || is_katakana(character),
        RubyOverhangKana::Jis => is_hiragana(character),
        RubyOverhangKana::Any => true,
        RubyOverhangKana::None => false,
    };
    if punctuation_allowance != 0 {
        punctuation_allowance
    } else if is_middle_dot(character) {
        ruby_em.min(adjacent_space.saturating_add(half_rounded_up(ruby_em)))
    } else if character == '\u{3000}' || is_inseparable_character(character) || kana {
        ruby_em
    } else {
        0
    }
}

fn half_rounded_up(value: i32) -> i32 {
    (value / 2).saturating_add(value % 2)
}

fn is_hiragana(character: char) -> bool {
    crate::spec::is_hiragana(character)
}

fn is_katakana(character: char) -> bool {
    crate::spec::is_katakana(character)
}

fn is_inseparable_character(character: char) -> bool {
    crate::spec::single_has_class(character, crate::spec::INSEPARABLE)
}

fn is_western_word_space(paragraph: &Paragraph, ordinal: usize) -> bool {
    let Some(cluster) = paragraph.text.clusters().get(ordinal) else {
        return false;
    };
    &paragraph.text.source()[cluster.range()] == " "
        && cluster.frame_override().unwrap_or(paragraph.text.frame()) == Frame::Proportional
        && matches!(cluster.role(), None | Some(ClusterRole::Text))
}

fn boundary_space_after(paragraph: &Paragraph, ordinal: usize) -> i32 {
    tate_chu_yoko_boundary_space_after(paragraph, ordinal)
        .or_else(|| formula_boundary_space_after(paragraph, ordinal))
        .unwrap_or_else(|| ordinary_boundary_space_after(paragraph, ordinal))
}

fn boundary_space_after_with_style(paragraph: &Paragraph, style: &Style, ordinal: usize) -> i32 {
    tate_chu_yoko_boundary_space_after(paragraph, ordinal)
        .or_else(|| formula_boundary_space_after(paragraph, ordinal))
        .unwrap_or_else(|| ordinary_boundary_space_after_with_style(paragraph, style, ordinal))
}

fn tate_chu_yoko_boundary_space_after(paragraph: &Paragraph, ordinal: usize) -> Option<i32> {
    if paragraph.writing_mode != WritingMode::VerticalRl {
        return None;
    }
    let clusters = paragraph.text.clusters();
    let current_group = tate_chu_yoko_cluster_range(paragraph, ordinal);
    if let Some(group) = current_group {
        if group.start != ordinal || group.end >= clusters.len() {
            return Some(0);
        }
        let following = &clusters[group.end];
        let character = single_cluster_character(paragraph, following);
        if character.is_some_and(is_opening_bracket) {
            return Some(half_inline_size(paragraph, following));
        }
        return Some(0);
    }

    let following_ordinal = ordinal.saturating_add(1);
    if following_ordinal >= clusters.len()
        || tate_chu_yoko_cluster_range(paragraph, following_ordinal)
            .is_none_or(|group| group.start != following_ordinal)
    {
        return None;
    }
    let current = &clusters[ordinal];
    let character = single_cluster_character(paragraph, current);
    if character.is_some_and(|character| {
        is_closing_bracket(character) || is_full_stop(character) || is_comma(character)
    }) {
        Some(half_inline_size(paragraph, current))
    } else {
        Some(0)
    }
}

fn ordinary_boundary_space_after(paragraph: &Paragraph, ordinal: usize) -> i32 {
    let clusters = paragraph.text.clusters();
    let Some(current) = clusters.get(ordinal) else {
        return 0;
    };
    let Some(following) = clusters.get(ordinal.saturating_add(1)) else {
        return 0;
    };
    let current_character = single_cluster_character(paragraph, current);
    let following_character = single_cluster_character(paragraph, following);
    let current_solid = current_character
        .is_some_and(|character| contextual_punctuation_is_solid(paragraph, current, character));
    let following_solid = following_character
        .is_some_and(|character| contextual_punctuation_is_solid(paragraph, following, character));
    let current_size = current.size_override().unwrap_or(paragraph.text.size());
    let following_size = following.size_override().unwrap_or(paragraph.text.size());
    crate::spec::table_one_space(
        class_of_cluster(paragraph, ordinal),
        class_of_cluster(paragraph, ordinal.saturating_add(1)),
        current_size,
        following_size,
        current_solid,
        following_solid,
    )
}

fn ordinary_boundary_space_after_with_style(
    paragraph: &Paragraph,
    style: &Style,
    ordinal: usize,
) -> i32 {
    let clusters = paragraph.text.clusters();
    let Some(current) = clusters.get(ordinal) else {
        return 0;
    };
    let Some(following) = clusters.get(ordinal.saturating_add(1)) else {
        return 0;
    };
    let current_character = single_cluster_character(paragraph, current);
    let following_character = single_cluster_character(paragraph, following);
    let current_solid = current_character
        .is_some_and(|character| contextual_punctuation_is_solid(paragraph, current, character));
    let following_solid = following_character
        .is_some_and(|character| contextual_punctuation_is_solid(paragraph, following, character));
    let current_size = current.size_override().unwrap_or(paragraph.text.size());
    let following_size = following.size_override().unwrap_or(paragraph.text.size());
    let before = class_of_cluster_with_style(paragraph, style, ordinal);
    let after = class_of_cluster_with_style(paragraph, style, ordinal.saturating_add(1));
    if before == 4 && current.role() == Some(ClusterRole::SentenceTerminator) {
        return if after == crate::spec::CLOSING_BRACKET {
            0
        } else {
            current_size.inline()
        };
    }
    let table = crate::spec::table_one_space(
        before,
        after,
        current_size,
        following_size,
        current_solid,
        following_solid,
    );
    let table_is_blank = crate::generated::table1::CELLS
        .iter()
        .find(|cell| cell.before == before && cell.after == after)
        .is_none_or(|cell| cell.terms.is_empty());
    if !table_is_blank
        || style.sentence_medial_dividing_mark() != SentenceMedialDividingMark::QuarterEm
    {
        return table;
    }
    let before_quarter = if before == 4 && current.role() == Some(ClusterRole::SentenceMedial) {
        quarter_inline_size(paragraph, current)
    } else {
        0
    };
    let after_quarter = if after == 4 && following.role() == Some(ClusterRole::SentenceMedial) {
        quarter_inline_size(paragraph, following)
    } else {
        0
    };
    table
        .saturating_add(before_quarter)
        .saturating_add(after_quarter)
}

fn class_of_cluster(paragraph: &Paragraph, ordinal: usize) -> u8 {
    class_of_cluster_impl(paragraph, None, ordinal)
}

fn class_of_cluster_with_style(paragraph: &Paragraph, style: &Style, ordinal: usize) -> u8 {
    class_of_cluster_impl(paragraph, Some(style), ordinal)
}

fn class_of_cluster_impl(paragraph: &Paragraph, style: Option<&Style>, ordinal: usize) -> u8 {
    if tate_chu_yoko_cluster_range(paragraph, ordinal).is_some() {
        return 30;
    }
    let cluster = &paragraph.text.clusters()[ordinal];
    let range = cluster.range();
    for construct in &paragraph.constructs {
        if !ranges_overlap(&construct.range(), &range) {
            continue;
        }
        match construct.kind() {
            ConstructKind::Ruby(ruby) => {
                return if ruby.kind() == RubyKind::Jukugo {
                    23
                } else {
                    22
                };
            },
            ConstructKind::Emphasis { .. } | ConstructKind::Script { .. } => return 21,
            ConstructKind::ReferenceMark { .. } => return 20,
            _ => {},
        }
    }
    let frame = cluster.frame_override().unwrap_or(paragraph.text.frame());
    crate::spec::class_of(
        &paragraph.text.source()[range],
        frame,
        cluster.role(),
        paragraph.writing_mode,
        style.is_some_and(|style| style.unlisted_code_point() == UnlistedCodePoint::Ideographic),
        style.is_some_and(|style| style.ambiguous_context() == AmbiguousContext::HighestClass),
        style.is_some_and(|style| {
            style.grouped_numeral_qualification() == GroupedNumeralQualification::ByRole
        }),
    )
}

fn contextual_punctuation_is_solid(
    paragraph: &Paragraph,
    cluster: &crate::Cluster,
    character: char,
) -> bool {
    match cluster.role() {
        Some(ClusterRole::WarichuBracket) => {
            is_opening_bracket(character) || is_closing_bracket(character)
        },
        Some(ClusterRole::DecimalPoint) => {
            character == '・' && paragraph.writing_mode == WritingMode::VerticalRl
        },
        Some(ClusterRole::DigitGroupSeparator) => {
            character == '、' && paragraph.writing_mode == WritingMode::VerticalRl
        },
        Some(ClusterRole::GroupedNumeral | ClusterRole::UnitSymbol | ClusterRole::Formula) => {
            character == '・'
        },
        _ => false,
    }
}

fn formula_boundary_space_after(paragraph: &Paragraph, ordinal: usize) -> Option<i32> {
    let clusters = paragraph.text.clusters();
    let following_ordinal = ordinal.saturating_add(1);
    let current_formula = formula_cluster_range(paragraph, ordinal);
    let following_formula = formula_cluster_range(paragraph, following_ordinal);
    if current_formula.is_none() && following_formula.is_none() {
        return None;
    }
    let current = clusters.get(ordinal)?;
    let following = clusters.get(following_ordinal)?;

    match (current_formula, following_formula) {
        (Some(current_range), Some(following_range)) if current_range == following_range => {
            if current_range.start == 0 && current_range.end == clusters.len() {
                let current_character = single_cluster_character(paragraph, current);
                let following_character = single_cluster_character(paragraph, following);
                let symbol_boundary = current_character.is_some_and(is_math_symbol)
                    ^ following_character.is_some_and(is_math_symbol);
                if symbol_boundary {
                    let symbol = if current_character.is_some_and(is_math_symbol) {
                        current
                    } else {
                        following
                    };
                    return Some(quarter_inline_size(paragraph, symbol));
                }
                if current_character.is_some_and(is_math_operator)
                    || following_character.is_some_and(is_math_operator)
                {
                    return Some(0);
                }
            }
            Some(0)
        },
        (None, Some(range)) if range.start == following_ordinal => {
            Some(formula_outer_boundary_space(paragraph, current, following))
        },
        (Some(range), None) if range.end == following_ordinal => {
            Some(formula_outer_boundary_space(paragraph, following, current))
        },
        _ => Some(0),
    }
}

fn formula_outer_boundary_space(
    paragraph: &Paragraph,
    outside: &crate::Cluster,
    endpoint: &crate::Cluster,
) -> i32 {
    if is_japanese_formula_neighbor(paragraph, outside)
        && formula_endpoint_needs_quarter(paragraph, endpoint)
    {
        quarter_inline_size(paragraph, endpoint)
    } else {
        0
    }
}

fn formula_endpoint_needs_quarter(paragraph: &Paragraph, cluster: &crate::Cluster) -> bool {
    let character = single_cluster_character(paragraph, cluster);
    !character.is_some_and(is_math_token)
        && (cluster.frame_override().unwrap_or(paragraph.text.frame()) == Frame::Proportional
            || cluster.role() == Some(ClusterRole::GroupedNumeral))
}

fn is_japanese_formula_neighbor(paragraph: &Paragraph, cluster: &crate::Cluster) -> bool {
    if cluster.frame_override().unwrap_or(paragraph.text.frame()) == Frame::Proportional {
        return false;
    }
    single_cluster_character(paragraph, cluster).is_some_and(|character| {
        !character.is_whitespace()
            && !is_opening_bracket(character)
            && !is_closing_bracket(character)
            && !is_full_stop(character)
            && !is_comma(character)
            && !is_middle_dot(character)
            && !is_math_token(character)
    })
}

fn furawake_segment(
    paragraph: &Paragraph,
    range: Range<usize>,
    columns: u16,
    line_gap: i32,
    line_end: usize,
) -> FurawakeSegment {
    let mut lanes = Vec::with_capacity(usize::from(columns));
    let start_offset = paragraph.text.clusters()[range.start].range().start;
    let end_offset = paragraph.text.clusters()[range.end.saturating_sub(1)]
        .range()
        .end;
    let mut start = range.start;
    for opportunity in paragraph.breaks.iter().filter(|opportunity| {
        start_offset < opportunity.offset() && opportunity.offset() < end_offset
    }) {
        let split = cluster_index_at_or_after(paragraph, opportunity.offset());
        lanes.push(start..split);
        start = split;
    }
    lanes.push(start..range.end);

    let widths: Vec<_> = lanes
        .iter()
        .map(|lane| construct_lane_width(paragraph, lane))
        .collect();
    let block_extents: Vec<_> = lanes
        .iter()
        .map(|lane| construct_lane_block_extent(paragraph, lane))
        .collect();
    let block_extent = block_extents
        .iter()
        .copied()
        .fold(0_i32, i32::saturating_add)
        .saturating_add(
            line_gap
                .saturating_mul(i32::try_from(lanes.len().saturating_sub(1)).unwrap_or(i32::MAX)),
        );
    let outer_space = if range.end < line_end {
        boundary_space_after(paragraph, range.end.saturating_sub(1))
    } else {
        0
    };
    let advance = widths
        .iter()
        .copied()
        .max()
        .unwrap_or(0)
        .saturating_add(outer_space);
    FurawakeSegment {
        range,
        lanes,
        block_extents,
        line_gap,
        advance,
        block_extent,
    }
}

fn construct_lane_width(paragraph: &Paragraph, lane: &Range<usize>) -> i32 {
    lane.clone().fold(0_i32, |sum, ordinal| {
        let logical_end = tate_chu_yoko_cluster_range(paragraph, ordinal)
            .map_or_else(|| ordinal.saturating_add(1), |group| group.end);
        let boundary = if logical_end < lane.end {
            boundary_space_after(paragraph, ordinal)
        } else {
            0
        };
        sum.saturating_add(nested_cluster_body_advance(paragraph, ordinal))
            .saturating_add(boundary)
    })
}

fn construct_lane_block_extent(paragraph: &Paragraph, lane: &Range<usize>) -> i32 {
    paragraph.text.clusters()[lane.clone()]
        .iter()
        .map(|cluster| {
            cluster
                .size_override()
                .unwrap_or(paragraph.text.size())
                .block()
        })
        .max()
        .unwrap_or(0)
}

fn place_furawake_segment(
    paragraph: &Paragraph,
    segment: &FurawakeSegment,
    inline: i64,
    block_origin: i32,
    placed: &mut Vec<ClusterPlacement>,
) {
    let main_block_extent = paragraph.text.size().block();
    let mut block = match paragraph.writing_mode {
        WritingMode::HorizontalTb => i64::from(block_origin).saturating_add(i64::from(
            main_block_extent.saturating_sub(segment.block_extent) / 2,
        )),
        WritingMode::VerticalRl => i64::from(block_origin).saturating_add(i64::from(
            segment.block_extent.saturating_sub(main_block_extent) / 2,
        )),
    };
    for (lane_index, lane) in segment.lanes.iter().enumerate() {
        let mut cursor = inline;
        for ordinal in lane.clone() {
            let logical_end = tate_chu_yoko_cluster_range(paragraph, ordinal)
                .map_or_else(|| ordinal.saturating_add(1), |group| group.end);
            let boundary = if logical_end < lane.end {
                boundary_space_after(paragraph, ordinal)
            } else {
                0
            };
            let advance = nested_cluster_body_advance(paragraph, ordinal).saturating_add(boundary);
            place_construct_member(paragraph, ordinal, cursor, block, advance, placed);
            cursor = cursor.saturating_add(i64::from(advance));
        }
        let step = segment.block_extents[lane_index].saturating_add(segment.line_gap);
        match paragraph.writing_mode {
            WritingMode::HorizontalTb => {
                block = block.saturating_add(i64::from(step));
            },
            WritingMode::VerticalRl => {
                block = block.saturating_sub(i64::from(step));
            },
        }
    }
}

fn place_construct_member(
    paragraph: &Paragraph,
    ordinal: usize,
    inline: i64,
    block: i64,
    advance: i32,
    placed: &mut Vec<ClusterPlacement>,
) {
    let cluster = &paragraph.text.clusters()[ordinal];
    let size = cluster.size_override().unwrap_or(paragraph.text.size());
    let frame = cluster.frame_override().unwrap_or(paragraph.text.frame());
    let (writing_mode, transform) = local_orientation(paragraph, ordinal, frame);
    placed.push(ClusterPlacement {
        origin: PlacementOrigin::Cluster(ordinal),
        range: cluster.range(),
        inline: clamp_i32(inline),
        block: clamp_i32(block),
        advance,
        size,
        frame,
        writing_mode,
        transform,
    });
}

fn warichu_segment(
    paragraph: &Paragraph,
    full_range: Range<usize>,
    line_start: usize,
    line_end: usize,
) -> WarichuSegment {
    let range = full_range.start.max(line_start)..full_range.end.min(line_end);
    let clusters = paragraph.text.clusters();
    let leading_bracket = (range.start == full_range.start)
        .then_some(range.start)
        .filter(|ordinal| {
            clusters
                .get(*ordinal)
                .is_some_and(|cluster| cluster.role() == Some(ClusterRole::WarichuBracket))
        });
    let trailing_ordinal = range.end.saturating_sub(1);
    let trailing_bracket = (range.end == full_range.end)
        .then_some(trailing_ordinal)
        .filter(|ordinal| {
            clusters
                .get(*ordinal)
                .is_some_and(|cluster| cluster.role() == Some(ClusterRole::WarichuBracket))
        });
    let interior_start = leading_bracket.map_or(range.start, |ordinal| ordinal.saturating_add(1));
    let interior_end = trailing_bracket.unwrap_or(range.end);
    let split = choose_warichu_split(paragraph, interior_start..interior_end);
    let first_lane = interior_start..split;
    let second_lane = split..interior_end;
    let first_width = warichu_lane_width(paragraph, &first_lane);
    let second_width = warichu_lane_width(paragraph, &second_lane);
    let leading_width =
        leading_bracket.map_or(0, |ordinal| paragraph.text.clusters()[ordinal].advance());
    let trailing_width =
        trailing_bracket.map_or(0, |ordinal| paragraph.text.clusters()[ordinal].advance());
    let outer_space = if range.end < line_end {
        boundary_space_after(paragraph, range.end.saturating_sub(1))
    } else {
        0
    };
    let advance = leading_width
        .saturating_add(first_width.max(second_width))
        .saturating_add(trailing_width)
        .saturating_add(outer_space);
    WarichuSegment {
        range,
        leading_bracket,
        first_lane,
        second_lane,
        trailing_bracket,
        first_width,
        second_width,
        advance,
    }
}

fn choose_warichu_split(paragraph: &Paragraph, interior: Range<usize>) -> usize {
    if interior.end.saturating_sub(interior.start) < 2 {
        return interior.end;
    }
    let mut best = None;
    for require_declared in [true, false] {
        for split in interior.start.saturating_add(1)..interior.end {
            let offset = paragraph.text.clusters()[split].range().start;
            let declared = paragraph
                .breaks
                .iter()
                .any(|opportunity| opportunity.offset() == offset);
            if require_declared != declared {
                continue;
            }
            let first = warichu_lane_width(paragraph, &(interior.start..split));
            let second = warichu_lane_width(paragraph, &(split..interior.end));
            let score = (
                i32::from(second > first),
                first.saturating_sub(second).saturating_abs(),
            );
            if best.is_none_or(|(_, best_score)| score < best_score) {
                best = Some((split, score));
            }
        }
        if best.is_some() {
            break;
        }
    }
    best.map_or(interior.end, |(split, _)| split)
}

fn warichu_lane_width(paragraph: &Paragraph, lane: &Range<usize>) -> i32 {
    lane.clone().fold(0_i32, |sum, ordinal| {
        sum.saturating_add(warichu_member_advance(paragraph, ordinal, lane))
    })
}

fn warichu_member_advance(paragraph: &Paragraph, ordinal: usize, lane: &Range<usize>) -> i32 {
    if is_western_word_space(paragraph, ordinal)
        && (ordinal == lane.start || ordinal.saturating_add(1) == lane.end)
    {
        return 0;
    }
    let logical_end = tate_chu_yoko_cluster_range(paragraph, ordinal)
        .map_or_else(|| ordinal.saturating_add(1), |group| group.end);
    let boundary = if logical_end < lane.end {
        boundary_space_after(paragraph, ordinal)
    } else {
        0
    };
    effective_cluster_body_advance(paragraph, ordinal).saturating_add(boundary)
}

fn place_warichu_segment(
    paragraph: &Paragraph,
    segment: &WarichuSegment,
    cursor: i64,
    block_origin: i32,
    placed: &mut Vec<ClusterPlacement>,
) {
    let leading_width = segment
        .leading_bracket
        .map_or(0, |ordinal| paragraph.text.clusters()[ordinal].advance());
    if let Some(ordinal) = segment.leading_bracket {
        place_warichu_member(
            paragraph,
            ordinal,
            cursor,
            i64::from(block_origin),
            paragraph.text.clusters()[ordinal].advance(),
            placed,
        );
    }

    let lane_inline = cursor.saturating_add(i64::from(leading_width));
    let first_block_extent = warichu_lane_block_extent(paragraph, &segment.first_lane);
    let second_block_extent = warichu_lane_block_extent(paragraph, &segment.second_lane);
    let total_block_extent = first_block_extent.saturating_add(second_block_extent);
    let main_block_extent = paragraph.text.size().block();
    let first_block = match paragraph.writing_mode {
        WritingMode::HorizontalTb => i64::from(block_origin).saturating_add(i64::from(
            main_block_extent.saturating_sub(total_block_extent) / 2,
        )),
        WritingMode::VerticalRl => i64::from(block_origin).saturating_add(i64::from(
            total_block_extent.saturating_sub(main_block_extent) / 2,
        )),
    };
    let second_block = match paragraph.writing_mode {
        WritingMode::HorizontalTb => first_block.saturating_add(i64::from(first_block_extent)),
        WritingMode::VerticalRl => first_block.saturating_sub(i64::from(first_block_extent)),
    };
    place_warichu_lane(
        paragraph,
        &segment.first_lane,
        lane_inline,
        first_block,
        placed,
    );
    place_warichu_lane(
        paragraph,
        &segment.second_lane,
        lane_inline,
        second_block,
        placed,
    );

    if let Some(ordinal) = segment.trailing_bracket {
        let inline =
            lane_inline.saturating_add(i64::from(segment.first_width.max(segment.second_width)));
        place_warichu_member(
            paragraph,
            ordinal,
            inline,
            i64::from(block_origin),
            paragraph.text.clusters()[ordinal].advance(),
            placed,
        );
    }
}

fn place_warichu_lane(
    paragraph: &Paragraph,
    lane: &Range<usize>,
    inline: i64,
    block: i64,
    placed: &mut Vec<ClusterPlacement>,
) {
    let mut cursor = inline;
    for ordinal in lane.clone() {
        let advance = warichu_member_advance(paragraph, ordinal, lane);
        place_warichu_member(paragraph, ordinal, cursor, block, advance, placed);
        cursor = cursor.saturating_add(i64::from(advance));
    }
}

fn place_warichu_member(
    paragraph: &Paragraph,
    ordinal: usize,
    inline: i64,
    block: i64,
    advance: i32,
    placed: &mut Vec<ClusterPlacement>,
) {
    let cluster = &paragraph.text.clusters()[ordinal];
    let size = cluster.size_override().unwrap_or(paragraph.text.size());
    let frame = cluster.frame_override().unwrap_or(paragraph.text.frame());
    let (writing_mode, transform) = local_orientation(paragraph, ordinal, frame);
    placed.push(ClusterPlacement {
        origin: PlacementOrigin::Cluster(ordinal),
        range: cluster.range(),
        inline: clamp_i32(inline),
        block: clamp_i32(block),
        advance,
        size,
        frame,
        writing_mode,
        transform,
    });
}

fn warichu_lane_block_extent(paragraph: &Paragraph, lane: &Range<usize>) -> i32 {
    paragraph.text.clusters()[lane.clone()]
        .iter()
        .map(|cluster| {
            cluster
                .size_override()
                .unwrap_or(paragraph.text.size())
                .block()
        })
        .max()
        .unwrap_or(0)
}

fn warichu_break_penalty(paragraph: &Paragraph, offset: usize) -> i64 {
    paragraph
        .constructs
        .iter()
        .filter_map(|construct| {
            let ConstructKind::Warichu(range) = construct.kind() else {
                return None;
            };
            if !(range.start < offset && offset < range.end) {
                return None;
            }
            let start = cluster_index_at_or_after(paragraph, range.start);
            let split = cluster_index_at_or_after(paragraph, offset);
            let end = cluster_index_at_or_after(paragraph, range.end);
            let before = split.saturating_sub(start);
            let after = end.saturating_sub(split);
            Some(
                i64::try_from(before.abs_diff(after))
                    .unwrap_or(i64::MAX)
                    .saturating_mul(1_000_000),
            )
        })
        .fold(0_i64, i64::saturating_add)
}

fn formula_break_penalty(paragraph: &Paragraph, offset: usize) -> i64 {
    let is_independent_formula = paragraph.constructs.iter().any(|construct| {
        matches!(
            construct.kind(),
            ConstructKind::Formula(range)
                if range.start == 0
                    && range.end == paragraph.text.source().len()
                    && range.start < offset
                    && offset < range.end
        )
    });
    if !is_independent_formula {
        return 0;
    }
    let after = paragraph.text.source()[offset..].chars().next();
    if after.is_some_and(is_math_symbol) {
        0
    } else if after.is_some_and(is_math_operator) {
        100_000_000
    } else {
        200_000_000
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

fn quarter_inline_size(paragraph: &Paragraph, cluster: &crate::Cluster) -> i32 {
    let size = cluster
        .size_override()
        .unwrap_or(paragraph.text.size())
        .inline();
    (size / 4).saturating_add(i32::from(size % 4 != 0))
}

fn is_opening_bracket(character: char) -> bool {
    crate::spec::single_has_class(character, crate::spec::OPENING_BRACKET)
}

fn is_closing_bracket(character: char) -> bool {
    crate::spec::single_has_class(character, crate::spec::CLOSING_BRACKET)
}

fn is_full_stop(character: char) -> bool {
    crate::spec::single_has_class(character, crate::spec::FULL_STOP)
}

fn is_comma(character: char) -> bool {
    crate::spec::single_has_class(character, crate::spec::COMMA)
}

fn is_middle_dot(character: char) -> bool {
    crate::spec::single_has_class(character, crate::spec::MIDDLE_DOT)
}

fn line_head_indent(
    paragraph: &Paragraph,
    style: &Style,
    line_start: usize,
    line_index: usize,
) -> i32 {
    let ordinary = if line_index == 0 {
        paragraph.first_line_indent
    } else {
        0
    };
    let Some(cluster) = paragraph.text.clusters().get(line_start) else {
        return ordinary;
    };
    if class_of_cluster_with_style(paragraph, style, line_start) != crate::spec::OPENING_BRACKET {
        return ordinary;
    }
    let half = half_inline_size(paragraph, cluster);
    match (line_index == 0, style.line_head_opening_bracket()) {
        (_, LineHeadOpeningBracket::Pattern2) => ordinary.saturating_add(half),
        (true, LineHeadOpeningBracket::Pattern3) => ordinary.saturating_sub(half),
        _ => ordinary,
    }
}

/// Whether the cluster at `ordinal` is a tab sign **of the line** in §3.6.3's sense.
///
/// §3.6.3 corresponds the signs of a line with the stops of that line, and both halves
/// of that sentence are about the line's own inline axis. A tate-chu-yoko run runs
/// across that axis and a warichu's and a furawake's sublines run beside it, so each of
/// those structures holds one position on the line however many characters it holds,
/// and a coordinate inside one is not a position a stop can name. A sign standing there
/// is therefore not a sign of the line: it takes no stop, sets the advance it was shaped
/// with, and never chooses §3.6.3's cut — including where it is the structure's first
/// character, which is set in the structure like every other one
/// (`docs/decisions/tab-line-correspondence.md`).
fn is_line_tab_sign(paragraph: &Paragraph, ordinal: usize) -> bool {
    let Some(cluster) = paragraph.text.clusters().get(ordinal) else {
        return false;
    };
    &paragraph.text.source()[cluster.range()] == "\t"
        && tate_chu_yoko_cluster_range(paragraph, ordinal).is_none()
        && warichu_cluster_range(paragraph, ordinal).is_none()
        && furawake_cluster_range(paragraph, ordinal).is_none()
}

fn measure_line(
    paragraph: &Paragraph,
    style: &Style,
    start: usize,
    end: usize,
    line_number: usize,
) -> i64 {
    let start_cluster = cluster_index_at_or_after(paragraph, start);
    let end_cluster = cluster_index_at_or_after(paragraph, end);
    let indent = i64::from(line_head_indent(
        paragraph,
        style,
        start_cluster,
        line_number,
    ));
    let mut cursor = indent.saturating_add(i64::from(ruby_line_leading_separation(
        paragraph,
        style,
        start_cluster,
        end_cluster,
        line_number,
    )));
    let mut tab_index = 0;
    for local in 0..end_cluster.saturating_sub(start_cluster) {
        let ordinal = start_cluster.saturating_add(local);
        if is_line_tab_sign(paragraph, ordinal) {
            let after_tab = ordinal.saturating_add(1);
            let segment_width = segment_width(
                paragraph,
                style,
                after_tab,
                end_cluster,
                LineContext {
                    start: start_cluster,
                    end: end_cluster,
                    index: line_number,
                },
            );
            if let Some(stop) = paragraph
                .tab_stops
                .iter()
                .skip(tab_index)
                .find(|stop| i64::from(stop.position()) > cursor)
            {
                tab_index = tab_index.saturating_add(1);
                let target = tab_target(
                    paragraph,
                    style,
                    *stop,
                    after_tab,
                    end_cluster,
                    LineContext {
                        start: start_cluster,
                        end: end_cluster,
                        index: line_number,
                    },
                    segment_width,
                );
                cursor = cursor.max(target);
            } else {
                return i64::MAX;
            }
        } else {
            cursor = cursor.saturating_add(i64::from(effective_cluster_advance_on_line(
                paragraph,
                style,
                ordinal,
                start_cluster,
                end_cluster,
                line_number,
            )));
        }
    }
    cursor
}

fn segment_width(
    paragraph: &Paragraph,
    style: &Style,
    start: usize,
    end: usize,
    line: LineContext,
) -> i64 {
    (start..end)
        .take_while(|ordinal| !is_line_tab_sign(paragraph, *ordinal))
        .fold(0_i64, |sum, ordinal| {
            sum.saturating_add(i64::from(effective_cluster_advance_on_line(
                paragraph, style, ordinal, line.start, line.end, line.index,
            )))
        })
}

fn tab_target(
    paragraph: &Paragraph,
    style: &Style,
    stop: TabStop,
    start: usize,
    end: usize,
    line: LineContext,
    segment_width: i64,
) -> i64 {
    let position = i64::from(stop.position());
    match stop.alignment() {
        TabAlignment::Start => position,
        TabAlignment::Center => position.saturating_sub(segment_width / 2),
        TabAlignment::End => position.saturating_sub(segment_width),
        TabAlignment::Character(character) => {
            let before = (start..end)
                .take_while(|ordinal| {
                    let cluster = paragraph.text.clusters()[*ordinal].range();
                    !paragraph.text.source()[cluster].contains(character)
                        && !is_line_tab_sign(paragraph, *ordinal)
                })
                .fold(0_i64, |sum, ordinal| {
                    sum.saturating_add(i64::from(effective_cluster_advance_on_line(
                        paragraph, style, ordinal, line.start, line.end, line.index,
                    )))
                });
            position.saturating_sub(before)
        },
    }
}

fn apply_tabs(
    paragraph: &Paragraph,
    style: &Style,
    start: usize,
    end: usize,
    line_number: usize,
    advances: &mut [i32],
) {
    let mut cursor = i64::from(line_head_indent(paragraph, style, start, line_number));
    let mut tab_index = 0;
    for local in 0..advances.len() {
        let ordinal = start.saturating_add(local);
        let after_tab = local.saturating_add(1);
        if is_line_tab_sign(paragraph, ordinal) {
            let width = advances[after_tab..]
                .iter()
                .enumerate()
                .take_while(|(following, _)| {
                    !is_line_tab_sign(
                        paragraph,
                        ordinal.saturating_add(1).saturating_add(*following),
                    )
                })
                .fold(0_i64, |sum, (_, advance)| {
                    sum.saturating_add(i64::from(*advance))
                });
            if let Some(stop) = paragraph
                .tab_stops
                .iter()
                .skip(tab_index)
                .find(|stop| i64::from(stop.position()) > cursor)
            {
                tab_index = tab_index.saturating_add(1);
                let target = tab_target(
                    paragraph,
                    style,
                    *stop,
                    ordinal.saturating_add(1),
                    end,
                    LineContext {
                        start,
                        end,
                        index: line_number,
                    },
                    width,
                );
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
    if offset == paragraph.text.source().len() {
        return true;
    }
    let after_ordinal = cluster_index_at_or_after(paragraph, offset);
    let Some(before_ordinal) = after_ordinal.checked_sub(1) else {
        return true;
    };
    if after_ordinal >= paragraph.text.clusters().len() {
        return true;
    }
    // §3.6.3's cut answers to no character class, but it is available only where the
    // sign is a sign of the line: inside a structure that stacks its text off the line
    // there is no line boundary to cut at, and the boundary before such a structure is
    // an ordinary one Table 2 decides.
    if is_line_tab_sign(paragraph, after_ordinal) {
        return true;
    }

    let raw_before = class_of_cluster_with_style(paragraph, style, before_ordinal);
    let raw_after = class_of_cluster_with_style(paragraph, style, after_ordinal);
    let before_character =
        single_cluster_character(paragraph, &paragraph.text.clusters()[before_ordinal]);
    let after_character =
        single_cluster_character(paragraph, &paragraph.text.clusters()[after_ordinal]);

    // §C.3 states these four prohibitions are common to every convention level.
    if raw_before == crate::spec::OPENING_BRACKET
        || matches!(
            raw_after,
            crate::spec::CLOSING_BRACKET | crate::spec::FULL_STOP | crate::spec::COMMA
        )
    {
        return false;
    }

    if c_3_relaxes_boundary(
        style,
        raw_before,
        raw_after,
        before_character,
        after_character,
    ) {
        return true;
    }

    let before = reclassified_break_class(style, raw_before, before_character);
    let after = reclassified_break_class(style, raw_after, after_character);
    let Some(cell) = crate::spec::table_two_cell(before, after) else {
        return true;
    };
    if cell.prohibited {
        return false;
    }

    match (before, after) {
        (8, 8) => !inseparable_member_pair(before_character, after_character),
        (24, 27) => {
            style.grouped_numeral_before_western() == GroupedNumeralBeforeWestern::Breakable
        },
        (27, 13) => {
            let role = paragraph.text.clusters()[before_ordinal].role();
            role != Some(ClusterRole::QuantitySymbol)
                && !before_character.is_some_and(|character| character.is_ascii_digit())
        },
        _ => cell.levels & kinsoku_level_bit(style.kinsoku_level()) == 0,
    }
}

fn kinsoku_level_bit(level: KinsokuLevel) -> u8 {
    match level {
        KinsokuLevel::VeryLoose => 0b0001,
        KinsokuLevel::Loose => 0b0010,
        KinsokuLevel::Strict => 0b0100,
        KinsokuLevel::VeryStrict => 0b1000,
    }
}

fn reclassified_break_class(style: &Style, class: u8, character: Option<char>) -> u8 {
    if character == Some('々')
        && style.iteration_mark_at_line_head() != IterationMarkAtLineHead::Prohibited
        && style.kinsoku_level() != KinsokuLevel::VeryStrict
    {
        return 19;
    }
    if style.relaxation_mechanism() == RelaxationMechanism::Reclassify
        && style.kinsoku_level() != KinsokuLevel::VeryStrict
    {
        if class == 10 {
            return 16;
        }
        if class == 11 {
            return if character.is_some_and(crate::spec::is_hiragana) {
                15
            } else {
                16
            };
        }
    }
    class
}

fn c_3_relaxes_boundary(
    style: &Style,
    before: u8,
    after: u8,
    before_character: Option<char>,
    after_character: Option<char>,
) -> bool {
    let either_class = |classes: &[u8]| classes.contains(&before) || classes.contains(&after);
    let either_character =
        |character| before_character == Some(character) || after_character == Some(character);
    let iteration_relaxed = style.iteration_mark_at_line_head()
        != IterationMarkAtLineHead::Prohibited
        && either_character('々');
    let matrix_kana =
        style.relaxation_mechanism() == RelaxationMechanism::Matrix && either_class(&[10, 11]);

    match style.kinsoku_level() {
        KinsokuLevel::VeryLoose => {
            either_class(&[3, 4, 5, 9, 12, 13])
                || matrix_kana
                || cl_08_same_kind(before_character, after_character)
        },
        KinsokuLevel::Loose => {
            either_class(&[3])
                || either_character('・')
                || matches!(
                    (before_character, after_character),
                    (Some('…'), Some('…')) | (Some('‥'), Some('‥'))
                )
                || iteration_relaxed
                || matrix_kana
                || either_character('%')
                || either_character('％')
        },
        KinsokuLevel::Strict => iteration_relaxed || matrix_kana,
        KinsokuLevel::VeryStrict => false,
    }
}

fn inseparable_member_pair(before: Option<char>, after: Option<char>) -> bool {
    matches!(
        (before, after),
        (Some('—'), Some('—'))
            | (Some('…'), Some('…'))
            | (Some('‥'), Some('‥'))
            | (Some('〳' | '〴'), Some('〵'))
    )
}

fn cl_08_same_kind(before: Option<char>, after: Option<char>) -> bool {
    match (before, after) {
        (Some(before), Some(after)) if before == after => true,
        (Some(before), Some(after)) => "〳〴〵".contains(before) && "〳〴〵".contains(after),
        _ => false,
    }
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

fn place_attachments(paragraph: &Paragraph, style: &Style, line_index: usize, line: &mut Line) {
    let mut attachment_extent = 0;
    for (ordinal, construct) in paragraph.constructs.iter().enumerate() {
        if !ranges_overlap(&construct.range(), &line.range) {
            continue;
        }
        match construct.kind() {
            ConstructKind::Ruby(ruby) => {
                place_ruby_attachments(
                    paragraph,
                    style,
                    line_index,
                    line,
                    ordinal,
                    ruby,
                    &mut attachment_extent,
                );
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

fn place_ruby_attachments(
    paragraph: &Paragraph,
    style: &Style,
    line_index: usize,
    line: &mut Line,
    construct: usize,
    ruby: &Ruby,
    attachment_extent: &mut i32,
) {
    match ruby.kind() {
        RubyKind::Group => {
            *attachment_extent = (*attachment_extent).max(place_ruby_span(
                paragraph,
                style,
                line,
                construct,
                &RubySpan {
                    annotation: ruby.annotation(),
                    base_range: ruby.base(),
                    annotation_range: 0..ruby.annotation().source().len(),
                    distribution: Some(style.group_ruby_distribution()),
                },
            ));
        },
        RubyKind::Mono => {
            for run in ruby.runs() {
                if range_fits_line(&run.base(), line) {
                    *attachment_extent = (*attachment_extent).max(place_ruby_span(
                        paragraph,
                        style,
                        line,
                        construct,
                        &RubySpan {
                            annotation: ruby.annotation(),
                            base_range: run.base(),
                            annotation_range: run.annotation(),
                            distribution: None,
                        },
                    ));
                }
            }
        },
        RubyKind::Jukugo => {
            if style.jukugo_ruby_layout() == JukugoRubyLayout::Phonetic {
                if let Some(extent) =
                    place_phonetic_jukugo(paragraph, style, line_index, line, construct, ruby)
                {
                    *attachment_extent = (*attachment_extent).max(extent);
                    return;
                }
            }
            let per_base = ruby
                .runs()
                .iter()
                .all(|run| annotation_cluster_count(ruby.annotation(), &run.annotation()) <= 2);
            if !per_base && range_fits_line(&ruby.base(), line) {
                *attachment_extent = (*attachment_extent).max(place_ruby_span(
                    paragraph,
                    style,
                    line,
                    construct,
                    &RubySpan {
                        annotation: ruby.annotation(),
                        base_range: ruby.base(),
                        annotation_range: 0..ruby.annotation().source().len(),
                        distribution: Some(GroupRubyDistribution::Jis),
                    },
                ));
            } else {
                for run in ruby.runs() {
                    if range_fits_line(&run.base(), line) {
                        *attachment_extent = (*attachment_extent).max(place_ruby_span(
                            paragraph,
                            style,
                            line,
                            construct,
                            &RubySpan {
                                annotation: ruby.annotation(),
                                base_range: run.base(),
                                annotation_range: run.annotation(),
                                distribution: None,
                            },
                        ));
                    }
                }
            }
        },
    }
}

fn place_phonetic_jukugo(
    paragraph: &Paragraph,
    style: &Style,
    line_index: usize,
    line: &mut Line,
    construct: usize,
    ruby: &Ruby,
) -> Option<i32> {
    let line_start = cluster_index_at_or_after(paragraph, line.range.start);
    let line_end = cluster_index_at_or_after(paragraph, line.range.end);
    let plan = phonetic_jukugo_plan(paragraph, style, ruby, line_start, line_end, line_index)?;
    let first = plan.runs.first()?;
    let actual_start = line.clusters.iter().find_map(|placement| {
        (placement.origin == PlacementOrigin::Cluster(first.base.start))
            .then_some(i64::from(placement.inline))
    })?;
    let offset = actual_start.saturating_sub(first.base_start);
    let mut attachment_extent = 0;
    for run in &plan.runs {
        let mut inline = offset.saturating_add(run.annotation_start);
        for cluster in ruby.annotation().clusters().iter().filter(|cluster| {
            let cluster = cluster.range();
            run.annotation.start <= cluster.start && cluster.end <= run.annotation.end
        }) {
            let size = cluster.size_override().unwrap_or(ruby.annotation().size());
            line.attachments.push(Attachment {
                construct,
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
    Some(attachment_extent)
}

fn annotation_cluster_count(annotation: &crate::ShapedText, range: &Range<usize>) -> usize {
    annotation
        .clusters()
        .iter()
        .filter(|cluster| {
            let cluster = cluster.range();
            range.start <= cluster.start && cluster.end <= range.end
        })
        .count()
}

fn range_fits_line(range: &Range<usize>, line: &Line) -> bool {
    line.range.start <= range.start && range.end <= line.range.end
}

struct RubySpan<'a> {
    annotation: &'a crate::ShapedText,
    base_range: Range<usize>,
    annotation_range: Range<usize>,
    distribution: Option<GroupRubyDistribution>,
}

fn place_ruby_span(
    paragraph: &Paragraph,
    style: &Style,
    line: &mut Line,
    construct: usize,
    span: &RubySpan<'_>,
) -> i32 {
    let Some((base_start, base_end)) = bounds_for_ruby_range(paragraph, line, &span.base_range)
    else {
        return 0;
    };
    let annotation_width = span
        .annotation
        .clusters()
        .iter()
        .filter(|cluster| {
            let cluster = cluster.range();
            span.annotation_range.start <= cluster.start && cluster.end <= span.annotation_range.end
        })
        .fold(0_i64, |sum, cluster| {
            sum.saturating_add(i64::from(cluster.advance()))
        });
    let base_width = i64::from(base_end).saturating_sub(i64::from(base_start));
    let annotation_count = span
        .annotation
        .clusters()
        .iter()
        .filter(|cluster| {
            let cluster = cluster.range();
            span.annotation_range.start <= cluster.start && cluster.end <= span.annotation_range.end
        })
        .count();
    let surplus = base_width.saturating_sub(annotation_width).max(0);
    let mut gaps = Vec::new();
    let base_plan = span.distribution.and_then(|distribution| {
        base_distribution_plan(
            paragraph,
            style,
            &span.base_range,
            span.annotation,
            &span.annotation_range,
            distribution,
        )
    });
    let mut inline = if let Some(plan) = base_plan {
        i64::from(base_start).saturating_sub(i64::from(plan.leading))
    } else if annotation_width > base_width {
        i64::from(base_start).saturating_add((base_width.saturating_sub(annotation_width)) / 2)
    } else if let Some(distribution) = span.distribution {
        let weights = match distribution {
            GroupRubyDistribution::Jis => {
                let mut weights = Vec::with_capacity(annotation_count.saturating_add(1));
                weights.push(1);
                weights.extend((1..annotation_count).map(|_| 2));
                weights.push(1);
                weights
            },
            GroupRubyDistribution::Flush => vec![1; annotation_count.saturating_sub(1)],
        };
        let shares = proportional_shares(surplus, &weights, style.remainder());
        match distribution {
            GroupRubyDistribution::Jis => {
                gaps.extend(
                    shares
                        .iter()
                        .skip(1)
                        .take(annotation_count.saturating_sub(1)),
                );
                i64::from(base_start)
                    .saturating_add(i64::from(shares.first().copied().unwrap_or(0)))
            },
            GroupRubyDistribution::Flush => {
                gaps.extend(shares);
                i64::from(base_start)
            },
        }
    } else {
        match style.ruby_alignment() {
            RubyAlignment::Nakatsuki => i64::from(base_start).saturating_add(surplus / 2),
            RubyAlignment::Katatsuki => i64::from(base_start),
        }
    };
    let mut attachment_extent = 0;
    for (index, cluster) in span
        .annotation
        .clusters()
        .iter()
        .filter(|cluster| {
            let cluster = cluster.range();
            span.annotation_range.start <= cluster.start && cluster.end <= span.annotation_range.end
        })
        .enumerate()
    {
        let size = cluster.size_override().unwrap_or(span.annotation.size());
        line.attachments.push(Attachment {
            construct,
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
        inline = inline.saturating_add(i64::from(gaps.get(index).copied().unwrap_or(0)));
    }
    attachment_extent
}

fn proportional_shares(total: i64, weights: &[i32], remainder: Remainder) -> Vec<i32> {
    if total <= 0 || weights.is_empty() {
        return vec![0; weights.len()];
    }
    let weight_sum = weights.iter().fold(0_i64, |sum, weight| {
        sum.saturating_add(i64::from((*weight).max(0)))
    });
    if weight_sum == 0 {
        return vec![0; weights.len()];
    }
    let mut shares: Vec<_> = weights
        .iter()
        .map(|weight| {
            total
                .saturating_mul(i64::from((*weight).max(0)))
                .checked_div(weight_sum)
                .unwrap_or(0)
        })
        .collect();
    let mut left = total.saturating_sub(shares.iter().copied().sum::<i64>());
    while left > 0 {
        match remainder {
            Remainder::Leading => {
                for share in &mut shares {
                    if left == 0 {
                        break;
                    }
                    *share = share.saturating_add(1);
                    left = left.saturating_sub(1);
                }
            },
            Remainder::Trailing => {
                for share in shares.iter_mut().rev() {
                    if left == 0 {
                        break;
                    }
                    *share = share.saturating_add(1);
                    left = left.saturating_sub(1);
                }
            },
        }
    }
    shares.into_iter().map(clamp_i32).collect()
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

fn bounds_for_ruby_range(
    paragraph: &Paragraph,
    line: &Line,
    range: &Range<usize>,
) -> Option<(i32, i32)> {
    let mut matching = line
        .clusters
        .iter()
        .filter(|placement| ranges_overlap(&placement.range, range));
    let first = matching.next()?;
    let mut end = ruby_base_inline_end(paragraph, first);
    for placement in matching {
        end = end.max(ruby_base_inline_end(paragraph, placement));
    }
    Some((first.inline, end))
}

fn ruby_base_inline_end(paragraph: &Paragraph, placement: &ClusterPlacement) -> i32 {
    let advance = match placement.origin {
        PlacementOrigin::Cluster(ordinal) => effective_cluster_body_advance(paragraph, ordinal),
        PlacementOrigin::Construct(_) => placement.advance,
    };
    placement.inline.saturating_add(advance)
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

    use crate::construct::Construct;
    use crate::model::{Cluster, Frame, ShapedText, Size, WritingMode};
    use crate::paragraph::{Break, Paragraph, Widow};
    use crate::style::Style;

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

    #[test]
    fn distinct_ornamented_complexes_lower_to_table_six_stage_three() {
        let paragraph = Paragraph::builder(text("日本"), 2_000)
            .constructs([
                Construct::script(0..3, text("注")),
                Construct::script(3..6, text("記")),
            ])
            .build()
            .expect("valid ornamented paragraph");
        assert_eq!(
            super::boundary_expansion_site(&paragraph, &Style::default(), 0),
            super::ExpansionSite::Site {
                weight: 1_000,
                bounded: Some((250, 3)),
                residual: false,
            }
        );
    }
}
