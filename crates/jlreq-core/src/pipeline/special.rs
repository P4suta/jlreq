// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

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
    debug_assert!(logical_end <= lane.end);
    let boundary = if logical_end == lane.end {
        0
    } else {
        boundary_space_after(paragraph, ordinal)
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
    let boundary = cluster_index_at_or_after(paragraph, offset);
    let Some(cluster) = boundary.checked_sub(1) else {
        return 0;
    };
    let mut penalty = 0_i64;
    paragraph.visit_constructs_containing(cluster, |_, construct| {
        let ConstructKind::Warichu(range) = construct.kind() else {
            return;
        };
        if !(range.start < offset && offset < range.end) {
            return;
        }
        let start = cluster_index_at_or_after(paragraph, range.start);
        let split = cluster_index_at_or_after(paragraph, offset);
        let end = cluster_index_at_or_after(paragraph, range.end);
        let before = split.saturating_sub(start);
        let after = end.saturating_sub(split);
        penalty = penalty.saturating_add(
            i64::try_from(before.abs_diff(after))
                .unwrap_or(i64::MAX)
                .saturating_mul(1_000_000),
        );
    });
    penalty
}

fn formula_break_penalty(paragraph: &Paragraph, offset: usize) -> i64 {
    let boundary = cluster_index_at_or_after(paragraph, offset);
    let is_independent_formula = boundary.checked_sub(1).is_some_and(|ordinal| {
        paragraph
            .find_construct_containing(ordinal, |construct| {
                matches!(
                    construct.kind(),
                    ConstructKind::Formula(range)
                        if range.start == 0
                            && range.end == paragraph.text.source().len()
                            && range.start < offset
                            && offset < range.end
                )
            })
            .is_some()
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

