// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#[derive(Debug, Clone, Copy)]
struct Candidate {
    offset: usize,
    mandatory: bool,
    discretionary: bool,
}

#[derive(Debug, Clone, Copy)]
struct Node {
    cost: u128,
    previous: usize,
    line_count: usize,
}

#[derive(Debug, Default)]
struct PreparedParagraph {
    candidate_ordinals: Vec<usize>,
    legal_candidates: Vec<bool>,
    natural_prefix: Vec<i64>,
    minimum_prefix: Vec<i64>,
    reduction_prefix: Vec<i64>,
    line_end_reduction: Vec<i64>,
    regular: bool,
    fast_measure: bool,
}

impl PreparedParagraph {
    const fn new() -> Self {
        Self {
            candidate_ordinals: Vec::new(),
            legal_candidates: Vec::new(),
            natural_prefix: Vec::new(),
            minimum_prefix: Vec::new(),
            reduction_prefix: Vec::new(),
            line_end_reduction: Vec::new(),
            regular: false,
            fast_measure: false,
        }
    }

    fn clear(&mut self) {
        self.candidate_ordinals.clear();
        self.legal_candidates.clear();
        self.natural_prefix.clear();
        self.minimum_prefix.clear();
        self.reduction_prefix.clear();
        self.line_end_reduction.clear();
        self.regular = false;
        self.fast_measure = false;
    }
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

#[derive(Debug, Default)]
struct DistributionScratch {
    assigned: Vec<i64>,
    capacities: Vec<i64>,
    extra: Vec<i64>,
}

impl DistributionScratch {
    const fn new() -> Self {
        Self {
            assigned: Vec::new(),
            capacities: Vec::new(),
            extra: Vec::new(),
        }
    }

    fn clear(&mut self) {
        self.assigned.clear();
        self.capacities.clear();
        self.extra.clear();
    }
}

#[derive(Debug, Default)]
struct LineScratch {
    reduction_sites: Vec<ReductionSite>,
    stage_reductions: Vec<ReductionSite>,
    expansion_sites: Vec<ExpansionSite>,
    distribution_sites: Vec<(usize, i32, Option<i32>)>,
    construct_ordinals: Vec<usize>,
    distribution: DistributionScratch,
}

impl LineScratch {
    const fn new() -> Self {
        Self {
            reduction_sites: Vec::new(),
            stage_reductions: Vec::new(),
            expansion_sites: Vec::new(),
            distribution_sites: Vec::new(),
            construct_ordinals: Vec::new(),
            distribution: DistributionScratch::new(),
        }
    }

    fn clear(&mut self) {
        self.reduction_sites.clear();
        self.stage_reductions.clear();
        self.expansion_sites.clear();
        self.distribution_sites.clear();
        self.construct_ordinals.clear();
        self.distribution.clear();
    }
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
    limits: CompositionLimits,
    transitions: usize,
    candidates: Vec<Candidate>,
    nodes: Vec<Node>,
    chosen: Vec<usize>,
    line_advances: Vec<i32>,
    line_adjustments: Vec<i32>,
    line_scratch: LineScratch,
    prepared: PreparedParagraph,
}

impl Composer {
    /// Build an empty reusable composer.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            limits: CompositionLimits::DEFAULT,
            transitions: 0,
            candidates: Vec::new(),
            nodes: Vec::new(),
            chosen: Vec::new(),
            line_advances: Vec::new(),
            line_adjustments: Vec::new(),
            line_scratch: LineScratch::new(),
            prepared: PreparedParagraph::new(),
        }
    }

    /// Build a reusable composer with explicit deterministic resource limits.
    #[must_use]
    pub const fn with_limits(limits: CompositionLimits) -> Self {
        Self {
            limits,
            transitions: 0,
            candidates: Vec::new(),
            nodes: Vec::new(),
            chosen: Vec::new(),
            line_advances: Vec::new(),
            line_adjustments: Vec::new(),
            line_scratch: LineScratch::new(),
            prepared: PreparedParagraph::new(),
        }
    }

    /// The limits used by subsequent composition calls.
    #[must_use]
    pub const fn limits(&self) -> CompositionLimits {
        self.limits
    }

    /// Replace the limits used by subsequent composition calls.
    pub const fn set_limits(&mut self, limits: CompositionLimits) {
        self.limits = limits;
    }

    /// Normalize, choose breaks globally, and place one validated paragraph.
    pub fn compose(
        &mut self,
        paragraph: &Paragraph,
        style: &Style,
    ) -> Result<Layout, ComposeError> {
        self.reset_for_call();
        if paragraph.text.clusters().is_empty() {
            return Ok(Layout::default());
        }
        self.check_static_limits(paragraph)?;
        self.prepare_candidates(paragraph);
        self.prepare_indexes(paragraph, style);
        self.search(paragraph, style)?;
        self.backtrack();
        Ok(self.place(paragraph, style))
    }

    fn reset_for_call(&mut self) {
        self.transitions = 0;
        self.candidates.clear();
        self.nodes.clear();
        self.chosen.clear();
        self.line_advances.clear();
        self.line_adjustments.clear();
        self.line_scratch.clear();
        self.prepared.clear();
    }

    fn check_static_limits(&self, paragraph: &Paragraph) -> Result<(), ComposeError> {
        check_limit(
            CompositionResource::Clusters,
            self.limits.max_clusters(),
            paragraph.text.clusters().len(),
        )?;
        check_limit(
            CompositionResource::BreakCandidates,
            self.limits.max_break_candidates(),
            paragraph.breaks.len(),
        )?;
        check_limit(
            CompositionResource::Constructs,
            self.limits.max_constructs(),
            paragraph.constructs.len(),
        )?;
        check_limit(
            CompositionResource::TabStops,
            self.limits.max_tab_stops(),
            paragraph.tab_stops.len(),
        )
    }

    fn charge_transitions(&mut self, amount: usize) -> Result<(), ComposeError> {
        let observed = self.transitions.saturating_add(amount);
        if observed > self.limits.max_search_transitions() {
            return Err(ComposeError::new(
                CompositionResource::SearchTransitions,
                self.limits.max_search_transitions(),
                observed,
            ));
        }
        self.transitions = observed;
        Ok(())
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

    fn prepare_indexes(&mut self, paragraph: &Paragraph, style: &Style) {
        self.prepared.candidate_ordinals.extend(
            self.candidates
                .iter()
                .map(|candidate| cluster_index_at_or_after(paragraph, candidate.offset)),
        );
        self.prepared
            .legal_candidates
            .extend(self.candidates.iter().map(|candidate| {
                candidate.mandatory || break_is_legal(paragraph, style, candidate.offset)
            }));

        self.prepared.regular = paragraph.constructs.is_empty()
            && !paragraph
                .line_tabs
                .iter()
                .copied()
                .any(core::convert::identity);
        self.prepared.fast_measure = !paragraph
            .line_tabs
            .iter()
            .copied()
            .any(core::convert::identity)
            && paragraph.constructs.iter().all(|construct| {
                matches!(
                    construct.kind(),
                    ConstructKind::TateChuYoko(_)
                        | ConstructKind::Emphasis { .. }
                        | ConstructKind::ReferenceMark { .. }
                        | ConstructKind::Script { .. }
                        | ConstructKind::Formula(_)
                )
            });
        if !self.prepared.fast_measure {
            return;
        }

        let cluster_count = paragraph.text.clusters().len();
        self.prepared
            .natural_prefix
            .reserve(cluster_count.saturating_add(1));
        self.prepared
            .minimum_prefix
            .reserve(cluster_count.saturating_add(1));
        self.prepared
            .reduction_prefix
            .reserve(cluster_count.saturating_add(1));
        self.prepared.line_end_reduction.reserve(cluster_count);
        self.prepared.natural_prefix.push(0);
        self.prepared.minimum_prefix.push(0);
        self.prepared.reduction_prefix.push(0);

        for ordinal in 0..cluster_count {
            let natural = i64::from(effective_cluster_advance(paragraph, style, ordinal));
            let previous_natural = self.prepared.natural_prefix.last().copied().unwrap_or(0);
            self.prepared
                .natural_prefix
                .push(previous_natural.saturating_add(natural));

            let minimum = if is_western_word_space(paragraph, ordinal) {
                0
            } else {
                i64::from(paragraph.text.clusters()[ordinal].advance())
            };
            let previous_minimum = self.prepared.minimum_prefix.last().copied().unwrap_or(0);
            self.prepared
                .minimum_prefix
                .push(previous_minimum.saturating_add(minimum));

            self.line_scratch.reduction_sites.clear();
            if paragraph
                .text
                .clusters()
                .get(ordinal.saturating_add(1))
                .is_some()
            {
                if is_western_word_space(paragraph, ordinal) {
                    let cluster = &paragraph.text.clusters()[ordinal];
                    let capacity = effective_cluster_body_advance(paragraph, ordinal)
                        .saturating_sub(quarter_inline_size(paragraph, cluster))
                        .max(0);
                    push_reduction_site(
                        &mut self.line_scratch.reduction_sites,
                        0,
                        cluster
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
                    0,
                    &mut self.line_scratch.reduction_sites,
                );
            }
            let capacity = self
                .line_scratch
                .reduction_sites
                .iter()
                .fold(0_i64, |sum, site| {
                    sum.saturating_add(i64::from(site.capacity))
                });
            let previous_capacity = self.prepared.reduction_prefix.last().copied().unwrap_or(0);
            self.prepared
                .reduction_prefix
                .push(previous_capacity.saturating_add(capacity));

            self.line_scratch.reduction_sites.clear();
            append_line_end_reduction_site(
                paragraph,
                style,
                ordinal,
                0,
                &mut self.line_scratch.reduction_sites,
            );
            self.prepared.line_end_reduction.push(
                self.line_scratch
                    .reduction_sites
                    .iter()
                    .fold(0_i64, |sum, site| {
                        sum.saturating_add(i64::from(site.capacity))
                    }),
            );
        }
    }

    fn search(&mut self, paragraph: &Paragraph, style: &Style) -> Result<(), ComposeError> {
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

        let mut mandatory_partition_start = 0_usize;
        for end in 1..self.candidates.len() {
            let candidate = self.candidates[end];
            if !self.prepared.legal_candidates[end] {
                continue;
            }
            for start in (mandatory_partition_start..end).rev() {
                self.charge_transitions(1)?;
                if self.nodes[start].cost == INFINITE_COST {
                    continue;
                }
                let line_number = self.nodes[start].line_count;
                let start_ordinal = self.prepared.candidate_ordinals[start];
                let end_ordinal = self.prepared.candidate_ordinals[end];
                if !self.prepared.regular {
                    self.charge_transitions(
                        end_ordinal
                            .saturating_sub(start_ordinal)
                            .saturating_add(paragraph.constructs.len()),
                    )?;
                }
                let measured_width = if self.prepared.fast_measure {
                    fast_measure_line(
                        &self.prepared,
                        paragraph,
                        style,
                        start_ordinal,
                        end_ordinal,
                        line_number,
                    )
                } else {
                    measure_line(
                        paragraph,
                        style,
                        self.candidates[start].offset,
                        candidate.offset,
                        line_number,
                    )
                };
                let available = i64::from(paragraph.line_extent);
                let width = if self.prepared.fast_measure {
                    fast_width_after_available_reduction(
                        &self.prepared,
                        paragraph,
                        style,
                        start_ordinal,
                        end_ordinal,
                        measured_width,
                        available,
                    )
                } else {
                    width_after_available_reduction_with_scratch(
                        paragraph,
                        style,
                        self.candidates[start].offset,
                        candidate.offset,
                        measured_width,
                        available,
                        &mut self.line_scratch.reduction_sites,
                    )
                };
                let delta = available.saturating_sub(width);
                let is_last = end.saturating_add(1) == self.candidates.len();
                let mut edge_cost =
                    non_negative_cost(line_badness(delta, is_last, style.adjustment_preference()));
                if candidate.discretionary {
                    edge_cost = edge_cost.saturating_add(100_000);
                }
                edge_cost = edge_cost.saturating_add(non_negative_cost(warichu_break_penalty(
                    paragraph,
                    candidate.offset,
                )));
                edge_cost = edge_cost.saturating_add(non_negative_cost(formula_break_penalty(
                    paragraph,
                    candidate.offset,
                )));
                if is_last {
                    edge_cost = edge_cost.saturating_add(non_negative_cost(widow_penalty(
                        paragraph,
                        self.candidates[start].offset,
                        candidate.offset,
                    )));
                }
                let cost = edge_cost.saturating_add(self.nodes[start].cost);
                if search_candidate_precedes(cost, start, self.nodes[end]) {
                    self.nodes[end] = Node {
                        cost,
                        previous: start,
                        line_count: line_number.saturating_add(1),
                    };
                }

                if self.prepared.regular {
                    let minimum_width =
                        fast_minimum_width(&self.prepared, start_ordinal, end_ordinal);
                    if search_lower_bound_exceeds(
                        minimum_width,
                        available,
                        is_last,
                        style.adjustment_preference(),
                        self.nodes[end].cost,
                    ) {
                        break;
                    }
                }
            }
            if candidate.mandatory {
                mandatory_partition_start = end;
            }
        }
        Ok(())
    }

    fn backtrack(&mut self) {
        self.chosen.clear();
        let mut cursor = self.nodes.len().saturating_sub(1);
        self.chosen.push(cursor);
        while cursor != 0 {
            cursor = self.nodes[cursor].previous;
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
        let justify = line_should_justify(paragraph.alignment, is_last, remaining, clusters.len());
        prepare_line_adjustments_with_scratch(
            paragraph,
            style,
            start_cluster,
            end_cluster,
            line_adjustment_need(remaining, justify),
            &mut self.line_adjustments,
            &mut self.line_scratch,
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
        place_attachments(
            paragraph,
            style,
            line_index,
            start_cluster,
            end_cluster,
            &mut line,
            &mut self.line_scratch.construct_ordinals,
        );
        line
    }
}

