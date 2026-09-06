// SPDX-FileCopyrightText: 2026 jlreq contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

struct FontCache {
    bytes: Arc<[u8]>,
    face_index: u32,
    shaper_data: Arc<ShaperData>,
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
    paragraph_index: usize,
    global_offset: usize,
    spans: &'a [(Range<usize>, SpanStyle)],
    fonts: &'a FontLibrary,
    options: &'a LayoutOptions,
    diagnostic_range: Option<Range<usize>>,
}

struct SelectRequest<'a> {
    source: &'a str,
    range: Range<usize>,
    fonts: &'a FontLibrary,
    style: &'a EffectiveStyle,
    direction: Direction,
    site: Site,
}

#[derive(Clone, Copy)]
struct LowerRequest<'a> {
    document: &'a Document,
    prepared: &'a PreparedText,
    fonts: &'a FontLibrary,
    options: &'a LayoutOptions,
}

struct ConstructParagraph<'a> {
    index: usize,
    range: &'a Range<usize>,
    next_construct: &'a mut usize,
}

/// Where a stretch of prepared text belongs in the document, for the trace.
///
/// A construct's annotation is exactly the text that carries its own diagnostic range: its
/// offsets are into a string the document does not contain, so both a diagnostic and a
/// trace site have to name the construct instead. The two travel on one carrier rather
/// than as two flags that could disagree, which is [ADR 0019]'s rule applied here.
///
/// [ADR 0019]: https://github.com/jlreq/jlreq/blob/main/docs/adr/0019-one-fact-one-carrier.md
#[derive(Debug, Clone)]
struct TraceFrame {
    paragraph: usize,
    offset: usize,
    attributed: Option<Range<usize>>,
}

impl TraceFrame {
    fn new(paragraph: usize, offset: usize, attributed: Option<Range<usize>>) -> Self {
        Self {
            paragraph,
            offset,
            attributed,
        }
    }

    /// The document site a paragraph-local byte range belongs to.
    fn site(&self, local: &Range<usize>) -> Site {
        self.attributed.clone().map_or_else(
            || {
                Site::in_paragraph(
                    self.paragraph,
                    local.start.saturating_add(self.offset)
                        ..local.end.saturating_add(self.offset),
                )
            },
            |range| Site::in_paragraph(self.paragraph, range),
        )
    }

    const fn annotation(&self) -> bool {
        self.attributed.is_some()
    }
}

/// Reusable high-level layout engine.
///
/// Font parsing and shaping caches are retained between calls. Returned layouts never borrow
/// the engine, and an error leaves it immediately reusable.
pub struct LayoutEngine {
    fonts: BTreeMap<FontId, FontCache>,
    composer: jlreq_core::Composer,
    unicode_buffer: Option<UnicodeBuffer>,
    shape_features: Vec<Feature>,
    shape_variations: Vec<Variation>,
}

/// Say what the engine is holding, not just how much of it.
///
/// This is the type a person reaches for when a glyph came from the wrong face, and a
/// single count answers none of the questions they have. The cached faces are named by
/// identifier and TTC face index — the pair `select_font` actually resolves against — and
/// the shaper's reusable buffer is reported as held or in flight, because a
/// `None` there means a previous `shape_font` call unwound between taking and returning it.
///
/// The parsed font data itself is deliberately not printed: it is megabytes of `Arc<[u8]>`
/// and `Debug` output is read in a terminal.
impl fmt::Debug for LayoutEngine {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let faces: Vec<_> = self
            .fonts
            .iter()
            .map(|(id, cached)| (id.get(), cached.face_index, cached.bytes.len()))
            .collect();
        formatter
            .debug_struct("LayoutEngine")
            .field("cached_fonts", &self.fonts.len())
            .field("faces_id_index_bytes", &faces)
            .field("shaper_buffer_held", &self.unicode_buffer.is_some())
            .field("feature_scratch", &self.shape_features.len())
            .field("variation_scratch", &self.shape_variations.len())
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
            unicode_buffer: Some(UnicodeBuffer::new()),
            shape_features: Vec::new(),
            shape_variations: Vec::new(),
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

    /// Lay out plain UTF-8 text, recording why the result came out as it did.
    ///
    /// The trace is a runtime choice and never a second code path, so what it explains is
    /// exactly what [`layout`](Self::layout) does. See [`crate::trace`] for the format and
    /// for choosing which families to record.
    pub fn layout_traced(
        &mut self,
        text: &str,
        fonts: &FontLibrary,
        options: LayoutOptions,
        trace: &mut DocumentTrace,
    ) -> Result<TextLayout, LayoutError> {
        let document = DocumentBuilder::new(text).build()?;
        self.layout_document_inner(&document, fonts, options, trace)
    }

    /// Shape, compose, reorder, and physically place a typed document.
    pub fn layout_document(
        &mut self,
        document: &Document,
        fonts: &FontLibrary,
        options: LayoutOptions,
    ) -> Result<TextLayout, LayoutError> {
        self.layout_document_inner(document, fonts, options, &mut DocumentTrace::off())
    }

    /// Lay out a typed document, recording why the result came out as it did.
    ///
    /// A paragraph that refuses still leaves its reasoning behind: the core's trace is
    /// absorbed before the refusal is returned, because that is the case a reader most
    /// needs it for.
    pub fn layout_document_traced(
        &mut self,
        document: &Document,
        fonts: &FontLibrary,
        options: LayoutOptions,
        trace: &mut DocumentTrace,
    ) -> Result<TextLayout, LayoutError> {
        self.layout_document_inner(document, fonts, options, trace)
    }

    fn layout_document_inner(
        &mut self,
        document: &Document,
        fonts: &FontLibrary,
        options: LayoutOptions,
        trace: &mut DocumentTrace,
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
                options: options.clone(),
            });
        }

        let segments = paragraph_segments(&document.text);
        check_limit(
            Resource::Paragraphs,
            options.limits.paragraphs,
            segments.len(),
        )?;
        trace.record(
            Site::document(0..document.text.len()),
            Fact::TextSegmented {
                paragraphs: segments.len(),
                bytes: document.text.len(),
                writing_mode: options.writing_mode,
                base_direction: options.base_direction,
                line_extent: options.line_extent,
                font_size: options.font_size,
            },
        );
        let mut call = CallState::new(options);
        let mut lines = Vec::new();
        let mut block_offset = 0_i32;
        let mut next_construct = 0_usize;

        for (paragraph_index, segment) in segments.iter().enumerate() {
            let content = &document.text[segment.content.clone()];
            let overrides = paragraph_style_for(document, &segment.content)?;
            let line_extent = overrides
                .and_then(|style| style.line_extent)
                .unwrap_or(options.line_extent);
            let alignment = overrides
                .and_then(|style| style.alignment)
                .unwrap_or(options.alignment);
            let first_line_indent = overrides
                .and_then(|style| style.first_line_indent)
                .unwrap_or(options.first_line_indent);
            let widow = overrides
                .and_then(|style| style.widow)
                .unwrap_or(options.widow);
            trace.record(
                Site::in_paragraph(paragraph_index, segment.content.clone()),
                Fact::ParagraphSegment {
                    index: paragraph_index,
                    blank: content.is_empty(),
                    line_extent,
                    alignment,
                    first_line_indent,
                    widow,
                },
            );
            if content.is_empty() {
                // A blank paragraph has no clusters for the core to place, so
                // the facade applies that paragraph's own indent and alignment
                // to the caret position directly. Its caret otherwise sat at
                // the margin while every neighboring line obeyed the style.
                let inline = empty_line_inline(line_extent, first_line_indent, alignment);
                let origin = match options.writing_mode {
                    WritingMode::HorizontalTb => Point::from_fixed(inline, block_offset),
                    WritingMode::VerticalRl => Point::from_fixed(block_offset, inline),
                };
                lines.push(TextLine {
                    range: segment.content.clone(),
                    origin,
                    inline_extent: 0,
                    block_extent: options.font_size,
                    writing_mode: options.writing_mode,
                    glyphs: Vec::new(),
                    hit_bounds: None,
                    index: 0,
                    paragraph_index,
                    first_in_paragraph: true,
                    last_in_paragraph: true,
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
                    paragraph_index,
                    global_offset: segment.content.start,
                    spans: &document.spans,
                    fonts,
                    options,
                    diagnostic_range: None,
                },
                &mut call,
                trace,
            )?;
            let shaped = prepared.to_core(content, options.font_size)?;
            let mut construct_paragraph = ConstructParagraph {
                index: paragraph_index,
                range: &segment.content,
                next_construct: &mut next_construct,
            };
            let (constructs, attachments, construct_globals) = self.lower_constructs(
                LowerRequest {
                    document,
                    prepared: &prepared,
                    fonts,
                    options,
                },
                &mut construct_paragraph,
                &mut call,
                trace,
            )?;
            let breaks =
                collect_breaks(document, &segment.content, content, &prepared, &constructs);
            trace_breaks(trace, paragraph_index, &segment.content, &breaks);
            let policy = overrides
                .and_then(|style| style.style.as_ref())
                .unwrap_or(&options.style);
            let explicit_stops = overrides
                .and_then(|style| style.tab_stops.as_deref())
                .unwrap_or(&options.tab_stops);
            let tabs = collect_tab_stops(content, options, line_extent, explicit_stops)?;
            let paragraph = jlreq_core::Paragraph::builder(shaped, line_extent)
                .breaks(breaks)
                .constructs(constructs)
                .tab_stops(tabs)
                .alignment(alignment.core())
                .writing_mode(options.writing_mode.core())
                .first_line_indent(first_line_indent)
                .widow(widow.core())
                .build()?;

            let core_limits = jlreq_core::CompositionLimits::default()
                .with_max_clusters(options.limits.glyphs)
                .with_max_break_candidates(options.limits.runs)
                .with_max_constructs(options.limits.constructs)
                .with_max_tab_stops(options.limits.constructs)
                .with_max_search_transitions(options.limits.core_operations);
            self.composer.set_limits(core_limits);
            // Absorb before the refusal is raised: a paragraph the composer could not set
            // is the case whose reasoning a reader most needs.
            let mut core_trace = trace.core_trace();
            let composed = self
                .composer
                .compose_traced(&paragraph, policy, &mut core_trace);
            trace.absorb(paragraph_index, segment.content.start, &mut core_trace);
            let core_layout = composed.map_err(map_core_resource_error)?;

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
                    message: core_diagnostic_message(diagnostic.code()),
                    jlreq: Some(diagnostic.jlreq()),
                });
            }

            let paragraph_lines = map_core_lines(
                &core_layout,
                &prepared,
                &LineMapping {
                    attachments: &attachments,
                    construct_globals: &construct_globals,
                    global_offset: segment.content.start,
                    block_offset,
                    paragraph_index,
                },
                options,
                trace,
            );
            let next_block_offset = next_paragraph_block_offset(
                &paragraph_lines,
                block_offset,
                options,
            );
            lines.extend(paragraph_lines);
            block_offset = next_block_offset;
        }

        assign_line_metadata(&mut lines);
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
            options: options.clone(),
        })
    }

    fn ensure_cache(&mut self, resource: &FontResource) -> Result<(), LayoutError> {
        let needs_replacement = self.fonts.get(&resource.id()).is_none_or(|cached| {
            cached.face_index != resource.face_index()
                || !Arc::ptr_eq(&cached.bytes, &resource.bytes)
        });
        if needs_replacement {
            let _ = harfrust::FontRef::from_index(resource.bytes(), resource.face_index())
                .map_err(|_| LayoutError::invalid_font(resource.face_index()))?;
            self.fonts.insert(
                resource.id(),
                FontCache {
                    bytes: resource.bytes.clone(),
                    face_index: resource.face_index(),
                    shaper_data: Arc::clone(&resource.shaper_data),
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
        self.shape_variations.clear();
        self.shape_variations
            .extend(variations.iter().map(|variation| Variation {
                tag: harfrust::Tag::new(&variation.tag().bytes()),
                value: variation.value(),
            }));
        self.shape_features.clear();
        self.shape_features.extend(features.iter().map(|feature| {
            Feature::new(
                harfrust::Tag::new(&feature.tag().bytes()),
                feature.value(),
                ..,
            )
        }));
        let cached = self
            .fonts
            .get(&resource.id())
            .ok_or_else(|| LayoutError::invalid_font(resource.face_index()))?;
        let font = harfrust::FontRef::from_index(&cached.bytes, cached.face_index)
            .map_err(|_| LayoutError::invalid_font(cached.face_index))?;
        let instance = ShaperInstance::from_variations(&font, &self.shape_variations);
        let shaper = cached
            .shaper_data
            .shaper(&font)
            .instance(Some(&instance))
            .build();
        let mut buffer = self.unicode_buffer.take().unwrap_or_default();
        buffer.clear();
        let _ = buffer.reserve(source[range.clone()].chars().count());
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
        let glyphs = shaper.shape(
            buffer,
            ShapeOptions::new()
                .scale(Some(size))
                .features(&self.shape_features),
        );
        let raw = glyphs
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
            .collect();
        self.unicode_buffer = Some(glyphs.clear());
        Ok(raw)
    }

    fn prepare_text(
        &mut self,
        request: PrepareRequest<'_>,
        call: &mut CallState,
        trace: &mut DocumentTrace,
    ) -> Result<PreparedText, LayoutError> {
        let PrepareRequest {
            source,
            paragraph_index,
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
        let frame = TraceFrame::new(paragraph_index, global_offset, diagnostic_range.clone());
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
        let mut styles = StyleResolver::new(spans, options, global_offset);
        for pair in boundaries.windows(2) {
            let range = pair[0]..pair[1];
            let global =
                range.start.saturating_add(global_offset)..range.end.saturating_add(global_offset);
            let effective = styles.resolve(&global)?;
            for family in &effective.families {
                if !fonts.has_family(family) && !call.reported_families.contains(family) {
                    call.reported_families.insert(family.clone());
                    call.diagnostics.push(Diagnostic {
                        code: "font.unknown-family",
                        severity: DiagnosticSeverity::Warning,
                        range: Some(diagnostic_range.clone().unwrap_or_else(|| global.clone())),
                        message:
                            "no registered face declares the requested family; the library fallback order was used",
                        jlreq: None,
                    });
                }
            }
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
                self.select_font(
                    SelectRequest {
                        source,
                        range: range.clone(),
                        fonts,
                        style: &effective,
                        direction,
                        site: frame.site(&range),
                    },
                    call,
                    trace,
                )?
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
        let mut runs = 0_usize;
        while index < graphemes.len() {
            if graphemes[index].is_tab {
                let item = &graphemes[index];
                let resource = fonts
                    .get(item.font_id)
                    .ok_or_else(crate::font::unknown_font_id)?;
                clusters.push(PreparedCluster {
                    range: item.range.clone(),
                    advance: 0,
                    size: item.effective.size,
                    frame: jlreq_core::Frame::Proportional,
                    role: None,
                    bidi_level: item.level.number(),
                    variations: resolved_variations(&item.effective, resource),
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
            runs = runs.saturating_add(1);
            let first = &graphemes[start];
            let run_range = first.range.start..graphemes[index.saturating_sub(1)].range.end;
            let resource = fonts
                .get(first.font_id)
                .ok_or_else(crate::font::unknown_font_id)?;
            let variations = resolved_variations(&first.effective, resource);
            #[cfg(test)]
            call.charge_shape();
            let raw = self.shape_font(ShapeRequest {
                source,
                range: run_range.clone(),
                resource,
                size: first.effective.size,
                direction: first.direction,
                language: &first.effective.language,
                features: &first.effective.features,
                variations: &variations,
            })?;
            call.used_fonts.insert(first.font_id);
            call.charge_glyphs(raw.len())?;
            trace.record(
                frame.site(&run_range),
                Fact::ShapingRun {
                    script: trace_script(first.script),
                    direction: trace_direction(first.direction),
                    level: first.level.number(),
                    face: first.font_id.get(),
                    glyphs: raw.len(),
                    annotation: frame.annotation(),
                },
            );
            clusters.extend(aggregate_run(
                source,
                run_range,
                raw,
                &first.effective,
                first.level.number(),
                first.direction,
                &variations,
            ));
        }
        trace.record(
            frame.site(&(0..source.len())),
            Fact::TextItemized {
                graphemes: graphemes.len(),
                runs,
                clusters: clusters.len(),
                base_level: bidi.paragraph_level.number(),
                mixed_levels: graphemes
                    .iter()
                    .any(|item| item.level != bidi.paragraph_level),
                annotation: frame.annotation(),
            },
        );
        Ok(PreparedText { clusters })
    }

    fn select_font(
        &mut self,
        request: SelectRequest<'_>,
        call: &mut CallState,
        trace: &mut DocumentTrace,
    ) -> Result<(FontId, bool), LayoutError> {
        let SelectRequest {
            source,
            range,
            fonts,
            style,
            direction,
            site,
        } = request;
        let candidate_key = FontCandidateKey::new(style);
        let selection_key = FontSelectionKey::new(
            &source[range.clone()],
            style,
            direction,
            candidate_key.clone(),
        );
        if let Some(selection) = call.font_selections.get(&selection_key) {
            return Ok(*selection);
        }
        let candidates = call
            .font_candidates
            .entry(candidate_key)
            .or_insert_with(|| {
                Arc::from(
                    fonts
                        .ordered_candidates(&style.families, style.font_style)
                        .into_boxed_slice(),
                )
            })
            .clone();
        for (position, id) in candidates.iter().copied().enumerate() {
            let resource = fonts
                .get(id)
                .ok_or_else(crate::font::unknown_font_id)?;
            let variations = resolved_variations(style, resource);
            #[cfg(test)]
            call.charge_shape();
            let glyphs = self.shape_font(ShapeRequest {
                source,
                range: range.clone(),
                resource,
                size: style.size,
                direction,
                language: &style.language,
                features: &style.features,
                variations: &variations,
            })?;
            if !glyphs.is_empty() && glyphs.iter().all(|glyph| glyph.glyph_id != 0) {
                let selection = (id, false);
                trace.record(
                    site,
                    Fact::FaceChosen {
                        face: id.get(),
                        family: resource.family().to_owned(),
                        position: position.saturating_add(1),
                        candidates: candidates.len(),
                    },
                );
                call.font_selections.insert(selection_key, selection);
                return Ok(selection);
            }
        }
        let primary = fonts.primary().ok_or(LayoutError::NoFonts)?;
        trace.record(
            site,
            Fact::FaceFallback {
                face: primary.get(),
                family: fonts
                    .get(primary)
                    .map_or_else(String::new, |resource| resource.family().to_owned()),
                candidates: candidates.len(),
            },
        );
        let selection = (primary, true);
        call.font_selections.insert(selection_key, selection);
        Ok(selection)
    }

    fn lower_constructs(
        &mut self,
        request: LowerRequest<'_>,
        paragraph: &mut ConstructParagraph<'_>,
        call: &mut CallState,
        trace: &mut DocumentTrace,
    ) -> Result<LoweredConstructs, LayoutError> {
        let LowerRequest {
            document,
            prepared,
            fonts,
            options,
        } = request;
        let mut constructs = Vec::new();
        let mut attachments = Vec::new();
        let mut construct_globals = Vec::new();
        while let Some(construct) = document.constructs.get(*paragraph.next_construct) {
            let global_ordinal = *paragraph.next_construct;
            let global_range = construct.range();
            if global_range.start >= paragraph.range.end {
                break;
            }
            *paragraph.next_construct = paragraph.next_construct.saturating_add(1);
            if !ranges_overlap(&global_range, paragraph.range) {
                continue;
            }
            if global_range.start < paragraph.range.start || global_range.end > paragraph.range.end
            {
                return Err(LayoutError::invalid_document(
                    "document.construct-crosses-paragraph",
                    Some(global_range),
                    "a construct must stay inside one paragraph",
                ));
            }
            let local_range = global_range.start.saturating_sub(paragraph.range.start)
                ..global_range.end.saturating_sub(paragraph.range.start);
            let local_ordinal = constructs.len();
            attachments.push(None);
            construct_globals.push((local_range.clone(), global_ordinal));
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
                            paragraph_index: paragraph.index,
                            global_offset: 0,
                            spans: &[],
                            fonts,
                            options: &annotation_options,
                            diagnostic_range: Some(global_range.clone()),
                        },
                        call,
                            trace,
                    )?;
                    let shaped =
                        annotation_prepared.to_core(annotation, annotation_options.font_size)?;
                    let core_runs = ruby_runs(
                        *kind,
                        &local_range,
                        paragraph.range.start,
                        runs,
                        prepared,
                        &annotation_prepared,
                        annotation.len(),
                    )?;
                    let ruby =
                        jlreq_core::Ruby::new(kind.core(), local_range.clone(), shaped, core_runs)?;
                    constructs.push(jlreq_core::Construct::ruby(ruby));
                    attachments[local_ordinal] = Some(AttachmentShape {
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
                            paragraph_index: paragraph.index,
                            global_offset: 0,
                            spans: &[],
                            fonts,
                            options: &annotation_options,
                            diagnostic_range: Some(global_range.clone()),
                        },
                        call,
                            trace,
                    )?;
                    attachments[local_ordinal] = Some(AttachmentShape {
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
                            paragraph_index: paragraph.index,
                            global_offset: 0,
                            spans: &[],
                            fonts,
                            options: &annotation_options,
                            diagnostic_range: Some(global_range.clone()),
                        },
                        call,
                            trace,
                    )?;
                    let shaped = annotation_prepared.to_core(mark, annotation_options.font_size)?;
                    constructs.push(jlreq_core::Construct::reference_mark(local_range, shaped));
                    attachments[local_ordinal] = Some(AttachmentShape {
                        global_ordinal,
                        base: global_range,
                        prepared: annotation_prepared,
                    });
                },
                DocumentConstruct::Script {
                    annotation,
                    position,
                    ..
                } => {
                    let annotation_options = annotation_options(options);
                    let annotation_prepared = self.prepare_text(
                        PrepareRequest {
                            source: annotation,
                            paragraph_index: paragraph.index,
                            global_offset: 0,
                            spans: &[],
                            fonts,
                            options: &annotation_options,
                            diagnostic_range: Some(global_range.clone()),
                        },
                        call,
                            trace,
                    )?;
                    let shaped =
                        annotation_prepared.to_core(annotation, annotation_options.font_size)?;
                    constructs.push(jlreq_core::Construct::script_at(
                        local_range,
                        shaped,
                        position.core(),
                    ));
                    attachments[local_ordinal] = Some(AttachmentShape {
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
        Ok((constructs, attachments, construct_globals))
    }
}

/// The lowered core constructs, per-construct attachment shapes, and each
/// construct's paragraph-local range paired with its document ordinal.
type LoweredConstructs = (
    Vec<jlreq_core::Construct>,
    Vec<Option<AttachmentShape>>,
    Vec<(Range<usize>, usize)>,
);

fn assign_line_metadata(lines: &mut [TextLine]) {
    let total = lines.len();
    for index in 0..total {
        let paragraph = lines[index].paragraph_index;
        let first = index
            .checked_sub(1)
            .and_then(|previous| lines.get(previous))
            .is_none_or(|previous| previous.paragraph_index != paragraph);
        let last = lines
            .get(index.saturating_add(1))
            .is_none_or(|next| next.paragraph_index != paragraph);
        if let Some(line) = lines.get_mut(index) {
            line.index = index;
            line.first_in_paragraph = first;
            line.last_in_paragraph = last;
        }
    }
}

/// Where a blank paragraph's caret sits on the inline axis.
///
/// An empty line occupies no inline space, so alignment distributes the whole
/// measure that the indent does not claim. Justify aligns like start, matching
/// the core's treatment of a line it cannot stretch.
fn empty_line_inline(line_extent: i32, first_line_indent: i32, alignment: Alignment) -> i32 {
    let remaining = line_extent.saturating_sub(first_line_indent).max(0);
    let offset = match alignment {
        Alignment::Start | Alignment::Justify => 0,
        Alignment::Center => remaining / 2,
        Alignment::End => remaining,
    };
    first_line_indent.saturating_add(offset)
}

/// Say what a core diagnostic means, in the facade's own words.
///
/// `jlreq_core::Diagnostic` carries no message, so every core-originating diagnostic used
/// to arrive at a caller wearing one constant sentence and could only be told apart by its
/// code. The core's codes are already a closed set with a documented meaning each, so the
/// sentence is derived here rather than added to a core output type: the seam gains the
/// message and the core keeps the shape the census was run against.
///
/// A code this does not know is a core release ahead of this facade. It says so plainly
/// rather than guessing, and the code itself remains the compatibility key.
/// Say how many opportunities of each strength the composer was given.
///
/// Counted from the list that is actually handed over, not from the sources it was merged
/// from, so the number a reader sees is the number the search searched.
fn trace_breaks(
    trace: &mut DocumentTrace,
    paragraph: usize,
    range: &Range<usize>,
    breaks: &[jlreq_core::Break],
) {
    let mandatory = breaks.iter().filter(|entry| entry.is_mandatory()).count();
    let discretionary = breaks
        .iter()
        .filter(|entry| entry.is_discretionary())
        .count();
    trace.record(
        Site::in_paragraph(paragraph, range.clone()),
        Fact::BreaksCollected {
            allowed: breaks
                .len()
                .saturating_sub(mandatory)
                .saturating_sub(discretionary),
            discretionary,
            mandatory,
        },
    );
}

/// The facade's own coarse script partition, as the trace names it.
const fn trace_script(class: ScriptClass) -> Script {
    match class {
        ScriptClass::Japanese => Script::Japanese,
        ScriptClass::Latin => Script::Latin,
        ScriptClass::Rtl => Script::Rtl,
        ScriptClass::Emoji => Script::Emoji,
        ScriptClass::Other => Script::Other,
    }
}

/// The direction a run was shaped in, as the trace names it.
const fn trace_direction(direction: Direction) -> RunDirection {
    match direction {
        Direction::LeftToRight => RunDirection::LeftToRight,
        Direction::RightToLeft => RunDirection::RightToLeft,
        Direction::TopToBottom => RunDirection::TopToBottom,
        Direction::BottomToTop | Direction::Invalid => RunDirection::Other,
    }
}

fn core_diagnostic_message(code: &str) -> &'static str {
    match code {
        "layout.overfull" => {
            "the line could not be reduced to the measure; its extent exceeds the line extent"
        },
        "layout.widow" => {
            "the final line holds fewer base clusters than the widow policy asks for"
        },
        _ => "the core composer produced a recoverable layout diagnostic this release does not name",
    }
}
