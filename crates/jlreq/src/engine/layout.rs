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
    global_offset: usize,
    spans: &'a [(Range<usize>, SpanStyle)],
    fonts: &'a FontLibrary,
    options: &'a LayoutOptions,
    diagnostic_range: Option<Range<usize>>,
}

struct ConstructParagraph<'a> {
    range: &'a Range<usize>,
    next_construct: &'a mut usize,
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
                options: options.clone(),
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
        let mut next_construct = 0_usize;

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
                    hit_bounds: None,
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
            let mut construct_paragraph = ConstructParagraph {
                range: &segment.content,
                next_construct: &mut next_construct,
            };
            let (constructs, attachments) = self.lower_constructs(
                document,
                &mut construct_paragraph,
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
            let next_block_offset = next_paragraph_block_offset(
                &paragraph_lines,
                block_offset,
                options,
            );
            lines.extend(paragraph_lines);
            block_offset = next_block_offset;
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
        let mut styles = StyleResolver::new(spans, options, global_offset);
        for pair in boundaries.windows(2) {
            let range = pair[0]..pair[1];
            let global =
                range.start.saturating_add(global_offset)..range.end.saturating_add(global_offset);
            let effective = styles.resolve(&global)?;
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
                self.select_font(source, range.clone(), fonts, &effective, direction, call)?
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
        Ok(PreparedText { clusters })
    }

    fn select_font(
        &mut self,
        source: &str,
        range: Range<usize>,
        fonts: &FontLibrary,
        style: &EffectiveStyle,
        direction: Direction,
        call: &mut CallState,
    ) -> Result<(FontId, bool), LayoutError> {
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
        for id in candidates.iter().copied() {
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
                call.font_selections.insert(selection_key, selection);
                return Ok(selection);
            }
        }
        let selection = (fonts.primary().ok_or(LayoutError::NoFonts)?, true);
        call.font_selections.insert(selection_key, selection);
        Ok(selection)
    }

    fn lower_constructs(
        &mut self,
        document: &Document,
        paragraph: &mut ConstructParagraph<'_>,
        prepared: &PreparedText,
        fonts: &FontLibrary,
        options: &LayoutOptions,
        call: &mut CallState,
    ) -> Result<(Vec<jlreq_core::Construct>, Vec<Option<AttachmentShape>>), LayoutError> {
        let mut constructs = Vec::new();
        let mut attachments = Vec::new();
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
                            global_offset: 0,
                            spans: &[],
                            fonts,
                            options: &annotation_options,
                            diagnostic_range: Some(global_range.clone()),
                        },
                        call,
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
                    attachments[local_ordinal] = Some(AttachmentShape {
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
        Ok((constructs, attachments))
    }
}
