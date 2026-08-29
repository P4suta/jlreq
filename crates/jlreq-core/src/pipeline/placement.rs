// SPDX-FileCopyrightText: 2026 jlreq contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

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
    paragraph.line_tabs.get(ordinal).copied().unwrap_or(false)
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
    if paragraph
        .find_construct_containing(ordinal, |construct| {
            matches!(construct.kind(), ConstructKind::TateChuYoko(_))
        })
        .is_some()
    {
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

fn place_attachments(
    paragraph: &Paragraph,
    style: &Style,
    line_index: usize,
    line_start: usize,
    line_end: usize,
    line: &mut Line,
    construct_ordinals: &mut Vec<usize>,
) {
    let mut attachment_extent = 0;
    paragraph.collect_constructs_overlapping(line_start, line_end, construct_ordinals);
    for ordinal in construct_ordinals.iter().copied() {
        let Some(construct) = paragraph.constructs.get(ordinal) else {
            continue;
        };
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
    let (annotation_count, annotation_width, _) =
        ruby_annotation_metrics(span.annotation, &span.annotation_range);
    let base_width = i64::from(base_end).saturating_sub(i64::from(base_start));
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
    } else if matches!(
        annotation_width.cmp(&base_width),
        core::cmp::Ordering::Greater
    ) {
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
    if total <= 0 {
        return vec![0; weights.len()];
    }
    let weight_sum = weights.iter().fold(0_i64, |sum, weight| {
        sum.saturating_add(i64::from((*weight).max(0)))
    });
    if weight_sum == 0 {
        return vec![0; weights.len()];
    }
    let total = u128::try_from(total).unwrap_or(0);
    let weight_sum = u128::try_from(weight_sum).unwrap_or(0);
    let mut shares: Vec<_> = weights
        .iter()
        .map(|weight| {
            let weight = u128::try_from((*weight).max(0)).unwrap_or(0);
            let share = total
                .saturating_mul(weight)
                .checked_div(weight_sum)
                .unwrap_or(0);
            i64::try_from(share).unwrap_or(i64::MAX)
        })
        .collect();
    let assigned = shares.iter().fold(0_u128, |sum, share| {
        sum.saturating_add(u128::try_from(*share).unwrap_or(0))
    });
    let left = usize::try_from(total.saturating_sub(assigned)).unwrap_or(usize::MAX);
    debug_assert!(left <= weights.len());
    match remainder {
        Remainder::Leading => {
            for share in shares.iter_mut().take(left) {
                *share = share.saturating_add(1);
            }
        },
        Remainder::Trailing => {
            for share in shares.iter_mut().rev().take(left) {
                *share = share.saturating_add(1);
            }
        },
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

