// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::ops::Range;
use std::sync::Arc;

use harfrust::{
    Direction, Feature, Language, ShapeOptions, ShaperData, ShaperInstance, UnicodeBuffer,
    Variation,
};
use icu_segmenter::{GraphemeClusterSegmenter, LineSegmenter, options::LineBreakOptions};
use unicode_bidi::{BidiInfo, Level, ParagraphBidiInfo};

use crate::document::{DocumentConstruct, TextRole};
use crate::{
    Alignment, AnnotationSource, BaseDirection, Diagnostic, DiagnosticSeverity, Document,
    DocumentBuilder, FontId, FontLibrary, FontResource, FontStyle, FontVariation, GlyphPlacement,
    GlyphTransform, LayoutError, LayoutOptions, OpenTypeFeature, Point, Resource, SpanStyle,
    TextLayout, TextLine, WritingMode,
};

struct FontCache {
    bytes: Arc<[u8]>,
    face_index: u32,
    shaper_data: ShaperData,
}

struct ShapeRequest<'a> {
    source: &'a str,
    range: Range<usize>,
    resource: &'a FontResource,
    size: i32,
    direction: Direction,
    language: &'a str,
    features: &'a [OpenTypeFeature],
    variations: &'a [FontVariation],
}

struct PrepareRequest<'a> {
    source: &'a str,
    global_offset: usize,
    spans: &'a [(Range<usize>, SpanStyle)],
    fonts: &'a FontLibrary,
    options: &'a LayoutOptions,
    diagnostic_range: Option<Range<usize>>,
}

/// Reusable high-level layout engine.
///
/// Font parsing and shaping caches are retained between calls. Returned layouts never borrow
/// the engine, and an error leaves it immediately reusable.
pub struct LayoutEngine {
    fonts: BTreeMap<FontId, FontCache>,
    composer: jlreq_core::Composer,
}

impl fmt::Debug for LayoutEngine {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LayoutEngine")
            .field("cached_fonts", &self.fonts.len())
            .finish_non_exhaustive()
    }
}

impl Default for LayoutEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutEngine {
    /// Build an empty reusable engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            fonts: BTreeMap::new(),
            composer: jlreq_core::Composer::new(),
        }
    }

    /// Shape, compose, reorder, and physically place plain UTF-8 text.
    pub fn layout(
        &mut self,
        text: &str,
        fonts: &FontLibrary,
        options: LayoutOptions,
    ) -> Result<TextLayout, LayoutError> {
        let document = DocumentBuilder::new(text).build()?;
        self.layout_document(&document, fonts, options)
    }

    /// Shape, compose, reorder, and physically place a typed document.
    pub fn layout_document(
        &mut self,
        document: &Document,
        fonts: &FontLibrary,
        options: LayoutOptions,
    ) -> Result<TextLayout, LayoutError> {
        // The public API intentionally takes an owned option set so callers can configure and
        // submit it in one expression. Moving it through this single-element container makes
        // that ownership boundary explicit while the pipeline borrows the stable value.
        let options = [options];
        let options = &options[0];
        validate_call(document, fonts, options)?;
        if document.text.is_empty() {
            return Ok(TextLayout {
                source: String::new(),
                lines: Vec::new(),
                fonts: Vec::new(),
                diagnostics: Vec::new(),
                writing_mode: options.writing_mode,
            });
        }

        let segments = paragraph_segments(&document.text);
        check_limit(
            Resource::Paragraphs,
            options.limits.paragraphs,
            segments.len(),
        )?;
        let mut call = CallState::new(options);
        let mut lines = Vec::new();
        let mut block_offset = 0_i32;

        for segment in segments {
            let content = &document.text[segment.content.clone()];
            if content.is_empty() {
                let origin = match options.writing_mode {
                    WritingMode::HorizontalTb => Point::from_fixed(0, block_offset),
                    WritingMode::VerticalRl => Point::from_fixed(block_offset, 0),
                };
                lines.push(TextLine {
                    range: segment.content.clone(),
                    origin,
                    inline_extent: 0,
                    block_extent: options.font_size,
                    writing_mode: options.writing_mode,
                    glyphs: Vec::new(),
                });
                block_offset = advance_block(
                    block_offset,
                    options.font_size.saturating_add(options.line_gap),
                    options.writing_mode,
                );
                continue;
            }

            let prepared = self.prepare_text(
                PrepareRequest {
                    source: content,
                    global_offset: segment.content.start,
                    spans: &document.spans,
                    fonts,
                    options,
                    diagnostic_range: None,
                },
                &mut call,
            )?;
            let shaped = prepared.to_core(content, options.font_size)?;
            let (constructs, attachments) = self.lower_constructs(
                document,
                &segment.content,
                &prepared,
                fonts,
                options,
                &mut call,
            )?;
            let breaks =
                collect_breaks(document, &segment.content, content, &prepared, &constructs);
            let tabs = collect_tab_stops(content, options)?;
            let paragraph = jlreq_core::Paragraph::builder(shaped, options.line_extent)
                .breaks(breaks)
                .constructs(constructs)
                .tab_stops(tabs)
                .alignment(options.alignment.core())
                .writing_mode(options.writing_mode.core())
                .build()?;

            let core_limits = jlreq_core::CompositionLimits::default()
                .with_max_clusters(options.limits.glyphs)
                .with_max_break_candidates(options.limits.runs)
                .with_max_constructs(options.limits.constructs)
                .with_max_tab_stops(options.limits.constructs)
                .with_max_search_transitions(options.limits.core_operations);
            self.composer.set_limits(core_limits);
            let core_layout = self
                .composer
                .compose(&paragraph, &options.style)
                .map_err(map_core_resource_error)?;

            for diagnostic in core_layout.diagnostics() {
                call.diagnostics.push(Diagnostic {
                    code: diagnostic.code(),
                    // Keep every public core severity distinct at the facade boundary.
                    // The helper is independently tested even when a particular composer
                    // release does not currently emit every declared severity.
                    severity: diagnostic_severity(diagnostic.severity()),
                    range: diagnostic.range().map(|range| {
                        range.start.saturating_add(segment.content.start)
                            ..range.end.saturating_add(segment.content.start)
                    }),
                    message: "the core composer produced a recoverable layout diagnostic",
                    jlreq: Some(diagnostic.jlreq()),
                });
            }

            let paragraph_lines = map_core_lines(
                &core_layout,
                &prepared,
                &attachments,
                segment.content.start,
                block_offset,
                options,
            );
            let consumed = paragraph_lines.len().max(1);
            lines.extend(paragraph_lines);
            let line_count = i32::try_from(consumed).unwrap_or(i32::MAX);
            let block_advance = line_count
                .saturating_mul(options.font_size.saturating_add(options.line_gap).max(0));
            block_offset = advance_block(block_offset, block_advance, options.writing_mode);
        }

        call.diagnostics.sort_by_key(|diagnostic| {
            diagnostic
                .range
                .as_ref()
                .map_or((usize::MAX, usize::MAX), |range| (range.start, range.end))
        });
        let retained_fonts = call
            .used_fonts
            .iter()
            .filter_map(|id| fonts.get(*id).cloned())
            .collect();
        Ok(TextLayout {
            source: document.text.clone(),
            lines,
            fonts: retained_fonts,
            diagnostics: call.diagnostics,
            writing_mode: options.writing_mode,
        })
    }

    fn ensure_cache(&mut self, resource: &FontResource) -> Result<(), LayoutError> {
        let needs_replacement = self.fonts.get(&resource.id()).is_none_or(|cached| {
            cached.face_index != resource.face_index()
                || !Arc::ptr_eq(&cached.bytes, &resource.bytes)
        });
        if needs_replacement {
            let font = harfrust::FontRef::from_index(resource.bytes(), resource.face_index())
                .map_err(|_| LayoutError::invalid_font(resource.face_index()))?;
            let shaper_data = ShaperData::new(&font);
            self.fonts.insert(
                resource.id(),
                FontCache {
                    bytes: resource.bytes.clone(),
                    face_index: resource.face_index(),
                    shaper_data,
                },
            );
        }
        Ok(())
    }

    fn shape_font(&mut self, request: ShapeRequest<'_>) -> Result<Vec<RawGlyph>, LayoutError> {
        let ShapeRequest {
            source,
            range,
            resource,
            size,
            direction,
            language,
            features,
            variations,
        } = request;
        self.ensure_cache(resource)?;
        let cached = self
            .fonts
            .get(&resource.id())
            .ok_or_else(|| LayoutError::invalid_font(resource.face_index()))?;
        let font = harfrust::FontRef::from_index(&cached.bytes, cached.face_index)
            .map_err(|_| LayoutError::invalid_font(cached.face_index))?;
        let variations: Vec<_> = variations
            .iter()
            .map(|variation| Variation {
                tag: harfrust::Tag::new(&variation.tag().bytes()),
                value: variation.value(),
            })
            .collect();
        let instance = ShaperInstance::from_variations(&font, variations);
        let shaper = cached
            .shaper_data
            .shaper(&font)
            .instance(Some(&instance))
            .build();
        let mut buffer = UnicodeBuffer::new();
        for (relative, character) in source[range.clone()].char_indices() {
            let cluster = u32::try_from(range.start.saturating_add(relative)).map_err(|_| {
                LayoutError::resource(Resource::InputBytes, u32::MAX as usize, source.len())
            })?;
            buffer.add(character, cluster);
        }
        buffer.set_direction(direction);
        if let Some(language) = Language::new(language) {
            buffer.set_language(language);
        }
        buffer.guess_segment_properties();
        let features: Vec<_> = features
            .iter()
            .map(|feature| {
                Feature::new(
                    harfrust::Tag::new(&feature.tag().bytes()),
                    feature.value(),
                    ..,
                )
            })
            .collect();
        let glyphs = shaper.shape(
            buffer,
            ShapeOptions::new().scale(Some(size)).features(&features),
        );
        Ok(glyphs
            .glyph_infos()
            .iter()
            .zip(glyphs.glyph_positions())
            .map(|(info, position)| RawGlyph {
                font_id: resource.id(),
                glyph_id: info.glyph_id,
                cluster: info.cluster as usize,
                x_advance: position.x_advance,
                y_advance: position.y_advance,
                x_offset: position.x_offset,
                y_offset: position.y_offset,
            })
            .collect())
    }

    fn prepare_text(
        &mut self,
        request: PrepareRequest<'_>,
        call: &mut CallState,
    ) -> Result<PreparedText, LayoutError> {
        let PrepareRequest {
            source,
            global_offset,
            spans,
            fonts,
            options,
            diagnostic_range,
        } = request;
        if source.is_empty() {
            return Ok(PreparedText {
                clusters: Vec::new(),
            });
        }
        let base_level = match options.base_direction {
            BaseDirection::Auto => None,
            BaseDirection::LeftToRight => Some(Level::ltr()),
            BaseDirection::RightToLeft => Some(Level::rtl()),
        };
        let bidi = ParagraphBidiInfo::new(source, base_level);
        let boundaries: Vec<_> = GraphemeClusterSegmenter::new()
            .segment_str(source)
            .collect();
        let mut graphemes = Vec::with_capacity(boundaries.len().saturating_sub(1));
        for pair in boundaries.windows(2) {
            let range = pair[0]..pair[1];
            let global =
                range.start.saturating_add(global_offset)..range.end.saturating_add(global_offset);
            let effective = effective_style(&global, spans, options)?;
            let level = bidi
                .levels
                .get(range.start)
                .copied()
                .unwrap_or(bidi.paragraph_level);
            let script = script_class(&source[range.clone()]);
            let direction = shape_direction(options.writing_mode, level, script);
            let is_tab = &source[range.clone()] == "\t";
            let (font_id, missing) = if is_tab {
                (fonts.primary().ok_or(LayoutError::NoFonts)?, false)
            } else {
                self.select_font(source, range.clone(), fonts, &effective, direction)?
            };
            if missing {
                call.diagnostics.push(Diagnostic {
                    code: "font.missing-glyph",
                    severity: DiagnosticSeverity::Warning,
                    range: Some(diagnostic_range.clone().unwrap_or(global)),
                    message: "no fallback face covers the complete grapheme; primary .notdef was retained",
                    jlreq: None,
                });
            }
            graphemes.push(GraphemeItem {
                range,
                level,
                script,
                direction,
                font_id,
                effective,
                is_tab,
            });
        }

        let mut clusters = Vec::new();
        let mut index = 0;
        while index < graphemes.len() {
            if graphemes[index].is_tab {
                let item = &graphemes[index];
                clusters.push(PreparedCluster {
                    range: item.range.clone(),
                    advance: 0,
                    size: item.effective.size,
                    frame: jlreq_core::Frame::Proportional,
                    role: None,
                    bidi_level: item.level.number(),
                    glyphs: Vec::new(),
                });
                index = index.saturating_add(1);
                continue;
            }
            let start = index;
            index = index.saturating_add(1);
            while index < graphemes.len()
                && !graphemes[index].is_tab
                && graphemes[index].same_run(&graphemes[start])
            {
                index = index.saturating_add(1);
            }
            call.charge_run()?;
            let first = &graphemes[start];
            let run_range = first.range.start..graphemes[index.saturating_sub(1)].range.end;
            let resource = fonts
                .get(first.font_id)
                .ok_or_else(|| LayoutError::invalid_document("font.unknown-id", None))?;
            let raw = self.shape_font(ShapeRequest {
                source,
                range: run_range.clone(),
                resource,
                size: first.effective.size,
                direction: first.direction,
                language: &first.effective.language,
                features: &first.effective.features,
                variations: &first.effective.variations,
            })?;
            call.used_fonts.insert(first.font_id);
            call.charge_glyphs(raw.len())?;
            clusters.extend(aggregate_run(
                source,
                run_range,
                &raw,
                first.effective.size,
                first.effective.role,
                first.level.number(),
                first.direction,
            ));
        }
        Ok(PreparedText { clusters })
    }

    fn select_font(
        &mut self,
        source: &str,
        range: Range<usize>,
        fonts: &FontLibrary,
        style: &EffectiveStyle,
        direction: Direction,
    ) -> Result<(FontId, bool), LayoutError> {
        for id in fonts.ordered_candidates(&style.families, style.font_style) {
            let resource = fonts
                .get(id)
                .ok_or_else(|| LayoutError::invalid_document("font.unknown-id", None))?;
            let glyphs = self.shape_font(ShapeRequest {
                source,
                range: range.clone(),
                resource,
                size: style.size,
                direction,
                language: &style.language,
                features: &style.features,
                variations: &style.variations,
            })?;
            if !glyphs.is_empty() && glyphs.iter().all(|glyph| glyph.glyph_id != 0) {
                return Ok((id, false));
            }
        }
        Ok((fonts.primary().ok_or(LayoutError::NoFonts)?, true))
    }

    fn lower_constructs(
        &mut self,
        document: &Document,
        paragraph_range: &Range<usize>,
        prepared: &PreparedText,
        fonts: &FontLibrary,
        options: &LayoutOptions,
        call: &mut CallState,
    ) -> Result<(Vec<jlreq_core::Construct>, Vec<AttachmentShape>), LayoutError> {
        let mut constructs = Vec::new();
        let mut attachments = Vec::new();
        for (global_ordinal, construct) in document.constructs.iter().enumerate() {
            let global_range = construct.range();
            if !ranges_overlap(&global_range, paragraph_range) {
                continue;
            }
            if global_range.start < paragraph_range.start || global_range.end > paragraph_range.end
            {
                return Err(LayoutError::invalid_document(
                    "document.construct-crosses-paragraph",
                    Some(global_range),
                ));
            }
            let local_range = global_range.start.saturating_sub(paragraph_range.start)
                ..global_range.end.saturating_sub(paragraph_range.start);
            let local_ordinal = constructs.len();
            match construct {
                DocumentConstruct::Ruby {
                    kind,
                    annotation,
                    runs,
                    ..
                } => {
                    let annotation_options = annotation_options(options);
                    let annotation_prepared = self.prepare_text(
                        PrepareRequest {
                            source: annotation,
                            global_offset: 0,
                            spans: &[],
                            fonts,
                            options: &annotation_options,
                            diagnostic_range: Some(global_range.clone()),
                        },
                        call,
                    )?;
                    let shaped =
                        annotation_prepared.to_core(annotation, annotation_options.font_size)?;
                    let core_runs = ruby_runs(
                        *kind,
                        &local_range,
                        paragraph_range.start,
                        runs,
                        prepared,
                        &annotation_prepared,
                        annotation.len(),
                    )?;
                    let ruby =
                        jlreq_core::Ruby::new(kind.core(), local_range.clone(), shaped, core_runs)?;
                    constructs.push(jlreq_core::Construct::ruby(ruby));
                    attachments.push(AttachmentShape {
                        local_ordinal,
                        global_ordinal,
                        base: global_range,
                        prepared: annotation_prepared,
                    });
                },
                DocumentConstruct::TateChuYoko(_) => {
                    constructs.push(jlreq_core::Construct::tate_chu_yoko(local_range));
                },
                DocumentConstruct::Emphasis { mark, .. } => {
                    constructs.push(jlreq_core::Construct::emphasis_dots(local_range, *mark));
                    let mark_text = mark.to_string();
                    let annotation_options = annotation_options(options);
                    let mark_prepared = self.prepare_text(
                        PrepareRequest {
                            source: &mark_text,
                            global_offset: 0,
                            spans: &[],
                            fonts,
                            options: &annotation_options,
                            diagnostic_range: Some(global_range.clone()),
                        },
                        call,
                    )?;
                    attachments.push(AttachmentShape {
                        local_ordinal,
                        global_ordinal,
                        base: global_range,
                        prepared: mark_prepared,
                    });
                },
                DocumentConstruct::Warichu(_) => {
                    constructs.push(jlreq_core::Construct::warichu(local_range));
                },
                DocumentConstruct::Furawake {
                    columns, line_gap, ..
                } => {
                    constructs.push(jlreq_core::Construct::furawake(
                        local_range,
                        *columns,
                        *line_gap,
                    ));
                },
                DocumentConstruct::Jidori { cells, .. } => {
                    constructs.push(jlreq_core::Construct::jidori(local_range, *cells));
                },
                DocumentConstruct::ReferenceMark { mark, .. } => {
                    let annotation_options = annotation_options(options);
                    let annotation_prepared = self.prepare_text(
                        PrepareRequest {
                            source: mark,
                            global_offset: 0,
                            spans: &[],
                            fonts,
                            options: &annotation_options,
                            diagnostic_range: Some(global_range.clone()),
                        },
                        call,
                    )?;
                    let shaped = annotation_prepared.to_core(mark, annotation_options.font_size)?;
                    constructs.push(jlreq_core::Construct::reference_mark(local_range, shaped));
                    attachments.push(AttachmentShape {
                        local_ordinal,
                        global_ordinal,
                        base: global_range,
                        prepared: annotation_prepared,
                    });
                },
                DocumentConstruct::Script { annotation, .. } => {
                    let annotation_options = annotation_options(options);
                    let annotation_prepared = self.prepare_text(
                        PrepareRequest {
                            source: annotation,
                            global_offset: 0,
                            spans: &[],
                            fonts,
                            options: &annotation_options,
                            diagnostic_range: Some(global_range.clone()),
                        },
                        call,
                    )?;
                    let shaped =
                        annotation_prepared.to_core(annotation, annotation_options.font_size)?;
                    constructs.push(jlreq_core::Construct::script(local_range, shaped));
                    attachments.push(AttachmentShape {
                        local_ordinal,
                        global_ordinal,
                        base: global_range,
                        prepared: annotation_prepared,
                    });
                },
                DocumentConstruct::Formula(_) => {
                    constructs.push(jlreq_core::Construct::formula(local_range));
                },
            }
        }
        Ok((constructs, attachments))
    }
}

#[derive(Debug)]
struct CallState {
    runs: usize,
    glyphs: usize,
    max_runs: usize,
    max_glyphs: usize,
    used_fonts: BTreeSet<FontId>,
    diagnostics: Vec<Diagnostic>,
}

impl CallState {
    fn new(options: &LayoutOptions) -> Self {
        Self {
            runs: 0,
            glyphs: 0,
            max_runs: options.limits.runs,
            max_glyphs: options.limits.glyphs,
            used_fonts: BTreeSet::new(),
            diagnostics: Vec::new(),
        }
    }

    fn charge_run(&mut self) -> Result<(), LayoutError> {
        self.runs = self.runs.saturating_add(1);
        check_limit(Resource::Runs, self.max_runs, self.runs)
    }

    fn charge_glyphs(&mut self, amount: usize) -> Result<(), LayoutError> {
        self.glyphs = self.glyphs.saturating_add(amount);
        check_limit(Resource::Glyphs, self.max_glyphs, self.glyphs)
    }
}

#[derive(Debug, Clone, PartialEq)]
struct EffectiveStyle {
    families: Vec<String>,
    font_style: FontStyle,
    size: i32,
    language: String,
    features: Vec<OpenTypeFeature>,
    variations: Vec<FontVariation>,
    role: TextRole,
}

#[derive(Debug)]
struct GraphemeItem {
    range: Range<usize>,
    level: Level,
    script: ScriptClass,
    direction: Direction,
    font_id: FontId,
    effective: EffectiveStyle,
    is_tab: bool,
}

impl GraphemeItem {
    fn same_run(&self, other: &Self) -> bool {
        self.font_id == other.font_id
            && self.level.is_rtl() == other.level.is_rtl()
            && self.script == other.script
            && self.direction == other.direction
            && self.effective == other.effective
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScriptClass {
    Japanese,
    Latin,
    Rtl,
    Emoji,
    Other,
}

#[derive(Debug, Clone)]
struct RawGlyph {
    font_id: FontId,
    glyph_id: u32,
    cluster: usize,
    x_advance: i32,
    y_advance: i32,
    x_offset: i32,
    y_offset: i32,
}

impl RawGlyph {
    fn inline_advance(&self, direction: Direction) -> i32 {
        match direction {
            Direction::TopToBottom | Direction::BottomToTop => self.y_advance.abs(),
            _ => self.x_advance.abs(),
        }
    }
}

#[derive(Debug, Clone)]
struct PreparedCluster {
    range: Range<usize>,
    advance: i32,
    size: i32,
    frame: jlreq_core::Frame,
    role: Option<jlreq_core::ClusterRole>,
    bidi_level: u8,
    glyphs: Vec<RawGlyph>,
}

#[derive(Debug, Clone)]
struct PreparedText {
    clusters: Vec<PreparedCluster>,
}

impl PreparedText {
    fn to_core(
        &self,
        source: &str,
        default_size: i32,
    ) -> Result<jlreq_core::ShapedText, LayoutError> {
        let size = jlreq_core::Size::square(default_size)?;
        let mut clusters = Vec::with_capacity(self.clusters.len());
        for cluster in &self.clusters {
            let local_size = jlreq_core::Size::square(cluster.size)?;
            let mut core = jlreq_core::Cluster::new(cluster.range.clone(), cluster.advance)
                .with_size(local_size)
                .with_frame(cluster.frame);
            if let Some(role) = cluster.role {
                core = core.with_role(role);
            }
            clusters.push(core);
        }
        Ok(jlreq_core::ShapedText::new(
            source,
            size,
            jlreq_core::Frame::FullEm,
            clusters,
        )?)
    }

    fn is_boundary(&self, offset: usize, source_len: usize) -> bool {
        offset == 0
            || offset == source_len
            || self
                .clusters
                .iter()
                .any(|cluster| cluster.range.start == offset)
    }
}

#[derive(Debug, Clone)]
struct AttachmentShape {
    local_ordinal: usize,
    global_ordinal: usize,
    base: Range<usize>,
    prepared: PreparedText,
}

#[derive(Debug, Clone)]
struct ParagraphSegment {
    content: Range<usize>,
}

fn validate_call(
    document: &Document,
    fonts: &FontLibrary,
    options: &LayoutOptions,
) -> Result<(), LayoutError> {
    if fonts.is_empty() {
        return Err(LayoutError::NoFonts);
    }
    check_limit(
        Resource::InputBytes,
        options.limits.input_bytes,
        document.text.len(),
    )?;
    check_limit(Resource::Fonts, options.limits.fonts, fonts.len())?;
    let font_bytes = fonts
        .fonts()
        .iter()
        .fold(0_usize, |sum, font| sum.saturating_add(font.bytes().len()));
    check_limit(Resource::FontBytes, options.limits.font_bytes, font_bytes)?;
    check_limit(
        Resource::Constructs,
        options.limits.constructs,
        document.constructs.len(),
    )
}

fn check_limit(resource: Resource, limit: usize, observed: usize) -> Result<(), LayoutError> {
    if observed > limit {
        Err(LayoutError::resource(resource, limit, observed))
    } else {
        Ok(())
    }
}

fn map_core_resource_error(error: jlreq_core::ComposeError) -> LayoutError {
    let Some(resource) = high_level_resource(error.resource()) else {
        return LayoutError::CoreComposition(error);
    };
    LayoutError::resource(resource, error.limit(), error.observed())
}

fn high_level_resource(resource: jlreq_core::CompositionResource) -> Option<Resource> {
    match resource {
        jlreq_core::CompositionResource::Clusters => Some(Resource::Glyphs),
        jlreq_core::CompositionResource::BreakCandidates => Some(Resource::Runs),
        jlreq_core::CompositionResource::Constructs | jlreq_core::CompositionResource::TabStops => {
            Some(Resource::Constructs)
        },
        jlreq_core::CompositionResource::SearchTransitions => Some(Resource::CoreOperations),
        _ => None,
    }
}

fn effective_style(
    global: &Range<usize>,
    spans: &[(Range<usize>, SpanStyle)],
    options: &LayoutOptions,
) -> Result<EffectiveStyle, LayoutError> {
    let mut selected = None;
    for (range, style) in spans {
        if ranges_overlap(range, global) {
            if range.start > global.start || range.end < global.end {
                return Err(LayoutError::invalid_document(
                    "document.span-splits-grapheme",
                    Some(global.clone()),
                ));
            }
            selected = Some(style);
            break;
        }
    }
    let mut result = EffectiveStyle {
        families: Vec::new(),
        font_style: FontStyle::default(),
        size: options.font_size,
        language: options.language.clone(),
        features: options.features.clone(),
        variations: options.variations.clone(),
        role: TextRole::Text,
    };
    if let Some(style) = selected {
        result.families.clone_from(&style.families);
        result.font_style = style.font_style;
        result.size = style.font_size.unwrap_or(options.font_size);
        if let Some(language) = &style.language {
            result.language.clone_from(language);
        }
        result.features.extend_from_slice(&style.features);
        result.variations.extend_from_slice(&style.variations);
        result.role = style.role;
    }
    Ok(result)
}

fn aggregate_run(
    source: &str,
    range: Range<usize>,
    glyphs: &[RawGlyph],
    size: i32,
    role: TextRole,
    bidi_level: u8,
    direction: Direction,
) -> Vec<PreparedCluster> {
    let mut starts: Vec<_> = glyphs
        .iter()
        .map(|glyph| glyph.cluster)
        .filter(|start| range.contains(start))
        .collect();
    starts.push(range.start);
    starts.sort_unstable();
    starts.dedup();
    let mut result = Vec::with_capacity(starts.len());
    for (ordinal, start) in starts.iter().copied().enumerate() {
        let end = starts
            .get(ordinal.saturating_add(1))
            .copied()
            .unwrap_or(range.end);
        let members: Vec<_> = glyphs
            .iter()
            .filter(|glyph| glyph.cluster == start)
            .cloned()
            .collect();
        let advance = members.iter().fold(0_i32, |sum, glyph| {
            sum.saturating_add(glyph.inline_advance(direction))
        });
        let piece = &source[start..end];
        result.push(PreparedCluster {
            range: start..end,
            advance,
            size,
            frame: frame_for(piece),
            role: classify_role(source, start..end, role),
            bidi_level,
            glyphs: members,
        });
    }
    result
}

fn frame_for(piece: &str) -> jlreq_core::Frame {
    if piece.chars().count() == 1
        && piece
            .chars()
            .next()
            .is_some_and(|character| is_japanese(character) || is_emoji(character))
    {
        jlreq_core::Frame::FullEm
    } else {
        jlreq_core::Frame::Proportional
    }
}

fn classify_role(
    source: &str,
    range: Range<usize>,
    asserted: TextRole,
) -> Option<jlreq_core::ClusterRole> {
    if asserted != TextRole::Text {
        return Some(asserted.core());
    }
    let mut characters = source[range.clone()].chars();
    let character = characters.next()?;
    if characters.next().is_some() {
        return None;
    }
    let previous = source[..range.start].chars().next_back();
    let next = source[range.end..].chars().next();
    if matches!(character, '.' | '．' | '・')
        && previous.is_some_and(char::is_numeric)
        && next.is_some_and(char::is_numeric)
    {
        return Some(jlreq_core::ClusterRole::DecimalPoint);
    }
    if matches!(character, ',' | '，' | '、')
        && previous.is_some_and(char::is_numeric)
        && next.is_some_and(char::is_numeric)
    {
        return Some(jlreq_core::ClusterRole::DigitGroupSeparator);
    }
    if matches!(character, '!' | '?' | '！' | '？') {
        return Some(if source[range.end..].trim().is_empty() {
            jlreq_core::ClusterRole::SentenceTerminator
        } else {
            jlreq_core::ClusterRole::SentenceMedial
        });
    }
    None
}

fn script_class(text: &str) -> ScriptClass {
    for character in text.chars() {
        let value = character as u32;
        if is_japanese(character) {
            return ScriptClass::Japanese;
        }
        if is_emoji(character) {
            return ScriptClass::Emoji;
        }
        if matches!(value, 0x0590..=0x08ff | 0xfb1d..=0xfdff | 0xfe70..=0xfeff) {
            return ScriptClass::Rtl;
        }
        if character.is_ascii_alphanumeric() || matches!(value, 0x00c0..=0x024f | 0x1e00..=0x1eff) {
            return ScriptClass::Latin;
        }
    }
    ScriptClass::Other
}

fn is_japanese(character: char) -> bool {
    matches!(
        character as u32,
        0x2e80..=0x2fff
            | 0x3000..=0x30ff
            | 0x31f0..=0x31ff
            | 0x3400..=0x4dbf
            | 0x4e00..=0x9fff
            | 0xf900..=0xfaff
            | 0x20000..=0x323af
    )
}

fn is_emoji(character: char) -> bool {
    matches!(character as u32, 0x1f000..=0x1faff | 0x2600..=0x27bf)
}

fn shape_direction(mode: WritingMode, level: Level, script: ScriptClass) -> Direction {
    if mode == WritingMode::VerticalRl
        && matches!(script, ScriptClass::Japanese | ScriptClass::Emoji)
    {
        Direction::TopToBottom
    } else if level.is_rtl() {
        Direction::RightToLeft
    } else {
        Direction::LeftToRight
    }
}

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
    let prohibited: BTreeSet<_> = document
        .prohibited_breaks
        .iter()
        .filter(|offset| paragraph_range.contains(offset))
        .map(|offset| offset.saturating_sub(paragraph_range.start))
        .collect();
    let mandatory: BTreeSet<_> = document
        .mandatory_breaks
        .iter()
        .filter(|offset| paragraph_range.contains(offset))
        .map(|offset| offset.saturating_sub(paragraph_range.start))
        .collect();
    let inside_construct = |offset: usize| {
        constructs.iter().any(|construct| {
            let range = construct.range();
            range.start < offset && offset < range.end
        })
    };
    let mut breaks = BTreeMap::new();
    for offset in LineSegmenter::new_auto(LineBreakOptions::default()).segment_str(source) {
        if automatic_break_allowed(
            offset,
            source.len(),
            prepared.is_boundary(offset, source.len()),
            prohibited.contains(&offset),
            inside_construct(offset),
        ) {
            breaks.insert(offset, false);
        }
    }
    for offset in mandatory {
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

fn map_core_lines(
    layout: &jlreq_core::Layout,
    prepared: &PreparedText,
    attachments: &[AttachmentShape],
    global_offset: usize,
    block_offset: i32,
    options: &LayoutOptions,
) -> Vec<TextLine> {
    let mut result = Vec::with_capacity(layout.lines().len());
    for (line_index, line) in layout.lines().iter().enumerate() {
        let mut cells = Vec::new();
        let mut used = BTreeSet::new();
        for placement in line.clusters() {
            let range = placement.range();
            let cluster_indices = placement_cluster_indices(placement.origin(), prepared, &range);
            let cluster_indices: Vec<_> = cluster_indices
                .into_iter()
                .filter(|index| used.insert(*index))
                .collect();
            if cluster_indices.is_empty() {
                continue;
            }
            let level = prepared.clusters[cluster_indices[0]].bidi_level;
            cells.push(Cell {
                clusters: cluster_indices,
                block: placement.block(),
                advance: placement.advance().max(0),
                level,
                transform: core_transform(placement.transform()),
            });
        }
        let levels: Vec<_> = cells
            .iter()
            .map(|cell| Level::new(cell.level).unwrap_or_else(|_| Level::ltr()))
            .collect();
        let visual = BidiInfo::reorder_visual(&levels);
        let mut cursor = line.inline_origin();
        let mut glyphs = Vec::new();
        for visual_index in visual {
            let cell = &cells[visual_index];
            let mut cluster_cursor = 0_i32;
            let indices = logical_cluster_order(cell.clusters.clone(), cell.level);
            for cluster_index in indices {
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
                        },
                    ));
                    glyph_cursor = glyph_cursor.saturating_add(raw.inline_advance(
                        direction_from_geometry(raw, options.writing_mode, cell.transform),
                    ));
                }
                cluster_cursor = cluster_cursor.saturating_add(cluster.advance);
            }
            cursor = cursor.saturating_add(cell.advance.max(cluster_cursor));
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
        result.push(TextLine {
            range: line.range().start.saturating_add(global_offset)
                ..line.range().end.saturating_add(global_offset),
            origin: physical_origin,
            inline_extent: line.inline_extent(),
            block_extent: line.block_extent(),
            writing_mode: options.writing_mode,
            glyphs,
        });
    }
    result
}

fn placement_cluster_indices(
    origin: jlreq_core::PlacementOrigin,
    prepared: &PreparedText,
    range: &Range<usize>,
) -> Vec<usize> {
    match origin {
        jlreq_core::PlacementOrigin::Cluster(index) => vec![index],
        jlreq_core::PlacementOrigin::Construct(_) => prepared
            .clusters
            .iter()
            .enumerate()
            .filter(|(_, cluster)| ranges_overlap(&cluster.range, range))
            .map(|(index, _)| index)
            .collect(),
        _ => Vec::new(),
    }
}

fn logical_cluster_order(mut indices: Vec<usize>, level: u8) -> Vec<usize> {
    if level % 2 == 1 {
        indices.reverse();
    }
    indices
}

#[derive(Debug)]
struct Cell {
    clusters: Vec<usize>,
    block: i32,
    advance: i32,
    level: u8,
    transform: GlyphTransform,
}

struct PlacementContext {
    source_range: Range<usize>,
    annotation: Option<AnnotationSource>,
    inline: i32,
    block: i32,
    transform: GlyphTransform,
    writing_mode: WritingMode,
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
        transform,
        bidi_level: cluster.bidi_level,
        writing_mode,
    }
}

fn append_attachments(
    glyphs: &mut Vec<GlyphPlacement>,
    line: &jlreq_core::Line,
    shapes: &[AttachmentShape],
    line_index: usize,
    block_offset: i32,
    options: &LayoutOptions,
) {
    for attachment in line.attachments() {
        let Some(shape) = shapes
            .iter()
            .find(|shape| shape.local_ordinal == attachment.construct())
        else {
            continue;
        };
        let requested = attachment.range();
        let clusters: Vec<_> = if requested.is_empty() {
            shape.prepared.clusters.iter().collect()
        } else {
            shape
                .prepared
                .clusters
                .iter()
                .filter(|cluster| ranges_overlap(&cluster.range, &requested))
                .collect()
        };
        let transform = core_transform(attachment.transform());
        let mut inline = attachment.inline();
        for cluster in clusters {
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

fn ranges_overlap(left: &Range<usize>, right: &Range<usize>) -> bool {
    left.start < right.end && right.start < left.end
}

fn diagnostic_severity(value: jlreq_core::Severity) -> DiagnosticSeverity {
    match value {
        jlreq_core::Severity::Info => DiagnosticSeverity::Info,
        jlreq_core::Severity::Error => DiagnosticSeverity::Error,
        _ => DiagnosticSeverity::Warning,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn fixture_fonts() -> (FontLibrary, FontId, FontId) {
        let mut fonts = FontLibrary::new();
        let first = fonts
            .register_face(
                Arc::<[u8]>::from(font_test_data::NOTO_SANS_JP_CFF),
                0,
                "Primary",
                FontStyle::default(),
            )
            .unwrap();
        let second = fonts
            .register_face(
                Arc::<[u8]>::from(font_test_data::TINOS_SUBSET),
                0,
                "Secondary",
                FontStyle::default(),
            )
            .unwrap();
        (fonts, first, second)
    }

    fn raw(font_id: FontId) -> RawGlyph {
        RawGlyph {
            font_id,
            glyph_id: 37,
            cluster: 0,
            x_advance: -256,
            y_advance: -384,
            x_offset: 64,
            y_offset: -128,
        }
    }

    fn cluster(range: Range<usize>, font_id: FontId, level: u8) -> PreparedCluster {
        PreparedCluster {
            range,
            advance: 256,
            size: 192,
            frame: jlreq_core::Frame::Proportional,
            role: None,
            bidi_level: level,
            glyphs: vec![raw(font_id)],
        }
    }

    fn effective(size: i32) -> EffectiveStyle {
        EffectiveStyle {
            families: vec!["Primary".into()],
            font_style: FontStyle::default(),
            size,
            language: "und".into(),
            features: Vec::new(),
            variations: Vec::new(),
            role: TextRole::Text,
        }
    }

    fn grapheme(font_id: FontId) -> GraphemeItem {
        GraphemeItem {
            range: 0..1,
            level: Level::ltr(),
            script: ScriptClass::Latin,
            direction: Direction::LeftToRight,
            font_id,
            effective: effective(1024),
            is_tab: false,
        }
    }

    #[test]
    fn engine_debug_and_font_cache_observe_face_and_byte_identity() {
        let (fonts, id, _) = fixture_fonts();
        let resource = fonts.get(id).unwrap().clone();
        let mut engine = LayoutEngine::new();
        assert!(format!("{engine:?}").contains("cached_fonts: 0"));
        engine.ensure_cache(&resource).unwrap();
        assert!(format!("{engine:?}").contains("cached_fonts: 1"));

        let replacement = FontResource {
            bytes: Arc::from(resource.bytes().to_vec()),
            ..resource.clone()
        };
        assert!(!Arc::ptr_eq(&resource.bytes, &replacement.bytes));
        engine.ensure_cache(&replacement).unwrap();
        assert!(Arc::ptr_eq(
            &engine.fonts.get(&id).unwrap().bytes,
            &replacement.bytes
        ));

        let invalid_face = FontResource {
            face_index: u32::MAX,
            ..replacement
        };
        assert!(engine.ensure_cache(&invalid_face).is_err());
    }

    #[test]
    fn preparation_groups_only_identical_non_tab_runs() {
        let (fonts, _, _) = fixture_fonts();
        let options = LayoutOptions::try_new(200.0, 16.0).unwrap();
        let mut engine = LayoutEngine::new();
        let mut call = CallState::new(&options);
        let prepared = engine
            .prepare_text(
                PrepareRequest {
                    source: "AB",
                    global_offset: 0,
                    spans: &[],
                    fonts: &fonts,
                    options: &options,
                    diagnostic_range: None,
                },
                &mut call,
            )
            .unwrap();
        assert_eq!(call.runs, 1);
        assert_eq!(prepared.clusters.len(), 2);

        let mut call = CallState::new(&options);
        let prepared = engine
            .prepare_text(
                PrepareRequest {
                    source: "A\tB",
                    global_offset: 0,
                    spans: &[],
                    fonts: &fonts,
                    options: &options,
                    diagnostic_range: None,
                },
                &mut call,
            )
            .unwrap();
        assert_eq!(call.runs, 2);
        assert_eq!(prepared.clusters.len(), 3);
        assert!(prepared.clusters[1].glyphs.is_empty());
    }

    #[test]
    fn call_charges_are_inclusive_and_accumulate() {
        let mut options = LayoutOptions::try_new(100.0, 16.0).unwrap();
        options.limits.runs = 1;
        options.limits.glyphs = 3;
        let mut call = CallState::new(&options);
        call.charge_run().unwrap();
        assert_eq!(call.runs, 1);
        assert_eq!(call.charge_run().unwrap_err().code(), "limit.runs");
        call.charge_glyphs(2).unwrap();
        assert_eq!(call.glyphs, 2);
        assert_eq!(call.charge_glyphs(2).unwrap_err().code(), "limit.glyphs");
        assert_eq!(call.glyphs, 4);
    }

    #[test]
    fn run_identity_checks_every_shaping_dimension() {
        let (_, first, second) = fixture_fonts();
        let base = grapheme(first);
        assert!(base.same_run(&grapheme(first)));

        let mut changed = grapheme(second);
        assert!(!base.same_run(&changed));
        changed = grapheme(first);
        changed.level = Level::rtl();
        assert!(!base.same_run(&changed));
        changed = grapheme(first);
        changed.script = ScriptClass::Japanese;
        assert!(!base.same_run(&changed));
        changed = grapheme(first);
        changed.direction = Direction::RightToLeft;
        assert!(!base.same_run(&changed));
        changed = grapheme(first);
        changed.effective.size = 2048;
        assert!(!base.same_run(&changed));
    }

    #[test]
    fn raw_advances_and_prepared_boundaries_are_directional_and_half_open() {
        let (_, id, _) = fixture_fonts();
        let glyph = raw(id);
        assert_eq!(glyph.inline_advance(Direction::LeftToRight), 256);
        assert_eq!(glyph.inline_advance(Direction::RightToLeft), 256);
        assert_eq!(glyph.inline_advance(Direction::TopToBottom), 384);
        assert_eq!(glyph.inline_advance(Direction::BottomToTop), 384);

        let prepared = PreparedText {
            clusters: vec![cluster(2..4, id, 0), cluster(4..6, id, 0)],
        };
        assert!(prepared.is_boundary(0, 8));
        assert!(prepared.is_boundary(8, 8));
        assert!(prepared.is_boundary(2, 8));
        assert!(prepared.is_boundary(4, 8));
        assert!(!prepared.is_boundary(1, 8));
        assert!(!prepared.is_boundary(6, 8));
    }

    #[test]
    fn limits_core_resources_and_severities_map_without_collapsing() {
        assert!(check_limit(Resource::Runs, 2, 2).is_ok());
        let error = check_limit(Resource::Runs, 2, 3).unwrap_err();
        assert_eq!(error.code(), "limit.runs");
        assert_eq!(
            high_level_resource(jlreq_core::CompositionResource::Clusters),
            Some(Resource::Glyphs)
        );
        assert_eq!(
            high_level_resource(jlreq_core::CompositionResource::BreakCandidates),
            Some(Resource::Runs)
        );
        assert_eq!(
            high_level_resource(jlreq_core::CompositionResource::Constructs),
            Some(Resource::Constructs)
        );
        assert_eq!(
            high_level_resource(jlreq_core::CompositionResource::TabStops),
            Some(Resource::Constructs)
        );
        assert_eq!(
            high_level_resource(jlreq_core::CompositionResource::SearchTransitions),
            Some(Resource::CoreOperations)
        );
        assert_eq!(
            diagnostic_severity(jlreq_core::Severity::Info),
            DiagnosticSeverity::Info
        );
        assert_eq!(
            diagnostic_severity(jlreq_core::Severity::Warning),
            DiagnosticSeverity::Warning
        );
        assert_eq!(
            diagnostic_severity(jlreq_core::Severity::Error),
            DiagnosticSeverity::Error
        );
    }

    #[test]
    fn effective_style_rejects_both_sides_of_a_split_grapheme_and_merges_values() {
        let feature = OpenTypeFeature::new(crate::OpenTypeTag::try_new("liga").unwrap(), 0);
        let variation =
            FontVariation::try_new(crate::OpenTypeTag::try_new("wght").unwrap(), 515.0).unwrap();
        let options = LayoutOptions::try_new(100.0, 16.0)
            .unwrap()
            .language("en")
            .unwrap()
            .feature(feature)
            .variation(variation);

        let left = vec![(2..4, SpanStyle::new())];
        assert_eq!(
            effective_style(&(1..3), &left, &options)
                .unwrap_err()
                .code(),
            "document.span-splits-grapheme"
        );
        let right = vec![(0..2, SpanStyle::new())];
        assert_eq!(
            effective_style(&(1..3), &right, &options)
                .unwrap_err()
                .code(),
            "document.span-splits-grapheme"
        );

        let span_feature = OpenTypeFeature::new(crate::OpenTypeTag::try_new("kern").unwrap(), 0);
        let span_variation =
            FontVariation::try_new(crate::OpenTypeTag::try_new("wdth").unwrap(), 90.0).unwrap();
        let span = SpanStyle::new()
            .family("Secondary")
            .font_style(FontStyle::new(600, 90, crate::FontSlant::Italic))
            .font_size(18.0)
            .unwrap()
            .language("ja")
            .unwrap()
            .feature(span_feature)
            .variation(span_variation)
            .role(TextRole::Formula);
        let merged = effective_style(&(1..3), &[(0..4, span)], &options).unwrap();
        assert_eq!(merged.families, ["Secondary"]);
        assert_eq!(merged.size, 18 * 64);
        assert_eq!(merged.language, "ja");
        assert_eq!(merged.features, [feature, span_feature]);
        assert_eq!(merged.variations, [variation, span_variation]);
        assert_eq!(merged.role, TextRole::Formula);
    }

    #[test]
    fn frames_roles_scripts_and_shape_directions_cover_the_closed_tables() {
        assert_eq!(frame_for("日"), jlreq_core::Frame::FullEm);
        assert_eq!(frame_for("😀"), jlreq_core::Frame::FullEm);
        assert_eq!(frame_for("A"), jlreq_core::Frame::Proportional);
        assert_eq!(frame_for("日本"), jlreq_core::Frame::Proportional);

        assert_eq!(
            classify_role("A", 0..1, TextRole::Formula),
            Some(jlreq_core::ClusterRole::Formula)
        );
        for text in ["1.2", "1．2", "1・2"] {
            let start = text.chars().next().unwrap().len_utf8();
            let end = start + text[start..].chars().next().unwrap().len_utf8();
            assert_eq!(
                classify_role(text, start..end, TextRole::Text),
                Some(jlreq_core::ClusterRole::DecimalPoint)
            );
        }
        for text in ["1,2", "1，2", "1、2"] {
            let start = text.chars().next().unwrap().len_utf8();
            let end = start + text[start..].chars().next().unwrap().len_utf8();
            assert_eq!(
                classify_role(text, start..end, TextRole::Text),
                Some(jlreq_core::ClusterRole::DigitGroupSeparator)
            );
        }
        assert_eq!(
            classify_role("!", 0..1, TextRole::Text),
            Some(jlreq_core::ClusterRole::SentenceTerminator)
        );
        assert_eq!(
            classify_role("! A", 0..1, TextRole::Text),
            Some(jlreq_core::ClusterRole::SentenceMedial)
        );
        assert_eq!(classify_role(".2", 0..1, TextRole::Text), None);
        assert_eq!(classify_role("1.", 1..2, TextRole::Text), None);
        assert_eq!(classify_role("1A2", 1..2, TextRole::Text), None);
        assert_eq!(classify_role("A,2", 1..2, TextRole::Text), None);
        assert_eq!(classify_role("12", 0..2, TextRole::Text), None);
        assert_eq!(classify_role("", 0..0, TextRole::Text), None);

        assert_eq!(script_class("日"), ScriptClass::Japanese);
        assert_eq!(script_class("😀"), ScriptClass::Emoji);
        assert_eq!(script_class("ع"), ScriptClass::Rtl);
        assert_eq!(script_class("A"), ScriptClass::Latin);
        assert_eq!(script_class("é"), ScriptClass::Latin);
        assert_eq!(script_class("\u{0301}"), ScriptClass::Other);
        assert!(is_japanese('日'));
        assert!(!is_japanese('A'));
        assert!(is_emoji('😀'));
        assert!(!is_emoji('A'));

        assert_eq!(
            shape_direction(WritingMode::VerticalRl, Level::ltr(), ScriptClass::Japanese),
            Direction::TopToBottom
        );
        assert_eq!(
            shape_direction(WritingMode::VerticalRl, Level::ltr(), ScriptClass::Emoji),
            Direction::TopToBottom
        );
        assert_eq!(
            shape_direction(
                WritingMode::HorizontalTb,
                Level::rtl(),
                ScriptClass::Japanese
            ),
            Direction::RightToLeft
        );
        assert_eq!(
            shape_direction(WritingMode::VerticalRl, Level::rtl(), ScriptClass::Latin),
            Direction::RightToLeft
        );
        assert_eq!(
            shape_direction(WritingMode::HorizontalTb, Level::ltr(), ScriptClass::Latin),
            Direction::LeftToRight
        );
    }

    #[test]
    fn paragraph_separators_keep_original_utf8_ranges() {
        let text = "a\r\nb\rc\nd\u{2028}e\u{2029}f";
        let ranges: Vec<_> = paragraph_segments(text)
            .into_iter()
            .map(|segment| segment.content)
            .collect();
        assert_eq!(ranges, [0..1, 3..4, 5..6, 7..8, 11..12, 15..16]);
        assert_eq!(
            paragraph_segments("\r\n")
                .into_iter()
                .map(|segment| segment.content)
                .collect::<Vec<_>>(),
            [0..0, 2..2]
        );
    }

    #[test]
    fn automatic_and_explicit_break_filters_are_independent() {
        assert!(automatic_break_allowed(3, 6, true, false, false));
        assert!(!automatic_break_allowed(0, 6, true, false, false));
        assert!(!automatic_break_allowed(6, 6, true, false, false));
        assert!(!automatic_break_allowed(3, 6, false, false, false));
        assert!(!automatic_break_allowed(3, 6, true, true, false));
        assert!(!automatic_break_allowed(3, 6, true, false, true));

        let (_, id, _) = fixture_fonts();
        let source = "日本";
        let prepared = PreparedText {
            clusters: vec![cluster(0..3, id, 0), cluster(3..6, id, 0)],
        };
        let plain = DocumentBuilder::new(source).build().unwrap();
        let breaks = collect_breaks(&plain, &(0..6), source, &prepared, &[]);
        assert_eq!(
            breaks.iter().map(|item| item.offset()).collect::<Vec<_>>(),
            [3]
        );
        assert!(!breaks[0].is_mandatory());

        let mut prohibited = DocumentBuilder::new(source);
        prohibited.prohibit_break(3).unwrap();
        assert!(
            collect_breaks(
                &prohibited.build().unwrap(),
                &(0..6),
                source,
                &prepared,
                &[]
            )
            .is_empty()
        );

        let mut mandatory = DocumentBuilder::new(source);
        mandatory.mandatory_break(3).unwrap();
        let breaks = collect_breaks(&mandatory.build().unwrap(), &(0..6), source, &prepared, &[]);
        assert_eq!(breaks.len(), 1);
        assert!(breaks[0].is_mandatory());

        assert!(
            collect_breaks(
                &plain,
                &(0..6),
                source,
                &prepared,
                &[jlreq_core::Construct::tate_chu_yoko(0..6)]
            )
            .is_empty()
        );
        assert_eq!(
            collect_breaks(
                &plain,
                &(0..6),
                source,
                &prepared,
                &[jlreq_core::Construct::tate_chu_yoko(0..3)]
            )
            .len(),
            1
        );
        assert_eq!(
            collect_breaks(
                &plain,
                &(0..6),
                source,
                &prepared,
                &[jlreq_core::Construct::tate_chu_yoko(3..6)]
            )
            .len(),
            1
        );
    }

    #[test]
    fn tab_stops_and_annotation_options_honor_exact_boundaries() {
        let exact = LayoutOptions::try_new(64.0, 16.0)
            .unwrap()
            .tab_width(2)
            .unwrap();
        assert_eq!(
            collect_tab_stops("\t", &exact)
                .unwrap()
                .iter()
                .map(|stop| stop.position())
                .collect::<Vec<_>>(),
            [2048]
        );
        let bounded = LayoutOptions::try_new(100.0, 16.0)
            .unwrap()
            .tab_width(2)
            .unwrap()
            .limits(crate::ResourceLimits::default().with_max_constructs(1));
        assert_eq!(collect_tab_stops("\t", &bounded).unwrap().len(), 1);
        assert!(collect_tab_stops("no tab", &bounded).unwrap().is_empty());

        let source = LayoutOptions::try_new(101.0, 17.0)
            .unwrap()
            .alignment(Alignment::End);
        let annotation = annotation_options(&source);
        assert_eq!(annotation.font_size, 544);
        assert_eq!(annotation.line_extent, source.line_extent);
        assert_eq!(annotation.alignment, Alignment::Start);
    }

    #[test]
    fn ruby_run_lowering_handles_declared_group_and_filtered_mono_runs() {
        let (_, id, _) = fixture_fonts();
        let base = PreparedText {
            clusters: vec![
                cluster(0..1, id, 0),
                cluster(1..2, id, 0),
                cluster(2..3, id, 0),
                cluster(3..4, id, 0),
                cluster(4..5, id, 0),
            ],
        };
        let annotation = PreparedText {
            clusters: vec![cluster(0..1, id, 0), cluster(1..2, id, 0)],
        };
        let declared = [crate::RubyRun::new(12..13, 0..1)];
        let runs = ruby_runs(
            crate::RubyKind::Mono,
            &(2..3),
            10,
            &declared,
            &base,
            &annotation,
            2,
        )
        .unwrap();
        assert_eq!(runs[0].base(), 2..3);
        assert_eq!(runs[0].annotation(), 0..1);

        let group = ruby_runs(
            crate::RubyKind::Group,
            &(2..4),
            0,
            &[],
            &base,
            &annotation,
            2,
        )
        .unwrap();
        assert_eq!(group.len(), 1);
        assert_eq!(group[0].base(), 2..4);
        assert_eq!(group[0].annotation(), 0..2);

        let mono = ruby_runs(
            crate::RubyKind::Mono,
            &(2..4),
            0,
            &[],
            &base,
            &annotation,
            2,
        )
        .unwrap();
        assert_eq!(mono.len(), 2);
        assert_eq!(mono[0].base(), 2..3);
        assert_eq!(mono[1].base(), 3..4);
    }

    #[test]
    fn placement_origins_bidi_order_and_physical_geometry_are_exact() {
        let (_, id, _) = fixture_fonts();
        let prepared = PreparedText {
            clusters: vec![
                cluster(0..2, id, 0),
                cluster(2..4, id, 1),
                cluster(4..6, id, 0),
            ],
        };
        assert_eq!(
            placement_cluster_indices(jlreq_core::PlacementOrigin::Cluster(2), &prepared, &(0..6)),
            [2]
        );
        assert_eq!(
            placement_cluster_indices(
                jlreq_core::PlacementOrigin::Construct(0),
                &prepared,
                &(1..5)
            ),
            [0, 1, 2]
        );
        assert_eq!(logical_cluster_order(vec![0, 1, 2], 0), [0, 1, 2]);
        assert_eq!(logical_cluster_order(vec![0, 1, 2], 1), [2, 1, 0]);

        let raw = raw(id);
        let horizontal = place_raw_glyph(
            &raw,
            &prepared.clusters[0],
            PlacementContext {
                source_range: 0..2,
                annotation: None,
                inline: 320,
                block: 640,
                transform: GlyphTransform::Identity,
                writing_mode: WritingMode::HorizontalTb,
            },
        );
        assert_eq!(horizontal.geometry_26_6(), (320, 832, 256, 0, 64, 128));
        let vertical = place_raw_glyph(
            &raw,
            &prepared.clusters[0],
            PlacementContext {
                source_range: 0..2,
                annotation: None,
                inline: 320,
                block: 640,
                transform: GlyphTransform::Identity,
                writing_mode: WritingMode::VerticalRl,
            },
        );
        assert_eq!(vertical.geometry_26_6(), (640, 320, 0, 384, 64, 128));
        let tate_chu_yoko = place_raw_glyph(
            &raw,
            &prepared.clusters[0],
            PlacementContext {
                source_range: 0..2,
                annotation: None,
                inline: 320,
                block: 640,
                transform: GlyphTransform::TateChuYoko,
                writing_mode: WritingMode::VerticalRl,
            },
        );
        assert_eq!(tate_chu_yoko.geometry_26_6(), (320, 832, 256, 0, 64, 128));
    }

    #[test]
    fn block_and_direction_helpers_distinguish_every_geometry_branch() {
        let horizontal = LayoutOptions::try_new(100.0, 16.0)
            .unwrap()
            .line_gap(2.0)
            .unwrap();
        assert_eq!(adjusted_block(10, 3, 20, &horizontal), 414);
        let vertical = horizontal.clone().writing_mode(WritingMode::VerticalRl);
        assert_eq!(adjusted_block(10, 3, 20, &vertical), -354);

        let (_, id, _) = fixture_fonts();
        let with_vertical_advance = raw(id);
        assert_eq!(
            direction_from_geometry(
                &with_vertical_advance,
                WritingMode::VerticalRl,
                GlyphTransform::Identity
            ),
            Direction::TopToBottom
        );
        assert_eq!(
            direction_from_geometry(
                &with_vertical_advance,
                WritingMode::VerticalRl,
                GlyphTransform::TateChuYoko
            ),
            Direction::LeftToRight
        );
        assert_eq!(
            direction_from_geometry(
                &with_vertical_advance,
                WritingMode::HorizontalTb,
                GlyphTransform::Identity
            ),
            Direction::LeftToRight
        );
        let mut without_vertical_advance = with_vertical_advance.clone();
        without_vertical_advance.y_advance = 0;
        assert_eq!(
            direction_from_geometry(
                &without_vertical_advance,
                WritingMode::VerticalRl,
                GlyphTransform::RotateClockwise
            ),
            Direction::LeftToRight
        );

        assert_eq!(advance_block(100, 30, WritingMode::HorizontalTb), 130);
        assert_eq!(advance_block(100, 30, WritingMode::VerticalRl), 70);
        assert!(ranges_overlap(&(0..2), &(1..3)));
        assert!(!ranges_overlap(&(0..1), &(1..2)));
        assert!(!ranges_overlap(&(1..2), &(0..1)));
    }
}
