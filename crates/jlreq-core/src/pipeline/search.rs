// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

fn check_limit(
    resource: CompositionResource,
    limit: usize,
    observed: usize,
) -> Result<(), ComposeError> {
    if observed > limit {
        Err(ComposeError::new(resource, limit, observed))
    } else {
        Ok(())
    }
}

fn non_negative_cost(cost: i64) -> u128 {
    u128::try_from(cost).unwrap_or(0)
}

fn search_candidate_precedes(cost: u128, start: usize, current: Node) -> bool {
    (cost, start) < (current.cost, current.previous)
}

fn search_lower_bound_exceeds(
    minimum_width: i64,
    available: i64,
    is_last: bool,
    preference: AdjustmentPreference,
    best_cost: u128,
) -> bool {
    match minimum_width.cmp(&available) {
        core::cmp::Ordering::Greater => {
            non_negative_cost(line_badness(
                available.saturating_sub(minimum_width),
                is_last,
                preference,
            )) > best_cost
        },
        core::cmp::Ordering::Less | core::cmp::Ordering::Equal => false,
    }
}

fn line_should_justify(
    alignment: Alignment,
    is_last: bool,
    remaining: i64,
    cluster_count: usize,
) -> bool {
    alignment == Alignment::Justify && !is_last && remaining.is_positive() && cluster_count > 1
}

fn line_adjustment_need(remaining: i64, justify: bool) -> i64 {
    if remaining.is_negative() || justify {
        remaining
    } else {
        0
    }
}

fn fast_measure_line(
    prepared: &PreparedParagraph,
    paragraph: &Paragraph,
    style: &Style,
    start: usize,
    end: usize,
    line_number: usize,
) -> i64 {
    if start >= end {
        return i64::from(line_head_indent(paragraph, style, start, line_number));
    }
    let mut width = range_sum(&prepared.natural_prefix, start, end);
    let last = end.saturating_sub(1);
    let last_natural = range_sum(&prepared.natural_prefix, last, end);
    if is_western_word_space(paragraph, last) {
        width = width.saturating_sub(last_natural);
    } else {
        width = width
            .saturating_sub(last_natural)
            .saturating_add(i64::from(effective_cluster_body_advance(paragraph, last)))
            .saturating_add(i64::from(line_end_space_after(paragraph, style, last)));
    }
    if start != last && is_western_word_space(paragraph, start) {
        width = width.saturating_sub(range_sum(
            &prepared.natural_prefix,
            start,
            start.saturating_add(1),
        ));
    }
    width.saturating_add(i64::from(line_head_indent(
        paragraph,
        style,
        start,
        line_number,
    )))
}

fn fast_width_after_available_reduction(
    prepared: &PreparedParagraph,
    paragraph: &Paragraph,
    style: &Style,
    start: usize,
    end: usize,
    width: i64,
    available: i64,
) -> i64 {
    let need = width.saturating_sub(available);
    if need <= 0 || start >= end {
        return width;
    }
    let last = end.saturating_sub(1);
    let internal_capacity = range_sum(&prepared.reduction_prefix, start, last);
    let line_end_capacity = prepared.line_end_reduction.get(last).copied().unwrap_or(0);
    let capacity = internal_capacity.saturating_add(line_end_capacity);
    let reduced = width.saturating_sub(need.min(capacity));
    reduced.saturating_sub(hanging_amount(paragraph, style, end, reduced, available))
}

fn fast_minimum_width(prepared: &PreparedParagraph, start: usize, end: usize) -> i64 {
    let Some(last) = end.checked_sub(1) else {
        return 0;
    };
    range_sum(&prepared.minimum_prefix, start, last)
}

fn range_sum(prefix: &[i64], start: usize, end: usize) -> i64 {
    prefix
        .get(end)
        .copied()
        .unwrap_or(i64::MAX)
        .saturating_sub(prefix.get(start).copied().unwrap_or(0))
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
    paragraph.text.clusters().get(ordinal)?;
    let (_, construct) = paragraph.find_construct_containing(ordinal, |construct| {
        matches!(construct.kind(), ConstructKind::TateChuYoko(_))
    })?;
    let ConstructKind::TateChuYoko(range) = construct.kind() else {
        return None;
    };
    Some(
        cluster_index_at_or_after(paragraph, range.start)
            ..cluster_index_at_or_after(paragraph, range.end),
    )
}

fn warichu_cluster_range(paragraph: &Paragraph, ordinal: usize) -> Option<Range<usize>> {
    paragraph.text.clusters().get(ordinal)?;
    let (_, construct) = paragraph.find_construct_containing(ordinal, |construct| {
        matches!(construct.kind(), ConstructKind::Warichu(_))
    })?;
    let ConstructKind::Warichu(range) = construct.kind() else {
        return None;
    };
    Some(
        cluster_index_at_or_after(paragraph, range.start)
            ..cluster_index_at_or_after(paragraph, range.end),
    )
}

fn furawake_cluster_range(
    paragraph: &Paragraph,
    ordinal: usize,
) -> Option<(Range<usize>, u16, i32)> {
    paragraph.text.clusters().get(ordinal)?;
    let (_, construct) = paragraph.find_construct_containing(ordinal, |construct| {
        matches!(construct.kind(), ConstructKind::Furawake { .. })
    })?;
    let ConstructKind::Furawake {
        range,
        columns,
        line_gap,
    } = construct.kind()
    else {
        return None;
    };
    Some((
        cluster_index_at_or_after(paragraph, range.start)
            ..cluster_index_at_or_after(paragraph, range.end),
        *columns,
        *line_gap,
    ))
}

fn jidori_cluster_range(paragraph: &Paragraph, ordinal: usize) -> Option<(Range<usize>, u16)> {
    paragraph.text.clusters().get(ordinal)?;
    let (_, construct) = paragraph.find_construct_containing(ordinal, |construct| {
        matches!(construct.kind(), ConstructKind::Jidori { .. })
    })?;
    let ConstructKind::Jidori { range, cells } = construct.kind() else {
        return None;
    };
    Some((
        cluster_index_at_or_after(paragraph, range.start)
            ..cluster_index_at_or_after(paragraph, range.end),
        *cells,
    ))
}

fn is_internal_jidori_boundary(paragraph: &Paragraph, ordinal: usize) -> bool {
    let Some(cluster) = paragraph.text.clusters().get(ordinal) else {
        return false;
    };
    let boundary = cluster.range().end;
    paragraph
        .find_construct_containing(ordinal, |construct| {
            matches!(construct.kind(), ConstructKind::Jidori { range, .. }
                if range.start < boundary && boundary < range.end)
        })
        .is_some()
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
    paragraph
        .find_construct_containing(ordinal, |construct| {
            matches!(
                construct.kind(),
                ConstructKind::Warichu(range) | ConstructKind::Furawake { range, .. }
                    if range.start < boundary && boundary < range.end
            )
        })
        .is_some()
}

fn formula_cluster_range(paragraph: &Paragraph, ordinal: usize) -> Option<Range<usize>> {
    paragraph.text.clusters().get(ordinal)?;
    let (_, construct) = paragraph.find_construct_containing(ordinal, |construct| {
        matches!(construct.kind(), ConstructKind::Formula(_))
    })?;
    let ConstructKind::Formula(range) = construct.kind() else {
        return None;
    };
    Some(
        cluster_index_at_or_after(paragraph, range.start)
            ..cluster_index_at_or_after(paragraph, range.end),
    )
}

fn is_internal_furawake_offset(paragraph: &Paragraph, offset: usize) -> bool {
    let boundary = cluster_index_at_or_after(paragraph, offset);
    boundary.checked_sub(1).is_some_and(|ordinal| {
        paragraph
            .find_construct_containing(ordinal, |construct| {
                matches!(construct.kind(), ConstructKind::Furawake { range, .. }
                    if range.start < offset && offset < range.end)
            })
            .is_some()
    })
}

