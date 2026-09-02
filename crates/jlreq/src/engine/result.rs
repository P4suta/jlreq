// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

struct LineMapping<'a> {
    attachments: &'a [Option<AttachmentShape>],
    construct_globals: &'a [(Range<usize>, usize)],
    global_offset: usize,
    block_offset: i32,
    paragraph_index: usize,
}

fn map_core_lines(
    layout: &jlreq_core::Layout,
    prepared: &PreparedText,
    mapping: &LineMapping<'_>,
    options: &LayoutOptions,
) -> Vec<TextLine> {
    let LineMapping {
        attachments,
        construct_globals,
        global_offset,
        block_offset,
        paragraph_index,
    } = *mapping;
    let mut result = Vec::with_capacity(layout.lines().len());
    let mut used_clusters = vec![0_usize; prepared.clusters.len()];
    for (line_index, line) in layout.lines().iter().enumerate() {
        let mut cells = Vec::new();
        let epoch = line_index.saturating_add(1);
        for placement in line.clusters() {
            let range = placement.range();
            let cluster_indices = placement_cluster_indices(placement.origin(), prepared, &range);
            let cluster_indices: Vec<_> = cluster_indices
                .into_iter()
                .filter(|index| {
                    let Some(seen) = used_clusters.get_mut(*index) else {
                        return false;
                    };
                    if *seen == epoch {
                        false
                    } else {
                        *seen = epoch;
                        true
                    }
                })
                .collect();
            if cluster_indices.is_empty() {
                continue;
            }
            let level = prepared.clusters[cluster_indices[0]].bidi_level;
            let construct = match placement.origin() {
                jlreq_core::PlacementOrigin::Construct(local) => construct_globals
                    .get(local)
                    .map(|(_, global)| *global),
                jlreq_core::PlacementOrigin::Cluster(_) => cluster_indices
                    .first()
                    .and_then(|index| prepared.clusters.get(*index))
                    .and_then(|cluster| covering_construct(construct_globals, &cluster.range)),
                _ => None,
            };
            cells.push(Cell {
                clusters: cluster_indices,
                inline: placement.inline(),
                block: placement.block(),
                advance: placement.advance().max(0),
                level,
                transform: core_transform(placement.transform()),
                construct,
                trailing_gap: 0,
            });
        }
        assign_trailing_gaps(&mut cells);
        let levels: Vec<_> = cells
            .iter()
            .map(|cell| Level::new(cell.level).unwrap_or_else(|_| Level::ltr()))
            .collect();
        let visual = BidiInfo::reorder_visual(&levels);
        // The physical run starts where the core placed its first cluster,
        // which folds in alignment, first-line indent, and ruby leading
        // separation; `inline_origin` alone carries only the alignment
        // offset. Warichu and furawake lanes restart inside the line, so the
        // minimum placement inline — not the first one — is that start.
        let mut cursor = cells
            .iter()
            .map(|cell| cell.inline)
            .min()
            .unwrap_or_else(|| line.inline_origin());
        let mut glyphs = Vec::new();
        for visual_index in visual {
            let cell = &cells[visual_index];
            let mut cluster_cursor = 0_i32;
            visit_logical_cluster_order(&cell.clusters, cell.level, |cluster_index| {
                let cluster = &prepared.clusters[cluster_index];
                let mut glyph_cursor = 0_i32;
                for raw in &cluster.glyphs {
                    glyphs.push(place_raw_glyph(
                        raw,
                        cluster,
                        PlacementContext {
                            source_range: global_offset.saturating_add(cluster.range.start)
                                ..global_offset.saturating_add(cluster.range.end),
                            annotation: None,
                            inline: cursor
                                .saturating_add(cluster_cursor)
                                .saturating_add(glyph_cursor),
                            block: adjusted_block(cell.block, line_index, block_offset, options),
                            transform: cell.transform,
                            writing_mode: options.writing_mode,
                            construct: cell.construct,
                        },
                    ));
                    glyph_cursor = glyph_cursor.saturating_add(raw.inline_advance(
                        direction_from_geometry(raw, options.writing_mode, cell.transform),
                    ));
                }
                cluster_cursor = cluster_cursor.saturating_add(cluster.advance);
            });
            cursor = cursor
                .saturating_add(cell.advance.max(cluster_cursor))
                .saturating_add(cell.trailing_gap);
        }
        append_attachments(
            &mut glyphs,
            line,
            attachments,
            line_index,
            block_offset,
            options,
        );
        let physical_origin = match options.writing_mode {
            WritingMode::HorizontalTb => Point::from_fixed(
                line.inline_origin(),
                adjusted_block(line.block_origin(), line_index, block_offset, options),
            ),
            WritingMode::VerticalRl => Point::from_fixed(
                adjusted_block(line.block_origin(), line_index, block_offset, options),
                line.inline_origin(),
            ),
        };
        let hit_bounds = TextLine::hit_bounds_for(&glyphs);
        result.push(TextLine {
            range: line.range().start.saturating_add(global_offset)
                ..line.range().end.saturating_add(global_offset),
            origin: physical_origin,
            inline_extent: line.inline_extent(),
            block_extent: line.block_extent(),
            writing_mode: options.writing_mode,
            glyphs,
            hit_bounds,
            index: 0,
            paragraph_index,
            first_in_paragraph: false,
            last_in_paragraph: false,
        });
    }
    result
}

/// The document ordinal of the innermost construct covering a cluster range.
fn covering_construct(
    construct_globals: &[(Range<usize>, usize)],
    cluster: &Range<usize>,
) -> Option<usize> {
    let mut best: Option<(usize, usize)> = None;
    for (range, global) in construct_globals {
        if range.start <= cluster.start && cluster.end <= range.end {
            let span = range.end.saturating_sub(range.start);
            if best.is_none_or(|(kept, _)| span < kept) {
                best = Some((span, *global));
            }
        }
    }
    best.map(|(_, global)| global)
}

fn placement_cluster_indices(
    origin: jlreq_core::PlacementOrigin,
    prepared: &PreparedText,
    range: &Range<usize>,
) -> Range<usize> {
    match origin {
        jlreq_core::PlacementOrigin::Cluster(index) => {
            index.min(prepared.clusters.len())..index.saturating_add(1).min(prepared.clusters.len())
        },
        jlreq_core::PlacementOrigin::Construct(_) => prepared.cluster_range(range),
        _ => 0..0,
    }
}

fn visit_logical_cluster_order(indices: &[usize], level: u8, mut visit: impl FnMut(usize)) {
    if level % 2 == 1 {
        for index in indices.iter().rev().copied() {
            visit(index);
        }
    } else {
        for index in indices.iter().copied() {
            visit(index);
        }
    }
}

#[cfg(test)]
fn logical_cluster_order(indices: &[usize], level: u8) -> Vec<usize> {
    let mut ordered = Vec::with_capacity(indices.len());
    visit_logical_cluster_order(indices, level, |index| ordered.push(index));
    ordered
}

#[derive(Debug)]
struct Cell {
    clusters: Vec<usize>,
    inline: i32,
    block: i32,
    advance: i32,
    level: u8,
    transform: GlyphTransform,
    construct: Option<usize>,
    trailing_gap: i32,
}

/// Record the inline space the core inserted after each cell.
///
/// The core applies alignment adjustment and JLReq spacing to its own cursor
/// rather than to a cluster's advance, so the space lives in the distance
/// between consecutive placements. Physical layout reorders cells visually
/// and cannot simply reuse each placement's inline position, so it carries
/// the gap alongside the advance instead: the line keeps the core's total
/// width, and each gap stays attached to the cell it followed. A lane that
/// restarts behind its predecessor (warichu, furawake) yields no gap.
fn assign_trailing_gaps(cells: &mut [Cell]) {
    for index in 0..cells.len() {
        let Some(next) = cells.get(index.saturating_add(1)) else {
            break;
        };
        let Some(cell) = cells.get(index) else {
            break;
        };
        let occupied = cell.inline.saturating_add(cell.advance);
        let gap = next.inline.saturating_sub(occupied).max(0);
        if let Some(cell) = cells.get_mut(index) {
            cell.trailing_gap = gap;
        }
    }
}

struct PlacementContext {
    source_range: Range<usize>,
    annotation: Option<AnnotationSource>,
    inline: i32,
    block: i32,
    transform: GlyphTransform,
    writing_mode: WritingMode,
    construct: Option<usize>,
}

fn place_raw_glyph(
    raw: &RawGlyph,
    cluster: &PreparedCluster,
    placement: PlacementContext,
) -> GlyphPlacement {
    let PlacementContext {
        source_range,
        annotation,
        inline,
        block,
        transform,
        writing_mode,
        construct,
    } = placement;
    let horizontal =
        writing_mode == WritingMode::HorizontalTb || transform == GlyphTransform::TateChuYoko;
    let (x, y, advance_x, advance_y, offset_x, offset_y) = if horizontal {
        (
            inline,
            block.saturating_add(cluster.size),
            raw.x_advance.abs(),
            0,
            raw.x_offset,
            raw.y_offset.saturating_neg(),
        )
    } else {
        (
            block,
            inline,
            0,
            raw.y_advance.abs().max(raw.x_advance.abs()),
            raw.x_offset,
            raw.y_offset.saturating_neg(),
        )
    };
    GlyphPlacement {
        font_id: raw.font_id,
        glyph_id: raw.glyph_id,
        source_range,
        annotation,
        x,
        y,
        advance_x,
        advance_y,
        offset_x,
        offset_y,
        font_size: cluster.size,
        variations: Arc::clone(&cluster.variations),
        transform,
        bidi_level: cluster.bidi_level,
        writing_mode,
        construct,
    }
}

fn append_attachments(
    glyphs: &mut Vec<GlyphPlacement>,
    line: &jlreq_core::Line,
    shapes: &[Option<AttachmentShape>],
    line_index: usize,
    block_offset: i32,
    options: &LayoutOptions,
) {
    for attachment in line.attachments() {
        let Some(shape) = shapes.get(attachment.construct()).and_then(Option::as_ref) else {
            continue;
        };
        let requested = attachment.range();
        let cluster_range = if requested.is_empty() {
            0..shape.prepared.clusters.len()
        } else {
            shape.prepared.cluster_range(&requested)
        };
        let transform = core_transform(attachment.transform());
        let mut inline = attachment.inline();
        for cluster in &shape.prepared.clusters[cluster_range] {
            let mut glyph_cursor = 0_i32;
            for raw in &cluster.glyphs {
                glyphs.push(place_raw_glyph(
                    raw,
                    cluster,
                    PlacementContext {
                        source_range: shape.base.clone(),
                        annotation: Some(AnnotationSource::new(
                            shape.global_ordinal,
                            cluster.range.clone(),
                        )),
                        inline: inline.saturating_add(glyph_cursor),
                        block: adjusted_block(
                            attachment.block(),
                            line_index,
                            block_offset,
                            options,
                        ),
                        transform,
                        writing_mode: options.writing_mode,
                        construct: Some(shape.global_ordinal),
                    },
                ));
                glyph_cursor = glyph_cursor.saturating_add(raw.inline_advance(
                    direction_from_geometry(raw, options.writing_mode, transform),
                ));
            }
            inline = inline.saturating_add(cluster.advance);
        }
    }
}

fn adjusted_block(
    core_block: i32,
    line_index: usize,
    block_offset: i32,
    options: &LayoutOptions,
) -> i32 {
    let gap = i32::try_from(line_index)
        .unwrap_or(i32::MAX)
        .saturating_mul(options.line_gap);
    match options.writing_mode {
        WritingMode::HorizontalTb => block_offset.saturating_add(core_block).saturating_add(gap),
        WritingMode::VerticalRl => block_offset.saturating_add(core_block).saturating_sub(gap),
    }
}

fn core_transform(value: jlreq_core::CoordinateTransform) -> GlyphTransform {
    match value {
        jlreq_core::CoordinateTransform::RotateClockwise => GlyphTransform::RotateClockwise,
        jlreq_core::CoordinateTransform::TateChuYoko => GlyphTransform::TateChuYoko,
        _ => GlyphTransform::Identity,
    }
}

fn direction_from_geometry(
    raw: &RawGlyph,
    mode: WritingMode,
    transform: GlyphTransform,
) -> Direction {
    if mode == WritingMode::VerticalRl
        && transform != GlyphTransform::TateChuYoko
        && raw.y_advance != 0
    {
        Direction::TopToBottom
    } else {
        Direction::LeftToRight
    }
}

fn advance_block(value: i32, amount: i32, mode: WritingMode) -> i32 {
    match mode {
        WritingMode::HorizontalTb => value.saturating_add(amount),
        WritingMode::VerticalRl => value.saturating_sub(amount),
    }
}

fn next_paragraph_block_offset(
    lines: &[TextLine],
    current: i32,
    options: &LayoutOptions,
) -> i32 {
    if lines.is_empty() {
        return advance_block(
            current,
            options.font_size.saturating_add(options.line_gap),
            options.writing_mode,
        );
    }
    match options.writing_mode {
        WritingMode::HorizontalTb => lines
            .iter()
            .map(|line| {
                let (_, y, _, height) = line.bounds().as_26_6();
                y.saturating_add(height)
            })
            .max()
            .unwrap_or(current)
            .saturating_add(options.line_gap),
        WritingMode::VerticalRl => lines
            .iter()
            .map(|line| line.bounds().as_26_6().0)
            .min()
            .unwrap_or(current)
            .saturating_sub(options.line_gap),
    }
}

fn ranges_overlap(left: &Range<usize>, right: &Range<usize>) -> bool {
    left.start < right.end && right.start < left.end
}

fn diagnostic_severity(value: jlreq_core::Severity) -> DiagnosticSeverity {
    match value {
        jlreq_core::Severity::Info => DiagnosticSeverity::Info,
        jlreq_core::Severity::Error => DiagnosticSeverity::Error,
        // Severity is non_exhaustive: Warning doubles as the conservative
        // mapping for any severity introduced by a future core.
        _ => DiagnosticSeverity::Warning,
    }
}
