// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

fn paragraph_segments(text: &str) -> Vec<ParagraphSegment> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut indices = text.char_indices().peekable();
    while let Some((offset, character)) = indices.next() {
        let mut separator_end = offset.saturating_add(character.len_utf8());
        let separator = match character {
            '\r' => {
                if indices.peek().is_some_and(|(_, next)| *next == '\n')
                    && let Some((next_offset, next)) = indices.next()
                {
                    separator_end = next_offset.saturating_add(next.len_utf8());
                }
                true
            },
            '\n' | '\u{2028}' | '\u{2029}' => true,
            _ => false,
        };
        if separator {
            result.push(ParagraphSegment {
                content: start..offset,
            });
            start = separator_end;
        }
    }
    result.push(ParagraphSegment {
        content: start..text.len(),
    });
    result
}

fn collect_breaks(
    document: &Document,
    paragraph_range: &Range<usize>,
    source: &str,
    prepared: &PreparedText,
    constructs: &[jlreq_core::Construct],
) -> Vec<jlreq_core::Break> {
    let prohibited = sorted_offsets_in_range(&document.prohibited_breaks, paragraph_range);
    let mandatory = sorted_offsets_in_range(&document.mandatory_breaks, paragraph_range);
    let mut breaks = BTreeMap::new();
    let mut construct_index = 0_usize;
    let mut maximum_construct_end = 0_usize;
    for offset in LineSegmenter::new_auto(LineBreakOptions::default()).segment_str(source) {
        while let Some(construct) = constructs.get(construct_index) {
            let range = construct.range();
            if range.start >= offset {
                break;
            }
            maximum_construct_end = maximum_construct_end.max(range.end);
            construct_index = construct_index.saturating_add(1);
        }
        if automatic_break_allowed(
            offset,
            source.len(),
            prepared.is_boundary(offset, source.len()),
            prohibited
                .binary_search(&offset.saturating_add(paragraph_range.start))
                .is_ok(),
            offset < maximum_construct_end,
        ) {
            breaks.insert(offset, false);
        }
    }
    for offset in mandatory.iter().copied() {
        let offset = offset.saturating_sub(paragraph_range.start);
        if prepared.is_boundary(offset, source.len()) {
            breaks.insert(offset, true);
        }
    }
    breaks
        .into_iter()
        .map(|(offset, required)| {
            if required {
                jlreq_core::Break::mandatory(offset)
            } else {
                jlreq_core::Break::allowed(offset)
            }
        })
        .collect()
}

fn sorted_offsets_in_range<'a>(offsets: &'a [usize], range: &Range<usize>) -> &'a [usize] {
    let start = offsets.partition_point(|offset| *offset < range.start);
    let end = offsets.partition_point(|offset| *offset < range.end);
    &offsets[start..end]
}

fn automatic_break_allowed(
    offset: usize,
    source_len: usize,
    is_cluster_boundary: bool,
    is_prohibited: bool,
    is_inside_construct: bool,
) -> bool {
    offset > 0
        && offset < source_len
        && is_cluster_boundary
        && !is_prohibited
        && !is_inside_construct
}

fn collect_tab_stops(
    source: &str,
    options: &LayoutOptions,
) -> Result<Vec<jlreq_core::TabStop>, LayoutError> {
    if !source.contains('\t') {
        return Ok(Vec::new());
    }
    let interval = options
        .font_size
        .saturating_mul(i32::from(options.tab_width));
    let mut position = interval;
    let mut result = Vec::new();
    while position < options.line_extent && result.len() < options.limits.constructs {
        result.push(jlreq_core::TabStop::new(
            position,
            jlreq_core::TabAlignment::Start,
        )?);
        position = position.saturating_add(interval);
    }
    Ok(result)
}

fn annotation_options(options: &LayoutOptions) -> LayoutOptions {
    let mut result = options.clone();
    result.font_size = (options.font_size / 2).max(1);
    result.alignment = Alignment::Start;
    result
}

fn ruby_runs(
    kind: crate::RubyKind,
    local_base: &Range<usize>,
    paragraph_offset: usize,
    declared: &[crate::RubyRun],
    base: &PreparedText,
    annotation: &PreparedText,
    annotation_len: usize,
) -> Result<Vec<jlreq_core::RubyRun>, LayoutError> {
    if !declared.is_empty() {
        return Ok(declared
            .iter()
            .map(|run| {
                let base = run.base();
                jlreq_core::RubyRun::new(
                    base.start.saturating_sub(paragraph_offset)
                        ..base.end.saturating_sub(paragraph_offset),
                    run.annotation(),
                )
            })
            .collect());
    }
    if kind != crate::RubyKind::Mono {
        return Ok(vec![jlreq_core::RubyRun::new(
            local_base.clone(),
            0..annotation_len,
        )]);
    }
    let base_clusters: Vec<_> = base
        .clusters
        .iter()
        .filter(|cluster| {
            local_base.start <= cluster.range.start && cluster.range.end <= local_base.end
        })
        .collect();
    if base_clusters.len() != annotation.clusters.len() || base_clusters.is_empty() {
        return Err(LayoutError::invalid_document(
            "document.mono-ruby-cluster-count",
            Some(
                local_base.start.saturating_add(paragraph_offset)
                    ..local_base.end.saturating_add(paragraph_offset),
            ),
        ));
    }
    Ok(base_clusters
        .iter()
        .zip(&annotation.clusters)
        .map(|(base, annotation)| {
            jlreq_core::RubyRun::new(base.range.clone(), annotation.range.clone())
        })
        .collect())
}

