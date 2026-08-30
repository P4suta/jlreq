// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

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
        let residual =
            crate::generated::table6::cell(26, after_class).is_some_and(|cell| cell.residual);
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
        .find_construct_containing(before, |construct| match construct.kind() {
            ConstructKind::TateChuYoko(range)
            | ConstructKind::Warichu(range)
            | ConstructKind::Formula(range)
            | ConstructKind::Furawake { range, .. }
            | ConstructKind::Jidori { range, .. }
            | ConstructKind::Script { range, .. } => range.start < boundary && boundary < range.end,
            ConstructKind::Ruby(_)
            | ConstructKind::Emphasis { .. }
            | ConstructKind::ReferenceMark { .. } => false,
        })
        .is_some()
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
    let Some(cell) = crate::generated::table6::cell(before_class, after_class) else {
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
    if cap == 0 {
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
            weight.saturating_add(2).checked_div(3).unwrap_or(0)
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
    let (construct, candidate) = paragraph.find_construct_containing(ordinal, |candidate| {
        matches!(
            candidate.kind(),
            ConstructKind::Script { .. } | ConstructKind::Ruby(_)
        ) || (paragraph.writing_mode == WritingMode::VerticalRl
            && matches!(candidate.kind(), ConstructKind::TateChuYoko(_)))
    })?;
    match candidate.kind() {
        ConstructKind::Script { .. } => Some(ComplexIdentity {
            kind: ComplexKind::Ornamented,
            construct,
            member: 0,
        }),
        ConstructKind::Ruby(ruby) => {
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
        ConstructKind::TateChuYoko(_) => Some(ComplexIdentity {
            kind: ComplexKind::TateChuYoko,
            construct,
            member: 0,
        }),
        _ => None,
    }
}

fn prepare_line_adjustments_with_scratch(
    paragraph: &Paragraph,
    style: &Style,
    line_start: usize,
    line_end: usize,
    need: i64,
    adjustments: &mut Vec<i32>,
    scratch: &mut LineScratch,
) {
    adjustments.clear();
    adjustments.resize(line_end.saturating_sub(line_start), 0);
    match need.cmp(&0) {
        core::cmp::Ordering::Less => {
            prepare_line_reductions_with_scratch(
                paragraph,
                style,
                line_start,
                line_end,
                need.saturating_abs(),
                adjustments,
                scratch,
            );
            return;
        },
        core::cmp::Ordering::Equal => return,
        core::cmp::Ordering::Greater => {},
    }

    scratch.expansion_sites.clear();
    scratch
        .expansion_sites
        .extend((line_start..line_end.saturating_sub(1)).map(|before| {
            boundary_expansion_site_on_line(paragraph, style, before, line_start, line_end)
        }));
    let mut remaining = need;
    for stage in 1_u8..=3 {
        if remaining == 0 {
            return;
        }
        scratch.distribution_sites.clear();
        scratch
            .distribution_sites
            .extend(scratch.expansion_sites.iter().enumerate().filter_map(
                |(index, site)| match site {
                    ExpansionSite::Site {
                        weight,
                        bounded: Some((cap, site_stage)),
                        ..
                    } if *site_stage == stage => Some((index, *weight, Some(*cap))),
                    _ => None,
                },
            ));
        let capacity = scratch
            .distribution_sites
            .iter()
            .fold(0_i64, |sum, (_, _, cap)| {
                sum.saturating_add(i64::from(cap.unwrap_or(0)))
            });
        let take = remaining.min(capacity);
        distribute_adjustment_with_scratch(
            take,
            &scratch.distribution_sites,
            style.remainder(),
            adjustments,
            &mut scratch.distribution,
        );
        remaining = remaining.saturating_sub(take);
    }

    if remaining == 0 {
        return;
    }
    scratch.distribution_sites.clear();
    scratch
        .distribution_sites
        .extend(scratch.expansion_sites.iter().enumerate().filter_map(
            |(index, site)| match site {
                ExpansionSite::Site {
                    weight,
                    bounded,
                    residual,
                } if *residual || bounded.is_some_and(|(_, stage)| (2..=3).contains(&stage)) => {
                    Some((index, *weight, None))
                },
                ExpansionSite::None | ExpansionSite::Site { .. } => None,
            },
        ));
    distribute_adjustment_with_scratch(
        remaining,
        &scratch.distribution_sites,
        style.remainder(),
        adjustments,
        &mut scratch.distribution,
    );
}

#[cfg(test)]
fn prepare_line_adjustments(
    paragraph: &Paragraph,
    style: &Style,
    line_start: usize,
    line_end: usize,
    need: i64,
    adjustments: &mut Vec<i32>,
) {
    let mut scratch = LineScratch::new();
    prepare_line_adjustments_with_scratch(
        paragraph,
        style,
        line_start,
        line_end,
        need,
        adjustments,
        &mut scratch,
    );
}

fn prepare_line_reductions_with_scratch(
    paragraph: &Paragraph,
    style: &Style,
    line_start: usize,
    line_end: usize,
    mut need: i64,
    adjustments: &mut [i32],
    scratch: &mut LineScratch,
) {
    collect_reduction_sites(
        paragraph,
        style,
        line_start,
        line_end,
        &mut scratch.reduction_sites,
    );
    for stage in 1_u8..=6 {
        if need <= 0 {
            break;
        }
        for site in scratch
            .reduction_sites
            .iter()
            .copied()
            .filter(|site| site.stage == stage && site.discrete)
        {
            if need <= 0 {
                break;
            }
            apply_reduction(site.boundary, i64::from(site.capacity), adjustments);
            need = need.saturating_sub(i64::from(site.capacity));
        }

        if need <= 0 {
            break;
        }
        scratch.stage_reductions.clear();
        scratch.stage_reductions.extend(
            scratch
                .reduction_sites
                .iter()
                .copied()
                .filter(|site| site.stage == stage && !site.discrete),
        );
        let capacity = scratch.stage_reductions.iter().fold(0_i64, |sum, site| {
            sum.saturating_add(i64::from(site.capacity))
        });
        let take = need.min(capacity);
        distribute_reduction_with_scratch(
            take,
            &scratch.stage_reductions,
            style.remainder(),
            adjustments,
            &mut scratch.distribution,
        );
        need = need.saturating_sub(take);
    }
}

fn collect_reduction_sites(
    paragraph: &Paragraph,
    style: &Style,
    line_start: usize,
    line_end: usize,
    sites: &mut Vec<ReductionSite>,
) {
    sites.clear();
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
                sites,
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
            sites,
        );
    }
    if let Some(ordinal) = line_end.checked_sub(1).filter(|_| line_start < line_end) {
        append_line_end_reduction_site(
            paragraph,
            style,
            ordinal,
            ordinal.saturating_sub(line_start),
            sites,
        );
    }
}

#[cfg(test)]
fn reduction_sites(
    paragraph: &Paragraph,
    style: &Style,
    line_start: usize,
    line_end: usize,
) -> Vec<ReductionSite> {
    let mut sites = Vec::new();
    collect_reduction_sites(paragraph, style, line_start, line_end, &mut sites);
    sites
}

fn width_after_available_reduction_with_scratch(
    paragraph: &Paragraph,
    style: &Style,
    start: usize,
    end: usize,
    width: i64,
    available: i64,
    sites: &mut Vec<ReductionSite>,
) -> i64 {
    let need = width.saturating_sub(available);
    if need <= 0 {
        return width;
    }
    let line_start = cluster_index_at_or_after(paragraph, start);
    let line_end = cluster_index_at_or_after(paragraph, end);
    collect_reduction_sites(paragraph, style, line_start, line_end, sites);
    let capacity = sites.iter().fold(0_i64, |sum, site| {
        sum.saturating_add(i64::from(site.capacity))
    });
    let reduced = width.saturating_sub(need.min(capacity));
    reduced.saturating_sub(hanging_amount(
        paragraph, style, line_end, reduced, available,
    ))
}

#[cfg(test)]
fn width_after_available_reduction(
    paragraph: &Paragraph,
    style: &Style,
    start: usize,
    end: usize,
    width: i64,
    available: i64,
) -> i64 {
    let mut sites = Vec::new();
    width_after_available_reduction_with_scratch(
        paragraph, style, start, end, width, available, &mut sites,
    )
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
    let cell = match style.reduction_table() {
        ReductionTable::Table3 => crate::generated::table3::cell(before, 0),
        ReductionTable::Table4 => crate::generated::table4::cell(before, 0),
        ReductionTable::Table5 => crate::generated::table5::cell(before, 0),
    };
    let Some(cell) = cell else {
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

    let cell = match style.reduction_table() {
        ReductionTable::Table3 => crate::generated::table3::cell(before, after),
        ReductionTable::Table4 => crate::generated::table4::cell(before, after),
        ReductionTable::Table5 => crate::generated::table5::cell(before, after),
    };
    let Some(cell) = cell else {
        return;
    };
    let active = match components {
        [amount, 0] => Some((amount, before_size.inline())),
        [0, amount] => Some((amount, after_size.inline())),
        _ => None,
    };
    let (Some((amount, weight)), Some(_)) = (active, cell.limit) else {
        return;
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

fn distribute_reduction_with_scratch(
    amount: i64,
    sites: &[ReductionSite],
    remainder: Remainder,
    adjustments: &mut [i32],
    scratch: &mut DistributionScratch,
) {
    if amount <= 0 {
        return;
    }
    let weight_sum = sites.iter().fold(0_i64, |sum, site| {
        sum.saturating_add(i64::from(site.weight.max(1)))
    });
    scratch.assigned.clear();
    scratch.assigned.extend(sites.iter().map(|site| {
        amount
            .saturating_mul(i64::from(site.weight.max(1)))
            .checked_div(weight_sum.max(1))
            .unwrap_or(0)
            .min(i64::from(site.capacity))
    }));
    let left = amount.saturating_sub(
        scratch
            .assigned
            .iter()
            .fold(0_i64, |sum, take| sum.saturating_add(*take)),
    );
    scratch.capacities.clear();
    scratch.capacities.extend(
        sites
            .iter()
            .zip(&scratch.assigned)
            .map(|(site, take)| i64::from(site.capacity).saturating_sub(*take).max(0)),
    );
    capped_round_robin_into(left, &scratch.capacities, remainder, &mut scratch.extra);
    for (take, addition) in scratch.assigned.iter_mut().zip(&scratch.extra) {
        *take = take.saturating_add(*addition);
    }
    for (site, take) in sites.iter().zip(&scratch.assigned) {
        apply_reduction(site.boundary, *take, adjustments);
    }
}

#[cfg(test)]
fn distribute_reduction(
    amount: i64,
    sites: &[ReductionSite],
    remainder: Remainder,
    adjustments: &mut [i32],
) {
    let mut scratch = DistributionScratch::new();
    distribute_reduction_with_scratch(amount, sites, remainder, adjustments, &mut scratch);
}

fn capped_round_robin_into(
    amount: i64,
    capacities: &[i64],
    remainder: Remainder,
    shares: &mut Vec<i64>,
) {
    let total_capacity = capacities.iter().fold(0_i64, |sum, capacity| {
        sum.saturating_add((*capacity).max(0))
    });
    let target = amount.max(0).min(total_capacity);
    let maximum_rounds = capacities
        .iter()
        .copied()
        .map(|capacity| capacity.max(0))
        .max()
        .unwrap_or(0)
        .min(target);

    let mut lower = 0_i64;
    let mut upper = maximum_rounds;
    for _ in 0..64 {
        let distance = upper.saturating_sub(lower);
        if distance == 0 {
            break;
        }
        let previous = (lower, upper);
        let rounds = lower
            .saturating_add(distance / 2)
            .saturating_add(distance % 2);
        let consumed = capacities.iter().fold(0_i64, |sum, capacity| {
            sum.saturating_add((*capacity).max(0).min(rounds))
        });
        if consumed <= target {
            lower = rounds;
        } else {
            upper = rounds.saturating_sub(1);
        }
        if (lower, upper) == previous {
            break;
        }
    }

    shares.clear();
    shares.extend(
        capacities
            .iter()
            .map(|capacity| (*capacity).max(0).min(lower)),
    );
    let placed = shares
        .iter()
        .fold(0_i64, |sum, share| sum.saturating_add(*share));
    let mut left = target.saturating_sub(placed);
    match remainder {
        Remainder::Leading => {
            for (capacity, share) in capacities.iter().zip(&mut *shares) {
                if left == 0 {
                    break;
                }
                if (*capacity).max(0) > lower {
                    *share = share.saturating_add(1);
                    left = left.saturating_sub(1);
                }
            }
        },
        Remainder::Trailing => {
            for (capacity, share) in capacities.iter().zip(&mut *shares).rev() {
                if left == 0 {
                    break;
                }
                if (*capacity).max(0) > lower {
                    *share = share.saturating_add(1);
                    left = left.saturating_sub(1);
                }
            }
        },
    }
}

#[cfg(test)]
fn capped_round_robin(amount: i64, capacities: &[i64], remainder: Remainder) -> Vec<i64> {
    let mut shares = Vec::new();
    capped_round_robin_into(amount, capacities, remainder, &mut shares);
    shares
}

fn apply_reduction(boundary: usize, amount: i64, adjustments: &mut [i32]) {
    if let Some(adjustment) = adjustments.get_mut(boundary) {
        *adjustment = adjustment.saturating_sub(clamp_i32(amount));
    }
}

fn distribute_adjustment_with_scratch(
    amount: i64,
    sites: &[(usize, i32, Option<i32>)],
    remainder: Remainder,
    adjustments: &mut [i32],
    scratch: &mut DistributionScratch,
) {
    if amount <= 0 {
        return;
    }
    let weight_sum = sites.iter().fold(0_i64, |sum, (_, weight, _)| {
        sum.saturating_add(i64::from((*weight).max(1)))
    });
    scratch.assigned.clear();
    scratch
        .assigned
        .extend(sites.iter().map(|&(_, weight, cap)| {
            let proportional = amount
                .saturating_mul(i64::from(weight.max(1)))
                .checked_div(weight_sum.max(1))
                .unwrap_or(0);
            cap.map_or(proportional, |cap| proportional.min(i64::from(cap)))
        }));
    let mut placed = 0_i64;
    for (&(index, _, _), share) in sites.iter().zip(&scratch.assigned) {
        if let Some(adjustment) = adjustments.get_mut(index) {
            *adjustment = adjustment.saturating_add(clamp_i32(*share));
            placed = placed.saturating_add(*share);
        }
    }

    let left = amount.saturating_sub(placed);
    scratch.capacities.clear();
    scratch
        .capacities
        .extend(sites.iter().map(|&(index, _, cap)| {
            adjustments.get(index).map_or(0, |adjustment| {
                cap.map_or(left, |cap| {
                    i64::from(cap).saturating_sub(i64::from(*adjustment)).max(0)
                })
            })
        }));
    capped_round_robin_into(left, &scratch.capacities, remainder, &mut scratch.extra);
    for (&(index, _, _), addition) in sites.iter().zip(&scratch.extra) {
        if let Some(adjustment) = adjustments.get_mut(index) {
            *adjustment = adjustment.saturating_add(clamp_i32(*addition));
        }
    }
}

#[cfg(test)]
fn distribute_adjustment(
    amount: i64,
    sites: &[(usize, i32, Option<i32>)],
    remainder: Remainder,
    adjustments: &mut [i32],
) {
    let mut scratch = DistributionScratch::new();
    distribute_adjustment_with_scratch(amount, sites, remainder, adjustments, &mut scratch);
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
