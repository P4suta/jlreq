// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

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

#[cfg(test)]
fn visit_ruby_spans(
    paragraph: &Paragraph,
    style: &Style,
    mut visit: impl FnMut(&Ruby, Range<usize>, Range<usize>),
) {
    for construct in &paragraph.constructs {
        let ConstructKind::Ruby(ruby) = construct.kind() else {
            continue;
        };
        visit_ruby_spans_for(style, ruby, &mut visit);
    }
}

fn visit_ruby_spans_for(
    style: &Style,
    ruby: &Ruby,
    mut visit: impl FnMut(&Ruby, Range<usize>, Range<usize>),
) {
    match ruby.kind() {
        RubyKind::Group => visit(ruby, ruby.base(), 0..ruby.annotation().source().len()),
        RubyKind::Mono => {
            for run in ruby.runs() {
                visit(ruby, run.base(), run.annotation());
            }
        },
        RubyKind::Jukugo => {
            if style.jukugo_ruby_layout() == JukugoRubyLayout::Phonetic {
                return;
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

fn visit_ruby_spans_overlapping(
    paragraph: &Paragraph,
    style: &Style,
    start: usize,
    end: usize,
    mut visit: impl FnMut(&Ruby, Range<usize>, Range<usize>),
) {
    paragraph.visit_constructs_overlapping(start, end, |_, construct| {
        if let ConstructKind::Ruby(ruby) = construct.kind() {
            visit_ruby_spans_for(style, ruby, &mut visit);
        }
    });
}

fn visit_rubies_overlapping(
    paragraph: &Paragraph,
    start: usize,
    end: usize,
    mut visit: impl FnMut(&Ruby),
) {
    paragraph.visit_constructs_overlapping(start, end, |_, construct| {
        if let ConstructKind::Ruby(ruby) = construct.kind() {
            visit(ruby);
        }
    });
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
        if base.start < line_start || base.end > line_end {
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
    let maximum_expansion = runs.iter().fold(0_i64, |sum, run| {
        let base_width = run.base_end.saturating_sub(run.base_start);
        sum.saturating_add(run.annotation_width.saturating_sub(base_width).max(0))
    });

    build_phonetic_jukugo_plan(paragraph, style, &runs, 0, edges).or_else(|| {
        let mut lower = 1_i64;
        let mut upper = maximum_expansion;
        let upper_plan = build_phonetic_jukugo_plan(paragraph, style, &runs, upper, edges)?;
        while lower < upper {
            let previous = (lower, upper);
            let middle = lower.saturating_add(upper.saturating_sub(lower) / 2);
            if build_phonetic_jukugo_plan(paragraph, style, &runs, middle, edges).is_some() {
                upper = middle;
            } else {
                lower = middle.saturating_add(1);
            }
            if (lower, upper) == previous {
                return Some(upper_plan);
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
        if amount == 0 {
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
    let mut inter_run_boundary = None;
    for raw in raw_runs {
        if let Some(boundary) = inter_run_boundary {
            cursor = cursor
                .saturating_add(i64::from(boundary_space_after(paragraph, boundary)))
                .saturating_add(phonetic_gap_after(&gaps_after, boundary));
        }
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
        inter_run_boundary = Some(raw.base.end.saturating_sub(1));
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
    if base.start < line_start || base.end > line_end {
        return None;
    }
    let base_width = ruby_base_width(paragraph, style, &base);
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
    let boundary_end = before
        .saturating_add(2)
        .min(paragraph.text.clusters().len());
    visit_ruby_spans_overlapping(
        paragraph,
        style,
        before,
        boundary_end,
        |ruby, base, annotation| {
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
        },
    );
    visit_rubies_overlapping(paragraph, before, boundary_end, |ruby| {
        if let Some(plan) =
            phonetic_jukugo_plan(paragraph, style, ruby, line_start, line_end, line_index)
        {
            required = required.max(plan.gap_after(before));
        }
    });
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
    visit_ruby_spans_overlapping(
        paragraph,
        style,
        line_start,
        line_end,
        |ruby, base, annotation| {
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
            let allowance = if line_index == 0
                && style.ruby_overhang_indent() == RubyOverhangIndent::Permitted
            {
                line_head_indent(paragraph, style, line_start, line_index).min(overhang.ruby_em)
            } else {
                0
            };
            required = required.max(overhang.leading.saturating_sub(allowance));
        },
    );
    visit_rubies_overlapping(paragraph, line_start, line_end, |ruby| {
        if let Some(plan) =
            phonetic_jukugo_plan(paragraph, style, ruby, line_start, line_end, line_index)
        {
            required = required.max(plan.leading_gap);
        }
    });
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
    let table_is_blank = table_one_cell_is_blank(before, after);
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

fn table_one_cell_is_blank(before: u8, after: u8) -> bool {
    crate::generated::table1::cell(before, after).is_none_or(|cell| cell.terms.is_empty())
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
    if let Some((_, construct)) = paragraph.find_construct_containing(ordinal, |construct| {
        matches!(
            construct.kind(),
            ConstructKind::Ruby(_)
                | ConstructKind::Emphasis { .. }
                | ConstructKind::Script { .. }
                | ConstructKind::ReferenceMark { .. }
        )
    }) {
        return match construct.kind() {
            ConstructKind::Ruby(ruby) if ruby.kind() == RubyKind::Jukugo => 23,
            ConstructKind::Ruby(_) => 22,
            ConstructKind::Emphasis { .. } | ConstructKind::Script { .. } => 21,
            ConstructKind::ReferenceMark { .. } => 20,
            _ => 19,
        };
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
        (Some(current_range), Some(_)) => {
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
            }
            Some(0)
        },
        (None, Some(_)) => Some(formula_outer_boundary_space(paragraph, current, following)),
        (Some(_), None) => Some(formula_outer_boundary_space(paragraph, following, current)),
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
